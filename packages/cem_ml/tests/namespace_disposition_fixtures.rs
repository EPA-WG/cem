//! Namespace-disposition fixture coverage (parser-side AC-P-V-6 / AC-P-6.7).
//!
//! Drives `examples/cem-ml/namespace-disposition/*.cem` through the schema
//! machine under each run mode and asserts the deterministic unknown-namespace
//! disposition: a region whose namespace resolves to a URI with no metadata,
//! schema, or rule yields reject / allow / ignore per the run-mode default
//! (BR-VC-9), while known Tier A namespaces (cem core / HTML / SVG) are
//! unaffected.
//!
//! This is the parser-side verifier split out of FF-4 (which covers the
//! BR-VC-9 contract disposition). See the decision-core in
//! `packages/cem_ml/src/schema/disposition.rs`.

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::events::cem::CemEventNormalizer;
use cem_ml::schema::disposition::RunMode;
use cem_ml::schema::machine::CemSchemaMachine;
use cem_ml::schema::vocab::CompiledSchema;
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;

const REJECT_CODE: &str = "cem.schema.unresolved_namespace";
const ALLOW_CODE: &str = "cem.schema.unresolved_namespace_allowed";
const IGNORE_CODE: &str = "cem.schema.unresolved_namespace_ignored";

fn read(stem: &str) -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/cem-ml/namespace-disposition")
        .join(format!("{stem}.cem"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"))
}

fn run(input: &str, mode: RunMode) -> Vec<Diagnostic> {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    let machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer).with_run_mode(mode);
    machine.run().diagnostics
}

fn has_code(diags: &[Diagnostic], code: &str) -> bool {
    diags.iter().any(|d| d.code == code)
}

#[test]
fn application_run_rejects_the_unresolved_namespace_region() {
    let diags = run(&read("unknown-namespace"), RunMode::Application);
    let reject = diags
        .iter()
        .find(|d| d.code == REJECT_CODE)
        .expect("application run must reject the unresolved-namespace region");
    assert_eq!(reject.severity, Severity::Error);
    assert!(reject.message.contains("urn:example:widgets:1"));
    assert!(!has_code(&diags, ALLOW_CODE));
    assert!(!has_code(&diags, IGNORE_CODE));
}

#[test]
fn build_ssr_run_also_rejects() {
    let diags = run(&read("unknown-namespace"), RunMode::BuildSsr);
    assert!(has_code(&diags, REJECT_CODE));
    assert!(!has_code(&diags, ALLOW_CODE));
}

#[test]
fn development_run_allows_with_a_report_event() {
    let diags = run(&read("unknown-namespace"), RunMode::Development);
    let allow = diags
        .iter()
        .find(|d| d.code == ALLOW_CODE)
        .expect("development run must allow (report) the unresolved-namespace region");
    assert_eq!(allow.severity, Severity::Info);
    // Development is tolerant — it must not reject.
    assert!(!has_code(&diags, REJECT_CODE));
}

#[test]
fn known_namespace_elements_emit_no_disposition() {
    // The fixture's `main`/`p` (HTML) + `cem:screen` (core) are known Tier A
    // namespaces; only the one `widget:` element is unresolved. Exactly one
    // disposition diagnostic regardless of mode.
    for mode in [RunMode::Application, RunMode::BuildSsr, RunMode::Development] {
        let diags = run(&read("unknown-namespace"), mode);
        let count = diags
            .iter()
            .filter(|d| {
                d.code == REJECT_CODE || d.code == ALLOW_CODE || d.code == IGNORE_CODE
            })
            .count();
        assert_eq!(count, 1, "exactly one unresolved-namespace region in {mode:?}");
    }
}
