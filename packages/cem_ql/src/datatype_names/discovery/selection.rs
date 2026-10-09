//! Consumer containers carry traversal roles; selected values never acquire a
//! container role, and descendant references in selected declarations stay authored.
use super::*;
use crate::schema_references::CemQlSchemaReferenceNode;
use cem_ml::{
    schema::reference_policy::{ReferenceOccurrence, ReferenceUnresolvedPolicy},
    value::reference_resolution::{
        resolve_owned_reference_structure, ReferenceLinkEvaluation, ReferenceResolutionHost,
        ReferenceResolutionIssueKind,
    },
};
#[derive(Clone)]
enum Node {
    Schema(CemQlSchemaReferenceNode),
    Collection(CemQlSchemaReferenceNode),
    Value(CemQlSchemaReferenceNode),
}
impl Node {
    fn value(&self) -> &CemQlSchemaReferenceNode {
        match self {
            Self::Schema(v) | Self::Collection(v) | Self::Value(v) => v,
        }
    }
}
struct Host<'a>(&'a mut CemQlSchemaDeclarationHost);
impl ReferenceResolutionHost for Host<'_> {
    type Node = Node;
    type Scope = <CemQlSchemaDeclarationHost as ReferenceResolutionHost>::Scope;
    fn scope(&self, node: &Node) -> Self::Scope {
        self.0.scope(node.value())
    }
    fn scope_limits(&self, scope: &Self::Scope) -> ReferenceTraversalLimits {
        self.0.scope_limits(scope)
    }
    fn reference_occurrence(&self, node: &Node) -> Option<ReferenceOccurrence> {
        match node {
            Node::Value(v) => self.0.reference_occurrence(v),
            _ => None,
        }
    }
    fn unresolved_policy(&self, node: &Node) -> &ReferenceUnresolvedPolicy {
        self.0.unresolved_policy(node.value())
    }
    fn permits_edge(&self, from: &Node, to: &Node) -> bool {
        self.0.permits_edge(from.value(), to.value())
    }
    fn evaluate(&mut self, node: &Node) -> ReferenceLinkEvaluation<Node> {
        match self.0.evaluate(node.value()) {
            ReferenceLinkEvaluation::Resolved(nodes) => {
                ReferenceLinkEvaluation::Resolved(nodes.into_iter().map(Node::Value).collect())
            }
            ReferenceLinkEvaluation::Pending(reason) => ReferenceLinkEvaluation::Pending(reason),
            ReferenceLinkEvaluation::Unresolved(reason) => {
                ReferenceLinkEvaluation::Unresolved(reason)
            }
            ReferenceLinkEvaluation::Invalid(diagnostics) => {
                ReferenceLinkEvaluation::Invalid(diagnostics)
            }
        }
    }
}
pub(super) fn collections(
    input: &DatatypeSchemaSource,
    host: &mut CemQlSchemaDeclarationHost,
    names: &mut DatatypeNameLimits,
    traversal: &mut ReferenceTraversalLimits,
) -> Result<(Vec<SchemaDeclarationNode>, Vec<SchemaDeclarationNode>), DatatypeNameError> {
    let mut collections = vec![];
    let mut slots = BTreeMap::new();
    let mut types = vec![];
    let mut uses = vec![];
    let mut has_references = false;
    for collection in children(&input.schema, names)? {
        if !matches!(collection.node(), CemAstNode::Element { .. }) {
            return Err(
                if matches!(collection.node(), CemAstNode::Reference { .. }) {
                    pending("datatype-schema-selection-unavailable", &collection)
                } else {
                    error("datatype-schema-content", &collection)
                },
            );
        }
        let is_types = named(host, &collection, "types")?;
        if !is_types && !named(host, &collection, "uses")? {
            continue;
        }
        let children = children(&collection, names)?;
        has_references |= children
            .iter()
            .any(|node| matches!(node.node(), CemAstNode::Reference { .. }));
        if is_types {
            types.extend(children.iter().cloned());
        } else {
            uses.extend(children.iter().cloned());
        }
        slots.insert(collection.identity(), (is_types, children));
        collections.push(collection);
    }
    if !has_references {
        return Ok((types, uses));
    }
    let limits = ReferenceTraversalLimits {
        max_work: traversal.max_work.min(names.max_work),
        ..*traversal
    };
    if limits.max_work == 0 {
        return Err(pending("datatype-selection-work-limit", &input.schema));
    }
    if limits.max_depth == 0 {
        return Err(error("datatype-selection-invalid-bounds", &input.schema));
    }
    let CemAstNode::Element { source, .. } = input.schema.node() else {
        unreachable!()
    };
    let origin = ReferenceOccurrence {
        identity: format!("datatype-collections:{}", input.schema.identity()),
        node_id: Some(input.schema.node_id()),
        expression: None,
        source_map: source.clone(),
    };
    let root = Node::Schema(host.source_reference(input.schema.clone()));
    let walk =
        resolve_owned_reference_structure(root, &mut Host(host), limits, origin, |host, node| {
            match node {
                Node::Schema(_) => Some(
                    collections
                        .iter()
                        .map(|n| Node::Collection(host.0.source_reference(n.clone())))
                        .collect(),
                ),
                Node::Collection(v) => {
                    let source = host.0.declaration_node(v)?;
                    Some(
                        slots
                            .get(&source.identity())?
                            .1
                            .iter()
                            .map(|n| Node::Value(host.0.source_reference(n.clone())))
                            .collect(),
                    )
                }
                Node::Value(_) => None,
            }
        })
        .map_err(|_| error("datatype-selection-invalid-bounds", &input.schema))?;
    traversal.max_work -= walk.resolution.work_used;
    names.max_work -= walk.resolution.work_used;
    if !walk.resolution.is_complete() || walk.resolution.failed {
        // A pending sibling must not hide a malformed or forbidden branch.
        let issue = walk
            .resolution
            .issues
            .iter()
            .find(|issue| {
                matches!(
                    issue.kind,
                    ReferenceResolutionIssueKind::Invalid
                        | ReferenceResolutionIssueKind::ScopeDenied
                        | ReferenceResolutionIssueKind::Cycle
                )
            })
            .or_else(|| walk.resolution.issues.first());
        let source = issue
            .and_then(|i| host.declaration_node(i.reference.value()))
            .unwrap_or_else(|| input.schema.clone());
        let (code, pending) = match issue.map(|i| i.kind) {
            Some(ReferenceResolutionIssueKind::Pending) => ("datatype-selection-pending", true),
            Some(ReferenceResolutionIssueKind::Unresolved) => {
                ("datatype-selection-unresolved", !walk.resolution.failed)
            }
            Some(ReferenceResolutionIssueKind::WorkLimit) => {
                ("datatype-selection-work-limit", true)
            }
            Some(ReferenceResolutionIssueKind::DepthLimit) => {
                ("datatype-selection-depth-limit", true)
            }
            Some(ReferenceResolutionIssueKind::Cycle) => ("datatype-selection-cycle", false),
            Some(ReferenceResolutionIssueKind::ScopeDenied) => {
                ("datatype-selection-scope-denied", false)
            }
            _ => ("datatype-selection-invalid", false),
        };
        return Err(DatatypeNameError {
            code,
            source,
            related: None,
            pending,
            diagnostics: walk.resolution.diagnostics,
        });
    }
    types.clear();
    uses.clear();
    for index in &walk.children[walk.roots[0]] {
        let collection = host
            .declaration_node(walk.resolution.nodes[*index].value())
            .unwrap();
        let output = if slots[&collection.identity()].0 {
            &mut types
        } else {
            &mut uses
        };
        for child in &walk.children[*index] {
            spend(names, 0, &collection)?;
            let source = host
                .declaration_node(walk.resolution.nodes[*child].value())
                .ok_or_else(|| error("datatype-selection-native-source-required", &collection))?;
            output.push(source);
        }
    }
    Ok((types, uses))
}
