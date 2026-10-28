//! Authored control resources are inert typed sources, not execution requests.
use cem_ml::{
    cli_command::{CLI_COMMAND_CONTENT_TYPE, CLI_COMMAND_SCHEMA_URI},
    diagnostics::Diagnostic,
    engine::*,
    real::RealCemMlEngine,
    run_config::{ScopeConfig, RUN_CONFIG_SCHEMA_URI},
    studio_project::{
        STUDIO_PROJECT_CEM_CONTENT_TYPE, STUDIO_PROJECT_JSON_CONTENT_TYPE,
        STUDIO_PROJECT_SCHEMA_URI,
    },
};

const COMMAND: &str =
    include_str!("../schema-packages/cli-command/v1/examples/parse-ast.command.json");
const RUN: &str =
    include_str!("../../../examples/projects/portable-config/authored/check.run.json");
const PROJECT_JSON: &str =
    include_str!("../../../examples/projects/portable-config/project/card.project.json");
const PROJECT_CEM: &str =
    include_str!("../../../examples/projects/portable-config/project/card.project.cem");
const URI: &str = "studio://control-source/original.control";
fn source(text: &str, mime: &str, schema: &str) -> EngineInput {
    EngineInput {
        uri: URI.into(),
        bytes: text.as_bytes().into(),
        from_format: None,
        identity: Some(FormatIdentity {
            content_type: Some(mime.into()),
            schema: Some(schema.into()),
            ..Default::default()
        }),
        root_scope: Default::default(),
    }
}
fn cases() -> [(&'static str, &'static str, &'static str, &'static str); 4] {
    [
        (
            COMMAND,
            CLI_COMMAND_CONTENT_TYPE,
            CLI_COMMAND_SCHEMA_URI,
            "argv",
        ),
        (RUN, "application/json", RUN_CONFIG_SCHEMA_URI, "inputs"),
        (
            PROJECT_JSON,
            STUDIO_PROJECT_JSON_CONTENT_TYPE,
            STUDIO_PROJECT_SCHEMA_URI,
            "dispatch-card",
        ),
        (
            PROJECT_CEM,
            STUDIO_PROJECT_CEM_CONTENT_TYPE,
            STUDIO_PROJECT_SCHEMA_URI,
            "dispatch-card",
        ),
    ]
}
fn scope() -> ScopeConfig {
    ScopeConfig {
        output_color_type: Some("none".into()),
        ..Default::default()
    }
}
fn parsed(input: EngineInput, projection: ParseProjection) -> ParseResponse {
    RealCemMlEngine::new()
        .parse(ParseRequest {
            input,
            projection,
            fail_level: FailLevel::Validate,
            preserve_source_offsets: true,
            presentation_scope: Some(scope()),
            context: EngineContext::default(),
        })
        .unwrap()
}
fn clean(diagnostics: &[Diagnostic]) {
    assert!(
        !diagnostics.iter().any(|d| d.severity.is_hard_violation()),
        "{diagnostics:?}"
    );
}
#[test]
fn control_json_parse_and_inspect_retain_native_syntax_and_original_identity() {
    for (text, mime, schema, expected) in cases().into_iter().take(3) {
        for projection in [ParseProjection::Ast, ParseProjection::Events] {
            let response = parsed(source(text, mime, schema), projection);
            clean(&response.diagnostics);
            let bytes = response.primary_bytes.expect("native source presentation");
            let rendered = String::from_utf8(bytes.bytes).unwrap();
            assert!(
                rendered.contains(schema) && rendered.contains(mime),
                "{rendered}"
            );
            assert!(
                rendered.contains(URI) && rendered.contains(expected),
                "{rendered}"
            );
        }
        for show in [
            InspectView::Ast,
            InspectView::Tree,
            InspectView::Events,
            InspectView::Summary,
            InspectView::Diagnostics,
            InspectView::SourceOffsets,
        ] {
            let response = RealCemMlEngine::new()
                .inspect(InspectRequest {
                    input: source(text, mime, schema),
                    show,
                    presentation_scope: Some(scope()),
                    context: EngineContext::default(),
                })
                .unwrap();
            clean(&response.diagnostics);
            assert!(response.primary_bytes.is_some(), "{mime}: {show:?}");
            if show == InspectView::SourceOffsets {
                let offsets = response.body["offsets"].as_array().unwrap();
                assert!(offsets.len() > 1);
                assert!(offsets.iter().all(|r| r["byteStart"].as_u64().unwrap()
                    + r["byteLen"].as_u64().unwrap()
                    <= text.len() as u64));
            }
        }
    }
}
#[test]
fn portable_project_cem_inspect_does_not_run_generic_html_schema_validation() {
    let parsed = parsed(
        source(
            PROJECT_CEM,
            STUDIO_PROJECT_CEM_CONTENT_TYPE,
            STUDIO_PROJECT_SCHEMA_URI,
        ),
        ParseProjection::Ast,
    );
    clean(&parsed.diagnostics);
    for show in [
        InspectView::Ast,
        InspectView::Tree,
        InspectView::Events,
        InspectView::Diagnostics,
        InspectView::SourceOffsets,
    ] {
        let response = RealCemMlEngine::new()
            .inspect(InspectRequest {
                input: source(
                    PROJECT_CEM,
                    STUDIO_PROJECT_CEM_CONTENT_TYPE,
                    STUDIO_PROJECT_SCHEMA_URI,
                ),
                show,
                presentation_scope: Some(scope()),
                context: EngineContext::default(),
            })
            .unwrap();
        assert!(
            response.diagnostics.is_empty(),
            "{show:?}: {:?}",
            response.diagnostics
        );
        let text = String::from_utf8(response.primary_bytes.unwrap().bytes).unwrap();
        assert!(
            text.contains(STUDIO_PROJECT_SCHEMA_URI) && text.contains(URI),
            "{text}"
        );
        if matches!(
            show,
            InspectView::Ast | InspectView::Tree | InspectView::Events
        ) {
            assert!(
                text.contains("dispatch-card") && text.contains("generated/index.html"),
                "{text}"
            );
        }
    }
}
#[test]
fn typed_control_refusals_survive_source_operations_and_repair() {
    let mut bad_command: serde_json::Value = serde_json::from_str(COMMAND).unwrap();
    bad_command["schemaVersion"] = 2.into();
    let bad_command = bad_command.to_string();
    let mut bad_project: serde_json::Value = serde_json::from_str(PROJECT_JSON).unwrap();
    bad_project["schemaVersion"] = 2.into();
    let bad_project = bad_project.to_string();
    let duplicate =
        include_str!("../../../examples/projects/portable-config/failures/duplicate.run.json");
    let bad_cem = PROJECT_CEM.replace("@schema-version=1", "@schema-version=2");
    for ((valid, mime, schema, _), invalid, code) in cases()
        .into_iter()
        .zip([
            (
                bad_command.as_str(),
                "cem.cli_command.schema_version_unsupported",
            ),
            (duplicate, "cem.run_config.input_uri_duplicate"),
            (
                bad_project.as_str(),
                "cem.studio_project.schema_version_unsupported",
            ),
            (
                bad_cem.as_str(),
                "cem.studio_project.schema_version_unsupported",
            ),
        ])
        .map(|(a, (b, c))| (a, b, c))
    {
        let validated = RealCemMlEngine::new()
            .validate(ValidateRequest {
                inputs: vec![source(invalid, mime, schema)],
                projection: ValidateProjection::Json,
                fail_level: FailLevel::Validate,
                context: EngineContext::default(),
            })
            .unwrap();
        assert!(
            validated
                .report
                .diagnostics
                .iter()
                .any(|d| d.code == code && d.uri.as_deref() == Some(URI)),
            "{mime}: {:?}",
            validated.report.diagnostics
        );
        let response = parsed(source(invalid, mime, schema), ParseProjection::Ast);
        assert!(
            response
                .diagnostics
                .iter()
                .any(|d| d.code == code && d.uri.as_deref() == Some(URI)),
            "{mime}: {:?}",
            response.diagnostics
        );
        assert!(
            response.primary_bytes.is_none(),
            "refused sources cannot fabricate a tree"
        );
        for show in [
            InspectView::Ast,
            InspectView::Events,
            InspectView::Diagnostics,
        ] {
            let inspected = RealCemMlEngine::new()
                .inspect(InspectRequest {
                    input: source(invalid, mime, schema),
                    show,
                    presentation_scope: None,
                    context: EngineContext::default(),
                })
                .unwrap();
            assert_eq!(
                serde_json::to_value(&inspected.diagnostics).unwrap(),
                serde_json::to_value(&response.diagnostics).unwrap()
            );
            if show == InspectView::Ast {
                assert!(inspected.body.is_null());
            }
        }
        clean(&parsed(source(valid, mime, schema), ParseProjection::Ast).diagnostics);
        let malformed = parsed(source("{", mime, schema), ParseProjection::Ast);
        assert!(malformed
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()));
        assert!(malformed.primary_bytes.is_none());
    }
}

#[test]
fn control_source_formatting_uses_native_writers_and_round_trips() {
    for (text, mime, schema, expected) in cases() {
        let input = source(text, mime, schema);
        let response = RealCemMlEngine::new()
            .convert(ConvertRequest {
                target: input.identity.clone(),
                input,
                to_format: if mime == STUDIO_PROJECT_CEM_CONTENT_TYPE {
                    LayerFormat::Cem
                } else {
                    LayerFormat::Json
                },
                preserve_source_offsets: true,
                context: EngineContext::default(),
                target_scope: scope(),
                scheduler_scope_id: 0,
            })
            .unwrap();
        clean(&response.diagnostics);
        let bytes = response
            .primary_bytes
            .expect("native control source writer");
        assert_eq!(bytes.content_type, mime);
        assert_eq!(bytes.schema.as_deref(), Some(schema));
        let output = String::from_utf8(bytes.bytes).unwrap();
        assert!(output.contains(expected), "{output}");
        clean(&parsed(source(&output, mime, schema), ParseProjection::Ast).diagnostics);
        if mime == STUDIO_PROJECT_JSON_CONTENT_TYPE || mime == STUDIO_PROJECT_CEM_CONTENT_TYPE {
            assert_eq!(
                cem_ml::studio_project::parse_studio_project(output.as_bytes(), mime, schema)
                    .unwrap(),
                cem_ml::studio_project::parse_studio_project(text.as_bytes(), mime, schema)
                    .unwrap()
            );
        } else {
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&output).unwrap(),
                serde_json::from_str::<serde_json::Value>(text).unwrap()
            );
        }
    }
}

#[test]
fn control_source_selectors_literals_and_refused_identities_keep_their_owners() {
    let mut command: serde_json::Value = serde_json::from_str(COMMAND).unwrap();
    let literal = "missing://Café 🧬/{#never} {$never()}";
    command["argv"][1] = literal.into();
    let text = serde_json::to_string_pretty(&command)
        .unwrap()
        .replace('\n', "\r\n");
    let response = parsed(
        source(&text, CLI_COMMAND_CONTENT_TYPE, CLI_COMMAND_SCHEMA_URI),
        ParseProjection::Ast,
    );
    clean(&response.diagnostics);
    let presentation = String::from_utf8(response.primary_bytes.unwrap().bytes).unwrap();
    assert!(presentation.contains(literal));
    let doc =
        cem_ml::parser::builder::CemAstBuilder::new(cem_ml::events::cem::CemEventNormalizer::new(
            cem_ml::tokenizer::cem::CemTokenizer::from_source(cem_ml::source::BytesSource::new(
                cem_ml::source::SourceId(1),
                presentation.into_bytes(),
            )),
        ))
        .build();
    assert!(doc.nodes.iter().all(|node| match node {
        cem_ml::parser::CemAstNode::Attribute { value_nodes, .. } => value_nodes.is_empty(),
        _ => true,
    }));
    let position = text.find("missing://").unwrap() as u64;
    let offsets = RealCemMlEngine::new()
        .inspect(InspectRequest {
            input: source(&text, CLI_COMMAND_CONTENT_TYPE, CLI_COMMAND_SCHEMA_URI),
            show: InspectView::SourceOffsets,
            presentation_scope: None,
            context: EngineContext::default(),
        })
        .unwrap();
    assert!(offsets.body["offsets"].as_array().unwrap().iter().any(|r| {
        let start = r["byteStart"].as_u64().unwrap();
        start <= position && position < start + r["byteLen"].as_u64().unwrap() && start > 0
    }));
    for (text, mime, schema, _) in cases() {
        if mime != STUDIO_PROJECT_JSON_CONTENT_TYPE {
            let mut input = source(text, mime, schema);
            input.identity.as_mut().unwrap().content_type = None;
            clean(&parsed(input, ParseProjection::Ast).diagnostics);
        }
        if schema != RUN_CONFIG_SCHEMA_URI {
            let mut input = source(text, mime, schema);
            input.identity.as_mut().unwrap().schema = None;
            clean(&parsed(input, ParseProjection::Ast).diagnostics);
        }
        let refused = parsed(
            source(
                text,
                if schema == RUN_CONFIG_SCHEMA_URI {
                    "text/plain"
                } else {
                    mime
                },
                if schema == RUN_CONFIG_SCHEMA_URI {
                    schema
                } else {
                    "urn:unsupported-control-version"
                },
            ),
            ParseProjection::Ast,
        );
        assert!(
            refused
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            "{mime}"
        );
        assert!(refused.primary_bytes.is_none());
        assert!(!refused
            .diagnostics
            .iter()
            .any(|d| d.code.starts_with("cem.tokenizer.")
                || d.code.starts_with("cem.schema.unknown_html")));
    }
}

#[test]
fn control_source_inspection_retains_its_typed_native_model_and_cem_allocation() {
    use cem_ml::{
        control_source::{ControlSourceSyntax, ControlSourceValue},
        lifecycle::{LifecycleRegistry, LoadedInputAstStream},
    };
    use std::sync::Arc;
    for (text, mime, schema, _) in cases() {
        let mut input = source(text, mime, schema);
        input.root_scope = input
            .identity
            .as_ref()
            .map(|identity| ScopeConfig {
                default_content_type: identity.content_type.clone(),
                schema: identity.schema.clone(),
                ..Default::default()
            })
            .unwrap();
        input.identity = None;
        let loaded = LifecycleRegistry::with_builtin_adapters()
            .load_for_source_validation(&input, &EngineContext::default());
        clean(&loaded.diagnostics);
        let native = Arc::new(loaded.ast_stream.unwrap());
        let LoadedInputAstStream::ControlDocument(document) = native.as_ref() else {
            panic!("typed control owner")
        };
        assert_eq!(document.schema, schema);
        match (&document.value, schema) {
            (ControlSourceValue::Command(value), CLI_COMMAND_SCHEMA_URI) => {
                assert_eq!(value.argv[0], "parse")
            }
            (ControlSourceValue::RunConfiguration(value), RUN_CONFIG_SCHEMA_URI) => {
                assert_eq!(value.inputs[0].uri, "data/card.xml")
            }
            (ControlSourceValue::Project(value), STUDIO_PROJECT_SCHEMA_URI) => {
                assert_eq!(value.id, "dispatch-card")
            }
            _ => panic!("incorrect typed source"),
        }
        let retained = cem_ml::import::retain_lifecycle(native.clone()).unwrap();
        let owner = retained
            .native_owner()
            .unwrap()
            .clone()
            .downcast::<LoadedInputAstStream>()
            .unwrap();
        assert!(Arc::ptr_eq(&owner, &native));
        if let ControlSourceSyntax::Cem { document, .. } = &document.syntax {
            assert!(
                std::ptr::eq(retained.ast(), document.as_ref()),
                "reuse the codec's original CEM parser allocation"
            );
        }
        let signal = cem_ml::scheduler::AbortSignal::new();
        signal.abort();
        assert!(matches!(
            RealCemMlEngine::new().parse(ParseRequest {
                input,
                projection: ParseProjection::Ast,
                fail_level: FailLevel::Validate,
                preserve_source_offsets: true,
                presentation_scope: None,
                context: EngineContext::default().with_abort_signal(signal)
            }),
            Err(EngineError::Cancelled { .. })
        ));
    }
}
