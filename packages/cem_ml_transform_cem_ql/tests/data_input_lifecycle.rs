use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    engine::{
        CemMlEngine, CheckRequest, EngineContext, EngineInput, FailLevel, ParseProjection,
        ParseRequest, ValidateProjection, ValidateRequest,
    },
    lifecycle::LoadedInputAstStream,
    parser::{
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode,
    },
    real::RealCemMlEngine,
    schema::{
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        document_model::compile_schema_document_model,
        input_validation::{
            InputValidationOutcome, InputValidationRequest, InputValidationStage,
            RUNTIME_INPUT_VALIDATION_FAILED,
        },
        registry::{CSV_SCHEMA_URI, JSON_VALUE_SCHEMA_URI, YAML_SCHEMA_URI},
    },
    value::reference_resolution::resolve_reference,
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{imported_cem_tree, retained_cem_node, ItemStream},
    schema_references::CemQlSchemaDeclarationHost,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct Stage {
    template: Arc<RetainedCemTree>,
    sources: Mutex<Vec<Arc<RetainedCemTree>>>,
}
impl Stage {
    fn new() -> Self {
        let text = "{#datadom}";
        let ast = cem_ml::parser::builder::CemAstBuilder::new(
            cem_ml::events::cem::CemEventNormalizer::new(
                cem_ml::tokenizer::cem::CemTokenizer::from_source(
                    cem_ml::source::BytesSource::new(
                        cem_ml::source::SourceId(1),
                        text.as_bytes().to_vec(),
                    ),
                ),
            ),
        )
        .build();
        Self {
            template: RetainedCemTree::new(
                ast,
                "template.cem",
                text,
                CemTreeSemantics::default(),
                None,
            )
            .unwrap(),
            sources: Mutex::new(vec![]),
        }
    }
}
impl InputValidationStage for Stage {
    fn validate(
        &self,
        request: InputValidationRequest<'_>,
    ) -> Result<InputValidationOutcome, Vec<Diagnostic>> {
        assert!(request.lexical_scopes.is_none());
        let native = request
            .source
            .native_owner()
            .unwrap()
            .downcast_ref::<LoadedInputAstStream>()
            .unwrap();
        assert!(matches!(
            native,
            LoadedInputAstStream::JsonDocument(_)
                | LoadedInputAstStream::YamlDocument(_)
                | LoadedInputAstStream::CsvDocument(_)
        ));
        let native_uri = match native {
            LoadedInputAstStream::JsonDocument(document) => &document.source.uri,
            LoadedInputAstStream::YamlDocument(document) => &document.source.uri,
            LoadedInputAstStream::CsvDocument(document) => &document.source.uri,
            _ => unreachable!(),
        };
        assert_eq!(native_uri, request.source.source_uri());
        assert!(!request
            .source
            .ast()
            .nodes
            .iter()
            .any(|n| matches!(n, CemAstNode::Reference { .. })));
        assert!(request
            .source
            .ast()
            .nodes
            .iter()
            .any(|n| matches!(n, CemAstNode::Text { data, .. } if data == "{#missing}")));
        self.sources.lock().unwrap().push(request.source.clone());
        if request.source.source_uri().starts_with("failed") {
            return Err(vec![Diagnostic {
                code: "fixture.data_stage.unavailable".into(),
                severity: Severity::Warning,
                uri: Some("vendor.cem".into()),
                line: Some(17),
                column: Some(23),
                message: "Runtime context preparation unavailable".into(),
                ..Default::default()
            }]);
        }
        let ready = !request.source.source_uri().starts_with("pending");
        let context = ready.then(|| {
            StandaloneExpressionContext::default().with_binding(
                "datadom",
                StandaloneExpressionBinding::any(ItemStream::once(imported_cem_tree(
                    request.source.clone(),
                ))),
            )
        });
        let mut host = CemQlSchemaDeclarationHost::new();
        let origin = host.register_scope(self.template.clone(), context, request.policy.clone());
        let destination = host.register_scope(request.source.clone(), None, request.policy.clone());
        assert!(host.allow_scope_crossing(origin, destination));
        let id = self
            .template
            .ast()
            .nodes
            .iter()
            .find_map(|n| match n {
                CemAstNode::Reference { node_id, .. } => Some(*node_id),
                _ => None,
            })
            .unwrap();
        let reference = host.source_reference(
            SchemaDeclarationNode::new(self.template.ast_owner().clone(), id).unwrap(),
        );
        let result = resolve_reference(reference, &mut host, request.policy.limits).unwrap();
        assert_eq!(result.is_complete(), ready);
        if ready {
            assert_eq!(result.nodes.len(), 1);
            let target = host.declaration_node(&result.nodes[0]).unwrap();
            assert!(Arc::ptr_eq(target.document(), request.source.ast_owner()));
            let query_node = retained_cem_node(&imported_cem_tree(request.source.clone())).unwrap();
            assert!(Arc::ptr_eq(query_node.owner(), &request.source));
        }
        assert!(matches!(
            self.template.ast().get(id),
            Some(CemAstNode::Reference { targets: None, .. })
        ));
        Ok(InputValidationOutcome {
            complete: result.is_complete(),
            diagnostics: result.diagnostics,
        })
    }
}
fn context(schema: &str, media: &str, stage: Arc<Stage>, ready_model: bool) -> EngineContext {
    let mut context = EngineContext::default();
    context.schema = Some(schema.into());
    context.content_type = Some(media.into());
    let mut model = compile_schema_document_model(
        schema,
        if ready_model {
            "{schema | {elements | {element @name=root}}}"
        } else {
            "{schema | {elements | {#unavailable}}}"
        },
    );
    model.schema_uri = schema.into();
    assert_eq!(model.is_ready_for_validation(), ready_model);
    context.schema_document_models.register(model);
    context.input_validation_stage = Some(stage);
    context
}
fn input(uri: &str, text: &str) -> EngineInput {
    EngineInput {
        uri: uri.into(),
        bytes: text.as_bytes().to_vec(),
        from_format: None,
        identity: None,
        root_scope: Default::default(),
    }
}
const FORMATS: [(&str, &str, &str, &str); 3] = [
    (
        JSON_VALUE_SCHEMA_URI,
        "application/json",
        r##"{"ref":"{#missing}"}"##,
        "{",
    ),
    (
        YAML_SCHEMA_URI,
        "application/yaml",
        "ref: '{#missing}'\n",
        "ref: [",
    ),
    (
        CSV_SCHEMA_URI,
        "text/csv",
        "ref\n{#missing}\n",
        "ref\n\"unterminated",
    ),
];
#[test]
fn data_runtime_stage_consumes_native_sources_on_validate_and_check_only() {
    for (schema, media, text, _) in FORMATS {
        let stage = Arc::new(Stage::new());
        let context = context(schema, media, stage.clone(), true);
        RealCemMlEngine
            .parse(ParseRequest {
                input: input("parse-only", text),
                projection: ParseProjection::Json,
                fail_level: FailLevel::Parse,
                preserve_source_offsets: true,
                presentation_scope: None,
                context: context.clone(),
            })
            .unwrap();
        assert!(stage.sources.lock().unwrap().is_empty());
        let response = RealCemMlEngine
            .validate(ValidateRequest {
                inputs: vec![input("ready", text), input("pending", text)],
                projection: ValidateProjection::Cem,
                fail_level: FailLevel::Validate,
                context: context.clone(),
            })
            .unwrap();
        assert!(
            !response
                .report
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            "{:?}",
            response.report.diagnostics
        );
        assert_eq!(
            response
                .report
                .report_ast
                .validation
                .as_ref()
                .unwrap()
                .inputs
                .iter()
                .map(|i| i.complete)
                .collect::<Vec<_>>(),
            [true, false]
        );
        let checked = RealCemMlEngine
            .check(CheckRequest {
                inputs: vec![input("checked", text)],
                projection: ValidateProjection::Cem,
                fail_level: FailLevel::Validate,
                zero_hard_violations: true,
                context,
            })
            .unwrap();
        assert!(
            checked
                .report
                .report_ast
                .validation
                .as_ref()
                .unwrap()
                .inputs[0]
                .complete
        );
        let sources = stage.sources.lock().unwrap();
        assert_eq!(sources.len(), 3);
        assert!(!Arc::ptr_eq(sources[0].ast_owner(), sources[1].ast_owner()));
        for source in sources.iter() {
            assert!(source.native_owner().is_some());
            assert!(!imported_cem_tree(source.clone())
                .source_map()
                .unwrap()
                .frames
                .is_empty());
        }
    }
}
#[test]
fn data_runtime_stage_rejects_malformed_sources_and_preserves_foreign_failure_provenance() {
    for (schema, media, text, malformed) in FORMATS {
        let stage = Arc::new(Stage::new());
        let response = RealCemMlEngine
            .validate(ValidateRequest {
                inputs: vec![input("broken", malformed), input("failed", text)],
                projection: ValidateProjection::Cem,
                fail_level: FailLevel::Validate,
                context: context(schema, media, stage.clone(), true),
            })
            .unwrap();
        assert_eq!(stage.sources.lock().unwrap().len(), 1);
        assert!(response
            .report
            .report_ast
            .validation
            .as_ref()
            .unwrap()
            .inputs
            .iter()
            .all(|i| !i.complete));
        let foreign = response
            .report
            .diagnostics
            .iter()
            .find(|d| d.code == "fixture.data_stage.unavailable")
            .unwrap();
        assert_eq!(foreign.uri.as_deref(), Some("vendor.cem"));
        assert_eq!((foreign.line, foreign.column), (Some(17), Some(23)));
        assert!(response
            .report
            .diagnostics
            .iter()
            .any(|d| d.code == RUNTIME_INPUT_VALIDATION_FAILED && d.severity.is_hard_violation()));
    }
}
#[test]
fn data_runtime_stage_does_not_admit_missing_or_incomplete_models() {
    for (schema, media, text, _) in FORMATS {
        let stage = Arc::new(Stage::new());
        let response = RealCemMlEngine
            .validate(ValidateRequest {
                inputs: vec![input("unavailable-model", text)],
                projection: ValidateProjection::Cem,
                fail_level: FailLevel::Validate,
                context: context(schema, media, stage.clone(), false),
            })
            .unwrap();
        assert!(stage.sources.lock().unwrap().is_empty());
        assert!(
            !response
                .report
                .report_ast
                .validation
                .as_ref()
                .unwrap()
                .inputs[0]
                .complete
        );
        let mut missing = EngineContext::default();
        missing.schema = Some("urn:unregistered-data-consumer".into());
        missing.content_type = Some(media.into());
        missing.input_validation_stage = Some(stage.clone());
        RealCemMlEngine
            .validate(ValidateRequest {
                inputs: vec![input("missing-model", text)],
                projection: ValidateProjection::Cem,
                fail_level: FailLevel::Validate,
                context: missing,
            })
            .unwrap();
        assert!(stage.sources.lock().unwrap().is_empty());
    }
}
