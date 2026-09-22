use super::*;
use std::cell::Cell;

thread_local! {
    static FRESH_CHECKERS: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn fresh_checkers_enabled() -> bool {
    FRESH_CHECKERS.with(Cell::get)
}

/// Compare complete compiler output against the unchanged standalone path.
/// Scoped to this test thread and restored even if the comparison panics.
pub(crate) fn without_prepared<T>(operation: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            FRESH_CHECKERS.with(|value| value.set(self.0));
        }
    }
    let _reset = Reset(FRESH_CHECKERS.with(|value| value.replace(true)));
    operation()
}

#[test]
fn prepared_checkers_preserve_diagnostics_ir_and_expression_isolation() {
    let mut prepared = PreparedTypeChecking::default();
    // Each declaration/import is followed by a query where it must be absent.
    let cases = [
        "1",
        "seq:map((1, 2), fn(n) => n + 1)",
        "import \"cem:stdlib/modules\" as other\nother:module_url(42)",
        "other:module_url(42)",
        "import \"memory:opaque\" as host\nhost:custom(1)",
        "host:custom(1)",
        "declare function local:echo(item as string) { item }\nlocal:echo(1)",
        "local:echo(1)",
        "declare function module:module_url(item as integer) { item }\nmodule:module_url(42)",
        "module:module_url(42)",
        "declare let local_value = 1\nlocal_value + 1",
        "local_value + 1",
        "policy_value",
        "native:call(\"fixture\", 1)",
        "same_node(1, 2)",
        "module_url()",
        "1 + true",
        "unknown:missing(1)",
        "1",
    ];
    for config in [
        TyConfig::strict(),
        TyConfig::dev_profile(),
        TyConfig::strict(),
    ] {
        for with_binding in [true, false] {
            let mut context = CompileContext {
                type_config: config.clone(),
                import_policy: ImportPolicy::new().allow_scheme("memory").unwrap(),
                ..CompileContext::default()
            };
            if with_binding {
                context
                    .policy_bindings
                    .insert("policy_value".into(), ItemStream::empty());
            }
            for source in cases {
                let parsed = parse(source);
                assert!(
                    parsed.diagnostics.is_empty(),
                    "{source}: {:?}",
                    parsed.diagnostics
                );
                assert_eq!(
                    prepared.type_check(&parsed.module, &context),
                    type_check(&parsed.module, &context),
                    "diagnostics: {source}"
                );
                // Includes every source frame and lowering diagnostic/error.
                let bytes = |query: CompiledQuery| rmp_serde::to_vec_named(&query).unwrap();
                assert_eq!(
                    prepared.compile(source, &context).map(bytes),
                    compile(source, &context).map(bytes),
                    "IR: {source}"
                );
            }
        }
    }
    let mut original = TypeChecker::new();
    original.seed_runtime_import_surface(&parse("1").module);
    let baseline = prepared.builtins.as_ref().unwrap();
    assert_eq!(baseline.functions, original.functions);
    assert!(baseline.diagnostics().is_empty());
}

#[test]
fn prepared_baseline_is_not_created_before_type_checking() {
    let mut prepared = PreparedTypeChecking::default();
    let context = CompileContext::default();
    for source in [
        "1 +",
        "same_node(1)",
        "import \"cem:stdlib/missing\" as other\nother:value()",
    ] {
        assert!(prepared.compile(source, &context).is_err());
        assert!(prepared.builtins.is_none(), "{source}");
    }
    assert!(prepared.compile("1 + true", &context).is_err());
    assert!(prepared.builtins.is_some());
    assert!(prepared.compile("1 + 2", &context).is_ok());
}
