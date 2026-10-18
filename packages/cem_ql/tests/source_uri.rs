use cem_ml::{
    import::{import_string, ImportStringProfile, ImportStringRequest},
    source_map::TransformKind,
};
use cem_ql::{
    eval::{imported_cem_tree, ItemStream},
    render::{
        compile_template, render_compiled_template, render_plan_to_html_with_source_map,
        CompileTemplateOptions, TemplateData,
    },
    template_artifact::{
        compile_template_artifact, TemplateArtifactLoadContext, TemplateArtifactSourceMapMode,
    },
};

#[test]
fn named_template_origins_survive_portable_reload_and_prod_maps_are_stripped() {
    let source = "{p | café Amsterdam}";
    for mode in [
        TemplateArtifactSourceMapMode::Dev,
        TemplateArtifactSourceMapMode::Prod,
    ] {
        let options = CompileTemplateOptions {
            source_uri: Some("studio://copy/templates/view.cemt".into()),
            ..Default::default()
        };
        let portable = compile_template_artifact(source, &options, mode);
        let artifact = portable
            .reload(&TemplateArtifactLoadContext {
                expected_source_hash: Some(portable.identity.source_hash.clone()),
                host_bindings: vec![],
                source_map_mode: mode,
            })
            .unwrap();
        let plan = render_compiled_template(&artifact, &TemplateData::default());
        let output = render_plan_to_html_with_source_map(&plan);
        assert_eq!(output.rendered, "<p>café Amsterdam</p>");
        let boundaries: Vec<_> = output.output_spans.iter().flat_map(|span| &span.origin.frames).filter(|frame| matches!(&frame.transform, TransformKind::SourceBoundary { uri } if uri == "studio://copy/templates/view.cemt")).collect();
        assert_eq!(
            !boundaries.is_empty(),
            mode == TemplateArtifactSourceMapMode::Dev
        );
        if mode == TemplateArtifactSourceMapMode::Dev {
            let other = compile_template_artifact(
                source,
                &CompileTemplateOptions {
                    source_uri: Some("studio://other/templates/view.cemt".into()),
                    ..Default::default()
                },
                mode,
            );
            assert_ne!(portable.content_hash, other.content_hash);
        }
    }
}

#[test]
fn copied_native_input_keeps_its_own_uri_instead_of_the_template_identity() {
    let data = "<p>Data café</p>";
    let owner = import_string(ImportStringRequest {
        source: data,
        source_uri: "studio://copy/data/input.xml",
        base_uri: None,
        profile: ImportStringProfile::Xml,
    })
    .unwrap();
    let artifact = compile_template(
        "{$input}",
        &CompileTemplateOptions {
            source_uri: Some("studio://copy/templates/view.cemt".into()),
            ..Default::default()
        },
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    let plan = render_compiled_template(
        &artifact,
        &TemplateData::default().with_binding("input", ItemStream::once(imported_cem_tree(owner))),
    );
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    let output = render_plan_to_html_with_source_map(&plan);
    assert!(output.rendered.contains("Data café"), "{}", output.rendered);
    let offset = output.rendered.find("Data café").unwrap() as u64;
    let span = output
        .output_spans
        .iter()
        .find(|span| span.output_range.start <= offset && offset < span.output_range.end())
        .unwrap();
    let frame = span.origin.frames.iter().find(|frame| matches!(&frame.transform, TransformKind::SourceBoundary { uri } if uri == "studio://copy/data/input.xml")).expect("copied text keeps its explicit input URI");
    let cem_ml::source_map::FrameSpan::Single(range) = &frame.span else {
        panic!("copied text has one authored range")
    };
    assert_eq!(
        data.get(range.start as usize..range.end() as usize)
            .unwrap(),
        "Data café"
    );
    assert_eq!(range.start, data.find("Data café").unwrap() as u64);
}
