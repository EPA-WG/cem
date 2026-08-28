//! Phase 3C precompiled component-template artifact verification.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use cem_ml::content_cache::ContentHash;
use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{
    compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
    TemplateData,
};
use cem_ql::template_artifact::{
    compile_template_artifact, CompiledTemplateArtifact, TemplateArtifactLoadContext,
    TemplateArtifactSourceMapMode, CEM_TEMPLATE_ARTIFACT_CONTENT_TYPE,
    CEM_TEMPLATE_ARTIFACT_VERSION,
};

const FIXTURE: &str = "packages/cem_ql/fixtures/component-template-artifact.cem";

#[test]
fn precompiled_template_matches_source_driven_render_without_source_reload() {
    let source = fixture_source();
    let options = compile_options();
    let source_driven = compile_template(&source, &options);
    let compiled = compile_template_artifact(&source, &options, TemplateArtifactSourceMapMode::Dev);

    assert_eq!(
        compiled.identity.content_type,
        CEM_TEMPLATE_ARTIFACT_CONTENT_TYPE
    );
    assert_eq!(
        compiled.identity.artifact_version,
        CEM_TEMPLATE_ARTIFACT_VERSION
    );
    assert_eq!(compiled.identity.ir_format, "cem-template-ir-v1");
    assert_eq!(
        compiled.identity.source_hash,
        ContentHash::from_blake3(source.as_bytes())
    );
    assert_eq!(
        compiled.content_hash,
        ContentHash::from_blake3(&compiled.bytes)
    );

    // Recreate the artifact from bytes only to prove reload does not retain or
    // re-read the source-driven compiler input.
    let from_bytes = CompiledTemplateArtifact::from_bytes(compiled.bytes.clone())
        .expect("artifact envelope reloads from bytes");
    let reloaded = from_bytes
        .reload(&load_context(&source))
        .expect("compiled template IR reloads");

    let data = template_data();
    assert_eq!(
        render_compiled_template(&reloaded, &data).nodes,
        render_compiled_template(&source_driven, &data).nodes
    );
}

#[test]
fn precompiled_template_preserves_static_declaration_stylesheets() {
    let source = r#"{module |
        {body |
            {style | ```
                :host { display: block; }
            ```}
            {style @scope="abc-lib" | ```
                .shared { color: green; }
            ```}
            {button | Save}
        }
    }"#;
    let options = compile_options();
    let source_driven = compile_template(source, &options);
    let compiled =
        compile_template_artifact(source, &options, TemplateArtifactSourceMapMode::Dev);
    let reloaded = CompiledTemplateArtifact::from_bytes(compiled.bytes)
        .expect("stylesheet artifact envelope reloads")
        .reload(&load_context(source))
        .expect("stylesheet artifact IR reloads");

    assert_eq!(reloaded.stylesheets, source_driven.stylesheets);
    assert_eq!(reloaded.stylesheets.len(), 2);
    let rendered = render_compiled_template(&reloaded, &TemplateData::default());
    assert!(!render_plan_to_html(&rendered).contains("<style"));
}

#[test]
fn template_artifact_bytes_are_deterministic() {
    let source = fixture_source();
    let options = compile_options();
    let first = compile_template_artifact(&source, &options, TemplateArtifactSourceMapMode::Prod);
    let second = compile_template_artifact(&source, &options, TemplateArtifactSourceMapMode::Prod);

    assert_eq!(first.bytes, second.bytes);
    assert_eq!(first.content_hash, second.content_hash);
}

#[test]
fn tampered_template_artifact_is_rejected() {
    let source = fixture_source();
    let mut artifact = compile_template_artifact(
        &source,
        &compile_options(),
        TemplateArtifactSourceMapMode::Dev,
    );
    let last = artifact.bytes.last_mut().expect("artifact contains bytes");
    *last = last.wrapping_add(1);

    let error = artifact
        .reload(&load_context(&source))
        .expect_err("tampered bytes must fail closed");
    assert_eq!(error.code, "cem.ql.template_artifact_hash_mismatch");
}

#[test]
fn source_binding_and_mode_mismatches_are_rejected() {
    let source = fixture_source();
    let artifact = compile_template_artifact(
        &source,
        &compile_options(),
        TemplateArtifactSourceMapMode::Dev,
    );

    let wrong_source = TemplateArtifactLoadContext {
        expected_source_hash: Some(ContentHash::from_blake3(b"changed source")),
        ..load_context(&source)
    };
    assert_eq!(
        artifact
            .reload(&wrong_source)
            .expect_err("source mismatch")
            .code,
        "cem.ql.template_artifact_hash_mismatch"
    );

    let wrong_bindings = TemplateArtifactLoadContext {
        host_bindings: vec!["different".to_owned()],
        ..load_context(&source)
    };
    assert_eq!(
        artifact
            .reload(&wrong_bindings)
            .expect_err("binding mismatch")
            .code,
        "cem.cc.policy_mismatch"
    );

    let wrong_mode = TemplateArtifactLoadContext {
        source_map_mode: TemplateArtifactSourceMapMode::Prod,
        ..load_context(&source)
    };
    assert_eq!(
        artifact
            .reload(&wrong_mode)
            .expect_err("mode mismatch")
            .code,
        "cem.cc.policy_mismatch"
    );
}

fn fixture_source() -> String {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("packages/cem_ql has workspace root two levels up")
        .to_path_buf();
    fs::read_to_string(workspace_root.join(FIXTURE))
        .expect("component template artifact fixture exists")
}

fn compile_options() -> CompileTemplateOptions {
    CompileTemplateOptions {
        host_bindings: vec![
            "visible".to_owned(),
            "tone".to_owned(),
            "items".to_owned(),
            "title".to_owned(),
        ],
        ..CompileTemplateOptions::default()
    }
}

fn load_context(source: &str) -> TemplateArtifactLoadContext {
    TemplateArtifactLoadContext {
        expected_source_hash: Some(ContentHash::from_blake3(source.as_bytes())),
        host_bindings: compile_options().host_bindings,
        source_map_mode: TemplateArtifactSourceMapMode::Dev,
    }
}

fn template_data() -> TemplateData {
    TemplateData {
        bindings: BTreeMap::from([
            (
                "visible".to_owned(),
                ItemStream::once(Item::Atomic(AtomValue::Boolean(true))),
            ),
            (
                "tone".to_owned(),
                ItemStream::once(Item::Atomic(AtomValue::String("raised".to_owned()))),
            ),
            (
                "title".to_owned(),
                ItemStream::once(Item::Atomic(AtomValue::String("Artifact card".to_owned()))),
            ),
            (
                "items".to_owned(),
                ItemStream::from_items(vec![
                    Item::Atomic(AtomValue::String("alpha".to_owned())),
                    Item::Atomic(AtomValue::String("beta".to_owned())),
                ]),
            ),
        ]),
    }
}
