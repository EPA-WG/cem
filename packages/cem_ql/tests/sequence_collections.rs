use cem_ml::scheduler::ScopePolicy;
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, Item, ItemStream};

fn eval(source: &str, limit: u32) -> ItemStream {
    let query =
        compile(source, &CompileContext::default()).expect("generic collection query compiles");
    evaluate(
        &query,
        &EvaluationContext {
            scope_policy: ScopePolicy::host_root().with_queue_size(limit),
            ..EvaluationContext::default()
        },
    )
}

fn strings(stream: ItemStream) -> Vec<String> {
    assert!(stream.error.is_none(), "{:?}", stream.diagnostics);
    stream
        .items
        .into_iter()
        .map(|item| match item {
            Item::Atomic(AtomValue::String(value)) => value,
            _ => panic!("expected string: {item:?}"),
        })
        .collect()
}

#[test]
fn grouping_uses_expression_keys_and_first_occurrence_order() {
    assert_eq!(
        strings(eval(
            r#"
        seq:group_by(("🍒", "🍋", "🍒", "🍏"), fn(value) => value).key
    "#,
            128
        )),
        ["🍒", "🍋", "🍏"]
    );
    assert_eq!(
        strings(eval(
            r#"
        seq:group_by(("Apple", "pear", "apricot"), fn(value) => str:lower(str:slice(value, 0, 1)))
            .items
    "#,
            128
        )),
        ["Apple", "apricot", "pear"]
    );
}

#[test]
fn stable_sort_accepts_explicit_text_numeric_and_direction_options() {
    assert_eq!(
        strings(eval(r#"seq:sorted(("10", "2", "3"), fn(x) => x)"#, 128)),
        ["10", "2", "3"]
    );
    assert_eq!(
        strings(eval(
            r#"seq:sorted(("10", "2", "3"), fn(x) => x, "ascending", "number")"#,
            128
        )),
        ["2", "3", "10"]
    );
    assert_eq!(
        strings(eval(
            r#"
        seq:sorted(({n:"2", label:"a"}, {label:"missing"}, {n:"10", label:"b"},
                    {n:"2", label:"c"}, {n:"bad", label:"invalid"}),
                   fn(x) => x.n, "descending", "number").label
    "#,
            128
        )),
        ["b", "a", "c", "missing", "invalid"]
    );
}

#[test]
fn collections_reject_invalid_options_keys_and_materialization_over_budget() {
    for source in [
        r#"seq:sorted((1,2), fn(x) => x, "sideways")"#,
        r#"seq:sorted((1,2), fn(x) => x, "ascending", "guess")"#,
        r#"seq:group_by((1,2), fn(x) => {a: x})"#,
        r#"seq:sorted((1,2), fn(x) => (x,x))"#,
    ] {
        let result = eval(source, 128);
        assert!(result.error.is_some(), "must reject {source}");
        assert!(result.items.is_empty(), "no partial collection on error");
    }
    for name in ["group_by", "sorted"] {
        let result = eval(&format!("seq:{name}((1,2,3,4,5), fn(x) => x)"), 2);
        assert!(
            result.error.is_some(),
            "{name} must honor collection budgets"
        );
        assert!(result.items.is_empty());
    }
}
