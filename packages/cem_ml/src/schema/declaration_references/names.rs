//! Bounded metadata readiness over an original declaration's owning subtree.
//! References retain their authored structure; target/context edges are not walked.
use super::{SchemaDeclarationHost, SchemaDeclarationNode};
use crate::{parser::CemAstNode, schema::reference_traversal::ReferenceTraversalLimits};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclarationNameIssueKind {
    Pending,
    WorkLimit,
}

#[derive(Debug, Clone)]
pub struct DeclarationNameIssue {
    pub source: SchemaDeclarationNode,
    pub kind: DeclarationNameIssueKind,
}

/// The compilation-wide metadata budget survives checks of foreign targets.
/// A destination can narrow a pass; it cannot replenish the requesting budget.
pub(super) fn check<H: SchemaDeclarationHost>(
    source: &SchemaDeclarationNode,
    host: &H,
    limits: ReferenceTraversalLimits,
    used: &mut usize,
) -> Result<(), DeclarationNameIssue> {
    if !host.declaration_name_metadata_required(source) {
        return Ok(());
    }
    let native = host.source_reference(source.clone());
    let cap = host
        .scope_limits(&host.scope(&native))
        .max_work
        .min(limits.max_work.saturating_sub(*used));
    let mut local = 0;
    let mut visited = BTreeSet::new();
    let mut pending = vec![source.node_id()];
    while let Some(id) = pending.pop() {
        if local >= cap || !visited.insert(id) {
            return Err(DeclarationNameIssue {
                source: source.clone(),
                kind: DeclarationNameIssueKind::WorkLimit,
            });
        }
        local += 1;
        *used += 1;
        let Some(node) = SchemaDeclarationNode::new(source.document().clone(), id) else {
            return Err(DeclarationNameIssue {
                source: source.clone(),
                kind: DeclarationNameIssueKind::Pending,
            });
        };
        match node.node() {
            CemAstNode::Element {
                attributes,
                children,
                expanded_name,
                ..
            } => {
                // Intrinsic expression wrappers have no authored QName.
                if expanded_name.local_name != "$" || !expanded_name.namespace_uri.is_empty() {
                    if host.input_expanded_name(&node).is_none() {
                        return Err(DeclarationNameIssue {
                            source: node,
                            kind: DeclarationNameIssueKind::Pending,
                        });
                    }
                }
                if pending
                    .len()
                    .saturating_add(attributes.len())
                    .saturating_add(children.len())
                    > cap - local
                {
                    return Err(DeclarationNameIssue {
                        source: node,
                        kind: DeclarationNameIssueKind::WorkLimit,
                    });
                }
                pending.extend(children.iter().rev());
                pending.extend(attributes.iter().rev());
            }
            CemAstNode::Attribute {
                expanded_name,
                value_nodes,
                ..
            } => {
                if expanded_name.namespace_uri == "xmlns"
                    && !value_nodes.is_empty()
                    && !host.declaration_namespace_is_ready(&node)
                {
                    return Err(DeclarationNameIssue {
                        source: node,
                        kind: DeclarationNameIssueKind::Pending,
                    });
                }
                // xmlns names are reserved metadata, independent of their URI value.
                if expanded_name.namespace_uri != "xmlns"
                    && host.input_expanded_name(&node).is_none()
                {
                    return Err(DeclarationNameIssue {
                        source: node,
                        kind: DeclarationNameIssueKind::Pending,
                    });
                }
                if pending.len().saturating_add(value_nodes.len()) > cap - local {
                    return Err(DeclarationNameIssue {
                        source: node,
                        kind: DeclarationNameIssueKind::WorkLimit,
                    });
                }
                pending.extend(value_nodes.iter().rev());
            }
            _ => {}
        }
    }
    Ok(())
}
