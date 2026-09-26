use cem_ql::api::{compile_module, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, Item, ItemStream};

#[test]
fn module_entrypoint_uses_stdlib_aliases_local_declarations_and_input() {
    let mut context = CompileContext::default();
    context
        .policy_bindings
        .insert("input".into(), ItemStream::empty());
    let query = compile_module(
        r#"module "urn:test:main"
import "cem:stdlib/url" as u
declare let base = "https://h/"
declare function local:link(path as string) { u:href(path, base) }
(local:link("next"), input)"#,
        &context,
    )
    .unwrap();
    let mut data = EvaluationContext::default();
    data.policy_bindings.insert(
        "input".into(),
        ItemStream::once(Item::Atomic(AtomValue::Integer(42))),
    );
    let result = evaluate(&query, &data);
    assert!(result.diagnostics.is_empty(), "{result:?}");
    assert!(
        matches!(result.items.as_slice(), [Item::Atomic(AtomValue::AnyUri(uri)), Item::Atomic(AtomValue::Integer(42))] if uri == "https://h/next")
    );
}

#[test]
fn module_errors_retain_original_diagnostics_and_ranges() {
    for (source, code) in [
        ("42", "cem.ql.module_uri_missing"),
        ("module \"urn:test\"", "cem.ql.parse_error"),
        ("module \"urn:test\" 1 2", "cem.ql.parse_error"),
        (
            "module \"urn:test\" module \"urn:other\" 1",
            "cem.ql.parse_error",
        ),
        (
            "module \"urn:test\" import \"https://host/query\" as x 1",
            "cem.ql.import_denied",
        ),
        (
            "module \"urn:test\" import \"urn:cem:plugin:missing\" as x 1",
            "cem.ql.import_unresolved",
        ),
        (
            "module \"urn:test\" import \"cem:stdlib/missing\" as x 1",
            "cem.ql.import_unresolved",
        ),
        (
            "module \"urn:test\" declare let x = 1 declare let x = 2 x",
            "cem.ql.declaration_duplicate",
        ),
        (
            "module \"urn:test\" import \"cem:stdlib/url\" as u import \"cem:stdlib/url\" as u 1",
            "cem.ql.import_alias_duplicate",
        ),
        ("module \"urn:test\" url:href(42)", "cem.ql.type_error"),
        ("module \"urn:test\" 1 +", "cem.ql.parse_error"),
    ] {
        let diagnostics = compile_module(source, &CompileContext::default()).expect_err(source);
        let diagnostic = diagnostics
            .iter()
            .find(|d| d.code == code)
            .unwrap_or_else(|| panic!("{source}: {diagnostics:?}"));
        let offset = diagnostic.byte_offset.expect("source offset");
        assert!(offset <= source.len() as u64);
        let frame = diagnostic
            .source_map
            .as_ref()
            .and_then(|map| map.current())
            .expect("exact source frame");
        assert!(format!("{frame:?}").contains("Query"));
    }
}

#[test]
fn external_grants_do_not_enable_unlinked_libraries_and_locals_do_not_leak() {
    let context = CompileContext {
        import_policy: cem_ql::resolve::ImportPolicy::new()
            .allow_scheme("https")
            .unwrap(),
        ..Default::default()
    };
    let errors = compile_module(
        "module \"urn:test\" import \"https://h/lib\" as lib 1",
        &context,
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "cem.ql.import_denied");
    let first = compile_module(
        "module \"urn:test\" declare function url:href(x) { x } url:href(42)",
        &context,
    )
    .unwrap();
    assert!(matches!(
        evaluate(&first, &EvaluationContext::default())
            .items
            .as_slice(),
        [Item::Atomic(AtomValue::Integer(42))]
    ));
    assert!(compile_module("module \"urn:test\" url:href(42)", &context).is_err());
}

#[test]
fn local_input_shadows_the_host_binding_without_mutating_it() {
    let mut context = CompileContext::default();
    context
        .policy_bindings
        .insert("input".into(), ItemStream::empty());
    let query =
        compile_module("module \"urn:test\" declare let input = 7 input", &context).unwrap();
    let mut data = EvaluationContext::default();
    data.policy_bindings.insert(
        "input".into(),
        ItemStream::once(Item::Atomic(AtomValue::Integer(42))),
    );
    assert!(matches!(
        evaluate(&query, &data).items.as_slice(),
        [Item::Atomic(AtomValue::Integer(7))]
    ));
    assert!(matches!(
        data.policy_bindings["input"].items.as_slice(),
        [Item::Atomic(AtomValue::Integer(42))]
    ));
}
