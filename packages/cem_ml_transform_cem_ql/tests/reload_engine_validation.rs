use cem_ml::{
    ast::reload::ReloadLimits,
    engine::{CemMlEngine, EngineContext, EngineInput, ValidateRequest},
    real::RealCemMlEngine,
    schema::{
        document_model::compile_schema_document_model,
        input_validation::{InputValidationOutcome, InputValidationRequest, InputValidationStage},
    },
};
use cem_ml_transform_cem_ql::RetainedReferenceSource;
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    sync::Arc,
};

#[derive(Debug)]
struct Stage {
    owner: Arc<cem_ml::parser::document::CemDocument>,
    calls: AtomicUsize,
}
impl InputValidationStage for Stage {
    fn validate(
        &self,
        request: InputValidationRequest<'_>,
    ) -> Result<InputValidationOutcome, Vec<cem_ml::diagnostics::Diagnostic>> {
        assert!(Arc::ptr_eq(request.source.ast_owner(), &self.owner));
        assert!(Arc::ptr_eq(
            request.lexical_scopes.unwrap().document(),
            &self.owner
        ));
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(InputValidationOutcome {
            complete: true,
            diagnostics: vec![],
        })
    }
}
#[test]
fn engine_admission_reuses_the_owner_and_guards_missing_capture_before_the_stage() {
    let limits = ReloadLimits::default();
    let source =
        RetainedReferenceSource::parse(b"{#input}", "text/cem-ml", "memory:source.cem", limits)
            .unwrap();
    let stage = Arc::new(Stage {
        owner: source.ingress().source().ast_owner().clone(),
        calls: AtomicUsize::new(0),
    });
    let mut context = EngineContext::default();
    context.schema = Some("memory:schema".into());
    context
        .schema_document_models
        .register(compile_schema_document_model(
        "memory:schema",
        "{schema @name=test @namespace=urn:test @version=1.0.0 | {elements | {element @name=div}}}",
    ));
    context.input_validation_stage = Some(stage.clone());
    context
        .reload_validation_sources
        .insert("memory:request".into(), source.ingress().clone());
    let request = |context| ValidateRequest {
        inputs: vec![EngineInput {
            uri: "memory:request".into(),
            bytes: vec![0xff],
            from_format: None,
            identity: None,
            root_scope: Default::default(),
        }],
        context,
        projection: cem_ml::engine::ValidateProjection::Json,
        fail_level: cem_ml::engine::FailLevel::Validate,
    };
    let ready = RealCemMlEngine::new()
        .validate(request(context.clone()))
        .unwrap();
    assert_eq!(stage.calls.load(Ordering::SeqCst), 1);
    assert!(ready.report.validation_complete());
    let mut bundle = cem_ml::ast::reload::ReferenceReloadBundle::decode(
        &source.export_bundle(limits).unwrap(),
        limits,
    )
    .unwrap();
    bundle.lexical = None;
    let partial =
        RetainedReferenceSource::reload(&bundle.encode(limits).unwrap(), 1, limits).unwrap();
    context
        .reload_validation_sources
        .insert("memory:request".into(), partial.ingress().clone());
    let pending = RealCemMlEngine::new().validate(request(context)).unwrap();
    assert!(!pending.report.validation_complete());
    assert_eq!(stage.calls.load(Ordering::SeqCst), 1);
    assert!(pending
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.reference.missing_lexical_metadata"));
}

#[test]
fn absent_consumer_keeps_schema_overrides_incomplete_without_inherited_validation() {
    let source = RetainedReferenceSource::parse(
        b"{host @schema-src=missing.cem | {child}}",
        "text/cem-ml",
        "memory:governed.cem",
        ReloadLimits::default(),
    )
    .unwrap();
    let mut context = EngineContext::default();
    context.schema = Some("memory:schema".into());
    context
        .schema_document_models
        .register(compile_schema_document_model(
            "memory:schema",
            "{schema | {elements | {element @name=host}}}",
        ));
    context
        .reload_validation_sources
        .insert("memory:request".into(), source.ingress().clone());
    let report = RealCemMlEngine::new()
        .validate(ValidateRequest {
            inputs: vec![EngineInput {
                uri: "memory:request".into(),
                bytes: vec![0xff],
                from_format: None,
                identity: None,
                root_scope: Default::default(),
            }],
            context,
            projection: cem_ml::engine::ValidateProjection::Json,
            fail_level: cem_ml::engine::FailLevel::Validate,
        })
        .unwrap()
        .report;
    assert!(!report.validation_complete());
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.reference.lifecycle_required"));
    assert!(
        !report
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        report.diagnostics
    );
}
