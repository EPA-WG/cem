//! XSLT-dispatch fixture coverage (AC-P-6.8 / AC-P-V-4 / AC-P-V-7).
//!
//! Drives `examples/cem-ml/xslt-dispatch/*.cem` through the schema machine with
//! and without the explicit opt-in and asserts: an opted-in `xsl:` region is
//! dispatched as an isolated, version-pinned handoff (its descendants are not
//! interpreted), a missing `@version` is a deterministic reject, and without
//! opt-in the same region falls to the AC-P-6.7 unknown-namespace default
//! (AC-P-V-7). See the decision-core in `packages/cem_ml/src/schema/xslt.rs`.

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::events::cem::CemEventNormalizer;
use cem_ml::schema::machine::CemSchemaMachine;
use cem_ml::schema::vocab::CompiledSchema;
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;

const DISPATCHED_CODE: &str = "cem.handoff.xslt_dispatched";
const VERSION_INVALID_CODE: &str = "cem.xslt.version_invalid";
const UNRESOLVED_CODE: &str = "cem.schema.unresolved_namespace";

fn read(stem: &str) -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/cem-ml/xslt-dispatch")
        .join(format!("{stem}.cem"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"))
}

fn run(input: &str, opt_in: bool) -> Vec<Diagnostic> {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    let machine =
        CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer).with_xslt_dispatch(opt_in);
    machine.run().diagnostics
}

fn find<'a>(diags: &'a [Diagnostic], code: &str) -> Option<&'a Diagnostic> {
    diags.iter().find(|d| d.code == code)
}

fn count(diags: &[Diagnostic], code: &str) -> usize {
    diags.iter().filter(|d| d.code == code).count()
}

#[test]
fn opt_in_dispatches_a_version_pinned_isolated_region() {
    let diags = run(&read("embedded-xslt"), true);
    let dispatch = find(&diags, DISPATCHED_CODE).expect("opted-in xsl: region must dispatch");
    assert_eq!(dispatch.severity, Severity::Info);
    // AC-P-V-4: pinned to the @version (1.0), not the namespace URI.
    assert!(dispatch.message.contains("XSLT 1.0"), "message: {}", dispatch.message);
    // Exactly one dispatch (only the region root), and the xsl:template
    // descendant is isolated — no unknown-namespace disposition fires.
    assert_eq!(count(&diags, DISPATCHED_CODE), 1);
    assert_eq!(count(&diags, UNRESOLVED_CODE), 0, "descendants must be isolated");
}

#[test]
fn no_opt_in_falls_to_the_unknown_namespace_default() {
    // AC-P-V-7: without an explicit XSLT dispatch rule, the xsl: region is just
    // an unknown namespace → AC-P-6.7 disposition (reject in an application run).
    let diags = run(&read("embedded-xslt"), false);
    let reject = find(&diags, UNRESOLVED_CODE)
        .expect("without opt-in the xsl: region follows the unknown-namespace default");
    assert_eq!(reject.severity, Severity::Error);
    assert!(reject.message.contains("XSL/Transform"), "message: {}", reject.message);
    assert_eq!(count(&diags, DISPATCHED_CODE), 0, "no dispatch without opt-in");
}

#[test]
fn opt_in_without_a_version_is_a_deterministic_reject() {
    let diags = run(&read("embedded-xslt-no-version"), true);
    let invalid = find(&diags, VERSION_INVALID_CODE)
        .expect("an opted-in region with no @version cannot version-pin → reject");
    assert_eq!(invalid.severity, Severity::Error);
    assert_eq!(count(&diags, DISPATCHED_CODE), 0);
    // It is dispatched (isolated), so it does not also fall to the
    // unknown-namespace disposition.
    assert_eq!(count(&diags, UNRESOLVED_CODE), 0);
}
