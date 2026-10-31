use super::selection::{limits, prepared, verified};
use super::*;
use cem_ml::schema::{
    datatype_contracts::ItemBounds,
    datatype_registry::{DatatypeKind, DatatypeKindSource},
    datatype_validation::{ScalarRepresentation, ValueRepresentation},
};
use cem_ql::{
    datatype_compilation::{DatatypeImplementation, TokenizerBinding},
    datatype_validation::{ValidationInput, ValidationLimits},
    eval::{AtomValue, Item},
    structural_receiver::FreshStructuralHost,
};
fn implementations(c: &capsule::VerifiedDatatypeDependencies) -> Vec<DatatypeImplementation> {
    let mut out = vec![];
    let mut sources = c.sources().values().collect::<Vec<_>>();
    for id in c.dependency_ids() {
        if let Some(s) = c.dependency_datatypes(id) {
            sources.extend(s.values());
        }
    }
    for source in sources {
        let kind = match source.plan().kind {
            DatatypeKindSource::Explicit(kind) => kind,
            _ => continue,
        };
        out.push(DatatypeImplementation {
            source: source.clone(),
            kind,
            representation: if kind == DatatypeKind::List {
                ValueRepresentation::List(ScalarRepresentation::String)
            } else {
                ValueRepresentation::Scalar(ScalarRepresentation::String)
            },
            accepted_bases: vec![],
            bounds: ItemBounds::default(),
            tokenizer: TokenizerBinding::Absent,
            validator: None,
        });
    }
    out
}
fn values(count: usize) -> ValidationInput {
    ValidationInput {
        value: vec![Item::Atomic(AtomValue::String("Tea".into())); count],
        candidate: vec![],
        fallback: Default::default(),
    }
}
#[test]
fn actual_host_compilation_preserves_imported_inheritance_root_bounds_and_ownership() {
    let text = SOURCE.replace(
        "@name=root @list-base=words",
        "@name=root @list-base=words @max-items=2",
    );
    let c = verified(&text);
    let (mut h, root, scopes, imports) = prepared(&c, 1, true);
    let impls = implementations(&c);
    let compiled = h
        .compile_datatype_dependencies(c.clone(), root, &scopes, &imports, &impls, limits())
        .unwrap();
    let id = declaration(c.capture(), "root");
    assert_eq!(
        compiled
            .validate(&h, id, &values(2), ValidationLimits::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    let rejected = compiled
        .validate(&h, id, &values(3), ValidationLimits::default())
        .unwrap();
    assert_eq!(rejected.accepted, Some(false));
    assert!(Arc::ptr_eq(
        rejected.cardinality[0].source.document(),
        c.capture().document()
    ));
    assert!(compiled
        .validate(
            &h,
            declaration(c.capture(), "words"),
            &values(1),
            ValidationLimits::default()
        )
        .is_err());
    assert!(!c.executable_closure_verified());
}
#[test]
fn missing_implementations_grants_and_stale_contexts_cannot_compile_or_reuse_a_descriptor() {
    let c = verified(SOURCE);
    let (mut h, root, scopes, imports) = prepared(&c, 1, true);
    let impls = implementations(&c);
    assert!(h
        .compile_datatype_dependencies(c.clone(), root, &scopes, &imports, &[], limits())
        .is_err());
    let compiled = h
        .compile_datatype_dependencies(c.clone(), root, &scopes, &imports, &impls, limits())
        .unwrap();
    assert!(compiled.is_live(&h));
    assert!(h
        .compile_datatype_dependencies(
            c.clone(),
            root,
            &BTreeMap::new(),
            &imports,
            &impls,
            limits()
        )
        .is_err());
    assert!(compiled.is_live(&h));
    h.set_context(root, None);
    assert!(!compiled.is_live(&h));
    assert!(compiled
        .validate(
            &h,
            declaration(c.capture(), "root"),
            &values(1),
            Default::default()
        )
        .is_err());
    let (mut denied, root, scopes, imports) = prepared(&c, 0, true);
    assert!(denied
        .compile_datatype_dependencies(c, root, &scopes, &imports, &impls, limits())
        .is_err());
}
#[test]
fn primitive_metadata_cannot_smuggle_foreign_sources_or_executable_hooks() {
    let c = verified(SOURCE);
    let (mut h, root, scopes, imports) = prepared(&c, 1, true);
    for case in 0..4 {
        let mut impls = implementations(&c);
        match case {
            0 => {
                impls[0].source = implementations(&verified(SOURCE))[0].source.clone();
            }
            1 => {
                impls[0].tokenizer = TokenizerBinding::Ready(
                    cem_ml::schema::datatype_contracts::RegisteredTokenizer::whitespace(),
                );
            }
            2 => {
                impls[0].kind = DatatypeKind::Node;
                impls[0].representation = ValueRepresentation::Nodes;
            }
            _ => {
                impls[0].validator = Some((
                    impls[0].source.scope().clone(),
                    impls[0].source.declaration().clone(),
                ));
            }
        }
        let Err(error) =
            h.compile_datatype_dependencies(c.clone(), root, &scopes, &imports, &impls, limits())
        else {
            panic!("unadmitted primitive metadata compiled")
        };
        assert_eq!(
            error.code, "datatype-dependency-primitive-metadata-required",
            "case {case}"
        );
    }
}
#[test]
fn imported_compiler_issues_keep_the_original_module_arena() {
    let vendor = VENDOR.replace("@kind=scalar", "@kind=scalar @min-items=5 @max-items=2");
    let c = super::selection::verified_vendor(SOURCE, &vendor, "base");
    let (mut h, root, scopes, imports) = prepared(&c, 1, true);
    let Err(error) = h.compile_datatype_dependencies(
        c.clone(),
        root,
        &scopes,
        &imports,
        &implementations(&c),
        limits(),
    ) else {
        panic!("invalid imported bounds compiled")
    };
    let id = c.dependency_ids().next().unwrap();
    assert!(error.issues.iter().any(|i| Arc::ptr_eq(
        i.source.document(),
        c.dependency_capture(id).unwrap().document()
    )));
    assert!(error
        .issues
        .iter()
        .all(|i| !Arc::ptr_eq(i.source.document(), c.capture().document())));
}
#[test]
fn root_validation_keeps_shared_type_and_input_budget_checks_and_private_lifetime() {
    let c = verified(SOURCE);
    let (mut h, root, scopes, imports) = prepared(&c, 1, true);
    let compiled = h
        .compile_datatype_dependencies(
            c.clone(),
            root,
            &scopes,
            &imports,
            &implementations(&c),
            limits(),
        )
        .unwrap();
    let id = declaration(c.capture(), "root");
    let mut input = values(1);
    input.value[0] = Item::Atomic(AtomValue::Integer(3));
    assert_eq!(
        compiled
            .validate(&h, id, &input, Default::default())
            .unwrap()
            .accepted,
        None
    );
    let mut low = ValidationLimits::default();
    low.max_input_values = 0;
    assert_eq!(
        compiled.validate(&h, id, &values(1), low).unwrap().accepted,
        None
    );
    let (other, _, _, _) = prepared(&c, 1, true);
    assert!(!compiled.is_live(&other));
    compiled.retire();
    assert!(!compiled.is_live(&h));
    assert!(compiled
        .validate(&h, id, &values(1), Default::default())
        .is_err());
    let compiled = h
        .compile_datatype_dependencies(
            c.clone(),
            root,
            &scopes,
            &imports,
            &implementations(&c),
            limits(),
        )
        .unwrap();
    h.close();
    assert!(!compiled.is_live(&h));
}
#[test]
fn imported_rule_pins_do_not_activate_unbound_query_or_native_implementations() {
    use cem_ml::schema::reference_policy::{ReferenceScopePolicy, ReferenceScopePolicyOverrides};
    use cem_ql::{
        api::{StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::{ItemStream, RetainedCemNode},
    };
    for query in [false, true] {
        let text="@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:consumer | {types | {type @name=root @kind=scalar @rule={#selected}}}}";
        let capture = capture(text);
        let (target, artifact) = rule_artifact(query);
        let id = artifact.id().to_owned();
        let artifacts = [artifact];
        let b = capsule::export(
            &capture,
            identity(text, "file:///app/rule-consumer.cem"),
            &hash(b"registry"),
            BTreeSet::from([declaration(&capture, "root")]),
            BTreeMap::from([(field(&capture, "root", "rule"), target)]),
            &artifacts,
            &OperationControl::default(),
        )
        .unwrap();
        let c = Arc::new(
            capsule::verify(&b.bytes, &b.pin, &artifacts, &OperationControl::default()).unwrap(),
        );
        let tree = c.dependency_source(&id).unwrap();
        let behavior = tree
            .ast()
            .nodes
            .iter()
            .find_map(|n| match n {
                CemAstNode::Element {
                    node_id,
                    expanded_name,
                    ..
                } if expanded_name.local_name == "behavior" => Some(*node_id),
                _ => None,
            })
            .unwrap();
        let ctx = StandaloneExpressionContext::default().with_binding(
            "selected",
            StandaloneExpressionBinding::any(ItemStream::once(
                RetainedCemNode::new(tree.clone(), behavior)
                    .unwrap()
                    .query_item(),
            )),
        );
        let mut h =
            FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
        let root = h
            .register_dependency_scope(
                &c,
                None,
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        let module = h
            .register_dependency_scope(
                &c,
                Some(&id),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        h.prepare_dependency_lexical_scopes(&c, None, |_, _, _| {
            (Some(ctx.clone()), ReferenceScopePolicyOverrides::default())
        })
        .unwrap();
        h.allow_scope_crossing(root, module);
        let scopes = BTreeMap::from([(id, module)]);
        assert!(h
            .select_datatype_dependencies(c.clone(), root, &scopes, &[], limits())
            .is_ok());
        let Err(error) = h.compile_datatype_dependencies(
            c.clone(),
            root,
            &scopes,
            &[],
            &implementations(&c),
            limits(),
        ) else {
            panic!("unbound rule became executable")
        };
        assert_eq!(error.code, "datatype-dependency-rule-bindings-unavailable");
    }
}
#[test]
fn compilation_reselects_mutable_native_targets_and_cancellation_never_publishes() {
    use cem_ml::schema::reference_policy::ReferenceScopePolicy;
    use cem_ql::{
        api::StandaloneExpressionBinding,
        eval::{ItemStream, RetainedCemNode},
        native::{NativeQueryFunction, NativeQueryRequest},
    };
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    #[derive(Debug)]
    struct Pick {
        bad: Arc<AtomicBool>,
        stop: Arc<AtomicBool>,
        calls: Arc<AtomicUsize>,
        _marker: Arc<()>,
    }
    impl NativeQueryFunction for Pick {
        fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.stop.load(Ordering::SeqCst) {
                request.control.cancel_root(None, None).unwrap();
            }
            request.arguments[usize::from(self.bad.load(Ordering::SeqCst))].clone()
        }
    }
    let text = SOURCE.replace("#selected", r#"#native:call("fixture.pick", good, wrong)"#);
    let c = verified(&text);
    let impls = implementations(&c);
    let module = c.dependency_ids().next().unwrap();
    let actual = RetainedCemNode::new(c.source().clone(), declaration(c.capture(), "imported"))
        .unwrap()
        .query_item();
    let foreign = RetainedCemNode::new(
        c.dependency_source(module).unwrap().clone(),
        declaration(c.dependency_capture(module).unwrap(), "base"),
    )
    .unwrap()
    .query_item();
    let mut ctx = super::selection::context(&c, "imported");
    ctx.bindings.insert(
        "good".into(),
        StandaloneExpressionBinding::any(ItemStream::once(actual)),
    );
    ctx.bindings.insert(
        "wrong".into(),
        StandaloneExpressionBinding::any(ItemStream::once(foreign)),
    );
    let bad = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));
    let calls = Arc::new(AtomicUsize::new(0));
    let marker = Arc::new(());
    let weak = Arc::downgrade(&marker);
    ctx.native_functions
        .register(
            "fixture.pick",
            2,
            Pick {
                bad: bad.clone(),
                stop: stop.clone(),
                calls: calls.clone(),
                _marker: marker.clone(),
            },
        )
        .unwrap();
    drop(marker);
    let ((mut h, root, scopes, imports), _) = super::selection::configured(
        &c,
        1,
        true,
        Some(ctx),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    let inspection = h
        .select_datatype_dependencies(c.clone(), root, &scopes, &imports, limits())
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    bad.store(true, Ordering::SeqCst);
    let Err(error) =
        h.compile_datatype_dependencies(c.clone(), root, &scopes, &imports, &impls, limits())
    else {
        panic!("historical inspection authorized a changed target")
    };
    assert!(error
        .issues
        .iter()
        .any(|i| i.code == "invalid-datatype-dependency-target"));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(inspection.is_live(&h));
    bad.store(false, Ordering::SeqCst);
    let compiled = h
        .compile_datatype_dependencies(c.clone(), root, &scopes, &imports, &impls, limits())
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(
        compiled
            .validate(
                &h,
                declaration(c.capture(), "root"),
                &values(1),
                Default::default()
            )
            .unwrap()
            .accepted,
        Some(true)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    stop.store(true, Ordering::SeqCst);
    assert!(h
        .compile_datatype_dependencies(c, root, &scopes, &imports, &impls, limits())
        .is_err());
    assert!(!compiled.is_live(&h));
    drop(h);
    assert!(
        weak.upgrade().is_none(),
        "compiled metadata kept host selector callbacks alive"
    );
}

#[test]
fn compiler_request_limits_refuse_without_invalidating_prior_private_snapshots() {
    let c = verified(SOURCE);
    let (mut h, root, scopes, imports) = prepared(&c, 1, true);
    let impls = implementations(&c);
    let compiled = h
        .compile_datatype_dependencies(c.clone(), root, &scopes, &imports, &impls, limits())
        .unwrap();
    for (work, depth) in [
        (0, 64),
        (100_001, 64),
        (100_000, 0),
        (100_000, 65),
        (1, 64),
        (100_000, 1),
    ] {
        let mut low = limits();
        low.max_work = work;
        low.max_depth = depth;
        assert!(
            h.compile_datatype_dependencies(c.clone(), root, &scopes, &imports, &impls, low)
                .is_err(),
            "work={work}, depth={depth}"
        );
        assert!(compiled.is_live(&h));
    }
    assert_eq!(
        compiled
            .validate(
                &h,
                declaration(c.capture(), "root"),
                &values(1),
                Default::default()
            )
            .unwrap()
            .accepted,
        Some(true)
    );
}
