//! DATA-CELL-NAVIGATION / DATA-CELL-NODE-TEXT / CEMT-VALUES:
//! every format enters through CEM-ML import, then uses the same native API.
use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    resolver::{ResolverPolicy, ResolverRegistry},
    scheduler::ScopePolicy,
};
use cem_ql::{
    api::{compile, evaluate, evaluate_with_control, CompileContext, EvaluationContext},
    eval::{
        AtomValue, BudgetAxis, EvalError, Item, ItemStream, QueryContextScope, QueryItemView,
        QueryItemViewKind, QueryNodeAccessError, QueryNodeIterator, QueryNodeTextIterator,
    },
    xpath::functions::CemtXPathFunctions,
};
use std::sync::Arc;

const IMPORTS: [(&str, &str); 4] = [
    ("xml", "<r><name>ivysaur</name><id>2</id></r>"),
    ("json", r#"{"name":"ivysaur","id":2}"#),
    ("yaml", "name: ivysaur\nid: 2\n"),
    ("csv", "name,id\nivysaur,2\n"),
];

fn string(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}

fn run(query: &str, context: &EvaluationContext) -> ItemStream {
    let compiled = compile(
        query,
        &CompileContext {
            policy_bindings: context.policy_bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let result = evaluate(&compiled, context);
    assert!(result.error.is_none(), "{query}: {:?}", result.diagnostics);
    result
}

fn bound(context: &EvaluationContext, node: &Item) -> EvaluationContext {
    let mut context = context.clone();
    context
        .policy_bindings
        .insert("node".into(), ItemStream::once(node.clone()));
    context
}

fn import(format: &str, source: &str) -> (EvaluationContext, Item) {
    let mut context = EvaluationContext::default();
    context
        .policy_bindings
        .insert("source".into(), ItemStream::once(string(source)));
    context
        .policy_bindings
        .insert("format".into(), ItemStream::once(string(format)));
    let nodes = run("data:read(source, format).root", &context);
    assert_eq!(nodes.items.len(), 1);
    (context, nodes.items[0].clone())
}

fn field(node: &Item, name: &str) -> Vec<Item> {
    node.view().unwrap().field(name).unwrap_or_default()
}

fn provenance(node: &Item) -> Option<(Option<String>, Option<String>, Option<u32>)> {
    node.view()
        .unwrap()
        .provenance()
        .map(|p| (p.source_uri, p.source_key, p.line_number))
}

fn lexical(node: &Item, name: &str) -> String {
    match field(node, name).as_slice() {
        [item] => match item.atom().unwrap() {
            AtomValue::String(v) => v,
            other => panic!("{other:?}"),
        },
        other => panic!("{name}: {other:?}"),
    }
}

fn text(context: &EvaluationContext, node: &Item) -> String {
    match run("dom:text(node)", &bound(context, node)).items[0]
        .atom()
        .unwrap()
    {
        AtomValue::String(v) => v,
        other => panic!("{other:?}"),
    }
}

fn xpath(context: &mut EvaluationContext) {
    let library = CemtXPathFunctions::compile(
        r#"@doc cem-ml 1
@ns t = "https://cem.dev/ns/transform/cem/1"
@default t
{module |
 {function @name=tree.keep @visibility=public @returns=any |
  {param @name=node @type=any @required=true}
  {body | {xpath @context=node @sequence-type="node()" | {expression | .}}}}}
"#,
        "memory:retained-values.cemt",
    )
    .unwrap();
    library
        .install(
            &mut context.native_functions,
            Arc::new(ResolverRegistry::new()),
            Arc::new(ResolverPolicy::new()),
        )
        .unwrap();
}

fn walk(context: &EvaluationContext, node: &Item, count: &mut usize) {
    *count += 1;
    assert!(*count < 100, "fixture must have a finite source tree");
    let context = bound(context, node);
    let children = run("dom:children(node)", &context).items;
    assert_eq!(children, field(node, "children"));
    let attrs = field(node, "attributes");
    for child in children.iter().chain(attrs.iter()) {
        let child_context = bound(&context, child);
        let parent = run("dom:parent(node)", &child_context).items;
        assert_eq!(parent.len(), 1);
        assert_eq!(parent[0].identity(), node.identity());
        assert_eq!(parent[0].source_map(), node.source_map());
        assert_eq!(provenance(&parent[0]), provenance(node));
        assert!(!child.source_map().unwrap().frames.is_empty());
        walk(&context, child, count);
    }
}

#[test]
fn all_imports_keep_native_navigation_text_and_source_maps_in_both_views() {
    for (format, source) in IMPORTS {
        let (mut context, root) = import(format, source);
        xpath(&mut context);
        let projected = run(r#"native:call("tree.keep", node)"#, &bound(&context, &root))
            .items
            .remove(0);
        for root in [root, projected] {
            assert!(run("dom:parent(node)", &bound(&context, &root))
                .items
                .is_empty());
            assert_eq!(text(&context, &root), "ivysaur2", "{format}");
            let mut count = 0;
            walk(&context, &root, &mut count);
            assert!(count >= 6);
        }
    }
}

#[test]
fn source_text_boundaries_and_xpath_coalescing_keep_their_own_parents() {
    let (mut context, root) = import(
        "xml",
        "<r xmlns:p='urn:one' p:a='&amp;'>a<![CDATA[b]]>c<p:x/></r>",
    );
    xpath(&mut context);
    let element = field(&root, "children").remove(0);
    let context = bound(&context, &element);
    let children = run("dom:children(node)", &context).items;
    assert_eq!(
        children
            .iter()
            .map(|n| lexical(n, "kind"))
            .collect::<Vec<_>>(),
        ["text", "cdata", "text", "element"]
    );
    assert_eq!(
        children
            .iter()
            .map(|n| text(&context, n))
            .collect::<Vec<_>>(),
        ["a", "b", "c", ""]
    );
    assert_eq!(lexical(&children[3], "namespace"), "urn:one");
    let attribute = field(&element, "attributes")
        .into_iter()
        .find(|n| lexical(n, "name") == "a")
        .unwrap();
    assert_eq!(text(&context, &attribute), "&");
    assert_eq!(lexical(&attribute, "namespace"), "urn:one");
    assert!(run("dom:children(node)", &bound(&context, &attribute))
        .items
        .is_empty());
    let semantic = run(r#"native:call("tree.keep", node)"#, &context)
        .items
        .remove(0);
    let semantic_children = run("dom:children(node)", &bound(&context, &semantic)).items;
    assert_eq!(semantic_children.len(), 2);
    assert_eq!(text(&context, &semantic_children[0]), "abc");
    for child in &children[..3] {
        let adapted = run(r#"native:call("tree.keep", node)"#, &bound(&context, child))
            .items
            .remove(0);
        assert_eq!(adapted.identity(), semantic_children[0].identity());
        let parent = run("dom:parent(node)", &bound(&context, &adapted))
            .items
            .remove(0);
        assert_eq!(parent.identity(), semantic.identity());
    }
}

#[test]
fn text_uses_import_decoding_and_excludes_descendant_comments() {
    let (mut context, root) = import("xml", "<?keep inert?><r a='&quot;&amp;'> a\r\nb<![CDATA[<c>]]><em>&amp;&#x1F352;</em><!--note--><?inside data?><empty/></r>");
    xpath(&mut context);
    assert_eq!(text(&context, &root), " a\nb<c>&🍒");
    walk(&context, &root, &mut 0);
    let children = field(&root, "children");
    assert_eq!(text(&context, &children[0]), "inert");
    let element = &children[1];
    assert_eq!(text(&context, &field(element, "attributes")[0]), "\"&");
    let descendants = field(element, "children");
    assert_eq!(lexical(&descendants[0], "value"), " a\r\nb");
    let comment = descendants
        .iter()
        .find(|n| lexical(n, "kind") == "comment")
        .unwrap();
    assert_eq!(text(&context, comment), "note");
    let pi = descendants
        .iter()
        .find(|n| lexical(n, "kind") == "processing-instruction")
        .unwrap();
    assert_eq!(text(&context, pi), "data");
    assert_eq!(text(&context, descendants.last().unwrap()), "");
    assert_eq!(
        run("dom:text(())", &context).items[0].atom(),
        Some(AtomValue::String(String::new()))
    );
    let semantic = run(r#"native:call("tree.keep", node)"#, &bound(&context, &root))
        .items
        .remove(0);
    assert_eq!(text(&context, &semantic), " a\nb<c>&🍒");
}

#[test]
fn reference_clone_and_empty_element_share_one_import_independent_contract() {
    for (format, source) in IMPORTS {
        let (context, root) = import(format, source);
        let original = field(&root, "children").remove(0);
        let context = bound(&context, &original);
        let reference = run("dom:reference((node, node))", &context).items.remove(0);
        let targets = field(&reference, "targets");
        assert_ne!(reference.identity(), original.identity());
        assert_eq!(targets.len(), 2);
        assert!(targets.iter().all(|n| n.identity() == original.identity()));
        assert_eq!(text(&context, &reference), "ivysaur2ivysaur2");
        let cloned = run("dom:clone(node)", &context).items.remove(0);
        assert_ne!(cloned.identity(), original.identity());
        assert_eq!(cloned.source_map(), original.source_map());
        assert_eq!(provenance(&cloned), provenance(&original));
        assert_eq!(text(&context, &cloned), "ivysaur2");
        assert!(run("dom:parent(node)", &bound(&context, &cloned))
            .items
            .is_empty());
        let old_children = field(&original, "children");
        let new_children = run("dom:children(node)", &bound(&context, &cloned)).items;
        assert_eq!(old_children.len(), new_children.len());
        for (old, new) in old_children.iter().zip(&new_children) {
            assert_ne!(old.identity(), new.identity());
            assert_eq!(text(&context, old), text(&context, new));
            assert_eq!(
                run("dom:parent(node)", &bound(&context, new)).items[0].identity(),
                cloned.identity()
            );
        }
        let empty = run("dom:element(node)", &context).items.remove(0);
        assert_ne!(empty.identity(), original.identity());
        assert_eq!(lexical(&empty, "name"), lexical(&original, "name"));
        assert_eq!(
            lexical(&empty, "namespace"),
            lexical(&original, "namespace")
        );
        assert!(field(&empty, "children").is_empty() && field(&empty, "attributes").is_empty());
        assert_eq!(text(&context, &empty), "");
        // Querying and constructing never reparents or mutates the imported source.
        assert_eq!(
            run("dom:parent(node)", &context).items[0].identity(),
            root.identity()
        );
        assert_eq!(text(&context, &original), "ivysaur2");
    }
}

#[test]
fn navigation_checks_cardinality_and_pipeline_steps_preserve_order() {
    let (context, root) = import("xml", "<r><a/><b/></r>");
    let context = bound(&context, &root);
    assert!(run("dom:parent(())", &context).items.is_empty());
    assert!(run("dom:children(())", &context).items.is_empty());
    assert_eq!(
        run("node.dom:children().dom:children().name", &context).items,
        vec![string("a"), string("b")]
    );
    let parents = run("node.dom:children().dom:children().dom:parent()", &context).items;
    assert_eq!(parents.len(), 2);
    assert_eq!(parents[0].identity(), parents[1].identity());
    for query in [
        "dom:parent(1)",
        "dom:children(\"text\")",
        "dom:children((node, node))",
        "dom:parent((node, node))",
        "dom:children({kind: \"element\"})",
        "dom:children(cemml:parse(\"{main | {p | Hi}}\"))",
        "dom:parent(cemml:parse(\"{main | {p | Hi}}\"))",
        "dom:text({value: \"text\"})",
    ] {
        let compiled = compile(
            query,
            &CompileContext {
                policy_bindings: context.policy_bindings.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        let result = evaluate(&compiled, &context);
        assert!(result.items.is_empty(), "{query}");
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.ql.type_error"),
            "{query}: {result:?}"
        );
    }
}

#[test]
fn template_parameters_and_results_retain_nodes_until_final_escaped_projection() {
    use cem_ql::render::{
        compile_template, render_compiled_template, render_plan_to_html, render_template,
        CompileTemplateOptions, RenderPlanNode, TemplateData,
    };
    let (context, root) = import(
        "xml",
        "<n quote='&quot;'>a<![CDATA[<raw>]]>&amp;<b>🍒</b></n>",
    );
    let node = field(&root, "children").remove(0);
    let data = TemplateData::default().with_binding("node", ItemStream::once(node.clone()));
    let compiled = compile_template(
        concat!(
            r#"{template @name=keep | {param @name=value}{body | {$value}}}"#,
            r#"{section @title="{$node}" | {call @template=keep @with:value="{$node}"}}"#
        ),
        &CompileTemplateOptions::default(),
    );
    let plan = render_compiled_template(&compiled, &data);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    let RenderPlanNode::Element {
        children,
        attributes,
        ..
    } = &plan.nodes[0]
    else {
        panic!()
    };
    let RenderPlanNode::Reference { reference, .. } = &children[0] else {
        panic!()
    };
    assert_eq!(reference.values().len(), 1);
    assert_eq!(reference.values()[0].identity(), node.identity());
    assert_eq!(
        attributes[0].value_stream.items[0].identity(),
        node.identity()
    );
    assert_eq!(render_plan_to_html(&plan), "<section title=\"a&lt;raw&gt;&amp;🍒\"><n quote=\"&quot;\">a&lt;raw&gt;&amp;<b>🍒</b></n></section>");
    let text_only = render_template("{section | {$dom:text(node)}}", &data);
    assert!(
        text_only.diagnostics.is_empty(),
        "{:?}",
        text_only.diagnostics
    );
    assert_eq!(text_only.rendered, "<section>a&lt;raw&gt;&amp;🍒</section>");
    assert_eq!(text(&context, &node), "a<raw>&🍒");
    assert_eq!(
        node.view()
            .unwrap()
            .parent(QueryContextScope(0))
            .unwrap()
            .unwrap()
            .identity(),
        root.identity()
    );
}

#[derive(Debug, Clone)]
enum Behavior {
    Leaf,
    Children,
    DenyAfterFirst,
    Unsupported,
    WrongKind,
    Cancel(OperationControl),
    Wide,
}
#[derive(Debug, Clone)]
struct HostNode(Behavior);
impl QueryItemView for HostNode {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "test.restricted-node"
    }
    fn identity(&self) -> String {
        format!("{:?}", self.0)
    }
    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Node
    }
    // These deliberately cannot substitute for the scope-aware capabilities.
    fn atom(&self) -> Option<AtomValue> {
        Some(AtomValue::String("private".into()))
    }
    fn field(&self, name: &str) -> Option<Vec<Item>> {
        match name {
            "value" => Some(vec![string("private")]),
            "children" => Some(vec![Item::native(HostNode(Behavior::Leaf))]),
            _ => None,
        }
    }
    fn parent(&self, scope: QueryContextScope) -> Result<Option<Item>, QueryNodeAccessError> {
        if scope.0 != 7 {
            return Err(QueryNodeAccessError::ScopeViolation);
        }
        match self.0 {
            Behavior::Unsupported => Err(QueryNodeAccessError::Unsupported),
            Behavior::Leaf => Ok(None),
            _ => Ok(Some(Item::native(HostNode(Behavior::Leaf)))),
        }
    }
    fn children(
        &self,
        scope: QueryContextScope,
    ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
        if scope.0 != 7 {
            return Err(QueryNodeAccessError::ScopeViolation);
        }
        if matches!(self.0, Behavior::Unsupported) {
            return Err(QueryNodeAccessError::Unsupported);
        }
        let count = match self.0 {
            Behavior::Leaf => 0,
            Behavior::Wide => 20,
            _ => 2,
        };
        Ok(Box::new((0..count).map(move |index| {
            if index == 1 {
                match &self.0 {
                    Behavior::DenyAfterFirst => return Err(QueryNodeAccessError::ScopeViolation),
                    Behavior::Cancel(control) => control.abort_signal().abort(),
                    Behavior::WrongKind => return Ok(string("not a node")),
                    _ => (),
                }
            }
            Ok(Item::native(HostNode(Behavior::Leaf)))
        })))
    }
    fn text_fragments(
        &self,
        scope: QueryContextScope,
    ) -> Result<QueryNodeTextIterator<'_>, QueryNodeAccessError> {
        if scope.0 != 7 {
            return Err(QueryNodeAccessError::ScopeViolation);
        }
        match self.0 {
            Behavior::Unsupported => Err(QueryNodeAccessError::Unsupported),
            Behavior::DenyAfterFirst => Ok(Box::new(
                [Ok("prefix"), Err(QueryNodeAccessError::ScopeViolation)].into_iter(),
            )),
            Behavior::Wide => Ok(Box::new(std::iter::repeat_n(Ok(""), 100))),
            _ => Ok(Box::new(std::iter::once(Ok("private")))),
        }
    }
}

fn host_result(
    query: &str,
    behavior: Behavior,
    scope: u32,
    policy: ScopePolicy,
    control: &OperationControl,
) -> ItemStream {
    let context = EvaluationContext {
        scope: QueryContextScope(scope),
        scope_policy: policy,
        ..bound(
            &EvaluationContext::default(),
            &Item::native(HostNode(behavior)),
        )
    };
    let compiled = compile(
        query,
        &CompileContext {
            policy_bindings: context.policy_bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    evaluate_with_control(&compiled, &context, control, ROOT_EXECUTION_SCOPE_ID)
}

#[test]
fn restricted_and_unsupported_views_fail_without_field_or_atom_fallback() {
    for query in ["dom:parent(node)", "dom:children(node)", "dom:text(node)"] {
        for (behavior, scope, code) in [
            (Behavior::Children, 0, "cem.ql.scope_violation"),
            (Behavior::Unsupported, 7, "cem.ql.type_error"),
        ] {
            let result = host_result(
                query,
                behavior,
                scope,
                ScopePolicy::host_root(),
                &OperationControl::default(),
            );
            assert!(
                result.error.is_some() && result.items.is_empty(),
                "{query}: {result:?}"
            );
            assert!(
                result.diagnostics.iter().any(|d| d.code == code),
                "{query}: {result:?}"
            );
            if scope == 7 {
                assert!(matches!(result.error, Some(EvalError::Unsupported(_))));
            }
        }
    }
    for query in [
        "dom:children(node)",
        "node.dom:children()",
        "dom:text(node)",
    ] {
        let result = host_result(
            query,
            Behavior::DenyAfterFirst,
            7,
            ScopePolicy::host_root(),
            &OperationControl::default(),
        );
        assert!(
            result.error.is_some() && result.items.is_empty(),
            "{query}: {result:?}"
        );
    }
    let result = host_result(
        "dom:children(node)",
        Behavior::WrongKind,
        7,
        ScopePolicy::host_root(),
        &OperationControl::default(),
    );
    assert!(
        result.items.is_empty()
            && result
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.ql.type_error")
    );
    let result = host_result(
        "dom:children(node)",
        Behavior::Children,
        7,
        ScopePolicy::host_root(),
        &OperationControl::default(),
    );
    assert_eq!(result.items.len(), 2);
    for child in result.items {
        assert_eq!(
            child
                .view()
                .unwrap()
                .text_fragments(QueryContextScope(0))
                .err(),
            Some(QueryNodeAccessError::ScopeViolation)
        );
    }
}

#[test]
fn navigation_and_text_work_are_bounded_and_discard_prefix_on_cancellation() {
    for query in ["dom:children(node)", "node.dom:children()"] {
        let control = OperationControl::default();
        let result = host_result(
            query,
            Behavior::Cancel(control.clone()),
            7,
            ScopePolicy::host_root(),
            &control,
        );
        assert!(
            result.error.is_some() && result.items.is_empty(),
            "{query}: {result:?}"
        );
        let result = host_result(
            query,
            Behavior::Wide,
            7,
            ScopePolicy::host_root().with_queue_size(4),
            &OperationControl::default(),
        );
        assert!(
            result.error.is_some() && result.items.is_empty(),
            "{query}: {result:?}"
        );
        assert!(matches!(
            result.error,
            Some(EvalError::BudgetExceeded(BudgetAxis::ItemsPerStage))
        ));
    }
    // Even visits that yield no text consume traversal work.
    let result = host_result(
        "dom:text(node)",
        Behavior::Wide,
        7,
        ScopePolicy::host_root().with_memory_bytes(32),
        &OperationControl::default(),
    );
    assert!(
        result.error.is_some() && result.items.is_empty(),
        "{result:?}"
    );
    assert!(matches!(
        result.error,
        Some(EvalError::BudgetExceeded(BudgetAxis::XPathWorkUnits))
    ));
    let control = OperationControl::default();
    control.abort_signal().abort();
    let result = host_result(
        "dom:parent(node)",
        Behavior::Children,
        7,
        ScopePolicy::host_root(),
        &control,
    );
    assert!(result.error.is_some() && result.items.is_empty());
}

#[test]
fn execution_child_can_lower_navigation_limit_without_cancelling_its_parent() {
    use cem_ml::operation_control::{ExecutionScopeKind, ExecutionScopeRegistration};
    let control = OperationControl::default();
    let child = control
        .register_scope(
            ROOT_EXECUTION_SCOPE_ID,
            ExecutionScopeRegistration::inherited(
                ExecutionScopeKind::Template,
                "small-navigation",
                ScopePolicy::host_root().with_queue_size(4),
            ),
        )
        .unwrap();
    let context = EvaluationContext {
        scope: QueryContextScope(7),
        ..bound(
            &EvaluationContext::default(),
            &Item::native(HostNode(Behavior::Wide)),
        )
    };
    let query = compile(
        "dom:children(node)",
        &CompileContext {
            policy_bindings: context.policy_bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let result = evaluate_with_control(&query, &context, &control, child);
    assert!(result.items.is_empty());
    assert!(matches!(
        result.error,
        Some(EvalError::BudgetExceeded(BudgetAxis::ItemsPerStage))
    ));
    let result = evaluate_with_control(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
    assert!(result.error.is_none(), "{result:?}");
    assert_eq!(result.items.len(), 20);
}
