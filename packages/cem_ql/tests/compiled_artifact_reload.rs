//! AC-QC-V-1 - compiled artifact reload verification.

use std::fs;
use std::path::PathBuf;

use cem_ml::content_cache::ContentHash;
use cem_ml::scheduler::ScopePolicy;
use cem_ql::api::{
    compile, compile_artifact, evaluate, reload_artifact, reload_artifact_with_context,
    CompileContext, EvaluationContext,
};
use cem_ql::artifact::{CompiledArtifact, QueryArtifactFormat};
use cem_ql::eval::{ItemStream, QueryContextScope};
use cem_ql::resolve::ImportPolicy;
use cem_ql::types::TyConfig;

const PACKAGE_FIXTURE: &str =
    "packages/cem_ml/schema-packages/cem-ql/v1/examples/compiled-artifact-identity.cemql";

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
    let artifact = artifact.clone();
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
        current_item: None,
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
    assert_eq!(err.code, "cem.ql.artifact_hash_mismatch");
    assert!(err.message.contains("hash mismatch"));
}

#[test]
fn ac_qc_v_1_package_fixture_declares_stable_identity_stamps() {
    let source = package_fixture_source();
    let context = package_fixture_context(&source);

    let artifact = compile_artifact(&source, &context)
        .unwrap_or_else(|err| panic!("package fixture artifact compiles: {err}"));
    assert_eq!(artifact.format, QueryArtifactFormat::CemQlIrV1);
    assert_eq!(
        artifact.identity.content_type,
        "application/vnd.cem.query-artifact+cem-bin"
    );
    assert_eq!(artifact.identity.artifact_version, "cem-ql-artifact/1");
    assert_eq!(artifact.identity.ir_format, "cem-ql-ir-v1");
    assert_eq!(
        artifact.identity.schema_uri,
        "https://cem.dev/ns/query/cem-ql/1"
    );
    assert_eq!(artifact.identity.schema_version, "1.0.0");
    assert_eq!(artifact.identity.compiler_version, cem_ql::VERSION);
    assert_eq!(
        artifact.identity.source_hash,
        ContentHash::from_blake3(source.as_bytes())
    );
    assert_eq!(
        artifact.identity.source_uri.as_deref(),
        Some(PACKAGE_FIXTURE)
    );
    assert_eq!(
        artifact.identity.module_uri.as_deref(),
        Some("https://example.test/queries/compiled-artifact-identity")
    );
    assert_eq!(artifact.identity.cache_mode, "prod");
    assert_eq!(artifact.identity.source_map_mode, "none");
    assert!(artifact
        .identity
        .import_policy_stamp
        .starts_with("import-policy/1;"));
    assert_eq!(artifact.identity.import_closure, "imports/1;");
    assert!(artifact
        .identity
        .stdlib_overlay_fingerprint
        .starts_with("cem:stdlib/all-known@"));
    assert!(artifact.identity.type_profile.starts_with("ty-config/1;"));
    assert_eq!(
        artifact.content_hash,
        ContentHash::from_blake3(&artifact.bytes),
        "artifact hash must cover the full identity envelope"
    );

    reload_artifact_with_context(&artifact, &context).expect("identity fixture reloads");
}

#[test]
fn ac_qc_v_1_source_hash_mismatch_is_rejected() {
    let source = package_fixture_source();
    let context = package_fixture_context(&source);
    let artifact = compile_artifact(&source, &context)
        .unwrap_or_else(|err| panic!("package fixture artifact compiles: {err}"));
    let changed_source = source.replace("artifact identity", "artifact identity changed");
    let changed_context = CompileContext {
        expected_source_hash: Some(ContentHash::from_blake3(changed_source.as_bytes())),
        ..context
    };

    let err = reload_artifact_with_context(&artifact, &changed_context)
        .expect_err("changed source bytes must reject cached artifact");
    assert_eq!(err.code, "cem.ql.artifact_hash_mismatch");
    assert!(err.message.contains("source hash"));
}

#[test]
fn ac_qc_v_1_policy_and_type_profile_mismatch_reject_cached_artifacts() {
    let source = package_fixture_source();
    let context = package_fixture_context(&source);
    let artifact = compile_artifact(&source, &context)
        .unwrap_or_else(|err| panic!("package fixture artifact compiles: {err}"));

    let policy_mismatch = CompileContext {
        import_policy: ImportPolicy::new().allow_scheme("https").unwrap(),
        ..context.clone()
    };
    let err = reload_artifact_with_context(&artifact, &policy_mismatch)
        .expect_err("import policy drift must reject cached artifact");
    assert_eq!(err.code, "cem.cc.policy_mismatch");

    let type_profile_mismatch = CompileContext {
        type_config: TyConfig::dev_profile(),
        ..context
    };
    let err = reload_artifact_with_context(&artifact, &type_profile_mismatch)
        .expect_err("type-check profile drift must reject cached artifact");
    assert_eq!(err.code, "cem.cc.policy_mismatch");
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

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("packages/cem_ql has workspace root two levels up")
        .to_path_buf()
}

fn package_fixture_source() -> String {
    fs::read_to_string(workspace_root().join(PACKAGE_FIXTURE))
        .unwrap_or_else(|err| panic!("read package fixture {PACKAGE_FIXTURE}: {err}"))
}

fn package_fixture_context(source: &str) -> CompileContext {
    CompileContext {
        source_uri: Some(PACKAGE_FIXTURE.to_owned()),
        expected_source_hash: Some(ContentHash::from_blake3(source.as_bytes())),
        ..CompileContext::default()
    }
}
