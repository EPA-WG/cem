//! Version-negotiation fixture coverage (FF-2 / AC-P-V-5).
//!
//! Drives each `examples/cem-ml/version-negotiation/*.cem` fixture through
//! the top-level builder and asserts the per-dispatched-namespace
//! version-negotiation outcome required by AC-P-V-5: a same-MAJOR
//! constraint loads forgivingly (binds the embedded Tier A profile and
//! records `cem.doc.version_resolved`), while an unsupported MAJOR or a
//! future MINOR within the supported MAJOR rejects deterministically with
//! `cem.doc.version_unsupported` and records no format identity.
//!
//! At Tier A the only dispatched content type whose version is negotiated
//! is the core CEM-ML document format itself (the `@doc cem-ml <version>`
//! directive); per-`@ns` schema-URI negotiation for other dispatched
//! namespaces is future work. The inline-string variants of these cases
//! live in `doc_directive.rs` (AC-F-8); this suite pins the AC-P-V-5
//! verification corpus to real on-disk fixtures, mirroring the
//! schema-scoping and namespace-rebinding fixture suites.

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::events::cem::CemEventNormalizer;
use cem_ml::parser::builder::CemAstBuilder;
use cem_ml::parser::document::CemDocument;
use cem_ml::parser::format::{SUPPORTED_VERSION, VERSION_RESOLVED_CODE};
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;

const VERSION_UNSUPPORTED_CODE: &str = "cem.doc.version_unsupported";

fn fixture(stem: &str) -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/cem-ml/version-negotiation")
        .join(format!("{stem}.cem"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"))
}

fn parse_top_level(input: &str) -> CemDocument {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    CemAstBuilder::new(normalizer).top_level(true).build()
}

fn doc_diagnostic<'a>(doc: &'a CemDocument, code: &str) -> Option<&'a Diagnostic> {
    doc.diagnostics.iter().find(|d| d.code == code)
}

#[test]
fn same_major_constraint_loads_forgivingly() {
    let doc = parse_top_level(&fixture("core-major-forgiving"));
    let identity = doc
        .format_identity
        .as_ref()
        .expect("forgiving same-MAJOR load must record a format identity");
    assert_eq!(identity.format_version, SUPPORTED_VERSION);
    let info = doc_diagnostic(&doc, VERSION_RESOLVED_CODE)
        .expect("forgiving load must emit cem.doc.version_resolved");
    assert_eq!(info.severity, Severity::Info);
    // The forgiving fixture is the FF-2 evidence file: it must parse with
    // no version-negotiation rejection.
    assert!(
        doc_diagnostic(&doc, VERSION_UNSUPPORTED_CODE).is_none(),
        "forgiving load must not also reject"
    );
}

#[test]
fn unsupported_major_rejects() {
    let doc = parse_top_level(&fixture("core-major-unsupported"));
    assert!(
        doc.format_identity.is_none(),
        "unsupported MAJOR must not record a format identity: {:?}",
        doc.format_identity
    );
    let d = doc_diagnostic(&doc, VERSION_UNSUPPORTED_CODE)
        .expect("unsupported MAJOR must emit cem.doc.version_unsupported");
    assert_eq!(d.severity, Severity::Error);
}

#[test]
fn future_minor_within_supported_major_rejects() {
    let doc = parse_top_level(&fixture("core-future-minor"));
    assert!(
        doc.format_identity.is_none(),
        "future MINOR must not record a format identity: {:?}",
        doc.format_identity
    );
    let d = doc_diagnostic(&doc, VERSION_UNSUPPORTED_CODE)
        .expect("future MINOR must emit cem.doc.version_unsupported");
    assert_eq!(d.severity, Severity::Error);
}
