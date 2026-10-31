use super::*;
use cem_ml::schema::{
    reference_policy::{ReferenceScopePolicy, ReferenceScopePolicyOverrides},
    reference_traversal::ReferenceTraversalLimits,
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    schema_references::DeclarationScope,
    structural_receiver::{DatatypeDependencyImport, FreshStructuralHost},
};
pub(super) fn verified(text: &str) -> Arc<capsule::VerifiedDatatypeDependencies> {
    verified_vendor(text, VENDOR, "base")
}
pub(super) fn verified_vendor(
    text: &str,
    vendor: &str,
    exported: &str,
) -> Arc<capsule::VerifiedDatatypeDependencies> {
    let c = capture(text);
    let v = capture(vendor);
    let root = declaration(&v, exported);
    let module = dt::export(
        &v,
        identity(vendor, "file:///app/vendor.cem"),
        &hash(b"registry"),
        BTreeSet::from([root]),
        &OperationControl::default(),
    )
    .unwrap();
    let bindings = BTreeMap::from([
        (
            field(&c, "imported", "base"),
            capsule::DependencyTarget::Datatype {
                pin: module.pin.clone(),
                root,
            },
        ),
        (
            field(&c, "words", "base"),
            capsule::DependencyTarget::LocalDatatype(declaration(&c, "imported")),
        ),
        (
            field(&c, "root", "list-base"),
            capsule::DependencyTarget::LocalDatatype(declaration(&c, "words")),
        ),
    ]);
    let artifacts = [capsule::DependencyArtifact::datatype(module)];
    let b = capsule::export(
        &c,
        identity(text, "file:///app/main.cem"),
        &hash(b"registry"),
        BTreeSet::from([declaration(&c, "root")]),
        bindings,
        &artifacts,
        &OperationControl::default(),
    )
    .unwrap();
    Arc::new(capsule::verify(&b.bytes, &b.pin, &artifacts, &OperationControl::default()).unwrap())
}

pub(super) fn context(
    c: &capsule::VerifiedDatatypeDependencies,
    name: &str,
) -> StandaloneExpressionContext {
    let item = RetainedCemNode::new(c.source().clone(), declaration(c.capture(), name))
        .unwrap()
        .query_item();
    StandaloneExpressionContext::default().with_binding(
        "selected",
        StandaloneExpressionBinding::any(ItemStream::once(item)),
    )
}
pub(super) fn prepared(
    c: &Arc<capsule::VerifiedDatatypeDependencies>,
    grant: u8,
    ready: bool,
) -> (
    FreshStructuralHost,
    DeclarationScope,
    BTreeMap<String, DeclarationScope>,
    Vec<DatatypeDependencyImport>,
) {
    configured(
        c,
        grant,
        ready,
        Some(context(c, "imported")),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    )
    .0
}
pub(super) fn configured(
    c: &Arc<capsule::VerifiedDatatypeDependencies>,
    grant: u8,
    ready: bool,
    ctx: Option<StandaloneExpressionContext>,
    destination: ReferenceScopePolicy,
) -> (
    (
        FreshStructuralHost,
        DeclarationScope,
        BTreeMap<String, DeclarationScope>,
        Vec<DatatypeDependencyImport>,
    ),
    Vec<(u32, DeclarationScope)>,
) {
    let mut host =
        FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    let root = host
        .register_dependency_scope(
            c,
            None,
            Some(context(c, "local")),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let id = c.dependency_ids().next().unwrap().to_owned();
    let target = host
        .register_dependency_scope(
            c,
            Some(&id),
            ready.then(StandaloneExpressionContext::default),
            destination,
        )
        .unwrap();
    let lexical = host
        .prepare_dependency_lexical_scopes(c, None, |_, saved, _| {
            assert!(saved.namespaces.binding("s").is_some());
            (ctx.clone(), ReferenceScopePolicyOverrides::default())
        })
        .unwrap();
    if grant == 1 {
        assert!(host.allow_scope_crossing(root, target));
    }
    if grant == 2 {
        assert!(host.allow_scope_crossing(target, root));
    }
    let import = DatatypeDependencyImport {
        consumer: root,
        module: id.clone(),
        root: c
            .sites()
            .iter()
            .find_map(|site| match &site.target {
                capsule::DependencyTarget::Datatype { root, .. } => Some(*root),
                _ => None,
            })
            .unwrap(),
    };
    (
        (host, root, BTreeMap::from([(id, target)]), vec![import]),
        lexical,
    )
}
pub(super) fn limits() -> ReferenceTraversalLimits {
    ReferenceTraversalLimits {
        max_depth: 32,
        max_work: 100_000,
    }
}
#[test]
fn selections_use_original_captured_names_and_actual_lexical_contexts() {
    let c = verified(SOURCE);
    let (mut h, root, scopes, imports) = prepared(&c, 1, true);
    let result = h
        .select_datatype_dependencies(c.clone(), root, &scopes, &imports, limits())
        .unwrap();
    assert_eq!(result.site_count(&h).unwrap(), 3);
    assert_eq!(result.datatype_count(&h).unwrap(), 4);
    assert!(result.is_live(&h));
    assert!(!c.selections_verified());
    assert!(!c.executable_closure_verified());
    assert!(c
        .capture()
        .document()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}
#[test]
fn missing_reverse_grants_pending_contexts_and_unexported_names_refuse() {
    for case in 0..7 {
        let c = verified(SOURCE);
        let (mut h, root, scopes, mut imports) = prepared(
            &c,
            if case == 0 {
                0
            } else if case == 1 {
                2
            } else {
                1
            },
            case != 2,
        );
        if case == 3 {
            imports.clear();
        }
        if case == 4 {
            imports[0].root = u32::MAX;
        }
        if case == 5 {
            imports.push(imports[0].clone());
        }
        if case == 6 {
            imports[0].module = hash(b"unknown module");
        }
        assert!(
            h.select_datatype_dependencies(c, root, &scopes, &imports, limits())
                .is_err(),
            "case {case}"
        );
    }
}
#[test]
fn namespace_mismatch_cannot_be_repaired_with_the_consumers_asserted_target() {
    let c = verified(&SOURCE.replace("urn:vendor", "urn:wrong"));
    let (mut h, root, scopes, imports) = prepared(&c, 1, true);
    assert!(h
        .select_datatype_dependencies(c, root, &scopes, &imports, limits())
        .is_err());
}
#[test]
fn failed_binding_preserves_prior_selection_but_context_changes_retire_it() {
    let c = verified(SOURCE);
    let (mut h, root, scopes, imports) = prepared(&c, 1, true);
    let selected = h
        .select_datatype_dependencies(c.clone(), root, &scopes, &imports, limits())
        .unwrap();
    assert!(h
        .select_datatype_dependencies(c.clone(), root, &BTreeMap::new(), &imports, limits())
        .is_err());
    assert!(selected.is_live(&h));
    assert!(h
        .prepare_dependency_lexical_scopes(&c, None, |_, _, _| panic!(
            "duplicate preparation callback must not run"
        ))
        .is_err());
    assert!(selected.is_live(&h));
    assert!(h.set_context(root, None));
    assert!(!selected.is_live(&h));
    assert!(selected.site_count(&h).is_err());
}
#[test]
fn foreign_scopes_hosts_and_reloaded_owners_never_match_by_identity_or_node_number() {
    let c = verified(SOURCE);
    let (mut h, root, scopes, imports) = prepared(&c, 1, true);
    let (other, foreign, foreign_scopes, _) = prepared(&c, 1, true);
    assert!(h
        .select_datatype_dependencies(c.clone(), foreign, &scopes, &imports, limits())
        .is_err());
    assert!(h
        .select_datatype_dependencies(c.clone(), root, &foreign_scopes, &imports, limits())
        .is_err());
    assert!(h
        .select_datatype_dependencies(verified(SOURCE), root, &scopes, &imports, limits())
        .is_err());
    let selected = h
        .select_datatype_dependencies(c.clone(), root, &scopes, &imports, limits())
        .unwrap();
    assert!(!selected.is_live(&other));
    other.close();
    assert!(selected.is_live(&h));
    selected.retire();
    assert!(!selected.is_live(&h));
}
#[test]
fn imported_inheritance_keeps_vendor_local_names_and_destination_budgets() {
    let vendor = VENDOR.replace(
        "{type @name=base @kind=scalar}",
        "{type @name=base @kind=scalar} {type @name=exported @base=base}",
    );
    let source = SOURCE
        .replace("@name=local", "@name=base")
        .replace("v:base", "v:exported");
    let c = verified_vendor(&source, &vendor, "exported");
    // This fixture's ambient consumer differs; the prepared occurrence remains imported.
    let ctx = context(&c, "imported");
    for case in 0..4 {
        let mut destination = ReferenceScopePolicy::schema_defaults().unwrap();
        if case == 1 {
            destination.limits.max_work = 1;
        }
        // Configure manually because the consumer deliberately also declares "base".
        let mut h =
            FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
        let root = h
            .register_dependency_scope(
                &c,
                None,
                Some(context(&c, "base")),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        let id = c.dependency_ids().next().unwrap().to_owned();
        let target = h
            .register_dependency_scope(&c, Some(&id), Some(Default::default()), destination)
            .unwrap();
        h.prepare_dependency_lexical_scopes(&c, None, |_, _, _| {
            (Some(ctx.clone()), ReferenceScopePolicyOverrides::default())
        })
        .unwrap();
        h.allow_scope_crossing(root, target);
        let imports = vec![DatatypeDependencyImport {
            consumer: root,
            module: id.clone(),
            root: declaration(c.dependency_capture(&id).unwrap(), "exported"),
        }];
        let mut budget = limits();
        if case == 2 {
            budget.max_work = 5;
        }
        if case == 3 {
            budget.max_depth = 1;
        }
        let result = h.select_datatype_dependencies(
            c.clone(),
            root,
            &BTreeMap::from([(id, target)]),
            &imports,
            budget,
        );
        assert_eq!(result.is_ok(), case == 0, "case {case}");
        if let Ok(result) = result {
            assert_eq!(result.datatype_count(&h).unwrap(), 5);
        }
    }
}
#[test]
fn original_use_aliases_are_discovered_and_conflicting_original_aliases_refuse() {
    let uses = "{uses | {use @as=v @schema=urn:vendor}} ";
    for conflicting in [false, true] {
        let mut text = SOURCE.replace("{types |", &format!("{uses}{{types |"));
        if conflicting {
            text = text.replace("@ns v = urn:vendor", "@ns v = urn:wrong");
        } else {
            text = text.replace("@ns v = urn:vendor\n", "");
        }
        let c = verified(&text);
        let (mut h, root, scopes, imports) = prepared(&c, 1, true);
        assert_eq!(
            h.select_datatype_dependencies(c, root, &scopes, &imports, limits())
                .is_ok(),
            !conflicting
        );
    }
}
#[test]
fn missing_occurrence_preparation_stays_pending_despite_ready_root_inputs() {
    let c = verified(SOURCE);
    let mut h = FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    let root = h
        .register_dependency_scope(
            &c,
            None,
            Some(context(&c, "imported")),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let id = c.dependency_ids().next().unwrap().to_owned();
    let target = h
        .register_dependency_scope(
            &c,
            Some(&id),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    h.allow_scope_crossing(root, target);
    let scopes = BTreeMap::from([(id.clone(), target)]);
    let imports = vec![DatatypeDependencyImport {
        consumer: root,
        module: id,
        root: declaration(
            c.dependency_capture(c.dependency_ids().next().unwrap())
                .unwrap(),
            "base",
        ),
    }];
    assert!(h
        .select_datatype_dependencies(c.clone(), root, &scopes, &imports, limits())
        .is_err());
    let handles = h
        .prepare_dependency_lexical_scopes(&c, None, |_, _, _| {
            (None, ReferenceScopePolicyOverrides::default())
        })
        .unwrap();
    assert!(h
        .select_datatype_dependencies(c.clone(), root, &scopes, &imports, limits())
        .is_err());
    h.set_context(handles[0].1, Some(context(&c, "imported")));
    assert!(h
        .select_datatype_dependencies(c, root, &scopes, &imports, limits())
        .is_ok());
}
#[test]
fn wrong_empty_multi_and_cyclic_native_selections_never_match_the_asserted_target() {
    for case in 0..4 {
        let c = verified(SOURCE);
        let mut ctx = context(&c, if case == 0 { "local" } else { "imported" });
        if case == 1 {
            ctx.bindings.insert(
                "selected".into(),
                StandaloneExpressionBinding::any(ItemStream::empty()),
            );
        }
        if case == 2 {
            let item =
                RetainedCemNode::new(c.source().clone(), declaration(c.capture(), "imported"))
                    .unwrap()
                    .query_item();
            ctx.bindings.insert(
                "selected".into(),
                StandaloneExpressionBinding::any(ItemStream::from_items(vec![item.clone(), item])),
            );
        }
        if case == 3 {
            let id = c.sites().iter().find_map(|s| s.reference).unwrap();
            let item = RetainedCemNode::new(c.source().clone(), id)
                .unwrap()
                .query_item();
            ctx.bindings.insert(
                "selected".into(),
                StandaloneExpressionBinding::any(ItemStream::once(item)),
            );
        }
        let ((mut h, root, scopes, imports), _) = configured(
            &c,
            1,
            true,
            Some(ctx),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        );
        assert!(
            h.select_datatype_dependencies(c, root, &scopes, &imports, limits())
                .is_err(),
            "case {case}"
        );
    }
}
#[test]
fn native_callbacks_are_not_replayed_or_retained_by_selection_results_and_cancellation_blocks_publish(
) {
    use cem_ql::native::{NativeQueryFunction, NativeQueryRequest};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    #[derive(Debug)]
    struct Pick {
        _marker: Arc<()>,
        calls: Arc<AtomicUsize>,
        stop: Arc<AtomicBool>,
    }
    impl NativeQueryFunction for Pick {
        fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.stop.load(Ordering::SeqCst) {
                request.control.cancel_root(None, None).unwrap();
            }
            request.arguments[0].clone()
        }
    }
    let text = SOURCE.replace("#selected", r#"#native:call("fixture.pick", selected)"#);
    let c = verified(&text);
    let marker = Arc::new(());
    let weak = Arc::downgrade(&marker);
    let calls = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let mut ctx = context(&c, "imported");
    ctx.native_functions
        .register(
            "fixture.pick",
            1,
            Pick {
                _marker: marker.clone(),
                calls: calls.clone(),
                stop: stop.clone(),
            },
        )
        .unwrap();
    drop(marker);
    let ((mut h, root, scopes, imports), _) = configured(
        &c,
        1,
        true,
        Some(ctx),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    let selected = h
        .select_datatype_dependencies(c.clone(), root, &scopes, &imports, limits())
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(selected.is_live(&h));
    assert_eq!(selected.site_count(&h).unwrap(), 3);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    stop.store(true, Ordering::SeqCst);
    assert!(h
        .select_datatype_dependencies(c, root, &scopes, &imports, limits())
        .is_err());
    assert!(!selected.is_live(&h));
    drop(h);
    assert!(
        weak.upgrade().is_none(),
        "selection kept the actual host callbacks alive"
    );
}
#[test]
fn rule_selections_match_exact_query_and_native_sources_without_running_their_implementations() {
    for query in [false, true] {
        let text="@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:consumer | {types | {type @name=root @kind=scalar @rule={#selected}}}}";
        let c = capture(text);
        let (target, artifact) = rule_artifact(query);
        let module = artifact.id().to_owned();
        let artifacts = [artifact];
        let built = capsule::export(
            &c,
            identity(text, "file:///app/rule-consumer.cem"),
            &hash(b"registry"),
            BTreeSet::from([declaration(&c, "root")]),
            BTreeMap::from([(field(&c, "root", "rule"), target)]),
            &artifacts,
            &OperationControl::default(),
        )
        .unwrap();
        let c = Arc::new(
            capsule::verify(
                &built.bytes,
                &built.pin,
                &artifacts,
                &OperationControl::default(),
            )
            .unwrap(),
        );
        let tree = c.dependency_source(&module).unwrap();
        let id = tree
            .ast()
            .nodes
            .iter()
            .find_map(|n| match n {
                CemAstNode::Element {
                    expanded_name,
                    node_id,
                    ..
                } if expanded_name.local_name == "behavior" => Some(*node_id),
                _ => None,
            })
            .unwrap();
        let item = RetainedCemNode::new(tree.clone(), id).unwrap().query_item();
        let ctx = StandaloneExpressionContext::default().with_binding(
            "selected",
            StandaloneExpressionBinding::any(ItemStream::once(item)),
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
        let destination = h
            .register_dependency_scope(
                &c,
                Some(&module),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        h.prepare_dependency_lexical_scopes(&c, None, |_, _, _| {
            (Some(ctx.clone()), ReferenceScopePolicyOverrides::default())
        })
        .unwrap();
        let scopes = BTreeMap::from([(module, destination)]);
        assert!(h
            .select_datatype_dependencies(c.clone(), root, &scopes, &[], limits())
            .is_err());
        h.allow_scope_crossing(root, destination);
        let selected = h
            .select_datatype_dependencies(c, root, &scopes, &[], limits())
            .unwrap();
        assert_eq!(selected.site_count(&h).unwrap(), 1);
        assert_eq!(selected.datatype_count(&h).unwrap(), 1);
    }
}
#[test]
fn raw_tree_registration_does_not_implicitly_admit_verified_capture_metadata() {
    let c = verified(SOURCE);
    let mut h = FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    let root = h
        .register_scope(
            c.source().clone(),
            Some(context(&c, "imported")),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let id = c.dependency_ids().next().unwrap().to_owned();
    let target = h
        .register_scope(
            c.dependency_source(&id).unwrap().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    h.allow_scope_crossing(root, target);
    assert!(h
        .prepare_dependency_lexical_scopes(&c, None, |_, _, _| (
            Some(context(&c, "imported")),
            ReferenceScopePolicyOverrides::default()
        ))
        .is_err());
    let scopes = BTreeMap::from([(id.clone(), target)]);
    let imports = vec![DatatypeDependencyImport {
        consumer: root,
        module: id.clone(),
        root: declaration(c.dependency_capture(&id).unwrap(), "base"),
    }];
    assert!(h
        .select_datatype_dependencies(c.clone(), root, &scopes, &imports, limits())
        .is_err());
    assert!(h
        .prepare_dependency_lexical_scopes(&c, None, |_, _, _| panic!(
            "selection must not implicitly attach a capture"
        ))
        .is_err());
}
#[test]
fn imported_selection_reference_cycles_across_prepared_occurrence_scopes_refuse_without_writeback()
{
    let text = format!(
        "{} {{body | {{#back}}}}}}",
        SOURCE.strip_suffix('}').unwrap()
    );
    let c = verified(&text);
    let first = c.sites().iter().find_map(|s| s.reference).unwrap();
    let second = c
        .capture()
        .document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Reference {
                node_id,
                expression,
                ..
            } if expression == "#back" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let runtime = |name: &str, target: u32| {
        StandaloneExpressionContext::default().with_binding(
            name,
            StandaloneExpressionBinding::any(ItemStream::once(
                RetainedCemNode::new(c.source().clone(), target)
                    .unwrap()
                    .query_item(),
            )),
        )
    };
    let mut h = FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    let root = h
        .register_dependency_scope(
            &c,
            None,
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let id = c.dependency_ids().next().unwrap().to_owned();
    let destination = h
        .register_dependency_scope(
            &c,
            Some(&id),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let handles = h
        .prepare_dependency_lexical_scopes(&c, None, |node, _, _| {
            (
                Some(if node.node_id() == first {
                    runtime("selected", second)
                } else {
                    runtime("back", first)
                }),
                ReferenceScopePolicyOverrides::default(),
            )
        })
        .unwrap();
    assert_ne!(handles[0].1, handles[1].1);
    h.allow_scope_crossing(root, destination);
    let imports = vec![DatatypeDependencyImport {
        consumer: root,
        module: id.clone(),
        root: declaration(c.dependency_capture(&id).unwrap(), "base"),
    }];
    assert!(h
        .select_datatype_dependencies(
            c.clone(),
            root,
            &BTreeMap::from([(id, destination)]),
            &imports,
            limits()
        )
        .is_err());
    assert!(c
        .capture()
        .document()
        .nodes
        .iter()
        .filter(|n| matches!(n, CemAstNode::Reference { .. }))
        .all(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}
