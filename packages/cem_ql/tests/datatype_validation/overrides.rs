use super::*;
use cem_ml::schema::{
    datatype_contracts::CompiledDatatypeContract, declaration_references::SchemaDeclarationHost,
    document_model::SchemaDocumentModel,
};
use cem_ml::value::reference_resolution::ReferenceResolutionHost;
use cem_ql::{datatype_names::*, datatype_overrides::*};

fn setup(
    declarations: &str,
) -> (
    cem_ql::schema_references::CemQlSchemaDeclarationHost,
    Vec<DatatypeSource>,
    DatatypeImplementations,
    DatatypeOverrideRegistry,
) {
    setup_with_registrations(declarations, |_, _| {})
}
fn setup_with_registrations(
    declarations: &str,
    configure: impl FnOnce(&[DatatypeSource], &mut DatatypeImplementations),
) -> (
    cem_ql::schema_references::CemQlSchemaDeclarationHost,
    Vec<DatatypeSource>,
    DatatypeImplementations,
    DatatypeOverrideRegistry,
) {
    let (mut host, sources) = types_fixture(declarations);
    let names = Arc::new(
        DatatypeNameCatalog::collect(
            &[DatatypeNameScope {
                scope: sources[0].scope().clone(),
                namespace: Some("urn:test".into()),
                imports: vec![],
                declarations: sources
                    .iter()
                    .map(|source| DatatypeNameDeclaration {
                        source: source.clone(),
                        aliases: Default::default(),
                    })
                    .collect(),
            }],
            &host,
            Default::default(),
        )
        .unwrap(),
    );
    host.install_datatype_names(names.clone()).unwrap();
    let scope = host
        .scope(&host.source_reference(sources[0].declaration().clone()))
        .unwrap();
    host.set_context(
        scope,
        Some(
            cem_ql::api::StandaloneExpressionContext::default().with_binding(
                "original",
                cem_ql::api::StandaloneExpressionBinding::any(ItemStream::once(native(
                    sources[0].declaration(),
                ))),
            ),
        ),
    );
    let mut implementations = DatatypeImplementations::default();
    for source in &sources {
        if source.attribute("kind").is_some() {
            implementations
                .register(implementation(
                    source,
                    DatatypeKind::Scalar,
                    ValueRepresentation::Scalar(ScalarRepresentation::String),
                ))
                .unwrap();
        }
    }
    configure(&sources, &mut implementations);
    let roots: Vec<_> = sources
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 1)
        .map(|(_, s)| s.clone())
        .collect();
    let compilation = compile_datatypes(
        sources[0].declaration().document().clone(),
        &roots,
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(compilation.is_ready(), "{:?}", compilation.issues);
    let model = SchemaDocumentModel {
        datatype_compilation: Some(Arc::new(compilation)),
        ..Default::default()
    };
    let registry = DatatypeOverrideRegistry::new(names, model).unwrap();
    (host, sources, implementations, registry)
}
const SIMPLE: &str = "{type @name=base @kind=scalar} {type @name=replacement @kind=scalar} {type @name=derived @base={#original}} {type @name=leaf @base=derived}";
fn override_request(sources: &[DatatypeSource], rebind: bool) -> DatatypeOverrideRequest {
    DatatypeOverrideRequest {
        scope: sources[0].scope().clone(),
        namespace: "urn:test".into(),
        name: "base".into(),
        expected: sources[0].clone(),
        replacement: sources[1].clone(),
        rebind: if rebind {
            vec![sources[2].attribute("base").unwrap().clone()]
        } else {
            vec![]
        },
    }
}
fn prepare(
    registry: &DatatypeOverrideRegistry,
    host: &cem_ql::schema_references::CemQlSchemaDeclarationHost,
    request: &DatatypeOverrideRequest,
    grants: &[DatatypeOverrideGrant],
    implementations: &DatatypeImplementations,
    limits: ReferenceTraversalLimits,
) -> Result<PreparedDatatypeOverride, DatatypeOverrideFailure> {
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    registry.prepare(
        &[request.clone()],
        grants,
        host,
        DatatypeOverrideOptions {
            implementations,
            validations: &Default::default(),
            runtime: &runtime,
            limits,
            preparation: Default::default(),
        },
    )
}
#[test]
fn overrides_pin_native_edges_unless_exact_slots_are_authorized() {
    for rebind in [false, true] {
        let (mut host, sources, implementations, mut registry) = setup(SIMPLE);
        let old = registry.model().clone();
        let request = override_request(&sources, rebind);
        let grant = registry.authorize(&request).unwrap();
        let prepared = prepare(
            &registry,
            &host,
            &request,
            &[grant],
            &implementations,
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        )
        .unwrap();
        assert!(Arc::ptr_eq(registry.model(), &old));
        registry.activate(&mut host, prepared).unwrap();
        let result = registry.model().datatype_compilation.as_ref().unwrap();
        let base = compiled(result, &sources[2]).base().unwrap();
        assert_eq!(
            base.source().declaration().identity(),
            sources[usize::from(rebind)].declaration().identity()
        );
        assert_eq!(
            compiled(result, &sources[3])
                .base()
                .unwrap()
                .base()
                .unwrap()
                .source()
                .declaration()
                .identity(),
            sources[usize::from(rebind)].declaration().identity()
        );
        assert_eq!(
            compiled(old.datatype_compilation.as_ref().unwrap(), &sources[2])
                .base()
                .unwrap()
                .source()
                .declaration()
                .identity(),
            sources[0].declaration().identity()
        );
        let DatatypeNameLookup::Target(selected) =
            registry
                .names()
                .lookup_in_scope(sources[0].scope(), Some("urn:test"), "base")
        else {
            panic!()
        };
        assert_eq!(
            selected.declaration().identity(),
            sources[1].declaration().identity()
        );
        assert!(sources[0]
            .declaration()
            .document()
            .nodes
            .iter()
            .all(|n| !matches!(
                n,
                CemAstNode::Reference {
                    targets: Some(_),
                    ..
                }
            )));
    }
}
#[test]
fn missing_altered_and_foreign_grants_cannot_replace_an_active_model() {
    let (host, sources, implementations, registry) = setup(SIMPLE);
    let request = override_request(&sources, true);
    assert_eq!(
        prepare(
            &registry,
            &host,
            &request,
            &[],
            &implementations,
            ReferenceTraversalLimits::schema_defaults().unwrap()
        )
        .unwrap_err()
        .code,
        "datatype-override-unauthorized"
    );
    let mut altered = request.clone();
    altered.rebind.clear();
    let grant = registry.authorize(&altered).unwrap();
    assert!(prepare(
        &registry,
        &host,
        &request,
        &[grant],
        &implementations,
        ReferenceTraversalLimits::schema_defaults().unwrap()
    )
    .is_err());
    let (_, foreign_sources, _, foreign) = setup(SIMPLE);
    let foreign_grant = foreign
        .authorize(&override_request(&foreign_sources, true))
        .unwrap();
    assert!(prepare(
        &registry,
        &host,
        &request,
        &[foreign_grant],
        &implementations,
        ReferenceTraversalLimits::schema_defaults().unwrap()
    )
    .is_err());
}
#[test]
fn stale_contexts_generations_and_bounds_leave_previous_publication_active() {
    let (mut host, sources, implementations, mut registry) = setup(SIMPLE);
    let request = override_request(&sources, true);
    let grant = registry.authorize(&request).unwrap();
    let old = registry.model().clone();
    let prepared = prepare(
        &registry,
        &host,
        &request,
        &[grant.clone()],
        &implementations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap();
    let scope = host
        .scope(&host.source_reference(sources[0].declaration().clone()))
        .unwrap();
    host.set_context(scope, Some(Default::default()));
    assert_eq!(
        registry.activate(&mut host, prepared).unwrap_err().code,
        "datatype-override-stale-host"
    );
    assert!(Arc::ptr_eq(registry.model(), &old));
    registry.revoke_authorizations().unwrap();
    assert!(prepare(
        &registry,
        &host,
        &request,
        &[grant],
        &implementations,
        ReferenceTraversalLimits::schema_defaults().unwrap()
    )
    .is_err());
    let grant = registry.authorize(&request).unwrap();
    assert!(
        prepare(
            &registry,
            &host,
            &request,
            &[grant],
            &implementations,
            ReferenceTraversalLimits {
                max_work: 1,
                max_depth: 1
            }
        )
        .unwrap_err()
        .pending
    );
}
#[test]
fn rebound_cycles_fail_and_shared_dependents_use_one_new_contract() {
    let (host, sources, implementations, registry) = setup("{type @name=base @kind=scalar} {type @name=replacement @base=leaf} {type @name=derived @base={#original}} {type @name=leaf @base=derived}");
    let request = override_request(&sources, true);
    let grant = registry.authorize(&request).unwrap();
    let failure = prepare(
        &registry,
        &host,
        &request,
        &[grant],
        &implementations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap_err();
    assert!(failure
        .compilation
        .unwrap()
        .issues
        .iter()
        .any(|i| i.code.contains("cycle")));
    let (mut host, sources, implementations, mut registry) = setup("{type @name=base @kind=scalar} {type @name=replacement @kind=scalar} {type @name=left @base={#original}} {type @name=right @base=base}");
    let mut request = override_request(&sources, true);
    request
        .rebind
        .push(sources[3].attribute("base").unwrap().clone());
    let grant = registry.authorize(&request).unwrap();
    let candidate = prepare(
        &registry,
        &host,
        &request,
        &[grant],
        &implementations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap();
    registry.activate(&mut host, candidate).unwrap();
    let result = registry.model().datatype_compilation.as_ref().unwrap();
    assert!(Arc::ptr_eq(
        compiled(result, &sources[2]).base().unwrap(),
        compiled(result, &sources[3]).base().unwrap()
    ));
}

#[test]
fn original_identity_duplicates_and_changed_native_selection_are_rejected() {
    let (mut host, sources, implementations, registry) = setup(SIMPLE);
    let (_, foreign_sources, _, _) = setup(SIMPLE);
    let mut copied = override_request(&sources, false);
    copied.expected = foreign_sources[0].clone();
    assert_eq!(
        registry.authorize(&copied).unwrap_err().code,
        "datatype-override-original-mismatch"
    );
    let mut duplicated = override_request(&sources, true);
    duplicated.rebind.push(duplicated.rebind[0].clone());
    let grant = registry.authorize(&duplicated).unwrap();
    assert_eq!(
        prepare(
            &registry,
            &host,
            &duplicated,
            &[grant],
            &implementations,
            ReferenceTraversalLimits::schema_defaults().unwrap()
        )
        .unwrap_err()
        .code,
        "datatype-override-duplicate-dependency"
    );
    let request = override_request(&sources, true);
    let grant = registry.authorize(&request).unwrap();
    let scope = host
        .scope(&host.source_reference(sources[0].declaration().clone()))
        .unwrap();
    host.set_context(
        scope,
        Some(
            cem_ql::api::StandaloneExpressionContext::default().with_binding(
                "original",
                cem_ql::api::StandaloneExpressionBinding::any(ItemStream::once(native(
                    sources[1].declaration(),
                ))),
            ),
        ),
    );
    let failure = prepare(
        &registry,
        &host,
        &request,
        &[grant],
        &implementations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap_err();
    assert!(failure
        .compilation
        .unwrap()
        .issues
        .iter()
        .any(|i| i.code == "datatype-override-dependency-mismatch"));
}

#[test]
fn incompatible_rebound_representation_and_missing_implementation_keep_old_contracts() {
    let (host, sources, _, registry) = setup("{type @name=base @kind=scalar} {type @name=replacement @kind=scalar} {type @name=derived @kind=scalar @base={#original}} {type @name=leaf @base=derived}");
    let request = override_request(&sources, true);
    let old = registry.model().clone();
    for missing in [false, true] {
        let mut implementations = DatatypeImplementations::default();
        for (index, source) in sources
            .iter()
            .enumerate()
            .filter(|(_, s)| s.attribute("kind").is_some())
        {
            if index == 1 && missing {
                continue;
            }
            implementations
                .register(implementation(
                    source,
                    DatatypeKind::Scalar,
                    ValueRepresentation::Scalar(if index == 1 {
                        ScalarRepresentation::Integer
                    } else {
                        ScalarRepresentation::String
                    }),
                ))
                .unwrap();
        }
        let grant = registry.authorize(&request).unwrap();
        let failure = prepare(
            &registry,
            &host,
            &request,
            &[grant],
            &implementations,
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        )
        .unwrap_err();
        assert!(failure.compilation.unwrap().issues.iter().any(|i| i.code
            == if missing {
                "datatype-implementation-unavailable"
            } else {
                "incompatible-datatype-base"
            }));
        assert!(Arc::ptr_eq(registry.model(), &old));
    }
}

#[test]
fn cancellation_and_competing_preparations_cannot_publish() {
    let (mut host, sources, implementations, mut registry) = setup(SIMPLE);
    let request = override_request(&sources, true);
    let grant = registry.authorize(&request).unwrap();
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let candidate = registry
        .prepare(
            &[request.clone()],
            &[grant.clone()],
            &host,
            DatatypeOverrideOptions {
                implementations: &implementations,
                validations: &Default::default(),
                runtime: &runtime,
                limits: ReferenceTraversalLimits::schema_defaults().unwrap(),
                preparation: Default::default(),
            },
        )
        .unwrap();
    let old = registry.model().clone();
    control
        .cancel_root(Some("override fixture".into()), None)
        .unwrap();
    assert_eq!(
        registry.activate(&mut host, candidate).unwrap_err().code,
        "datatype-override-operation-stopped"
    );
    assert!(Arc::ptr_eq(registry.model(), &old));
    let first = prepare(
        &registry,
        &host,
        &request,
        &[grant.clone()],
        &implementations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap();
    let second = prepare(
        &registry,
        &host,
        &request,
        &[grant],
        &implementations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap();
    registry.activate(&mut host, first).unwrap();
    let active = registry.model().clone();
    assert_eq!(
        registry.activate(&mut host, second).unwrap_err().code,
        "datatype-override-stale-generation"
    );
    assert!(Arc::ptr_eq(registry.model(), &active));
}

#[test]
fn successive_explicit_rebinding_keeps_the_original_authored_selection() {
    let (mut host, sources, implementations, mut registry) = setup(SIMPLE);
    let first = override_request(&sources, true);
    let grant = registry.authorize(&first).unwrap();
    let candidate = prepare(
        &registry,
        &host,
        &first,
        &[grant],
        &implementations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap();
    registry.activate(&mut host, candidate).unwrap();
    let second = DatatypeOverrideRequest {
        expected: sources[1].clone(),
        replacement: sources[0].clone(),
        ..first
    };
    let grant = registry.authorize(&second).unwrap();
    let candidate = prepare(
        &registry,
        &host,
        &second,
        &[grant],
        &implementations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap();
    registry.activate(&mut host, candidate).unwrap();
    assert_eq!(registry.generation(), 2);
    let result = registry.model().datatype_compilation.as_ref().unwrap();
    assert_eq!(
        compiled(result, &sources[2])
            .base()
            .unwrap()
            .source()
            .declaration()
            .identity(),
        sources[0].declaration().identity()
    );
}

#[test]
fn public_binding_grants_do_not_authorize_crossing_to_a_foreign_replacement() {
    let (mut host, original) =
        types_fixture("{type @name=base @kind=scalar} {type @name=derived @base=base}");
    let (vendor_host, vendor) = types_fixture("{type @name=new @kind=scalar}");
    let from = host
        .scope(&host.source_reference(original[0].declaration().clone()))
        .unwrap();
    let to = host.register_scope(
        vendor_host
            .source_tree(vendor[0].declaration())
            .unwrap()
            .clone(),
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    host.register_datatype_source(vendor[0].clone()).unwrap();
    let scopes = [(&original, "urn:test"), (&vendor, "urn:vendor")].map(|(sources, namespace)| {
        DatatypeNameScope {
            scope: sources[0].scope().clone(),
            namespace: Some(namespace.into()),
            imports: vec![],
            declarations: sources
                .iter()
                .map(|source| DatatypeNameDeclaration {
                    source: source.clone(),
                    aliases: Default::default(),
                })
                .collect(),
        }
    });
    let names = Arc::new(DatatypeNameCatalog::collect(&scopes, &host, Default::default()).unwrap());
    host.install_datatype_names(names.clone()).unwrap();
    let mut implementations = DatatypeImplementations::default();
    for source in [&original[0], &vendor[0]] {
        implementations
            .register(implementation(
                source,
                DatatypeKind::Scalar,
                ValueRepresentation::Scalar(ScalarRepresentation::String),
            ))
            .unwrap();
    }
    let compilation = compile_datatypes(
        original[0].declaration().document().clone(),
        &original,
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(compilation.is_ready());
    let mut registry = DatatypeOverrideRegistry::new(
        names,
        SchemaDocumentModel {
            datatype_compilation: Some(Arc::new(compilation)),
            ..Default::default()
        },
    )
    .unwrap();
    let request = DatatypeOverrideRequest {
        scope: original[0].scope().clone(),
        namespace: "urn:test".into(),
        name: "base".into(),
        expected: original[0].clone(),
        replacement: vendor[0].clone(),
        rebind: vec![original[1].attribute("base").unwrap().clone()],
    };
    let grant = registry.authorize(&request).unwrap();
    assert_eq!(
        prepare(
            &registry,
            &host,
            &request,
            &[grant.clone()],
            &implementations,
            ReferenceTraversalLimits::schema_defaults().unwrap()
        )
        .unwrap_err()
        .code,
        "datatype-override-scope-denied"
    );
    host.allow_scope_crossing(from, to);
    let candidate = prepare(
        &registry,
        &host,
        &request,
        &[grant],
        &implementations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap();
    registry.activate(&mut host, candidate).unwrap();
    assert_eq!(
        compiled(
            registry.model().datatype_compilation.as_ref().unwrap(),
            &original[1]
        )
        .base()
        .unwrap()
        .source()
        .declaration()
        .identity(),
        vendor[0].declaration().identity()
    );
}

#[test]
fn attribute_model_and_public_names_switch_together_while_native_consumers_stay_pinned() {
    use cem_ml::schema::document_model::{
        attribute_facets::FacetFamily, shipped_datatypes::ShippedDatatype as T,
        validate_document_model,
    };
    use cem_ql::{
        attribute_activation::activate_attribute_datatypes,
        datatype_facets::{FacetProfileBinding, RegisteredFacetProfile},
        datatype_preparation::PreparationBinding,
    };
    for (field, ready) in [("", true), ("@default=words", false)] {
        let text = "{schema @name=test @namespace=urn:test | {types | {type @name=base @kind=scalar} {type @name=replacement @kind=scalar}} {attributes | {attribute @name=literal @type=base FIELD} {attribute @name=native @type={#original}}} {elements | {element @name=item @optional-attributes=\"literal native\"}}}".replace("FIELD", field);
        let profile = source(&text);
        let (mut host, sources) = types_fixture_source(profile);
        let names = Arc::new(
            DatatypeNameCatalog::collect(
                &[DatatypeNameScope {
                    scope: sources[0].scope().clone(),
                    namespace: Some("urn:test".into()),
                    imports: vec![],
                    declarations: sources
                        .iter()
                        .map(|source| DatatypeNameDeclaration {
                            source: source.clone(),
                            aliases: Default::default(),
                        })
                        .collect(),
                }],
                &host,
                Default::default(),
            )
            .unwrap(),
        );
        host.install_datatype_names(names.clone()).unwrap();
        host.enable_attribute_datatypes();
        let scope = host
            .scope(&host.source_reference(sources[0].declaration().clone()))
            .unwrap();
        host.set_context(
            scope,
            Some(
                cem_ql::api::StandaloneExpressionContext::default().with_binding(
                    "original",
                    cem_ql::api::StandaloneExpressionBinding::any(ItemStream::once(native(
                        sources[0].declaration(),
                    ))),
                ),
            ),
        );
        let mut implementations = DatatypeImplementations::default();
        for (source, ty) in [(&sources[0], T::String), (&sources[1], T::Integer)] {
            implementations
                .register(implementation(
                    source,
                    DatatypeKind::Scalar,
                    ty.representation(),
                ))
                .unwrap();
            implementations
                .select_facets(
                    source.clone(),
                    FacetProfileBinding::Ready(
                        RegisteredFacetProfile::new(
                            source.clone(),
                            "override:facets",
                            FacetFamily::Shipped(ty),
                        )
                        .unwrap(),
                    ),
                )
                .unwrap();
            implementations
                .select_preparation(
                    source.clone(),
                    PreparationBinding::Ready(
                        cem_ql::datatype_shipped::lexical_preparation(source.clone(), ty).unwrap(),
                    ),
                )
                .unwrap();
        }
        let tree = host.source_tree(sources[0].declaration()).unwrap().clone();
        let limits = ReferenceTraversalLimits::schema_defaults().unwrap();
        let mut model = host.compile("urn:test", tree, limits).unwrap();
        let mut compilation = compile_datatypes(
            sources[0].declaration().document().clone(),
            &[sources[0].clone()],
            &mut host,
            &implementations,
            &Default::default(),
            limits,
        );
        let control = OperationControl::default();
        let runtime = ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        };
        activate_attribute_datatypes(&mut model, &mut compilation, &mut host, limits, &runtime)
            .unwrap();
        assert!(compilation.is_ready(), "{:?}", compilation.issues);
        model.datatype_compilation = Some(Arc::new(compilation));
        assert!(model.is_ready_for_validation());
        let mut registry = DatatypeOverrideRegistry::new(names, model).unwrap();
        let old = registry.model().clone();
        use cem_ml::schema::attribute_datatypes::{AttributeDatatypeContext, AttributeDatatypeInput, AttributeDatatypeValue};
        let evidence_document = source("{schema | {item @literal=123}}");
        let evidence_tree = RetainedCemTree::from_shared(evidence_document.schema.document().clone(), "input.cem", "", Default::default(), None).unwrap();
        let evidence_source = evidence_tree.ast().nodes.iter().find(|node|matches!(node,CemAstNode::Attribute {expanded_name,..} if expanded_name.local_name=="literal")).unwrap();
        let evidence_context = AttributeDatatypeContext::default();
        let evidence_values = BTreeMap::new();
        macro_rules! evidence_input { ($value:expr) => { AttributeDatatypeInput {value:$value,source:evidence_source,source_tree:Some(evidence_tree.clone()),element_name:"item",attribute_values:&evidence_values,control:&control,context:Some(&evidence_context)} }; }
        let evidence = old.attribute_datatypes["literal"].prepare(evidence_input!(AttributeDatatypeValue::Lexical("123"))).handle.unwrap();

        let request = override_request(&sources, false);
        let grant = registry.authorize(&request).unwrap();
        let candidate = prepare(
            &registry,
            &host,
            &request,
            &[grant],
            &implementations,
            limits,
        );
        if !ready {
            assert!(
                candidate.is_err(),
                "invalid replacement default must block publication"
            );
            assert!(Arc::ptr_eq(registry.model(), &old));
            assert_eq!(old.attribute_datatypes["literal"].validate(evidence_input!(AttributeDatatypeValue::Prepared(&evidence))).accepted,Some(true));
            let DatatypeNameLookup::Target(target) =
                registry
                    .names()
                    .lookup_in_scope(sources[0].scope(), Some("urn:test"), "base")
            else {
                panic!()
            };
            assert_eq!(
                target.declaration().identity(),
                sources[0].declaration().identity()
            );
            continue;
        }
        let candidate = candidate.unwrap();
        let application = |text: &str| {
            cem_ml::parser::builder::CemAstBuilder::new(CemEventNormalizer::new(
                CemTokenizer::from_source(BytesSource::new(SourceId(1), text.as_bytes().to_vec())),
            ))
            .build()
        };
        assert!(Arc::ptr_eq(registry.model(), &old));
        registry.activate(&mut host, candidate).unwrap();
        assert_eq!(old.attribute_datatypes["literal"].validate(evidence_input!(AttributeDatatypeValue::Prepared(&evidence))).accepted,None);
        let fresh = registry.model().attribute_datatypes["literal"].prepare(evidence_input!(AttributeDatatypeValue::Lexical("123"))).handle.unwrap();
        assert_eq!(registry.model().attribute_datatypes["literal"].validate(evidence_input!(AttributeDatatypeValue::Prepared(&fresh))).accepted,Some(true));
        let input = application("{item @literal=oops @native=words}");
        let old_diagnostics = validate_document_model(&input, &old);
        assert!(
            !old_diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            "{old_diagnostics:?}"
        );
        let diagnostics = validate_document_model(&input, registry.model());
        assert!(
            diagnostics.iter().any(|d| d.severity.is_hard_violation()),
            "{diagnostics:?}"
        );
        let input = application("{item @literal=123 @native=words}");
        let diagnostics = validate_document_model(&input, registry.model());
        assert!(
            !diagnostics.iter().any(|d| d.severity.is_hard_violation()),
            "{diagnostics:?}"
        );
    }
}

#[test]
fn views_cannot_drop_effective_dependency_policy_by_recreating_a_registry() {
    let (mut host, sources, implementations, mut registry) = setup(SIMPLE);
    let request = override_request(&sources, true);
    let grant = registry.authorize(&request).unwrap();
    let candidate = prepare(
        &registry,
        &host,
        &request,
        &[grant],
        &implementations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap();
    registry.activate(&mut host, candidate).unwrap();
    assert_eq!(
        DatatypeOverrideRegistry::new(registry.names().clone(), registry.model().as_ref().clone())
            .unwrap_err()
            .code,
        "datatype-override-registry-required"
    );
}

#[test]
fn checked_facet_profiles_rebind_only_authorized_edges_and_keep_old_owners() {
    use cem_ml::schema::document_model::{
        attribute_facets::FacetFamily, shipped_datatypes::ShippedDatatype as T,
    };
    use cem_ql::datatype_facets::{FacetProfileBinding, RegisteredFacetProfile};
    for rebind in [false, true] {
        let (mut host, sources, implementations, mut registry) =
            setup_with_registrations(SIMPLE, |sources, implementations| {
                for (index, family) in [(0, T::Uri), (1, T::Path), (2, T::String)] {
                    let profile = RegisteredFacetProfile::new(
                        sources[index].clone(),
                        "same-id",
                        FacetFamily::Shipped(family),
                    )
                    .unwrap();
                    implementations
                        .select_facets(
                            sources[index].clone(),
                            if index == 2 {
                                FacetProfileBinding::CheckedReplacement(profile)
                            } else {
                                FacetProfileBinding::Ready(profile)
                            },
                        )
                        .unwrap();
                }
            });
        let old = registry.model().clone();
        let request = override_request(&sources, rebind);
        let grant = registry.authorize(&request).unwrap();
        let candidate = prepare(
            &registry,
            &host,
            &request,
            &[grant],
            &implementations,
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        )
        .unwrap();
        registry.activate(&mut host, candidate).unwrap();
        for selected in &sources[2..] {
            let before =
                compiled(old.datatype_compilation.as_ref().unwrap(), selected).facet_profiles();
            let after = compiled(
                registry.model().datatype_compilation.as_ref().unwrap(),
                selected,
            )
            .facet_profiles();
            assert_eq!(before.len(), 2);
            assert_eq!(after.len(), 2);
            assert_eq!(before[0].family(), FacetFamily::Shipped(T::Uri));
            assert_eq!(
                after[0].family(),
                FacetFamily::Shipped(if rebind { T::Path } else { T::Uri })
            );
            assert_eq!(
                after[0].source().declaration().identity(),
                sources[usize::from(rebind)].declaration().identity()
            );
            assert_eq!(
                after[1].source().declaration().identity(),
                sources[2].declaration().identity()
            );
        }
    }
}
