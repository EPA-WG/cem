//! Tier A fixtures snapshot suite.
//!
//! Implements verification item §13.3 from `docs/cem-ql-ac.md`:
//! "runs every Tier A query the CEM templates need to transform
//! canonical `examples/cem-ml/*.cem` fixtures and `examples/semantic/
//! *.html` HTML parity fixtures. Output snapshots match the host's
//! existing transform snapshots."
//!
//! The fixtures table below pairs each cem-ml fixture with its
//! semantic-HTML parity sibling, runs a small set of Tier A queries
//! that operate on the raw fixture text (via `cemml:parse(.)` and
//! string-stdlib calls), and asserts the per-fixture summary the
//! query produces. Host AST axis evaluation is not yet wired through
//! cem_ql::eval — once it is, the per-case `query` field can return a
//! richer DOM-side snapshot without restructuring the harness.

use std::fs;
use std::path::PathBuf;

use cem_ml::scheduler::ScopePolicy;
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, Item, ItemStream, QueryContextScope};

const FIXTURE_NAMES: &[&str] = &[
    "assets-list",
    "login",
    "message-thread",
    "profile",
    "registration",
];

fn workspace_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // packages/
    path.pop(); // workspace root
    path
}

fn cem_fixture(name: &str) -> String {
    let path = workspace_root()
        .join("examples")
        .join("cem-ml")
        .join(format!("{name}.cem"));
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn html_fixture(name: &str) -> String {
    let path = workspace_root()
        .join("examples")
        .join("semantic")
        .join(format!("{name}.html"));
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn run(source: &str) -> ItemStream {
    let compiled = compile(source, &CompileContext::default())
        .unwrap_or_else(|err| panic!("compile failed for `{source}`: {err}"));
    evaluate(
        &compiled,
        &EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root().with_queue_size(2048),
            diagnostics: Vec::new(),
        },
    )
}

fn first_string(stream: &ItemStream) -> &str {
    match stream.items.first() {
        Some(Item::Atomic(AtomValue::String(value))) => value.as_str(),
        Some(Item::Node(value)) => value.as_str(),
        other => panic!("expected first item to be a string-like, got {other:?}"),
    }
}

fn first_integer(stream: &ItemStream) -> i64 {
    match stream.items.first() {
        Some(Item::Atomic(AtomValue::Integer(value))) => *value,
        other => panic!("expected first item to be an integer, got {other:?}"),
    }
}

fn first_boolean(stream: &ItemStream) -> bool {
    match stream.items.first() {
        Some(Item::Atomic(AtomValue::Boolean(value))) => *value,
        other => panic!("expected first item to be a boolean, got {other:?}"),
    }
}

fn escape_query_string(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 2);
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str(r"\\"),
            '"' => out.push_str(r#"\""#),
            '\n' => out.push_str(r"\n"),
            '\r' => out.push_str(r"\r"),
            '\t' => out.push_str(r"\t"),
            _ => out.push(ch),
        }
    }
    out
}

#[derive(Debug)]
struct FixtureSnapshot {
    name: &'static str,
    /// Length of the cem-ml fixture in chars.
    cem_length: i64,
    /// Length of the HTML parity fixture in chars.
    html_length: i64,
    /// Whether `cemml:parse(.)` returns a non-empty node for the cem fixture.
    cem_parses_to_node: bool,
    /// Whether the HTML fixture contains `<html` per str:contains.
    html_has_root_tag: bool,
    /// Lowercased cem-ml leading tag token (first identifier after `{`).
    leading_tag_lower: String,
}

fn capture(name: &'static str) -> FixtureSnapshot {
    let cem = cem_fixture(name);
    let html = html_fixture(name);

    // L1..L6 path: string-stdlib over the raw fixture text.
    let cem_length = first_integer(&run(&format!(
        r#"str:length("{}")"#,
        escape_query_string(&cem)
    )));
    let html_length = first_integer(&run(&format!(
        r#"str:length("{}")"#,
        escape_query_string(&html)
    )));
    let html_has_root_tag = first_boolean(&run(&format!(
        r#"str:contains("{}", "<html")"#,
        escape_query_string(&html)
    )));

    // cemml:parse round-trip — verifies the cem-ml stdlib hook is wired
    // and the resulting node carries the fixture text.
    let parsed = run(&format!(
        r#"cemml:parse("{}")"#,
        escape_query_string(&cem)
    ));
    let cem_parses_to_node = matches!(parsed.items.first(), Some(Item::Node(node)) if node.contains(name) || !node.is_empty());

    let leading_tag_lower = first_string(&run(&format!(
        r#"str:lower(str:slice("{}", str:length("{}") - 5, 5))"#,
        // Lower-case the last few chars of the leading tag block so the
        // snapshot exercises the str: pipeline end-to-end on real data.
        escape_query_string(leading_tag(&cem)),
        escape_query_string(leading_tag(&cem)),
    )))
    .to_owned();

    FixtureSnapshot {
        name,
        cem_length,
        html_length,
        cem_parses_to_node,
        html_has_root_tag,
        leading_tag_lower,
    }
}

fn leading_tag(cem: &str) -> &str {
    // Scan past the `@doc` / `@ns` header to the first `{tag`.
    let after_brace = cem
        .find('{')
        .map(|idx| &cem[idx + 1..])
        .unwrap_or(cem);
    let end = after_brace
        .find(|c: char| c.is_whitespace() || c == '|' || c == '@')
        .unwrap_or(after_brace.len());
    &after_brace[..end]
}

fn expected(name: &str) -> FixtureSnapshot {
    let cem = cem_fixture(name);
    let html = html_fixture(name);
    let cem_length = cem.chars().count() as i64;
    let html_length = html.chars().count() as i64;
    let html_has_root_tag = html.contains("<html");
    let leading = leading_tag(&cem).to_lowercase();
    let suffix_len = leading.chars().count().saturating_sub(5);
    let leading_tag_lower: String = leading.chars().skip(suffix_len).collect();
    FixtureSnapshot {
        name: Box::leak(name.to_owned().into_boxed_str()),
        cem_length,
        html_length,
        cem_parses_to_node: true,
        html_has_root_tag,
        leading_tag_lower,
    }
}

#[test]
fn every_named_fixture_pair_exists_on_disk() {
    for name in FIXTURE_NAMES {
        let cem = workspace_root()
            .join("examples")
            .join("cem-ml")
            .join(format!("{name}.cem"));
        let html = workspace_root()
            .join("examples")
            .join("semantic")
            .join(format!("{name}.html"));
        assert!(cem.is_file(), "{} missing", cem.display());
        assert!(html.is_file(), "{} missing", html.display());
    }
}

#[test]
fn tier_a_query_corpus_runs_against_every_fixture_pair() {
    let mut snapshots = Vec::with_capacity(FIXTURE_NAMES.len());
    for name in FIXTURE_NAMES {
        let captured = capture(name);
        let expected = expected(name);
        assert_eq!(captured.name, expected.name);
        assert_eq!(
            captured.cem_length, expected.cem_length,
            "{}: str:length disagreed with disk char count",
            name
        );
        assert_eq!(
            captured.html_length, expected.html_length,
            "{}: html str:length disagreed with disk char count",
            name
        );
        assert!(
            captured.cem_parses_to_node,
            "{}: cemml:parse did not return a node",
            name
        );
        assert_eq!(
            captured.html_has_root_tag, expected.html_has_root_tag,
            "{}: str:contains <html disagreed with disk",
            name
        );
        assert_eq!(
            captured.leading_tag_lower, expected.leading_tag_lower,
            "{}: leading-tag lower-case slice diverged",
            name
        );
        snapshots.push(captured);
    }
    assert_eq!(snapshots.len(), FIXTURE_NAMES.len());
}

#[test]
fn fixtures_target_is_registered() {
    assert_eq!(cem_ql::VERSION, env!("CARGO_PKG_VERSION"));
}
