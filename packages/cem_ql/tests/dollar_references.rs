use cem_ml::source::ByteRange;
use cem_ql::api::{
    compile, compile_expression, evaluate, evaluate_expression, parse, CompileContext,
    EvaluationContext, StandaloneExpressionBinding, StandaloneExpressionContext,
};
use cem_ql::embedded::{compile_embedded_expression, extract_embedded_expressions_from_source};
use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::parser::{BinaryOp, Expression, SurfaceNode};
use cem_ql::render::{
    compile_template, render_compiled_template, render_plan_to_html, render_template,
    CompileTemplateOptions, TemplateData,
};

fn string(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}

#[test]
fn parser_preserves_original_source_and_reference_ranges() {
    let source = "/* 😁 $literal */ $left ?? $right";
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.module.source, source);
    let SurfaceNode::Expression(Expression::BinaryOp {
        op,
        lhs,
        rhs,
        range,
    }) = &parsed.module.nodes[0]
    else {
        panic!("expected coalescing expression: {:?}", parsed.module.nodes);
    };
    assert_eq!(*op, BinaryOp::Coalesce);
    assert_eq!(
        *range,
        ByteRange::new(source.find("$left").unwrap() as u64, 15)
    );
    for (expr, name) in [(lhs, "left"), (rhs, "right")] {
        let Expression::Name(qname, range) = expr.as_ref() else {
            panic!("expected name");
        };
        assert_eq!(qname.local, name);
        assert_eq!(qname.prefix, None);
        assert_eq!(
            range.start,
            source.find(&format!("${name}")).unwrap() as u64
        );
        assert_eq!(range.len as usize, name.len() + 1);
        assert_eq!(qname.range.start, range.start + 1);
    }
}

#[test]
fn standalone_queries_resolve_aliases_without_changing_values_or_types() {
    let context = StandaloneExpressionContext::default()
        .with_binding(
            "s",
            StandaloneExpressionBinding::any(ItemStream::once(Item::Atomic(AtomValue::Null))),
        )
        .with_binding(
            "a",
            StandaloneExpressionBinding::any(ItemStream::once(string("A"))),
        )
        .with_binding(
            "record",
            StandaloneExpressionBinding::any(ItemStream::once(Item::Record(
                [("field".into(), vec![string("field")])].into(),
            ))),
        );
    for (source, expected) in [
        ("$s ?? $a", string("A")),
        ("$record.field", string("field")),
        (
            r#""price $a 😁" + $a /* $untouched */ // $untouched"#,
            string("price $a 😁A"),
        ),
        ("str:concat(($a, a), $a)", string("AAA")),
        ("{ let local = $a; $local + local }", string("AA")),
        ("{ [$a]: $record.field }.A", string("field")),
    ] {
        let result = evaluate_expression(source, &context)
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert!(
            result.result.error.is_none(),
            "{source}: {:?}",
            result.result.error
        );
        assert_eq!(result.result.items, vec![expected], "{source}");
    }
    assert!(
        compile_expression("$a + 1", &context).is_ok(),
        "unknown binding types defer to runtime"
    );
    assert!(evaluate_expression("$a + 1", &context)
        .unwrap()
        .result
        .error
        .is_some());
    let error = compile_expression("/* 😁 */ $missing", &context).unwrap_err();
    assert!(
        error
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.ql.data_binding_missing"),
        "{error:?}"
    );
    assert!(
        error
            .diagnostics
            .iter()
            .any(|d| d.byte_offset == Some("/* 😁 */ ".len() as u64)),
        "{error:?}"
    );
}

#[test]
fn module_queries_reuse_bare_declarations_and_qualified_names() {
    let source = "declare let amount = 2\ndeclare function local:twice(value) { $value + value }\n$local:twice($amount)";
    let query = compile(source, &CompileContext::default()).unwrap();
    let result = evaluate(&query, &EvaluationContext::default());
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(result.items, vec![Item::Atomic(AtomValue::Integer(4))]);
}

#[test]
fn only_expression_references_accept_the_prefix() {
    for source in [
        "declare let $name = 1",
        "{ let $name = 1; name }",
        "declare function $name(arg) { arg }",
        "declare function local:f($arg) { arg }",
        "fn($arg) => arg",
        "|$arg| arg",
        "for $item in (1, 2) { item }",
        "import \"cem:stdlib/strings\" as $str",
        "{ $key: 1 }",
        "record.$field",
        "value as $integer",
        "value is $integer",
        "treat_as(value, $integer)",
        "$",
        "$$name",
        "$1",
        "$ name",
        "$/*comment*/name",
    ] {
        let parsed = parse(source);
        assert!(!parsed.diagnostics.is_empty(), "must reject {source}");
    }
}

#[test]
fn templates_and_embedded_audits_share_reference_syntax() {
    let source = r#"{attribute @name=a | A}{slice @name=s}{p @title="{$s ?? $a}" | {$s ?? $a}}{cem:for-each @select='$s ?? $a' @as=item | {$ $item}}"#;
    let rendered = render_template(source, &TemplateData::default());
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
    assert_eq!(rendered.rendered, "<p title=\"A\">A</p>A");
    let expressions = extract_embedded_expressions_from_source("probe.cemt", source);
    assert_eq!(expressions.len(), 4);
    for expression in expressions {
        assert_eq!(expression.normalized_source, expression.source);
        let report = compile_embedded_expression(&expression);
        assert!(!report.has_hard_diagnostics(), "{report:?}");
    }
    let source = "{p @title='{$a + $}'}";
    let expression = extract_embedded_expressions_from_source("probe.cemt", source).remove(0);
    let report = compile_embedded_expression(&expression);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.source_byte_offset == Some(source.rfind('$').unwrap() as u64)),
        "{report:?}"
    );
}

#[test]
fn authored_data_slice_attribute_fallback_handles_absent_and_empty_slices() {
    let source = include_str!("../../cem-elements/demo/data-slices.html")
        .split_once("<cem-element tag=\"cem-slice-attribute-initial\">")
        .unwrap()
        .1
        .split_once("<template type=\"text/cem-ml\">")
        .unwrap()
        .1
        .split_once("</template>")
        .unwrap()
        .0;
    let artifact = compile_template(source, &CompileTemplateOptions::default());
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    for (attribute, slice, expected) in [
        (None, None, "😁"),
        (Some("🤗"), None, "🤗"),
        (Some("🤗"), Some("edited"), "edited"),
        (Some("🤗"), Some(""), ""),
    ] {
        let mut data = TemplateData::default().with_binding(
            "datadom",
            ItemStream::once(Item::Record(
                [
                    (
                        "attributes".into(),
                        vec![Item::Record(
                            attribute
                                .into_iter()
                                .map(|v| ("a".into(), vec![string(v)]))
                                .collect(),
                        )],
                    ),
                    (
                        "slices".into(),
                        vec![Item::Record(
                            slice
                                .into_iter()
                                .map(|v| ("s".into(), vec![string(v)]))
                                .collect(),
                        )],
                    ),
                ]
                .into(),
            )),
        );
        if let Some(value) = attribute {
            data = data.with_binding("a", ItemStream::once(string(value)));
        }
        if let Some(value) = slice {
            data = data.with_binding("s", ItemStream::once(string(value)));
        }
        let rendered = render_compiled_template(&artifact, &data);
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
        let html = render_plan_to_html(&rendered);
        let input = html
            .split_once("<input")
            .unwrap()
            .1
            .split_once('>')
            .unwrap()
            .0;
        if expected.is_empty() {
            assert!(!input.contains(" value="), "{html}");
        } else {
            assert!(input.contains(&format!("value=\"{expected}\"")), "{html}");
        }
    }
}
