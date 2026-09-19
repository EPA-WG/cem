//! XSLT-VIEW-OPTIONAL-FOCUS: absent initial focus remains a native dynamic state.
use cem_ml::validation::xpath::XPathExpandedName;
use cem_ql::{
    render::{render_plan_to_html, TemplateData},
    xslt::{
        compiler::{compile_xslt_bundle_with_options, XsltCompileOptions},
        XsltBundle,
    },
};

fn source(body: &str, templates: &str) -> String {
    format!(
        r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:err="http://www.w3.org/2005/xqt-errors" version="3.0"><xsl:template name="main">{body}</xsl:template>{templates}</xsl:stylesheet>"#
    )
}
fn compile(source: &str) -> cem_ql::xslt::compiler::CompiledXsltBundle {
    compile_xslt_bundle_with_options(
        source,
        "memory:optional-focus.xslt",
        &XsltCompileOptions {
            entrypoint: Some(XPathExpandedName::unqualified("main")),
            ..Default::default()
        },
    )
    .unwrap()
}
fn bundle(source: &str) -> XsltBundle {
    let compiled = compile(source);
    XsltBundle::from_bytes(
        &compiled.bytes,
        &compiled.content_hash,
        &compiled.source_hash,
    )
    .unwrap()
}

#[test]
fn named_calls_preserve_absence_and_loops_establish_and_restore_focus() {
    let source = source(
        r#"<p><xsl:value-of select="'scalar'"/><xsl:for-each select="(2, 4)"><xsl:call-template name="item"/></xsl:for-each><xsl:for-each select="()"><xsl:value-of select="."/></xsl:for-each><xsl:call-template name="missing"/></p>"#,
        r#"<xsl:template name="item"><xsl:value-of select="':' || . || '/' || position() || '/' || last()"/></xsl:template><xsl:template name="missing"><xsl:try><xsl:value-of select="."/><xsl:catch errors="err:XPDY0002"><xsl:text>:absent</xsl:text></xsl:catch></xsl:try></xsl:template>"#,
    );
    let plan = bundle(&source).render(&TemplateData::default());
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert_eq!(
        render_plan_to_html(&plan),
        "<p>scalar:2/1/2:4/2/2:absent</p>"
    );
    if let Some(directory) = std::env::var_os("CEM_XSLT_OPTIONAL_FOCUS_FIXTURE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        let compiled = compile(&source);
        std::fs::write(directory.join("optional-focus.xslt"), &source).unwrap();
        std::fs::write(directory.join("optional-focus.bin"), compiled.bytes).unwrap();
        std::fs::write(directory.join("optional-focus.json"), serde_json::to_vec(&serde_json::json!({
            "contentHash": compiled.content_hash.header_value(), "sourceHash": compiled.source_hash.header_value(),
            "html": render_plan_to_html(&plan)
        })).unwrap()).unwrap();
    }
}

#[test]
fn missing_focus_errors_are_lazy_typed_and_recoverable() {
    let literal = bundle(&source("<p>literal</p>", "")).render(&TemplateData::default());
    assert!(literal.diagnostics.is_empty(), "{:?}", literal.diagnostics);
    assert_eq!(render_plan_to_html(&literal), "<p>literal</p>");
    for expression in [".", "position()", "last()", "string()", "local-name()"] {
        let source = source(
            &format!(
                r#"<xsl:try><p>partial<xsl:value-of select="{expression}"/></p><xsl:catch errors="err:XPDY0002"><p>absent</p></xsl:catch></xsl:try>"#
            ),
            "",
        );
        let plan = bundle(&source).render(&TemplateData::default());
        assert!(
            plan.diagnostics.is_empty(),
            "{expression}: {:?}",
            plan.diagnostics
        );
        assert_eq!(render_plan_to_html(&plan), "<p>absent</p>", "{expression}");
    }
    let source = source(
        r#"<p><xsl:value-of select="if (true()) then 'safe' else string(.)"/></p>"#,
        "",
    );
    let plan = bundle(&source).render(&TemplateData::default());
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert_eq!(render_plan_to_html(&plan), "<p>safe</p>");
}

#[test]
fn sequence_forwarding_preserves_native_nodes_arrays_and_empty_members_without_focus() {
    let calls = r#"<xsl:call-template name="forward"><xsl:with-param name="value" select="$value"/></xsl:call-template>"#.repeat(140);
    let source = source(
        &format!(
            r#"<xsl:variable name="value" select="map {{'node':parse-xml('&lt;r&gt;kept&lt;/r&gt;'), 'array':[(),(2,3)]}}"/>{calls}"#
        ),
        r#"<xsl:template name="forward"><xsl:param name="value"/><xsl:variable name="alias" select="$value"/>
            <p><xsl:value-of select="($alias?node is $value?node, string($alias?node), array:size($alias?array), count($alias?array?1), $alias?array?2)"/></p>
        </xsl:template>"#,
    );
    let plan = bundle(&source).render(&TemplateData::default());
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert_eq!(
        render_plan_to_html(&plan),
        "<p>true kept 2 0 2 3</p>".repeat(140)
    );
}
