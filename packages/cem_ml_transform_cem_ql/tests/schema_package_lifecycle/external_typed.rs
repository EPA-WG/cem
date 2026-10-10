use super::*;
use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    schema::{attribute_datatypes::*, document_model::attribute_facets::FacetFamily},
};
use cem_ql::{
    datatype_validation::ValidationRuntime,
    eval::{AtomValue, Item},
    external_typed::ExternalTypedProducer,
};
use std::collections::BTreeMap;

fn compiler(ready: bool) -> CemQlSchemaPackageCompiler {
    CemQlSchemaPackageCompiler::new(|request| {
        let policy = ReferenceScopePolicy::schema_defaults().unwrap();
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(request.source.clone(), Some(Default::default()), policy.clone());
        host.attach_captured_names(&request.lexical_scopes).unwrap();
        Ok((host, policy.limits))
    }).with_datatype_discovery(Default::default(), |request, _| {
        let schema = request.source.ast().nodes.iter().find_map(|n| match n {
            CemAstNode::Element { node_id, expanded_name, .. } if expanded_name.local_name == "schema" => SchemaDeclarationNode::new(request.source.ast_owner().clone(), *node_id), _ => None,
        }).unwrap();
        Ok(vec![cem_ql::datatype_names::DatatypeSchemaSource { schema, captured: request.lexical_scopes.clone(), imports: vec![] }])
    }, move |request, host, sources, limits| {
        let mut regs = DatatypeImplementations::default();
        for source in sources {
            let list = source.attribute("kind").is_some_and(|field| matches!(field.node(), CemAstNode::Attribute { value: Some(name), .. } if name == "list"));
            let family = if list { FacetFamily::TypedList(ScalarRepresentation::Integer) } else { FacetFamily::TypedScalar(ScalarRepresentation::Integer) };
            regs.register(DatatypeImplementation { source: source.clone(), kind: if list { DatatypeKind::List } else { DatatypeKind::Scalar }, representation: family.representation(), accepted_bases: vec![], bounds: Default::default(), tokenizer: TokenizerBinding::Absent, validator: None }).unwrap();
            regs.select_facets(source.clone(), if ready { FacetProfileBinding::Ready(RegisteredFacetProfile::new(source.clone(), "explicit-typed-only", family).unwrap()) } else { FacetProfileBinding::Unavailable }).unwrap();
        }
        Ok(compile_datatypes(request.source.ast_owner().clone(), sources, host, &regs, &Default::default(), limits))
    })
}
fn authored_typed(list: bool) -> String {
    let schema = authored("integer", "").replace(
        "@maxInclusive=4",
        if list { "@minItems=0 @maxItems=2" } else { "" },
    );
    if list {
        schema.replace(
            "{type @name=integer @kind=scalar}",
            "{type @name=item @kind=scalar} {type @name=integer @kind=list @base=item}",
        )
    } else {
        schema
    }
}

#[test]
fn external_typed_core_handoff_checks_issuer_context_owner_and_each_current_invocation() {
    for list in [false, true] {
        let mut context = context(&authored_typed(list));
        context.schema_package_compiler = Some(Arc::new(compiler(true)));
        load(&mut context, &input());
        let active = context
            .schema_document_models
            .resolve_for_identity(Some(SCHEMA_URI), None, None)
            .unwrap();
        let contract = &active.attribute_datatypes["count"];
        let admission = contract.typed_admission().unwrap();
        let producer = ExternalTypedProducer::from_native(&admission, "host:computed").unwrap();
        assert!(ExternalTypedProducer::from_native(
            &NativeAttributeTypedAdmission::new("forged"),
            "host:computed"
        )
        .is_err());
        let control = OperationControl::default();
        let runtime = ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        };
        let original = tree("{sample @count=unrelated-spelling}");
        let source = original.ast().nodes.iter().find(|n| matches!(n, CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name == "count")).unwrap();
        let attrs = BTreeMap::new();
        let mut epoch = AttributeDatatypeContext::default();
        macro_rules! args {
            ($value:expr, $epoch:expr) => {
                AttributeDatatypeInput {
                    value: $value,
                    source,
                    source_tree: Some(original.clone()),
                    element_name: "sample",
                    attribute_values: &attrs,
                    control: &control,
                    context: Some($epoch),
                }
            };
        }
        assert!(contract
            .prepare(args!(
                AttributeDatatypeValue::Lexical("unrelated-spelling"),
                &epoch
            ))
            .handle
            .is_none());
        assert!(contract
            .validate(args!(AttributeDatatypeValue::Lexical("3"), &epoch))
            .accepted
            .is_none());
        let fake = NativeAttributeTypedValue::new(vec![Item::Atomic(AtomValue::Integer(3))]);
        assert!(contract
            .validate(args!(AttributeDatatypeValue::ExternalTyped(&fake), &epoch))
            .accepted
            .is_none());
        let counts: &[usize] = if list { &[0, 2, 3] } else { &[1] };
        for count in counts {
            let data = producer
                .produce(
                    Some(vec![Item::Atomic(AtomValue::Integer(3)); *count]),
                    &runtime,
                    Default::default(),
                )
                .unwrap();
            let native = data.native_handle();
            for _ in 0..2 {
                assert_eq!(
                    contract
                        .validate(args!(
                            AttributeDatatypeValue::ExternalTyped(&native),
                            &epoch
                        ))
                        .accepted,
                    Some(*count <= 2)
                );
            }
            let mut missing = args!(AttributeDatatypeValue::ExternalTyped(&native), &epoch);
            missing.context = None;
            assert!(contract.validate(missing).accepted.is_none());
            let mut foreign = args!(AttributeDatatypeValue::ExternalTyped(&native), &epoch);
            foreign.source_tree = Some(tree("{sample @count=unrelated-spelling}"));
            assert!(contract.validate(foreign).accepted.is_none());
        }
        let value = producer
            .produce(
                Some(vec![Item::Atomic(AtomValue::Integer(3))]),
                &runtime,
                Default::default(),
            )
            .unwrap();
        let native = value.native_handle();
        let fake_preparation = NativeAttributePreparation::new(value.clone());
        assert!(contract
            .validate(args!(
                AttributeDatatypeValue::Prepared(&fake_preparation),
                &epoch
            ))
            .accepted
            .is_none());
        epoch.close();
        assert!(contract
            .validate(args!(
                AttributeDatatypeValue::ExternalTyped(&native),
                &epoch
            ))
            .accepted
            .is_none());
        epoch.advance();
        assert_eq!(
            contract
                .validate(args!(
                    AttributeDatatypeValue::ExternalTyped(&native),
                    &epoch
                ))
                .accepted,
            Some(true),
            "immutable data can enter a fresh live context; every check reruns"
        );
        control.complete_scope(ROOT_EXECUTION_SCOPE_ID).unwrap();
        assert!(contract
            .validate(args!(
                AttributeDatatypeValue::ExternalTyped(&native),
                &epoch
            ))
            .accepted
            .is_none());
    }
}

#[test]
fn typed_only_publication_rejects_defaults_and_expires_external_admission_on_commit() {
    let authored = authored_typed(false);
    let mut context = context(&authored);
    context.schema_package_compiler = Some(Arc::new(compiler(true)));
    load(&mut context, &input());
    let old = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .clone();
    let contract = &old.attribute_datatypes["count"];
    let admission = contract.typed_admission().unwrap();
    let producer = ExternalTypedProducer::from_native(&admission, "typed").unwrap();
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let values = || Some(vec![Item::Atomic(AtomValue::Integer(3))]);
    let value = producer
        .produce(values(), &runtime, Default::default())
        .unwrap()
        .native_handle();
    let original = tree("{sample @count=ignored}");
    let source = original.ast().nodes.iter().find(|n| matches!(n, CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name == "count")).unwrap();
    let attrs = BTreeMap::new();
    let epoch = AttributeDatatypeContext::default();
    let args = || AttributeDatatypeInput {
        value: AttributeDatatypeValue::ExternalTyped(&value),
        source,
        source_tree: Some(original.clone()),
        element_name: "sample",
        attribute_values: &attrs,
        control: &control,
        context: Some(&epoch),
    };
    for mode in ["default", "lexical-field", "unavailable"] {
        set_source(
            &mut context,
            &authored.replace(
                "@type=integer",
                match mode {
                    "default" => "@type=integer @default=3",
                    "lexical-field" => "@type=integer @minInclusive=0",
                    _ => "@type=integer @type-diagnostic=\"\"",
                },
            ),
        );
        context.schema_package_compiler = Some(Arc::new(compiler(mode != "unavailable")));
        load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
        assert!(Arc::ptr_eq(
            old.datatype_compilation.as_ref().unwrap(),
            context
                .schema_document_models
                .resolve_for_identity(Some(SCHEMA_URI), None, None)
                .unwrap()
                .datatype_compilation
                .as_ref()
                .unwrap()
        ));
        assert_eq!(contract.validate(args()).accepted, Some(true));
        assert!(producer
            .produce(values(), &runtime, Default::default())
            .is_ok());
    }
    set_source(
        &mut context,
        &authored.replace("@name=runtime", "@name=replacement"),
    );
    context.schema_package_compiler = Some(Arc::new(compiler(true)));
    load(&mut context, &input());
    assert!(contract.typed_admission().is_none());
    assert!(producer
        .produce(values(), &runtime, Default::default())
        .is_err());
    assert!(ExternalTypedProducer::from_native(&admission, "same-name").is_err());
    assert!(contract.validate(args()).accepted.is_none());
    let new = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap();
    assert!(new.attribute_datatypes["count"]
        .validate(args())
        .accepted
        .is_none());
}
