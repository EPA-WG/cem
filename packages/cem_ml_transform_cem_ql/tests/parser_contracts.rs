use cem_ml::{
    engine::{EngineContext, EngineInput, FormatIdentity},
    events::cem::CemEventNormalizer,
    lifecycle::{LifecycleRegistry, LoadedInputAstStream},
    parser::tree::{CemTreeSemantics, RetainedCemTree},
    schema::{
        machine::CemSchemaMachine,
        package_compilation::{SchemaPackageCompilationRequest, SchemaPackageCompiler},
        package_sources::builtin_schema_package_source,
        reference_policy::ReferenceScopePolicy,
        registry::CSV_SCHEMA_URI,
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use cem_ml_transform_cem_ql::schema_packages::CemQlSchemaPackageCompiler;
use cem_ql::schema_references::CemQlSchemaDeclarationHost;
use std::sync::Arc;

#[test]
fn linked_parser_contract_keeps_the_original_native_schema_owner_and_csv_import() {
    let package = builtin_schema_package_source("csv").unwrap();
    let captured = Arc::new(
        CemSchemaMachine::new(
            CompiledSchema::cem_core(),
            CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                SourceId(1),
                package.schema_source.as_bytes().to_vec(),
            ))),
        )
        .build_with_lexical_scopes(),
    );
    let source = RetainedCemTree::from_shared(
        captured.document().clone(),
        package.schema_path,
        package.schema_source,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    let compiler = CemQlSchemaPackageCompiler::new(|request| {
        let mut host = CemQlSchemaDeclarationHost::new();
        let policy = ReferenceScopePolicy::schema_defaults().unwrap();
        host.register_scope(
            request.source.clone(),
            Some(Default::default()),
            policy.clone(),
        );
        host.attach_captured_names(&request.lexical_scopes).unwrap();
        Ok((host, policy.limits))
    });
    let model = compiler
        .compile(&SchemaPackageCompilationRequest {
            package_id: "csv".into(),
            manifest_uri: "schema-packages/csv/v1/package.cem".into(),
            schema_uri: CSV_SCHEMA_URI.into(),
            source: source.clone(),
            lexical_scopes: captured,
        })
        .unwrap();
    assert!(
        model.is_ready_for_validation(),
        "{:?}",
        model.compile_diagnostics
    );
    assert!(
        !model
            .compile_diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        model.compile_diagnostics
    );
    let contract = model.parser_contract.as_ref().unwrap();
    assert!(Arc::ptr_eq(contract.source_owner().unwrap(), &source));
    assert!(contract.matches_owner(source.ast_owner()));
    let mut context = EngineContext::default();
    context.schema_document_models.register(model);
    let loaded = LifecycleRegistry::with_builtin_adapters().load(
        &EngineInput {
            uri: "memory://linked/input.csv".into(),
            bytes: b"id,name\n1,\"Ada\n".to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some("text/csv".into()),
                schema: Some(CSV_SCHEMA_URI.into()),
                ..Default::default()
            }),
            from_format: None,
            root_scope: Default::default(),
        },
        &context,
    );
    assert!(matches!(
        loaded.ast_stream,
        Some(LoadedInputAstStream::CsvDocument(_))
    ));
    assert!(loaded
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.csv.unclosed_quote" && d.severity.is_hard_violation()));
}
