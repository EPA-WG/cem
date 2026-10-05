use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    engine::{
        CemMlEngine, CheckRequest, EngineContext, EngineInput, FailLevel, InputFormat,
        ValidateProjection, ValidateRequest,
    },
    parser::CemAstNode,
    real::RealCemMlEngine,
    schema::{
        document_model::{
            compile_schema_document_model, SchemaBehaviorEvaluator, SchemaDocumentModel,
        },
        input_validation::{InputValidationOutcome, InputValidationRequest, InputValidationStage},
    },
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
fn input(uri: &str, text: &str) -> EngineInput {
    EngineInput {
        uri: uri.into(),
        bytes: text.as_bytes().to_vec(),
        from_format: Some(InputFormat::Cem),
        identity: None,
        root_scope: Default::default(),
    }
}
fn context(stage: Arc<dyn InputValidationStage>) -> EngineContext {
    let mut context = EngineContext::default();
    context.schema = Some(cem_ml::schema::registry::CEM_ML_SCHEMA_URI.into());
    context.schema_document_models.register(compile_schema_document_model(cem_ml::schema::registry::CEM_ML_SCHEMA_URI,"{schema | {elements | {element @name=div @children=span} {element @name=span}} {field-contracts | {field-contract @name=needs-child @target=div @required-children=span}}}"));
    context.input_validation_stage = Some(stage);
    context
}
#[derive(Debug, Default)]
struct SnapshotStage {
    calls: Mutex<Vec<String>>,
    owners: Mutex<Vec<Arc<cem_ml::parser::tree::RetainedCemTree>>>,
}
impl InputValidationStage for SnapshotStage {
    fn validate(
        &self,
        request: InputValidationRequest<'_>,
    ) -> Result<InputValidationOutcome, Vec<Diagnostic>> {
        assert_eq!(
            request.model.schema_uri,
            cem_ml::schema::registry::CEM_ML_SCHEMA_URI
        );
        assert!(request.policy.limits.max_work > 0);
        let uri = request.source.source_uri().to_owned();
        self.calls.lock().unwrap().push(uri.clone());
        let owner = request.source.ast_owner();
        assert!(owner
            .nodes
            .iter()
            .filter_map(|node| if let CemAstNode::Reference { targets, .. } = node {
                Some(targets)
            } else {
                None
            })
            .all(|targets| targets.is_none()));
        self.owners.lock().unwrap().push(request.source.clone());
        match uri.as_str() {
            "pending.cem" => Ok(InputValidationOutcome {
                complete: false,
                diagnostics: vec![],
            }),
            "foreign.cem" => Ok(InputValidationOutcome {
                complete: true,
                diagnostics: vec![Diagnostic {
                    code: "fixture.original".into(),
                    severity: Severity::Warning,
                    uri: Some("https://vendor.test/library.cem".into()),
                    byte_offset: Some(300),
                    line: Some(20),
                    column: Some(4),
                    message: "original source".into(),
                    ..Default::default()
                }],
            }),
            "failure.cem" => Err(vec![]),
            "warning-failure.cem" => Err(vec![Diagnostic {
                code: "fixture.warning".into(),
                severity: Severity::Warning,
                message: "stage setup".into(),
                ..Default::default()
            }]),
            "invalid.cem" => Ok(InputValidationOutcome {
                complete: true,
                diagnostics: vec![Diagnostic {
                    code: "fixture.violation".into(),
                    severity: Severity::Error,
                    message: "complete violation".into(),
                    ..Default::default()
                }],
            }),
            _ => Ok(InputValidationOutcome {
                complete: true,
                diagnostics: vec![],
            }),
        }
    }
}
#[derive(Debug, Default)]
struct LegacyCounter(AtomicUsize);
impl SchemaBehaviorEvaluator for LegacyCounter {
    fn validate_document(
        &self,
        _: &cem_ml::parser::document::CemDocument,
        _: &SchemaDocumentModel,
    ) -> Vec<Diagnostic> {
        self.0.fetch_add(1, Ordering::SeqCst);
        vec![]
    }
}
#[test]
fn engine_runtime_stage_replaces_source_model_checks_and_reports_stable_completion() {
    let stage = Arc::new(SnapshotStage::default());
    let legacy = Arc::new(LegacyCounter::default());
    let mut context = context(stage.clone());
    context.schema_behavior_evaluator = Some(legacy.clone());
    let response = RealCemMlEngine
        .validate(ValidateRequest {
            inputs: vec![
                input("complete.cem", "{div | {#items}}"),
                input("pending.cem", "{div | {#items}}"),
                input("foreign.cem", "{div}"),
                input("invalid.cem", "{div}"),
            ],
            fail_level: FailLevel::Validate,
            projection: ValidateProjection::Cem,
            context,
        })
        .unwrap();
    let completion = &response
        .report
        .report_ast
        .validation
        .as_ref()
        .unwrap()
        .inputs;
    assert_eq!(
        completion
            .iter()
            .map(|c| (c.input.as_str(), c.complete))
            .collect::<Vec<_>>(),
        vec![
            ("complete.cem", true),
            ("pending.cem", false),
            ("foreign.cem", true),
            ("invalid.cem", true)
        ]
    );
    assert_eq!(legacy.0.load(Ordering::SeqCst), 0);
    assert_eq!(stage.calls.lock().unwrap().len(), 4);
    assert!(!response.report.diagnostics.iter().any(|d| d
        .details
        .as_ref()
        .and_then(|v| v.get("contract"))
        .and_then(|v| v.as_str())
        == Some("needs-child")));
    let foreign = response
        .report
        .diagnostics
        .iter()
        .find(|d| d.code == "fixture.original")
        .unwrap();
    assert_eq!(
        foreign.uri.as_deref(),
        Some("https://vendor.test/library.cem")
    );
    assert_eq!((foreign.line, foreign.column), (Some(20), Some(4)));
    assert_eq!(
        response.report.summary.hard_violation_count, 1,
        "{:#?}",
        response.report.diagnostics
    );
    let owners = stage.owners.lock().unwrap();
    assert_eq!(owners.len(), 4);
    assert!(owners
        .windows(2)
        .all(|owners| !Arc::ptr_eq(owners[0].ast_owner(), owners[1].ast_owner())));
}
#[test]
fn stage_preparation_failures_stay_incomplete_and_keep_original_diagnostics() {
    let stage = Arc::new(SnapshotStage::default());
    let response = RealCemMlEngine
        .check(CheckRequest {
            inputs: vec![
                input("failure.cem", "{div}"),
                input("warning-failure.cem", "{div}"),
            ],
            zero_hard_violations: true,
            fail_level: FailLevel::Validate,
            projection: ValidateProjection::Cem,
            context: context(stage),
        })
        .unwrap();
    assert!(!response.report.validation_complete());
    assert!(response
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "fixture.warning"));
    assert_eq!(
        response
            .report
            .diagnostics
            .iter()
            .filter(|d| d.code == "cem.schema_validation.runtime_stage_failed")
            .count(),
        2
    );
    assert_eq!(
        response.hard_violation_count, 2,
        "{:#?}",
        response.report.diagnostics
    );
}
#[test]
fn pending_schema_models_do_not_invoke_the_runtime_stage() {
    let stage = Arc::new(SnapshotStage::default());
    let mut context = context(stage.clone());
    context.schema_document_models = Default::default();
    context
        .schema_document_models
        .register(compile_schema_document_model(
            cem_ml::schema::registry::CEM_ML_SCHEMA_URI,
            "{schema | {elements | {#items}}}",
        ));
    let response = RealCemMlEngine
        .validate(ValidateRequest {
            inputs: vec![input("blocked.cem", "{div}")],
            fail_level: FailLevel::Validate,
            projection: ValidateProjection::Cem,
            context,
        })
        .unwrap();
    assert!(
        !response.report.validation_complete(),
        "{:#?}",
        response.report.diagnostics
    );
    assert!(stage.calls.lock().unwrap().is_empty());
}

#[test]
fn registering_runtime_stage_does_not_evaluate_parse_or_load() {
    let stage = Arc::new(SnapshotStage::default());
    RealCemMlEngine
        .parse(cem_ml::engine::ParseRequest {
            input: input("source.cem", "{div | {#items}}"),
            projection: cem_ml::engine::ParseProjection::Ast,
            fail_level: FailLevel::Validate,
            preserve_source_offsets: true,
            presentation_scope: None,
            context: context(stage.clone()),
        })
        .unwrap();
    assert!(stage.calls.lock().unwrap().is_empty());
}
#[test]
fn invalid_schema_traversal_bounds_prevent_runtime_stage_execution() {
    let stage = Arc::new(SnapshotStage::default());
    let mut context = context(stage.clone());
    context.schema_document_models.register(compile_schema_document_model(cem_ml::schema::registry::CEM_ML_SCHEMA_URI,
        "{schema | {elements | {element @name=div}} {constraints | {constraint @kind=reference-traversal-work @value=0}}}"));
    let response = RealCemMlEngine
        .validate(ValidateRequest {
            inputs: vec![input("bounds.cem", "{div | {#items}}")],
            projection: ValidateProjection::Cem,
            fail_level: FailLevel::Validate,
            context,
        })
        .unwrap();
    assert!(!response.report.validation_complete());
    assert!(stage.calls.lock().unwrap().is_empty());
    assert!(response
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.schema_validation.runtime_stage_failed" && d.source_map.is_some()));
}
