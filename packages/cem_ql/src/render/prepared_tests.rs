use super::*;
use crate::compile_profile::measure;

#[test]
fn prepared_baseline_is_lazy_and_owned_by_each_template() {
    let options = CompileTemplateOptions::default();
    for source in ["{p | literal}", "{p | {$ 1 + }}"] {
        let (_, stages) = measure(|| compile_template(source, &options));
        assert!(!stages.contains_key("stdlib/assemble-registry"), "{source}");
    }
    // Repeat the compilation to catch accidental sharing across templates.
    for _ in 0..2 {
        let (artifact, stages) =
            measure(|| compile_template("{p @title='{1 + 2}' | {$ 3 + 4} {$ 5 + 6}}", &options));
        assert!(
            artifact.diagnostics.is_empty(),
            "{:?}",
            artifact.diagnostics
        );
        assert_eq!(stages["query/type-check"].calls, 3);
        assert_eq!(stages["stdlib/assemble-registry"].calls, 19);
    }
}

#[test]
fn prepared_templates_preserve_portable_bytes_and_rendering() {
    use crate::api::prepared_tests::without_prepared;
    use crate::template_artifact::{
        compile_template_artifact, TemplateArtifactLoadContext, TemplateArtifactSourceMapMode,
    };
    let options = CompileTemplateOptions {
        host_bindings: vec!["host_label".into()],
        ..CompileTemplateOptions::default()
    };
    let valid = r#"{cem:variable @name=label @select='"hello"'}
        {p @title='{label}' | {$label} {$host_label} {$seq:count((1, 2))}}"#;
    for mode in [
        TemplateArtifactSourceMapMode::Dev,
        TemplateArtifactSourceMapMode::Prod,
    ] {
        for source in [valid, "{p | {$missing} {$1 + true} {$3}}"] {
            let original = without_prepared(|| compile_template_artifact(source, &options, mode));
            let prepared = compile_template_artifact(source, &options, mode);
            // Whole-envelope equality covers source frames, diagnostics,
            // host-binding metadata, IR, version stamps and content hashes.
            assert_eq!(prepared, original);
            let context = TemplateArtifactLoadContext {
                expected_source_hash: Some(prepared.identity.source_hash.clone()),
                host_bindings: options.host_bindings.clone(),
                source_map_mode: mode,
            };
            let artifact = prepared.reload(&context).unwrap();
            if source == valid {
                assert!(
                    artifact.diagnostics.is_empty(),
                    "{:?}",
                    artifact.diagnostics
                );
                let data = TemplateData::default().with_binding(
                    "host_label",
                    ItemStream::once(Item::Atomic(AtomValue::String("host".into()))),
                );
                let plan = render_compiled_template(&artifact, &data);
                assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
                let html = render_plan_to_html(&plan);
                assert!(html.contains("title=\"hello\""), "{html}");
                assert!(html.contains("host"), "{html}");
            } else {
                assert_eq!(artifact.diagnostics.len(), 2);
            }
        }
    }
}
