use super::*;
#[test]
fn original_declaring_hops_need_their_own_directed_grants() {
    let g = verified(&chain());
    let (mut h, root, scopes) = prepared(&g, false);
    assert!(h
        .select_datatype_modules(g.clone(), root, &scopes, &[], limits())
        .is_err());
    h.allow_scope_crossing(root, scopes["vendor"]);
    h.allow_scope_crossing(root, scopes["core"]);
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
    h.allow_scope_crossing(scopes["core"], scopes["vendor"]);
    assert!(h
        .select_datatype_modules(g.clone(), root, &scopes, &[], limits())
        .is_err());
    h.allow_scope_crossing(scopes["vendor"], scopes["core"]);
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
        .is_ok());
}
#[test]
fn source_and_occurrence_contexts_are_never_replaced_by_ready_root_defaults() {
    let g = verified(&chain());
    for case in 0..4 {
        let mut h =
            FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
        let (root, scopes) = register(&mut h, &g);
        h.allow_scope_crossing(root, scopes["vendor"]);
        h.allow_scope_crossing(scopes["vendor"], scopes["core"]);
        if case > 0 {
            let c = context(&g, "vendor");
            h.prepare_datatype_module_lexical_scopes(&g, "app", |_, _, _| {
                (Some(c.clone()), Default::default())
            })
            .unwrap();
        }
        if case == 2 {
            h.prepare_datatype_module_lexical_scopes(&g, "vendor", |_, _, _| {
                (None, Default::default())
            })
            .unwrap();
        }
        if case == 3 {
            let c = context(&g, "core");
            h.prepare_datatype_module_lexical_scopes(&g, "vendor", |_, _, _| {
                (Some(c.clone()), Default::default())
            })
            .unwrap();
            h.set_context(scopes["core"], None);
        }
        assert!(
            h.select_datatype_modules(g.clone(), root, &scopes, &[], limits())
                .is_err(),
            "case {case}"
        );
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
    }
    let mut h = FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    let mut scopes = BTreeMap::new();
    for id in g.module_ids() {
        scopes.insert(
            id.into(),
            h.register_scope(
                g.module(id).unwrap().source().clone(),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap(),
        );
    }
    let root = scopes.remove("app").unwrap();
    assert!(h
        .select_datatype_modules(g.clone(), root, &scopes, &[], limits())
        .is_err());
    assert!(h
        .prepare_datatype_module_lexical_scopes(&g, "app", |_, _, _| (
            Some(Default::default()),
            Default::default()
        ))
        .is_err());
}
#[test]
fn wrong_missing_extra_and_copied_scopes_preserve_unchanged_prior_results() {
    let g = verified(&chain());
    let (mut h, root, scopes) = prepared(&g, true);
    let selected = h
        .select_datatype_modules(g.clone(), root, &scopes, &[], limits())
        .unwrap();
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
    for case in 0..3 {
        let mut wrong = scopes.clone();
        match case {
            0 => {
                wrong.remove("core");
            }
            1 => {
                wrong.insert("core".into(), scopes["vendor"]);
            }
            _ => {
                wrong.insert("extra".into(), scopes["core"]);
            }
        }
        assert!(h
            .compile_datatype_modules(
                g.clone(),
                root,
                &wrong,
                &[],
                &implementations(&g),
                &BTreeMap::new(),
                limits()
            )
            .is_err());
        assert!(selected.is_live(&h));
        assert!(compiled.is_live(&h));
    }
    let copied = verified(&chain());
    assert_eq!(g.pin().hash, copied.pin().hash);
    assert!(h
        .compile_datatype_modules(
            copied.clone(),
            root,
            &scopes,
            &[],
            &implementations(&copied),
            &BTreeMap::new(),
            limits()
        )
        .is_err());
    assert!(compiled.is_live(&h));
    let (mut foreign, _, _) = prepared(&g, true);
    assert!(!compiled.is_live(&foreign));
    assert!(!selected.is_live(&foreign));
    assert!(foreign
        .select_datatype_modules(g.clone(), root, &scopes, &[], limits())
        .is_err());
    h.set_context(scopes["vendor"], None);
    assert!(!compiled.is_live(&h));
    assert!(!selected.is_live(&h));
    assert!(compiled
        .validate(
            &h,
            *g.module("app").unwrap().roots().first().unwrap(),
            &input(1),
            Default::default()
        )
        .is_err());
}
#[test]
fn qualified_names_use_original_module_imports_and_exported_roots() {
    let core=source("file:///app/core.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:core | {types | {type @name=scalar @kind=scalar} {type @name=private @kind=scalar}}}","scalar");
    let mut vendor=source("file:///app/vendor.cem","@ns s = https://cem.dev/ns/schema/1\n@ns v = urn:core\n@default s\n{schema @namespace=urn:vendor | {types | {type @name=value @base=v:scalar}}}","value");
    vendor.bindings.insert(
        field(&vendor.capture, "value", "base"),
        ModuleTarget::ModuleDatatype {
            module: "core".into(),
            root: *core.roots.first().unwrap(),
        },
    );
    let mut app=source("file:///app/app.cem","@ns s = https://cem.dev/ns/schema/1\n@ns v = urn:vendor\n@default s\n{schema @namespace=urn:app | {types | {type @name=value @base=v:value}}}","value");
    app.bindings.insert(
        field(&app.capture, "value", "base"),
        ModuleTarget::ModuleDatatype {
            module: "vendor".into(),
            root: *vendor.roots.first().unwrap(),
        },
    );
    let g = verified(&BTreeMap::from([
        ("app".into(), app),
        ("vendor".into(), vendor),
        ("core".into(), core),
    ]));
    let mut h = FreshStructuralHost::new(&hash(b"registry"), Default::default()).unwrap();
    let (root, scopes) = register(&mut h, &g);
    h.allow_scope_crossing(root, scopes["vendor"]);
    h.allow_scope_crossing(scopes["vendor"], scopes["core"]);
    let requests = vec![
        DatatypeDependencyImport {
            consumer: root,
            module: "vendor".into(),
            root: *g.module("vendor").unwrap().roots().first().unwrap(),
        },
        DatatypeDependencyImport {
            consumer: scopes["vendor"],
            module: "core".into(),
            root: *g.module("core").unwrap().roots().first().unwrap(),
        },
    ];
    for imports in [&[][..], &requests[..1]] {
        assert!(h
            .compile_datatype_modules(
                g.clone(),
                root,
                &scopes,
                imports,
                &implementations(&g),
                &BTreeMap::new(),
                limits()
            )
            .is_err());
    }
    let mut bad = requests.clone();
    bad[1].consumer = root;
    assert!(h
        .select_datatype_modules(g.clone(), root, &scopes, &bad, limits())
        .is_err());
    let mut bad = requests.clone();
    bad[1].root = datatype(g.module("core").unwrap().capture(), "private");
    assert!(h
        .select_datatype_modules(g.clone(), root, &scopes, &bad, limits())
        .is_err());
    let selected = h
        .select_datatype_modules(g.clone(), root, &scopes, &requests, limits())
        .unwrap();
    assert_eq!(selected.datatype_count(&h).unwrap(), 3);
    let compiled = h
        .compile_datatype_modules(
            g.clone(),
            root,
            &scopes,
            &requests,
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
}
#[test]
fn request_limits_and_destination_policy_refuse_without_publishing_or_mutating_inputs() {
    let g = verified(&chain());
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
    for (work, depth) in [
        (0, 64),
        (100_001, 64),
        (1, 64),
        (100_000, 0),
        (100_000, 65),
        (100_000, 1),
    ] {
        let mut low = limits();
        low.max_work = work;
        low.max_depth = depth;
        assert!(h
            .compile_datatype_modules(
                g.clone(),
                root,
                &scopes,
                &[],
                &implementations(&g),
                &BTreeMap::new(),
                low
            )
            .is_err());
        assert!(compiled.is_live(&h));
    }
    let mut h = FreshStructuralHost::new(&hash(b"registry"), Default::default()).unwrap();
    let mut scopes = BTreeMap::new();
    for id in g.module_ids() {
        let mut policy = ReferenceScopePolicy::schema_defaults().unwrap();
        if id == "vendor" {
            policy.limits.max_work = 1;
        }
        scopes.insert(
            id.to_owned(),
            h.register_datatype_module_scope(&g, id, Some(Default::default()), policy)
                .unwrap(),
        );
    }
    let root = scopes.remove("app").unwrap();
    for (c, t) in [("app", "vendor"), ("vendor", "core")] {
        let ctx = context(&g, t);
        h.prepare_datatype_module_lexical_scopes(&g, c, |_, _, _| {
            (Some(ctx.clone()), Default::default())
        })
        .unwrap();
        h.allow_scope_crossing(if c == "app" { root } else { scopes[c] }, scopes[t]);
    }
    assert!(h
        .select_datatype_modules(g.clone(), root, &scopes, &[], limits())
        .is_err());
}
