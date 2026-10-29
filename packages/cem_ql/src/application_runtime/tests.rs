use super::*;
use crate::render::CompileTemplateOptions;
use crate::template_artifact::{
    compile_template_artifact, compile_template_artifact_with_compiled_styles,
    TemplateArtifactSourceMapMode,
};

fn compiled(source: &str) -> (Vec<u8>, String) {
    let artifact = compile_template_artifact_with_compiled_styles(
        source,
        &CompileTemplateOptions {
            host_bindings: vec!["response".into()],
            ..Default::default()
        },
        TemplateArtifactSourceMapMode::Prod,
    )
    .unwrap();
    let loaded = artifact
        .reload(&crate::template_artifact::TemplateArtifactLoadContext {
            expected_source_hash: Some(artifact.identity.source_hash.clone()),
            host_bindings: vec!["response".into()],
            source_map_mode: TemplateArtifactSourceMapMode::Prod,
        })
        .unwrap();
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let pins = PinnedTemplateArtifactContext {
        content_hash: artifact.content_hash.header_value(),
        source_hash: artifact.identity.source_hash.header_value(),
        cem_ml_version: cem_ml::VERSION.into(),
        cem_ql_version: crate::VERSION.into(),
        host_bindings: vec!["response".into()],
        source_map_mode: TemplateArtifactSourceMapMode::Prod,
    };
    (artifact.bytes, serde_json::to_string(&pins).unwrap())
}

#[test]
fn pinned_compiled_json_render_and_input_refusals() {
    let (bytes, pins) = compiled("{p | {$response.children.children.children.children.value}}");
    let output = render(
        &bytes,
        &pins,
        br#"{"name":"Tea <img>"}"#,
        "app://cart/data.json",
        "response",
    )
    .unwrap();
    assert!(output.contains("Tea &lt;img&gt;"), "{output}");
    assert!(render(&bytes, &pins, b"{", "app://cart/data.json", "response").is_err());
    assert!(render(&bytes, &pins, b"{}", "", "response").is_err());
    assert!(render(&bytes, &pins, b"{}", "app://cart/data.json", "missing").is_err());
    let mut wrong: PinnedTemplateArtifactContext = serde_json::from_str(&pins).unwrap();
    wrong.content_hash.clear();
    assert!(render(
        &bytes,
        &serde_json::to_string(&wrong).unwrap(),
        b"{}",
        "app://cart/data.json",
        "response"
    )
    .is_err());
    assert!(render(
        &bytes,
        &pins,
        &vec![b' '; MAX_INPUT_BYTES + 1],
        "app://cart/data.json",
        "response"
    )
    .is_err());
    let legacy = compile_template_artifact(
        "{p | legacy}",
        &Default::default(),
        TemplateArtifactSourceMapMode::Prod,
    );
    assert!(render(
        &legacy.bytes,
        &pins,
        b"{}",
        "app://cart/data.json",
        "response"
    )
    .unwrap_err()
    .contains("compiled CSS"));
}

#[test]
fn missing_build_selection_denies_calls_without_general_fallback() {
    let (bytes, pins) = compiled(r#"{p | {$ | str:upper("secret")}}"#);
    assert!(render(&bytes, &pins, b"{}", "app://cart/data.json", "response").is_err());
}
