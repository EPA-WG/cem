//! Adapt the same explicitly registered lifecycle scopes to DOM ID consumption.
use super::*;

pub struct CemQlElementReferenceHost<'a> {
    host: &'a mut CemQlSchemaDeclarationHost,
    requesting: DeclarationScope,
}
impl CemQlSchemaDeclarationHost {
    /// Contexts, lexical captures, policies and directed grants must already be
    /// supplied. This adapter registers no owners and creates no authority.
    pub fn element_reference_host(
        &mut self,
        requesting: DeclarationScope,
    ) -> Option<CemQlElementReferenceHost<'_>> {
        self.scope_record(requesting)?;
        Some(CemQlElementReferenceHost {
            host: self,
            requesting,
        })
    }
}
impl ReferenceResolutionHost for CemQlElementReferenceHost<'_> {
    type Node = Item;
    type Scope = Option<DeclarationScope>;
    fn scope(&self, node: &Item) -> Self::Scope {
        self.host
            .scope(&self.host.query_node(node.clone(), Some(self.requesting)))
    }
    fn scope_limits(&self, scope: &Self::Scope) -> ReferenceTraversalLimits {
        self.host.scope_limits(scope)
    }
    fn reference_occurrence(&self, node: &Item) -> Option<ReferenceOccurrence> {
        self.host
            .reference_occurrence(&self.host.query_node(node.clone(), Some(self.requesting)))
    }
    fn unresolved_policy(&self, node: &Item) -> &ReferenceUnresolvedPolicy {
        let scope = self.scope(node).and_then(|s| self.host.scope_record(s));
        &scope
            .map_or(&self.host.fallback_policy, |s| &s.policy)
            .unresolved
    }
    fn permits_edge(&self, from: &Item, to: &Item) -> bool {
        self.host.permits_edge(
            &self.host.query_node(from.clone(), Some(self.requesting)),
            &self.host.query_node(to.clone(), Some(self.requesting)),
        )
    }
    fn evaluate(&mut self, node: &Item) -> ReferenceLinkEvaluation<Item> {
        let node = self.host.query_node(node.clone(), Some(self.requesting));
        match self.host.evaluate(&node) {
            ReferenceLinkEvaluation::Resolved(nodes) => {
                let mut items = Vec::new();
                for node in nodes {
                    if let Some(query) = node.query {
                        items.push(query);
                    } else if let Some(source) = node.source {
                        let Some(tree) = self.host.source_tree(&source) else {
                            return ReferenceLinkEvaluation::Pending(
                                "target-owner-not-registered".into(),
                            );
                        };
                        let Some(node) =
                            crate::eval::RetainedCemNode::new(tree.clone(), source.node_id())
                        else {
                            return ReferenceLinkEvaluation::Pending(
                                "target-node-unavailable".into(),
                            );
                        };
                        items.push(node.query_item());
                    }
                }
                ReferenceLinkEvaluation::Resolved(items)
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
