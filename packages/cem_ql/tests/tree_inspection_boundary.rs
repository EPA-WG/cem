//! XML-VIEW-3-INSPECTION-API: retained-document presentation through the typed writer.
use cem_ml::{
    conversion::{
        direct_cem_output_pipeline,
        execute_conversion_output_pipeline_from_cem_tree_with_environment,
        ConversionOutputPipelineEnvironment, ConversionRegistry,
    },
    import::import_data,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    projection::cem_tree_inspection,
    scheduler::{AbortSignal, ScopePolicy},
    schema::SchemaRegistry,
    validation::xpath::XPathNativeNode,
};
use cem_ql::{
    api::{compile, evaluate, evaluate_with_control, CompileContext, EvaluationContext},
    eval::{imported_cem_tree, AtomValue, BudgetAxis, EvalError, Item, ItemStream},
    render::{render_template, TemplateData},
    xpath::functions::XPathQueryItem,
};
use std::{collections::BTreeMap, sync::Arc};

#[test]
fn both_query_document_views_use_the_same_retained_typed_writer_for_every_import() {
    let registry = SchemaRegistry::with_builtin_schemas();
    let conversions = ConversionRegistry::with_builtin_converters();
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &registry,
        conversion_registry: &conversions,
        package_artifact_reader: None,
        artifact_cache: None,
    };
    let mut pipeline = direct_cem_output_pipeline();
    pipeline.cemt_options.formatter_profile = Some("tabular".into());
    pipeline.cemt_insertion_context.formatter_profile = Some("tabular".into());
    pipeline.writer_insertion_context.formatter_profile = Some("tabular".into());
    for (format, source) in [
        ("xml", "<?xml-stylesheet href='https://invalid.test/inert.xsl'?><r empty=''><![CDATA[<raw>🍒]]><?keep inert?></r>"),
        ("json", r#"{"fruit":"<raw>🍒","empty":""}"#),
        ("yaml", "fruit: '<raw>🍒'\nempty: ''"),
        ("csv", "fruit,empty\n<raw>🍒,\n"),
    ] {
        let uri = format!("memory:tree-inspection.{format}");
        let owner = import_data(source, format, "cem", &uri).unwrap();
        let stream = Arc::new(cem_tree_inspection(owner.clone()));
        let result = execute_conversion_output_pipeline_from_cem_tree_with_environment(
            &environment,
            &pipeline,
            stream,
            Some(owner.node(0).unwrap().source.clone()),
            vec![],
            "tree-inspection-query-boundary",
            None,
            Some(&uri),
        );
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        // Read the named final text-output envelope, never an AST JSON handoff.
        let output = result.output.as_ref().and_then(|value| value.as_str()).unwrap();
        assert!(output.contains("{ast"), "{format}: {output}");
        assert!(output.contains("<raw>🍒"), "{format}: {output}");
        assert!(!output.contains('\u{1b}'), "browser text has no terminal escapes");
        assert!(Arc::ptr_eq(
            result.raw_cem_tree.as_ref().unwrap().owner().source_owner().unwrap(),
            &owner,
        ));
        assert!(Arc::ptr_eq(
            result.formatted_cemt_tree.as_ref().unwrap().owner().source_owner().unwrap(),
            &owner,
        ));
        assert!(!result.output_spans.is_empty());
        for document in [
            imported_cem_tree(owner.clone()),
            XPathQueryItem::from_node(XPathNativeNode::cem_document(owner.clone())),
        ] {
            let actual = inspect("cemml:inspect(doc)", ItemStream::once(document.clone()), ScopePolicy::host_root());
            assert!(actual.error.is_none(), "{format}: {:?}", actual.diagnostics);
            assert_eq!(actual.items, vec![Item::Atomic(AtomValue::String(output.into()))]);
            let rendered = render_template(
                "{pre | {code | {$cemml:inspect(doc)}}}",
                &TemplateData::default().with_binding("doc", ItemStream::once(document)),
            );
            assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);
            assert!(rendered.rendered.contains("&lt;raw&gt;🍒"));
            assert!(!rendered.rendered.contains("<raw>"));
            assert!(rendered.rendered.starts_with("<pre><code>"));
        }
        if format == "xml" {
            assert!(output.contains("@kind=cdata"));
            assert!(output.contains("@target=xml-stylesheet"));
            assert!(output.contains("@target=keep"));
            assert!(output.contains("@value=inert"));
        }
    }
}

#[test]
fn current_format_string_pass_through_is_not_a_structural_inspection_contract() {
    let source = "{r    @empty='' |  🍒  }";
    let context = CompileContext {
        policy_bindings: std::collections::BTreeMap::from([(
            "source".into(),
            ItemStream::once(Item::Atomic(AtomValue::String(source.into()))),
        )]),
        ..Default::default()
    };
    let query = compile("cemml:format(source)", &context).unwrap();
    let output = evaluate(
        &query,
        &EvaluationContext {
            policy_bindings: context.policy_bindings,
            ..Default::default()
        },
    );
    assert!(output.error.is_none(), "{:?}", output.diagnostics);
    assert_eq!(
        output.items,
        vec![Item::Atomic(AtomValue::String(source.into()))]
    );
}

fn inspect(source: &str, document: ItemStream, policy: ScopePolicy) -> ItemStream {
    let bindings = BTreeMap::from([("doc".into(), document)]);
    let query = compile(
        source,
        &CompileContext {
            policy_bindings: bindings.clone(),
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

fn document() -> ItemStream {
    ItemStream::once(imported_cem_tree(
        import_data(
            "<r><row>one</row><row>two</row></r>",
            "xml",
            "cem",
            "memory:inspect.xml",
        )
        .unwrap(),
    ))
}

#[test]
fn inspection_validates_cardinality_and_requires_a_retained_document() {
    let policy = ScopePolicy::host_root();
    let empty = inspect("cemml:inspect(doc)", ItemStream::empty(), policy);
    assert!(empty.error.is_none(), "{:?}", empty.diagnostics);
    assert!(empty.items.is_empty());
    let tree = import_data("<r/>", "xml", "cem", "memory:inspect.xml").unwrap();
    for items in [
        vec![Item::Atomic(AtomValue::String("<r/>".into()))],
        vec![Item::Node("{r}".into())],
        vec![Item::Record(BTreeMap::new())],
        vec![Item::Atomic(AtomValue::Integer(1))],
        vec![
            imported_cem_tree(tree.clone()),
            imported_cem_tree(tree.clone()),
        ],
        vec![XPathQueryItem::from_node(
            XPathNativeNode::cem_node(tree, 1).unwrap(),
        )],
    ] {
        let failure = inspect("cemml:inspect(doc)", ItemStream::from_items(items), policy);
        assert!(
            matches!(failure.error, Some(EvalError::TypeError(_))),
            "{:?}",
            failure
        );
        assert!(failure.items.is_empty());
        assert!(failure
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.ql.type_error" && d.source_map.is_some()));
    }
    let failure = inspect("cemml:inspect(doc.children)", document(), policy);
    assert!(matches!(failure.error, Some(EvalError::TypeError(_))));
    assert!(failure.items.is_empty());
}

#[test]
fn inspection_honors_lowered_work_and_payload_limits_without_partial_or_catch_output() {
    use cem_ml::scheduler::{tree::PolicyScopeId, ScopePolicyTree};
    let root = PolicyScopeId(0);
    let child = PolicyScopeId(1);
    let policy = ScopePolicy::host_root().with_queue_size(256);
    let mut scopes = ScopePolicyTree::new(root, policy);
    scopes
        .install(child, root, policy.with_queue_size(2))
        .unwrap();
    assert!(scopes.install(PolicyScopeId(2), child, policy).is_err());
    let query = r#"try { cemml:inspect(doc) } catch (code, message) { "partial" }"#;
    let result = inspect(query, document(), scopes.effective(root).unwrap());
    assert!(result.error.is_none(), "{:?}", result.diagnostics);
    let failure = inspect(query, document(), scopes.effective(child).unwrap());
    assert_eq!(
        failure.error,
        Some(EvalError::BudgetExceeded(BudgetAxis::ItemsPerStage))
    );
    assert!(failure.items.is_empty());
    let [Item::Atomic(AtomValue::String(output))] = result.items.as_slice() else {
        panic!("expected display text")
    };
    let exact = inspect(
        query,
        document(),
        policy.with_memory_bytes(output.len() as u64),
    );
    assert!(exact.error.is_none(), "{:?}", exact.diagnostics);
    assert_eq!(exact.items, result.items);
    for bytes in [1, output.len() as u64 - 1] {
        let failure = inspect(query, document(), policy.with_memory_bytes(bytes));
        assert!(failure.error.is_some());
        assert!(failure.items.is_empty());
        assert!(failure
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.ql.inspect_limit" && d.source_map.is_some()));
    }
}

#[test]
fn inspection_honors_enclosing_control_memory_and_cancellation() {
    let bindings = BTreeMap::from([("doc".into(), document())]);
    let query = compile(
        "cemml:inspect(doc)",
        &CompileContext {
            policy_bindings: bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let context = EvaluationContext {
        policy_bindings: bindings,
        ..Default::default()
    };
    let control = OperationControl::default();
    let success = evaluate_with_control(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
    assert!(success.error.is_none(), "{:?}", success.diagnostics);
    assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    let control = OperationControl::with_root_policy(
        AbortSignal::new(),
        ScopePolicy::host_root().with_memory_bytes(1),
    )
    .unwrap();
    let failure = evaluate_with_control(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
    assert_eq!(failure.error, Some(EvalError::Unsupported("inspection payload limit exceeded".into())));
    assert!(failure.diagnostics.iter().any(|d| d.code == "cem.ql.inspect_limit"));
    assert!(failure.items.is_empty());
    assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    let cancelled = evaluate_with_control(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
    assert_eq!(cancelled.error, Some(EvalError::Cancelled));
    assert!(cancelled.items.is_empty());
}
