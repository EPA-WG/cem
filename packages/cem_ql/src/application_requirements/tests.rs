use super::*;
use crate::{
    api::{compile, CompileContext},
    declaration_artifact::{
        admit_declaration_metadata, build_declaration_artifact, build_declaration_imports,
        BuiltDeclarationArtifact,
    },
    ir::{CompiledQuery, IrId, IrNode},
    template_artifact::TemplateArtifactSourceMapMode,
};
fn build(body: &str) -> BuiltDeclarationArtifact {
    build_declaration_artifact(&format!("<cem-element tag=\"cem-requirements\"><template id=\"cem-requirements\" type=\"text/cem-ml\">{body}</template></cem-element>"),
        "https://app.test/components/view.xhtml",TemplateArtifactSourceMapMode::Prod,None).unwrap()
}
fn inventory() -> Inventory {
    Inventory {
        output: compiled_declaration_requirements(&build("{p | Plain}")).unwrap(),
        registry: ModuleRegistry::with_all_known(),
        items: 0,
    }
}
fn query(source: &str) -> CompiledQuery {
    compile(source, &CompileContext::default()).unwrap()
}
fn has(output: &DeclarationRequirements, module: &str, name: &str, arity: usize) -> bool {
    output
        .functions
        .iter()
        .any(|f| f.module == format!("cem:stdlib/{module}") && f.name == name && f.arity == arity)
}
#[test]
fn collects_aliases_unused_function_bodies_branches_and_pipeline_arities_from_ir() {
    let mut collector = inventory();
    let mut compiled = query(
        r#"import "cem:stdlib/strings" as words
        declare function local:unused(x) { words:length(x) }
        if true { seq:first((1,2)) } else { (1,2).seq:take(1) }"#,
    );
    // Neither original source string is read or reparsed by the inventory.
    compiled.source = "not a query; native:call('not-real')".into();
    collector.query(&compiled, "query").unwrap();
    assert!(has(&collector.output, "strings", "length", 1));
    assert!(has(&collector.output, "sequence", "first", 1));
    assert!(has(&collector.output, "sequence", "take", 2));
    assert!(collector
        .output
        .functions
        .iter()
        .any(|f| f.name == "take" && f.dispatch == "pipeline"));
    assert!(!collector
        .output
        .functions
        .iter()
        .any(|f| f.module == "cem:stdlib/native"));
    assert!(
        collector.output.unresolved.is_empty(),
        "{:?}",
        collector.output.unresolved
    );
}
#[test]
fn visits_template_attributes_variables_conditionals_loops_and_lambdas_without_evaluation() {
    let built = build(
        r#"{cem:variable @name=label @select='str:upper("item")'}
    {cem:choose | {cem:when @test='seq:count((1,2)) > 0' |
      {p @title="prefix-{str:lower(label)}" | {$ | str:length(label)}}
    }{cem:otherwise | {$ | num:abs(-1)}}}
    {cem:for-each @as=item @select='seq:map((1,2), fn(x) => num:abs(x))' | {$ | item}}"#,
    );
    let (report, stages) =
        crate::compile_profile::measure(|| compiled_declaration_requirements(&built).unwrap());
    assert!(
        stages.keys().all(|key| *key == "stdlib/assemble-registry"),
        "{stages:?}"
    );
    for (module, name, arity) in [
        ("strings", "upper", 1),
        ("strings", "lower", 1),
        ("strings", "length", 1),
        ("sequence", "count", 1),
        ("sequence", "map", 2),
        ("numbers", "abs", 1),
    ] {
        assert!(has(&report, module, name, arity), "{module}/{name}");
    }
    assert_eq!(report.metadata_hash, built.metadata_hash);
    assert!(report.expression_count >= 7);
    assert!(report.unresolved.is_empty(), "{:?}", report.unresolved);
    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains("prefix-")); // no authored values/query text
    assert_eq!(
        serialized,
        serde_json::to_string(&compiled_declaration_requirements(&built).unwrap()).unwrap()
    );
}
#[test]
fn reports_literal_and_dynamic_data_formats_and_host_contracts_without_exporting_inputs() {
    let mut collector = inventory();
    for source in [
        r#"data:parse("do-not-export-input", "json")"#,
        r#"data:read("do-not-export-input", "text/csv", "cem")"#,
        r#"fn(format) => data:parse("do-not-export-input", format)"#,
        r#"native:call("do-not-export-host-argument", "do-not-export-input")"#,
        r#"import "cem:stdlib/template" as t t:names()"#,
    ] {
        let compiled = query(source);
        collector.query(&compiled, "query").unwrap();
    }
    assert!(collector
        .output
        .data_imports
        .iter()
        .any(|d| d.format.as_deref() == Some("json")));
    assert!(collector
        .output
        .data_imports
        .iter()
        .any(|d| d.format.as_deref() == Some("text/csv") && d.schema.as_deref() == Some("cem")));
    for kind in [
        "dynamic-data-format",
        "native-host-dispatch",
        "template-provider",
    ] {
        assert!(
            collector
                .output
                .unresolved
                .iter()
                .any(|gap| gap.kind == kind),
            "{kind}: {:?}",
            collector.output.unresolved
        );
    }
    assert!(!serde_json::to_string(&collector.output)
        .unwrap()
        .contains("do-not-export"));
}
#[test]
fn dynamic_callbacks_calls_payloads_and_html_resources_remain_open() {
    let mut collector = inventory();
    collector
        .query(&query("fn(callback) => seq:map((1,2), callback)"), "query")
        .unwrap();
    assert!(collector
        .output
        .unresolved
        .iter()
        .any(|g| g.kind == "dynamic-callback"));
    let mut compiled = query("declare function local:identity(x) { x } local:identity(1)");
    let root = compiled.tree.root;
    let IrNode::Call { callee, .. } = compiled.tree.nodes[root.0 as usize].clone() else {
        panic!("expected call")
    };
    compiled.tree.nodes[callee.0 as usize] = IrNode::LocalVar(crate::resolve::BindingId(1000));
    collector.query(&compiled, "query").unwrap();
    assert!(collector
        .output
        .unresolved
        .iter()
        .any(|g| g.kind == "dynamic-call"));
    let report =
        compiled_declaration_requirements(&build("{a @href=./help | Help}{slot}")).unwrap();
    assert!(report
        .unresolved
        .iter()
        .any(|g| g.kind == "html-resource-or-style-contract"));
    assert!(report
        .unresolved
        .iter()
        .any(|g| g.kind == "dynamic-component-or-payload"));
    let mut application = ApplicationRequirements::default();
    application.roots.insert("cem-requirements".into());
    application
        .component_dependencies
        .insert("cem-requirements".into(), BTreeSet::new());
    application
        .declarations
        .insert("cem-requirements".into(), report);
    assert!(application.require_static_inventory().is_err());
    assert!(!application.native_link_closure_verified);
}
#[test]
fn inventories_transitive_css_with_final_bases_source_keys_and_no_loader_reuse() {
    let mut loaded = 0;
    let built = build_declaration_imports(
        build("{style | ``` @import './a.css'; :scope {display:block} ```}{p | Imports}"),
        &mut |request| {
            loaded += 1;
            let (uri, css) = if request.resolution.resolved_url.ends_with("a.css") {
                (
                    "https://cdn.test/styles/a.css".into(),
                    "@import './b.css'; :scope {background:url('../icon.svg')}",
                )
            } else {
                (
                    request.resolution.resolved_url.clone(),
                    ":scope {color:red}",
                )
            };
            Ok(cem_ml::resolver::ResolvedRead {
                uri,
                bytes: css.as_bytes().to_vec(),
                content_type: Some("text/css".into()),
            })
        },
    )
    .unwrap();
    assert_eq!(loaded, 2);
    let report = compiled_declaration_requirements(&built).unwrap();
    assert_eq!(loaded, 2);
    assert_eq!(report.stylesheets.len(), 3);
    assert!(report.unresolved.is_empty(), "{:?}", report.unresolved);
    assert_eq!(
        report.stylesheets[0].base_url,
        "https://app.test/components/view.xhtml"
    );
    assert_eq!(
        report.stylesheets[1].source_uri,
        "https://cdn.test/styles/a.css"
    );
    assert_eq!(
        report.stylesheets[2].source_uri,
        "https://cdn.test/styles/b.css"
    );
    assert!(report.stylesheets.iter().all(|s| s
        .source_key
        .as_ref()
        .is_some_and(|k| k.starts_with("cem-source:1:"))));
    assert!(report.stylesheets[1]
        .resources
        .iter()
        .any(|r| r.kind == "css-url"
            && r.resolved_url.as_deref() == Some("https://cdn.test/icon.svg")));
    let source =
        compiled_declaration_requirements(&build("{style | ``` @import './a.css'; ```}")).unwrap();
    assert!(source
        .unresolved
        .iter()
        .any(|g| g.kind == "source-css-import"));
    assert_eq!(source.stylesheets.len(), 1);
}
#[test]
fn refuses_invalid_pins_and_format_claims_before_issuing_requirements() {
    let mut built = build("{p | Pins}");
    built.metadata_hash.clear();
    assert!(compiled_declaration_requirements(&built).is_err());
    let mut built = build("{p | Pins}");
    let mut metadata =
        admit_declaration_metadata(&built.metadata_json, &built.metadata_hash).unwrap();
    metadata.template_format = "cem-template-artifact/3".into();
    built.metadata_json = serde_json::to_string(&metadata).unwrap();
    built.metadata_hash =
        cem_ml::content_cache::ContentHash::from_blake3(built.metadata_json.as_bytes())
            .header_value();
    assert!(compiled_declaration_requirements(&built)
        .unwrap_err()
        .contains("format/pin"));
    let mut built = build("{p | Pins}");
    built.template.bytes[0] ^= 1;
    assert!(compiled_declaration_requirements(&built).is_err());
}
#[test]
fn rejects_dangling_cyclic_oversized_and_inconsistent_native_ir() {
    for case in 0..4 {
        let mut collector = inventory();
        let mut compiled = query("seq:first((1,2))");
        let root = compiled.tree.root;
        match case {
            0 => compiled.tree.types.clear(),
            1 => {
                let IrNode::StdlibCall { args, .. } = &mut compiled.tree.nodes[root.0 as usize]
                else {
                    panic!()
                };
                args[0] = IrId(u32::MAX);
            }
            2 => {
                let IrNode::StdlibCall { args, .. } = &mut compiled.tree.nodes[root.0 as usize]
                else {
                    panic!()
                };
                args[0] = root;
            }
            _ => {
                let IrNode::StdlibCall { args, .. } = &mut compiled.tree.nodes[root.0 as usize]
                else {
                    panic!()
                };
                *args = vec![IrId(0); MAX_ITEMS + 1];
            }
        }
        assert!(collector.query(&compiled, "bad").is_err(), "case {case}");
    }
}
#[test]
fn cart_and_canonical_components_produce_deterministic_conservative_requirements() {
    let cart = include_str!("../../fixtures/component-application/shopping-cart/cart-view.cem");
    // The native source fixture binds response explicitly as a template input.
    let mut collector = inventory();
    let artifact = crate::render::compile_template(
        cart,
        &crate::render::CompileTemplateOptions {
            host_bindings: vec!["response".into()],
            ..Default::default()
        },
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    collector.nodes(&artifact.nodes, 0).unwrap();
    assert!(has(&collector.output, "sequence", "where", 2));
    assert!(
        collector.output.unresolved.is_empty(),
        "{:?}",
        collector.output.unresolved
    );
    for tag in [
        "cem-action",
        "cem-text-field",
        "cem-card",
        "cem-text",
        "cem-alert",
    ] {
        let source =
            std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
                format!("../cem-components/src/components/{tag}/{tag}.xhtml"),
            ))
            .unwrap();
        let built = build_declaration_artifact(
            &source,
            &format!("https://app.test/{tag}.xhtml"),
            TemplateArtifactSourceMapMode::Prod,
            None,
        )
        .unwrap();
        let report = compiled_declaration_requirements(&built).unwrap();
        assert_eq!(
            serde_json::to_string(&report).unwrap(),
            serde_json::to_string(&compiled_declaration_requirements(&built).unwrap()).unwrap()
        );
        if report.capability.is_some() {
            assert!(report
                .unresolved
                .iter()
                .any(|g| g.kind == "shared-behavior-closure"));
        }
    }
}

#[test]
fn pins_and_bounds_the_build_control_sidecar_without_claiming_native_link_closure() {
    let mut application = ApplicationRequirements::default();
    application.roots.insert("cem-probe".into());
    application
        .component_dependencies
        .insert("cem-probe".into(), BTreeSet::new());
    application.declarations.insert(
        "cem-probe".into(),
        compiled_declaration_requirements(&build("{p | Plain}")).unwrap(),
    );
    application.require_static_inventory().unwrap();
    let (bytes, hash) = application.pinned_json().unwrap();
    assert_eq!(
        hash,
        cem_ml::content_cache::ContentHash::from_blake3(&bytes).header_value()
    );
    assert_eq!(application.pinned_json().unwrap(), (bytes.clone(), hash));
    assert!(std::str::from_utf8(&bytes)
        .unwrap()
        .contains("\"nativeLinkClosureVerified\": false"));
    application
        .declarations
        .get_mut("cem-probe")
        .unwrap()
        .source_uri = "x".repeat(1024 * 1024);
    assert!(application.pinned_json().is_err());
}

#[test]
fn rejects_incomplete_root_and_component_graphs_in_sidecars() {
    let mut application = ApplicationRequirements::default();
    assert!(application.pinned_json().is_err());
    application.roots.insert("cem-missing".into());
    application.declarations.insert(
        "cem-probe".into(),
        compiled_declaration_requirements(&build("{p | Plain}")).unwrap(),
    );
    application
        .component_dependencies
        .insert("cem-probe".into(), BTreeSet::new());
    assert!(application.pinned_json().is_err());
    application.roots = BTreeSet::from(["cem-probe".into()]);
    application
        .component_dependencies
        .get_mut("cem-probe")
        .unwrap()
        .insert("cem-missing".into());
    assert!(application.require_static_inventory().is_err());
}
