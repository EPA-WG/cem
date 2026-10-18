use cem_ml::{
    engine::*,
    real::RealCemMlEngine,
    run_config::ScopeConfig,
    schema::registry::{
        CEM_QL_CONTENT_TYPE, CEM_QL_EXPRESSION_CONTENT_TYPE, CEM_QL_EXPRESSION_SCHEMA_URI,
        CEM_QL_SCHEMA_URI,
    },
};
use cem_ml_transform_cem_ql::engine_context_with_cem_ql_template_adapter;
const URI: &str = "studio://login/queries/source.cemql";
fn input(source: &[u8], expression: bool) -> EngineInput {
    EngineInput {
        uri: URI.into(),
        bytes: source.to_vec(),
        from_format: None,
        identity: Some(FormatIdentity {
            content_type: Some(
                if expression {
                    CEM_QL_EXPRESSION_CONTENT_TYPE
                } else {
                    CEM_QL_CONTENT_TYPE
                }
                .into(),
            ),
            schema: Some(
                if expression {
                    CEM_QL_EXPRESSION_SCHEMA_URI
                } else {
                    CEM_QL_SCHEMA_URI
                }
                .into(),
            ),
            ..Default::default()
        }),
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
fn parse(input: EngineInput, projection: ParseProjection) -> ParseResponse {
    RealCemMlEngine::new()
        .parse(ParseRequest {
            input,
            projection,
            fail_level: FailLevel::Parse,
            preserve_source_offsets: true,
            presentation_scope: Some(scope()),
            context: engine_context_with_cem_ql_template_adapter(),
        })
        .unwrap()
}
#[test]
fn native_source_ast_and_events_preserve_identity_and_unicode_offsets() {
    for (source, expression) in [
        ("module \"urn:login\"\nlet title = \"🍒\"; title", false),
        ("let title = \"🍒\"; title", true),
    ] {
        for projection in [ParseProjection::Ast, ParseProjection::Events] {
            let response = parse(input(source.as_bytes(), expression), projection);
            assert!(
                response.diagnostics.is_empty(),
                "{:?}",
                response.diagnostics
            );
            let output = String::from_utf8(response.primary_bytes.unwrap().bytes).unwrap();
            assert!(output.contains(URI), "{output}");
            assert!(output.contains("🍒"), "{output}");
            assert!(
                output.contains(if expression {
                    CEM_QL_EXPRESSION_SCHEMA_URI
                } else {
                    CEM_QL_SCHEMA_URI
                }),
                "{output}"
            );
            assert!(output.contains("byte-offset"), "{output}");
            assert!(!output.contains('\u{1b}'));
            if projection == ParseProjection::Ast {
                assert!(output.contains("@kind=let"), "{output}");
            } else {
                assert!(output.contains("StringLit"), "{output}");
            }
        }
    }
}
#[test]
fn native_source_inspect_views_are_inert_and_typed() {
    let source = b"module \"urn:login\"\nunknownFunction(input)";
    for show in [
        InspectView::Summary,
        InspectView::Ast,
        InspectView::Events,
        InspectView::Diagnostics,
        InspectView::SourceOffsets,
        InspectView::Tree,
    ] {
        let response = RealCemMlEngine::new()
            .inspect(InspectRequest {
                input: input(source, false),
                show,
                presentation_scope: Some(scope()),
                context: engine_context_with_cem_ql_template_adapter(),
            })
            .unwrap();
        let output = String::from_utf8(response.primary_bytes.unwrap().bytes).unwrap();
        assert!(output.contains(URI), "{show:?}: {output}");
        assert!(
            !output.contains("cem.ql.unbound"),
            "parse must not bind or evaluate: {output}"
        );
        if matches!(show, InspectView::Ast | InspectView::Tree) {
            assert!(output.contains("@kind=call"), "{output}");
        }
    }
}
#[test]
fn native_source_reports_parse_errors_without_module_wrappers() {
    for (source, expression) in [
        ("1 +", true),
        ("1 2", true),
        ("module \"urn:bad\" 1", true),
        ("1", false),
    ] {
        let response = parse(input(source.as_bytes(), expression), ParseProjection::Ast);
        assert!(
            response
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            "{source}"
        );
        assert!(response
            .diagnostics
            .iter()
            .all(|d| d.uri.as_deref() == Some(URI)));
        if expression {
            assert!(!response
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.ql.module_uri_missing"));
        }
    }
    let response = parse(input(b"\xff", true), ParseProjection::Ast);
    assert!(response
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.invalid_utf8"));
    let mut mismatched = input(b"1", true);
    mismatched.identity.as_mut().unwrap().schema = Some(CEM_QL_SCHEMA_URI.into());
    assert!(parse(mismatched, ParseProjection::Ast)
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.query_identity_unsupported"));
}
#[test]
fn native_source_inspection_obeys_cancellation() {
    let mut limited = input(b"1", true);
    limited
        .root_scope
        .budgets
        .insert("parseMs".into(), "0".into());
    let limited = parse(limited, ParseProjection::Ast);
    assert!(limited
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.scope.budget_exceeded" && d.uri.as_deref() == Some(URI)));
    let context = engine_context_with_cem_ql_template_adapter();
    context.abort_signal().abort();
    assert!(RealCemMlEngine::new()
        .inspect(InspectRequest {
            input: input(b"1", true),
            show: InspectView::Ast,
            presentation_scope: Some(scope()),
            context
        })
        .is_err());
}

#[test]
fn native_source_retains_typed_nodes_original_frames_and_explicit_export_boundaries() {
    use cem_ml::{
        projection::CemTreeAstNode,
        source_map::{FrameSpan, TransformKind},
    };
    let source = "let title = \"🍒\"; title";
    let context = engine_context_with_cem_ql_template_adapter();
    let retained = context.source_inspection_stages[0]
        .inspect(&input(source.as_bytes(), true))
        .unwrap();
    let events = retained.events.as_ref().unwrap().as_nodes().last().unwrap();
    let reconstructed: String = events
        .children()
        .iter()
        .map(|event| {
            assert!(matches!(event, CemTreeAstNode::Element { name, .. } if name == "event"));
            event
                .attributes()
                .iter()
                .find(|a| a.name == "value")
                .unwrap()
                .value
                .as_deref()
                .unwrap()
        })
        .collect();
    assert_eq!(reconstructed, source);
    let ast = retained.ast.unwrap();
    let root = ast.as_nodes().last().unwrap();
    let literal = root
        .children()
        .iter()
        .find(|node| {
            node.attributes()
                .iter()
                .any(|a| a.name == "kind" && a.value.as_deref() == Some("literal"))
        })
        .unwrap();
    let CemTreeAstNode::Element { source: map, .. } = literal else {
        panic!("typed node")
    };
    let frame = &map.frames[0];
    assert_eq!(
        frame.transform,
        TransformKind::SourceBoundary { uri: URI.into() }
    );
    let FrameSpan::Single(range) = frame.span else {
        panic!("authored range")
    };
    assert_eq!(
        &source.as_bytes()[range.start as usize..range.end() as usize],
        "\"🍒\"".as_bytes()
    );
    assert_eq!(range.len, 6);
    assert!(retained.offsets.contains(&range));
    let function = "module \"urn:typed\" declare function echo(item as string) { item }";
    let typed = context.source_inspection_stages[0]
        .inspect(&input(function.as_bytes(), false))
        .unwrap();
    assert!(typed.diagnostics.is_empty());
    let typed_ast = typed.ast.unwrap();
    let parameter = typed_ast
        .as_nodes()
        .last()
        .unwrap()
        .children()
        .iter()
        .find(|node| {
            node.attributes()
                .iter()
                .any(|a| a.name == "kind" && a.value.as_deref() == Some("parameter"))
        })
        .unwrap();
    let CemTreeAstNode::Element { source: map, .. } = parameter else {
        panic!("typed parameter")
    };
    let FrameSpan::Single(range) = map.frames[0].span else {
        panic!("parameter range")
    };
    assert_eq!(
        &function[range.start as usize..range.end() as usize],
        "item as string"
    );
    let dom = parse(input(source.as_bytes(), true), ParseProjection::DomJson);
    assert!(dom
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.source.dom_projection_unsupported"));
    let json = RealCemMlEngine::new()
        .parse(ParseRequest {
            input: input(source.as_bytes(), true),
            projection: ParseProjection::Ast,
            fail_level: FailLevel::Parse,
            preserve_source_offsets: true,
            presentation_scope: None,
            context,
        })
        .unwrap();
    assert!(json.primary_bytes.is_none());
    assert!(json.primary.to_string().contains("literal"));
}

#[test]
fn native_source_identity_aliases_and_rejection_do_not_fall_back_to_document_import() {
    for identity in [
        FormatIdentity {
            content_type: Some("text/cem-ql; charset=utf-8".into()),
            ..Default::default()
        },
        FormatIdentity {
            schema: Some(CEM_QL_SCHEMA_URI.into()),
            ..Default::default()
        },
    ] {
        let mut source = input(b"module \"urn:login\" 1", false);
        source.identity = Some(identity);
        assert!(parse(source, ParseProjection::Ast).diagnostics.is_empty());
    }
    let mut relative = input(b"1", true);
    relative.uri = "source.cemql".into();
    relative.root_scope.base_uri = Some("studio://login/queries/".into());
    let output = parse(relative, ParseProjection::Ast).primary_bytes.unwrap();
    assert!(String::from_utf8(output.bytes).unwrap().contains(URI));
    let mut wrong = input(b"1", true);
    wrong.identity.as_mut().unwrap().schema = Some("urn:unsupported".into());
    assert!(RealCemMlEngine::new()
        .inspect(InspectRequest {
            input: wrong,
            show: InspectView::Ast,
            presentation_scope: Some(scope()),
            context: engine_context_with_cem_ql_template_adapter()
        })
        .is_err());
    let expression = input("\"🍒\" +".as_bytes(), true);
    let broken = parse(expression, ParseProjection::Ast);
    let error = broken
        .diagnostics
        .iter()
        .find(|d| d.byte_offset.is_some())
        .unwrap();
    assert_eq!(error.uri.as_deref(), Some(URI));
    assert_eq!(error.byte_offset, Some(8));
    assert_eq!(error.column, Some(7));
    assert_eq!(error.line, Some(1));
    assert!(error.source_map.is_some());
    assert!(
        parse(input("\"🍒\"".as_bytes(), true), ParseProjection::Ast)
            .diagnostics
            .is_empty()
    );
}
