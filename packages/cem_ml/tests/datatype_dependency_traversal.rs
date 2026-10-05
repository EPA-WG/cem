//! Bounded dependency consumption preserves original source, not executable types.
use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::{builder::CemAstBuilder, document::CemDocument, CemAstNode},
    schema::{
        datatype_registry::{
            traverse_native_datatype_dependencies, DatatypeDeferredReason, DatatypeDependency,
            DatatypeDependencyHost, DatatypeDependencyIssueKind, DatatypeDependencyValue,
            DatatypeRegistry, DatatypeSource,
        },
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        reference_policy::{ReferenceOccurrence, ReferenceUnresolvedPolicy},
        reference_traversal::ReferenceTraversalLimits,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
    value::reference_resolution::{
        ReferenceLinkEvaluation, ReferenceResolutionHost, ReferenceResolutionIssueKind,
    },
};
use std::{collections::BTreeMap, sync::Arc};

fn parse(text: &str) -> Arc<CemDocument> {
    let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), text.as_bytes().to_vec()),
    )))
    .build();
    assert!(
        document.diagnostics.is_empty(),
        "{:?}",
        document.diagnostics
    );
    Arc::new(document)
}
fn elements(owner: &Arc<CemDocument>, name: &str) -> Vec<SchemaDeclarationNode> {
    owner
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => {
                SchemaDeclarationNode::new(owner.clone(), *node_id)
            }
            _ => None,
        })
        .collect()
}
fn name(node: &SchemaDeclarationNode) -> &str {
    let CemAstNode::Element { attributes, .. } = node.node() else {
        panic!()
    };
    attributes
        .iter()
        .find_map(|id| match node.document().get(*id) {
            Some(CemAstNode::Attribute {
                expanded_name,
                value,
                ..
            }) if expanded_name.local_name == "name" => value.as_deref(),
            _ => None,
        })
        .unwrap()
}
fn limits(work: usize) -> ReferenceTraversalLimits {
    ReferenceTraversalLimits {
        max_depth: 128,
        max_work: work,
    }
}
struct Host {
    scopes: BTreeMap<String, String>,
    sources: BTreeMap<String, DatatypeSource>,
    bindings: BTreeMap<String, ReferenceLinkEvaluation<SchemaDeclarationNode>>,
    calls: Vec<String>,
    literal_calls: Vec<(String, String, String)>,
    bounds: BTreeMap<String, ReferenceTraversalLimits>,
    policy: ReferenceUnresolvedPolicy,
    crossing: bool,
}
impl Host {
    fn new() -> Self {
        Self {
            scopes: BTreeMap::new(),
            sources: BTreeMap::new(),
            bindings: BTreeMap::new(),
            calls: vec![],
            literal_calls: vec![],
            bounds: BTreeMap::new(),
            policy: ReferenceUnresolvedPolicy::schema_defaults().unwrap(),
            crossing: false,
        }
    }
    fn add(&mut self, owner: &Arc<CemDocument>, scope_id: &str) -> Vec<DatatypeSource> {
        self.bounds.entry(scope_id.into()).or_insert(limits(1000));
        for index in 0..owner.nodes.len() {
            if let Some(node) = SchemaDeclarationNode::new(owner.clone(), index as u32) {
                self.scopes.insert(node.identity(), scope_id.into());
            }
        }
        let scope = elements(owner, "schema").remove(0);
        let declarations = elements(owner, "type");
        let mut registry = DatatypeRegistry::default();
        for declaration in &declarations {
            registry.insert(scope.clone(), declaration.clone()).unwrap();
        }
        declarations
            .iter()
            .map(|declaration| {
                let source = registry.source(&scope, name(declaration)).unwrap();
                self.sources.insert(declaration.identity(), source.clone());
                source
            })
            .collect()
    }
    fn reference(source: &DatatypeSource, field: &str) -> SchemaDeclarationNode {
        let plan = source.plan();
        plan.dependencies
            .iter()
            .find_map(|edge| {
                let CemAstNode::Attribute { expanded_name, .. } = edge.attribute.node() else {
                    panic!()
                };
                match &edge.value {
                    DatatypeDependencyValue::Native(reference)
                        if expanded_name.local_name == field =>
                    {
                        Some(reference.clone())
                    }
                    _ => None,
                }
            })
            .unwrap()
    }
    fn bind(&mut self, source: &DatatypeSource, field: &str, targets: Vec<SchemaDeclarationNode>) {
        self.bindings.insert(
            Self::reference(source, field).identity(),
            ReferenceLinkEvaluation::Resolved(targets),
        );
    }
}
impl ReferenceResolutionHost for Host {
    type Node = SchemaDeclarationNode;
    type Scope = String;
    fn scope(&self, node: &Self::Node) -> String {
        self.scopes[&node.identity()].clone()
    }
    fn scope_limits(&self, scope: &String) -> ReferenceTraversalLimits {
        self.bounds[scope]
    }
    fn reference_occurrence(&self, node: &Self::Node) -> Option<ReferenceOccurrence> {
        match node.node() {
            CemAstNode::Reference {
                node_id,
                expression,
                source,
                ..
            } => Some(ReferenceOccurrence {
                identity: node.identity(),
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
    fn permits_edge(&self, reference: &Self::Node, target: &Self::Node) -> bool {
        self.scope(reference) == self.scope(target) || self.crossing
    }
    fn evaluate(&mut self, reference: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        self.calls.push(reference.identity());
        self.bindings
            .get(&reference.identity())
            .cloned()
            .unwrap_or_else(|| ReferenceLinkEvaluation::Pending("context unavailable".into()))
    }
}
impl SchemaDeclarationHost for Host {
    fn source_reference(&self, source: SchemaDeclarationNode) -> Self::Node {
        source
    }
    fn declaration_node(&self, node: &Self::Node) -> Option<SchemaDeclarationNode> {
        Some(node.clone())
    }
    fn declaration_schema(&self, target: &SchemaDeclarationNode) -> Option<SchemaDeclarationNode> {
        self.sources
            .get(&target.identity())
            .map(|source| source.scope().clone())
    }
}
impl DatatypeDependencyHost for Host {
    fn lookup_literal_type(
        &mut self,
        source: &DatatypeSource,
        dependency: &DatatypeDependency,
        qname: &str,
    ) -> ReferenceLinkEvaluation<Self::Node> {
        self.literal_calls.push((
            source.scope().identity(),
            dependency.attribute.identity(),
            qname.into(),
        ));
        self.bindings
            .get(&dependency.attribute.identity())
            .cloned()
            .unwrap_or_else(|| {
                ReferenceLinkEvaluation::Pending("literal lookup unavailable".into())
            })
    }
    fn datatype_source(&self, target: &SchemaDeclarationNode) -> Option<DatatypeSource> {
        self.sources.get(&target.identity()).cloned()
    }
}

#[test]
fn native_base_and_rule_forest_preserves_fields_and_original_targets() {
    let owner = parse("{schema | {type @name=root @kind=scalar @base={#base} @rule={#rule}} {type @name=base @kind=scalar @base={#leaf}} {type @name=leaf @kind=scalar} {behavior @name=rule}}");
    let mut host = Host::new();
    let sources = host.add(&owner, "given");
    host.bind(&sources[0], "base", vec![sources[1].declaration().clone()]);
    host.bind(&sources[1], "base", vec![sources[2].declaration().clone()]);
    host.bind(&sources[0], "rule", elements(&owner, "behavior"));
    let report =
        traverse_native_datatype_dependencies(sources[0].clone(), &mut host, limits(100)).unwrap();
    assert!(report.is_complete());
    assert!(!report.failed());
    assert_eq!(host.calls.len(), 3);
    assert_eq!(report.walk.resolution.work_used, 10);
    assert_eq!(report.sites.len(), 3);
    assert!(report.sites.iter().all(|site| site.targets.len() == 1));
    for site in &report.sites {
        assert!(Arc::ptr_eq(site.attribute.document(), &owner));
        assert!(site
            .targets
            .iter()
            .all(|target| Arc::ptr_eq(target.document(), &owner)));
    }
    assert_eq!(report.plans.len(), 3);
}

#[test]
fn unavailable_literal_lookup_stays_pending_without_native_evaluation() {
    let owner =
        parse("{schema | {type @name=root @base=vendor:integer @rule='integer description'}}");
    let mut host = Host::new();
    let root = host.add(&owner, "given").remove(0);
    let report =
        traverse_native_datatype_dependencies(root.clone(), &mut host, limits(100)).unwrap();
    assert!(!report.is_complete());
    assert!(!report.failed());
    assert!(host.calls.is_empty());
    assert_eq!(report.walk.resolution.work_used, 3);
    assert!(report.deferred.is_empty());
    assert_eq!(host.literal_calls[0].0, root.scope().identity());
    assert_eq!(host.literal_calls[0].2, "vendor:integer");
    assert!(!report.sites[0].complete);
    assert_eq!(
        report.sites[0].attribute.identity(),
        root.attribute("base").unwrap().identity()
    );
}

#[test]
fn native_dependency_cycles_share_the_active_reference_stack() {
    let owner = parse("{schema | {type @name=root @kind=scalar @base={#base}} {type @name=base @kind=scalar @base={#root}}}");
    let mut host = Host::new();
    let sources = host.add(&owner, "given");
    host.bind(&sources[0], "base", vec![sources[1].declaration().clone()]);
    host.bind(&sources[1], "base", vec![sources[0].declaration().clone()]);
    let report =
        traverse_native_datatype_dependencies(sources[0].clone(), &mut host, limits(100)).unwrap();
    assert!(!report.is_complete());
    assert_eq!(host.calls.len(), 2);
    assert!(report
        .walk
        .resolution
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::Cycle));
}

#[test]
fn request_budget_is_not_reset_for_each_dependency_field() {
    let owner = parse("{schema | {type @name=root @kind=scalar @base={#base}} {type @name=base @kind=scalar @base={#leaf}} {type @name=leaf @kind=scalar}}");
    let mut host = Host::new();
    let sources = host.add(&owner, "given");
    host.bind(&sources[0], "base", vec![sources[1].declaration().clone()]);
    host.bind(&sources[1], "base", vec![sources[2].declaration().clone()]);
    let report =
        traverse_native_datatype_dependencies(sources[0].clone(), &mut host, limits(4)).unwrap();
    assert!(!report.is_complete());
    assert_eq!(report.walk.resolution.work_used, 4);
    assert_eq!(host.calls.len(), 1);
    assert!(report
        .walk
        .resolution
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::WorkLimit));
}

#[test]
fn destination_limits_and_explicit_crossing_grants_remain_effective() {
    for granted in [false, true] {
        let owner = parse("{schema | {type @name=root @kind=scalar @base={#base}}}");
        let vendor = parse("{schema | {type @name=base @kind=scalar @base={#leaf}} {type @name=leaf @kind=scalar}}");
        let mut host = Host::new();
        let root = host.add(&owner, "request").remove(0);
        let targets = host.add(&vendor, "destination");
        host.bounds.insert("destination".into(), limits(2));
        host.crossing = granted;
        host.bind(&root, "base", vec![targets[0].declaration().clone()]);
        host.bind(&targets[0], "base", vec![targets[1].declaration().clone()]);
        let report = traverse_native_datatype_dependencies(root, &mut host, limits(100)).unwrap();
        assert!(!report.is_complete());
        assert_eq!(host.calls.len(), 1);
        let expected = if granted {
            ReferenceResolutionIssueKind::WorkLimit
        } else {
            ReferenceResolutionIssueKind::ScopeDenied
        };
        assert!(report
            .walk
            .resolution
            .issues
            .iter()
            .any(|issue| issue.kind == expected));
    }
}

#[test]
fn pending_selection_is_not_a_complete_empty_selection() {
    for pending in [true, false] {
        let owner = parse("{schema | {type @name=root @kind=scalar @base={#base}}}");
        let mut host = Host::new();
        let root = host.add(&owner, "given").remove(0);
        if !pending {
            host.bind(&root, "base", vec![]);
        }
        let report = traverse_native_datatype_dependencies(root, &mut host, limits(100)).unwrap();
        assert!(!report.is_complete());
        assert_eq!(
            report
                .issues
                .iter()
                .any(|issue| issue.kind == DatatypeDependencyIssueKind::Cardinality),
            !pending
        );
        if !pending {
            assert!(report.failed());
        }
    }
}

#[test]
fn multiple_and_wrong_kind_targets_are_attributed_to_original_fields() {
    let owner = parse("{schema | {type @name=root @kind=scalar @base={#base} @rule={#rule}} {type @name=base @kind=scalar} {type @name=other @kind=scalar} {behavior @name=rule}}");
    let mut host = Host::new();
    let sources = host.add(&owner, "given");
    host.bind(
        &sources[0],
        "base",
        sources[1..]
            .iter()
            .map(|source| source.declaration().clone())
            .collect(),
    );
    host.bind(&sources[0], "rule", vec![sources[1].declaration().clone()]);
    let report =
        traverse_native_datatype_dependencies(sources[0].clone(), &mut host, limits(100)).unwrap();
    assert!(report.failed());
    assert!(!report.is_complete());
    let cardinality = report
        .issues
        .iter()
        .find(|issue| issue.kind == DatatypeDependencyIssueKind::Cardinality)
        .unwrap();
    assert_eq!(
        cardinality.attribute.identity(),
        sources[0].attribute("base").unwrap().identity()
    );
    let wrong = report
        .issues
        .iter()
        .find(|issue| issue.kind == DatatypeDependencyIssueKind::WrongTarget)
        .unwrap();
    assert_eq!(
        wrong.attribute.identity(),
        sources[0].attribute("rule").unwrap().identity()
    );
}

#[test]
fn missing_declaring_source_context_remains_deferred() {
    let owner = parse("{schema | {type @name=root @kind=scalar @base={#base}} {type @name=base @kind=scalar @base={#hidden}}}");
    let mut host = Host::new();
    let sources = host.add(&owner, "given");
    host.bind(&sources[0], "base", vec![sources[1].declaration().clone()]);
    host.sources.remove(&sources[1].declaration().identity());
    let report =
        traverse_native_datatype_dependencies(sources[0].clone(), &mut host, limits(100)).unwrap();
    assert!(!report.is_complete());
    assert!(report
        .deferred
        .iter()
        .any(|issue| issue.reason == DatatypeDeferredReason::MissingSource));
    assert_eq!(host.calls.len(), 1);
}

#[test]
fn invalid_rule_target_does_not_traverse_unrelated_datatype_dependencies() {
    let owner = parse("{schema | {type @name=root @kind=scalar @rule={#rule}} {type @name=wrong @kind=scalar @base={#hidden}} {type @name=hidden @kind=scalar}}");
    let mut host = Host::new();
    let sources = host.add(&owner, "given");
    host.bind(&sources[0], "rule", vec![sources[1].declaration().clone()]);
    host.bind(&sources[1], "base", vec![sources[2].declaration().clone()]);
    let report =
        traverse_native_datatype_dependencies(sources[0].clone(), &mut host, limits(100)).unwrap();
    assert!(report.failed());
    assert!(!report.is_complete());
    assert_eq!(host.calls.len(), 1);
    assert_eq!(report.plans.len(), 1);
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.kind == DatatypeDependencyIssueKind::WrongTarget));
}

#[test]
fn mixed_dependencies_use_original_scope_and_share_one_budget() {
    let owner = parse("{schema | {type @name=root @base=vendor:base} {type @name=base @base={#leaf}} {type @name=leaf @kind=scalar}}");
    let mut host = Host::new();
    let sources = host.add(&owner, "given");
    host.bindings.insert(
        sources[0].attribute("base").unwrap().identity(),
        ReferenceLinkEvaluation::Resolved(vec![sources[1].declaration().clone()]),
    );
    host.bind(&sources[1], "base", vec![sources[2].declaration().clone()]);
    let report =
        traverse_native_datatype_dependencies(sources[0].clone(), &mut host, limits(100)).unwrap();
    assert!(report.is_complete());
    assert_eq!(report.sites.len(), 2);
    assert_eq!(host.literal_calls[0].0, sources[0].scope().identity());
    assert_eq!(host.literal_calls[0].2, "vendor:base");
    assert_eq!(host.calls.len(), 1);
    let bounded =
        traverse_native_datatype_dependencies(sources[0].clone(), &mut host, limits(4)).unwrap();
    assert!(!bounded.is_complete());
    assert!(bounded
        .walk
        .resolution
        .issues
        .iter()
        .any(|i| i.kind == ReferenceResolutionIssueKind::WorkLimit));
}

#[test]
fn literal_cycles_and_crossings_use_native_policy() {
    let owner = parse("{schema | {type @name=root @base=other:root}}");
    let other = parse("{schema | {type @name=root @base=first:root}}");
    let mut host = Host::new();
    let root = host.add(&owner, "first").remove(0);
    let destination = host.add(&other, "other").remove(0);
    host.bindings.insert(
        root.attribute("base").unwrap().identity(),
        ReferenceLinkEvaluation::Resolved(vec![destination.declaration().clone()]),
    );
    host.bindings.insert(
        destination.attribute("base").unwrap().identity(),
        ReferenceLinkEvaluation::Resolved(vec![root.declaration().clone()]),
    );
    let denied =
        traverse_native_datatype_dependencies(root.clone(), &mut host, limits(100)).unwrap();
    assert!(denied
        .walk
        .resolution
        .issues
        .iter()
        .any(|i| i.kind == ReferenceResolutionIssueKind::ScopeDenied));
    host.crossing = true;
    host.bounds.insert("other".into(), limits(2));
    let bounded =
        traverse_native_datatype_dependencies(root.clone(), &mut host, limits(100)).unwrap();
    assert!(bounded
        .walk
        .resolution
        .issues
        .iter()
        .any(|i| i.kind == ReferenceResolutionIssueKind::WorkLimit));
    host.bounds.insert("other".into(), limits(100));
    let cycle =
        traverse_native_datatype_dependencies(root.clone(), &mut host, limits(100)).unwrap();
    assert!(cycle
        .walk
        .resolution
        .issues
        .iter()
        .any(|i| i.kind == ReferenceResolutionIssueKind::Cycle));
    assert!(
        host.literal_calls
            .iter()
            .any(|(scope, _, qname)| scope == &destination.scope().identity()
                && qname == "first:root")
    );
}

#[test]
fn literal_complete_empty_multiple_and_wrong_targets_keep_field_provenance() {
    let owner = parse("{schema | {type @name=root @base=base} {type @name=base @kind=scalar} {behavior @name=wrong}}");
    let mut host = Host::new();
    let sources = host.add(&owner, "given");
    let attribute = sources[0].attribute("base").unwrap().clone();
    for targets in [
        vec![],
        vec![sources[1].declaration().clone(); 2],
        elements(&owner, "behavior"),
    ] {
        host.bindings.insert(
            attribute.identity(),
            ReferenceLinkEvaluation::Resolved(targets),
        );
        let report =
            traverse_native_datatype_dependencies(sources[0].clone(), &mut host, limits(100))
                .unwrap();
        assert!(report.failed());
        assert!(report
            .issues
            .iter()
            .all(|issue| issue.attribute.identity() == attribute.identity()));
        assert!(Arc::ptr_eq(report.sites[0].attribute.document(), &owner));
    }
}

#[test]
fn imported_declaration_literal_lookup_keeps_its_declaring_scope() {
    let importer =
        parse("{schema | {type @name=root @base={#imported}} {type @name=leaf @kind=scalar}}");
    let vendor =
        parse("{schema | {type @name=imported @base=local:leaf} {type @name=leaf @kind=scalar}}");
    let mut host = Host::new();
    let local = host.add(&importer, "importer");
    let foreign = host.add(&vendor, "vendor");
    host.crossing = true;
    host.bind(&local[0], "base", vec![foreign[0].declaration().clone()]);
    let attribute = foreign[0].attribute("base").unwrap().clone();
    host.bindings.insert(
        attribute.identity(),
        ReferenceLinkEvaluation::Resolved(vec![foreign[1].declaration().clone()]),
    );
    let report =
        traverse_native_datatype_dependencies(local[0].clone(), &mut host, limits(100)).unwrap();
    assert!(report.is_complete());
    assert_eq!(
        host.literal_calls,
        vec![(
            foreign[0].scope().identity(),
            attribute.identity(),
            "local:leaf".into()
        )]
    );
    assert_eq!(
        report.sites[1].targets[0].identity(),
        foreign[1].declaration().identity()
    );
    assert_ne!(
        report.sites[1].targets[0].identity(),
        local[1].declaration().identity()
    );
    assert!(
        matches!(attribute.node(), CemAstNode::Attribute { value: Some(value), value_nodes, .. } if value == "local:leaf" && value_nodes.is_empty())
    );

    let bounded = traverse_native_datatype_dependencies(
        local[0].clone(),
        &mut host,
        ReferenceTraversalLimits {
            max_depth: 1,
            max_work: 100,
        },
    )
    .unwrap();
    assert!(!bounded.is_complete());
    assert!(bounded.walk.resolution.issues.iter().any(|issue| issue.kind
        == ReferenceResolutionIssueKind::DepthLimit
        && issue.occurrence.node_id == Some(attribute.node_id())));
}
