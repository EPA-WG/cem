use cem_ml::{
    engine::{
        CemMlEngine, EngineContext, EngineInput, FailLevel, InputFormat, ValidateProjection,
        ValidateRequest,
    },
    real::RealCemMlEngine,
    report::Report,
    schema::document_model::load_builtin_document_model_for_identity,
};

const SCHEMA: &str = "https://cem.dev/ns/runtime/data-island";
const CONTENT_TYPE: &str = "application/vnd.cem.data-island+cem";
const EXAMPLE: &str =
    include_str!("../schema-packages/cem-data-island/v1/examples/complete-instance.cem");

fn validate(source: &str, schema: Option<&str>, content_type: Option<&str>) -> Report {
    validate_with_context(source, schema, content_type, EngineContext::default())
}

fn validate_with_context(
    source: &str,
    schema: Option<&str>,
    content_type: Option<&str>,
    context: EngineContext,
) -> Report {
    let mut input = EngineInput {
        uri: "complete-instance.cem".into(),
        bytes: source.as_bytes().to_vec(),
        from_format: Some(InputFormat::Cem),
        identity: None,
        root_scope: Default::default(),
    };
    input.root_scope.schema = schema.map(str::to_owned);
    input.root_scope.default_content_type = content_type.map(str::to_owned);
    RealCemMlEngine
        .validate(ValidateRequest {
            inputs: vec![input],
            projection: ValidateProjection::Cem,
            fail_level: FailLevel::Validate,
            context,
        })
        .unwrap()
        .report
}

#[test]
fn data_island_has_a_ready_native_model() {
    let model = load_builtin_document_model_for_identity(Some(SCHEMA), None)
        .expect("registered island schema must have a native model");
    assert!(
        model.is_ready_for_validation(),
        "{:?}",
        model.compile_diagnostics
    );
    assert!(model.element("context-root").is_some());
    assert!(
        model.compile_diagnostics.is_empty(),
        "{:?}",
        model.compile_diagnostics
    );
}

#[test]
fn complete_instance_passes_with_schema_content_type_or_both() {
    for (schema, content_type) in [
        (Some(SCHEMA), None),
        (None, Some(CONTENT_TYPE)),
        (Some(SCHEMA), Some(CONTENT_TYPE)),
    ] {
        let report = validate(EXAMPLE, schema, content_type);
        assert!(
            report.diagnostics.is_empty(),
            "{schema:?}/{content_type:?}: {:?}",
            report.diagnostics
        );
        assert!(report.validation_complete());
    }
}

#[test]
fn malformed_islands_keep_structural_validation_and_source_coordinates() {
    for (source, code) in [
        (
            EXAMPLE.replace("@version=\"0.1.2\"", ""),
            "cem.schema_model.missing_required_attribute",
        ),
        (
            EXAMPLE.replacen("@kind=string", "@kind=invalid", 1),
            "cem.schema_model.invalid_attribute_value",
        ),
        (
            EXAMPLE.replace("cem-dataset:entry", "cem-dataset:unknown"),
            "cem.schema_model.unknown_element",
        ),
    ] {
        let report = validate(&source, Some(SCHEMA), Some(CONTENT_TYPE));
        let diagnostic = report
            .diagnostics
            .iter()
            .find(|d| d.code == code)
            .unwrap_or_else(|| panic!("missing {code}: {:?}", report.diagnostics));
        assert_eq!(diagnostic.uri.as_deref(), Some("complete-instance.cem"));
        assert!(diagnostic.byte_offset.is_some());
        assert!(diagnostic.line.is_some());
    }
}

#[test]
fn root_schema_claims_do_not_admit_foreign_namespaces_or_unknown_schema() {
    let foreign = EXAMPLE.replace(
        "https://cem.dev/ns/runtime/hydration-data",
        "https://example.test/unowned",
    );
    for (source, schema, content_type) in [
        (foreign.as_str(), Some(SCHEMA), Some(CONTENT_TYPE)),
        (
            EXAMPLE,
            Some("https://example.test/unknown-schema"),
            Some(CONTENT_TYPE),
        ),
        (EXAMPLE, None, None),
    ] {
        let report = validate(source, schema, content_type);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.schema.unresolved_namespace"),
            "{:?}",
            report.diagnostics
        );
    }
}

#[test]
fn embedded_html_still_receives_open_content_validation() {
    let source = EXAMPLE.replace("html:strong", "html:unknown");
    let report = validate(&source, Some(SCHEMA), Some(CONTENT_TYPE));
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema.unknown_html_element"),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn unready_selected_model_cannot_fall_back_to_builtin_namespace_claims() {
    let mut context = EngineContext::default();
    context.schema_document_models = Default::default();
    let mut model = cem_ml::schema::document_model::compile_schema_document_model(
        SCHEMA,
        include_str!("../schema-packages/cem-data-island/v1/schema/cem-data-island.cem"),
    );
    model
        .attributes
        .get_mut("kind")
        .unwrap()
        .native_type_pending = true;
    assert!(!model.is_ready_for_validation());
    context.schema_document_models.register(model);
    let report = validate_with_context(EXAMPLE, Some(SCHEMA), Some(CONTENT_TYPE), context);
    assert!(!report.validation_complete());
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.schema.unresolved_namespace"));
}

#[test]
fn pinned_read_only_resource_state_is_portable_domain_data() {
    let source = EXAMPLE.replace(
        "{cem-resources:resources |",
        r#"{cem-resources:resources |
        {cem-resources:resource @name="location:page" @kind=record |
            {cem-resources:resource @name=kind @kind=string | location}
            {cem-resources:resource @name=key @kind=string | pinned-url}
            {cem-resources:resource @name=live @kind=boolean | false}
            {cem-resources:resource @name=lastValue @kind=string | https://example.test/docs}
        }
        {cem-resources:resource @name=checkpointResources @kind=record |
            {cem-resources:resource @name=disposition @kind=string | read-again}
        }"#,
    );
    assert_ne!(source, EXAMPLE);
    let report = validate(&source, Some(SCHEMA), Some(CONTENT_TYPE));
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    assert!(report.validation_complete());
}

#[test]
fn checkpoint_control_envelope_belongs_to_the_form_domain() {
    let source = EXAMPLE
        .replace("@version=\"0.1.2\"", "@version=\"1.0.0-checkpoint.1\"")
        .replace(
            "{cem-form:form-state |",
            r#"{cem-form:form-state |
            {cem-form:field @name=controls @kind=record |
                {cem-form:field @name=version @kind=string | cem-checkpoint-controls/1}
                {cem-form:field @name=controls @kind=array}
            }"#,
        );
    assert_ne!(source, EXAMPLE);
    let report = validate(&source, Some(SCHEMA), Some(CONTENT_TYPE));
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    assert!(report.validation_complete());
}

#[test]
fn checkpoint_capability_and_reacquisition_records_use_existing_owned_domains() {
    let source = EXAMPLE
        .replace(
            "{cem-form:form-state |",
            r#"{cem-form:form-state |
        {cem-form:field @name=capability @kind=record |
            {cem-form:field @name=kind @kind=string | form-control}
            {cem-form:field @name=customValidity @kind=string | Review}
        }"#,
        )
        .replace(
            "{cem-resources:resources |",
            r#"{cem-resources:resources |
        {cem-resources:resource @name=checkpointRuntime @kind=record |
            {cem-resources:resource @name=version @kind=string | cem-checkpoint-runtime/1}
            {cem-resources:resource @name=writeDisposition @kind=string | suppress-until-new-input}
            {cem-resources:resource @name=resources @kind=array}
            {cem-resources:resource @name=writers @kind=array}
        }"#,
        );
    assert_ne!(source, EXAMPLE);
    let report = validate(&source, Some(SCHEMA), Some(CONTENT_TYPE));
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    assert!(report.validation_complete());
}
