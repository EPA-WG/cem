//! Passive function selection must finish before any executable binding is made.
use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::CemAstNode,
    schema::{
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        function_references::{FunctionCatalog, FunctionSelection, FunctionSelectionBudget},
        machine::CemSchemaMachine,
        reference_policy::{ReferenceOccurrence, ReferenceUnresolvedPolicy},
        reference_traversal::ReferenceTraversalLimits,
        registry::CEM_SCHEMA_URI,
        value_contracts::ValueContractSource,
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
    value::reference_resolution::{
        ReferenceLinkEvaluation, ReferenceResolutionHost, ReferenceResolutionIssueKind,
        ReferenceResolutionState,
    },
};
use std::{collections::BTreeMap, sync::Arc};

#[path = "function_references/bindings.rs"]
mod bindings;

fn source(body: &str, namespace: &str) -> ValueContractSource {
    let text = format!("@ns schema = \"{CEM_SCHEMA_URI}\"\n@ns foreign = \"urn:foreign\"\n@default schema\n{{schema | {body}}}");
    let captured = Arc::new(
        CemSchemaMachine::new(
            CompiledSchema::cem_core(),
            CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                SourceId(1),
                text.into_bytes(),
            ))),
        )
        .build_with_lexical_scopes(),
    );
    let doc = captured.document().clone();
    assert!(doc.diagnostics.is_empty(), "{:?}", doc.diagnostics);
    let id = doc
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "schema" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    ValueContractSource::new(
        SchemaDeclarationNode::new(doc, id).unwrap(),
        namespace,
        BTreeMap::new(),
    )
    .with_captured_names(captured)
    .unwrap()
}
fn nodes(source: &ValueContractSource, local: &str) -> Vec<SchemaDeclarationNode> {
    source
        .schema
        .document()
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == local => {
                SchemaDeclarationNode::new(source.schema.document().clone(), *node_id)
            }
            _ => None,
        })
        .collect()
}
fn references(source: &ValueContractSource) -> Vec<SchemaDeclarationNode> {
    source
        .schema
        .document()
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Reference { node_id, .. } => {
                SchemaDeclarationNode::new(source.schema.document().clone(), *node_id)
            }
            _ => None,
        })
        .collect()
}
fn budget() -> FunctionSelectionBudget {
    FunctionSelectionBudget::new(ReferenceTraversalLimits {
        max_depth: 64,
        max_work: 10_000,
    })
    .unwrap()
}
struct Host {
    bindings: BTreeMap<String, ReferenceLinkEvaluation<SchemaDeclarationNode>>,
    policy: ReferenceUnresolvedPolicy,
    crossing: bool,
    calls: usize,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            bindings: BTreeMap::new(),
            policy: ReferenceUnresolvedPolicy::schema_defaults().unwrap(),
            crossing: false,
            calls: 0,
        }
    }
}
impl Host {
    fn bind(&mut self, reference: &SchemaDeclarationNode, targets: Vec<SchemaDeclarationNode>) {
        self.bindings.insert(
            reference.identity(),
            ReferenceLinkEvaluation::Resolved(targets),
        );
    }
}
impl ReferenceResolutionHost for Host {
    type Node = SchemaDeclarationNode;
    type Scope = usize;
    fn scope(&self, n: &Self::Node) -> usize {
        Arc::as_ptr(n.document()) as usize
    }
    fn scope_limits(&self, _: &usize) -> ReferenceTraversalLimits {
        ReferenceTraversalLimits {
            max_depth: 64,
            max_work: 10_000,
        }
    }
    fn reference_occurrence(&self, n: &Self::Node) -> Option<ReferenceOccurrence> {
        match n.node() {
            CemAstNode::Reference {
                node_id,
                expression,
                source,
                ..
            } => Some(ReferenceOccurrence {
                identity: n.identity(),
                node_id: Some(*node_id),
                expression: Some(expression.clone()),
                source_map: source.clone(),
            }),
            _ => None,
        }
    }
    fn unresolved_policy(&self, _: &Self::Node) -> &ReferenceUnresolvedPolicy {
        &self.policy
    }
    fn permits_edge(&self, a: &Self::Node, b: &Self::Node) -> bool {
        self.crossing || self.scope(a) == self.scope(b)
    }
    fn evaluate(&mut self, n: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        self.calls += 1;
        self.bindings
            .get(&n.identity())
            .cloned()
            .unwrap_or_else(|| ReferenceLinkEvaluation::Pending("missing-context".into()))
    }
}
impl SchemaDeclarationHost for Host {
    fn source_reference(&self, n: SchemaDeclarationNode) -> Self::Node {
        n
    }
    fn declaration_node(&self, n: &Self::Node) -> Option<SchemaDeclarationNode> {
        Some(n.clone())
    }
    fn declaration_schema(&self, _: &SchemaDeclarationNode) -> Option<SchemaDeclarationNode> {
        None
    }
}
fn assert_error(result: &FunctionSelection, code: &str) {
    assert!(result.target().is_none(), "{result:?}");
    assert!(result.resolution.failed, "{result:?}");
    assert!(
        result
            .resolution
            .diagnostics
            .iter()
            .any(|d| d.code.ends_with(code)),
        "{result:?}"
    );
}

#[test]
fn native_singleton_retains_original_function_body_and_slot() {
    let source = source("{behaviors | {behavior @name=caller @implementation=function @function={#chosen} | {function @name=f @returns=string | {param @name=value @type=string} {body | {$ value}}}}}", "local");
    let caller = nodes(&source, "behavior").remove(0);
    let target = nodes(&source, "function").remove(0);
    let reference = references(&source).remove(0);
    let mut budget = budget();
    let catalog = FunctionCatalog::collect(&[source.clone()], &mut budget).unwrap();
    let mut host = Host::default();
    host.bind(&reference, vec![target.clone()]);
    let selected = catalog.select(&caller, &mut host, &mut budget).unwrap();
    let selected = selected.target().unwrap();
    assert_eq!(selected.function().identity(), target.identity());
    assert_eq!(selected.behavior().identity(), caller.identity());
    assert_eq!(
        selected.source().schema.identity(),
        source.schema.identity()
    );
    assert!(Arc::ptr_eq(
        selected.function().document(),
        source.schema.document()
    ));
    assert!(matches!(
        reference.node(),
        CemAstNode::Reference { targets: None, .. }
    ));
    assert_eq!(host.calls, 1);
}

#[test]
fn complete_empty_and_multiple_selections_never_pick_or_deduplicate() {
    let source = source("{behaviors | {behavior @name=caller @implementation=function @function={#chosen} | {function @name=f @returns=string}}}", "local");
    let caller = nodes(&source, "behavior").remove(0);
    let f = nodes(&source, "function").remove(0);
    let reference = references(&source).remove(0);
    for targets in [vec![], vec![f.clone(), f.clone()]] {
        let mut budget = budget();
        let catalog = FunctionCatalog::collect(&[source.clone()], &mut budget).unwrap();
        let mut host = Host::default();
        host.bind(&reference, targets);
        assert_error(
            &catalog.select(&caller, &mut host, &mut budget).unwrap(),
            "function-cardinality",
        );
    }
}

#[test]
fn pending_is_not_empty_and_retry_uses_current_targets() {
    let source = source("{behaviors | {behavior @name=caller @implementation=function @function={#chosen} | {function @name=f @returns=string}}}", "local");
    let caller = nodes(&source, "behavior").remove(0);
    let mut budget = budget();
    let catalog = FunctionCatalog::collect(&[source.clone()], &mut budget).unwrap();
    let mut host = Host::default();
    let pending = catalog.select(&caller, &mut host, &mut budget).unwrap();
    assert_eq!(pending.resolution.state, ReferenceResolutionState::Pending);
    assert!(!pending
        .resolution
        .diagnostics
        .iter()
        .any(|d| d.code.ends_with("function-cardinality")));
    host.bind(&references(&source)[0], nodes(&source, "function"));
    assert!(catalog
        .select(&caller, &mut host, &mut budget)
        .unwrap()
        .target()
        .is_some());
    host.bind(&references(&source)[0], vec![]);
    assert_error(
        &catalog.select(&caller, &mut host, &mut budget).unwrap(),
        "function-cardinality",
    );
}

#[test]
fn nested_references_preserve_singleton_and_cycles_stay_incomplete() {
    let source = source("{behaviors | {behavior @name=caller @implementation=function @function={#chosen} | {function @name=f @returns=string}}} {#next}", "local");
    let caller = nodes(&source, "behavior").remove(0);
    let refs = references(&source);
    let mut budget = budget();
    let catalog = FunctionCatalog::collect(&[source.clone()], &mut budget).unwrap();
    let mut host = Host::default();
    host.bind(&refs[0], vec![refs[1].clone()]);
    host.bind(&refs[1], nodes(&source, "function"));
    assert!(catalog
        .select(&caller, &mut host, &mut budget)
        .unwrap()
        .target()
        .is_some());
    host.bind(&refs[1], vec![refs[0].clone()]);
    let cycle = catalog.select(&caller, &mut host, &mut budget).unwrap();
    assert!(cycle.target().is_none());
    assert!(cycle
        .resolution
        .issues
        .iter()
        .any(|i| i.kind == ReferenceResolutionIssueKind::Cycle));
}

#[test]
fn wrong_kind_unnamed_and_wrong_namespace_targets_are_errors() {
    for target in [
        "{type @name=f}",
        "{function @returns=string}",
        "{foreign:function @name=f @returns=string}",
    ] {
        let source = source(&format!("{{behaviors | {{behavior @name=caller @implementation=function @function={{#chosen}} | {target}}}}}"), "local");
        let caller = nodes(&source, "behavior").remove(0);
        let target = nodes(
            &source,
            if target.starts_with("{type") {
                "type"
            } else {
                "function"
            },
        )
        .remove(0);
        let mut budget = budget();
        let catalog = FunctionCatalog::collect(&[source.clone()], &mut budget).unwrap();
        let mut host = Host::default();
        host.bind(&references(&source)[0], vec![target]);
        assert!(
            catalog
                .select(&caller, &mut host, &mut budget)
                .unwrap()
                .resolution
                .failed
        );
    }
}

#[test]
fn an_attribute_named_function_is_not_a_function_declaration() {
    let source = source(
        "{behaviors | {behavior @name=caller @implementation=function @function={#chosen}}}",
        "local",
    );
    let caller = nodes(&source, "behavior").remove(0);
    let target = source
        .schema
        .document()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Attribute {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "function" => {
                SchemaDeclarationNode::new(source.schema.document().clone(), *node_id)
            }
            _ => None,
        })
        .unwrap();
    let mut budget = budget();
    let catalog = FunctionCatalog::collect(&[source.clone()], &mut budget).unwrap();
    let mut host = Host::default();
    host.bind(&references(&source)[0], vec![target]);
    assert_error(
        &catalog.select(&caller, &mut host, &mut budget).unwrap(),
        "function-target-required",
    );
}

#[test]
fn expression_and_inline_payloads_are_rejected_but_quoted_reference_is_literal() {
    for slot in ["{$ chosen}", "{function @name=f}"] {
        let source = source(&format!("{{behaviors | {{behavior @name=caller @implementation=function @function={slot}}}}}"), "local");
        let mut budget = budget();
        let catalog = FunctionCatalog::collect(&[source.clone()], &mut budget).unwrap();
        let mut host = Host::default();
        let error = catalog
            .select(&nodes(&source, "behavior")[0], &mut host, &mut budget)
            .unwrap_err();
        assert_eq!(error.code, "function-slot-reference-required");
        assert_eq!(host.calls, 0);
    }
    let source = source(
        "{behaviors | {behavior @name=caller @implementation=function @function=\"{#chosen}\"}}",
        "local",
    );
    let mut budget = budget();
    let catalog = FunctionCatalog::collect(&[source.clone()], &mut budget).unwrap();
    let mut host = Host::default();
    assert!(catalog
        .select(&nodes(&source, "behavior")[0], &mut host, &mut budget)
        .is_err());
    assert_eq!(host.calls, 0);
}

#[test]
fn literal_inline_shadows_reusable_and_ambiguous_reuse_fails() {
    for inline in [true, false] {
        let own = if inline {
            "{function @name=f @returns=string}"
        } else {
            ""
        };
        let source = source(&format!("{{behaviors | {{behavior @name=caller @implementation=function @function=f | {own}}} {{behavior @name=a | {{function @name=f @returns=string @visibility=package}}}} {{behavior @name=b | {{function @name=f @returns=string @visibility=public}}}}}}"), "local");
        let mut budget = budget();
        let catalog = FunctionCatalog::collect(&[source.clone()], &mut budget).unwrap();
        let mut host = Host::default();
        let result = catalog
            .select(&nodes(&source, "behavior")[0], &mut host, &mut budget)
            .unwrap();
        if inline {
            assert_eq!(
                result.target().unwrap().function().identity(),
                nodes(&source, "function")[0].identity()
            );
        } else {
            assert_error(&result, "function-ambiguous");
        }
        assert_eq!(host.calls, 0, "literal lookup never evaluates a query");
    }
}

#[test]
fn visibility_is_independent_of_crossing_grants_and_equal_namespace_strings() {
    for visibility in ["private", "package", "public", "unknown"] {
        let caller_source = source(
            "{behaviors | {behavior @name=caller @implementation=function @function={#chosen}}}",
            "same",
        );
        let library = source(&format!("{{behaviors | {{behavior @name=library | {{function @name=f @returns=string @visibility={visibility}}}}}}}"), "same");
        let mut budget = budget();
        let catalog =
            FunctionCatalog::collect(&[caller_source.clone(), library.clone()], &mut budget)
                .unwrap();
        let mut host = Host::default();
        host.bind(&references(&caller_source)[0], nodes(&library, "function"));
        let caller = &nodes(&caller_source, "behavior")[0];
        let denied = catalog.select(caller, &mut host, &mut budget).unwrap();
        assert!(denied.target().is_none());
        assert!(denied
            .resolution
            .issues
            .iter()
            .any(|i| i.kind == ReferenceResolutionIssueKind::ScopeDenied));
        host.crossing = true;
        let selected = catalog.select(caller, &mut host, &mut budget).unwrap();
        assert_eq!(
            selected.target().is_some(),
            visibility == "public",
            "{selected:?}"
        );
    }
}

#[test]
fn package_function_is_reusable_only_inside_original_schema() {
    for native in [false, true] {
        let slot = if native { "{#chosen}" } else { "f" };
        let source = source(&format!("{{behaviors | {{behavior @name=caller @implementation=function @function={slot}}} {{behavior @name=library | {{function @name=f @returns=string @visibility=package}}}}}}"), "local");
        let mut budget = budget();
        let catalog = FunctionCatalog::collect(&[source.clone()], &mut budget).unwrap();
        let mut host = Host::default();
        if native {
            host.bind(&references(&source)[0], nodes(&source, "function"));
        }
        assert!(catalog
            .select(&nodes(&source, "behavior")[0], &mut host, &mut budget)
            .unwrap()
            .target()
            .is_some());
    }
}

#[test]
fn qualified_literals_use_original_aliases_explicit_exports_and_grants() {
    let mut caller = source(
        "{behaviors | {behavior @name=caller @implementation=function @function=lib:f}}",
        "caller",
    );
    caller.bindings.insert("lib".into(), "original".into());
    let original = source("{behaviors | {behavior @name=library | {function @name=f @returns=string @visibility=public}}}", "original");
    let mut other = source("{behaviors | {behavior @name=other}}", "consumer");
    other.bindings.insert("lib".into(), "wrong".into());
    let mut budget = budget();
    let mut catalog =
        FunctionCatalog::collect(&[caller.clone(), original.clone(), other], &mut budget).unwrap();
    let behavior = &nodes(&caller, "behavior")[0];
    let mut host = Host::default();
    assert!(catalog
        .select(behavior, &mut host, &mut budget)
        .unwrap()
        .target()
        .is_none());
    catalog
        .export(&nodes(&original, "function")[0], &mut budget)
        .unwrap();
    assert!(catalog
        .select(behavior, &mut host, &mut budget)
        .unwrap()
        .target()
        .is_none());
    host.crossing = true;
    let selected = catalog.select(behavior, &mut host, &mut budget).unwrap();
    assert_eq!(
        selected.target().unwrap().function().identity(),
        nodes(&original, "function")[0].identity()
    );
    assert_eq!(host.calls, 0);
}

#[test]
fn foreign_unregistered_owner_does_not_bind_by_equal_name() {
    let caller = source("{behaviors | {behavior @name=caller @implementation=function @function={#chosen} | {function @name=f @returns=string}}}", "local");
    let foreign = source("{behaviors | {behavior @name=library | {function @name=f @returns=string @visibility=public}}}", "local");
    let mut budget = budget();
    let catalog = FunctionCatalog::collect(&[caller.clone()], &mut budget).unwrap();
    let mut host = Host {
        crossing: true,
        ..Host::default()
    };
    host.bind(&references(&caller)[0], nodes(&foreign, "function"));
    let result = catalog
        .select(&nodes(&caller, "behavior")[0], &mut host, &mut budget)
        .unwrap();
    assert!(result.target().is_none());
    assert!(result
        .resolution
        .issues
        .iter()
        .any(|i| i.reason == "function-source-unavailable"));
}

#[test]
fn incomplete_behavior_collection_cannot_prove_missing_function() {
    let source = source(
        "{behaviors | {behavior @name=caller @implementation=function @function=missing} {#more}}",
        "local",
    );
    let mut budget = budget();
    let catalog = FunctionCatalog::collect(&[source.clone()], &mut budget).unwrap();
    let result = catalog
        .select(
            &nodes(&source, "behavior")[0],
            &mut Host::default(),
            &mut budget,
        )
        .unwrap();
    assert_eq!(result.resolution.state, ReferenceResolutionState::Pending);
    assert!(result.target().is_none());
}

#[test]
fn collection_and_repeated_selections_share_finite_work() {
    let source = source("{behaviors | {behavior @name=caller @implementation=function @function=f | {function @name=f @returns=string}}}", "local");
    let mut budget = FunctionSelectionBudget::new(ReferenceTraversalLimits {
        max_depth: 8,
        max_work: 100,
    })
    .unwrap();
    let catalog = FunctionCatalog::collect(&[source.clone()], &mut budget).unwrap();
    let before = budget.work_used();
    let mut host = Host::default();
    let mut exhausted = false;
    for _ in 0..100 {
        match catalog.select(&nodes(&source, "behavior")[0], &mut host, &mut budget) {
            Ok(result) if result.target().is_some() => {}
            _ => {
                exhausted = true;
                break;
            }
        }
    }
    assert!(exhausted);
    assert!(budget.work_used() > before);
    assert!(budget.work_used() <= 100);
}

#[test]
fn distinct_targets_and_invalid_partial_targets_keep_original_diagnostics() {
    let source = source("{behaviors | {behavior @name=caller @implementation=function @function={#chosen} | {function @name=a @returns=string} {function @name=b @returns=string} {type @name=wrong}}} {#pending}", "local");
    let caller = nodes(&source, "behavior").remove(0);
    let refs = references(&source);
    let mut budget = budget();
    let catalog = FunctionCatalog::collect(&[source.clone()], &mut budget).unwrap();
    let mut host = Host::default();
    host.bind(&refs[0], nodes(&source, "function"));
    let multiple = catalog.select(&caller, &mut host, &mut budget).unwrap();
    assert_error(&multiple, "function-cardinality");
    let wrong = nodes(&source, "type").remove(0);
    host.bind(&refs[0], vec![wrong.clone(), refs[1].clone()]);
    let partial = catalog.select(&caller, &mut host, &mut budget).unwrap();
    assert_error(&partial, "function-target-required");
    assert!(partial
        .resolution
        .issues
        .iter()
        .any(|i| i.kind == ReferenceResolutionIssueKind::Pending));
    let diagnostic = partial
        .resolution
        .diagnostics
        .iter()
        .find(|d| d.code.ends_with("function-target-required"))
        .unwrap();
    assert_eq!(
        diagnostic.node.as_deref(),
        Some(partial.attribute.identity().as_str())
    );
    assert_eq!(
        diagnostic.details.as_ref().unwrap()["target"],
        wrong.identity()
    );
    assert!(diagnostic.source_map.is_some());
}

#[test]
fn duplicate_binding_fields_and_engine_function_slots_fail_before_evaluation() {
    for fields in [
        "@implementation=function @function={#a} @function={#b}",
        "@implementation=engine @function={#a}",
    ] {
        let source = source(
            &format!("{{behaviors | {{behavior @name=caller {fields}}}}}"),
            "local",
        );
        let mut budget = budget();
        let catalog = FunctionCatalog::collect(&[source.clone()], &mut budget).unwrap();
        let mut host = Host::default();
        assert!(catalog
            .select(&nodes(&source, "behavior")[0], &mut host, &mut budget)
            .is_err());
        assert_eq!(host.calls, 0);
    }
}

#[test]
fn missing_declaring_alias_never_borrows_another_schema_binding() {
    let caller = source(
        "{behaviors | {behavior @name=caller @implementation=function @function=lib:f}}",
        "caller",
    );
    let mut consumer = source("{behaviors | {behavior @name=library | {function @name=f @returns=string @visibility=public}}}", "library");
    consumer.bindings.insert("lib".into(), "library".into());
    let mut budget = budget();
    let mut catalog =
        FunctionCatalog::collect(&[caller.clone(), consumer.clone()], &mut budget).unwrap();
    catalog
        .export(&nodes(&consumer, "function")[0], &mut budget)
        .unwrap();
    let selected = catalog
        .select(
            &nodes(&caller, "behavior")[0],
            &mut Host {
                crossing: true,
                ..Host::default()
            },
            &mut budget,
        )
        .unwrap();
    assert!(selected.target().is_none());
    assert!(selected
        .resolution
        .issues
        .iter()
        .any(|i| i.reason == "unknown-function-prefix"));
}

#[test]
fn explicit_exports_reject_private_members_and_do_not_merge_distinct_owners() {
    let private = source(
        "{behaviors | {behavior @name=library | {function @name=f @returns=string}}}",
        "library",
    );
    let mut budget = budget();
    let mut catalog = FunctionCatalog::collect(&[private.clone()], &mut budget).unwrap();
    assert_eq!(
        catalog
            .export(&nodes(&private, "function")[0], &mut budget)
            .unwrap_err()
            .code,
        "function-export-not-public"
    );
    let mut caller = source(
        "{behaviors | {behavior @name=caller @implementation=function @function=lib:f}}",
        "caller",
    );
    caller.bindings.insert("lib".into(), "library".into());
    let a = source("{behaviors | {behavior @name=library | {function @name=f @returns=string @visibility=public}}}", "library");
    let b = source("{behaviors | {behavior @name=library | {function @name=f @returns=string @visibility=public}}}", "library");
    let mut catalog =
        FunctionCatalog::collect(&[caller.clone(), a.clone(), b.clone()], &mut budget).unwrap();
    for source in [&a, &b] {
        catalog
            .export(&nodes(source, "function")[0], &mut budget)
            .unwrap();
    }
    assert_error(
        &catalog
            .select(
                &nodes(&caller, "behavior")[0],
                &mut Host {
                    crossing: true,
                    ..Host::default()
                },
                &mut budget,
            )
            .unwrap(),
        "function-ambiguous",
    );
}

#[test]
fn inline_lookup_does_not_inspect_shadowed_library_signatures() {
    let source = source("{behaviors | {behavior @name=caller @implementation=function @function=f | {function @name=f @returns=string}} {behavior @name=library | {function @name=f @returns=string @visibility=unknown}}}", "local");
    let mut budget = budget();
    let catalog = FunctionCatalog::collect(&[source.clone()], &mut budget).unwrap();
    let result = catalog
        .select(
            &nodes(&source, "behavior")[0],
            &mut Host::default(),
            &mut budget,
        )
        .unwrap();
    assert_eq!(
        result.target().unwrap().function().identity(),
        nodes(&source, "function")[0].identity()
    );
}
