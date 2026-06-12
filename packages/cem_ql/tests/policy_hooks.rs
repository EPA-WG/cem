//! AC-QV-V-2 - policy-hook binding fixture.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use cem_ml::scheduler::ScopePolicy;
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, EvalError, Item, ItemStream, QueryContextScope, ResourceHandle};

fn scope_stream() -> ItemStream {
    let mut theme = BTreeMap::new();
    theme.insert(
        "name".to_owned(),
        vec![Item::Atomic(AtomValue::String("midnight".to_owned()))],
    );

    let mut scope = BTreeMap::new();
    scope.insert("theme".to_owned(), vec![Item::Record(theme)]);
    scope.insert(
        "user".to_owned(),
        vec![Item::Resource(ResourceHandle {
            id: "u-1".to_owned(),
            content_type: "user-profile".to_owned(),
            schema: Some("user-schema".to_owned()),
            roles: vec!["admin".to_owned(), "reviewer".to_owned()],
            fail_accessor: false,
        })],
    );
    scope.insert(
        "failing_user".to_owned(),
        vec![Item::Resource(ResourceHandle {
            id: "u-fail".to_owned(),
            content_type: "user-profile".to_owned(),
            schema: Some("user-schema".to_owned()),
            roles: Vec::new(),
            fail_accessor: true,
        })],
    );

    ItemStream::once(Item::Record(scope))
}

fn contexts() -> (CompileContext, EvaluationContext) {
    let mut policy_bindings = BTreeMap::new();
    policy_bindings.insert("scope".to_owned(), scope_stream());
    (
        CompileContext {
            policy_bindings: policy_bindings.clone(),
            ..CompileContext::default()
        },
        EvaluationContext {
            scope: QueryContextScope(9),
            scope_policy: ScopePolicy::host_root().with_queue_size(128),
            diagnostics: Vec::new(),
            policy_bindings,
        },
    )
}

fn eval(source: &str) -> ItemStream {
    let (compile_context, eval_context) = contexts();
    let query = compile(source, &compile_context)
        .unwrap_or_else(|err| panic!("compile failed for `{source}`: {err}"));
    evaluate(&query, &eval_context)
}

#[test]
fn ac_qv_v_2_reads_scope_theme_record_field() {
    let stream = eval("scope.theme.name");

    assert_eq!(
        stream.items,
        vec![Item::Atomic(AtomValue::String("midnight".to_owned()))]
    );
    assert!(stream.error.is_none(), "{:?}", stream.diagnostics);
}

#[test]
fn ac_qv_v_2_user_resource_accessor_checks_role() {
    let stream = eval(r#"user:has_role(scope.user, "admin")"#);

    assert_eq!(stream.items, vec![Item::Atomic(AtomValue::Boolean(true))]);
    assert!(stream.error.is_none(), "{:?}", stream.diagnostics);
}

#[test]
fn ac_qv_v_2_inherited_resource_identity_is_stable() {
    let stream = eval("scope.user is scope.user");

    assert_eq!(stream.items, vec![Item::Atomic(AtomValue::Boolean(true))]);
    assert!(stream.error.is_none(), "{:?}", stream.diagnostics);
}

#[test]
fn ac_qv_v_2_accessor_failure_emits_policy_accessor_failed() {
    let stream = eval(r#"user:has_role(scope.failing_user, "admin")"#);

    assert!(stream.items.is_empty());
    assert_eq!(
        stream.error,
        Some(EvalError::Unsupported("policy accessor failed"))
    );
    assert!(stream.diagnostics.iter().any(|diag| {
        diag.code == "cem.ql.policy_accessor_failed" && diag.message.contains("u-fail")
    }));
}

#[test]
fn policy_hooks_target_is_registered() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("project.json");
    let text = fs::read_to_string(&project)
        .unwrap_or_else(|err| panic!("read {}: {err}", project.display()));
    assert!(
        text.contains("\"test:policy-hooks\""),
        "project.json must expose the AC-QV-V-2 verification target"
    );
}
