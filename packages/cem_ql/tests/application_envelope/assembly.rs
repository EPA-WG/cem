use super::*;
use cem_ml::{
    import::import_bytes_with_lexical_scopes, parser::tree::RetainedCemTree,
    schema::reference_policy::ReferenceScopePolicy,
};
use cem_ql::structural_receiver::{
    application::{stage, ApplicationSlot, ComponentReceiverSource},
    FreshStructuralHost,
};

fn tree(text: &str) -> Arc<RetainedCemTree> {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "file:///app/receiver.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
    .tree
}
fn source(name: &str, bound: &str) -> ComponentReceiverSource {
    let tree = tree(&format!("{{schema @name=receiver @namespace=https://app.test/receiver | {{attributes | {{attribute @name={name} @type=node @maxItems={bound}}}}}}}"));
    let attribute = tree
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
    ComponentReceiverSource {
        schema_uri: "https://app.test/receiver".into(),
        source: tree,
        attributes: BTreeMap::from([(name.into(), attribute)]),
    }
}
fn envelope() -> Arc<envelope::VerifiedApplicationEnvelope> {
    let mut builder = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    let id = builder.add_contract(structural(), BTreeSet::new()).unwrap();
    builder
        .add_component(
            declaration(
                "cem-app",
                "{attribute @name=items @type=cart-lines @maxItems=2}{p | Hello}",
            ),
            BTreeMap::from([(
                "cart-lines".into(),
                NodeReceiverRequirement {
                    contract: id,
                    schema_uri: SCHEMA.into(),
                },
            )]),
        )
        .unwrap();
    let b = builder
        .build(
            BTreeSet::from(["cem-app".into()]),
            BTreeSet::new(),
            &OperationControl::default(),
        )
        .unwrap();
    Arc::new(envelope::verify(&b.bytes, &b.pin, &OperationControl::default()).unwrap())
}
fn host(source: &ComponentReceiverSource) -> FreshStructuralHost {
    let mut host =
        FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    host.register_scope(
        source.source.clone(),
        None,
        ReferenceScopePolicy::schema_defaults().unwrap(),
    )
    .unwrap();
    host
}
#[test]
fn exact_native_receiver_occurrences_stage_and_publish_without_runtime_admission() {
    let source = source("items", "2");
    let host = host(&source);
    let candidate = stage(
        &host,
        envelope(),
        BTreeMap::from([("cem-app".into(), source)]),
        BTreeMap::new(),
    )
    .unwrap();
    let mut slot = ApplicationSlot::default();
    slot.publish(&host, candidate).unwrap();
    let linked = slot.current(&host).unwrap();
    assert_eq!(
        linked
            .model(&host, "cem-app")
            .unwrap()
            .attribute_datatypes
            .len(),
        1
    );
    assert!(!linked.envelope().native_link_closure_verified());
}
#[test]
fn changed_local_cardinality_refuses_before_replacing_the_prior_owner() {
    let original = source("items", "2");
    let mut host = host(&original);
    let wrong = source("items", "64");
    register(&mut host, wrong.source.clone());
    let env = envelope();
    let candidate = stage(
        &host,
        env.clone(),
        BTreeMap::from([("cem-app".into(), original)]),
        BTreeMap::new(),
    )
    .unwrap();
    let mut slot = ApplicationSlot::default();
    slot.publish(&host, candidate).unwrap();
    assert!(stage(
        &host,
        env,
        BTreeMap::from([("cem-app".into(), wrong)]),
        BTreeMap::new()
    )
    .is_err());
    assert!(slot.current(&host).is_ok());
}

fn register(
    host: &mut FreshStructuralHost,
    tree: Arc<RetainedCemTree>,
) -> cem_ql::schema_references::DeclarationScope {
    host.register_scope(
        tree,
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    )
    .unwrap()
}
fn raw_source(text: &str) -> ComponentReceiverSource {
    let tree = tree(text);
    let attributes = tree
        .ast()
        .nodes
        .iter()
        .filter_map(|node| {
            let CemAstNode::Element {
                node_id,
                expanded_name,
                attributes,
                ..
            } = node
            else {
                return None;
            };
            if expanded_name.local_name != "attribute" {
                return None;
            }
            let name = attributes.iter().find_map(|id| match tree.ast().get(*id) {
                Some(CemAstNode::Attribute {
                    expanded_name,
                    value: Some(value),
                    ..
                }) if expanded_name.local_name == "name" => Some(value.clone()),
                _ => None,
            })?;
            Some((name, *node_id))
        })
        .collect();
    ComponentReceiverSource {
        schema_uri: "https://app.test/receiver".into(),
        source: tree,
        attributes,
    }
}
#[test]
fn source_membership_relabelled_occurrences_and_extra_sources_refuse() {
    for case in 0..6 {
        let mut original = source("items", "2");
        let mut host = host(&original);
        match case {
            0 => original.attributes.clear(),
            1 => {
                let id = original.attributes["items"];
                original.attributes.insert("extra".into(), id);
            }
            2 => {
                original = source("other", "2");
                let id = original.attributes.remove("other").unwrap();
                original.attributes.insert("items".into(), id);
                register(&mut host, original.source.clone());
            }
            3 => original.schema_uri = "https://app.test/foreign".into(),
            4 => original.source = source("items", "2").source,
            _ => {
                original=raw_source("{schema @name=receiver @namespace=https://app.test/receiver | {attributes | {attribute @name=items @type=string @maxItems=2}}}");
                register(&mut host, original.source.clone());
            }
        }
        assert!(
            stage(
                &host,
                envelope(),
                BTreeMap::from([("cem-app".into(), original)]),
                BTreeMap::new()
            )
            .is_err(),
            "case {case}"
        );
    }
    let original = source("items", "2");
    let host = host(&original);
    assert!(stage(&host, envelope(), BTreeMap::new(), BTreeMap::new()).is_err());
    assert!(stage(
        &host,
        envelope(),
        BTreeMap::from([("cem-foreign".into(), original)]),
        BTreeMap::new()
    )
    .is_err());
}
#[test]
fn multiple_components_and_occurrences_keep_separate_checked_models() {
    let mut builder = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    let id = builder.add_contract(structural(), BTreeSet::new()).unwrap();
    for tag in ["cem-app", "cem-child"] {
        builder.add_component(declaration(tag,"{attribute @name=items @type=cart-lines @maxItems=2}{attribute @name=other @type=cart-lines @minItems=1}{p | Hello}"),BTreeMap::from([("cart-lines".into(),NodeReceiverRequirement{contract:id.clone(),schema_uri:SCHEMA.into()})])).unwrap();
    }
    let b = builder
        .build(
            BTreeSet::from(["cem-app".into(), "cem-child".into()]),
            BTreeSet::new(),
            &OperationControl::default(),
        )
        .unwrap();
    let env = Arc::new(envelope::verify(&b.bytes, &b.pin, &OperationControl::default()).unwrap());
    let mut host =
        FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    let mut sources = BTreeMap::new();
    for tag in ["cem-app", "cem-child"] {
        let source=raw_source("{schema @name=receiver @namespace=https://app.test/receiver | {attributes | {attribute @name=items @type=node @maxItems=2}{attribute @name=other @type=schema:node @minItems=1}}}");
        register(&mut host, source.source.clone());
        sources.insert(tag.into(), source);
    }
    let linked = stage(&host, env, sources, BTreeMap::new()).unwrap();
    assert_eq!(
        linked
            .model(&host, "cem-app")
            .unwrap()
            .attribute_datatypes
            .len(),
        2
    );
    assert_eq!(
        linked
            .model(&host, "cem-child")
            .unwrap()
            .attribute_datatypes
            .len(),
        2
    );
    assert!(!Arc::ptr_eq(
        linked.model(&host, "cem-app").unwrap().attribute_datatypes["items"]
            .declaration()
            .document(),
        linked
            .model(&host, "cem-child")
            .unwrap()
            .attribute_datatypes["items"]
            .declaration()
            .document()
    ));
}
#[test]
fn stale_foreign_and_cancelled_staging_never_replace_the_committed_slot() {
    let original = source("items", "2");
    let host = host(&original);
    let mut slot = ApplicationSlot::default();
    slot.publish(
        &host,
        stage(
            &host,
            envelope(),
            BTreeMap::from([("cem-app".into(), original)]),
            BTreeMap::new(),
        )
        .unwrap(),
    )
    .unwrap();
    for case in 0..3 {
        let original = source("items", "2");
        let control = OperationControl::default();
        let mut fresh = FreshStructuralHost::new(&hash(b"registry"), control.clone()).unwrap();
        let scope = register(&mut fresh, original.source.clone());
        let candidate = stage(
            &fresh,
            envelope(),
            BTreeMap::from([("cem-app".into(), original)]),
            BTreeMap::new(),
        )
        .unwrap();
        match case {
            0 => assert!(fresh.set_context(scope, Some(Default::default()))),
            1 => {
                control.cancel_root(None, None).unwrap();
            }
            _ => fresh.close(),
        }
        assert!(slot.publish(&fresh, candidate).is_err());
        assert!(slot.current(&host).is_ok());
    }
    let foreign =
        FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    assert!(slot.current(&foreign).is_err());
}
#[test]
fn replacement_and_clear_retire_cloned_models_without_retiring_the_host() {
    use cem_ql::{
        api::{StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::{ItemStream, RetainedCemNode},
    };
    let original=raw_source("{schema @name=receiver @namespace=https://app.test/receiver | {elements | {element @name=view @required-attributes=items}} {attributes | {attribute @name=items @type=node @maxItems=2}}}");
    let mut host = host(&original);
    let input = tree("{view @items={#data}}");
    let data = tree("{cart @title=Tea | {item @quantity=3}}");
    let id = data
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let value = RetainedCemNode::new(data.clone(), id).unwrap().query_item();
    let from = host
        .register_scope(
            input.clone(),
            Some(StandaloneExpressionContext::default().with_binding(
                "data",
                StandaloneExpressionBinding::any(ItemStream::once(value)),
            )),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let to = register(&mut host, data);
    assert!(host.allow_scope_crossing(from, to));
    let sources = || {
        BTreeMap::from([(
            "cem-app".into(),
            ComponentReceiverSource {
                schema_uri: original.schema_uri.clone(),
                source: original.source.clone(),
                attributes: original.attributes.clone(),
            },
        )])
    };
    let mut slot = ApplicationSlot::default();
    slot.publish(
        &host,
        stage(&host, envelope(), sources(), BTreeMap::new()).unwrap(),
    )
    .unwrap();
    let before = slot
        .current(&host)
        .unwrap()
        .model(&host, "cem-app")
        .unwrap()
        .clone();
    let report = host.validate_input(input.clone(), &before).unwrap();
    assert!(
        report.complete && !report.failed,
        "{:?}",
        report.diagnostics
    );
    let abandoned = stage(&host, envelope(), sources(), BTreeMap::new()).unwrap();
    let abandoned_model = abandoned.model(&host, "cem-app").unwrap().clone();
    drop(abandoned);
    assert!(
        !host
            .validate_input(input.clone(), &abandoned_model)
            .unwrap()
            .complete
    );
    assert!(
        host.validate_input(input.clone(), &before)
            .unwrap()
            .complete
    );
    slot.publish(
        &host,
        stage(&host, envelope(), sources(), BTreeMap::new()).unwrap(),
    )
    .unwrap();
    assert!(
        !host
            .validate_input(input.clone(), &before)
            .unwrap()
            .complete
    );
    let current = slot
        .current(&host)
        .unwrap()
        .model(&host, "cem-app")
        .unwrap()
        .clone();
    assert!(
        host.validate_input(input.clone(), &current)
            .unwrap()
            .complete
    );
    slot.clear();
    assert!(!host.validate_input(input, &current).unwrap().complete);
    assert!(host.query_invocation(from).is_ok());
}

use cem_ql::{
    datatype_results::DatatypeResultAdapter, structural_receiver::application::ContractBinding,
};
fn adapter() -> DatatypeResultAdapter {
    let c = capture(
        cem_ml::schema::package_sources::builtin_schema_package_source("schema")
            .unwrap()
            .schema_source,
    );
    let src = ValueContractSource::new(
        node(&c, "schema"),
        NS,
        BTreeMap::from([("schema".into(), NS.into())]),
    )
    .with_captured_names(c)
    .unwrap();
    DatatypeResultAdapter::new(
        Arc::new(
            cem_ml::schema::value_contracts::ValueContracts::compile(&[src], Default::default())
                .unwrap(),
        ),
        ContractName::new(NS, "datatype-validation-result"),
        ContractName::new(NS, "datatype-diagnostic"),
    )
    .unwrap()
}
#[derive(Debug)]
struct NoPreparation;
impl cem_ql::datatype_preparation::NativeLexicalPreparer for NoPreparation {
    fn prepare(
        &self,
        _: cem_ql::datatype_preparation::PreparationCall<'_>,
    ) -> cem_ql::datatype_preparation::PreparationExecution {
        panic!("assembly must never invoke native code")
    }
}
fn dependencies() -> (Arc<envelope::VerifiedApplicationEnvelope>, String, String) {
    let mut b = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    let native = b.add_contract(native("prepare"), BTreeSet::new()).unwrap();
    let query = b
        .add_contract(query(), BTreeSet::from([native.clone()]))
        .unwrap();
    b.add_component(declaration("cem-app", "{p | Hello}"), BTreeMap::new())
        .unwrap();
    let b = b
        .build(
            BTreeSet::from(["cem-app".into()]),
            BTreeSet::from([query.clone()]),
            &OperationControl::default(),
        )
        .unwrap();
    (
        Arc::new(envelope::verify(&b.bytes, &b.pin, &OperationControl::default()).unwrap()),
        native,
        query,
    )
}
#[test]
fn exact_query_and_native_dependencies_link_without_callback_execution() {
    use cem_ql::{
        datatype_validation::{ValidationInput, ValidationLimits},
        eval::{AtomValue, Item},
        selected_registry::SelectedFunctionRegistry,
    };
    let (env, native, query) = dependencies();
    let mut host =
        FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    host.register_native_preparer(
        env.native(&native).unwrap().implementation().clone(),
        NoPreparation,
    )
    .unwrap();
    let native_scope = register(&mut host, env.native(&native).unwrap().source().clone());
    let query_scope = register(&mut host, env.query(&query).unwrap().source().clone());
    let datatype = env
        .query(&query)
        .unwrap()
        .source()
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                expanded_name,
                node_id,
                ..
            } if expanded_name.local_name == "type" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let linked = stage(
        &host,
        env,
        BTreeMap::new(),
        BTreeMap::from([
            (
                native.clone(),
                ContractBinding::NativeOperation {
                    scope: native_scope,
                },
            ),
            (
                query.clone(),
                ContractBinding::Query {
                    scope: query_scope,
                    datatype,
                    functions: SelectedFunctionRegistry::new([]).unwrap(),
                    adapter: adapter(),
                },
            ),
        ]),
    )
    .unwrap();
    assert!(linked.operation(&host, &native).is_ok());
    let input = ValidationInput {
        value: vec![Item::Atomic(AtomValue::String("hello".into()))],
        candidate: vec![],
        fallback: Default::default(),
    };
    let result = host
        .validate_query(
            &host.query_invocation(query_scope).unwrap(),
            linked.query(&host, &query).unwrap(),
            &input,
            ValidationLimits::default(),
        )
        .unwrap();
    assert_eq!(result.accepted, Some(true));
    linked.operation(&host, &native).unwrap().retire();
    assert!(linked.query(&host, &query).is_err());
}
#[test]
fn missing_native_implementations_and_wrong_dependency_roles_refuse() {
    for case in 0..5 {
        let (env, native, query) = dependencies();
        let mut host =
            FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
        if case != 0 {
            let mut identity = env.native(&native).unwrap().implementation().clone();
            if case == 1 {
                identity.module_hash = hash(b"wrong native module");
            }
            host.register_native_preparer(identity, NoPreparation)
                .unwrap();
        }
        let actual_scope = register(&mut host, env.native(&native).unwrap().source().clone());
        let mut foreign =
            FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
        let foreign_scope = register(&mut foreign, env.native(&native).unwrap().source().clone());
        let scope = if case == 4 {
            foreign_scope
        } else {
            actual_scope
        };
        let query_scope = register(&mut host, env.query(&query).unwrap().source().clone());
        let datatype = env
            .query(&query)
            .unwrap()
            .source()
            .ast()
            .nodes
            .iter()
            .find_map(|node| match node {
                CemAstNode::Element {
                    expanded_name,
                    node_id,
                    ..
                } if expanded_name.local_name == "type" => Some(*node_id),
                _ => None,
            })
            .unwrap();
        let request = if case == 2 {
            ContractBinding::NativeTokenizer { scope }
        } else {
            ContractBinding::NativeOperation { scope }
        };
        let mut bindings = BTreeMap::from([(
            query,
            ContractBinding::Query {
                scope: query_scope,
                datatype,
                functions: cem_ql::selected_registry::SelectedFunctionRegistry::new([]).unwrap(),
                adapter: adapter(),
            },
        )]);
        if case != 3 {
            bindings.insert(native, request);
        }
        assert!(
            stage(&host, env, BTreeMap::new(), bindings).is_err(),
            "case {case}"
        );
    }
}
#[test]
fn source_and_occurrence_counts_are_bounded_before_linking() {
    let mut source = source("items", "2");
    let host = host(&source);
    let id = source.attributes["items"];
    for i in 0..65 {
        source.attributes.insert(format!("field{i}"), id);
    }
    assert!(stage(
        &host,
        envelope(),
        BTreeMap::from([("cem-app".into(), source)]),
        BTreeMap::new()
    )
    .is_err());
}

#[test]
fn nested_or_duplicate_compiled_receiver_occurrences_refuse() {
    for body in [
        "{section | {attribute @name=items @type=cart-lines @maxItems=2}}",
        "{attribute @name=items @type=cart-lines @maxItems=2}{attribute @name=items @type=cart-lines @maxItems=2}",
        "{attribute @name=items @type=cart-lines @maxItems=2 @default=empty}",
    ] {
        let mut builder=EnvelopeBuilder::new(&hash(b"registry")).unwrap();
        let id=builder.add_contract(structural(),BTreeSet::new()).unwrap();
        builder.add_component(declaration("cem-app",body),BTreeMap::from([("cart-lines".into(),NodeReceiverRequirement{contract:id,schema_uri:SCHEMA.into()})])).unwrap();
        let b=builder.build(BTreeSet::from(["cem-app".into()]),BTreeSet::new(),&OperationControl::default()).unwrap();
        let env=Arc::new(envelope::verify(&b.bytes,&b.pin,&OperationControl::default()).unwrap());
        let source=source("items","2");
        let host=host(&source);
        assert!(stage(&host,env,BTreeMap::from([("cem-app".into(),source)]),BTreeMap::new()).is_err());
    }
}

#[test]
fn retired_staged_structural_models_cannot_replace_a_live_prior_owner() {
    let original = source("items", "2");
    let host = host(&original);
    let sources = || {
        BTreeMap::from([(
            "cem-app".into(),
            ComponentReceiverSource {
                schema_uri: original.schema_uri.clone(),
                source: original.source.clone(),
                attributes: original.attributes.clone(),
            },
        )])
    };
    let mut slot = ApplicationSlot::default();
    slot.publish(
        &host,
        stage(&host, envelope(), sources(), BTreeMap::new()).unwrap(),
    )
    .unwrap();
    let candidate = stage(&host, envelope(), sources(), BTreeMap::new()).unwrap();
    candidate
        .model(&host, "cem-app")
        .unwrap()
        .attribute_datatypes["items"]
        .retire_preparations();
    assert!(candidate.model(&host, "cem-app").is_err());
    assert!(slot.publish(&host, candidate).is_err());
    assert!(slot.current(&host).is_ok());
}
