//! AC-QC-V-1 - compiled artifact reload verification.

use std::fs;
use std::path::PathBuf;

use cem_ml::scheduler::ScopePolicy;
use cem_ql::api::{
    compile, compile_artifact, evaluate, reload_artifact, CompileContext, EvaluationContext,
};
use cem_ql::artifact::{CompiledArtifact, QueryArtifactFormat};
use cem_ql::eval::{ItemStream, QueryContextScope};

fn eval_source(source: &str) -> ItemStream {
    let query = compile(source, &CompileContext::default())
        .unwrap_or_else(|err| panic!("compile failed for `{source}`: {err}"));
    evaluate(&query, &eval_context())
}

fn eval_reloaded(source: &str) -> (CompiledArtifact, ItemStream) {
    let artifact = compile_artifact(source, &CompileContext::default())
        .unwrap_or_else(|err| panic!("artifact compile failed for `{source}`: {err}"));
    assert_eq!(artifact.format, QueryArtifactFormat::CemQlIrV1);
    assert!(artifact
        .content_hash
        .to_sidecar_string()
        .starts_with("cem-bin/1+blake3:"));

    // Evict the source-compiled query by leaving only artifact bytes.
    let artifact = CompiledArtifact {
        format: artifact.format,
        content_hash: artifact.content_hash.clone(),
        bytes: artifact.bytes.clone(),
    };
    let query = reload_artifact(&artifact)
        .unwrap_or_else(|err| panic!("artifact reload failed for `{source}`: {err}"));
    let stream = evaluate(&query, &eval_context());
    (artifact, stream)
}

fn eval_context() -> EvaluationContext {
    EvaluationContext {
        scope: QueryContextScope(0),
        scope_policy: ScopePolicy::host_root().with_queue_size(2048),
        diagnostics: Vec::new(),
        policy_bindings: Default::default(),
    }
}

#[test]
fn ac_qc_v_1_reloaded_artifact_matches_source_driven_corpus() {
    let corpus = [
        "1 + 2 * 3",
        r#"(1, 2, 2) | (2, 3)"#,
        r#"cemml:parse("{form | {input @id=email} {label @for=email | Email}}").target"#,
        r#"str:concat(("alpha", "beta", "gamma"), "/")"#,
    ];

    for source in corpus {
        let source_stream = eval_source(source);
        let (_, reloaded_stream) = eval_reloaded(source);
        assert_eq!(reloaded_stream, source_stream, "corpus query `{source}`");
        assert_eq!(
            diagnostics(&reloaded_stream),
            diagnostics(&source_stream),
            "diagnostics for `{source}`"
        );
    }
}

#[test]
fn ac_qc_v_1_artifact_hash_mismatch_is_rejected() {
    let (mut artifact, _) = eval_reloaded("42");
    let last = artifact.bytes.last_mut().expect("artifact has bytes");
    *last = last.wrapping_add(1);

    let err = reload_artifact(&artifact).expect_err("tampered artifact must fail");
    assert_eq!(err.code, "cem.ql.unsupported");
    assert!(err.message.contains("hash mismatch"));
}

#[test]
fn compiled_artifact_reload_target_is_registered() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("project.json");
    let text = fs::read_to_string(&project)
        .unwrap_or_else(|err| panic!("read {}: {err}", project.display()));
    assert!(
        text.contains("\"test:compiled-artifact-reload\""),
        "project.json must expose the AC-QC-V-1 verification target"
    );
}

fn diagnostics(stream: &ItemStream) -> Vec<&str> {
    stream
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}
