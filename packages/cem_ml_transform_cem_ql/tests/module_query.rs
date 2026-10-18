use cem_ml::engine::{EngineInput, FormatIdentity, InputFormat};
use cem_ml::query::{run_query, QueryLanguage, QueryRunRequest, QuerySource};
use cem_ml::run_config::ScopeConfig;
use cem_ml::schema::registry::{
    CEM_QL_CONTENT_TYPE, CEM_QL_EXPRESSION_CONTENT_TYPE, CEM_QL_EXPRESSION_SCHEMA_URI,
    CEM_QL_SCHEMA_URI,
};
use cem_ml_transform_cem_ql::{
    engine_context_with_cem_ql_template_adapter, CemQlQueryAstOwner, CemQlQueryResultArtifact,
};

fn identity(media: &str, schema: Option<&str>) -> FormatIdentity {
    FormatIdentity {
        content_type: Some(media.into()),
        schema: schema.map(str::to_owned),
        ..Default::default()
    }
}
fn request(source: &str) -> QueryRunRequest {
    QueryRunRequest {
        data: EngineInput {
            uri: "memory:data.xml".into(),
            bytes: b"<root><child/></root>".to_vec(),
            from_format: Some(InputFormat::Xml),
            identity: Some(identity("application/xml", None)),
            root_scope: ScopeConfig {
                default_content_type: Some("application/xml".into()),
                ..Default::default()
            },
        },
        query: QuerySource {
            uri: "memory:query.cemql".into(),
            bytes: source.as_bytes().to_vec(),
            identity: identity(CEM_QL_CONTENT_TYPE, None),
        },
        context: engine_context_with_cem_ql_template_adapter(),
        context_item: None,
        bindings: Default::default(),
        limits: None,
    }
}

#[test]
fn module_and_expression_identities_are_exact_pairs() {
    let contract = QueryLanguage::CemQl.contract();
    assert!(!contract.matches_query_identity(&identity(
        CEM_QL_CONTENT_TYPE,
        Some(CEM_QL_EXPRESSION_SCHEMA_URI)
    )));
    assert!(!contract.matches_query_identity(&identity(
        CEM_QL_EXPRESSION_CONTENT_TYPE,
        Some(CEM_QL_SCHEMA_URI)
    )));
    for media in [CEM_QL_CONTENT_TYPE, "text/cem-ql"] {
        let owner = CemQlQueryAstOwner::from_source_bytes(
            b"module \"urn:test\" 1",
            "memory:query.cemql",
            identity(media, None),
            "test",
        )
        .unwrap();
        use cem_ml::query::QueryAstOwner;
        assert_eq!(owner.identity().schema.as_deref(), Some(CEM_QL_SCHEMA_URI));
    }
}

#[test]
fn common_runner_retains_module_input_and_ordered_warnings() {
    let response = run_query(request(r#"module "urn:test"
import "cem:stdlib/url" as u
declare function local:uri(x as string) { u:href(x) }
(input, local:uri("https://h"), u:with_parts("https://h:8443", {protocol: "mailto:", host: "other:70000"}))"#)).unwrap();
    assert_eq!(
        response.result.query_identity.schema.as_deref(),
        Some(CEM_QL_SCHEMA_URI)
    );
    let native = response
        .result
        .native_result
        .as_any()
        .downcast_ref::<CemQlQueryResultArtifact>()
        .unwrap();
    assert_eq!(native.stream().items.len(), 3);
    assert!(matches!(
        native.stream().items[0],
        cem_ql::eval::Item::Native(_)
    ));
    assert_eq!(
        response
            .diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .collect::<Vec<_>>(),
        vec!["cem.ql.url_setter_ignored"; 2]
    );
    for d in &response.diagnostics {
        assert_eq!(d.uri.as_deref(), Some("memory:query.cemql"));
        assert!(d.source_map.is_some());
    }
}

#[test]
fn module_compile_errors_preserve_query_uri_and_native_source_frames() {
    for (source, code) in [
        (
            b"module \"urn:test\" url:href(42)".as_slice(),
            "cem.ql.type_error",
        ),
        (
            b"module \"urn:test\" import \"file:///tmp/never-read\" as x 1".as_slice(),
            "cem.ql.import_denied",
        ),
        (b"\xff".as_slice(), "cem.ql.query_invalid_utf8"),
    ] {
        let diagnostics = CemQlQueryAstOwner::from_source_bytes(
            source,
            "memory:test.cemql",
            identity(CEM_QL_CONTENT_TYPE, None),
            "test",
        )
        .unwrap_err();
        let d = diagnostics.iter().find(|d| d.code == code).unwrap();
        assert_eq!(d.uri.as_deref(), Some("memory:test.cemql"));
        assert!(d.source_map.is_some(), "{d:?}");
    }
}

#[test]
fn module_runner_preserves_cancellation_and_result_budgets() {
    let mut limited = request("module \"urn:test\" (1, 2)");
    limited.limits = Some(cem_ml::query::QueryExecutionLimits {
        max_result_items: Some(1),
        ..Default::default()
    });
    let error = run_query(limited).unwrap_err();
    assert!(
        format!("{error:?}").contains("cem.ql.query_result_limit_exceeded"),
        "{error:?}"
    );
    let cancelled = request("module \"urn:test\" 1");
    cancelled.context.abort_signal().abort();
    let error = run_query(cancelled).unwrap_err();
    assert!(format!("{error:?}").contains("cancelled"), "{error:?}");
}

#[test]
fn query_module_validation_declares_only_its_implicit_input_host_binding() {
    use cem_ml_transform_cem_ql::{validate_cem_ql_source_bytes, CemQlSourceValidationRequest};
    let validate = |bytes, content_type, schema| {
        validate_cem_ql_source_bytes(CemQlSourceValidationRequest {
            bytes,
            source_uri: "memory:editable.cemql",
            content_type,
            schema,
        })
    };
    let source = br#"module "urn:test:editable" seq:count(input.children)"#;
    let diagnostics = validate(source, Some(CEM_QL_CONTENT_TYPE), Some(CEM_QL_SCHEMA_URI));
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let ordinary = validate(source, None, None);
    assert!(ordinary
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.ql.unknown_variable"));
    let unknown = validate(
        br#"module "urn:test:editable" misspelled_input"#,
        Some(CEM_QL_CONTENT_TYPE),
        Some(CEM_QL_SCHEMA_URI),
    );
    assert!(unknown
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.ql.unknown_variable"));
    assert!(unknown
        .iter()
        .all(|diagnostic| diagnostic.uri.as_deref() == Some("memory:editable.cemql")));
}

#[test]
fn explicit_query_budgets_respect_the_trusted_host_ceiling() {
    use cem_ml::{
        command_host_config::decode_command_host_configuration_v1,
        operation_control::OperationControl, scheduler::AbortSignal,
    };
    let config = decode_command_host_configuration_v1(Some(r#"{"rootQueueSize":1024}"#)).unwrap();
    assert_eq!(
        decode_command_host_configuration_v1(None)
            .unwrap()
            .root_policy()
            .queue_size,
        64
    );
    assert!(decode_command_host_configuration_v1(Some(r#"{"rootQueueSize":0}"#)).is_err());
    let mut request = request(r#"module "urn:test:bounded" seq:count(dom:descendants(input))"#);
    request.data.bytes = format!("<root>{}</root>", "<item/>".repeat(70)).into_bytes();
    request
        .data
        .root_scope
        .budgets
        .insert("queueSize".into(), "1024".into());
    assert!(
        run_query(request.clone()).is_err(),
        "command data cannot raise the host ceiling"
    );
    request.context.operation_control =
        OperationControl::with_root_policy(AbortSignal::new(), config.root_policy()).unwrap();
    let response = run_query(request.clone()).unwrap();
    let values = response
        .result
        .native_result
        .as_any()
        .downcast_ref::<CemQlQueryResultArtifact>()
        .unwrap()
        .stream();
    assert_eq!(values.items.len(), 1);
    request
        .data
        .root_scope
        .budgets
        .insert("queueSize".into(), "32".into());
    assert!(
        run_query(request).is_err(),
        "authored lower bounds still apply"
    );
}

#[test]
fn composed_engine_validates_query_sources_through_the_registered_native_stage() {
    use cem_ml::engine::{CemMlEngine, FailLevel, ValidateRequest};
    let mut input = request(r#"module "urn:test:validation" input"#).data;
    input.bytes = br#"module "urn:test:validation" input"#.to_vec();
    input.uri = "memory:editable.cemql".into();
    input.identity = Some(identity(CEM_QL_CONTENT_TYPE, Some(CEM_QL_SCHEMA_URI)));
    let validate = |input| {
        cem_ml::real::RealCemMlEngine
            .validate(ValidateRequest {
                inputs: vec![input],
                projection: cem_ml::engine::ValidateProjection::Json,
                fail_level: FailLevel::Validate,
                context: engine_context_with_cem_ql_template_adapter(),
            })
            .unwrap()
    };
    assert_eq!(validate(input.clone()).report.summary.error_count, 0);
    let mut conflict = input.clone();
    conflict.identity = Some(identity("application/xml", Some(CEM_QL_SCHEMA_URI)));
    assert!(validate(conflict)
        .report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.ql.query_identity_unsupported"));
    input.bytes = br#"module "urn:test:validation" unknown_input"#.to_vec();
    let failed = validate(input);
    assert!(failed
        .report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.ql.unknown_variable"));
    assert!(failed
        .report
        .diagnostics
        .iter()
        .all(|diagnostic| diagnostic.uri.as_deref() == Some("memory:editable.cemql")));
}
