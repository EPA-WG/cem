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
    Attribute,
    Children,
    DenyAfterFirst,
    Unsupported,
    WrongKind,
    Cancel(OperationControl),
    Wide,
    Cycle,
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
            "children" | "attributes" => Some(vec![Item::native(HostNode(Behavior::Leaf))]),
            "kind" => Some(vec![string(if matches!(self.0, Behavior::Attribute) {
                "attribute"
            } else {
                "element"
            })]),
            "name" => Some(vec![string("id")]),
            "namespace" => Some(vec![string("")]),
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
            Behavior::Leaf | Behavior::Attribute => 0,
            Behavior::Wide => 20,
            Behavior::Cycle => 1,
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
            Ok(Item::native(HostNode(
                if matches!(self.0, Behavior::Cycle) {
                    Behavior::Cycle
                } else {
                    Behavior::Leaf
                },
            )))
        })))
    }
    fn attributes(
        &self,
        scope: QueryContextScope,
    ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
        Ok(Box::new(self.children(scope)?.map(|node| {
            node.map(|node| {
                if node.view().is_some() {
                    Item::native(HostNode(Behavior::Attribute))
                } else {
                    node
                }
            })
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
    for query in [
        "dom:parent(node)",
        "dom:children(node)",
        "dom:descendants(node)",
        r#"dom:attribute(node, "id")"#,
        "dom:text(node)",
    ] {
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
        "dom:descendants(node)",
        "node.dom:descendants()",
        r#"dom:attribute(node, "id")"#,
        r#"node.dom:attribute("id")"#,
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
    for query in [
        "dom:children(node)",
        "dom:descendants(node)",
        r#"dom:attribute(node, "id")"#,
    ] {
        let result = host_result(
            query,
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
    }
    for query in [
        "dom:children(node)",
        "dom:descendants(node)",
        r#"dom:attribute(node, "id")"#,
    ] {
        let result = host_result(
            query,
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
}

#[test]
fn navigation_and_text_work_are_bounded_and_discard_prefix_on_cancellation() {
    for query in [
        "dom:children(node)",
        "node.dom:children()",
        "dom:descendants(node)",
        "node.dom:descendants()",
        r#"dom:attribute(node, "id")"#,
        r#"node.dom:attribute("id")"#,
    ] {
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
    // Even visits that yield no text or matching attributes consume work.
    for query in ["dom:text(node)", r#"dom:attribute(node, "missing")"#] {
        let result = host_result(
            query,
            Behavior::Wide,
            7,
            ScopePolicy::host_root().with_memory_bytes(16),
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
    }
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
    for source in [
        "dom:children(node)",
        "dom:descendants(node)",
        r#"dom:attribute(node, "id")"#,
    ] {
        let query = compile(
            source,
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
}

#[test]
fn cyclic_host_descendants_are_bounded_without_recursion_or_partial_results() {
    let result = host_result(
        "dom:descendants(node)",
        Behavior::Cycle,
        7,
        ScopePolicy::host_root().with_queue_size(64),
        &OperationControl::default(),
    );
    assert!(result.items.is_empty());
    assert!(matches!(
        result.error,
        Some(EvalError::BudgetExceeded(BudgetAxis::ItemsPerStage))
    ));
}

fn transported(node: &Item) -> Item {
    use cem_ml::value::artifact::CemValueArtifactLimits;
    use cem_ql::eval::portable::{decode_values, encode_values};
    let limits = CemValueArtifactLimits::default();
    decode_values(
        &encode_values(&ItemStream::once(node.clone()), &limits).unwrap(),
        &limits,
    )
    .unwrap()
    .items
    .remove(0)
}

#[test]
fn descendants_preserve_depth_first_order_identity_and_each_native_view() {
    for (format, source) in IMPORTS {
        let (mut context, root) = import(format, source);
        xpath(&mut context);
        let semantic = run(r#"native:call("tree.keep", node)"#, &bound(&context, &root))
            .items
            .remove(0);
        let cloned = run("dom:clone(node)", &bound(&context, &root))
            .items
            .remove(0);
        for node in [root, semantic, cloned] {
            for node in [node.clone(), transported(&node)] {
                let context = bound(&context, &node);
                let mut expected = Vec::new();
                fn collect(node: &Item, out: &mut Vec<Item>) {
                    for child in field(node, "children") {
                        out.push(child.clone());
                        collect(&child, out);
                    }
                }
                collect(&node, &mut expected);
                let actual = run("dom:descendants(node)", &context).items;
                assert!(!actual.is_empty(), "{format}");
                assert_eq!(actual.len(), expected.len());
                for (actual, expected) in actual.iter().zip(&expected) {
                    assert_eq!(actual.identity(), expected.identity());
                    assert_eq!(actual.source_map(), expected.source_map());
                    assert_eq!(provenance(actual), provenance(expected));
                    assert_ne!(actual.identity(), node.identity());
                }
                let repeated = run("(node, node).dom:descendants()", &context).items;
                assert_eq!(repeated, [actual.clone(), actual].concat());
            }
        }
    }
}

#[test]
fn attribute_selectors_are_exact_and_keep_native_identity_and_provenance() {
    let (mut context, root) = import(
        "xml",
        "<r xmlns='urn:default' xmlns:p='urn:catalog' id='local' p:id='qualified'><child/></r>",
    );
    xpath(&mut context);
    let source = field(&root, "children").remove(0);
    let semantic = run(
        r#"native:call("tree.keep", node)"#,
        &bound(&context, &source),
    )
    .items
    .remove(0);
    let cloned = run("dom:clone(node)", &bound(&context, &source))
        .items
        .remove(0);
    for node in [source, semantic, cloned] {
        for node in [node.clone(), transported(&node)] {
            let context = bound(&context, &node);
            for (selector, namespace, value) in [
                (r#""id""#, "", "local"),
                (r#"{namespace: "", name: "id"}"#, "", "local"),
                (
                    r#"{namespace: "urn:catalog", name: "id"}"#,
                    "urn:catalog",
                    "qualified",
                ),
            ] {
                let selected = run(&format!("dom:attribute(node, {selector})"), &context).items;
                assert_eq!(selected.len(), 1);
                let attribute = &selected[0];
                let expected = field(&node, "attributes")
                    .into_iter()
                    .find(|a| lexical(a, "name") == "id" && lexical(a, "namespace") == namespace)
                    .unwrap();
                assert_eq!(attribute.identity(), expected.identity());
                assert_eq!(attribute.source_map(), expected.source_map());
                assert_eq!(provenance(attribute), provenance(&expected));
                assert_eq!(text(&context, attribute), value);
                assert_eq!(
                    run("dom:parent(node)", &bound(&context, attribute)).items[0].identity(),
                    node.identity()
                );
                assert!(run("dom:descendants(node)", &bound(&context, attribute))
                    .items
                    .is_empty());
                assert_eq!(
                    run(&format!("(node, node).dom:attribute({selector})"), &context).items,
                    [selected.clone(), selected].concat()
                );
            }
            assert!(run(r#"dom:attribute(node, "missing")"#, &context)
                .items
                .is_empty());
            assert!(run(
                r#"dom:attribute(node, {namespace: "urn:other", name: "id"})"#,
                &context
            )
            .items
            .is_empty());
            // Namespace declarations retain the semantics of each source/XPath view.
            let declarations = field(&node, "attributes")
                .into_iter()
                .filter(|a| {
                    lexical(a, "namespace") == "http://www.w3.org/2000/xmlns/"
                        && lexical(a, "name") == "p"
                })
                .collect::<Vec<_>>();
            assert_eq!(run(r#"dom:attribute(node, {namespace: "http://www.w3.org/2000/xmlns/", name: "p"})"#, &context).items, declarations);
        }
    }
}

#[test]
fn constructed_attributes_retain_types_and_references_without_extra_axes() {
    use cem_ql::{eval::output::output_nodes, render::*};
    let template = compile_template(
        r#"{child | {attribute @name=count @type=integer @value='002'}{attribute @name=label @type=node @content-type=text/html @value='{data:read("<name>ivy</name>", "xml").root.children}'}{$data:read("<content><b>body</b></content>", "xml").root.children}}"#,
        &CompileTemplateOptions::default(),
    );
    let plan = render_compiled_template(&template, &TemplateData::default());
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    let node = output_nodes(plan.nodes).items.remove(0);
    for node in [node.clone(), transported(&node)] {
        let context = bound(&EvaluationContext::default(), &node);
        let count = run(r#"dom:attribute(node, "count")"#, &context)
            .items
            .remove(0);
        assert_eq!(
            count
                .view()
                .unwrap()
                .value_contract()
                .unwrap()
                .model
                .value_type
                .as_deref(),
            Some("integer")
        );
        assert_eq!(
            field(&count, "values")[0].atom(),
            Some(AtomValue::Integer(2))
        );
        assert!(count.view().unwrap().value_contract().is_some());
        let label = run(r#"dom:attribute(node, "label")"#, &context)
            .items
            .remove(0);
        assert_eq!(text(&context, &label), "ivy");
        assert_eq!(
            field(&label, "values")[0].view().unwrap().kind(),
            QueryItemViewKind::Node
        );
        let descendants = run("dom:descendants(node)", &context).items;
        assert_eq!(descendants.len(), 1);
        assert_eq!(lexical(&descendants[0], "kind"), "reference");
        assert!(!field(&descendants[0], "targets").is_empty());
        let reference = run("dom:reference(node)", &context).items.remove(0);
        for reference in [reference.clone(), transported(&reference)] {
            let context = bound(&context, &reference);
            assert!(run("dom:descendants(node)", &context).items.is_empty());
            assert!(run(r#"dom:attribute(node, "count")"#, &context)
                .items
                .is_empty());
        }
    }
}

#[test]
fn native_axes_reject_invalid_nodes_and_selectors_even_on_empty_input() {
    let (context, root) = import("xml", "<r id='a'/>");
    let context = bound(&context, &root);
    assert!(run("dom:descendants(())", &context).items.is_empty());
    assert!(run(r#"dom:attribute((), "id")"#, &context).items.is_empty());
    for query in [
        "dom:descendants(1)",
        "dom:descendants({})",
        "dom:descendants((node, node))",
        r#"dom:descendants(cemml:parse("{r}"))"#,
        r#"dom:attribute(1, "id")"#,
        r#"dom:attribute({}, "id")"#,
        r#"dom:attribute((node, node), "id")"#,
        r#"dom:attribute(cemml:parse("{r}"), "id")"#,
        r#"dom:attribute(node, ())"#,
        r#"dom:attribute(node, ("id", "name"))"#,
        r#"dom:attribute(node, 1)"#,
        r#"dom:attribute(node, "")"#,
        r#"dom:attribute(node, "p:id")"#,
        r#"dom:attribute(node, "*")"#,
        r#"dom:attribute(node, {name: "id"})"#,
        r#"dom:attribute(node, {namespace: ""})"#,
        r#"dom:attribute(node, {namespace: "", name: 1})"#,
        r#"dom:attribute(node, {namespace: (), name: "id"})"#,
        r#"dom:attribute(node, {namespace: "", name: "id", extra: true})"#,
        r#"dom:attribute((), "p:id")"#,
        r#"().dom:attribute("p:id")"#,
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
        assert!(
            result.error.is_some() && result.items.is_empty(),
            "{query}: {result:?}"
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.ql.type_error"),
            "{query}: {result:?}"
        );
    }
}
