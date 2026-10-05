use cem_ml::{
    engine::{
        CemMlEngine, EngineContext, EngineInput, FailLevel, InputFormat, ValidateProjection,
        ValidateRequest,
    },
    real::RealCemMlEngine,
    report::{InputValidationCompletion, Report, ReportOptionsSnapshot, ValidationCompletion},
    schema::document_model::compile_schema_document_model,
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
#[test]
fn source_only_validation_reports_pending_children_without_speculative_contract_errors() {
    let mut context = EngineContext::default();
    context.schema = Some("consumer".into());
    context.schema_document_models.register(compile_schema_document_model("consumer", "{schema | {elements | {element @name=div @children=input} {element @name=input}} {field-contracts | {field-contract @name=needs-input @target=div @required-children=input}}}"));
    let response = RealCemMlEngine
        .validate(ValidateRequest {
            inputs: vec![
                input("pending.cem", "{div | {#items}}"),
                input("complete.cem", "{div | {input}}"),
            ],
            fail_level: FailLevel::Validate,
            projection: ValidateProjection::Cem,
            context,
        })
        .unwrap();
    let completion = response.report.report_ast.validation.as_ref().unwrap();
    assert!(!completion.complete);
    assert_eq!(
        completion
            .inputs
            .iter()
            .map(|i| (i.input.as_str(), i.complete))
            .collect::<Vec<_>>(),
        vec![("pending.cem", false), ("complete.cem", true)]
    );
    assert!(!response.report.diagnostics.iter().any(|d| d
        .details
        .as_ref()
        .and_then(|d| d.get("contract"))
        .and_then(|d| d.as_str())
        == Some("needs-input")));
}
#[test]
fn completion_metadata_preserves_diagnostic_counts_and_legacy_reports() {
    let mut report = Report::deterministic(
        vec!["a".into()],
        vec![],
        ReportOptionsSnapshot {
            fail_level: FailLevel::Validate,
            schema: None,
            content_type: None,
            base_uri: None,
        },
    );
    assert!(report.validation_complete());
    let legacy = serde_json::to_value(&report).unwrap();
    report.report_ast.validation = Some(ValidationCompletion::from_inputs(vec![
        InputValidationCompletion {
            input: "a".into(),
            complete: false,
        },
    ]));
    assert!(!report.validation_complete());
    assert_eq!(report.summary.hard_violation_count, 0);
    assert!(serde_json::from_value::<Report>(legacy)
        .unwrap()
        .validation_complete());
}

#[derive(Debug, Default)]
struct CountingBehavior(std::sync::atomic::AtomicUsize);
impl cem_ml::schema::document_model::SchemaBehaviorEvaluator for CountingBehavior {
    fn validate_document(
        &self,
        _: &cem_ml::parser::document::CemDocument,
        _: &cem_ml::schema::document_model::SchemaDocumentModel,
    ) -> Vec<cem_ml::diagnostics::Diagnostic> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Vec::new()
    }
}

#[test]
fn source_only_stage_defers_whole_document_hooks_and_preserves_unknown_element_boundaries() {
    let evaluator = std::sync::Arc::new(CountingBehavior::default());
    let mut context = EngineContext::default();
    context.schema = Some("consumer".into());
    context.schema_behavior_evaluator = Some(evaluator.clone());
    context
        .schema_document_models
        .register(compile_schema_document_model(
        "consumer",
        "{schema | {elements | {element @name=div @children=template}}}",
    ));
    let response = RealCemMlEngine
        .validate(ValidateRequest {
            inputs: vec![
                input("pending.cem", "{div | {#items}}"),
                input("unknown.cem", "{div | {template | {#items}}}"),
            ],
            fail_level: FailLevel::Validate,
            projection: ValidateProjection::Cem,
            context,
        })
        .unwrap();
    assert_eq!(evaluator.0.load(std::sync::atomic::Ordering::SeqCst), 1);
    let inputs = &response.report.report_ast.validation.unwrap().inputs;
    assert!(!inputs[0].complete);
    assert!(inputs[1].complete);
}

#[test]
fn check_reports_pending_untyped_references_without_evaluating_them() {
    let response = RealCemMlEngine
        .check(cem_ml::engine::CheckRequest {
            projection: ValidateProjection::Cem,
            zero_hard_violations: true,
            inputs: vec![input("pending.cem", "{#items}")],
            fail_level: FailLevel::Validate,
            context: EngineContext::default(),
        })
        .unwrap();
    assert!(!response.report.validation_complete());
    assert_eq!(
        response.hard_violation_count,
        response.report.summary.hard_violation_count
    );
}
