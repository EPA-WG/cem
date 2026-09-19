//! XSLT-SORT-NATIVE: sorting operates on retained native values after import.
use cem_ml::import::import_data;
use cem_ql::{
    eval::{imported_cem_tree, ItemStream},
    render::{render_plan_to_html, RenderPlan, TemplateData},
    xslt::{compiler::compile_xslt_bundle, XsltBundle},
};

fn stylesheet(body: &str) -> String {
    format!(
        r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xs="http://www.w3.org/2001/XMLSchema" version="3.0">{body}</xsl:stylesheet>"#
    )
}
fn compile(body: &str) -> XsltBundle {
    let source = stylesheet(body);
    let compiled = compile_xslt_bundle(&source, "memory:sort.xslt").unwrap();
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
            import_data(xml, "xml", "cem", "memory:sort-input.xml").unwrap(),
        )),
    ))
}
fn render(bundle: &XsltBundle, xml: &str) -> String {
    let plan = plan(bundle, xml);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    render_plan_to_html(&plan)
}
fn export_fixture(body: &str, name: &str) {
    if let Ok(directory) = std::env::var("CEM_XSLT_SORT_FIXTURE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        let source = stylesheet(body);
        let compiled = compile_xslt_bundle(&source, "memory:sort.xslt").unwrap();
        std::fs::write(directory.join(format!("{name}.xslt")), source).unwrap();
        std::fs::write(directory.join(format!("{name}.bin")), compiled.bytes).unwrap();
        // Explicit bundle deployment metadata, never document data or an AST.
        std::fs::write(directory.join(format!("{name}.json")), serde_json::to_vec(&serde_json::json!({"contentHash": compiled.content_hash.header_value(), "sourceHash": compiled.source_hash.header_value()})).unwrap()).unwrap();
    }
}

#[test]
fn existing_xpath_sort_supports_common_promotion_and_stable_descending() {
    let bundle = compile(
        r#"<xsl:template match="/">
<xsl:variable name="records" select="([xs:decimal('16777217'), 'a'], [xs:decimal('16777216'), 'b'], [xs:float('0'), 'c'])"/>
<p><xsl:value-of select="sort($records, (), function($r) { xs:float($r?(1)) }) ! .?(2)"/></p>
<p><xsl:value-of select="reverse(sort(reverse($records), (), function($r) { xs:float($r?(1)) })) ! .?(2)"/></p>
</xsl:template>"#,
    );
    assert_eq!(render(&bundle, "<r/>"), "<p>c a b</p><p>a b c</p>");
}

#[test]
fn dynamic_multi_key_sort_preserves_ties_and_authored_validity_policy() {
    let body = r#"<xsl:template match="/">
<xsl:for-each select="/*/*">
<xsl:sort select="not(@n castable as xs:decimal)"/>
<xsl:sort select="if (@n castable as xs:decimal) then xs:decimal(@n) else 0" order="{/*/@order}"/>
<xsl:sort select="@label"/>
<p><xsl:value-of select="@id, position(), last(), local-name(parent::*)" separator="|"/></p>
</xsl:for-each></xsl:template>"#;
    let bundle = compile(body);
    let xml = "<r order='ascending'><row id='a' n='2' label='B'/><row id='b' n='10'/><row id='c' n='2' label='A'/><row id='d' n='02' label='A'/><row id='e' n='bad'/><row id='f'/></r>";
    assert_eq!(
        render(&bundle, xml),
        "<p>c|1|6|r</p><p>d|2|6|r</p><p>a|3|6|r</p><p>b|4|6|r</p><p>e|5|6|r</p><p>f|6|6|r</p>"
    );
    assert_eq!(
        render(&bundle, &xml.replace("ascending", "descending")),
        "<p>b|1|6|r</p><p>c|2|6|r</p><p>d|3|6|r</p><p>a|4|6|r</p><p>e|5|6|r</p><p>f|6|6|r</p>"
    );
    assert_eq!(render(&bundle, "<r order='ascending'/>"), "");
    export_fixture(body, "sorted");
}

#[test]
fn key_focus_is_original_and_controls_use_outer_focus() {
    let bundle = compile(
        r#"<xsl:template match="/">
<xsl:for-each select="/*/*">
<xsl:sort select="position() + last()" order="{if (position() = last()) then 'descending' else 'ascending'}"/>
<p><xsl:value-of select="@id, position(), last()" separator="|"/></p>
</xsl:for-each></xsl:template>"#,
    );
    assert_eq!(
        render(&bundle, "<r><row id='a'/><row id='b'/><row id='c'/></r>"),
        "<p>c|1|3</p><p>b|2|3</p><p>a|3|3</p>"
    );
}

#[test]
fn common_numeric_promotion_empty_nan_and_default_key_are_standard() {
    let bundle = compile(
        r#"<xsl:template match="/">
<xsl:for-each select="([xs:decimal('16777217'), 'a'], [xs:decimal('16777216'), 'b'], [xs:float('0'), 'c'], [(), 'empty'], [xs:double('NaN'), 'nan'])">
<xsl:sort select=".?(1)"/><p><xsl:value-of select=".?(2)"/></p>
</xsl:for-each>
<xsl:for-each select="(xs:decimal('16777217'), xs:decimal('16777216'), xs:float('0'))"><xsl:sort/><b><xsl:value-of select="."/></b></xsl:for-each>
</xsl:template>"#,
    );
    // The double NaN promotes the whole first key column to double; the
    // second population promotes to float, making its two decimals tied.
    assert_eq!(
        render(&bundle, "<r/>"),
        "<p>empty</p><p>nan</p><p>c</p><p>b</p><p>a</p><b>0</b><b>16777217</b><b>16777216</b>"
    );
}

#[test]
fn sorting_dispatch_and_groups_preserves_group_context_and_member_order() {
    let body = r#"<xsl:template match="/">
<xsl:for-each-group select="/*/*" group-by="@g">
<xsl:sort select="count(current-group())" order="descending"/>
<xsl:sort select="current-grouping-key()"/>
<p><xsl:value-of select="current-grouping-key(), position(), last(), current-group()/@id" separator="|"/></p>
<xsl:apply-templates select="current-group()"><xsl:sort select="@id" order="descending"/><xsl:with-param name="label" select="current-grouping-key()"/></xsl:apply-templates>
</xsl:for-each-group></xsl:template>
<xsl:template match="row"><xsl:param name="label"/><b><xsl:value-of select="$label, @id, position(), last(), current-group()/@id" separator="|"/></b></xsl:template>"#;
    let bundle = compile(body);
    assert_eq!(
        render(
            &bundle,
            "<r><row id='a' g='B'/><row id='b' g='A'/><row id='c' g='A'/></r>"
        ),
        "<p>A|1|2|b|c</p><b>A|c|1|2|b|c</b><b>A|b|2|2|b|c</b><p>B|2|2|a</p><b>B|a|1|1|a</b>"
    );
    export_fixture(body, "sorted-groups");
}

#[test]
fn malformed_sort_specs_fail_statically() {
    for (body, code) in [
        (r#"<xsl:sort select=".">text</xsl:sort>"#, "XTSE1015"),
        (r#"<xsl:sort/><xsl:sort stable="yes"/>"#, "XTSE1017"),
        (r#"<p/><xsl:sort/>"#, "cem.xslt.compile_content"),
        (r#"<xsl:sort lang="en"/>"#, "cem.xslt.compile_unsupported"),
        (r#"<xsl:sort order="sideways"/>"#, "XTSE0020"),
    ] {
        let source = stylesheet(&format!(
            r#"<xsl:template match="/"><xsl:for-each select="/*/*">{body}</xsl:for-each></xsl:template>"#
        ));
        let errors = compile_xslt_bundle(&source, "memory:sort.xslt").unwrap_err();
        assert!(
            errors
                .iter()
                .any(|d| d.code.contains(code) || d.message.contains(code)),
            "{errors:?}"
        );
        assert!(errors.iter().all(|d| d.source_map.is_some()));
    }
}

#[test]
fn sort_runtime_errors_discard_partial_output() {
    for (sort, code) in [
        (r#"<xsl:sort select="(1, 2)"/>"#, "XTTE1020"),
        (
            r#"<xsl:sort select="if (position() = 1) then 1 else 'a'"/>"#,
            "XTDE1030",
        ),
        (r#"<xsl:sort order="{/*/@order}"/>"#, "XTDE0030"),
        (r#"<xsl:sort collation="{'urn:unknown'}"/>"#, "XTDE1035"),
    ] {
        let bundle = compile(&format!(
            r#"<xsl:template match="/"><p>prefix</p><xsl:for-each select="/*/*">{sort}<b>bad</b></xsl:for-each></xsl:template>"#
        ));
        let result = plan(&bundle, "<r order='sideways'><row/><row/></r>");
        assert!(result.nodes.is_empty(), "{result:?}");
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code.contains(code) || d.message.contains(code)),
            "{code}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn conversion_controls_nan_infinities_empty_keys_and_stability() {
    let body = r#"<xsl:template match="/">
<xsl:for-each select="/*/*"><xsl:sort select="@n" data-type="{/*/@kind}" order=" { /*/@order } " stable="{ 'true' }" collation="{ 'http://www.w3.org/2005/xpath-functions/collation/codepoint' }"/><p><xsl:value-of select="@id"/></p></xsl:for-each>
</xsl:template>"#;
    let bundle = compile(body);
    let xml = "<r kind='number' order='ascending'><row id='a' n='2'/><row id='b' n='bad'/><row id='c'/><row id='d' n='NaN'/><row id='e' n='-INF'/><row id='f' n='INF'/><row id='g' n='02'/><row id='h' n=''/></r>";
    assert_eq!(
        render(&bundle, xml),
        "<p>b</p><p>c</p><p>d</p><p>h</p><p>e</p><p>a</p><p>g</p><p>f</p>"
    );
    assert_eq!(
        render(&bundle, &xml.replace("ascending", "descending")),
        "<p>f</p><p>a</p><p>g</p><p>e</p><p>b</p><p>c</p><p>d</p><p>h</p>"
    );
    assert_eq!(
        render(&bundle, &xml.replace("number", "text")),
        "<p>c</p><p>h</p><p>e</p><p>g</p><p>a</p><p>f</p><p>d</p><p>b</p>"
    );
    export_fixture(body, "sorted-conversions");
}

#[test]
fn sorting_preserves_native_containers_owners_and_author_namespace_bindings() {
    let bundle = compile(
        r#"<xsl:template match="/" xmlns:fn="urn:author" xmlns:array="urn:author-array">
<xsl:variable name="records" select="'authored'"/>
<xsl:variable name="index" select="1"/>
<xsl:variable name="first" select="/*/*[1]"/>
<xsl:for-each select="(/*/*[1], /*/*)"><xsl:sort select="@n" order="descending"/><p><xsl:value-of select="@id, . is $first, local-name(parent::*), $records" separator="|"/></p></xsl:for-each>
<xsl:for-each select="(map{'k': 2, 'node': $first}, map{'k': 1, 'node': $first})"><xsl:sort select=".?k"/><b><xsl:value-of select=".?k, .?node is $first" separator="|"/></b></xsl:for-each>
<xsl:for-each select="([2, (), $first], [1, (), $first])"><xsl:sort select=".?($index)"/><i><xsl:value-of select=".?(1), empty(.?(2)), .?(3) is $first" separator="|"/></i></xsl:for-each>
</xsl:template>"#,
    );
    assert_eq!(render(&bundle, "<r><row id='a' n='1'/><row id='b' n='2'/></r>"), "<p>b|false|r|authored</p><p>a|true|r|authored</p><p>a|true|r|authored</p><b>1|true</b><b>2|true</b><i>1|true|true</i><i>2|true|true</i>");
}

#[test]
fn one_sorting_bundle_consumes_all_shared_import_formats() {
    let body = r#"<xsl:template match="/"><xsl:for-each select="/*/*"><xsl:sort select="string(.)"/><p><xsl:value-of select="."/></p></xsl:for-each></xsl:template>"#;
    let bundle = compile(body);
    for (source, format) in [
        ("<r><a>B</a><b>A</b></r>", "xml"),
        (r#"{"a":"B","b":"A"}"#, "json"),
        ("a: B\nb: A\n", "yaml"),
        ("v\nB\nA\n", "csv"),
    ] {
        let result = bundle.render(&TemplateData::default().with_binding(
            "document",
            ItemStream::once(imported_cem_tree(
                import_data(source, format, "cem", "memory:sort-input").unwrap(),
            )),
        ));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(render_plan_to_html(&result), "<p>A</p><p>B</p>");
    }
    export_fixture(body, "sorted-formats");
}

#[test]
fn authored_key_errors_retain_stylesheet_ranges_and_cancel_discards_output() {
    let body = r#"<xsl:template match="/"><p>prefix</p><xsl:for-each select="/*/*"><xsl:sort select="xs:integer(@n)"/><b>unreachable</b></xsl:for-each></xsl:template>"#;
    let bundle = compile(body);
    let result = plan(&bundle, "<r><row n='bad'/></r>");
    assert!(result.nodes.is_empty());
    let start = stylesheet(body).find("xs:integer").unwrap() as u64;
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.uri.as_deref() == Some("memory:sort.xslt")
                && d.byte_offset == Some(start)
                && d.source_map.is_some()),
        "{:?}",
        result.diagnostics
    );
    let data = TemplateData::default().with_binding(
        "document",
        ItemStream::once(imported_cem_tree(
            import_data("<r><row n='1'/></r>", "xml", "cem", "memory:input").unwrap(),
        )),
    );
    let control = cem_ml::operation_control::OperationControl::default();
    control.cancel_root(None, None).unwrap();
    let result = bundle.render_with_control(
        &data,
        &control,
        cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
    );
    assert!(result.nodes.is_empty());
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn sorting_observes_native_work_limits_without_partial_output() {
    let bundle = compile(
        r#"<xsl:template match="/"><p>prefix</p><xsl:for-each select="/*/*"><xsl:sort select="@n"/><b>row</b></xsl:for-each></xsl:template>"#,
    );
    let source = format!(
        "<r>{}</r>",
        (0..20)
            .rev()
            .map(|n| format!("<row n='{n}'/>"))
            .collect::<String>()
    );
    let mut data = TemplateData::default().with_binding(
        "document",
        ItemStream::once(imported_cem_tree(
            import_data(&source, "xml", "cem", "memory:input").unwrap(),
        )),
    );
    data.native_functions = bundle
        .native_functions(cem_ml::validation::xpath::XPathEvaluationLimits {
            max_work_units: Some(500),
            ..Default::default()
        })
        .unwrap();
    let result = cem_ql::render::render_compiled_template(bundle.template(), &data);
    assert!(result.nodes.is_empty());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.xpath.work_limit_exceeded"),
        "{:?}",
        result.diagnostics
    );
}
