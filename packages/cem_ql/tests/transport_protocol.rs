//! AC-QC-V-2 — `If-CEM-Hash` transport protocol verification.
//!
//! Mirrors the AC-QC-V-1 reload fixture but exercises the `CEM-Hash` /
//! `If-CEM-Hash` round-trip from `cem-ml-ac.md` §14 (AC-CC-6 / AC-CC-7).
//! Pass 1 fetches without `If-CEM-Hash`, the mock server returns `200`
//! with a body and `CEM-Hash`, and the loader compiles via
//! `compile_artifact` (parser entered, artifact cached). Pass 2 fetches
//! with `If-CEM-Hash` populated, the server responds `304` with an
//! empty body, and the loader resolves the query from cache via
//! `reload_artifact` — the cem-ql parser MUST NOT be entered on the
//! second pass.

use std::fs;
use std::path::PathBuf;

use cem_ml::content_cache::ContentHash;
use cem_ml::scheduler::ScopePolicy;
use cem_ql::api::{compile_artifact, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{ItemStream, QueryContextScope};
use cem_ql::transport::{ArtifactLoader, InMemoryTransport, LoadOutcome};

const URI: &str = "cem-ql://ac-qc-v-2/fixture.ql";

fn eval_context() -> EvaluationContext {
    EvaluationContext {
        scope: QueryContextScope(0),
        scope_policy: ScopePolicy::host_root().with_queue_size(2048),
        diagnostics: Vec::new(),
        policy_bindings: Default::default(),
    }
}

fn diagnostic_codes(stream: &ItemStream) -> Vec<&str> {
    stream
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

fn publish_source(transport: &mut InMemoryTransport, source: &str) -> ContentHash {
    let artifact = compile_artifact(source, &CompileContext::default())
        .unwrap_or_else(|err| panic!("artifact precompile failed: {err}"));
    transport.publish(
        URI,
        source.as_bytes().to_vec(),
        artifact.content_hash.clone(),
    );
    artifact.content_hash
}

#[test]
fn ac_qc_v_2_if_cem_hash_round_trip_skips_parser_on_pass_two() {
    let source = r#"str:concat(("alpha", "beta", "gamma"), "/")"#;

    let mut transport = InMemoryTransport::new();
    let published_hash = publish_source(&mut transport, source);

    let mut loader = ArtifactLoader::new(transport);
    let context = CompileContext::default();

    let (pass1_query, outcome1) = loader
        .load(URI, &context)
        .expect("pass 1: loader resolves source body");
    assert_eq!(outcome1, LoadOutcome::Compiled, "pass 1 must compile");

    let cached_hash = loader
        .cached_hash(URI)
        .expect("pass 1 must populate cache")
        .clone();
    assert_eq!(
        cached_hash, published_hash,
        "cached artifact hash must equal the server's CEM-Hash",
    );

    let (pass2_query, outcome2) = loader
        .load(URI, &context)
        .expect("pass 2: loader resolves from cache");
    assert_eq!(outcome2, LoadOutcome::CacheHit, "pass 2 must hit cache");

    let stream1 = evaluate(&pass1_query, &eval_context());
    let stream2 = evaluate(&pass2_query, &eval_context());
    assert_eq!(stream2, stream1, "reloaded query must match source-driven");
    assert_eq!(
        diagnostic_codes(&stream2),
        diagnostic_codes(&stream1),
        "diagnostics must match across passes",
    );

    let telemetry = loader.telemetry();
    assert_eq!(telemetry.compiled(), 1, "parser entered exactly once");
    assert_eq!(telemetry.cache_hits(), 1, "cache hit on pass two");
    assert_eq!(
        telemetry.conditional_requests(),
        1,
        "If-CEM-Hash sent exactly once",
    );

    // Confirm the wire-level header sequence the loader sent.
    let requests = loader.transport().requests();
    assert_eq!(requests.len(), 2, "two transport round-trips");
    assert!(
        requests[0].if_cem_hash.is_none(),
        "pass 1 must omit If-CEM-Hash",
    );
    let pass2_hash = requests[1]
        .if_cem_hash
        .as_ref()
        .expect("pass 2 must send If-CEM-Hash");
    assert_eq!(
        *pass2_hash, published_hash,
        "If-CEM-Hash must equal the cached artifact hash",
    );
}

#[test]
fn ac_qc_v_2_server_hash_mismatch_fails_closed() {
    let source = "42";

    let mut transport = InMemoryTransport::new();
    // Publish the body under a deliberately wrong CEM-Hash so the
    // engine's recomputed hash diverges and the loader MUST refuse.
    transport.publish(
        URI,
        source.as_bytes().to_vec(),
        ContentHash::from_blake3(b"unrelated bytes"),
    );

    let mut loader = ArtifactLoader::new(transport);
    let err = loader
        .load(URI, &CompileContext::default())
        .expect_err("mismatched CEM-Hash must fail closed");
    assert_eq!(err.code, "cem.cc.hash_mismatch");
    assert_eq!(loader.telemetry().compiled(), 0, "no artifact cached");
}

#[test]
fn ac_qc_v_2_target_is_registered() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("project.json");
    let text = fs::read_to_string(&project)
        .unwrap_or_else(|err| panic!("read {}: {err}", project.display()));
    assert!(
        text.contains("\"test:transport-protocol\""),
        "project.json must expose the AC-QC-V-2 verification target",
    );
}
