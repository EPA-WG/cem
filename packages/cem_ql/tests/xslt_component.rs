//! XSLT-VIEW-BROWSER-SCALARS: component control expressions stay native.
use cem_ql::{
    eval::{imported_cem_tree, AtomValue, Item, ItemStream},
    render::{render_plan_to_html, TemplateData},
    xslt::component::{XsltComponent, XsltComponentOptions, XsltScalarMapping},
};

const SOURCE: &str = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:params" version="3.0">
<xsl:template name="view"><xsl:param name="p:text" select="'default'"/><xsl:param name="enabled" select="false()"/>
<p><xsl:value-of select="$p:text"/>:<xsl:value-of select="$enabled"/></p></xsl:template></xsl:stylesheet>"#;

const IMPORT_SOURCE: &str = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:params" version="3.0"><xsl:import href="./base.xslt"/></xsl:stylesheet>"#;

fn imported_options() -> XsltComponentOptions {
    let mut mapped = options("\"imported\"");
    mapped
        .modules
        .push(cem_ql::xslt::component::XsltSourceModule {
            parent_uri: "memory:component.xslt".into(),
            href: "./base.xslt".into(),
            uri: "memory:base.xslt".into(),
            source: SOURCE.into(),
            content_hash: cem_ml::content_cache::ContentHash::from_blake3(SOURCE.as_bytes())
                .header_value(),
        });
    mapped
}

fn options(select: &str) -> XsltComponentOptions {
    XsltComponentOptions {
        entrypoint: Some("view".into()),
        parameters: vec![XsltScalarMapping {
            name: "p:text".into(),
            select: select.into(),
        }],
        ..Default::default()
    }
}

fn data() -> TemplateData {
    // An explicit native host context, not synthesized by the component adapter.
    TemplateData::default().with_binding(
        "document",
        ItemStream::once(imported_cem_tree(
            cem_ml::import::import_data("<input/>", "xml", "cem", "memory:context").unwrap(),
        )),
    )
}

#[test]
fn maps_native_control_expressions_and_preserves_unmapped_defaults() {
    let mut mapped = options("state ?? \"fallback\"");
    mapped.parameters.push(XsltScalarMapping {
        name: "enabled".into(),
        select: "true".into(),
    });
    let component =
        XsltComponent::compile(SOURCE, "memory:component.xslt", &mapped, &["state".into()])
            .unwrap();
    for value in ["one", "two"] {
        let data = data().with_binding(
            "state",
            ItemStream::once(Item::Atomic(AtomValue::String(value.into()))),
        );
        let plan = component.render(&data);
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        assert_eq!(render_plan_to_html(&plan), format!("<p>{value}:true</p>"));
    }
    let component = XsltComponent::compile(
        SOURCE,
        "memory:component.xslt",
        &options("state"),
        &["state".into()],
    )
    .unwrap();
    let plan = component.render(&data());
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert_eq!(render_plan_to_html(&plan), "<p>:false</p>");
    let unmapped = XsltComponentOptions {
        entrypoint: Some("view".into()),
        ..Default::default()
    };
    let component =
        XsltComponent::compile(SOURCE, "memory:component.xslt", &unmapped, &[]).unwrap();
    assert_eq!(
        render_plan_to_html(&component.render(&data())),
        "<p>default:false</p>"
    );
}

#[test]
fn characterizes_the_named_entry_absent_focus_gate() {
    let component =
        XsltComponent::compile(SOURCE, "memory:component.xslt", &options("\"scalar\""), &[])
            .unwrap();
    let plan = component.render(&TemplateData::default());
    assert!(plan.nodes.is_empty());
    assert!(
        plan.diagnostics
            .iter()
            .any(|d| d.code == "cem.xslt.bundle_argument"
                && d.message
                    .contains("context requires exactly one native item")),
        "{:?}",
        plan.diagnostics
    );
}

#[test]
fn imports_modules_with_namespaced_names_and_rejects_hash_drift() {
    use cem_ql::xslt::compiler::stylesheet_imports;
    let source = IMPORT_SOURCE;
    assert_eq!(
        stylesheet_imports(source, "memory:component.xslt").unwrap(),
        ["./base.xslt"]
    );
    let mut mapped = imported_options();
    let component = XsltComponent::compile(source, "memory:component.xslt", &mapped, &[]).unwrap();
    let plan = component.render(&data());
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert_eq!(render_plan_to_html(&plan), "<p>imported:false</p>");
    mapped.modules[0].source = mapped.modules[0].source.replace("default", "changed");
    assert!(XsltComponent::compile(source, "memory:component.xslt", &mapped, &[]).is_err());
}

#[test]
fn rejects_undeclared_duplicate_and_invalid_mappings() {
    for options in [
        XsltComponentOptions {
            parameters: vec![XsltScalarMapping {
                name: "missing".into(),
                select: "1".into(),
            }],
            ..options("1")
        },
        XsltComponentOptions {
            parameters: vec![options("1").parameters[0].clone(); 2],
            ..options("1")
        },
        options("unknown +"),
        options("undeclared"),
    ] {
        assert!(XsltComponent::compile(SOURCE, "memory:component.xslt", &options, &[]).is_err());
    }
    for value in [Item::Array(vec![]), Item::Record(Default::default())] {
        let component = XsltComponent::compile(
            SOURCE,
            "memory:component.xslt",
            &options("state"),
            &["state".into()],
        )
        .unwrap();
        let plan = component
            .render(&TemplateData::default().with_binding("state", ItemStream::once(value)));
        assert!(plan.nodes.is_empty());
        assert!(
            plan.diagnostics
                .iter()
                .any(|d| d.code == "cem.xslt.scalar_mapping"),
            "{:?}",
            plan.diagnostics
        );
    }
    let component = XsltComponent::compile(
        SOURCE,
        "memory:component.xslt",
        &options("document"),
        &["document".into()],
    )
    .unwrap();
    let plan = component.render(&data());
    assert!(plan.nodes.is_empty());
    assert!(plan
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.xslt.scalar_mapping"));
    let component =
        XsltComponent::compile(SOURCE, "memory:component.xslt", &options("(1, 2)"), &[]).unwrap();
    let plan = component.render(&data());
    assert!(plan.nodes.is_empty());
    assert!(plan
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.xslt.scalar_mapping"));
}

#[test]
fn native_document_properties_can_supply_scalar_controls() {
    let component = XsltComponent::compile(
        SOURCE,
        "memory:component.xslt",
        &options("data:node_key(document)"),
        &["document".into()],
    )
    .unwrap();
    let plan = component.render(&data());
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert!(render_plan_to_html(&plan).starts_with("<p>cem-source:1:"));
}

#[test]
fn exports_component_control_fixture() {
    let Some(path) = std::env::var_os("CEM_XSLT_COMPONENT_FIXTURE_DIR") else {
        return;
    };
    let directory = std::path::PathBuf::from(path);
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("component.xslt"), SOURCE).unwrap();
    std::fs::write(directory.join("component-import.xslt"), IMPORT_SOURCE).unwrap();
    std::fs::write(
        directory.join("component-import-options.json"),
        serde_json::to_vec(&imported_options()).unwrap(),
    )
    .unwrap();
    std::fs::write(
        directory.join("component-options.json"),
        serde_json::to_vec(&options("datadom.slices.text ?? \"fallback\"")).unwrap(),
    )
    .unwrap();
}
