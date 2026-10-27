use cem_ml::{
    diagnostics::Diagnostic,
    engine::*,
    real::RealCemMlEngine,
    run_config::ScopeConfig,
    schema::registry::{CEM_ELEMENT_TEMPLATE_CONTENT_TYPE, CEM_ELEMENT_TEMPLATE_SCHEMA_URI},
};
use cem_ml_transform_cem_ql::{
    engine_context_with_cem_ql_template_adapter, validate_cem_ql_template_embedding_source_bytes,
    CemQlTemplateEmbeddingIdentity, CemQlTemplateEmbeddingValidationRequest,
};

const LOGIN: &str = include_str!("../../../examples/projects/login/views/login.cem");
const URI: &str = "studio://login/views/login.cem";
fn input(text: &str) -> EngineInput {
    EngineInput {
        uri: URI.into(),
        bytes: text.as_bytes().into(),
        from_format: None,
        identity: Some(FormatIdentity {
            content_type: Some(CEM_ELEMENT_TEMPLATE_CONTENT_TYPE.into()),
            schema: Some(CEM_ELEMENT_TEMPLATE_SCHEMA_URI.into()),
            ..Default::default()
        }),
        root_scope: ScopeConfig {
            default_content_type: Some(CEM_ELEMENT_TEMPLATE_CONTENT_TYPE.into()),
            schema: Some(CEM_ELEMENT_TEMPLATE_SCHEMA_URI.into()),
            ..Default::default()
        },
    }
}
fn validate(text: &str) -> Vec<Diagnostic> {
    RealCemMlEngine::new()
        .validate(ValidateRequest {
            inputs: vec![input(text)],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: engine_context_with_cem_ql_template_adapter(),
        })
        .unwrap()
        .report
        .diagnostics
}

#[test]
fn composed_validation_reports_the_original_undeclared_login_binding_once_and_repairs() {
    let clean = validate(LOGIN);
    assert_eq!(clean.len(), 15);
    assert!(clean.iter().all(|d| !d.severity.is_hard_violation()));
    let broken = LOGIN.replace("dom:text(heading)", "dom:text(not_declared)");
    assert_ne!(broken, LOGIN);
    let expected =
        validate_cem_ql_template_embedding_source_bytes(CemQlTemplateEmbeddingValidationRequest {
            bytes: broken.as_bytes(),
            from_format: InputFormat::Cem,
            source_uri: Some(URI),
            identity: CemQlTemplateEmbeddingIdentity {
                content_type: Some(CEM_ELEMENT_TEMPLATE_CONTENT_TYPE),
                schema: Some(CEM_ELEMENT_TEMPLATE_SCHEMA_URI),
            },
        });
    let errors: Vec<_> = expected
        .iter()
        .filter(|d| d.severity.is_hard_violation())
        .collect();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "cem.ql.render.compile_failed");
    assert_eq!(errors[0].byte_offset, Some(655));
    let mut core_context = engine_context_with_cem_ql_template_adapter();
    core_context.source_validation_stages.clear();
    let core = RealCemMlEngine::new()
        .validate(ValidateRequest {
            inputs: vec![input(&broken)],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: core_context,
        })
        .unwrap();
    let actual = validate(&broken);
    let errors: Vec<_> = actual
        .iter()
        .filter(|d| d.severity.is_hard_violation())
        .collect();
    assert_eq!(errors.len(), 1, "{actual:?}");
    assert_eq!(
        serde_json::to_value(&errors).unwrap(),
        serde_json::to_value(
            expected
                .iter()
                .filter(|d| d.severity.is_hard_violation())
                .collect::<Vec<_>>()
        )
        .unwrap()
    );
    let warnings: Vec<_> = actual
        .iter()
        .filter(|d| !d.severity.is_hard_violation())
        .collect();
    assert_eq!(
        serde_json::to_value(warnings).unwrap(),
        serde_json::to_value(&core.report.diagnostics).unwrap()
    );
    let checked = RealCemMlEngine::new()
        .check(CheckRequest {
            inputs: vec![input(&broken)],
            projection: ValidateProjection::Json,
            zero_hard_violations: true,
            fail_level: FailLevel::Validate,
            context: engine_context_with_cem_ql_template_adapter(),
        })
        .unwrap();
    assert_eq!(checked.hard_violation_count, 1);
    assert_eq!(
        serde_json::to_value(&checked.report.diagnostics).unwrap(),
        serde_json::to_value(actual).unwrap()
    );
    assert_eq!(
        serde_json::to_value(validate(LOGIN)).unwrap(),
        serde_json::to_value(clean).unwrap()
    );
}

#[test]
fn runtime_and_declared_element_bindings_stay_admitted_and_malformed_source_refuses() {
    let source = r#"{attribute @name=label | Default}
{slice @name=open | false}
{button @aria-expanded="{$open}" @slice-value="{$target.value}" |
 {$label} {$hostLabel} {$datadom.attributes.label}
}"#;
    let diagnostics = validate(source);
    assert!(
        diagnostics.iter().all(|d| !d.severity.is_hard_violation()),
        "{diagnostics:?}"
    );
    let malformed = &LOGIN[..LOGIN.len() - 2];
    let diagnostics = validate(malformed);
    assert!(diagnostics
        .iter()
        .any(|d| d.code == "cem.tokenizer.unterminated_node" && d.uri.as_deref() == Some(URI)));
}

#[derive(Debug)]
struct AdmissionProbe {
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    abort: Option<cem_ml::scheduler::AbortSignal>,
    delay: bool,
}
impl SourceValidationStage for AdmissionProbe {
    fn validate(&self, _: &EngineInput) -> Option<Vec<Diagnostic>> {
        None
    }
    fn validate_admitted(
        &self,
        input: &EngineInput,
        identity: &FormatIdentity,
        format: InputFormat,
    ) -> Vec<Diagnostic> {
        assert_eq!(
            identity.schema.as_deref(),
            Some(CEM_ELEMENT_TEMPLATE_SCHEMA_URI)
        );
        assert_eq!(format, InputFormat::Cem);
        assert_eq!(input.uri, URI);
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.delay {
            std::thread::sleep(std::time::Duration::from_millis(3));
        }
        if let Some(signal) = &self.abort {
            signal.abort();
        }
        Vec::new()
    }
}
#[test]
fn admitted_source_checks_wait_for_syntax_and_keep_cancellation_and_budgets() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let probe = Arc::new(AdmissionProbe {
        calls: calls.clone(),
        abort: None,
        delay: false,
    });
    let mut context = engine_context_with_cem_ql_template_adapter();
    context.source_validation_stages.push(probe);
    let run = |input, context| {
        RealCemMlEngine::new().validate(ValidateRequest {
            inputs: vec![input],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context,
        })
    };
    assert_eq!(
        run(input(LOGIN), context.clone())
            .unwrap()
            .report
            .summary
            .hard_violation_count,
        0
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let malformed = run(input(&LOGIN[..LOGIN.len() - 2]), context).unwrap();
    assert!(malformed
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.tokenizer.unterminated_node"));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "refused syntax never reaches the supplement"
    );
    #[derive(Debug)]
    struct RefusedGrammar;
    impl SourceValidationStage for RefusedGrammar {
        fn validate(&self, input: &EngineInput) -> Option<Vec<Diagnostic>> {
            Some(vec![Diagnostic {
                uri: Some(input.uri.clone()),
                code: "test.source.refused".into(),
                severity: cem_ml::diagnostics::Severity::Error,
                message: "source admission refused".into(),
                ..Default::default()
            }])
        }
        fn validate_admitted(
            &self,
            _: &EngineInput,
            _: &FormatIdentity,
            _: InputFormat,
        ) -> Vec<Diagnostic> {
            panic!("a claimed source refusal cannot enter the supplement")
        }
    }
    let mut denied_context = engine_context_with_cem_ql_template_adapter();
    denied_context
        .source_validation_stages
        .push(Arc::new(RefusedGrammar));
    denied_context
        .source_validation_stages
        .push(Arc::new(AdmissionProbe {
            calls: calls.clone(),
            abort: None,
            delay: false,
        }));
    let denied = run(input(LOGIN), denied_context).unwrap();
    assert!(denied
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "test.source.refused" && d.severity.is_hard_violation()));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "refused source grammar never reaches the supplement"
    );
    let signal = cem_ml::scheduler::AbortSignal::new();
    let mut context =
        engine_context_with_cem_ql_template_adapter().with_abort_signal(signal.clone());
    context
        .source_validation_stages
        .push(Arc::new(AdmissionProbe {
            calls: calls.clone(),
            abort: Some(signal),
            delay: false,
        }));
    assert!(matches!(
        run(input(LOGIN), context),
        Err(EngineError::Cancelled { .. })
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let mut context = engine_context_with_cem_ql_template_adapter();
    context
        .source_validation_stages
        .push(Arc::new(AdmissionProbe {
            calls: calls.clone(),
            abort: None,
            delay: true,
        }));
    let mut source = input(LOGIN);
    source
        .root_scope
        .budgets
        .insert("validateMs".into(), "1".into());
    let outcome = run(source, context).unwrap();
    assert!(outcome
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.scope.budget_exceeded"));
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[test]
fn normalized_root_and_mime_or_schema_selectors_share_element_template_validation() {
    let broken = LOGIN.replace("dom:text(heading)", "dom:text(not_declared)");
    for selector in 0..3 {
        let mut source = input(&broken);
        match selector {
            0 => source.identity = None,
            1 => source.identity.as_mut().unwrap().schema = None,
            _ => source.identity.as_mut().unwrap().content_type = None,
        }
        let report = RealCemMlEngine::new()
            .validate(ValidateRequest {
                inputs: vec![source],
                projection: ValidateProjection::Json,
                fail_level: FailLevel::Validate,
                context: engine_context_with_cem_ql_template_adapter(),
            })
            .unwrap()
            .report;
        let errors: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|d| d.severity.is_hard_violation())
            .collect();
        assert_eq!(errors.len(), 1, "selector {selector}: {errors:?}");
        assert_eq!(errors[0].code, "cem.ql.render.compile_failed");
        assert_eq!(errors[0].uri.as_deref(), Some(URI));
        assert_eq!(errors[0].byte_offset, Some(655));
    }
}
