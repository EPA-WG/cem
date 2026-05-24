//! AC-QA-V-1 - `read()` content-negotiation fixture.

use std::fs;
use std::path::{Path, PathBuf};

use cem_ml::scheduler::ScopePolicy;
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, EvalError, Item, ItemStream, QueryContextScope};

fn eval(source: &str) -> ItemStream {
    let query = compile(source, &CompileContext::default())
        .unwrap_or_else(|err| panic!("compile failed for `{source}`: {err}"));
    evaluate(
        &query,
        &EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root().with_queue_size(128),
            diagnostics: Vec::new(),
        },
    )
}

fn fixture_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cem_ql_read_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap_or_else(|err| panic!("create {}: {err}", dir.display()));
    dir
}

fn fixture(name: &str, body: &str) -> String {
    let path = fixture_dir().join(name);
    fs::write(&path, body).unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
    file_uri(&path)
}

fn file_uri(path: &Path) -> String {
    format!("file://{}", path.display())
}

fn node_text(stream: &ItemStream) -> &str {
    let Some(Item::Node(value)) = stream.items.first() else {
        panic!("expected node, got {:?}", stream.items);
    };
    value
}

#[test]
fn ac_qa_v_1_form_omitted_uses_floor_for_common_fixture_types() {
    let html = eval(&format!(
        r#"read("{}")"#,
        fixture("read.html", "<main></main>")
    ));
    assert!(node_text(&html).starts_with("text/html:"));

    let json = eval(&format!(
        r#"read("{}")"#,
        fixture("read.json", r#"{"ok":true}"#)
    ));
    assert!(node_text(&json).starts_with("application/json:"));

    let csv = eval(&format!(r#"read("{}")"#, fixture("read.csv", "a,b\n1,2\n")));
    assert!(node_text(&csv).starts_with("text/csv:"));
}

#[test]
fn ac_qa_v_1_form_header_string_honors_q_values_and_wildcards() {
    let yaml_uri = fixture("preferred.yaml", "ok: true\n");
    let yaml = eval(&format!(
        r#"read("{yaml_uri}", "application/json;q=0.9, application/yaml;q=1.0")"#
    ));
    assert!(node_text(&yaml).starts_with("application/yaml:"));

    let css_uri = fixture("theme.css", ":root { color: black; }\n");
    let css = eval(&format!(r#"read("{css_uri}", "text/*")"#));
    assert!(node_text(&css).starts_with("text/css:"));
}

#[test]
fn ac_qa_v_1_form_collection_uses_caller_order_and_normalizes_aliases() {
    let xml_uri = fixture("doc.xml", "<root />\n");
    let alias = eval(&format!(r#"read("{xml_uri}", ("text/xml"))"#));
    assert!(node_text(&alias).starts_with("application/xml:"));

    let yaml_uri = fixture("transform.yaml", "ok: true\n");
    let transformed = eval(&format!(r#"read("{yaml_uri}", (ct:json()))"#));
    assert!(node_text(&transformed).starts_with("application/json:"));
}

#[test]
fn ac_qa_v_1_unsatisfiable_binary_emits_read_unsatisfiable() {
    let bin_uri = fixture("payload.bin", "not a supported content type");
    let stream = eval(&format!(r#"read("{bin_uri}", (ct:json()))"#));

    assert!(stream.items.is_empty());
    assert_eq!(
        stream.error,
        Some(EvalError::Unsupported("read unsatisfiable"))
    );
    assert!(stream
        .diagnostics
        .iter()
        .any(|diag| diag.code == "cem.ql.read_unsatisfiable"
            && diag.message.contains("application/octet-stream")
            && diag.message.contains("application/json")));
}

#[test]
fn ac_qa_v_1_content_type_constants_are_exposed() {
    let json = eval("ct:json()");
    assert_eq!(
        json.items,
        vec![Item::Atomic(AtomValue::String(
            "application/json".to_owned()
        ))]
    );

    let floor = eval("ct:floor()");
    assert!(matches!(
        floor.items.first(),
        Some(Item::Atomic(AtomValue::String(value))) if value == "text/html"
    ));
    assert!(matches!(
        floor.items.last(),
        Some(Item::Atomic(AtomValue::String(value))) if value == "application/cem+xml"
    ));
}

#[test]
fn read_content_negotiation_target_is_registered() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("project.json");
    let text = fs::read_to_string(&project)
        .unwrap_or_else(|err| panic!("read {}: {err}", project.display()));
    assert!(
        text.contains("\"test:read-content-negotiation\""),
        "project.json must expose the AC-QA-V-1 verification target"
    );
}
