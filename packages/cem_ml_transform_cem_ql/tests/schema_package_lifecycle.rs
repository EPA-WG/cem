use cem_ml::{
    engine::{EngineContext, EngineInput, InputFormat},
    events::cem::CemEventNormalizer,
    parser::{
        builder::CemAstBuilder,
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode,
    },
    real::load_schema_package_manifest_into_context,
    resolver::{
        ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedRead, ResolverDiagnostic,
        ResourceResolver,
    },
    schema::{
        reference_policy::ReferenceScopePolicy,
        registry::{SchemaPackageOrigin, CEM_SCHEMA_CONTENT_TYPE},
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
    value::reference_resolution::ReferenceResolutionState,
};
use cem_ml_transform_cem_ql::schema_packages::CemQlSchemaPackageCompiler;
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    schema_references::CemQlSchemaDeclarationHost,
};
use std::sync::Arc;
const SCHEMA_URI: &str = "https://example.test/schema/runtime/1";
const SOURCE_URI: &str = "cem+test://runtime/schema.cem";
const SOURCE: &str = r#"{schema @name="runtime" @namespace="https://example.test/schema/runtime/1" @version="1.0.0" | {elements | {#library}} }"#;
const MANIFEST: &str = r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg
{package @id="runtime" @version="1.0.0" |
    {schema @uri="https://example.test/schema/runtime/1" @source="schema.cem"}
    {content-type @value="application/x-runtime+cem" @primary=true}
    {converter @id="runtime-converter" @implementation="rust" @rust-symbol="CemMlDomProjectionConverter" @streamable=true @lossiness="lossless" @implicit=false @readiness="ready" @cost=100 |
        {from @content-type="application/x-runtime+cem" @schema="https://example.test/schema/runtime/1"}
        {to @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
    }
    {artifact @kind="formatter" @path="formatters/old.cemt" @content-type="application/vnd.cem.transform+cem" @schema="https://cem.dev/ns/transform/cem/1" @target-content-type="application/x-runtime+cem" @target-schema="https://example.test/schema/runtime/1" @target-category="cem-tree" @function-name="fixture.format" @formatter-profile="compact"}
}"#;
#[derive(Debug, Clone)]
struct Resolver {
    source: String,
}
impl ResourceResolver for Resolver {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        if request.uri.ends_with(".cemt") {
            return Ok(ResolvedRead { uri: request.uri.clone(), bytes: br#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform
{module @version="1.0.0" | {format-function @name="fixture.format" @category="cem-tree" @subject="cem-ast-node" @produces="cem-tree" @content-type="application/x-runtime+cem" @schema="https://example.test/schema/runtime/1" @canonical=true @deterministic=true @streamable=true} }
"#.to_vec(), content_type: Some("application/vnd.cem.transform+cem".into()) });
        }
        Ok(ResolvedRead {
            uri: SOURCE_URI.into(),
            bytes: self.source.as_bytes().to_vec(),
            content_type: Some(CEM_SCHEMA_CONTENT_TYPE.into()),
        })
    }
    fn write(
        &self,
        request: &ResolveRequest,
        _bytes: &[u8],
    ) -> Result<cem_ml::resolver::ResolvedWrite, ResolverDiagnostic> {
        Err(ResolverDiagnostic::UnsupportedResolver {
            uri: request.uri.clone(),
            purpose: request.purpose,
            direction: ResolveDirection::Write,
        })
    }
}
fn context(source: &str) -> EngineContext {
    let mut context = EngineContext::default();
    set_source(&mut context, source);
    context
}
fn set_source(context: &mut EngineContext, source: &str) {
    for purpose in [ResolvePurpose::Template, ResolvePurpose::Input] {
        context.resolver_registry.register(
            "cem+test",
            purpose,
            ResolveDirection::Read,
            Resolver {
                source: source.into(),
            },
        );
    }
}
fn input() -> EngineInput {
    EngineInput {
        uri: "cem+test://runtime/package.cem".into(),
        bytes: MANIFEST.as_bytes().to_vec(),
        from_format: Some(InputFormat::Cem),
        identity: None,
        root_scope: Default::default(),
    }
}
fn tree(text: &str) -> Arc<RetainedCemTree> {
    let ast = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), text.as_bytes().to_vec()),
    )))
    .build();
    RetainedCemTree::new(ast, "library.cem", text, CemTreeSemantics::default(), None).unwrap()
}
fn compiler(name: Option<&str>) -> CemQlSchemaPackageCompiler {
    let library = name.map(|name| tree(&format!("{{schema | {{elements | {{element @name=\"{name}\" @required-attributes=\"command\"}} }} }}")));
    CemQlSchemaPackageCompiler::new(move |request| {
        let policy = ReferenceScopePolicy::schema_defaults().unwrap();
        let mut host = CemQlSchemaDeclarationHost::new();
        let evaluation = library.as_ref().map(|library| {
            let id = library
                .ast()
                .nodes
                .iter()
                .find_map(|n| match n {
                    CemAstNode::Element {
                        node_id,
                        expanded_name,
                        ..
                    } if expanded_name.local_name == "element" => Some(*node_id),
                    _ => None,
                })
                .unwrap();
            StandaloneExpressionContext::default().with_binding(
                "library",
                StandaloneExpressionBinding::any(ItemStream::once(
                    RetainedCemNode::new(library.clone(), id)
                        .unwrap()
                        .query_item(),
                )),
            )
        });
        let scope = host.register_scope(request.source.clone(), evaluation, policy.clone());
        if let Some(library) = &library {
            let target = host.register_scope(library.clone(), None, policy.clone());
            host.allow_scope_crossing(scope, target);
        }
        Ok((host, policy.limits))
    })
}
#[test]
fn explicit_lifecycle_completes_the_retained_source_and_preserves_last_active_package() {
    let mut context = context(SOURCE);
    let errors = load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
    assert!(
        !errors.iter().any(|d| d.severity.is_hard_violation()),
        "{errors:?}"
    );
    assert_eq!(
        context
            .schema_document_models
            .get(SCHEMA_URI)
            .unwrap()
            .declaration_references
            .state(),
        ReferenceResolutionState::Pending
    );
    assert!(context.schema_registry.schema(SCHEMA_URI).is_none());
    let source = context
        .schema_package_sources
        .get(SOURCE_URI)
        .unwrap()
        .clone();
    context.schema_package_compiler = Some(Arc::new(compiler(None)));
    let errors = load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
    assert!(
        !errors.iter().any(|d| d.severity.is_hard_violation()),
        "{errors:?}"
    );
    assert!(context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .declaration_references
        .sites[0]
        .resolution
        .is_some());
    assert!(Arc::ptr_eq(
        &source,
        context.schema_package_sources.get(SOURCE_URI).unwrap()
    ));
    context.schema_package_compiler = Some(Arc::new(compiler(Some("first"))));
    let errors = load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
    assert!(
        !errors.iter().any(|d| d.severity.is_hard_violation()),
        "{errors:?}"
    );
    assert!(context.schema_registry.schema(SCHEMA_URI).is_some());
    assert!(context
        .converter_registry
        .converter("runtime-converter")
        .is_some());
    assert!(context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .elements
        .contains_key("first"));
    let mut independent = context.clone();
    independent.schema_package_compiler = Some(Arc::new(compiler(Some("second"))));
    assert!(
        !load_schema_package_manifest_into_context(&mut independent, &input())
            .unwrap()
            .iter()
            .any(|d| d.severity.is_hard_violation())
    );
    assert!(independent
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .elements
        .contains_key("second"));
    assert!(context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .elements
        .contains_key("first"));
    assert!(Arc::ptr_eq(
        &source,
        independent.schema_package_sources.get(SOURCE_URI).unwrap()
    ));
    context.schema_package_compiler = Some(Arc::new(compiler(None)));
    assert!(
        !load_schema_package_manifest_into_context(&mut context, &input())
            .unwrap()
            .iter()
            .any(|d| d.severity.is_hard_violation())
    );
    assert!(!context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .is_ready_for_validation());
    assert!(context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .elements
        .contains_key("first"));
    assert!(context
        .converter_registry
        .converter("runtime-converter")
        .is_some());
    assert!(source
        .ast()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}

fn assert_active(context: &EngineContext, name: &str, converter: &str, artifact: &str) {
    assert!(context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .elements
        .contains_key(name));
    assert!(context.converter_registry.converter(converter).is_some());
    let artifacts: Vec<_> = context
        .converter_registry
        .package_artifacts()
        .filter(|a| a.package_id == "runtime")
        .collect();
    assert_eq!(artifacts.len(), 1);
    assert!(artifacts[0].path.ends_with(artifact), "{artifacts:?}");
}
fn load(context: &mut EngineContext, input: &EngineInput) {
    let diagnostics = load_schema_package_manifest_into_context(context, input).unwrap();
    assert!(
        !diagnostics.iter().any(|d| d.severity.is_hard_violation()),
        "{diagnostics:?}"
    );
}
#[test]
fn changed_source_and_incomplete_candidate_preserve_then_replace_all_registrations() {
    let mut context = context(SOURCE);
    context.schema_package_compiler = Some(Arc::new(compiler(Some("old"))));
    load(&mut context, &input());
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    let original = context
        .schema_package_sources
        .get(SOURCE_URI)
        .unwrap()
        .clone();
    let changed = SOURCE.replace("{#library}", "{#library} {element @name=\"local\"}");
    set_source(&mut context, &changed);
    let mut replacement = input();
    replacement.bytes = MANIFEST
        .replace("runtime-converter", "new-converter")
        .replace("old.cemt", "new.cemt")
        .into_bytes();
    context.schema_package_compiler = Some(Arc::new(compiler(None)));
    load(&mut context, &replacement);
    let candidate = context
        .schema_package_sources
        .get(SOURCE_URI)
        .unwrap()
        .clone();
    assert!(!Arc::ptr_eq(&original, &candidate));
    assert!(context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .elements
        .contains_key("local"));
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    assert!(context
        .converter_registry
        .converter("new-converter")
        .is_none());
    context.schema_package_compiler = Some(Arc::new(compiler(Some("new"))));
    load(&mut context, &replacement);
    assert_active(&context, "new", "new-converter", "new.cemt");
    assert!(context
        .converter_registry
        .converter("runtime-converter")
        .is_none());
    assert!(Arc::ptr_eq(
        &candidate,
        context.schema_package_sources.get(SOURCE_URI).unwrap()
    ));
    assert!(original
        .ast()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}
#[test]
fn replacement_authority_is_checked_before_invoking_the_compiler() {
    use cem_ml::engine::SchemaPackageReplacementGrant;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let mut context = context(SOURCE);
    context.schema_package_compiler = Some(Arc::new(compiler(Some("old"))));
    load(&mut context, &input());
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    context.schema_package_compiler =
        Some(Arc::new(CemQlSchemaPackageCompiler::new(move |request| {
            counted.fetch_add(1, Ordering::SeqCst);
            let policy = ReferenceScopePolicy::schema_defaults().unwrap();
            let mut host = CemQlSchemaDeclarationHost::new();
            host.register_scope(
                request.source.clone(),
                Some(StandaloneExpressionContext::default().with_binding(
                    "library",
                    StandaloneExpressionBinding::any(ItemStream::empty()),
                )),
                policy.clone(),
            );
            Ok((host, policy.limits))
        })));
    let mut replacement = input();
    replacement.uri = "cem+test://other/package.cem".into();
    let grant = SchemaPackageReplacementGrant {
        package_id: "runtime".into(),
        expected_origin: SchemaPackageOrigin::Manifest(input().uri),
        replacement_manifest_uri: replacement.uri.clone(),
    };
    for grants in [
        vec![],
        vec![SchemaPackageReplacementGrant {
            expected_origin: SchemaPackageOrigin::Builtin,
            ..grant.clone()
        }],
    ] {
        context.schema_package_replacement_grants = grants;
        let diagnostics =
            load_schema_package_manifest_into_context(&mut context, &replacement).unwrap();
        assert!(diagnostics
            .iter()
            .any(|d| d.code == "cem.schema_package.replacement_not_authorized"));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_active(&context, "old", "runtime-converter", "old.cemt");
    }
    context.schema_package_replacement_grants = vec![grant];
    load(&mut context, &replacement);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .elements
        .is_empty());
    assert_eq!(
        context.schema_registry.package_origin("runtime"),
        Some(&SchemaPackageOrigin::Manifest(replacement.uri))
    );
}
#[test]
fn native_query_errors_keep_their_code_and_source_while_active_package_survives() {
    let mut context = context(SOURCE);
    context.schema_package_compiler = Some(Arc::new(compiler(Some("old"))));
    load(&mut context, &input());
    set_source(&mut context, &SOURCE.replace("#library", "#1 +"));
    context.schema_package_compiler = Some(Arc::new(CemQlSchemaPackageCompiler::new(|request| {
        let policy = ReferenceScopePolicy::schema_defaults().unwrap();
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(
            request.source.clone(),
            Some(Default::default()),
            policy.clone(),
        );
        Ok((host, policy.limits))
    })));
    let diagnostics = load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
    let candidate = context.schema_document_models.get(SCHEMA_URI).unwrap();
    assert_eq!(
        candidate.declaration_references.state(),
        ReferenceResolutionState::Invalid
    );
    let original = &candidate.declaration_references.sites[0]
        .resolution
        .as_ref()
        .unwrap()
        .diagnostics;
    assert!(!original.is_empty());
    for diagnostic in original {
        assert!(
            diagnostics.iter().any(|d| d.code == diagnostic.code
                && d.message == diagnostic.message
                && d.uri.as_deref() == Some(SOURCE_URI)),
            "{diagnostics:?}"
        );
    }
    assert!(!diagnostics
        .iter()
        .any(|d| d.code == "cem.schema_package.schema_compilation_failed"));
    assert_active(&context, "old", "runtime-converter", "old.cemt");
}
#[test]
fn failed_compiler_and_foreign_model_identity_cannot_publish() {
    use cem_ml::{
        diagnostics::{Diagnostic, Severity},
        schema::{
            document_model::SchemaDocumentModel,
            package_compilation::{SchemaPackageCompilationRequest, SchemaPackageCompiler},
        },
    };
    #[derive(Debug)]
    struct ForeignIdentity;
    impl SchemaPackageCompiler for ForeignIdentity {
        fn compile(
            &self,
            _: &SchemaPackageCompilationRequest,
        ) -> Result<SchemaDocumentModel, Vec<Diagnostic>> {
            Ok(SchemaDocumentModel {
                schema_uri: "schema:foreign".into(),
                ..Default::default()
            })
        }
    }
    let mut context = context(SOURCE);
    context.schema_package_compiler = Some(Arc::new(compiler(Some("old"))));
    load(&mut context, &input());
    let warning = Diagnostic {
        code: "fixture.compiler_warning".into(),
        severity: Severity::Warning,
        message: "runtime unavailable".into(),
        ..Default::default()
    };
    let failures: Vec<Arc<dyn SchemaPackageCompiler>> = vec![
        Arc::new(CemQlSchemaPackageCompiler::new(|_| Err(vec![]))),
        Arc::new(CemQlSchemaPackageCompiler::new(move |_| {
            Err(vec![warning.clone()])
        })),
        Arc::new(ForeignIdentity),
        Arc::new(CemQlSchemaPackageCompiler::new(|_| {
            let mut limits = ReferenceScopePolicy::schema_defaults().unwrap().limits;
            limits.max_work = 0;
            Ok((CemQlSchemaDeclarationHost::new(), limits))
        })),
    ];
    for compiler in failures {
        context.schema_package_compiler = Some(compiler);
        let diagnostics =
            load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == "cem.schema_package.schema_compilation_failed"
                    && d.severity.is_hard_violation()
                    && d.uri.as_deref() == Some(SOURCE_URI)),
            "{diagnostics:?}"
        );
        assert_active(&context, "old", "runtime-converter", "old.cemt");
    }
}

#[test]
fn engine_manifest_stage_uses_compiled_declarations_for_final_validation() {
    use cem_ml::{
        engine::{CemMlEngine, FailLevel, ValidateProjection, ValidateRequest},
        real::RealCemMlEngine,
    };
    let mut context = context(SOURCE);
    context.schema_package_manifests = vec![input()];
    context.schema = Some(SCHEMA_URI.into());
    context.schema_package_compiler = Some(Arc::new(compiler(Some("instance"))));
    let document = EngineInput {
        uri: "cem+test://runtime/input.cem".into(),
        bytes: b"{instance}".to_vec(),
        from_format: Some(InputFormat::Cem),
        identity: None,
        root_scope: Default::default(),
    };
    let result = RealCemMlEngine::new()
        .validate(ValidateRequest {
            inputs: vec![document],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: context.clone(),
        })
        .unwrap();
    assert!(
        result
            .report
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema_model.missing_required_attribute"
                && d.message.contains("command")),
        "{:?}",
        result.report.diagnostics
    );
    assert!(!result
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.schema_model.not_ready"));
    // Enrichment compiles on the request's own context, not the original caller.
    assert!(context.schema_document_models.get(SCHEMA_URI).is_none());
    assert!(context.schema_package_sources.get(SOURCE_URI).is_none());
}

#[test]
fn engine_package_stage_reuses_attribute_constraints_for_input_validation() {
    use cem_ml::{
        engine::{CemMlEngine, FailLevel, ValidateProjection, ValidateRequest},
        real::RealCemMlEngine,
    };
    let source = SOURCE.replace(
        "{elements | {#library}}",
        "{elements | {element @name=input @optional-attributes=size}} {attributes | {#library}}",
    );
    let mut context = context(&source);
    context.schema_package_manifests = vec![input()];
    context.schema = Some(SCHEMA_URI.into());
    load(&mut context, &input());
    assert!(!context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .is_ready_for_validation());
    assert!(context.schema_registry.schema(SCHEMA_URI).is_none());
    assert!(context
        .converter_registry
        .converter("runtime-converter")
        .is_none());
    let retained_source = context
        .schema_package_sources
        .get(SOURCE_URI)
        .unwrap()
        .clone();
    let library = tree(
        "{schema | {attributes | {attribute @name=size @type=schema:integer @maxInclusive=10}} }",
    );
    context.schema_package_compiler =
        Some(Arc::new(CemQlSchemaPackageCompiler::new(move |request| {
            assert!(Arc::ptr_eq(&retained_source, &request.source));
            let policy = ReferenceScopePolicy::schema_defaults().unwrap();
            let mut host = CemQlSchemaDeclarationHost::new();
            let id = library
                .ast()
                .nodes
                .iter()
                .find_map(|n| match n {
                    CemAstNode::Element {
                        node_id,
                        expanded_name,
                        ..
                    } if expanded_name.local_name == "attribute" => Some(*node_id),
                    _ => None,
                })
                .unwrap();
            let context = StandaloneExpressionContext::default().with_binding(
                "library",
                StandaloneExpressionBinding::any(ItemStream::once(
                    RetainedCemNode::new(library.clone(), id)
                        .unwrap()
                        .query_item(),
                )),
            );
            let source = host.register_scope(request.source.clone(), Some(context), policy.clone());
            let target = host.register_scope(library.clone(), None, policy.clone());
            host.allow_scope_crossing(source, target);
            Ok((host, policy.limits))
        })));
    for (size, expected_invalid) in [(5, false), (11, true)] {
        let document = EngineInput {
            uri: "cem+test://runtime/input.cem".into(),
            bytes: format!("{{input @size={size}}}").into_bytes(),
            from_format: Some(InputFormat::Cem),
            identity: None,
            root_scope: Default::default(),
        };
        let result = RealCemMlEngine::new()
            .validate(ValidateRequest {
                inputs: vec![document],
                projection: ValidateProjection::Json,
                fail_level: FailLevel::Validate,
                context: context.clone(),
            })
            .unwrap();
        assert_eq!(
            result
                .report
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            expected_invalid,
            "{:?}",
            result.report.diagnostics
        );
        assert_eq!(
            result
                .report
                .diagnostics
                .iter()
                .any(|d| d.code
                    == cem_ml::schema::document_model::INVALID_ATTRIBUTE_DATATYPE_PARAM_CODE),
            expected_invalid
        );
        assert!(!result
            .report
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema_model.not_ready"));
    }
}

#[test]
fn package_compilation_reuses_behavior_and_diagnostic_dependencies_together() {
    let source=SOURCE.replace("{elements | {#library}}", "{elements | {element @name=input @optional-attributes=size}} {attributes | {attribute @name=size @type=schema:integer @type-diagnostic=fixture.value}} {diagnostics | {#diagnostic}} {behaviors | {#behavior}}");
    let mut context = context(&source);
    load(&mut context, &input());
    assert!(!context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .is_ready_for_validation());
    assert!(context.schema_registry.schema(SCHEMA_URI).is_none());
    let library=tree("{schema @namespace=library | {behaviors | {behavior @name=value-check @implementation=engine @execution=ast-validation @primitive=schema:scalar-type}} {diagnostics | {diagnostic @code=fixture.value @behavior=value-check @severity=warning}} }");
    context.schema_package_compiler =
        Some(Arc::new(CemQlSchemaPackageCompiler::new(move |request| {
            let policy = ReferenceScopePolicy::schema_defaults().unwrap();
            let mut host = CemQlSchemaDeclarationHost::new();
            let mut evaluation = StandaloneExpressionContext::default();
            for kind in ["diagnostic", "behavior"] {
                let id = library
                    .ast()
                    .nodes
                    .iter()
                    .find_map(|n| match n {
                        CemAstNode::Element {
                            node_id,
                            expanded_name,
                            ..
                        } if expanded_name.local_name == kind => Some(*node_id),
                        _ => None,
                    })
                    .unwrap();
                evaluation = evaluation.with_binding(
                    kind,
                    StandaloneExpressionBinding::any(ItemStream::once(
                        RetainedCemNode::new(library.clone(), id)
                            .unwrap()
                            .query_item(),
                    )),
                );
            }
            let a = host.register_scope(request.source.clone(), Some(evaluation), policy.clone());
            let b = host.register_scope(library.clone(), None, policy.clone());
            host.allow_scope_crossing(a, b);
            Ok((host, policy.limits))
        })));
    load(&mut context, &input());
    let model = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap();
    assert!(model.is_ready_for_validation() && model.compile_diagnostics.is_empty());
    let diagnostics = cem_ml::schema::document_model::validate_document_model(
        tree("{input @size=no}").ast(),
        model,
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code == "fixture.value"
                && d.severity == cem_ml::diagnostics::Severity::Warning)
    );
    assert!(context
        .converter_registry
        .converter("runtime-converter")
        .is_some());
    assert!(context
        .converter_registry
        .package_artifacts()
        .any(|a| a.package_id == "runtime"));
}

#[test]
fn field_contract_target_errors_preserve_active_package_until_target_is_available() {
    let complete = SOURCE.replace("{elements | {#library}}", "{elements | {element @name=input @optional-attributes=command}} {field-contracts | {#library}}");
    let library = tree("{schema | {field-contracts | {field-contract @name=shared @target=input @required-attributes=command}}}");
    let mut context = context(&complete);
    context.schema_package_compiler =
        Some(Arc::new(CemQlSchemaPackageCompiler::new(move |request| {
            let policy = ReferenceScopePolicy::schema_defaults().unwrap();
            let id = library
                .ast()
                .nodes
                .iter()
                .find_map(|n| match n {
                    CemAstNode::Element {
                        node_id,
                        expanded_name,
                        ..
                    } if expanded_name.local_name == "field-contract" => Some(*node_id),
                    _ => None,
                })
                .unwrap();
            let evaluation = StandaloneExpressionContext::default().with_binding(
                "library",
                StandaloneExpressionBinding::any(ItemStream::once(
                    RetainedCemNode::new(library.clone(), id)
                        .unwrap()
                        .query_item(),
                )),
            );
            let mut host = CemQlSchemaDeclarationHost::new();
            let a = host.register_scope(request.source.clone(), Some(evaluation), policy.clone());
            let b = host.register_scope(library.clone(), None, policy.clone());
            host.allow_scope_crossing(a, b);
            Ok((host, policy.limits))
        })));
    load(&mut context, &input());
    assert_eq!(
        context
            .schema_document_models
            .resolve_for_identity(Some(SCHEMA_URI), None, None)
            .unwrap()
            .elements["input"]
            .field_contracts
            .len(),
        1
    );
    set_source(
        &mut context,
        &complete.replace(
            "{elements | {element @name=input @optional-attributes=command}}",
            "{elements}",
        ),
    );
    let diagnostics = load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
    assert!(diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::INVALID_SCHEMA_FIELD_CONTRACT_CODE));
    assert!(context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .compile_diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::INVALID_SCHEMA_FIELD_CONTRACT_CODE));
    assert!(context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .elements
        .contains_key("input"));
    assert!(context
        .converter_registry
        .converter("runtime-converter")
        .is_some());
    assert!(context
        .converter_registry
        .package_artifacts()
        .any(|a| a.package_id == "runtime"));
    set_source(&mut context, &complete);
    load(&mut context, &input());
    assert!(context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .compile_diagnostics
        .is_empty());
}

#[test]
fn native_base_candidates_preserve_active_package_until_inheritance_is_ready() {
    let source = SOURCE.replace("{#library}", "{element @name=derived @base={#library}}");
    let mut context = context(&source);
    load(&mut context, &input());
    assert!(context.schema_registry.schema(SCHEMA_URI).is_none());
    assert!(!context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .is_ready_for_validation());
    context.schema_package_compiler = Some(Arc::new(compiler(Some("base"))));
    load(&mut context, &input());
    assert_active(&context, "derived", "runtime-converter", "old.cemt");
    context.schema_package_compiler = Some(Arc::new(compiler(None)));
    load(&mut context, &input());
    assert!(!context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .is_ready_for_validation());
    assert_active(&context, "derived", "runtime-converter", "old.cemt");
}
