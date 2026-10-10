//! A symbolic link is compiler metadata over an original slot, never an AST node.
use super::*;
use crate::{
    schema::reference_policy::ReferenceUnresolvedPolicy,
    value::reference_resolution::{ReferenceResolutionError, ReferenceResolutionHost},
};

#[derive(Clone)]
pub(super) enum Node<N> {
    Value(N),
    Literal(N),
}
impl<N> Node<N> {
    pub(super) fn value(&self) -> &N {
        match self {
            Self::Value(n) | Self::Literal(n) => n,
        }
    }
}
pub(super) struct Host<'a, H: SchemaDeclarationHost> {
    pub host: &'a mut H,
    pub literal: Option<ReferenceLinkEvaluation<H::Node>>,
    pub origin: ReferenceOccurrence,
}
impl<H: SchemaDeclarationHost> ReferenceResolutionHost for Host<'_, H> {
    type Node = Node<H::Node>;
    type Scope = H::Scope;
    fn prepare_node(&mut self, node: &mut Self::Node) -> Result<(), ReferenceResolutionError> {
        match node {
            Node::Value(n) | Node::Literal(n) => self.host.prepare_node(n),
        }
    }
    fn scope(&self, node: &Self::Node) -> Self::Scope {
        self.host.scope(node.value())
    }
    fn scope_limits(&self, scope: &Self::Scope) -> ReferenceTraversalLimits {
        self.host.scope_limits(scope)
    }
    fn reference_occurrence(&self, node: &Self::Node) -> Option<ReferenceOccurrence> {
        match node {
            Node::Literal(_) => Some(self.origin.clone()),
            Node::Value(n) => self.host.reference_occurrence(n),
        }
    }
    fn unresolved_policy(&self, node: &Self::Node) -> &ReferenceUnresolvedPolicy {
        self.host.unresolved_policy(node.value())
    }
    fn permits_edge(&self, a: &Self::Node, b: &Self::Node) -> bool {
        self.host.permits_edge(a.value(), b.value())
    }
    fn evaluate(&mut self, node: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        let value = match node {
            Node::Literal(_) => self
                .literal
                .take()
                .expect("one symbolic root per selection"),
            Node::Value(n) => self.host.evaluate(n),
        };
        match value {
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
