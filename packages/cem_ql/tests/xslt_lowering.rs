//! XSLT-LOWER-CORE: compile stylesheet instructions, never runtime documents.
use cem_ml::{content_cache::ContentHash, import::import_data};
use cem_ql::{
    eval::{imported_cem_tree, ItemStream},
    render::{render_plan_to_html, TemplateData},
    xslt::{compiler::compile_xslt_bundle, XsltBundle},
};

fn stylesheet(body: &str) -> String {
    format!(
        r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:v="urn:variables" version="3.0"><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#
    )
}
fn load(source: &str) -> XsltBundle {
    let compiled = compile_xslt_bundle(source, "memory:lower.xslt").unwrap();
    XsltBundle::from_bytes(
        &compiled.bytes,
        &compiled.content_hash,
        &compiled.source_hash,
    )
    .unwrap()
}
fn data(source: &str, format: &str) -> TemplateData {
    TemplateData::default().with_binding(
        "document",
        ItemStream::once(imported_cem_tree(
            import_data(source, format, "cem", "memory:data").unwrap(),
        )),
    )
}
fn html(bundle: &XsltBundle, source: &str, format: &str) -> String {
    let output = bundle.render(&data(source, format));
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    render_plan_to_html(&output)
}

const LOOPS: &str = r#"<ul><xsl:for-each select="/*/*"><li><xsl:value-of select="string(.) || ':' || position() || '/' || last()"/><xsl:for-each select="(10, 20)"><b><xsl:value-of select=". || ':' || position() || '/' || last()"/></b></xsl:for-each><i><xsl:value-of select="position() || '/' || last()"/></i></li></xsl:for-each></ul>"#;

#[test]
fn lowered_loops_keep_runtime_focus_and_reload_across_formats() {
    let source = stylesheet(LOOPS);
    let compiled = compile_xslt_bundle(&source, "memory:lower.xslt").unwrap();
    assert_eq!(
        compiled.bytes,
        compile_xslt_bundle(&source, "memory:lower.xslt")
            .unwrap()
            .bytes
    );
    assert!(compiled.generated_cemt.contains("for-each"));
    assert!(compiled.generated_cemt.contains("native:call"));
    let bundle = load(&source);
    assert!(bundle
        .expressions()
        .iter()
        .all(|x| x.source_text.is_none() && x.tokens.is_empty()));
    for (input, format) in [
        ("<r><a>A</a><b>B</b></r>", "xml"),
        (r#"{"a":"A","b":"B"}"#, "json"),
        ("a: A\nb: B\n", "yaml"),
        ("v\nA\nB\n", "csv"),
    ] {
        assert_eq!(html(&bundle, input, format), "<ul><li>A:1/2<b>10:1/2</b><b>20:2/2</b><i>1/2</i></li><li>B:2/2<b>10:1/2</b><b>20:2/2</b><i>2/2</i></li></ul>");
    }
    assert_eq!(
        html(&bundle, "<r><a>C</a></r>", "xml"),
        "<ul><li>C:1/1<b>10:1/2</b><b>20:2/2</b><i>1/1</i></li></ul>"
    );
    assert_eq!(html(&bundle, "<r/>", "xml"), "<ul></ul>");
    if let Ok(directory) = std::env::var("CEM_XSLT_LOWER_FIXTURE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::write(directory.join("lowered.bin"), &compiled.bytes).unwrap();
        std::fs::write(directory.join("stylesheet.xslt"), &source).unwrap();
        // Explicit deployment metadata only; documents never become JS records.
        std::fs::write(directory.join("manifest.json"), serde_json::to_vec(&serde_json::json!({
            "contentHash": compiled.content_hash.header_value(), "sourceHash": compiled.source_hash.header_value(),
        })).unwrap()).unwrap();
    }
}

#[test]
fn variables_keep_native_maps_arrays_nodes_and_lexical_scope() {
    let bundle = load(&stylesheet(
        r#"<xsl:variable name="v:saved" select="/*/*[position() le last()]"/><xsl:variable name="bag" select="map {'rows': [(), $v:saved, ('x','y')]}"/><p><xsl:value-of select="count($bag?rows?(1))"/>:<xsl:value-of select="count($bag?rows?(2))"/>:<xsl:value-of select="$bag?rows?(3)" separator="|"/></p><xsl:for-each select="$v:saved"><xsl:variable name="v:saved" select="map {'node': ., 'n': position()}"/><b><xsl:value-of select="string($v:saved?node) || $v:saved?n"/></b></xsl:for-each><i><xsl:value-of select="count($v:saved)"/></i>"#,
    ));
    assert_eq!(
        html(&bundle, "<r><a>A</a><b>B</b></r>", "xml"),
        "<p>0:2:x|y</p><b>A1</b><b>B2</b><i>2</i>"
    );
}

#[test]
fn conditionals_use_xpath_effective_boolean_values_and_selected_branches_only() {
    let bundle = load(&stylesheet(
        r#"<p><xsl:if test="'false'">text</xsl:if><xsl:if test="false()">bad</xsl:if><xsl:if test="/*/*">node</xsl:if><xsl:choose><xsl:when test="count(/*/*) = 1">one</xsl:when><xsl:when test="true()">many</xsl:when><xsl:otherwise><xsl:value-of select="1 div 0"/></xsl:otherwise></xsl:choose></p>"#,
    ));
    assert_eq!(html(&bundle, "<r><a/></r>", "xml"), "<p>textnodeone</p>");
    assert_eq!(html(&bundle, "<r/>", "xml"), "<p>textmany</p>");
}

#[test]
fn simple_content_merges_adjacent_text_and_preserves_atomic_separators() {
    let bundle = load(&stylesheet(
        r#"<p><xsl:value-of select="(1, true(), (), 3)"/></p><b><xsl:value-of select="(/*/a/text(), /*/b/text(), 'X', /*/a/text(), [], /*/b/text())" separator="|"/></b><i title="&quot;it's&quot; }} {{ &amp; &#xA;" data-empty=""><xsl:text> {literal} &amp; &#x1F352; } ' " \ </xsl:text></i>"#,
    ));
    assert_eq!(
        html(&bundle, "<r><a>A</a><b>B</b></r>", "xml"),
        concat!(
            r#"<p>1 true 3</p><b>AB|X|A|B</b><i title="&quot;it's&quot; } { &amp; "#,
            "\n",
            r#"" data-empty=""> {literal} &amp; 🍒 } ' " \ </i>"#
        )
    );
}

#[test]
fn xpath_errors_keep_stylesheet_locations_and_discard_partial_output() {
    for body in [
        r#"<p>prefix<xsl:if test="map {'a':1}">bad</xsl:if>suffix</p>"#,
        r#"<p>prefix<xsl:value-of select="map {'a':1}"/>suffix</p>"#,
        r#"<p>prefix<xsl:value-of select="current-dateTime()"/>suffix</p>"#,
    ] {
        let bundle = load(&stylesheet(body));
        let plan = bundle.render(&data("<r/>", "xml"));
        assert!(plan.nodes.is_empty(), "{plan:?}");
        assert!(
            plan.diagnostics
                .iter()
                .any(|d| d.uri.as_deref() == Some("memory:lower.xslt")
                    && d.byte_offset.is_some_and(|n| n > 0)
                    && d.source_map.is_some()),
            "{:?}",
            plan.diagnostics
        );
    }
}

#[test]
fn unsupported_or_malformed_instructions_fail_with_original_locations() {
    for body in [
        r#"<xsl:apply-imports/>"#,
        r#"<xsl:variable name="bad:name" select="1"/>"#,
        r#"<xsl:variable name="x + 1" select="1"/>"#,
        r#"<xsl:for-each/>"#,
        r#"<xsl:variable name="x"><p/></xsl:variable>"#,
        r#"<xsl:perform-sort select="/*/*"><xsl:sort select="."/></xsl:perform-sort>"#,
        r#"<xsl:value-of select="1 +"/>"#,
        r#"<xsl:value-of select="1" disable-output-escaping="yes"/>"#,
        r#"<p title="{position(}">invalid AVT</p>"#,
        r#"<xsl:choose><xsl:otherwise>A</xsl:otherwise><xsl:when test="true()">B</xsl:when></xsl:choose>"#,
    ] {
        let source = stylesheet(body);
        let errors = compile_xslt_bundle(&source, "memory:lower.xslt").err().expect("must reject");
        assert!(
            errors.iter().any(|d| d.severity.is_hard_violation()
                && d.uri.as_deref() == Some("memory:lower.xslt")
                && d.byte_offset.is_some()
                && d.source_map.is_some()),
            "{body}: {errors:?}"
        );
    }
    assert!(compile_xslt_bundle(
        &stylesheet("<xsl:template match='/'/>"),
        "memory:lower.xslt"
    )
    .is_err());
    assert_ne!(
        ContentHash::from_blake3(stylesheet(LOOPS).as_bytes()),
        ContentHash::from_blake3(b"runtime document")
    );
}

#[test]
fn compiler_limits_and_literal_text_compilation_are_input_independent() {
    let source = stylesheet("<p>plain<![CDATA[ ]]>&amp;<!--split-->&#32;</p>");
    let bundle = load(&source);
    assert_eq!(
        render_plan_to_html(&bundle.render(&data("<r/>", "xml"))),
        "<p>plain &amp;</p>"
    );
    for source in [
        stylesheet(&"x".repeat(128 * 1024)),
        stylesheet(&format!("{}text{}", "<b>".repeat(70), "</b>".repeat(70))),
        stylesheet(&"<xsl:value-of select='1'/>".repeat(129)),
    ] {
        assert!(compile_xslt_bundle(&source, "memory:lower.xslt").is_err());
    }
}

#[test]
fn compiler_locals_do_not_capture_author_variables() {
    let bundle = load(&stylesheet(
        r#"<xsl:variable name="input" select="'authored'"/><xsl:variable name="separator" select="'value'"/><xsl:variable name="Q{urn:variables}s" select="'scoped'"/><p><xsl:value-of select="($input, $separator, $v:s)" separator="|"/></p>"#,
    ));
    assert_eq!(html(&bundle, "<r/>", "xml"), "<p>authored|value|scoped</p>");
}
