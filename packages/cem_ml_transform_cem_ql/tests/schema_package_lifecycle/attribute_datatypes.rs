#[path = "pretyped_attributes.rs"]
mod pretyped_attributes;
#[path = "preparation_replacement.rs"]
mod preparation_replacement;
#[path = "facet_replacement.rs"]
mod facet_replacement;
#[path = "external_typed.rs"]
mod external_typed;
use super::*;
use cem_ml::schema::{
    datatype_registry::DatatypeKind,
    datatype_validation::{ScalarRepresentation, ValueRepresentation},
    declaration_references::SchemaDeclarationNode,
    document_model::{shipped_datatypes::ShippedDatatype, validate_document_model},
};
use cem_ql::{
    datatype_compilation::{
        compile_datatypes, DatatypeImplementation, DatatypeImplementations, TokenizerBinding,
    },
    datatype_facets::{FacetProfileBinding, RegisteredFacetProfile},
    datatype_preparation::PreparationBinding,
};

fn authored(slot: &str, default: &str) -> String {
    format!("{{schema @name=runtime @namespace=\"{SCHEMA_URI}\" @version=1.0.0 | {{types | {{type @name=integer @kind=scalar}}}} {{attributes | {{attribute @name=count @type={slot} @maxInclusive=4 {default}}}}} {{elements | {{element @name=sample @optional-attributes=count}}}}}}")
}
fn compiled(count: usize, ready: bool) -> CemQlSchemaPackageCompiler {
    compiled_kind(count, ready, false, false)
}
fn compiled_kind(
    count: usize,
    ready: bool,
    nodes: bool,
    require_candidate: bool,
) -> CemQlSchemaPackageCompiler {
    compiled_kind_with_counter(count, ready, nodes, require_candidate, None)
}
fn compiled_kind_with_counter(count: usize, ready: bool, nodes: bool, require_candidate: bool, calls: Option<Arc<std::sync::atomic::AtomicUsize>>) -> CemQlSchemaPackageCompiler {
    compiled_kind_with_hooks(count, ready, nodes, require_candidate, calls, None)
}
fn compiled_kind_with_hooks(count: usize, ready: bool, nodes: bool, require_candidate: bool, calls: Option<Arc<std::sync::atomic::AtomicUsize>>, hook: Option<Arc<dyn Fn() + Send + Sync>>) -> CemQlSchemaPackageCompiler {
    CemQlSchemaPackageCompiler::new(move |request| {
        let policy = ReferenceScopePolicy::schema_defaults().unwrap();
        let mut host = CemQlSchemaDeclarationHost::new();
        let id = request.source.ast().nodes.iter().find_map(|n| match n {
            CemAstNode::Element {node_id, expanded_name, ..} if expanded_name.local_name == "type" => Some(*node_id), _ => None,
        }).unwrap();
        let target = RetainedCemNode::new(request.source.clone(), id).unwrap().query_item();
        host.register_scope(request.source.clone(), Some(StandaloneExpressionContext::default().with_binding(
            "target", StandaloneExpressionBinding::any(ItemStream::from_items(vec![target; count])),
        )), policy.clone());
        host.attach_captured_names(&request.lexical_scopes).unwrap();
        Ok((host, policy.limits))
    }).with_datatype_discovery(Default::default(), |request, _| {
        let schema = request.source.ast().nodes.iter().find_map(|n| match n {
            CemAstNode::Element {node_id, expanded_name, ..} if expanded_name.local_name == "schema" => SchemaDeclarationNode::new(request.source.ast_owner().clone(), *node_id), _ => None,
        }).unwrap();
        Ok(vec![cem_ql::datatype_names::DatatypeSchemaSource {schema, captured: request.lexical_scopes.clone(), imports: vec![]}])
    }, move |request, host, sources, limits| {
        let mut implementations = DatatypeImplementations::default();
        for source in sources {
            implementations.register(DatatypeImplementation {
                source: source.clone(), kind: if nodes { DatatypeKind::Node } else { DatatypeKind::Scalar },
                representation: if nodes { ValueRepresentation::Nodes } else { ValueRepresentation::Scalar(ScalarRepresentation::Integer) },
                accepted_bases: vec![], bounds: Default::default(), tokenizer: TokenizerBinding::Absent, validator: None,
            }).unwrap();
            if ready && !nodes {
                let selected =
                    if require_candidate {
                        cem_ql::datatype_preparation::RegisteredLexicalPreparation::new(
                            source.clone(), "fixture:required-candidate", cem_ql::datatype_preparation::PreparationSignature {
                                kind: DatatypeKind::Scalar, output: ValueRepresentation::Scalar(ScalarRepresentation::Integer),
                                candidate: cem_ml::schema::datatype_validation::CandidateRequirement::Required,
                            }, CheckCandidate(calls.clone(), hook.clone()),
                        ).unwrap()
                    } else { cem_ql::datatype_shipped::lexical_preparation(source.clone(), ShippedDatatype::Integer).unwrap() };
                implementations.select_preparation(source.clone(), if source.attribute("base").is_some() {
                    PreparationBinding::CheckedReplacement(selected)
                } else { PreparationBinding::Ready(selected) }).unwrap();
            }
            if source.attribute("base").is_none() {
            implementations.select_facets(source.clone(), FacetProfileBinding::Ready(
                RegisteredFacetProfile::new(source.clone(), "fixture:integer", if nodes { cem_ml::schema::document_model::attribute_facets::FacetFamily::Nodes } else { cem_ml::schema::document_model::attribute_facets::FacetFamily::Shipped(ShippedDatatype::Integer) }).unwrap(),
            )).unwrap();
            }
        }
        Ok(compile_datatypes(request.source.ast_owner().clone(), sources, host, &implementations, &Default::default(), limits))
    })
}
#[test]
fn automatic_attribute_types_validate_literal_and_native_contracts() {
    for slot in ["integer", "{#target}"] {
        let mut context = context(&authored(slot, "@default=003"));
        context.schema_package_compiler = Some(Arc::new(compiled(1, true)));
        let errors = load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
        assert!(
            !errors.iter().any(|d| d.severity.is_hard_violation()),
            "{errors:?}"
        );
        let model = context
            .schema_document_models
            .resolve_for_identity(Some(SCHEMA_URI), None, None)
            .expect("complete attribute consumer activates package");
        assert!(model.is_ready_for_validation());
        for (value, valid) in [("003", true), ("5", false), ("wrong", false)] {
            let input = tree(&format!("{{sample @count={value}}}"));
            let diagnostics = validate_document_model(input.ast(), &model);
            assert_eq!(
                !diagnostics.iter().any(|d| d.severity.is_hard_violation()),
                valid,
                "{slot} {value}: {diagnostics:?}"
            );
        }
    }
}
#[test]
fn automatic_attribute_types_keep_active_package_on_incomplete_or_invalid_replacement() {
    let mut context = context(&authored("{#target}", "@default=003"));
    context.schema_package_compiler = Some(Arc::new(compiled(1, true)));
    load(&mut context, &input());
    let active = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .datatype_compilation
        .clone()
        .unwrap();
    let owner = context
        .schema_package_sources
        .get(SOURCE_URI)
        .unwrap()
        .clone();
    set_source(&mut context, &authored("{#target}", "@default=002"));
    for (count, ready) in [(0, true), (2, true), (1, false)] {
        context.schema_package_compiler = Some(Arc::new(compiled(count, ready)));
        let _ = load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
        let current = context
            .schema_document_models
            .resolve_for_identity(Some(SCHEMA_URI), None, None)
            .unwrap();
        assert!(Arc::ptr_eq(
            &active,
            current.datatype_compilation.as_ref().unwrap()
        ));
        assert!(!context
            .schema_document_models
            .get(SCHEMA_URI)
            .unwrap()
            .is_ready_for_validation());
    }
    let candidate = context
        .schema_package_sources
        .get(SOURCE_URI)
        .unwrap()
        .clone();
    assert!(!Arc::ptr_eq(&owner, &candidate));
    context.schema_package_compiler = Some(Arc::new(compiled(1, true)));
    load(&mut context, &input());
    assert!(Arc::ptr_eq(
        &candidate,
        context.schema_package_sources.get(SOURCE_URI).unwrap()
    ));
    let current = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap();
    assert!(!Arc::ptr_eq(
        &active,
        current.datatype_compilation.as_ref().unwrap()
    ));
    let current = current.datatype_compilation.clone().unwrap();
    set_source(&mut context, &authored("{#target}", "@default=9"));
    let errors = load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
    assert!(
        errors.iter().any(|d| d.severity.is_hard_violation()),
        "{errors:?}"
    );
    assert!(Arc::ptr_eq(
        &current,
        context
            .schema_document_models
            .resolve_for_identity(Some(SCHEMA_URI), None, None)
            .unwrap()
            .datatype_compilation
            .as_ref()
            .unwrap()
    ));
}

fn request(text: &str) -> cem_ml::schema::package_compilation::SchemaPackageCompilationRequest {
    use cem_ml::schema::{machine::CemSchemaMachine, vocab::CompiledSchema};
    let text = format!("@doc cem-ml 1\n{text}");
    let text = text.as_str();
    let captured = Arc::new(
        CemSchemaMachine::new(
            CompiledSchema::cem_core(),
            CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                SourceId(1),
                text.as_bytes().to_vec(),
            ))),
        )
        .build_with_lexical_scopes(),
    );
    let source = RetainedCemTree::from_shared(
        captured.document().clone(),
        SOURCE_URI,
        text,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    cem_ml::schema::package_compilation::SchemaPackageCompilationRequest {
        package_id: "runtime".into(),
        manifest_uri: "cem+test://runtime/package.cem".into(),
        schema_uri: SCHEMA_URI.into(),
        source,
        lexical_scopes: captured,
    }
}
#[test]
fn automatic_attribute_qnames_use_original_scope_and_reject_unknown_names() {
    use cem_ml::schema::package_compilation::SchemaPackageCompiler;
    for (prefix, slot, expected) in [
        (format!("@ns own = \"{SCHEMA_URI}\"\n"), "own:integer", true),
        ("@ns own = \"urn:other\"\n".into(), "own:integer", false),
        (String::new(), "missing:integer", false),
        (String::new(), "string", false),
    ] {
        let request = request(&format!("{prefix}{}", authored(slot, "")));
        let model = compiled(1, true).compile(&request).unwrap();
        assert_eq!(
            model.is_ready_for_validation(),
            expected,
            "{slot}: {:?}",
            model.datatype_compilation.as_ref().map(|c| c
                .issues
                .iter()
                .map(|i| i.code)
                .collect::<Vec<_>>())
        );
        assert_eq!(model.attribute_datatypes.contains_key("count"), expected);
    }
}
#[test]
fn automatic_attribute_bindings_require_singletons_valid_defaults_and_diagnostics() {
    use cem_ml::schema::package_compilation::SchemaPackageCompiler;
    for (slot, fields, count, issue) in [
        ("{#target}", "", 0, "attribute-type-singleton-required"),
        ("{#target}", "", 2, "attribute-type-singleton-required"),
        (
            "{#target}",
            "@default=wrong",
            1,
            "attribute-default-invalid",
        ),
        ("{#target}", "@default=5", 1, "attribute-default-invalid"),
        (
            "integer",
            "@type-diagnostic=unknown",
            1,
            "attribute-diagnostic-unresolved",
        ),
    ] {
        let model = compiled(count, true)
            .compile(&request(&authored(slot, fields)))
            .unwrap();
        assert!(!model.is_ready_for_validation());
        let compilation = model.datatype_compilation.as_ref().unwrap();
        assert!(
            compilation.issues.iter().any(|i| i.code == issue),
            "{:?}",
            compilation
                .issues
                .iter()
                .map(|i| i.code)
                .collect::<Vec<_>>()
        );
        assert!(model
            .compile_diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation() && d.source_map.is_some()));
        assert!(model.attribute_datatypes.is_empty());
    }
}
#[test]
fn automatic_attribute_activation_is_bounded_and_cancelled_rebinding_cannot_reuse_consumers() {
    use cem_ml::{
        operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
        scheduler::AbortSignal,
        schema::{
            package_compilation::SchemaPackageCompiler,
            reference_traversal::ReferenceTraversalLimits,
        },
    };
    let request = request(&authored("integer", ""));
    let model = compiled(1, true).compile(&request).unwrap();
    assert!(model.is_ready_for_validation());
    for cancelled in [false, true] {
        let mut model = model.clone();
        let mut compilation = (**model.datatype_compilation.as_ref().unwrap()).clone();
        let mut host = CemQlSchemaDeclarationHost::new();
        let signal = AbortSignal::default();
        if cancelled {
            signal.abort();
        }
        let control = OperationControl::new(signal);
        cem_ql::attribute_activation::activate_attribute_datatypes(
            &mut model,
            &mut compilation,
            &mut host,
            ReferenceTraversalLimits {
                max_depth: 10,
                max_work: if cancelled { 100 } else { 1 },
            },
            &cem_ql::datatype_validation::ValidationRuntime {
                control: &control,
                scope: ROOT_EXECUTION_SCOPE_ID,
                query: Default::default(),
            },
        )
        .unwrap();
        assert!(!compilation.is_ready());
        assert!(!model.is_ready_for_validation());
        assert!(model.attribute_datatypes.is_empty());
    }
}
#[test]
fn automatic_node_datatype_validates_retained_targets_per_input_context() {
    use cem_ml::schema::package_compilation::SchemaPackageCompiler;
    let source = authored("{#target}", "")
        .replace("@kind=scalar", "@kind=node")
        .replace("@maxInclusive=4", "@maxItems=1");
    let model = compiled_kind(1, false, true, false)
        .compile(&request(&source))
        .unwrap();
    assert!(
        model.is_ready_for_validation(),
        "{:?}",
        model.datatype_compilation.as_ref().map(|c| c
            .issues
            .iter()
            .map(|i| i.code)
            .collect::<Vec<_>>())
    );
    assert!(model.attribute_is_node_valued("count"));
    let input = tree("{sample @count={#picked}}");
    let target = tree("{original | {#untouched}}");
    let id = target
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let item = RetainedCemNode::new(target.clone(), id)
        .unwrap()
        .query_item();
    let policy = ReferenceScopePolicy::schema_defaults().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let input_scope = host.register_scope(input.clone(), None, policy.clone());
    let target_scope = host.register_scope(target.clone(), None, policy.clone());
    host.allow_scope_crossing(input_scope, target_scope);
    for count in [1, 2, 0, 1] {
        host.set_context(
            input_scope,
            Some(StandaloneExpressionContext::default().with_binding(
                "picked",
                StandaloneExpressionBinding::any(ItemStream::from_items(vec![item.clone(); count])),
            )),
        );
        let result = host
            .validate_input(input.clone(), &model, policy.limits)
            .unwrap();
        assert!(result.complete, "{result:?}");
        assert_eq!(result.failed, count > 1, "{result:?}");
        let attribute = input
            .ast()
            .nodes
            .iter()
            .find_map(|node| match node {
                CemAstNode::Attribute { node_id, .. } => {
                    SchemaDeclarationNode::new(input.ast_owner().clone(), *node_id)
                }
                _ => None,
            })
            .unwrap();
        let direct = cem_ml::schema::attribute_references::validate_native_attribute_reference(
            attribute,
            &model,
            "sample",
            &mut host,
            policy.limits,
        )
        .unwrap();
        assert!(direct.complete);
        assert_eq!(direct.failed, count > 1);
        assert_eq!(direct.targets.len(), count);
        let value = &result.nodes[0].attribute_values[0];
        assert_eq!(value.access.roots().len(), count);
        for index in value.access.roots() {
            assert!(Arc::ptr_eq(
                value.access.node(*index).unwrap().document(),
                target.ast_owner()
            ));
        }
    }
    assert!(target
        .ast()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}

#[test]
fn automatic_attribute_qnames_preserve_local_namespace_and_uses_bindings() {
    use cem_ml::schema::package_compilation::SchemaPackageCompiler;
    for (fields, uses, expected) in [
        (format!("@xmlns:own=\"{SCHEMA_URI}\""), String::new(), true),
        (
            String::new(),
            format!("{{uses | {{use @as=own @schema=\"{SCHEMA_URI}\"}}}}"),
            true,
        ),
        (
            "@xmlns:own=urn:other".into(),
            format!("{{uses | {{use @as=own @schema=\"{SCHEMA_URI}\"}}}}"),
            false,
        ),
    ] {
        let source = authored("own:integer", "")
            .replace("@name=count @type", &format!("@name=count {fields} @type"))
            .replace("{types |", &format!("{uses} {{types |"));
        let model = compiled(1, true).compile(&request(&source)).unwrap();
        assert_eq!(
            model.is_ready_for_validation(),
            expected,
            "fields={fields}, uses={uses}, diagnostics={:?}: {:?}",
            model
                .compile_diagnostics
                .iter()
                .map(|d| &d.message)
                .collect::<Vec<_>>(),
            model.datatype_compilation.as_ref().map(|c| c
                .issues
                .iter()
                .map(|i| i.code)
                .collect::<Vec<_>>())
        );
    }
}

struct CheckCandidate(Option<Arc<std::sync::atomic::AtomicUsize>>, Option<Arc<dyn Fn() + Send + Sync>>);
impl std::fmt::Debug for CheckCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("CheckCandidate") }
}
impl cem_ql::datatype_preparation::NativeLexicalPreparer for CheckCandidate {
    fn prepare(
        &self,
        call: cem_ql::datatype_preparation::PreparationCall<'_>,
    ) -> cem_ql::datatype_preparation::PreparationExecution {
        use cem_ql::{
            datatype_preparation::PreparationExecution,
            eval::{AtomValue, Item, QueryContextScope},
        };
        if let Some(calls) = &self.0 { calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst); }
        let [candidate] = call.candidate else {
            panic!("original candidate required")
        };
        let name = candidate.view().unwrap().field("name").unwrap()[0].atom();
        assert!(matches!(&name, Some(AtomValue::String(name)) if name == "count" || name == "default"));
        assert_eq!(candidate.source_map(), Some(call.original.source.clone()));
        // Declaration-time defaults use the original authorized schema view;
        // runtime attributes receive the restricted native candidate adapter.
        if name == Some(AtomValue::String("count".into())) {
        assert!(
            candidate
                .view()
                .unwrap()
                .parent(QueryContextScope(0))
                .is_err(),
            "candidate must not grant ancestor access"
        );
        assert!(
            candidate.view().unwrap().field("valueNodes").is_none(),
            "resolved value is supplied through its own bounded input"
        );
        }
        if let Some(hook) = &self.1 { hook(); }
        match call.lexical.text.parse::<i64>() {
            Ok(value) => PreparationExecution::Prepared {
                value: vec![Item::Atomic(AtomValue::Integer(value))],
                diagnostics: vec![],
            },
            Err(_) => PreparationExecution::Rejected(vec![]),
        }
    }
}
#[test]
fn automatic_attribute_validation_requires_original_candidate_without_widening_access() {
    use cem_ml::schema::package_compilation::SchemaPackageCompiler;
    let model = compiled_kind(1, true, false, true)
        .compile(&request(&authored("integer", "")))
        .unwrap();
    assert!(model.is_ready_for_validation());
    let input = tree("{sample @count=003}");
    let diagnostics = validate_document_model(input.ast(), &model);
    assert!(diagnostics
        .iter()
        .any(|d| d.code == "cem.schema_validation.attribute_datatype_incomplete"));
    let policy = ReferenceScopePolicy::schema_defaults().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(input.clone(), Some(Default::default()), policy.clone());
    let report = host
        .validate_input(input.clone(), &model, policy.limits)
        .unwrap();
    assert!(report.complete && !report.failed);
}
