use cem_ml::{
    engine::{
        CemMlEngine, EngineContext, EngineInput, FailLevel, FormatIdentity, InspectRequest,
        InspectView, ParseProjection, ParseRequest,
    },
    real::RealCemMlEngine,
    run_config::ScopeConfig,
};

fn input(text: &str, mime: &str, schema: &str) -> EngineInput {
    EngineInput {
        uri: "studio://original/source.xml".into(),
        bytes: text.as_bytes().into(),
        identity: Some(FormatIdentity {
            content_type: Some(mime.into()),
            schema: Some(schema.into()),
            ..Default::default()
        }),
        from_format: None,
        root_scope: Default::default(),
    }
}
fn scope() -> ScopeConfig {
    ScopeConfig {
        cemt_formatter_profile: Some("tabular".into()),
        output_color_type: Some("none".into()),
        ..Default::default()
    }
}

#[test]
fn retained_xml_family_sources_do_not_acquire_cem_namespace_or_binding_diagnostics() {
    for (text, mime, schema) in [
        ("<catalog xmlns='urn:supplier'><name>🥝 {#not_a_binding}</name></catalog>", "application/xml", "https://cem.dev/ns/data/xml/1"),
        ("<html xmlns='http://www.w3.org/1999/xhtml'><head><title>Source</title></head><body><p>{#not_a_binding}</p></body></html>", "application/xhtml+xml", "https://cem.dev/ns/data/xhtml/1"),
        ("<svg xmlns='http://www.w3.org/2000/svg'><text>🥝 {#not_a_binding}</text></svg>", "image/svg+xml", "https://cem.dev/ns/data/svg/1"),
        ("<math xmlns='http://www.w3.org/1998/Math/MathML'><mi>x</mi></math>", "application/mathml+xml", "https://cem.dev/ns/data/mathml/1"),
        ("<xsl:stylesheet xmlns:xsl='http://www.w3.org/1999/XSL/Transform' version='1.0'><xsl:template match='/'><p>🥝</p></xsl:template></xsl:stylesheet>", "application/xslt+xml", "https://cem.dev/ns/transform/xslt/1"),
    ] {
        let source = input(text, mime, schema);
        let parsed = RealCemMlEngine::new().parse(ParseRequest { input: source.clone(), projection: ParseProjection::Ast,
            fail_level: FailLevel::Validate, preserve_source_offsets: true, presentation_scope: Some(scope()),
            context: EngineContext::default() }).unwrap();
        assert!(!parsed.diagnostics.iter().any(|d| d.severity.is_hard_violation()), "{schema}: {:?}", parsed.diagnostics);
        assert!(parsed.primary_bytes.is_some());
        for show in [InspectView::Ast, InspectView::Tree, InspectView::Events, InspectView::Diagnostics,
            InspectView::Summary, InspectView::SourceOffsets] {
            let inspected = RealCemMlEngine::new().inspect(InspectRequest { input: source.clone(), show,
                presentation_scope: Some(scope()), context: EngineContext::default() }).unwrap();
            let wire = serde_json::to_value(&inspected).unwrap();
            assert_eq!(wire["diagnostics"], serde_json::to_value(&parsed.diagnostics).unwrap(), "{schema} {show:?}");
            assert!(inspected.primary_bytes.is_some());
            if show == InspectView::Diagnostics {
                assert_eq!(inspected.body["diagnostics"], wire["diagnostics"]);
            }
        }
    }
}

#[test]
fn every_inspect_view_publishes_original_warnings_without_fabricating_coordinates() {
    use cem_ml::{
        diagnostics::{Diagnostic, Severity},
        engine::{SourceInspection, SourceInspectionStage},
    };
    use std::sync::Arc;
    #[derive(Debug)]
    struct Owner;
    impl SourceInspectionStage for Owner {
        fn inspect(&self, input: &EngineInput) -> Option<SourceInspection> {
            let mut diagnostics = vec![Diagnostic {
                code: "owner.absent".into(),
                severity: Severity::Warning,
                uri: Some(input.uri.clone()),
                message: "no coordinates supplied".into(),
                ..Default::default()
            }];
            diagnostics.push(Diagnostic {
                code: "owner.zero".into(),
                severity: Severity::Warning,
                uri: Some(input.uri.clone()),
                byte_offset: Some(0),
                line: Some(0),
                column: Some(0),
                message: "explicit original zero coordinates".into(),
                source_map: Some(cem_ml::source_map::SourceMapStack {
                    frames: vec![cem_ml::source_map::SourceMapFrame {
                        source_id: cem_ml::source::SourceId(1),
                        span: cem_ml::source_map::FrameSpan::Single(
                            cem_ml::source::ByteRange::new(0, 0),
                        ),
                        transform: cem_ml::source_map::TransformKind::SourceBoundary {
                            uri: input.uri.clone(),
                        },
                    }],
                }),
                ..Default::default()
            });
            Some(SourceInspection {
                identity: input.identity.clone().unwrap(),
                ast: Some(cem_ml::projection::CemTreeAstStream::new(vec![])),
                events: Some(cem_ml::projection::CemTreeAstStream::new(vec![])),
                offsets: vec![],
                diagnostics,
            })
        }
    }
    let source = input("original", "text/owner", "urn:owner");
    let context = EngineContext {
        source_inspection_stages: vec![Arc::new(Owner)],
        ..Default::default()
    };
    let parsed = RealCemMlEngine::new()
        .parse(ParseRequest {
            input: source.clone(),
            projection: ParseProjection::Ast,
            fail_level: FailLevel::Validate,
            preserve_source_offsets: true,
            presentation_scope: Some(scope()),
            context: context.clone(),
        })
        .unwrap();
    assert_eq!(parsed.diagnostics.len(), 2);
    assert_eq!(parsed.diagnostics[0].byte_offset, None);
    assert_eq!(parsed.diagnostics[0].line, None);
    assert_eq!(parsed.diagnostics[0].column, None);
    assert_eq!(parsed.diagnostics[1].byte_offset, Some(0));
    assert_eq!(parsed.diagnostics[1].line, Some(0));
    assert_eq!(parsed.diagnostics[1].column, Some(0));
    for show in [
        InspectView::Ast,
        InspectView::Tree,
        InspectView::Events,
        InspectView::Diagnostics,
        InspectView::Summary,
        InspectView::SourceOffsets,
    ] {
        let inspected = RealCemMlEngine::new()
            .inspect(InspectRequest {
                input: source.clone(),
                show,
                presentation_scope: Some(scope()),
                context: context.clone(),
            })
            .unwrap();
        let wire = serde_json::to_value(&inspected).unwrap();
        assert_eq!(
            wire["diagnostics"],
            serde_json::to_value(&parsed.diagnostics).unwrap(),
            "{show:?}"
        );
    }
}

#[test]
fn refused_import_preserves_native_diagnostics_without_publishing_a_partial_tree() {
    let source = input(
        "<r>🥝\r\n<x></r>",
        "application/xml",
        "https://cem.dev/ns/data/xml/1",
    );
    let parsed = RealCemMlEngine::new()
        .parse(ParseRequest {
            input: source.clone(),
            projection: ParseProjection::Ast,
            fail_level: FailLevel::Validate,
            preserve_source_offsets: true,
            presentation_scope: Some(scope()),
            context: EngineContext::default(),
        })
        .unwrap();
    assert!(parsed.primary.is_null());
    assert!(parsed.primary_bytes.is_none());
    assert!(parsed
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation()));
    assert!(parsed
        .diagnostics
        .iter()
        .all(|d| d.uri.as_deref() == Some(source.uri.as_str())));
    for show in [
        InspectView::Ast,
        InspectView::Tree,
        InspectView::Events,
        InspectView::Diagnostics,
    ] {
        let inspected = RealCemMlEngine::new()
            .inspect(InspectRequest {
                input: source.clone(),
                show,
                presentation_scope: Some(scope()),
                context: EngineContext::default(),
            })
            .unwrap();
        assert_eq!(
            serde_json::to_value(&inspected).unwrap()["diagnostics"],
            serde_json::to_value(&parsed.diagnostics).unwrap()
        );
        if show != InspectView::Diagnostics {
            assert!(inspected.body.is_null());
            assert!(inspected.primary_bytes.is_none());
        }
    }
}

#[test]
fn markdown_source_inspection_keeps_native_event_ranges_before_html_lowering() {
    let source = input(
        "# Café 🥝\r\n\r\n**literal {#inert}**\r\n",
        "text/markdown",
        "https://cem.dev/ns/data/markdown/1",
    );
    let inspected = RealCemMlEngine::new()
        .inspect(InspectRequest {
            input: source.clone(),
            show: InspectView::Ast,
            presentation_scope: Some(scope()),
            context: EngineContext::default(),
        })
        .unwrap();
    let text = String::from_utf8(inspected.primary_bytes.unwrap().bytes).unwrap();
    assert!(text.contains("markdown-event"), "{text}");
    assert!(text.contains("Café 🥝"), "{text}");
    assert!(text.contains("@byte-offset=2"), "{text}");
    let document =
        cem_ml::parser::builder::CemAstBuilder::new(cem_ml::events::cem::CemEventNormalizer::new(
            cem_ml::tokenizer::cem::CemTokenizer::from_source(cem_ml::source::BytesSource::new(
                cem_ml::source::SourceId(7),
                text.as_bytes().to_vec(),
            )),
        ))
        .build();
    assert!(
        document.diagnostics.is_empty(),
        "{:?}",
        document.diagnostics
    );
    let admitted = cem_ml::lifecycle::LifecycleRegistry::with_builtin_adapters()
        .load(&source, &EngineContext::default());
    assert_eq!(
        serde_json::to_value(&inspected.diagnostics).unwrap(),
        serde_json::to_value(&admitted.diagnostics).unwrap()
    );
    assert!(
        !inspected
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        inspected.diagnostics
    );
    assert!(
        !text.contains("@name=h1"),
        "source inspection must not inspect generated HTML"
    );
    let native = cem_ml::real::load_document_input(&source, &EngineContext::default());
    assert!(
        native.ast_stream.is_some(),
        "document conversion stays available separately"
    );
}
