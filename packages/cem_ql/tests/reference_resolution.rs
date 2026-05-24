//! AC-QD-V-1 - reference-resolution fixture.
//!
//! Covers the Tier A `.target` / `dom:resolve_ref` behavior from
//! `docs/cem-ql-ac.md` section 13 over the current `Item::Node(String)` host
//! fragment representation.

use std::fs;
use std::path::PathBuf;

use cem_ml::diagnostics::Severity;
use cem_ml::scheduler::ScopePolicy;
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{Item, ItemStream, QueryContextScope};

fn eval(source: &str) -> ItemStream {
    let query = compile(source, &CompileContext::default())
        .unwrap_or_else(|err| panic!("compile failed for `{source}`: {err}"));
    let stream = evaluate(
        &query,
        &EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root().with_queue_size(128),
            diagnostics: Vec::new(),
        },
    );
    assert!(
        stream.error.is_none(),
        "evaluator failure for `{source}`: {:?} / {:?}",
        stream.error,
        stream.diagnostics
    );
    stream
}

#[test]
fn ac_qd_v_1_target_pipeline_resolves_for_attribute() {
    let stream =
        eval(r#"cemml:parse("{form | {input @id=email} {label @for=email | Email}}").target"#);

    assert_eq!(stream.items, vec![Item::Node("input#email".to_owned())]);
    assert!(stream
        .diagnostics
        .iter()
        .all(|diag| diag.code != "cem.ql.unresolved_reference"));
}

#[test]
fn ac_qd_v_1_dom_resolve_ref_resolves_aria_labelledby() {
    let stream = eval(
        r#"dom:resolve_ref(cemml:parse("{main | {h1 @id=title | Title} {section @aria-labelledby=title | Body}}"))"#,
    );

    assert_eq!(stream.items, vec![Item::Node("h1#title".to_owned())]);
    assert!(stream
        .diagnostics
        .iter()
        .all(|diag| diag.code != "cem.ql.unresolved_reference"));
}

#[test]
fn ac_qd_v_1_missing_target_emits_warning() {
    let stream = eval(r#"cemml:parse("{label @for=missing | Missing}").target"#);

    assert!(stream.items.is_empty());
    assert!(stream.diagnostics.iter().any(|diag| {
        diag.code == "cem.ql.unresolved_reference"
            && diag.severity == Severity::Warning
            && diag.message.contains("missing")
    }));
}

#[test]
fn reference_resolution_target_is_registered() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("project.json");
    let text = fs::read_to_string(&project)
        .unwrap_or_else(|err| panic!("read {}: {err}", project.display()));
    assert!(
        text.contains("\"test:reference-resolution\""),
        "project.json must expose the AC-QD-V-1 verification target"
    );
}
