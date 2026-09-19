//! XSLT-OUTPUT-LOWERING / XSLT-OUTPUT-WASM: native construction after bundle reload.
use cem_ml::import::import_data;
use cem_ql::{
    eval::{imported_cem_tree, ItemStream},
    render::{render_plan_to_html, RenderPlan, TemplateData},
    xslt::{compiler::compile_xslt_bundle, XsltBundle},
};

fn stylesheet(body: &str) -> String {
    format!(
        r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:err="http://www.w3.org/2005/xqt-errors" xmlns:xs="http://www.w3.org/2001/XMLSchema" version="3.0">{body}</xsl:stylesheet>"#
    )
}
fn bundle(body: &str) -> XsltBundle {
    let compiled = compile_xslt_bundle(&stylesheet(body), "memory:output.xslt").unwrap();
    XsltBundle::from_bytes(
        &compiled.bytes,
        &compiled.content_hash,
        &compiled.source_hash,
    )
    .unwrap()
}
fn plan(bundle: &XsltBundle, xml: &str) -> RenderPlan {
    bundle.render(&TemplateData::default().with_binding(
        "document",
        ItemStream::once(imported_cem_tree(
            import_data(xml, "xml", "cem", "memory:output-input.xml").unwrap(),
        )),
    ))
}
fn html(bundle: &XsltBundle, xml: &str) -> String {
    let plan = plan(bundle, xml);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    render_plan_to_html(&plan)
}

#[test]
fn selected_documents_and_subtrees_remain_nodes_after_reload_and_changed_input() {
    let body = r#"<xsl:template match="/"><main><xsl:try select="parse-xml(string(/*))"><xsl:catch errors="err:FODC0006" select="json-to-xml(string(/*))"/></xsl:try></main></xsl:template>"#;
    let bundle = bundle(body);
    assert_eq!(
        html(
            &bundle,
            "<input>&lt;r a='1'&gt;&lt;b/&gt;&lt;!--end--&gt;&lt;?test data?&gt;&lt;/r&gt;</input>"
        ),
        "<main><r a=\"1\"><b></b><!--end--><?test data?></r></main>"
    );
    assert_eq!(html(&bundle, "<input>{\"a\":null,\"b\":\"\"}</input>"), "<main><map xmlns=\"http://www.w3.org/2005/xpath-functions\"><null key=\"a\"></null><string key=\"b\"></string></map></main>");
    export_fixture(body, "native-output");
}

#[test]
fn atomic_spacing_survives_calls_loops_arrays_and_successful_recovery() {
    let bundle = bundle(
        r#"<xsl:template match="/"><p><xsl:sequence select="1"/><xsl:call-template name="two"/><xsl:text>x</xsl:text><xsl:for-each select="3,4"><xsl:sequence select="."/></xsl:for-each><xsl:try select="parse-xml('bad')"><xsl:catch select="[5,[6,7]]"/></xsl:try></p></xsl:template><xsl:template name="two"><xsl:sequence select="2"/></xsl:template>"#,
    );
    assert_eq!(html(&bundle, "<r/>"), "<p>1 2x3 4 5 6 7</p>");
    let bundle = self::bundle(
        r#"<xsl:template match="/"><p><xsl:sequence select="1, parse-xml(''), 2"/></p></xsl:template>"#,
    );
    assert!(!plan(&bundle, "<r/>").diagnostics.is_empty());
}

#[test]
fn parent_construction_owns_order_errors_and_recovery_discards_partial_output() {
    let bundle = bundle(
        r#"<xsl:template match="/"><xsl:try><p><xsl:attribute name="a">first</xsl:attribute><xsl:attribute name="a">last</xsl:attribute><xsl:text>body</xsl:text><xsl:sequence select="/*/@late"/></p><xsl:catch errors="err:XTDE0410"><b><xsl:value-of select="$err:code, $err:module, exists($err:line-number)" separator="|"/></b></xsl:catch></xsl:try></xsl:template>"#,
    );
    assert_eq!(html(&bundle, "<r/>"), "<p a=\"last\">body</p>");
    assert_eq!(
        html(&bundle, "<r late='bad'/>"),
        "<b>err:XTDE0410|memory:output.xslt|true</b>"
    );
    let bundle = self::bundle(
        r#"<xsl:template match="/"><p><xsl:try select="map {}"><xsl:catch><b>too early</b></xsl:catch></xsl:try></p></xsl:template>"#,
    );
    let failed = plan(&bundle, "<r/>");
    assert!(failed.nodes.is_empty());
    assert!(
        failed
            .diagnostics
            .iter()
            .any(|d| d.error_name().is_some_and(|n| n.local_name == "XTDE0450")),
        "{:?}",
        failed.diagnostics
    );
}

#[test]
fn avt_whitespace_and_binding_attributes_are_lowered_without_browser_logic() {
    let body = r#"<xsl:template match="/"><p title="{{{string(/*)}}}:{1,2}" slice="chosen" slice-event="click" slice-value="$target.value" xml:space="preserve"> <b xml:space="default"> </b> <xsl:value-of select="/*"/> </p></xsl:template>"#;
    let bundle = bundle(body);
    assert_eq!(html(&bundle, "<r>A &amp; B</r>"), "<p title=\"{A &amp; B}:1 2\" slice=\"chosen\" slice-event=\"click\" slice-value=\"$target.value\" xml:space=\"preserve\"> <b xml:space=\"default\"></b> A &amp; B </p>");
    assert!(html(&bundle, "<r>changed</r>").contains("{changed}:1 2"));
    export_fixture(body, "avt-output");
}

#[test]
fn namespaces_fix_up_by_expanded_name_and_reset_default_namespace() {
    let bundle = bundle(
        r#"<xsl:template match="/"><outer xmlns="urn:outer" xmlns:a="urn:attr" a:id="one"><xsl:copy-of select="/*"/><inner xmlns=""/></outer></xsl:template>"#,
    );
    assert_eq!(html(&bundle, "<r xmlns='urn:input' xmlns:b='urn:attr' b:id='two'><plain xmlns=''/></r>"), "<outer xmlns=\"urn:outer\" xmlns:a=\"urn:attr\" a:id=\"one\"><r xmlns=\"urn:input\" xmlns:ns1=\"urn:attr\" ns1:id=\"two\"><plain xmlns=\"\"></plain></r><inner xmlns=\"\"></inner></outer>");
}

#[test]
fn static_styles_are_branch_output_and_unsupported_forms_have_source_locations() {
    let bundle = bundle(
        r#"<xsl:template match="/"><xsl:if test="/*/@show"><style>.card { color: red; }</style></xsl:if><p>body</p></xsl:template>"#,
    );
    assert_eq!(html(&bundle, "<r/>"), "<p>body</p>");
    assert_eq!(
        html(&bundle, "<r show='yes'/>"),
        "<style>.card { color: red; }</style><p>body</p>"
    );
    for body in [
        "<style><xsl:value-of select='1'/></style>",
        "<script>example()</script>",
        "<p xml:space='invalid'/>",
        "<xsl:sequence select='1'><b/></xsl:sequence>",
    ] {
        let errors = compile_xslt_bundle(
            &stylesheet(&format!("<xsl:template match='/'> {body} </xsl:template>")),
            "memory:output.xslt",
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .all(|d| d.uri.as_deref() == Some("memory:output.xslt") && d.source_map.is_some()),
            "{errors:?}"
        );
    }
}

#[test]
fn attribute_simple_content_atomizes_nodes_and_merges_adjacent_text() {
    let bundle = bundle(
        r#"<xsl:template match="/"><p><xsl:attribute name="a"><xsl:text>x</xsl:text><xsl:text>y</xsl:text><xsl:sequence select="/*/*, [1,[xs:double('1e0'),xs:boolean('1')]]"/></xsl:attribute></p></xsl:template>"#,
    );
    assert_eq!(
        html(&bundle, "<r><a>A</a><b>B</b></r>"),
        "<p a=\"xy A B 1 1 true\"></p>"
    );
    let bundle = self::bundle(
        r#"<xsl:template match="/"><p><xsl:sequence select="1"/><xsl:document/><xsl:sequence select="2"/></p></xsl:template>"#,
    );
    assert_eq!(html(&bundle, "<r/>"), "<p>12</p>");
}

#[test]
fn construction_errors_keep_original_offsets_and_escaped_source_uris() {
    let source = stylesheet(
        "<xsl:template match='/'>\n<p><xsl:sequence select='map{}'/></p></xsl:template>",
    );
    let uri = "memory:out'put{a}.xslt";
    let compiled = compile_xslt_bundle(&source, uri).unwrap();
    let bundle = XsltBundle::from_bytes(
        &compiled.bytes,
        &compiled.content_hash,
        &compiled.source_hash,
    )
    .unwrap();
    let failed = plan(&bundle, "<r/>");
    let error = failed
        .diagnostics
        .iter()
        .find(|d| d.error_name().is_some_and(|n| n.local_name == "XTDE0450"))
        .unwrap();
    assert_eq!(error.uri.as_deref(), Some(uri));
    assert_eq!(error.byte_offset, Some(source.find("<p>").unwrap() as u64));
    assert_eq!(error.line, Some(2));
    assert!(!error.source_map.as_ref().unwrap().frames.is_empty());
}

#[test]
fn static_constructors_resolve_element_defaults_and_explicit_namespaces() {
    let bundle = bundle(
        r#"<xsl:template match="/" xmlns="urn:default"><xsl:element name="p"><xsl:attribute name="a" select="1, 2"/><xsl:element name="other:x" namespace="urn:explicit"/><xsl:element name="other:y" namespace=""/></xsl:element></xsl:template>"#,
    );
    assert_eq!(html(&bundle, "<r/>"), "<p xmlns=\"urn:default\" a=\"1 2\"><other:x xmlns:other=\"urn:explicit\"></other:x><y xmlns=\"\"></y></p>");
    let bundle = self::bundle(
        r#"<xsl:template match="/"><style>.a::before { content: "&amp;&lt;"; }</style></xsl:template>"#,
    );
    assert_eq!(
        html(&bundle, "<r/>"),
        "<style>.a::before { content: \"&<\"; }</style>"
    );
}

fn export_fixture(body: &str, name: &str) {
    if let Ok(directory) = std::env::var("CEM_XSLT_OUTPUT_FIXTURE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        let source = stylesheet(body);
        let compiled = compile_xslt_bundle(&source, "memory:output.xslt").unwrap();
        std::fs::write(directory.join(format!("{name}.xslt")), source).unwrap();
        std::fs::write(directory.join(format!("{name}.bin")), compiled.bytes).unwrap();
        std::fs::write(directory.join(format!("{name}.json")), serde_json::to_vec(&serde_json::json!({"contentHash":compiled.content_hash.header_value(),"sourceHash":compiled.source_hash.header_value()})).unwrap()).unwrap();
    }
}
