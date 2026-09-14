//! XSLT-VIEW-ERROR-GATE: characterize the existing diagnostic/recovery boundary.
//! These assert current CEM behavior, not XPath/XSLT error semantics. Changing
//! that shared contract requires approval; see docs/xslt-data-table-parity.md.
use cem_ml::diagnostics::Severity;
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{render_template, TemplateData};

fn eval(source: &str) -> ItemStream {
    let query = compile(source, &CompileContext::default()).expect("probe must compile");
    evaluate(&query, &EvaluationContext::default())
}

#[test]
fn native_parse_failures_are_report_data_not_dynamic_evaluation_errors() {
    for source in [
        r#"data:read("[", "json", "json-to-xml")"#,
        r#"data:read("<root>", "xml")"#,
    ] {
        let result = eval(source);
        assert!(result.error.is_none(), "{:?}", result.error);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let [report] = result.items.as_slice() else {
            panic!("expected one report: {:?}", result.items);
        };
        let view = report.view().expect("reader report retains a native view");
        assert!(view.field("root").unwrap_or_default().is_empty());
        let error = view.field("error").unwrap_or_default();
        assert!(
            matches!(error.as_slice(), [Item::Atomic(AtomValue::String(message))] if !message.is_empty())
        );
    }
}

#[test]
fn even_fatal_report_emission_does_not_raise_an_evaluation_error() {
    let result = eval(r#"(report:emit("probe.parse", "invalid source", "fatal"), "after")"#);
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(
        result.items,
        vec![Item::Atomic(AtomValue::String("after".into()))]
    );
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "probe.parse" && d.severity == Severity::Fatal));
}

#[test]
fn empty_sequence_coalescing_does_not_catch_evaluation_errors() {
    let empty = eval("() ?? 42");
    assert!(empty.error.is_none());
    assert!(empty.diagnostics.is_empty());
    assert_eq!(empty.items, vec![Item::Atomic(AtomValue::Integer(42))]);

    let failed = eval("(1 / 0) ?? 42");
    assert!(failed.error.is_some());
    assert!(failed.items.is_empty(), "must not treat failure as absence");
    assert!(failed
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.type_error"));
}

#[test]
fn failed_nested_template_does_not_roll_back_output_or_stop_later_siblings() {
    let result = render_template(
        r#"{template @name=inner |
            {b | partial}
            {$1 / 0}
            {i | inner-after}
        }
        {p | before}
        {call @template=inner}
        {p | after}"#,
        &TemplateData::default(),
    );
    assert_eq!(
        result.rendered.split_whitespace().collect::<String>(),
        "<p>before</p><b>partial</b><i>inner-after</i><p>after</p>"
    );
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.type_error"));
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.render.eval_failed"));
}
