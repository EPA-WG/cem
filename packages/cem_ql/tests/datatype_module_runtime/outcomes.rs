use super::*;
#[test]
fn inherited_list_bounds_and_nonexported_entrypoints_keep_original_source_owners() {
    let core = leaf();
    let mut vendor=source("file:///app/vendor.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:vendor | {types | {type @name=value @kind=list @base={#selected}}}}","value");
    vendor.bindings.insert(
        field(&vendor.capture, "value", "base"),
        ModuleTarget::ModuleDatatype {
            module: "core".into(),
            root: *core.roots.first().unwrap(),
        },
    );
    let mut app=source("file:///app/app.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=value @list-base={#selected} @max-items=2} {type @name=private @kind=scalar}}}","value");
    app.bindings.insert(
        field(&app.capture, "value", "list-base"),
        ModuleTarget::ModuleDatatype {
            module: "vendor".into(),
            root: *vendor.roots.first().unwrap(),
        },
    );
    let g = verified(&BTreeMap::from([
        ("core".into(), core),
        ("vendor".into(), vendor),
        ("app".into(), app),
    ]));
    let (mut h, root, scopes) = prepared(&g, true);
    let compiled = h
        .compile_datatype_modules(
            g.clone(),
            root,
            &scopes,
            &[],
            &implementations(&g),
            &BTreeMap::new(),
            limits(),
        )
        .unwrap();
    let id = *g.module("app").unwrap().roots().first().unwrap();
    assert_eq!(
        compiled
            .validate(&h, id, &input(2), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    let rejected = compiled
        .validate(&h, id, &input(3), Default::default())
        .unwrap();
    assert_eq!(rejected.accepted, Some(false));
    assert!(Arc::ptr_eq(
        rejected.cardinality[0].source.document(),
        g.module("app").unwrap().capture().document()
    ));
    let private = datatype(g.module("app").unwrap().capture(), "private");
    assert!(compiled
        .validate(&h, private, &input(1), Default::default())
        .is_err());
    assert_eq!(
        compiled
            .validate(&h, id, &input(1), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    compiled.retire();
    assert!(!compiled.is_live(&h));
}
#[test]
fn shared_diamond_preserves_one_original_source_and_each_declaring_scope() {
    let core = leaf();
    let left = derived("left", "core", *core.roots.first().unwrap());
    let right = derived("right", "core", *core.roots.first().unwrap());
    let mut app=source("file:///app/diamond.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=a @base={#left}} {type @name=b @base={#right}}}}","a");
    app.roots.insert(datatype(&app.capture, "b"));
    for (name, target, m) in [("a", "left", &left), ("b", "right", &right)] {
        app.bindings.insert(
            field(&app.capture, name, "base"),
            ModuleTarget::ModuleDatatype {
                module: target.into(),
                root: *m.roots.first().unwrap(),
            },
        );
    }
    let g = verified(&BTreeMap::from([
        ("core".into(), core),
        ("left".into(), left),
        ("right".into(), right),
        ("app".into(), app),
    ]));
    let mut h = FreshStructuralHost::new(&hash(b"registry"), Default::default()).unwrap();
    let (root, scopes) = register(&mut h, &g);
    let mut ctx = StandaloneExpressionContext::default();
    for id in ["left", "right"] {
        ctx = ctx.with_binding(
            id,
            StandaloneExpressionBinding::any(context(&g, id).bindings["selected"].value.clone()),
        );
        let c = context(&g, "core");
        h.prepare_datatype_module_lexical_scopes(&g, id, |_, _, _| {
            (Some(c.clone()), Default::default())
        })
        .unwrap();
        h.allow_scope_crossing(root, scopes[id]);
        h.allow_scope_crossing(scopes[id], scopes["core"]);
    }
    h.prepare_datatype_module_lexical_scopes(&g, "app", |_, _, _| {
        (Some(ctx.clone()), Default::default())
    })
    .unwrap();
    let selected = h
        .select_datatype_modules(g.clone(), root, &scopes, &[], limits())
        .unwrap();
    assert_eq!(selected.datatype_count(&h).unwrap(), 5);
    assert_eq!(selected.site_count(&h).unwrap(), 4);
    let compiled = h
        .compile_datatype_modules(
            g.clone(),
            root,
            &scopes,
            &[],
            &implementations(&g),
            &BTreeMap::new(),
            limits(),
        )
        .unwrap();
    for id in g.module("app").unwrap().roots() {
        assert_eq!(
            compiled
                .validate(&h, *id, &input(1), Default::default())
                .unwrap()
                .accepted,
            Some(true)
        );
    }
}

#[test]
fn compilation_renews_mutable_selectors_and_rejects_cycles_and_callback_stops() {
    use cem_ql::native::{NativeQueryFunction, NativeQueryRequest};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    #[derive(Debug)]
    struct Pick {
        choice: Arc<AtomicUsize>,
        stop: Arc<AtomicBool>,
        calls: Arc<AtomicUsize>,
        _marker: Arc<()>,
    }
    impl NativeQueryFunction for Pick {
        fn call(&self, r: NativeQueryRequest<'_>) -> ItemStream {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.stop.load(Ordering::SeqCst) {
                r.control.cancel_root(None, None).unwrap();
            }
            r.arguments[self.choice.load(Ordering::SeqCst)].clone()
        }
    }
    let mut sources = chain();
    let text="@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=value @base={#native:call(\"fixture.pick\", good, wrong, cycle)}}}}";
    let mut app = source("file:///app/types/app.cem", text, "value");
    app.bindings.insert(
        field(&app.capture, "value", "base"),
        ModuleTarget::ModuleDatatype {
            module: "vendor".into(),
            root: *sources["vendor"].roots.first().unwrap(),
        },
    );
    sources.insert("app".into(), app);
    let g = verified(&sources);
    let mut h = FreshStructuralHost::new(&hash(b"registry"), Default::default()).unwrap();
    let (root, scopes) = register(&mut h, &g);
    h.allow_scope_crossing(root, scopes["vendor"]);
    h.allow_scope_crossing(root, scopes["core"]);
    h.allow_scope_crossing(scopes["vendor"], scopes["core"]);
    let ctx = context(&g, "core");
    h.prepare_datatype_module_lexical_scopes(&g, "vendor", |_, _, _| {
        (Some(ctx.clone()), Default::default())
    })
    .unwrap();
    let mut ctx = StandaloneExpressionContext::default()
        .with_binding("good", context(&g, "vendor").bindings["selected"].clone())
        .with_binding("wrong", context(&g, "core").bindings["selected"].clone());
    let app = g.module("app").unwrap();
    let cycle = app.capture().occurrences().next().unwrap();
    ctx = ctx.with_binding(
        "cycle",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(app.source().clone(), cycle)
                .unwrap()
                .query_item(),
        )),
    );
    let choice = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let calls = Arc::new(AtomicUsize::new(0));
    let marker = Arc::new(());
    let weak = Arc::downgrade(&marker);
    ctx.native_functions
        .register(
            "fixture.pick",
            3,
            Pick {
                choice: choice.clone(),
                stop: stop.clone(),
                calls: calls.clone(),
                _marker: marker,
            },
        )
        .unwrap();
    h.prepare_datatype_module_lexical_scopes(&g, "app", |_, _, _| {
        (Some(ctx.clone()), Default::default())
    })
    .unwrap();
    drop(ctx);
    let selected = h
        .select_datatype_modules(g.clone(), root, &scopes, &[], limits())
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    choice.store(1, Ordering::SeqCst);
    let Err(error) = h.compile_datatype_modules(
        g.clone(),
        root,
        &scopes,
        &[],
        &implementations(&g),
        &BTreeMap::new(),
        limits(),
    ) else {
        panic!("historical selection authorized a changed target")
    };
    assert!(error
        .issues
        .iter()
        .any(|i| i.code == "invalid-datatype-dependency-target"));
    assert!(selected.is_live(&h));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    choice.store(0, Ordering::SeqCst);
    let compiled = h
        .compile_datatype_modules(
            g.clone(),
            root,
            &scopes,
            &[],
            &implementations(&g),
            &BTreeMap::new(),
            limits(),
        )
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    let root_id = *app.roots().first().unwrap();
    assert_eq!(
        compiled
            .validate(&h, root_id, &input(1), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    choice.store(2, Ordering::SeqCst);
    assert!(h
        .compile_datatype_modules(
            g.clone(),
            root,
            &scopes,
            &[],
            &implementations(&g),
            &BTreeMap::new(),
            limits()
        )
        .is_err());
    assert!(compiled.is_live(&h));
    choice.store(0, Ordering::SeqCst);
    stop.store(true, Ordering::SeqCst);
    assert!(h
        .compile_datatype_modules(
            g.clone(),
            root,
            &scopes,
            &[],
            &implementations(&g),
            &BTreeMap::new(),
            limits()
        )
        .is_err());
    assert!(!compiled.is_live(&h));
    drop(h);
    assert!(weak.upgrade().is_none());
}
#[test]
fn dt1_leaf_metadata_and_implementations_keep_the_exact_verified_arena() {
    use cem_ql::{datatype_dependency_capsule::DependencyArtifact, datatype_source_capsule as dt};
    let core = leaf();
    let root_id = *core.roots.first().unwrap();
    let b = dt::export(
        &core.capture,
        core.identity.clone(),
        &hash(b"registry"),
        core.roots.clone(),
        &OperationControl::default(),
    )
    .unwrap();
    let foreign = dt::verify(&b.bytes, &b.pin, &OperationControl::default()).unwrap();
    let id = b.pin.hash.clone();
    let mut app = derived("app", "unused", root_id);
    *app.bindings.values_mut().next().unwrap() = ModuleTarget::Datatype {
        pin: b.pin.clone(),
        root: root_id,
    };
    let artifacts = [DependencyArtifact::datatype(b)];
    let b = modules::export(
        "app",
        &BTreeMap::from([("app".into(), app)]),
        &artifacts,
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    let g = Arc::new(
        modules::verify(&b.bytes, &b.pin, &artifacts, &OperationControl::default()).unwrap(),
    );
    let mut h = FreshStructuralHost::new(&hash(b"registry"), Default::default()).unwrap();
    let (root, scopes) = register(&mut h, &g);
    let c = StandaloneExpressionContext::default().with_binding(
        "selected",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(g.leaf_source(&id).unwrap().clone(), root_id)
                .unwrap()
                .query_item(),
        )),
    );
    h.prepare_datatype_module_lexical_scopes(&g, "app", |_, _, _| {
        (Some(c.clone()), Default::default())
    })
    .unwrap();
    h.allow_scope_crossing(root, scopes[&id]);
    let compiled = h
        .compile_datatype_modules(
            g.clone(),
            root,
            &scopes,
            &[],
            &implementations(&g),
            &BTreeMap::new(),
            limits(),
        )
        .unwrap();
    assert_eq!(
        compiled
            .validate(
                &h,
                *g.module("app").unwrap().roots().first().unwrap(),
                &input(1),
                Default::default()
            )
            .unwrap()
            .accepted,
        Some(true)
    );
    let mut wrong = implementations(&g);
    wrong[0].source = foreign.sources()[&root_id].clone();
    assert!(h
        .compile_datatype_modules(
            g.clone(),
            root,
            &scopes,
            &[],
            &wrong,
            &BTreeMap::new(),
            limits()
        )
        .is_err());
    assert!(compiled.is_live(&h));
}
