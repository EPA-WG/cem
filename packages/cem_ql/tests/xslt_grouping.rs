//! XSLT-GROUP-CONTEXT: native prerequisites and pending grouping contracts.
use cem_ml::import::import_data;
use cem_ql::{
    eval::{imported_cem_tree, ItemStream},
    render::{render_plan_to_html, RenderPlan, TemplateData},
    xslt::{compiler::compile_xslt_bundle, XsltBundle},
};

fn compile(body: &str) -> XsltBundle {
    let source = format!(
        r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xs="http://www.w3.org/2001/XMLSchema" version="3.0">{body}</xsl:stylesheet>"#
    );
    let compiled = compile_xslt_bundle(&source, "memory:group.xslt").unwrap();
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
            import_data(xml, "xml", "cem", "memory:group-input.xml").unwrap(),
        )),
    ))
}

fn render(bundle: &XsltBundle, xml: &str) -> String {
    let plan = plan(bundle, xml);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    render_plan_to_html(&plan)
}

#[test]
fn existing_xpath_operations_compare_group_keys_without_cem_identity() {
    // XSLT 3.0 §14.1/§14.5: untyped values become strings, NaN equals NaN,
    // unlike types stay separate, and numeric promotion is not transitive.
    // Pairwise distinct-values supplies this equality for supported atoms;
    // map same-key and CEM item identity are different contracts.
    let bundle = compile(
        r#"<xsl:template match="/">
<p><xsl:value-of select="
count(distinct-values((xs:float('1'), xs:decimal('1.0000000000100000000001')))) = 1,
count(distinct-values((xs:decimal('1.0000000000100000000001'), xs:double('1.00000000001')))) = 1,
count(distinct-values((xs:float('1'), xs:double('1.00000000001')))) = 1,
count(distinct-values((xs:double('NaN'), xs:float('NaN')))) = 1,
count(distinct-values(('1', 1))) = 1,
count(distinct-values((xs:untypedAtomic('1'), '1'))) = 1"/></p>
</xsl:template>"#,
    );
    assert_eq!(
        render(&bundle, "<r/>"),
        "<p>true true false true false true</p>"
    );
}

#[test]
fn existing_xpath_arrays_preserve_members_and_first_seen_keys() {
    // A native prerequisite, not an implementation of xsl:for-each-group.
    // Keys are atomized at XPath level; original node identity/axes survive.
    let bundle = compile(
        r#"<xsl:template match="/">
<xsl:variable name="rows" select="array { for $row in /*/* return [$row, data($row/k) ! string(.)] }"/>
<xsl:variable name="keys" select="array { distinct-values(for $row in $rows?* return $row?(2)) }"/>
<xsl:variable name="first" select="/*/*[1]"/>
<xsl:for-each select="1 to array:size($keys)">
<xsl:variable name="key" select="$keys?(.)"/>
<xsl:variable name="members" select="for $row in $rows?* return if ($row?(2) = $key) then $row?(1) else ()"/>
<p><xsl:value-of select="$key, $members/@id, $members[1] is $first, local-name($members[1]/parent::*)" separator="|"/></p>
</xsl:for-each>
</xsl:template>"#,
    );
    assert_eq!(
        render(&bundle, "<r><row id='a'><k>B</k><k>B</k><k>A</k></row><row id='b'/><row id='c'><k>A</k></row></r>"),
        "<p>B|a|true|r</p><p>A|a|c|true|r</p>"
    );
    assert_eq!(
        render(&bundle, "<r><row id='d'><k>C</k></row></r>"),
        "<p>C|d|true|r</p>"
    );
    assert_eq!(render(&bundle, "<r/>"), "");
}

#[test]
#[ignore = "XSLT-GROUP-CONTEXT: pending shared XPath group-context decision and grouping lowering"]
fn stylesheet_derives_repeated_rows_and_union_headings() {
    let bundle = compile(
        r#"<xsl:template match="/">
<xsl:for-each-group select="/*/*" group-by="namespace-uri(.) || '|' || local-name(.)">
<xsl:if test="count(current-group()) gt 1"><table><tr>
<xsl:for-each-group select="current-group()/(@* | *)" group-by="local-name(.)">
<th><xsl:value-of select="current-grouping-key()"/></th>
</xsl:for-each-group>
</tr><xsl:for-each select="current-group()"><tr><td><xsl:value-of select="@id"/></td></tr></xsl:for-each></table></xsl:if>
</xsl:for-each-group>
</xsl:template>"#,
    );
    assert_eq!(
        render(&bundle, "<r><row id='a'><name>A</name></row><note/><row id='b'><age>2</age><name>B</name></row></r>"),
        "<table><tr><th>id</th><th>name</th><th>age</th></tr><tr><td>a</td></tr><tr><td>b</td></tr></table>"
    );
    assert_eq!(
        render(&bundle, "<r><entry id='c'><city>C</city></entry><entry id='d'><extra>D</extra></entry></r>"),
        "<table><tr><th>id</th><th>city</th><th>extra</th></tr><tr><td>c</td></tr><tr><td>d</td></tr></table>"
    );
    assert_eq!(render(&bundle, "<r><only/></r>"), "");
}

#[test]
#[ignore = "XSLT-GROUP-CONTEXT: pending shared XPath group-context decision and grouping lowering"]
fn group_context_crosses_templates_and_restores_after_nested_groups() {
    let bundle = compile(
        r#"<xsl:template match="/">
<xsl:for-each-group select="/*/*" group-by="position() mod 2">
<p><xsl:value-of select="current-grouping-key(), position(), last(), @id, current-group()/@id" separator="|"/></p>
<xsl:call-template name="group"/>
<xsl:for-each-group select="current-group()" group-by="last()"><i><xsl:value-of select="current-grouping-key(), current-group()/@id" separator="|"/></i></xsl:for-each-group>
<xsl:apply-templates select="current-group()[1]"/>
</xsl:for-each-group>
</xsl:template>
<xsl:template name="group"><b><xsl:value-of select="current-grouping-key(), current-group()/@id" separator="|"/></b></xsl:template>
<xsl:template match="row"><xsl:call-template name="group"/></xsl:template>"#,
    );
    assert_eq!(
        render(&bundle, "<r><row id='a'/><row id='b'/><row id='c'/></r>"),
        "<p>1|1|2|a|a|c</p><b>1|a|c</b><i>2|a|c</i><b>1|a|c</b><p>0|2|2|b|b</p><b>0|b</b><i>1|b</i><b>0|b</b>"
    );
}

#[test]
#[ignore = "XSLT-GROUP-CONTEXT: pending shared XPath group-context decision and grouping lowering"]
fn grouping_supports_multiple_keys_duplicate_population_positions_and_empty_keys() {
    let bundle = compile(
        r#"<xsl:template match="/">
<xsl:for-each-group select="(/*/*[1], /*/*)" group-by="k">
<p><xsl:value-of select="current-grouping-key(), current-group()/@id, count(current-group())" separator="|"/></p>
</xsl:for-each-group>
</xsl:template>"#,
    );
    // The group contains both occurrences of row a. XPath /@id removes the
    // duplicate node, while count(current-group()) preserves population size.
    assert_eq!(
        render(&bundle, "<r><row id='a'><k>B</k><k>B</k><k>A</k></row><row id='b'/><row id='c'><k>A</k></row></r>"),
        "<p>B|a|2</p><p>A|a|c|3</p>"
    );
}

#[test]
#[ignore = "XSLT-GROUP-CONTEXT: pending shared XPath group-context decision and grouping lowering"]
fn absent_group_errors_are_lazy_and_function_calls_do_not_capture_group_context() {
    let lazy = compile(
        r#"<xsl:template match="/"><p><xsl:value-of select="if (false()) then current-group() else 'ok'"/></p></xsl:template>"#,
    );
    assert_eq!(render(&lazy, "<r/>"), "<p>ok</p>");
    for (expression, code) in [
        ("current-group()", "XTDE1061"),
        ("current-grouping-key()", "XTDE1071"),
        ("(function() { current-group() })()", "XTDE1061"),
        ("(function() { current-grouping-key() })()", "XTDE1071"),
    ] {
        let instruction = format!(r#"<p>prefix<xsl:value-of select="{expression}"/></p>"#);
        let body = if expression.starts_with("(function") {
            format!(
                r#"<xsl:for-each-group select="/*/*" group-by="'one'">{instruction}</xsl:for-each-group>"#
            )
        } else {
            instruction
        };
        let bundle = compile(&format!(r#"<xsl:template match="/">{body}</xsl:template>"#));
        let result = plan(&bundle, "<r><row/></r>");
        assert!(result.nodes.is_empty());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| (d.code.contains(code) || d.message.contains(code))
                    && d.uri.as_deref() == Some("memory:group.xslt")
                    && d.source_map.is_some()),
            "{expression}: {:?}",
            result.diagnostics
        );
    }
}
