use cem_ml::diagnostics::Severity;
use cem_ml::scheduler::ScopePolicy;
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, Item, QueryContextScope};
use cem_ql::stdlib::{ModuleRegistry, StdlibImplKind, Tier};

fn eval(source: &str) -> cem_ql::eval::ItemStream {
    let query = compile(source, &CompileContext::default()).unwrap();
    evaluate(
        &query,
        &EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root().with_queue_size(128),
            diagnostics: Vec::new(),
            policy_bindings: Default::default(),
        },
    )
}

#[test]
fn tier_a_registry_lists_every_documented_module_function() {
    let registry = ModuleRegistry::tier_a();

    assert_eq!(registry.functions.len(), 49);
    assert!(registry.resolve("cem:stdlib/sequence", "map", 2).is_some());
    assert!(registry.resolve("cem:stdlib/strings", "slice", 3).is_some());
    assert!(registry
        .resolve("cem:stdlib/numbers", "format", 2)
        .is_some());
    assert!(registry
        .resolve("cem:stdlib/datetime", "components", 1)
        .is_some());
    assert!(registry
        .resolve("cem:stdlib/dom", "resolve_ref", 1)
        .is_some());
    assert!(registry.resolve("cem:stdlib/report", "emit", 3).is_some());
    assert!(registry.resolve("cem:stdlib/state", "keys", 0).is_some());
    assert!(registry
        .resolve("cem:stdlib/template", "names", 0)
        .is_some());
    assert!(registry.resolve("cem:stdlib/cemml", "parse", 1).is_some());
    assert!(registry
        .functions
        .iter()
        .all(|function| function.tier == Tier::A));
    assert!(registry.functions.iter().any(|function| {
        function.module == "cem:stdlib/dom"
            && function.implementation == StdlibImplKind::HostContext
    }));
}

#[test]
fn string_stdlib_functions_evaluate() {
    let codepoints = eval(r#"str:codepoints("AZ")"#);
    assert_eq!(
        codepoints.items,
        vec![
            Item::Atomic(AtomValue::Integer(65)),
            Item::Atomic(AtomValue::Integer(90)),
        ]
    );

    let slice = eval(r#"str:slice("abcdef", 2, 3)"#);
    assert_eq!(
        slice.items,
        vec![Item::Atomic(AtomValue::String("cde".to_owned()))]
    );

    let concat = eval(r#"str:concat(("a", "b", "c"), "-")"#);
    assert_eq!(
        concat.items,
        vec![Item::Atomic(AtomValue::String("a-b-c".to_owned()))]
    );

    let contains = eval(r#"str:contains("semantic", "man")"#);
    assert_eq!(contains.items, vec![Item::Atomic(AtomValue::Boolean(true))]);

    // normalize_space: trims and collapses internal whitespace (XSLT normalize-space parity),
    // the primitive the converted cem-theme CSS generators use to read token table cells.
    let normalized = eval("str:normalize_space(\"  --cem-gap   \n  0.5rem  \")");
    assert_eq!(
        normalized.items,
        vec![Item::Atomic(AtomValue::String("--cem-gap 0.5rem".to_owned()))]
    );
}

#[test]
fn number_datetime_report_and_cemml_stdlib_functions_evaluate() {
    let rounded = eval(r#"num:round(3.6)"#);
    assert_eq!(rounded.items, vec![Item::Atomic(AtomValue::Integer(4))]);

    let formatted = eval(r#"num:format(12, "value={}")"#);
    assert_eq!(
        formatted.items,
        vec![Item::Atomic(AtomValue::String("value=12".to_owned()))]
    );

    let components = eval(r#"dt:components("2026-05-23T01:02:03Z")"#);
    let Some(Item::Record(record)) = components.items.first() else {
        panic!(
            "expected datetime components record, got {:?}",
            components.items
        );
    };
    assert_eq!(
        record.get("year").and_then(|items| items.first()),
        Some(&Item::Atomic(AtomValue::Integer(2026)))
    );

    let report = eval(r#"report:emit("cem.ql.test", "hello", "info")"#);
    assert!(report.items.is_empty());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "cem.ql.test"
            && diagnostic.message == "hello"
            && diagnostic.severity == Severity::Info
    }));

    let parsed = eval(r#"cemml:parse("{p | Hi}")"#);
    assert_eq!(parsed.items, vec![Item::Node("{p | Hi}\n".to_owned())]);
}
