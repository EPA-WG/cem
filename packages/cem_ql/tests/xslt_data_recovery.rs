//! XSLT-DATA-RECOVERY: native imports and ordered catches after bundle reload.
use cem_ml::import::import_data;
use cem_ql::{
    eval::{imported_cem_tree, ItemStream},
    render::{render_plan_to_html, RenderPlan, TemplateData},
    xslt::{compiler::compile_xslt_bundle, XsltBundle},
};

fn stylesheet(body: &str) -> String {
    format!(
        r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:err="http://www.w3.org/2005/xqt-errors" xmlns:e="http://www.w3.org/2005/xqt-errors" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:import="urn:cem:import" version="3.0">{body}</xsl:stylesheet>"#
    )
}
fn bundle(body: &str) -> XsltBundle {
    let compiled = compile_xslt_bundle(&stylesheet(body), "memory:data-recovery.xslt").unwrap();
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
            import_data(xml, "xml", "cem", "memory:data-input.xml").unwrap(),
        )),
    ))
}
fn render(bundle: &XsltBundle, xml: &str) -> String {
    let plan = plan(bundle, xml);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    render_plan_to_html(&plan)
}
fn export_fixture(body: &str, name: &str) {
    if let Ok(directory) = std::env::var("CEM_XSLT_DATA_FIXTURE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        let source = stylesheet(body);
        let compiled = compile_xslt_bundle(&source, "memory:data-recovery.xslt").unwrap();
        std::fs::write(directory.join(format!("{name}.xslt")), source).unwrap();
        std::fs::write(directory.join(format!("{name}.bin")), compiled.bytes).unwrap();
        // Explicit deployment control metadata; imported documents stay native.
        std::fs::write(directory.join(format!("{name}.json")), serde_json::to_vec(&serde_json::json!({"contentHash": compiled.content_hash.header_value(), "sourceHash": compiled.source_hash.header_value()})).unwrap()).unwrap();
    }
}

#[test]
fn changed_inputs_parse_as_native_nodes_and_catches_rollback_output() {
    let body = r#"<xsl:template match="/">
<xsl:try><b>before</b><xsl:variable name="parsed" select="if (/*/@format = 'json') then json-to-xml(string(/*)) else parse-xml(string(/*))"/>
<xsl:for-each select="$parsed/*/*"><p><xsl:value-of select="., position(), last()" separator="|"/></p></xsl:for-each>
<xsl:catch errors="err:FOJS0001"><em>json error</em></xsl:catch>
<xsl:catch errors="e:FODC0006"><i><xsl:value-of select="$err:code, $err:code instance of xs:QName, local-name(/*)" separator="|"/></i></xsl:catch>
<xsl:catch><b>wrong wildcard</b></xsl:catch></xsl:try>
</xsl:template>"#;
    let bundle = bundle(body);
    assert_eq!(
        render(
            &bundle,
            "<input><![CDATA[<r><x>A</x><x>B</x></r>]]></input>"
        ),
        "<b>before</b><p>A|1|2</p><p>B|2|2</p>"
    );
    assert_eq!(
        render(&bundle, "<input>bad XML</input>"),
        "<i>err:FODC0006|true|input</i>"
    );
    assert_eq!(
        render(&bundle, r#"<input format="json">{"A":1,"B":2}</input>"#),
        "<b>before</b><p>1|1|2</p><p>2|2|2</p>"
    );
    assert_eq!(
        render(&bundle, "<input format='json'>[</input>"),
        "<em>json error</em>"
    );
    export_fixture(body, "parsed-recovered");
}

#[test]
fn nested_catches_propagate_across_template_calls_and_restore_bindings() {
    let bundle = bundle(
        r#"<xsl:template match="/">
<xsl:variable name="v" select="'outer'"/>
<xsl:try><xsl:try><xsl:variable name="v" select="'inner'"/><xsl:call-template name="parse"/>
<xsl:catch errors="*:FODC0006"><xsl:value-of select="json-to-xml('bad')"/></xsl:catch>
<xsl:catch><b>must not catch sibling failure</b></xsl:catch></xsl:try>
<xsl:catch errors="err:*"><p><xsl:value-of select="$v, $err:code, $err:module, exists($err:line-number), exists($err:column-number), empty($err:value)" separator="|"/></p></xsl:catch>
</xsl:try><b><xsl:value-of select="$v"/></b></xsl:template>
<xsl:template name="parse"><p>discard</p><xsl:value-of select="parse-xml('bad')"/></xsl:template>"#,
    );
    assert_eq!(
        render(&bundle, "<r/>"),
        "<p>outer|err:FOJS0001|memory:data-recovery.xslt|true|true|true</p><b>outer</b>"
    );
}

#[test]
fn json_options_and_csv_yaml_extensions_use_the_common_node_path() {
    let bundle = bundle(
        r#"<xsl:template match="/">
<xsl:variable name="json" select="json-to-xml('{&quot;rows&quot;:[null,&quot;&quot;]}')"/>
<p><xsl:value-of select="$json/*/*/* ! local-name()" separator="|"/></p>
<p><xsl:value-of select="import:parse-csv('a,b&#10;one,two')/* ! local-name()"/></p>
<p><xsl:value-of select="import:parse-yaml('a: one')/* ! local-name()"/></p>
<xsl:try><xsl:value-of select="json-to-xml('{&quot;a&quot;:1,&quot;a&quot;:2}', map {'duplicates':'reject'})"/>
<xsl:catch errors="Q{http://www.w3.org/2005/xqt-errors}FOJS0003"><i>duplicate</i></xsl:catch></xsl:try>
</xsl:template>"#,
    );
    assert_eq!(
        render(&bundle, "<r/>"),
        "<p>null|string</p><p>array</p><p>object</p><i>duplicate</i>"
    );
}

#[test]
fn import_limits_and_unsupported_capabilities_cannot_be_caught() {
    for select in [
        "parse-xml('&lt;!DOCTYPE r&gt;&lt;r/&gt;')",
        "json-to-xml(string-join((1 to 33000) ! ' ', ''))",
    ] {
        let bundle = bundle(&format!(
            r#"<xsl:template match="/"><p>discard</p><xsl:try><xsl:value-of select="{select}"/><xsl:catch><b>must not recover</b></xsl:catch></xsl:try></xsl:template>"#
        ));
        let plan = plan(&bundle, "<r/>");
        assert_eq!(render_plan_to_html(&plan), "");
        assert!(!plan.diagnostics.is_empty());
    }
}

#[test]
fn catch_preserves_group_focus_and_does_not_leak_variables() {
    let groups = bundle(
        r#"<xsl:template match="/"><xsl:for-each-group select="/*/*" group-by="@g">
<xsl:try><xsl:value-of select="parse-xml('')"/><xsl:catch errors="Q{http://www.w3.org/2005/xqt-errors}*"><p><xsl:value-of select="current-grouping-key(), count(current-group()), position(), last(), @id" separator="|"/></p></xsl:catch></xsl:try>
</xsl:for-each-group></xsl:template>"#,
    );
    assert_eq!(
        render(
            &groups,
            "<r><x id='a' g='A'/><x id='b' g='B'/><x id='c' g='A'/></r>"
        ),
        "<p>A|2|1|2|a</p><p>B|1|2|2|b</p>"
    );
    let outside = bundle(
        r#"<xsl:template match="/"><xsl:try><xsl:value-of select="parse-xml('')"/><xsl:catch/></xsl:try><xsl:value-of select="$err:code"/></xsl:template>"#,
    );
    let plan = plan(&outside, "<r/>");
    assert_eq!(render_plan_to_html(&plan), "");
    assert!(plan
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.xpath.variable_unbound"));
}

#[test]
fn unmatched_errors_propagate_and_invalid_catch_forms_fail_statically() {
    let unmatched = bundle(
        r#"<xsl:template match="/"><xsl:try><p>discard</p><xsl:value-of select="parse-xml('')"/><xsl:catch errors="err:FOJS0001"><b>wrong</b></xsl:catch></xsl:try></xsl:template>"#,
    );
    let plan = plan(&unmatched, "<r/>");
    assert_eq!(render_plan_to_html(&plan), "");
    assert!(plan
        .diagnostics
        .iter()
        .any(|d| d.error_name().is_some_and(|n| n.local_name == "FODC0006")));
    for body in [
        "<xsl:try/>",
        "<xsl:try><xsl:catch/><p/></xsl:try>",
        "<xsl:try select='1'><p/><xsl:catch/></xsl:try>",
        "<xsl:try><xsl:catch select='1'><p/></xsl:catch></xsl:try>",
        "<xsl:try><xsl:catch errors='missing:ERROR'/></xsl:try>",
        "<xsl:try><xsl:catch errors='err:X[1]'/></xsl:try>",
        "<xsl:try rollback-output='invalid'><xsl:catch/></xsl:try>",
    ] {
        let source = stylesheet(&format!("<xsl:template match='/'> {body} </xsl:template>"));
        let diagnostics = compile_xslt_bundle(&source, "memory:invalid-catch.xslt").unwrap_err();
        assert!(
            diagnostics.iter().all(|d| d.source_map.is_some()),
            "{diagnostics:?}"
        );
    }
}
