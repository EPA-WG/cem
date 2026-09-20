use cem_ml::{
    import::import_data,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    projection::cem_tree_nodes,
    scheduler::{tree::PolicyScopeId, ScopePolicy, ScopePolicyTree},
};
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{imported_cem_tree, AtomValue, BudgetAxis, EvalError, Item, ItemStream};
use std::{collections::BTreeMap, sync::Arc};

fn eval(source: &str, limit: u32) -> ItemStream {
    let query =
        compile(source, &CompileContext::default()).expect("generic collection query compiles");
    evaluate(
        &query,
        &EvaluationContext {
            scope_policy: ScopePolicy::host_root().with_queue_size(limit),
            ..EvaluationContext::default()
        },
    )
}

fn strings(stream: ItemStream) -> Vec<String> {
    assert!(stream.error.is_none(), "{:?}", stream.diagnostics);
    stream
        .items
        .into_iter()
        .map(|item| match item {
            Item::Atomic(AtomValue::String(value)) => value,
            _ => panic!("expected string: {item:?}"),
        })
        .collect()
}

#[test]
fn grouping_uses_expression_keys_and_first_occurrence_order() {
    assert_eq!(
        strings(eval(
            r#"
        seq:group_by(("🍒", "🍋", "🍒", "🍏"), fn(value) => value).key
    "#,
            128
        )),
        ["🍒", "🍋", "🍏"]
    );
    assert_eq!(
        strings(eval(
            r#"
        seq:group_by(("Apple", "pear", "apricot"), fn(value) => str:lower(str:slice(value, 0, 1)))
            .items
    "#,
            128
        )),
        ["Apple", "apricot", "pear"]
    );
}

#[test]
fn grouping_distinguishes_empty_keys_empty_strings_and_atomic_types() {
    let result = eval(
        r#"seq:group_by(({k:()}, {k:""}, {k:1}, {k:"1"}, {k:()}), fn(row) => row.k)"#,
        128,
    );
    assert!(result.error.is_none(), "{:?}", result.diagnostics);
    assert_eq!(result.items.len(), 4);
    for (group, (key, count)) in result.items.iter().zip([
        (vec![], 2),
        (vec![Item::Atomic(AtomValue::String("".into()))], 1),
        (vec![Item::Atomic(AtomValue::Integer(1))], 1),
        (vec![Item::Atomic(AtomValue::String("1".into()))], 1),
    ]) {
        assert_eq!(field(group, "key"), key);
        assert_eq!(field(group, "items").len(), count);
    }
}

#[test]
fn stable_sort_accepts_explicit_text_numeric_and_direction_options() {
    assert_eq!(
        strings(eval(r#"seq:sorted(("10", "2", "3"), fn(x) => x)"#, 128)),
        ["10", "2", "3"]
    );
    assert_eq!(
        strings(eval(
            r#"seq:sorted(("10", "2", "3"), fn(x) => x, "ascending", "number")"#,
            128
        )),
        ["2", "3", "10"]
    );
    assert_eq!(
        strings(eval(
            r#"
        seq:sorted(({n:"2", label:"a"}, {label:"missing"}, {n:"10", label:"b"},
                    {n:"2", label:"c"}, {n:"bad", label:"invalid"}),
                   fn(x) => x.n, "descending", "number").label
    "#,
            128
        )),
        ["b", "a", "c", "missing", "invalid"]
    );
}

#[test]
fn collections_reject_invalid_options_keys_and_materialization_over_budget() {
    for source in [
        r#"seq:sorted((1,2), fn(x) => x, "sideways")"#,
        r#"seq:sorted((1,2), fn(x) => x, "ascending", "guess")"#,
        r#"seq:group_by((1,2), fn(x) => {a: x})"#,
        r#"seq:sorted((1,2), fn(x) => (x,x))"#,
    ] {
        let result = eval(source, 128);
        assert!(result.error.is_some(), "must reject {source}");
        assert!(result.items.is_empty(), "no partial collection on error");
    }
    for name in ["group_by", "sorted"] {
        let result = eval(&format!("seq:{name}((1,2,3,4,5), fn(x) => x)"), 2);
        assert!(
            result.error.is_some(),
            "{name} must honor collection budgets"
        );
        assert!(result.items.is_empty());
    }
}

fn field(item: &Item, name: &str) -> Vec<Item> {
    match item {
        Item::Record(fields) => fields.get(name).cloned().unwrap_or_default(),
        _ => item.view().unwrap().field(name).unwrap_or_default(),
    }
}

fn native_eval(source: &str, document: &Item, policy: ScopePolicy) -> ItemStream {
    let bindings = BTreeMap::from([("doc".into(), ItemStream::once(document.clone()))]);
    let query = compile(
        source,
        &CompileContext {
            policy_bindings: bindings.clone(),
            source_uri: Some("memory:xml-view-2.cemql".into()),
            ..Default::default()
        },
    )
    .unwrap();
    evaluate(
        &query,
        &EvaluationContext {
            policy_bindings: bindings,
            scope_policy: policy,
            ..Default::default()
        },
    )
}

#[test]
fn native_groups_keep_parent_boundaries_expanded_names_and_source_order() {
    let tree = import_data(
        "<r xmlns:a='urn:one' xmlns:b='urn:one' xmlns:c='urn:two'><left><c:row/>before<a:row/><b:row/><a:other/></left><right><b:row/><a:row/></right></r>",
        "xml", "cem", "memory:groups.xml",
    ).unwrap();
    let before = cem_tree_nodes(tree.ast());
    let document = imported_cem_tree(tree.clone());
    let parents = field(&field(&document, "children")[0], "children");
    let left = field(&parents[0], "children");
    let right = field(&parents[1], "children");
    let groups = native_eval(
        r#"for parent in doc.children.children {
            seq:group_by(seq:where(parent.children, fn(n) => n.kind == "element"),
                fn(n) => n.namespace + "|" + n.name)
        }"#,
        &document,
        ScopePolicy::host_root().with_queue_size(256),
    );
    assert!(groups.error.is_none(), "{:?}", groups.diagnostics);
    assert_eq!(groups.items.len(), 4);
    let expected = [
        ("urn:two|row", vec![left[0].clone()]),
        ("urn:one|row", vec![left[2].clone(), left[3].clone()]),
        ("urn:one|other", vec![left[4].clone()]),
        ("urn:one|row", right),
    ];
    for (group, (key, originals)) in groups.items.iter().zip(expected) {
        assert_eq!(strings(ItemStream::from_items(field(group, "key"))), [key]);
        let members = field(group, "items");
        assert_eq!(members, originals);
        for (actual, original) in members.iter().zip(originals) {
            assert_eq!(actual.source_map(), original.source_map());
            assert_eq!(field(actual, "id"), field(&original, "id"));
        }
    }
    assert_eq!(cem_tree_nodes(tree.ast()), before);
}

#[test]
fn native_sort_preserves_ties_missing_keys_provenance_and_owner_in_both_directions() {
    let tree = import_data(
        "<r>\n<row n='2'/>\n<row/>\n<row n='10'/>\n<row n='2'/>\n<row n=''/>\n<row n='bad'/>\n<row n='NaN'/>\n<row n='INF'/>\n</r>",
        "xml", "cem", "memory:sort.xml",
    ).unwrap();
    let owner = Arc::downgrade(&tree);
    let before = cem_tree_nodes(tree.ast());
    let document = imported_cem_tree(tree.clone());
    let rows: Vec<_> = field(&field(&document, "children")[0], "children")
        .into_iter()
        .filter(|node| matches!(field(node, "kind")[0].atom(), Some(AtomValue::String(kind)) if kind == "element"))
        .collect();
    let mut retained = None;
    for (mode, direction, order) in [
        ("number", "ascending", [0, 3, 2, 1, 4, 5, 6, 7]),
        ("number", "descending", [2, 0, 3, 1, 4, 5, 6, 7]),
        ("text", "ascending", [4, 2, 0, 3, 7, 6, 5, 1]),
        ("text", "descending", [5, 6, 7, 0, 3, 2, 4, 1]),
    ] {
        let result = native_eval(
            &format!(
                r#"seq:sorted(seq:where(doc.children.children, fn(n) => n.kind == "element"),
                    fn(n) => n.attributes.value, "{direction}", "{mode}")"#
            ),
            &document,
            ScopePolicy::host_root()
                .with_cpu_workers(1)
                .with_queue_size(256),
        );
        assert!(result.error.is_none(), "{:?}", result.diagnostics);
        for (actual, index) in result.items.iter().zip(order) {
            assert_eq!(actual, &rows[index], "{mode}/{direction}");
            assert_eq!(actual.source_map(), rows[index].source_map());
            assert_eq!(
                strings(ItemStream::from_items(field(actual, "line"))),
                [(index + 2).to_string()]
            );
        }
        assert_eq!(result.items.len(), rows.len());
        assert_eq!(cem_tree_nodes(tree.ast()), before);
        retained = Some(result);
    }
    drop(rows);
    drop(document);
    drop(tree);
    assert!(
        owner.upgrade().is_some(),
        "sorted nodes retain their source owner"
    );
    drop(retained);
    assert!(
        owner.upgrade().is_none(),
        "no hidden owner survives the results"
    );
}

#[test]
fn collections_obey_lowered_scope_budgets_and_discard_late_key_failures() {
    let root = PolicyScopeId(0);
    let child = PolicyScopeId(1);
    let policy = ScopePolicy::host_root()
        .with_cpu_workers(1)
        .with_queue_size(256);
    let mut scopes = ScopePolicyTree::new(root, policy);
    scopes
        .install(child, root, policy.with_queue_size(2))
        .unwrap();
    assert!(scopes.install(PolicyScopeId(2), child, policy).is_err());
    let document = imported_cem_tree(
        import_data(
            "<r><row n='3'/><row n='1'/><row n='2'/></r>",
            "xml",
            "cem",
            "memory:budget.xml",
        )
        .unwrap(),
    );
    for name in ["group_by", "sorted"] {
        let query = format!("seq:{name}(doc.children.children, fn(n) => n.attributes.value)");
        let success = native_eval(&query, &document, scopes.effective(root).unwrap());
        assert!(success.error.is_none(), "{:?}", success.diagnostics);
        let failure = native_eval(
            &format!("try {{ {query} }} catch (code, message) {{ 99 }}"),
            &document,
            scopes.effective(child).unwrap(),
        );
        assert_eq!(
            failure.error,
            Some(EvalError::BudgetExceeded(BudgetAxis::ItemsPerStage))
        );
        assert!(failure.items.is_empty());
        let diagnostic = failure
            .diagnostics
            .iter()
            .find(|d| d.code == "cem.ql.budget_exceeded")
            .unwrap();
        assert!(diagnostic
            .source_map
            .as_ref()
            .is_some_and(|s| !s.frames.is_empty()));

        // Fail after two valid keys; no partial groups or sorted prefix escapes.
        for bad in ["(1, 2)", "n", r#"report:raise("test.key", "failed key")"#] {
            let result = native_eval(
                &format!(
                    r#"seq:{name}(doc.children.children, fn(n) => if n.attributes.value == "2" {{ {bad} }} else {{ n.attributes.value }})"#
                ),
                &document,
                policy,
            );
            assert!(result.error.is_some(), "{name}/{bad}");
            assert!(result.items.is_empty(), "{name}/{bad}");
        }
    }
}

#[test]
fn keyed_collections_release_call_depth_and_honor_host_cancellation() {
    let bindings = BTreeMap::from([(
        "items".into(),
        ItemStream::from_items(
            (0..100)
                .rev()
                .map(|n| Item::Atomic(AtomValue::Integer(n)))
                .collect(),
        ),
    )]);
    for name in ["group_by", "sorted"] {
        let query = compile(
            &format!("seq:{name}(items, fn(n) => n)"),
            &CompileContext {
                policy_bindings: bindings.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        let context = EvaluationContext {
            policy_bindings: bindings.clone(),
            scope_policy: ScopePolicy::host_root()
                .with_cpu_workers(1)
                .with_queue_size(256),
            ..Default::default()
        };
        let result = evaluate(&query, &context);
        assert!(result.error.is_none(), "{:?}", result.diagnostics);
        assert_eq!(result.items.len(), 100);
        let control = OperationControl::default();
        control.cancel_root(None, None).unwrap();
        let result =
            cem_ql::api::evaluate_with_control(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
        assert_eq!(result.error, Some(EvalError::Cancelled));
        assert!(result.items.is_empty());
    }
}
