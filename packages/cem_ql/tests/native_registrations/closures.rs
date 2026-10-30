use super::*;
use cem_ml::schema::document_model::attribute_facets::FacetFamily;
use cem_ql::{
    datatype_enumeration::{ConstantCall, ConstantExecution, NativeConstantInterpreter},
    native_registrations::FacetMode,
};

impl NativeConstantInterpreter for Engine {
    fn interpret(&self, call: ConstantCall<'_>) -> ConstantExecution {
        self.enter(0, call.datatype);
        ConstantExecution::Prepared {
            value: vec![string(call.token.text())],
            diagnostics: vec![],
        }
    }
}
fn additions() -> Vec<(OperationMode, &'static str, ImplementationIdentity)> {
    vec![
        (
            OperationMode::Constant,
            "sample",
            ImplementationIdentity::constant_interpreter(
                "constant",
                hash(b"native"),
                "constant",
                ScalarRepresentation::String,
            ),
        ),
        (
            OperationMode::FacetProfile(FacetMode::Ready),
            "sample",
            ImplementationIdentity::facet_profile(
                "facets",
                hash(b"native"),
                "facets",
                FacetFamily::Shipped(ShippedDatatype::String),
            ),
        ),
        (
            OperationMode::ListItems(PreparationMode::Ready),
            "names",
            ImplementationIdentity::list_items(
                "items",
                hash(b"native"),
                "items",
                ScalarRepresentation::String,
            ),
        ),
    ]
}
#[test]
fn additional_requirements_roundtrip_without_implying_available_implementations() {
    for (mode, name, id) in additions() {
        let req = shared_requirement(mode, name, id, None);
        let mut host = host();
        let scope = host
            .register_scope(
                req.source().clone(),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        assert!(matches!(
            host.link_native_operation(req, scope),
            Err(native::NativeLinkError::Unavailable)
        ));
    }
}

fn register_additional(
    host: &mut FreshStructuralHost,
    id: ImplementationIdentity,
    engine: Engine,
) -> Result<(), native::NativeLinkError> {
    match id.signature {
        NativeSignature::Constant(_) => host.register_native_constant_interpreter(id, engine),
        NativeSignature::FacetProfile(family) => host.register_native_facet_profile(id, family),
        NativeSignature::ListItems(item) => host.register_list_item_preparation(id, item),
        _ => register(host, id, engine),
    }
}
#[test]
fn constant_interpretation_uses_original_values_and_refuses_retired_or_incompatible_implementations(
) {
    let (mode, _, id) = additions().remove(0);
    let root = shared_requirement(mode, "sample", id.clone(), None);
    let mut host = host();
    let engine = Engine::default();
    register_additional(&mut host, id.clone(), engine.clone()).unwrap();
    let scope = host
        .register_scope(
            root.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let binding = host.link_native_operation(root.clone(), scope).unwrap();
    let mut registrations = implementations(&root);
    let source = datatype(root.capture(), "sample");
    binding.install(&source, &mut registrations).unwrap();
    registrations
        .select_equality(
            source.clone(),
            cem_ql::datatype_enumeration::EqualityBinding::Ready(
                cem_ql::datatype_enumeration::RegisteredScalarEquality::new(
                    source.clone(),
                    "eq",
                    ScalarRepresentation::String,
                    engine.clone(),
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let result = compile(&root, &["choice"], &registrations);
    let choice = compiled(&result, "choice");
    assert_eq!(engine.calls[0].load(Ordering::Acquire), 2);
    assert!(choice.enumerations()[0]
        .constants()
        .iter()
        .all(|c| Arc::ptr_eq(c.token.source.document(), root.source().ast_owner())));
    binding.retire();
    let result = compile(&root, &["choice"], &registrations);
    assert!(!result.is_ready());
    assert!(
        result.issues.iter().any(|i| i.code.contains("unavailable")),
        "{:?}",
        result.issues
    );
    let mut wrong = id.clone();
    wrong.module_hash = hash(b"other");
    let mut fresh = super::super::host();
    register_additional(&mut fresh, wrong, engine).unwrap();
    let scope = fresh
        .register_scope(
            root.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    assert!(matches!(
        fresh.link_native_operation(root, scope),
        Err(native::NativeLinkError::IdentityMismatch)
    ));
}

fn closure_text() -> String {
    text().replacen("{behaviors |", "{attributes | {attribute @name=value @type=leaf @minLength=2 @pattern=\"[a-z]+\"} {attribute @name=items @type=names @minItems=0}} {behaviors |", 1)
}
fn capsule(
    mode: OperationMode,
    name: &str,
    id: ImplementationIdentity,
    root: Option<&native::VerifiedNativeRequirement>,
) -> Arc<native::VerifiedNativeRequirement> {
    let text = closure_text();
    let c = capture(&text);
    let built = native::export_operation(
        &c,
        &datatype(&c, name),
        mode,
        SourceIdentity::new("file:///apps/cart/types.cem", SourceId(1), text.as_bytes()),
        id,
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    let verified = native::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap();
    Arc::new(if let Some(root) = root {
        verified
            .share_source(root, &OperationControl::default())
            .unwrap()
    } else {
        verified
    })
}
fn registrations(
    root: &native::VerifiedNativeRequirement,
    tokenizer: bool,
) -> DatatypeImplementations {
    let mut registry = DatatypeImplementations::default();
    for name in ["sample", "leaf", "choice", "names"] {
        let list = name == "names";
        registry
            .register(DatatypeImplementation {
                source: datatype(root.capture(), name),
                kind: if list {
                    DatatypeKind::List
                } else {
                    DatatypeKind::Scalar
                },
                representation: if list {
                    ValueRepresentation::List(ScalarRepresentation::String)
                } else {
                    ValueRepresentation::Scalar(ScalarRepresentation::String)
                },
                accepted_bases: vec![],
                bounds: Default::default(),
                tokenizer: if list && tokenizer {
                    TokenizerBinding::Ready(RegisteredTokenizer::whitespace())
                } else {
                    TokenizerBinding::Absent
                },
                validator: None,
            })
            .unwrap();
    }
    registry
}
fn compilation(
    root: &native::VerifiedNativeRequirement,
    names: &[&str],
    registrations: &DatatypeImplementations,
) -> (
    DatatypeCompilation,
    cem_ql::schema_references::CemQlSchemaDeclarationHost,
) {
    let mut host = cem_ql::schema_references::CemQlSchemaDeclarationHost::new();
    host.register_scope(
        root.source().clone(),
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    for name in ["sample", "leaf", "choice", "names", "derived"] {
        let source = datatype(root.capture(), name);
        host.register_datatype_source(source.clone()).unwrap();
        host.bind_literal_datatype(source.scope(), name, source.declaration().clone())
            .unwrap();
    }
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let result = compile_datatypes_with_runtime(
        root.source().ast_owner().clone(),
        &names
            .iter()
            .map(|n| datatype(root.capture(), n))
            .collect::<Vec<_>>(),
        &mut host,
        registrations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
        &runtime,
        Default::default(),
    );
    (result, host)
}
fn bound(
    root: &native::VerifiedNativeRequirement,
    c: &DatatypeCompilation,
    host: &mut cem_ql::schema_references::CemQlSchemaDeclarationHost,
    name: &str,
    ty: &str,
) -> cem_ql::datatype_facets::BoundAttributeFacets {
    let declaration = root.capture().document().nodes.iter().find_map(|n| match n {
        CemAstNode::Element {expanded_name, node_id, attributes,..} if expanded_name.local_name=="attribute" && attributes.iter().any(|a| matches!(root.capture().document().get(*a), Some(CemAstNode::Attribute {expanded_name, value:Some(v),..}) if expanded_name.local_name=="name" && v==name)) => SchemaDeclarationNode::new(root.capture().document().clone(), *node_id), _ => None,
    }).unwrap();
    let slot = match declaration.node() {
        CemAstNode::Element {attributes,..} => attributes.iter().find_map(|a| matches!(declaration.document().get(*a), Some(CemAstNode::Attribute {expanded_name,..}) if expanded_name.local_name=="type").then(|| SchemaDeclarationNode::new(declaration.document().clone(), *a).unwrap())).unwrap(), _ => unreachable!(),
    };
    host.bind_literal_attribute_type(slot, datatype(root.capture(), ty).declaration().clone())
        .unwrap();
    assert!(c.is_ready(), "{:?}", c.issues);
    let binding = cem_ql::attribute_datatypes::bind_attribute_datatype(
        declaration,
        c,
        host,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap();
    assert!(
        binding.bound.is_some(),
        "{:?} {:?}",
        binding.state,
        binding.issue
    );
    binding
        .bound
        .unwrap()
        .compile_facets("urn:example", Default::default())
        .unwrap()
}
fn context<'a>(
    b: &'a cem_ql::datatype_facets::BoundAttributeFacets,
) -> cem_ml::schema::document_model::attribute_facets::FacetContext<'a> {
    static EMPTY: std::sync::LazyLock<BTreeMap<String, String>> =
        std::sync::LazyLock::new(BTreeMap::new);
    static DIAGNOSTICS: std::sync::LazyLock<
        BTreeMap<String, cem_ml::schema::document_model::DiagnosticBehavior>,
    > = std::sync::LazyLock::new(BTreeMap::new);
    cem_ml::schema::document_model::attribute_facets::FacetContext {
        element_name: "test",
        source: b.binding().declaration.node(),
        attribute_values: &EMPTY,
        diagnostic_behaviors: &DIAGNOSTICS,
    }
}
#[test]
fn checked_facet_profiles_keep_local_constraints_and_expire_sealed_preparation() {
    let (mode, _, prep_id) = descriptors().remove(0);
    let root = capsule(mode, "sample", prep_id.clone(), None);
    let base_id = ImplementationIdentity::facet_profile(
        "base-facets",
        hash(b"native"),
        "facets",
        FacetFamily::Shipped(ShippedDatatype::String),
    );
    let leaf_id = ImplementationIdentity::facet_profile(
        "leaf-facets",
        hash(b"native"),
        "facets",
        FacetFamily::Shipped(ShippedDatatype::String),
    );
    let base_req = capsule(
        OperationMode::FacetProfile(FacetMode::Ready),
        "sample",
        base_id.clone(),
        Some(&root),
    );
    let leaf_req = capsule(
        OperationMode::FacetProfile(FacetMode::CheckedReplacement),
        "leaf",
        leaf_id.clone(),
        Some(&root),
    );
    let mut native = host();
    let engine = Engine::default();
    register(&mut native, prep_id, engine.clone()).unwrap();
    register_additional(&mut native, base_id, engine.clone()).unwrap();
    register_additional(&mut native, leaf_id, engine).unwrap();
    let scope = native
        .register_scope(
            root.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let prep = native.link_native_operation(root.clone(), scope).unwrap();
    let base = native.link_native_operation(base_req, scope).unwrap();
    let leaf = native.link_native_operation(leaf_req, scope).unwrap();
    let mut registry = registrations(&root, true);
    prep.install(&datatype(root.capture(), "sample"), &mut registry)
        .unwrap();
    base.install(&datatype(root.capture(), "sample"), &mut registry)
        .unwrap();
    leaf.install(&datatype(root.capture(), "leaf"), &mut registry)
        .unwrap();
    let (c, mut host) = compilation(&root, &["leaf"], &registry);
    assert!(c.is_ready(), "{:?}", c.issues);
    let b = bound(&root, &c, &mut host, "value", "leaf");
    assert_eq!(b.profiles().len(), 2);
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    for (word, accepted) in [("ab", true), ("a", false), ("a b", false)] {
        let result = b.validate_lexical(
            &PreparationInput {
                lexical: lexical(word),
                candidate: vec![],
                fallback: Default::default(),
            },
            context(&b),
            &runtime,
            Default::default(),
        );
        assert_eq!(result.accepted, Some(accepted), "{result:?}");
    }
    let invocation = cem_ql::preparation_evidence::AttributePreparationInvocation::new(
        &b,
        PreparationInput {
            lexical: lexical("ab"),
            candidate: vec![],
            fallback: Default::default(),
        },
        &runtime,
        Default::default(),
    )
    .unwrap();
    let evidence = invocation.prepare().evidence.unwrap();
    assert!(evidence.is_live());
    base.retire();
    assert!(!evidence.is_live());
    assert!(evidence.verify(&invocation).is_err());
    let result = b.validate_lexical(
        &PreparationInput {
            lexical: lexical("ab"),
            candidate: vec![],
            fallback: Default::default(),
        },
        context(&b),
        &runtime,
        Default::default(),
    );
    assert_eq!(result.accepted, None);
    assert!(b
        .binding()
        .compile_facets("urn:example", Default::default())
        .is_err());
    let read = b.check_declaration_readiness(
        &host,
        &cem_ql::attribute_readiness::AttributeDiagnosticBindings {
            schema: node(root.capture(), "schema"),
            complete: true,
            behaviors: Default::default(),
        },
        &runtime,
        Default::default(),
    );
    assert_eq!(
        read.state,
        cem_ql::attribute_readiness::AttributeReadinessState::Pending
    );
}

#[test]
fn explicit_list_orchestration_preserves_spans_empty_input_and_live_owners() {
    let (mode, _, prep_id) = descriptors().remove(0);
    let root = capsule(mode, "sample", prep_id.clone(), None);
    let items_id = ImplementationIdentity::list_items(
        "items",
        hash(b"native"),
        "items",
        ScalarRepresentation::String,
    );
    let items_req = capsule(
        OperationMode::ListItems(PreparationMode::Ready),
        "names",
        items_id.clone(),
        Some(&root),
    );
    let mut child_id = items_id.clone();
    child_id.id = "derived-items".into();
    let child_req = capsule(
        OperationMode::ListItems(PreparationMode::CheckedReplacement),
        "derived",
        child_id.clone(),
        Some(&root),
    );
    let mut native = host();
    let engine = Engine::default();
    register(&mut native, prep_id, engine.clone()).unwrap();
    register_additional(&mut native, items_id, engine.clone()).unwrap();
    register_additional(&mut native, child_id, engine).unwrap();
    let scope = native
        .register_scope(
            root.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let prep = native.link_native_operation(root.clone(), scope).unwrap();
    let items = native.link_native_operation(items_req, scope).unwrap();
    let child = native.link_native_operation(child_req, scope).unwrap();
    let mut registry = registrations(&root, true);
    prep.install(&datatype(root.capture(), "sample"), &mut registry)
        .unwrap();
    items
        .install(&datatype(root.capture(), "names"), &mut registry)
        .unwrap();
    child
        .install(&datatype(root.capture(), "derived"), &mut registry)
        .unwrap();
    let (c, _) = compilation(&root, &["derived"], &registry);
    let d = compiled(&c, "derived");
    assert_eq!(d.preparation().unwrap().invocations(), 2);
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let request = |s| PreparationInput {
        lexical: lexical(s),
        candidate: vec![],
        fallback: Default::default(),
    };
    let result = d.prepare_lexical(&request("  α  beta "), &runtime, Default::default());
    assert_eq!(result.accepted, Some(true), "{result:?}");
    assert_eq!(result.token_spans, vec![2..4, 6..10]);
    assert_eq!(
        d.prepare_lexical(&request(""), &runtime, Default::default())
            .accepted,
        Some(true)
    );
    items.retire();
    let result = d.prepare_lexical(&request(""), &runtime, Default::default());
    assert_eq!(result.accepted, None);
    assert!(result.value.is_none());
    assert!(matches!(result.stopped, Some(PreparationStop::Unavailable)));
    control.cancel_root(None, None).unwrap();
    assert!(matches!(
        d.prepare_lexical(&request(""), &runtime, Default::default())
            .stopped,
        Some(PreparationStop::Control(_))
    ));
}

use cem_ql::{
    datatype_conversion::{DatatypeConversionResultAdapter, SourceDatatypeConverter},
    datatype_enumeration::{
        DatatypeConstantResultAdapter, DatatypeEqualityResultAdapter, SourceConstantInterpreter,
        SourceScalarEquality,
    },
    native_registrations::SourceProfile,
    structural_receiver::NativeResultAdapter,
};
fn record(fields: Vec<(&str, Vec<Item>)>) -> RuleExecution {
    RuleExecution::Complete(ItemStream::once(Item::Record(
        fields.into_iter().map(|(k, v)| (k.into(), v)).collect(),
    )))
}
fn diagnostic(source: &Item) -> Item {
    Item::Record(BTreeMap::from([
        ("code".into(), vec![string("source-profile")]),
        ("severity".into(), vec![string("info")]),
        ("message".into(), vec![string("checked")]),
        ("source".into(), vec![source.clone()]),
    ]))
}
impl SourceDatatypeConverter for Engine {
    fn convert(&self, call: ConversionCall<'_>) -> RuleExecution {
        self.enter(1, call.datatype);
        let ConversionValue::Lexical(lexical) = call.value else {
            unreachable!()
        };
        match lexical.text.as_ref() {
            "unavailable" => return RuleExecution::Unavailable(vec![]),
            "pending" => return RuleExecution::Pending(vec![]),
            _ => (),
        }
        let value = if self.invalid_output {
            vec![Item::Atomic(AtomValue::Boolean(true))]
        } else {
            vec![string(&lexical.text)]
        };
        record(vec![
            (
                "status",
                vec![string(if self.changed { "pending" } else { "converted" })],
            ),
            ("value", value),
            ("diagnostics", vec![diagnostic(call.datatype)]),
        ])
    }
}
impl SourceScalarEquality for Engine {
    fn compare(&self, call: EqualityCall<'_>) -> RuleExecution {
        self.enter(2, call.datatype);
        record(vec![
            (
                "equal",
                vec![if self.invalid_output {
                    string("yes")
                } else {
                    Item::Atomic(AtomValue::Boolean(call.left == call.right))
                }],
            ),
            ("diagnostics", vec![]),
        ])
    }
}
impl SourceConstantInterpreter for Engine {
    fn interpret(&self, call: ConstantCall<'_>) -> RuleExecution {
        self.enter(0, call.datatype);
        let value = if self.invalid_output {
            Item::Atomic(AtomValue::Boolean(true))
        } else {
            string(call.token.text())
        };
        record(vec![
            (
                "status",
                vec![string(if self.changed { "pending" } else { "prepared" })],
            ),
            ("value", vec![value]),
            ("diagnostics", vec![diagnostic(call.candidate)]),
        ])
    }
}
fn source_descriptors() -> Vec<(SourceProfile, ImplementationIdentity)> {
    let conversion = match descriptors().remove(1).2.signature {
        NativeSignature::Conversion(sig) => sig,
        _ => unreachable!(),
    };
    vec![
        (
            SourceProfile::Conversion,
            ImplementationIdentity::source_conversion(
                "convert-source",
                hash(b"native"),
                "convert",
                conversion,
            ),
        ),
        (
            SourceProfile::Equality,
            ImplementationIdentity::source_equality(
                "equal-source",
                hash(b"native"),
                "equal",
                ScalarRepresentation::String,
            ),
        ),
        (
            SourceProfile::Constant,
            ImplementationIdentity::source_constant(
                "constant-source",
                hash(b"native"),
                "constant",
                ScalarRepresentation::String,
            ),
        ),
    ]
}
fn source_text(selected: SourceProfile) -> String {
    let profiles = source_descriptors().iter().filter(|(profile, _)| *profile == selected).map(|(profile,id)| {
        let (execution, roles) = match profile {
            SourceProfile::Conversion => ("datatype-conversion", vec![("value","string",true),("datatype","node",true),("candidate","node",false)]),
            SourceProfile::Equality => ("datatype-equality", vec![("left","string",true),("right","string",true),("datatype","node",true)]),
            SourceProfile::Constant => ("datatype-constant", vec![("value","string",true),("datatype","node",true),("candidate","node",true)]),
        };
        let inputs = roles.iter().map(|(name,ty,required)| format!("{{input-binding @name={name} @type=schema:{ty} @source={name} @required={required} @cardinality={}}}", if *required { "one" } else { "zero-or-one" })).collect::<String>();
        format!("{{behavior @name={} @implementation=engine @primitive={} @execution={execution} | {{inputs | {inputs}}} {{result @type=schema:{execution}-result}} }}",id.id,id.id)
    }).collect::<String>();
    format!("@ns schema = \"https://cem.dev/ns/schema/1\"\n@default schema\n{{schema @name=example @namespace=urn:example | {{types | {{type @name=sample @kind=scalar}} {{type @name=leaf @base=sample}} {{type @name=choice @base=sample @values=\"red blue\"}} {{type @name=names @kind=list @base=sample}} {{type @name=derived @list-base=names}} }} {{behaviors | {profiles}}} }}")
}
fn source_requirement(
    profile: SourceProfile,
    id: ImplementationIdentity,
    root: Option<&native::VerifiedNativeRequirement>,
) -> Arc<native::VerifiedNativeRequirement> {
    let text = source_text(profile);
    let c = capture(&text);
    let behavior = c.document().nodes.iter().find_map(|n| match n {
        CemAstNode::Element{expanded_name,node_id,attributes,..} if expanded_name.local_name=="behavior" && attributes.iter().any(|a| matches!(c.document().get(*a), Some(CemAstNode::Attribute{expanded_name,value:Some(v),..}) if expanded_name.local_name=="name" && v==&id.id)) => SchemaDeclarationNode::new(c.document().clone(),*node_id), _=>None,
    }).unwrap();
    let built = native::export_source_operation(
        &c,
        &datatype(&c, "sample"),
        &behavior,
        profile,
        SourceIdentity::new("file:///apps/cart/types.cem", SourceId(1), text.as_bytes()),
        id,
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    let req = native::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap();
    Arc::new(if let Some(root) = root {
        req.share_source(root, &OperationControl::default())
            .unwrap()
    } else {
        req
    })
}
fn adapter_for(profile: SourceProfile) -> NativeResultAdapter {
    let c = capture(
        cem_ml::schema::package_sources::builtin_schema_package_source("schema")
            .unwrap()
            .schema_source,
    );
    let source = ValueContractSource::new(
        node(&c, "schema"),
        NS,
        BTreeMap::from([("schema".into(), NS.into())]),
    )
    .with_captured_names(c)
    .unwrap();
    let contracts = Arc::new(ValueContracts::compile(&[source], Default::default()).unwrap());
    let diagnostic = ContractName::new(NS, "datatype-diagnostic");
    match profile {
        SourceProfile::Conversion => NativeResultAdapter::Conversion(
            DatatypeConversionResultAdapter::new(
                contracts,
                ContractName::new(NS, "datatype-conversion-result"),
                diagnostic,
            )
            .unwrap(),
        ),
        SourceProfile::Equality => NativeResultAdapter::Equality(
            DatatypeEqualityResultAdapter::new(
                contracts,
                ContractName::new(NS, "datatype-equality-result"),
                diagnostic,
            )
            .unwrap(),
        ),
        SourceProfile::Constant => NativeResultAdapter::Constant(
            DatatypeConstantResultAdapter::new(
                contracts,
                ContractName::new(NS, "datatype-constant-result"),
                diagnostic,
            )
            .unwrap(),
        ),
    }
}
fn register_source(
    host: &mut FreshStructuralHost,
    id: ImplementationIdentity,
    engine: Engine,
) -> Result<(), native::NativeLinkError> {
    match id.signature {
        NativeSignature::SourceConversion(_) => host.register_source_converter(id, engine),
        NativeSignature::SourceEquality(_) => host.register_source_equality(id, engine),
        NativeSignature::SourceConstant(_) => host.register_source_constant_interpreter(id, engine),
        _ => unreachable!(),
    }
}
#[test]
fn source_profiles_use_original_behaviors_checked_adapters_and_diagnostics_without_query_replay() {
    for (profile, id) in source_descriptors() {
        let root = source_requirement(profile, id.clone(), None);
        let mut native = host();
        let engine = Engine::default();
        register_source(&mut native, id, engine.clone()).unwrap();
        let scope = native
            .register_scope(
                root.source().clone(),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        assert!(matches!(
            native.link_native_operation(root.clone(), scope),
            Err(native::NativeLinkError::InvalidSource)
        ));
        let wrong = if profile == SourceProfile::Conversion {
            SourceProfile::Equality
        } else {
            SourceProfile::Conversion
        };
        assert!(matches!(
            native.link_native_source_operation(root.clone(), scope, adapter_for(wrong)),
            Err(native::NativeLinkError::InvalidSource)
        ));
        let binding = native
            .link_native_source_operation(root.clone(), scope, adapter_for(profile))
            .unwrap();
        let mut registry = registrations(&root, true);
        let source = datatype(root.capture(), "sample");
        binding.install(&source, &mut registry).unwrap();
        if profile == SourceProfile::Equality {
            registry
                .select_constant_interpreter(
                    source.clone(),
                    ConstantBinding::Ready(
                        cem_ql::datatype_enumeration::RegisteredConstantInterpreter::new(
                            source.clone(),
                            "constants",
                            ScalarRepresentation::String,
                            Engine::default(),
                        )
                        .unwrap(),
                    ),
                )
                .unwrap();
        } else if profile == SourceProfile::Constant {
            registry
                .select_equality(
                    source.clone(),
                    cem_ql::datatype_enumeration::EqualityBinding::Ready(
                        cem_ql::datatype_enumeration::RegisteredScalarEquality::new(
                            source.clone(),
                            "equality",
                            ScalarRepresentation::String,
                            Engine::default(),
                        )
                        .unwrap(),
                    ),
                )
                .unwrap();
        }
        let target = if profile == SourceProfile::Conversion {
            "sample"
        } else {
            "choice"
        };
        let (c, _) = compilation(&root, &[target], &registry);
        let d = compiled(&c, target);
        let control = OperationControl::default();
        let runtime = ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        };
        assert_eq!(
            d.validate(&input("red"), &runtime, Default::default())
                .accepted,
            Some(true)
        );
        assert_eq!(
            engine.calls[1].load(Ordering::Acquire),
            0,
            "validation called conversion"
        );
        if profile == SourceProfile::Conversion {
            let request = |s| ConversionInput {
                value: ConversionValue::Lexical(lexical(s)),
                candidate: vec![],
                fallback: Default::default(),
            };
            let result = d.convert(&request("red"), &runtime, Default::default());
            assert_eq!(result.accepted, Some(true), "{:?}", result.stopped);
            assert_eq!(result.value, Some(vec![string("red")]));
            assert!(result
                .diagnostics
                .iter()
                .any(|d| d.node.is_some() && d.source_map.is_some()));
            for (word, pending) in [("pending", true), ("unavailable", false)] {
                let r = d.convert(&request(word), &runtime, Default::default());
                assert_eq!(r.accepted, None);
                assert!(matches!(
                    (r.stopped, pending),
                    (Some(ConversionStop::Pending), true)
                        | (Some(ConversionStop::Unavailable), false)
                ));
            }
            native.close();
            assert_unavailable(1, &d);
        } else {
            assert_eq!(d.enumerations()[0].constants().len(), 2);
            assert!(d.enumerations()[0]
                .constants()
                .iter()
                .all(|c| Arc::ptr_eq(c.token.source.document(), root.source().ast_owner())));
            assert_eq!(
                d.validate(&input("green"), &runtime, Default::default())
                    .accepted,
                Some(false)
            );
            native.close();
            if profile == SourceProfile::Equality {
                assert_unavailable(2, &d);
            } else {
                assert!(!compilation(&root, &[target], &registry).0.is_ready());
            }
        }
    }
}

#[test]
fn typed_producers_expire_when_their_facet_owner_retires() {
    let id = ImplementationIdentity::facet_profile(
        "typed-list",
        hash(b"native"),
        "facets",
        FacetFamily::TypedList(ScalarRepresentation::String),
    );
    let root = capsule(
        OperationMode::FacetProfile(FacetMode::Ready),
        "names",
        id.clone(),
        None,
    );
    let mut native = host();
    native
        .register_native_facet_profile(id, FacetFamily::TypedList(ScalarRepresentation::String))
        .unwrap();
    let item_id = ImplementationIdentity::facet_profile(
        "typed-item",
        hash(b"native"),
        "facets",
        FacetFamily::TypedScalar(ScalarRepresentation::String),
    );
    let item_req = capsule(
        OperationMode::FacetProfile(FacetMode::Ready),
        "sample",
        item_id.clone(),
        Some(&root),
    );
    native
        .register_native_facet_profile(
            item_id,
            FacetFamily::TypedScalar(ScalarRepresentation::String),
        )
        .unwrap();
    let scope = native
        .register_scope(
            root.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let binding = native.link_native_operation(root.clone(), scope).unwrap();
    let item_binding = native.link_native_operation(item_req, scope).unwrap();
    let mut registry = registrations(&root, false);
    item_binding
        .install(&datatype(root.capture(), "sample"), &mut registry)
        .unwrap();
    binding
        .install(&datatype(root.capture(), "names"), &mut registry)
        .unwrap();
    let (c, mut host) = compilation(&root, &["names"], &registry);
    let b = bound(&root, &c, &mut host, "items", "names");
    let producer = cem_ql::external_typed::ExternalTypedProducer::new(&b, "test").unwrap();
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    assert!(producer
        .produce(Some(vec![]), &runtime, Default::default())
        .is_ok());
    binding.retire();
    assert!(producer
        .produce(Some(vec![]), &runtime, Default::default())
        .is_err());
    assert!(cem_ql::external_typed::ExternalTypedProducer::new(&b, "test").is_err());
}

#[test]
fn source_results_refuse_wrong_shapes_statuses_and_late_success_without_retaining_callbacks() {
    for (profile, id) in source_descriptors() {
        for case in 0..3 {
            let root = source_requirement(profile, id.clone(), None);
            let native_control = OperationControl::default();
            let mut native =
                FreshStructuralHost::new(&hash(b"registry"), native_control.clone()).unwrap();
            let engine = Engine {
                invalid_output: case == 0,
                changed: case == 1,
                cancel: Some(native_control),
                ..Default::default()
            };
            let weak = Arc::downgrade(&engine.owner);
            let armed = engine.armed.clone();
            register_source(&mut native, id.clone(), engine).unwrap();
            let scope = native
                .register_scope(
                    root.source().clone(),
                    Some(Default::default()),
                    ReferenceScopePolicy::schema_defaults().unwrap(),
                )
                .unwrap();
            let binding = native
                .link_native_source_operation(root.clone(), scope, adapter_for(profile))
                .unwrap();
            let mut registry = registrations(&root, true);
            let source = datatype(root.capture(), "sample");
            binding.install(&source, &mut registry).unwrap();
            if profile == SourceProfile::Equality {
                registry
                    .select_constant_interpreter(
                        source.clone(),
                        ConstantBinding::Ready(
                            cem_ql::datatype_enumeration::RegisteredConstantInterpreter::new(
                                source.clone(),
                                "constant",
                                ScalarRepresentation::String,
                                Engine::default(),
                            )
                            .unwrap(),
                        ),
                    )
                    .unwrap();
            }
            if profile == SourceProfile::Constant {
                registry
                    .select_equality(
                        source.clone(),
                        cem_ql::datatype_enumeration::EqualityBinding::Ready(
                            cem_ql::datatype_enumeration::RegisteredScalarEquality::new(
                                source.clone(),
                                "equality",
                                ScalarRepresentation::String,
                                Engine::default(),
                            )
                            .unwrap(),
                        ),
                    )
                    .unwrap();
            }
            if case == 2 {
                armed.store(true, Ordering::Release);
            }
            let target = if profile == SourceProfile::Conversion {
                "sample"
            } else {
                "choice"
            };
            let (c, _) = compilation(&root, &[target], &registry);
            if profile == SourceProfile::Conversion {
                let d = compiled(&c, target);
                let control = OperationControl::default();
                let runtime = ValidationRuntime {
                    control: &control,
                    scope: ROOT_EXECUTION_SCOPE_ID,
                    query: Default::default(),
                };
                let result = d.convert(
                    &ConversionInput {
                        value: ConversionValue::Lexical(lexical("red")),
                        candidate: vec![],
                        fallback: Default::default(),
                    },
                    &runtime,
                    Default::default(),
                );
                assert_eq!(result.accepted, None);
                if case == 2 {
                    assert!(matches!(result.stopped, Some(ConversionStop::Unavailable)));
                } else {
                    assert!(matches!(result.stopped, Some(ConversionStop::Failed)));
                }
            } else if profile == SourceProfile::Equality {
                let d = compiled(&c, target);
                let control = OperationControl::default();
                let runtime = ValidationRuntime {
                    control: &control,
                    scope: ROOT_EXECUTION_SCOPE_ID,
                    query: Default::default(),
                };
                let result = d.validate(&input("red"), &runtime, Default::default());
                assert_eq!(result.accepted, if case == 1 { Some(true) } else { None });
            } else {
                assert!(!c.is_ready(), "profile {profile:?}, case {case}");
            }

            drop(native);
            assert!(
                weak.upgrade().is_none(),
                "a retained source registration kept native code alive"
            );
        }
    }
}

#[test]
fn additional_modes_budgets_and_actual_family_mismatches_refuse_without_fallback() {
    for (mode, name, id) in additions() {
        let root = shared_requirement(mode, name, id.clone(), None);
        let mut native = host();
        if let NativeSignature::FacetProfile(_) = id.signature {
            assert_eq!(
                native.register_native_facet_profile(
                    id.clone(),
                    FacetFamily::Shipped(ShippedDatatype::Boolean)
                ),
                Err(native::NativeLinkError::IdentityMismatch)
            );
        }
        if let NativeSignature::ListItems(_) = id.signature {
            assert_eq!(
                native.register_list_item_preparation(id.clone(), ScalarRepresentation::Boolean),
                Err(native::NativeLinkError::IdentityMismatch)
            );
        }
        register_additional(&mut native, id.clone(), Engine::default()).unwrap();
        let scope = native
            .register_scope(
                root.source().clone(),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        let binding = native.link_native_operation(root.clone(), scope).unwrap();
        let mut registry = implementations(&root);
        binding
            .install(&datatype(root.capture(), name), &mut registry)
            .unwrap();
        assert_eq!(
            binding.install(&datatype(root.capture(), name), &mut registry),
            Err(native::NativeLinkError::Duplicate)
        );
        let foreign = shared_requirement(mode, name, id, None);
        assert!(matches!(
            native.link_native_operation(foreign, scope),
            Err(native::NativeLinkError::InvalidSource)
        ));
        native.close();
        assert_eq!(
            binding.install(
                &datatype(root.capture(), name),
                &mut DatatypeImplementations::default()
            ),
            Err(native::NativeLinkError::Expired)
        );
    }
    let id = additions().remove(2).2;
    let root = capsule(
        OperationMode::ListItems(PreparationMode::Ready),
        "names",
        id.clone(),
        None,
    );
    let mut native = host();
    register_additional(&mut native, id, Engine::default()).unwrap();
    let scope = native
        .register_scope(
            root.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let binding = native.link_native_operation(root.clone(), scope).unwrap();
    let mut registry = registrations(&root, false);
    binding
        .install(&datatype(root.capture(), "names"), &mut registry)
        .unwrap();
    assert!(
        !compilation(&root, &["names"], &registry).0.is_ready(),
        "missing tokenizer/item preparation was inferred"
    );
}

#[test]
fn repinned_additional_records_refuse_changed_modes_abis_occurrences_and_profiles() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Packet {
        abi: String,
        ml_version: String,
        ql_version: String,
        registry_hash: String,
        identity: SourceIdentity,
        implementation: ImplementationIdentity,
        selection: Selection,
        source: cem_ml::schema::native_source::NativeSourceSnapshot,
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    enum Selection {
        Operation {
            scope: u32,
            declaration: u32,
            mode: OperationMode,
        },
        SourceOperation {
            scope: u32,
            declaration: u32,
            behavior: u32,
            profile: SourceProfile,
        },
    }
    let mut requests = additions()
        .into_iter()
        .map(|(mode, name, id)| shared_requirement(mode, name, id, None))
        .collect::<Vec<_>>();
    requests.extend(
        source_descriptors()
            .into_iter()
            .map(|(profile, id)| source_requirement(profile, id, None)),
    );
    for req in requests {
        // Re-export under the public API to get canonical bytes for this source.
        let source = req.capture();
        let pin = req.pin();
        let id = req.implementation().clone();
        let bytes = match &id.signature {
            NativeSignature::Constant(_) => {
                native::export_operation(
                    source,
                    &datatype(source, "sample"),
                    OperationMode::Constant,
                    pin.source.clone(),
                    id,
                    &pin.registry_hash,
                    &OperationControl::default(),
                )
                .unwrap()
                .bytes
            }
            NativeSignature::FacetProfile(_) => {
                native::export_operation(
                    source,
                    &datatype(source, "sample"),
                    OperationMode::FacetProfile(FacetMode::Ready),
                    pin.source.clone(),
                    id,
                    &pin.registry_hash,
                    &OperationControl::default(),
                )
                .unwrap()
                .bytes
            }
            NativeSignature::ListItems(_) => {
                native::export_operation(
                    source,
                    &datatype(source, "names"),
                    OperationMode::ListItems(PreparationMode::Ready),
                    pin.source.clone(),
                    id,
                    &pin.registry_hash,
                    &OperationControl::default(),
                )
                .unwrap()
                .bytes
            }
            _ => {
                let profile = match id.signature {
                    NativeSignature::SourceConversion(_) => SourceProfile::Conversion,
                    NativeSignature::SourceEquality(_) => SourceProfile::Equality,
                    _ => SourceProfile::Constant,
                };
                let behavior=source.document().nodes.iter().find_map(|n| match n { CemAstNode::Element{expanded_name,node_id,attributes,..} if expanded_name.local_name=="behavior" && attributes.iter().any(|a| matches!(source.document().get(*a),Some(CemAstNode::Attribute{expanded_name,value:Some(v),..}) if expanded_name.local_name=="primitive" && v==&id.id))=>SchemaDeclarationNode::new(source.document().clone(),*node_id),_=>None }).unwrap();
                native::export_source_operation(
                    source,
                    &datatype(source, "sample"),
                    &behavior,
                    profile,
                    pin.source.clone(),
                    id,
                    &pin.registry_hash,
                    &OperationControl::default(),
                )
                .unwrap()
                .bytes
            }
        };
        for change in 0..8 {
            let mut packet: Packet = rmp_serde::from_slice(&bytes[native::MAGIC.len()..]).unwrap();
            match change {
                0 => packet.abi.push_str("future"),
                1 => packet.implementation.abi.push_str("future"),
                2 => packet.identity.uri.push_str("other"),
                3 => packet.ql_version.push_str("future"),
                4 => packet.implementation.signature = NativeSignature::TokenizerSpans,
                _ => match &mut packet.selection {
                    Selection::Operation {
                        scope,
                        declaration,
                        mode,
                    } => match change {
                        5 => *scope = u32::MAX,
                        6 => *declaration = u32::MAX,
                        _ => *mode = OperationMode::Conversion,
                    },
                    Selection::SourceOperation {
                        scope,
                        declaration,
                        profile,
                        ..
                    } => match change {
                        5 => *scope = u32::MAX,
                        6 => *declaration = u32::MAX,
                        _ => {
                            *profile = if *profile == SourceProfile::Constant {
                                SourceProfile::Equality
                            } else {
                                SourceProfile::Constant
                            }
                        }
                    },
                },
            }
            let bad = [native::MAGIC, &rmp_serde::to_vec_named(&packet).unwrap()].concat();
            let mut pin = pin.clone();
            pin.hash = hash(&bad);
            assert!(
                native::verify(&bad, &pin, &OperationControl::default()).is_err(),
                "change {change}"
            );
        }
    }
}

#[test]
fn owner_retirement_during_result_adapter_reads_cannot_publish_a_verdict_or_value() {
    use cem_ql::eval::{QueryItemView, QueryItemViewKind};
    #[derive(Debug)]
    struct LazyRecord {
        control: OperationControl,
        fields: Vec<(String, Vec<Item>)>,
        reads: Arc<AtomicUsize>,
    }
    impl QueryItemView for LazyRecord {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "test.native-result"
        }
        fn identity(&self) -> String {
            "result".into()
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Record
        }
        fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
            self.reads.fetch_add(1, Ordering::AcqRel);
            self.control.cancel_root(None, None).unwrap();
            Some(self.fields.clone())
        }
    }
    #[derive(Debug, Clone)]
    struct Lazy {
        control: OperationControl,
        reads: Arc<AtomicUsize>,
    }
    impl Lazy {
        fn result(&self, profile: SourceProfile) -> RuleExecution {
            let fields = match profile {
                SourceProfile::Conversion => vec![
                    ("status".into(), vec![string("converted")]),
                    ("value".into(), vec![string("red")]),
                    ("diagnostics".into(), vec![]),
                ],
                SourceProfile::Constant => vec![
                    ("status".into(), vec![string("prepared")]),
                    ("value".into(), vec![string("red")]),
                    ("diagnostics".into(), vec![]),
                ],
                SourceProfile::Equality => vec![
                    ("equal".into(), vec![Item::Atomic(AtomValue::Boolean(true))]),
                    ("diagnostics".into(), vec![]),
                ],
            };
            RuleExecution::Complete(ItemStream::once(Item::native(LazyRecord {
                control: self.control.clone(),
                fields,
                reads: self.reads.clone(),
            })))
        }
    }
    impl SourceDatatypeConverter for Lazy {
        fn convert(&self, _: ConversionCall<'_>) -> RuleExecution {
            self.result(SourceProfile::Conversion)
        }
    }
    impl SourceScalarEquality for Lazy {
        fn compare(&self, _: EqualityCall<'_>) -> RuleExecution {
            self.result(SourceProfile::Equality)
        }
    }
    impl SourceConstantInterpreter for Lazy {
        fn interpret(&self, _: ConstantCall<'_>) -> RuleExecution {
            self.result(SourceProfile::Constant)
        }
    }
    for (profile, id) in source_descriptors() {
        let root = source_requirement(profile, id.clone(), None);
        let native_control = OperationControl::default();
        let reads = Arc::new(AtomicUsize::new(0));
        let callback = Lazy {
            control: native_control.clone(),
            reads: reads.clone(),
        };
        let mut native = FreshStructuralHost::new(&hash(b"registry"), native_control).unwrap();
        match profile {
            SourceProfile::Conversion => native.register_source_converter(id, callback).unwrap(),
            SourceProfile::Equality => native.register_source_equality(id, callback).unwrap(),
            SourceProfile::Constant => native
                .register_source_constant_interpreter(id, callback)
                .unwrap(),
        }
        let scope = native
            .register_scope(
                root.source().clone(),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        let binding = native
            .link_native_source_operation(root.clone(), scope, adapter_for(profile))
            .unwrap();
        let mut registry = registrations(&root, true);
        let source = datatype(root.capture(), "sample");
        binding.install(&source, &mut registry).unwrap();
        if profile == SourceProfile::Equality {
            registry
                .select_constant_interpreter(
                    source.clone(),
                    ConstantBinding::Ready(
                        cem_ql::datatype_enumeration::RegisteredConstantInterpreter::new(
                            source.clone(),
                            "constant",
                            ScalarRepresentation::String,
                            Engine::default(),
                        )
                        .unwrap(),
                    ),
                )
                .unwrap();
        }
        if profile == SourceProfile::Constant {
            registry
                .select_equality(
                    source.clone(),
                    cem_ql::datatype_enumeration::EqualityBinding::Ready(
                        cem_ql::datatype_enumeration::RegisteredScalarEquality::new(
                            source.clone(),
                            "equality",
                            ScalarRepresentation::String,
                            Engine::default(),
                        )
                        .unwrap(),
                    ),
                )
                .unwrap();
        }
        let target = if profile == SourceProfile::Conversion {
            "sample"
        } else {
            "choice"
        };
        let (c, _) = compilation(&root, &[target], &registry);
        if profile == SourceProfile::Constant {
            assert!(!c.is_ready());
        } else {
            let d = compiled(&c, target);
            assert_unavailable(
                if profile == SourceProfile::Conversion {
                    1
                } else {
                    2
                },
                &d,
            );
        }
        assert!(
            reads.load(Ordering::Acquire) > 0,
            "the fixture never reached adapter consumption"
        );
    }
}

#[test]
fn additional_availability_guards_cannot_erase_an_existing_refusal() {
    #[derive(Debug)]
    struct Available(bool);
    impl cem_ql::datatype_availability::DatatypeAvailability for Available {
        fn is_available(&self) -> bool {
            self.0
        }
    }
    let root = shared_requirement(
        OperationMode::Constant,
        "sample",
        additions().remove(0).2,
        None,
    );
    let source = datatype(root.capture(), "sample");
    let mut registry = implementations(&root);
    let profile = cem_ql::datatype_facets::RegisteredFacetProfile::new(
        source.clone(),
        "guarded",
        FacetFamily::Shipped(ShippedDatatype::String),
    )
    .unwrap()
    .with_availability(Arc::new(Available(false)))
    .with_availability(Arc::new(Available(true)));
    registry
        .select_facets(
            source,
            cem_ql::datatype_facets::FacetProfileBinding::Ready(profile),
        )
        .unwrap();
    let c = compile(&root, &["sample"], &registry);
    assert!(!c.is_ready());
    assert!(c
        .issues
        .iter()
        .any(|i| i.code == "datatype-facet-profile-unavailable"));
}
