//! XSLT-MATCH-RUNTIME: stylesheet dispatch over retained native documents.
use cem_ml::{import::import_data, validation::xpath::XPathExpandedName};
use cem_ql::{
    eval::{imported_cem_tree, ItemStream},
    render::{render_plan_to_html, TemplateData},
    xslt::{
        compiler::{compile_xslt_bundle_with_options, XsltCompileOptions, XsltModuleSource},
        XsltBundle,
    },
};

fn stylesheet(body: &str) -> String {
    format!(
        r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:v="urn:vars" version="3.0">{body}</xsl:stylesheet>"#
    )
}
fn compile(body: &str, options: &XsltCompileOptions) -> XsltBundle {
    let compiled =
        compile_xslt_bundle_with_options(&stylesheet(body), "memory:match.xslt", options).unwrap();
    XsltBundle::from_bytes(
        &compiled.bytes,
        &compiled.content_hash,
        &compiled.source_hash,
    )
    .unwrap()
}
fn render(bundle: &XsltBundle, xml: &str) -> String {
    let data = TemplateData::default().with_binding(
        "document",
        ItemStream::once(imported_cem_tree(
            import_data(xml, "xml", "cem", "memory:input.xml").unwrap(),
        )),
    );
    let plan = bundle.render(&data);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    render_plan_to_html(&plan)
}

#[test]
fn recursive_named_calls_preserve_native_parameters_defaults_and_focus() {
    let bundle = compile(
        r#"
<xsl:template match="/"><xsl:for-each select="/*/*"><xsl:call-template name="v:walk"><xsl:with-param name="n" select="2"/><xsl:with-param name="bag" select="map{'rows': [(), .]}"/></xsl:call-template><i><xsl:value-of select="position() || '/' || last()"/></i></xsl:for-each></xsl:template>
<xsl:template name="v:walk"><xsl:param name="n" select="1"/><xsl:param name="label" select="'default'"/><xsl:param name="bag" required="yes"/><b><xsl:value-of select="string($bag?rows?(2)) || ':' || $n || ':' || $label || ':' || position() || '/' || last()"/></b><xsl:if test="$n gt 0"><xsl:call-template name="v:walk"><xsl:with-param name="n" select="$n - 1"/><xsl:with-param name="label" select="()"/><xsl:with-param name="bag" select="$bag"/></xsl:call-template></xsl:if></xsl:template>
"#,
        &XsltCompileOptions::default(),
    );
    assert_eq!(render(&bundle, "<r><a>A</a><a>B</a></r>"), "<b>A:2:default:1/2</b><b>A:1::1/2</b><b>A:0::1/2</b><i>1/2</i><b>B:2:default:2/2</b><b>B:1::2/2</b><b>B:0::2/2</b><i>2/2</i>");
}

#[test]
fn named_entrypoint_selects_declared_template() {
    let options = XsltCompileOptions {
        entrypoint: Some(XPathExpandedName::new(Some("urn:vars"), "entry")),
        ..Default::default()
    };
    let bundle = compile(
        r#"<xsl:template match="/"><p>wrong</p></xsl:template><xsl:template name="v:entry"><p>selected</p></xsl:template>"#,
        &options,
    );
    assert_eq!(render(&bundle, "<r/>"), "<p>selected</p>");
}

#[test]
fn matched_templates_recurse_with_modes_predicates_and_sequence_focus() {
    let bundle = compile(
        r##"
<xsl:template match="/"><ul><xsl:apply-templates select="/*/*" mode="v:rows"><xsl:with-param name="label" select="'row'"/></xsl:apply-templates></ul></xsl:template>
<xsl:template match="*" mode="v:rows"><xsl:param name="label" select="'fallback'"/><li><xsl:value-of select="$label || ':' || position() || '/' || last()"/><xsl:apply-templates mode="#current"/></li></xsl:template>
<xsl:template match="a[@kind='special']" mode="v:rows" priority="0.75"><xsl:param name="label"/><b><xsl:value-of select="$label || ':' || string(.) || ':' || position() || '/' || last()"/></b></xsl:template>
<xsl:template match="text()" mode="v:rows"><xsl:value-of select="string(.) || '!'"/></xsl:template>
<xsl:template match="a" mode="unused" priority="999"><p>wrong mode</p></xsl:template>
"##,
        &XsltCompileOptions::default(),
    );
    assert_eq!(
        render(&bundle, "<r><a kind='special'>a</a><a>b</a></r>"),
        "<ul><b>row:a:1/2</b><li>row:2/2b!</li></ul>"
    );
}

#[test]
fn import_precedence_beats_priority_and_imported_calls_use_overrides() {
    let library = stylesheet(
        r#"<xsl:template name="entry"><xsl:call-template name="card"/></xsl:template><xsl:template name="card"><b>base</b></xsl:template><xsl:template match="a" priority="999"><i>base</i></xsl:template>"#,
    );
    let options = XsltCompileOptions {
        modules: vec![XsltModuleSource {
            parent_uri: "memory:match.xslt".into(),
            href: "base.xslt".into(),
            uri: "memory:base.xslt".into(),
            content_hash: cem_ml::content_cache::ContentHash::from_blake3(library.as_bytes()),
            source: library,
        }],
        ..Default::default()
    };
    let bundle = compile(
        r#"<xsl:import href="base.xslt"/><xsl:template match="/"><xsl:call-template name="entry"/><xsl:apply-templates select="/*/*"/></xsl:template><xsl:template name="card"><b>override</b></xsl:template><xsl:template match="a" priority="-999"><i><xsl:value-of select="."/></i></xsl:template>"#,
        &options,
    );
    assert_eq!(
        render(&bundle, "<r><a>A</a></r>"),
        "<b>override</b><i>A</i>"
    );
    assert_eq!(
        render(&bundle, "<r><a>B</a><a>C</a></r>"),
        "<b>override</b><i>B</i><i>C</i>"
    );
    assert_eq!(bundle.stylesheets().len(), 2);
    if let Ok(directory) = std::env::var("CEM_XSLT_MATCH_FIXTURE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        let source = stylesheet(
            r#"<xsl:import href="base.xslt"/><xsl:template match="/"><xsl:call-template name="entry"/><xsl:apply-templates select="/*/*"/></xsl:template><xsl:template name="card"><b>override</b></xsl:template><xsl:template match="a" priority="-999"><i><xsl:value-of select="."/></i></xsl:template>"#,
        );
        let compiled =
            compile_xslt_bundle_with_options(&source, "memory:match.xslt", &options).unwrap();
        std::fs::write(directory.join("matched.bin"), &compiled.bytes).unwrap();
        std::fs::write(directory.join("matched.json"), serde_json::to_vec(&serde_json::json!({"contentHash": compiled.content_hash.header_value(), "sourceHash": compiled.source_hash.header_value()})).unwrap()).unwrap();
    }
}

fn module(parent: &str, href: &str, uri: &str, body: &str) -> XsltModuleSource {
    let source = stylesheet(body);
    XsltModuleSource {
        parent_uri: parent.into(),
        href: href.into(),
        uri: uri.into(),
        content_hash: cem_ml::content_cache::ContentHash::from_blake3(source.as_bytes()),
        source,
    }
}

#[test]
fn exact_priorities_union_defaults_and_document_order() {
    let bundle = compile(
        r#"<xsl:template match="/"><xsl:apply-templates select="/*/*"/></xsl:template>
<xsl:template match="a" priority="0.50000000000000000002"><b>precise</b></xsl:template>
<xsl:template match="a" priority="0.50000000000000000001"><b>rounded wrong</b></xsl:template>
<xsl:template match="b"><b>name</b></xsl:template>
<xsl:template match="* | c"><b>union</b></xsl:template>
<xsl:template match="c"><b>last</b></xsl:template>"#,
        &Default::default(),
    );
    assert_eq!(
        render(&bundle, "<r><a/><b/><c/><d/></r>"),
        "<b>precise</b><b>name</b><b>last</b><b>union</b>"
    );
}

#[test]
fn includes_share_precedence_and_later_imports_override_earlier_imports() {
    let options = XsltCompileOptions {
        modules: vec![
            module(
                "memory:match.xslt",
                "first",
                "memory:first",
                r#"<xsl:template match="a" priority="999"><b>first</b></xsl:template>"#,
            ),
            module(
                "memory:match.xslt",
                "second",
                "memory:second",
                r#"<xsl:template match="a" priority="-999"><b>second</b></xsl:template>"#,
            ),
            module(
                "memory:match.xslt",
                "shared",
                "memory:shared",
                r#"<xsl:template match="b"><b>included</b></xsl:template>"#,
            ),
        ],
        ..Default::default()
    };
    let source = r#"<xsl:import href="first"/><xsl:import href="second"/><xsl:include href="shared"/>
<xsl:template match="/"><xsl:apply-templates select="/*/*"/></xsl:template><xsl:template match="b"><b>root</b></xsl:template>"#;
    let bundle = compile(source, &options);
    assert_eq!(
        render(&bundle, "<r><a/><b/></r>"),
        "<b>second</b><b>root</b>"
    );
    let mut reordered = options;
    reordered.modules.reverse();
    assert_eq!(
        compile_xslt_bundle_with_options(&stylesheet(source), "memory:match.xslt", &reordered)
            .unwrap()
            .bytes,
        compile_xslt_bundle_with_options(
            &stylesheet(source),
            "memory:match.xslt",
            &XsltCompileOptions {
                modules: reordered.modules.iter().rev().cloned().collect(),
                ..Default::default()
            }
        )
        .unwrap()
        .bytes
    );
}

#[test]
fn builtins_forward_supplied_parameters_and_namespace_patterns() {
    let bundle = compile(
        r##"<xsl:template match="/"><xsl:apply-templates mode="v:rows"><xsl:with-param name="p" select="'passed'"/></xsl:apply-templates></xsl:template>
<xsl:template match="v:a" mode="v:rows"><xsl:param name="p" required="yes"/><b><xsl:value-of select="$p || ':' || position() || '/' || last()"/></b></xsl:template>"##,
        &Default::default(),
    );
    assert_eq!(
        render(&bundle, "<r xmlns:v='urn:vars'><v:a/><v:a/></r>"),
        "<b>passed:1/2</b><b>passed:2/2</b>"
    );
}

#[test]
fn compiler_rejects_unsupported_or_invalid_template_contracts() {
    for body in [
        r#"<xsl:template name="x"/><xsl:template name="x"/>"#,
        r#"<xsl:template name="x" mode="m"/>"#,
        r#"<xsl:template match="a" priority="1e2"/>"#,
        r##"<xsl:template match="a" mode="#current"/>"##,
        r#"<xsl:template match="/" ><xsl:call-template name="missing"/></xsl:template>"#,
        r#"<xsl:template name="x"><xsl:param name="p"/><xsl:param name="p"/></xsl:template>"#,
        r#"<xsl:template name="x"><p/><xsl:param name="p"/></xsl:template>"#,
        r#"<xsl:template name="x"><xsl:param name="p" required="yes" select="1"/></xsl:template>"#,
        r#"<xsl:template match="/"><xsl:call-template name="x"/></xsl:template><xsl:template name="x"><xsl:param name="p" required="yes"/></xsl:template>"#,
        r#"<xsl:template match="/"><xsl:call-template name="x"><xsl:with-param name="p" select="1"/></xsl:call-template></xsl:template><xsl:template name="x"/>"#,
        r#"<xsl:template match="ancestor::a"/>"#,
        r#"<xsl:template match="element(a)"/>"#,
        r#"<xsl:template match="document-node(element(a))"/>"#,
    ] {
        let errors = compile_xslt_bundle_with_options(
            &stylesheet(body),
            "memory:match.xslt",
            &Default::default(),
        )
        .err()
        .expect(body);
        assert!(
            errors.iter().any(|d| d.severity.is_hard_violation()
                && d.uri.as_deref() == Some("memory:match.xslt")
                && d.source_map.is_some()),
            "{body}: {errors:?}"
        );
    }
}

#[test]
fn preflight_is_closed_hash_checked_and_does_not_authorize_runtime_uris() {
    let root = r#"<xsl:import href="lib"/><xsl:template match="/"/>"#;
    let mut options = XsltCompileOptions {
        modules: vec![module(
            "memory:match.xslt",
            "lib",
            "memory:lib",
            r#"<xsl:template name="entry"/>"#,
        )],
        ..Default::default()
    };
    assert!(compile_xslt_bundle_with_options(
        &stylesheet(root),
        "memory:match.xslt",
        &Default::default()
    )
    .is_err());
    compile(root, &options);
    for construct in [
        r#"<xsl:value-of select="document('blocked')"/>"#,
        r#"<xsl:result-document href="blocked"/>"#,
    ] {
        let source = stylesheet(&format!(
            r#"<xsl:import href="lib"/><xsl:template match="/">{construct}</xsl:template>"#
        ));
        let errors = compile_xslt_bundle_with_options(&source, "memory:match.xslt", &options)
            .err()
            .expect("must reject");
        assert!(
            errors
                .iter()
                .any(|d| d.code == "cem.xslt.external_uri_rejected"),
            "{errors:?}"
        );
    }
    options.modules[0].source.push(' ');
    assert!(
        compile_xslt_bundle_with_options(&stylesheet(root), "memory:match.xslt", &options).is_err()
    );
    options.modules = vec![module(
        "memory:match.xslt",
        "lib",
        "memory:lib",
        r#"<xsl:include href="self"/><xsl:template name="entry"/>"#,
    )];
    options.modules.push(module(
        "memory:lib",
        "self",
        "memory:lib",
        r#"<xsl:include href="self"/><xsl:template name="entry"/>"#,
    ));
    assert!(
        compile_xslt_bundle_with_options(&stylesheet(root), "memory:match.xslt", &options).is_err()
    );
}

#[test]
fn imported_expression_errors_retain_owner_and_recursion_is_bounded() {
    let options = XsltCompileOptions {
        modules: vec![module(
            "memory:match.xslt",
            "lib",
            "memory:lib",
            r#"<xsl:template name="bad"><xsl:value-of select="1 div 0"/></xsl:template>"#,
        )],
        ..Default::default()
    };
    let bundle = compile(
        r#"<xsl:import href="lib"/><xsl:template match="/"><p>prefix<xsl:call-template name="bad"/></p></xsl:template>"#,
        &options,
    );
    let data = TemplateData::default().with_binding(
        "document",
        ItemStream::once(imported_cem_tree(
            import_data("<r/>", "xml", "cem", "memory:input").unwrap(),
        )),
    );
    let plan = bundle.render(&data);
    assert!(plan.nodes.is_empty());
    assert!(
        plan.diagnostics
            .iter()
            .any(|d| d.uri.as_deref() == Some("memory:lib") && d.source_map.is_some()),
        "{:?}",
        plan.diagnostics
    );
    let bundle = compile(
        r#"<xsl:template match="/"><xsl:call-template name="loop"/></xsl:template><xsl:template name="loop"><p>prefix</p><xsl:call-template name="loop"/></xsl:template>"#,
        &Default::default(),
    );
    let plan = bundle.render(&data);
    assert!(plan.nodes.is_empty());
    assert!(!plan.diagnostics.is_empty());
}

#[test]
fn caller_locals_are_not_visible_in_a_named_callee() {
    let bundle = compile(
        r#"<xsl:template match="/"><xsl:variable name="hidden" select="1"/><xsl:call-template name="x"/></xsl:template><xsl:template name="x"><xsl:value-of select="$hidden"/></xsl:template>"#,
        &Default::default(),
    );
    let data = TemplateData::default().with_binding(
        "document",
        ItemStream::once(imported_cem_tree(
            import_data("<r/>", "xml", "cem", "memory:input").unwrap(),
        )),
    );
    let plan = bundle.render(&data);
    assert!(plan.nodes.is_empty());
    assert!(!plan.diagnostics.is_empty());
}

#[test]
fn explicit_host_parameters_are_native_and_binding_names_are_validated() {
    let source = stylesheet(
        r#"<xsl:template name="entry"><xsl:param name="p" required="yes"/><p><xsl:value-of select="$p"/></p></xsl:template>"#,
    );
    let mut options = XsltCompileOptions {
        entrypoint: Some(XPathExpandedName::unqualified("entry")),
        parameters: std::collections::BTreeMap::from([(
            XPathExpandedName::unqualified("p"),
            "host_param_0".into(),
        )]),
        ..Default::default()
    };
    let compiled =
        compile_xslt_bundle_with_options(&source, "memory:match.xslt", &options).unwrap();
    let bundle = XsltBundle::from_bytes(
        &compiled.bytes,
        &compiled.content_hash,
        &compiled.source_hash,
    )
    .unwrap();
    let data = TemplateData::default()
        .with_binding(
            "document",
            ItemStream::once(imported_cem_tree(
                import_data("<r/>", "xml", "cem", "memory:input").unwrap(),
            )),
        )
        .with_binding(
            "host_param_0",
            ItemStream::once(cem_ql::eval::Item::Atomic(cem_ql::eval::AtomValue::String(
                "Intro".into(),
            ))),
        );
    let plan = bundle.render(&data);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert_eq!(render_plan_to_html(&plan), "<p>Intro</p>");
    for invalid in ["document", "xslt_context", "a + 1", "bad'name", "if"] {
        options
            .parameters
            .insert(XPathExpandedName::unqualified("p"), invalid.into());
        assert!(compile_xslt_bundle_with_options(&source, "memory:match.xslt", &options).is_err());
    }
}

#[test]
fn compiled_dispatch_observes_host_cancellation_without_partial_output() {
    let bundle = compile(
        r#"<xsl:template match="/"><p>before</p><xsl:apply-templates/></xsl:template>"#,
        &Default::default(),
    );
    let data = TemplateData::default().with_binding(
        "document",
        ItemStream::once(imported_cem_tree(
            import_data("<r><a>A</a></r>", "xml", "cem", "memory:input").unwrap(),
        )),
    );
    let control = cem_ml::operation_control::OperationControl::default();
    control.cancel_root(None, None).unwrap();
    let plan = bundle.render_with_control(
        &data,
        &control,
        cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
    );
    assert!(plan.nodes.is_empty());
    assert!(!plan.diagnostics.is_empty());
}
