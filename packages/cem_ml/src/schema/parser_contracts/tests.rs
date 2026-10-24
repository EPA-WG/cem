use crate::{
    engine::{EngineContext, EngineInput, FormatIdentity},
    lifecycle::{LifecycleRegistry, LoadedInputAstStream},
    schema::{
        document_model::{
            compile_schema_document_model, compile_schema_document_model_from_retained,
        },
        package_compilation::SchemaPackageSourceRegistry,
        package_sources::builtin_schema_package_source,
        registry::{CSV_SCHEMA_URI, JSON_VALUE_SCHEMA_URI},
    },
};
use std::sync::Arc;

fn source() -> &'static str {
    builtin_schema_package_source("csv").unwrap().schema_source
}

fn retained(source: &str) -> Arc<crate::parser::tree::RetainedCemTree> {
    SchemaPackageSourceRegistry::default()
        .retain("memory://owned/csv-schema.cem", source)
        .unwrap()
}

#[test]
fn csv_parser_contract_compiles_from_the_original_source_and_inspection_stays_unbound() {
    let tree = retained(source());
    let model = compile_schema_document_model_from_retained(CSV_SCHEMA_URI, &tree);
    assert!(model.is_ready_for_validation());
    assert!(!model.is_ready_for_structural_validation());
    let projection = retained("{table @encoding=utf-8 @delimiter=\",\"}");
    let blocked = crate::schema::document_model::validate_document_model(projection.ast(), &model);
    assert!(blocked.iter().any(
        |d| d.code == crate::schema::document_model::MODEL_NOT_READY_CODE
            && d.message.contains("source import")
    ));
    assert!(
        !model
            .compile_diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        model.compile_diagnostics
    );
    let contract = model.parser_contract.as_ref().unwrap();
    assert!(Arc::ptr_eq(contract.source_owner().unwrap(), &tree));
    let other = retained(source());
    assert!(!contract.matches_owner(other.ast_owner()));
    let inspection = compile_schema_document_model(CSV_SCHEMA_URI, source());
    assert!(!inspection.is_ready_for_validation());
    assert!(inspection
        .parser_contract
        .as_ref()
        .unwrap()
        .source_owner()
        .is_none());
    let synthetic = crate::schema::document_model::SchemaDocumentModel {
        schema_uri: CSV_SCHEMA_URI.into(),
        ..Default::default()
    };
    assert!(!synthetic.is_ready_for_validation());
}

#[test]
fn csv_parser_contract_refuses_unknown_missing_duplicate_or_unsafe_fact_bindings() {
    let header = source()
        .lines()
        .find(|line| line.contains("constraint @kind=\"header-parameter-values\""))
        .unwrap();
    for invalid in [
        source().replace("fact-kind=\"unclosed-quote\"", "fact-kind=\"unrecognized\""),
        source().replace(header, ""),
        source().replace(
            header,
            &format!(
                "{header}\n{}",
                header.replace("header-parameter-values", "duplicated")
            ),
        ),
        source().replace(
            "@code=\"cem.csv.unclosed_quote\" @severity=\"error\"",
            "@code=\"cem.csv.unclosed_quote\" @severity=\"warning\"",
        ),
        source().replace(
            "@behavior=\"csv-parse-report-fact\"",
            "@behavior=\"unregistered-parser-fact\"",
        ),
    ] {
        let model =
            compile_schema_document_model_from_retained(CSV_SCHEMA_URI, &retained(&invalid));
        assert!(!model.is_ready_for_validation(), "{invalid}");
        assert!(
            model
                .compile_diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            "{:?}",
            model.compile_diagnostics
        );
    }
    let foreign =
        compile_schema_document_model_from_retained(JSON_VALUE_SCHEMA_URI, &retained(source()));
    assert!(foreign.parser_contract.is_none());
    assert!(foreign
        .compile_diagnostics
        .iter()
        .any(|d| d.code == "cem.schema_definition.unknown_diagnostic_behavior"));
}

#[test]
fn csv_parser_contract_drives_native_lifecycle_facts_from_owned_diagnostics() {
    let customized = source()
        .replace("cem.csv.invalid_header_parameter", "example.csv.header")
        .replace(
            "@code=\"example.csv.header\" @severity=\"warning\"",
            "@code=\"example.csv.header\" @severity=\"error\"",
        );
    let owner = retained(&customized);
    let model = compile_schema_document_model_from_retained(CSV_SCHEMA_URI, &owner);
    assert!(model.is_ready_for_validation());
    let mut context = EngineContext::default();
    context.schema_document_models.register(model);
    let input = EngineInput {
        uri: "memory://source/table.csv".into(),
        bytes: b"id,name\n1,Ada\n".to_vec(),
        from_format: None,
        identity: Some(FormatIdentity {
            content_type: Some("text/csv; header=maybe".into()),
            schema: Some(CSV_SCHEMA_URI.into()),
            ..Default::default()
        }),
        root_scope: Default::default(),
    };
    for loaded in [
        LifecycleRegistry::with_builtin_adapters().load(&input, &context),
        LifecycleRegistry::with_builtin_adapters().load_for_source_validation(&input, &context),
    ] {
        assert!(loaded
            .diagnostics
            .iter()
            .any(|d| d.code == "example.csv.header" && d.severity.is_hard_violation()));
        assert!(!loaded
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.csv.invalid_header_parameter"));
        let LoadedInputAstStream::CsvDocument(table) = loaded.ast_stream.unwrap() else {
            panic!("native CSV AST")
        };
        assert_eq!(table.source.uri, input.uri);
        assert_eq!(table.rows[1].fields[1].value, "Ada");
        assert_eq!(
            table.parse_facts[0].diagnostic_code.as_deref(),
            Some("example.csv.header")
        );
        assert!(table.parse_facts[0].fatal);
    }
    let inspection = compile_schema_document_model(CSV_SCHEMA_URI, source());
    let mut pending = EngineContext::default();
    pending.schema_document_models.inspect_candidate(inspection);
    let blocked = LifecycleRegistry::with_builtin_adapters().load(&input, &pending);
    assert!(blocked.ast_stream.is_none());
    assert!(blocked
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation()));
}

#[test]
fn csv_parser_contract_keeps_the_builtin_incumbent_after_unauthorized_manifest_inspection() {
    let package = builtin_schema_package_source("csv").unwrap();
    let mut context = EngineContext::default();
    let diagnostics = crate::real::load_schema_package_manifest_into_context(
        &mut context,
        &EngineInput {
            uri: concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/schema-packages/csv/v1/package.cem"
            )
            .into(),
            bytes: package.manifest_source.as_bytes().to_vec(),
            from_format: None,
            identity: None,
            root_scope: Default::default(),
        },
    )
    .unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code == "cem.schema_package.replacement_not_authorized"),
        "{diagnostics:?}"
    );
    let model = context
        .schema_document_models
        .resolve_for_identity(Some(CSV_SCHEMA_URI), None, None)
        .unwrap();
    assert!(model.is_ready_for_validation());
    assert_eq!(
        model
            .parser_contract
            .as_ref()
            .unwrap()
            .source_owner()
            .unwrap()
            .source_uri(),
        package.schema_path
    );
    let input = EngineInput {
        uri: "memory://incumbent/input.csv".into(),
        bytes: b"id,name\n1,Ada\n".to_vec(),
        from_format: None,
        identity: Some(FormatIdentity {
            content_type: Some("text/csv; header=maybe".into()),
            ..Default::default()
        }),
        root_scope: Default::default(),
    };
    let actual = LifecycleRegistry::with_builtin_adapters().load(&input, &context);
    let expected =
        LifecycleRegistry::with_builtin_adapters().load(&input, &EngineContext::default());
    assert_eq!(actual.diagnostics, expected.diagnostics);
    assert!(matches!(
        actual.ast_stream,
        Some(LoadedInputAstStream::CsvDocument(_))
    ));
}
