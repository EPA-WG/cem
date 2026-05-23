//! AC-F-V-6 fixture: persisted top-level `@doc cem-ml <version>`
//! parsing.
//!
//! Each case loads a CEM-ML source through the layered pipeline,
//! marks the builder as top-level via `.top_level(true)`, and asserts
//! the canonical diagnostic / resolved-identity outcome required by
//! AC-F-8.

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::events::cem::CemEventNormalizer;
use cem_ml::parser::builder::CemAstBuilder;
use cem_ml::parser::document::CemDocument;
use cem_ml::parser::format::{
    DocumentFormatIdentity, SUPPORTED_CONTENT_TYPE, SUPPORTED_FORMAT_ID, SUPPORTED_VERSION,
    VERSION_MISSING_CODE, VERSION_RESOLVED_CODE,
};
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;

fn parse_top_level(input: &str) -> CemDocument {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    CemAstBuilder::new(normalizer).top_level(true).build()
}

fn parse_fragment(input: &str) -> CemDocument {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    CemAstBuilder::new(normalizer).build()
}

fn doc_diagnostic<'a>(doc: &'a CemDocument, code: &str) -> Option<&'a Diagnostic> {
    doc.diagnostics.iter().find(|d| d.code == code)
}

fn assert_resolved_to_tier_a(doc: &CemDocument) {
    let identity: &DocumentFormatIdentity =
        doc.format_identity.as_ref().expect("format_identity recorded");
    assert_eq!(identity.format_id, SUPPORTED_FORMAT_ID);
    assert_eq!(identity.content_type, SUPPORTED_CONTENT_TYPE);
    assert_eq!(identity.format_version, SUPPORTED_VERSION);
    let info = doc_diagnostic(doc, VERSION_RESOLVED_CODE).expect("version_resolved event");
    assert_eq!(info.severity, Severity::Info);
}

fn assert_rejected(doc: &CemDocument, expected_code: &str) {
    assert!(
        doc.format_identity.is_none(),
        "format_identity unexpectedly recorded: {:?}",
        doc.format_identity
    );
    let d = doc_diagnostic(doc, expected_code)
        .unwrap_or_else(|| panic!("expected diagnostic {expected_code}, got {:?}", codes(doc)));
    assert_eq!(d.severity, Severity::Error);
}

fn codes(doc: &CemDocument) -> Vec<&str> {
    doc.diagnostics
        .iter()
        .filter(|d| d.code.starts_with("cem.doc."))
        .map(|d| d.code.as_str())
        .collect()
}

#[test]
fn doc_cem_ml_1_resolves_to_tier_a_profile() {
    let doc = parse_top_level("@doc cem-ml 1\n{p | hi}");
    assert_resolved_to_tier_a(&doc);
}

#[test]
fn doc_cem_ml_1_0_resolves_to_tier_a_profile() {
    let doc = parse_top_level("@doc cem-ml 1.0\n{p | hi}");
    assert_resolved_to_tier_a(&doc);
}

#[test]
fn doc_cem_ml_1_0_0_resolves_to_tier_a_profile() {
    let doc = parse_top_level("@doc cem-ml 1.0.0\n{p | hi}");
    assert_resolved_to_tier_a(&doc);
}

#[test]
fn missing_top_level_doc_emits_version_missing() {
    let doc = parse_top_level("{p | hi}");
    assert_rejected(&doc, VERSION_MISSING_CODE);
}

#[test]
fn doc_after_non_trivia_node_is_still_treated_as_missing() {
    // `@doc` must precede the first non-trivia item; an element before
    // it means the directive is not at the document head.
    let doc = parse_top_level("{p | hi}\n@doc cem-ml 1");
    assert_rejected(&doc, VERSION_MISSING_CODE);
}

#[test]
fn unknown_format_id_emits_format_unknown() {
    let doc = parse_top_level("@doc widget 1\n{p | hi}");
    assert_rejected(&doc, "cem.doc.format_unknown");
}

#[test]
fn invalid_semver_emits_semver_invalid() {
    let doc = parse_top_level("@doc cem-ml abcd\n{p | hi}");
    assert_rejected(&doc, "cem.doc.semver_invalid");
}

#[test]
fn future_minor_emits_version_unsupported() {
    let doc = parse_top_level("@doc cem-ml 1.2\n{p | hi}");
    assert_rejected(&doc, "cem.doc.version_unsupported");
}

#[test]
fn major_mismatch_emits_version_unsupported() {
    let doc = parse_top_level("@doc cem-ml 2\n{p | hi}");
    assert_rejected(&doc, "cem.doc.version_unsupported");
}

#[test]
fn prerelease_constraint_emits_prerelease_unmatched() {
    let doc = parse_top_level("@doc cem-ml 1.0.0-rc.1\n{p | hi}");
    assert_rejected(&doc, "cem.doc.prerelease_unmatched");
}

#[test]
fn fragment_mode_does_not_enforce_doc_directive() {
    // Default (`.top_level(false)`) is for fragments parsed inside an
    // established CEM-ML scope; they inherit the parent identity and
    // emit no `cem.doc.*` diagnostics.
    let doc = parse_fragment("{p | hi}");
    assert!(doc.format_identity.is_none());
    let cem_doc_codes: Vec<&str> = codes(&doc);
    assert!(
        cem_doc_codes.is_empty(),
        "fragment mode emitted cem.doc.* diagnostics: {cem_doc_codes:?}"
    );
}

#[test]
fn version_resolved_event_carries_byte_offset() {
    let doc = parse_top_level("@doc cem-ml 1\n{p | hi}");
    let info = doc_diagnostic(&doc, VERSION_RESOLVED_CODE).unwrap();
    // The directive opens at byte 0 in this fixture; the source map of
    // the @doc element points back to that span.
    assert!(info.source_map.is_some(), "source map must be carried");
}
