//! Explicit loader completion handoff. URL and public-part resolution belong
//! to the loader; consumption uses the ordinary bounded retained-node contract.
use super::{
    scope_preparation::{prepare_schema_scope_selection, SchemaPreparationHost},
    CemQlSchemaDeclarationHost, CemQlSchemaReferenceNode, SchemaScopePreparation,
};
use cem_ml::{
    parser::{AstNodeId, CemAstNode},
    schema::{
        declaration_references::SchemaDeclarationNode,
        reference_policy::{ReferenceOccurrence, ReferenceUnresolvedPolicy},
        reference_traversal::ReferenceTraversalLimits,
        scope_controls::{SchemaHostControl, SchemaHostSource},
    },
    value::reference_resolution::{
        resolve_reference, ReferenceLinkEvaluation, ReferenceResolution, ReferenceResolutionError,
        ReferenceResolutionHost, ReferenceResolutionIssue,
    },
};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaUriLoadBindingError {
    UnregisteredOwner,
    InvalidControl,
    EmptyUri,
}
impl std::fmt::Display for SchemaUriLoadBindingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Schema URI loader handoff: {self:?}")
    }
}
impl std::error::Error for SchemaUriLoadBindingError {}

pub(super) fn key(control: &SchemaDeclarationNode, uri: &str) -> (usize, AstNodeId, String) {
    (
        Arc::as_ptr(control.document()) as usize,
        control.node_id(),
        uri.to_owned(),
    )
}
impl CemQlSchemaDeclarationHost {
    /// Publish an explicit lifecycle snapshot for an original control and exact
    /// authored URI. The loader supplies selected original targets after any URL,
    /// base, redirect and public-part handling. Publication performs no evaluation,
    /// scope assignment or authorization. A later invocation consumes this snapshot;
    /// replacing it with Pending does not reuse the previous Ready selection.
    /// Manual publication supersedes any outstanding byte-acquisition generation.
    pub fn set_schema_uri_load(
        &mut self,
        control: &SchemaDeclarationNode,
        uri: &str,
        outcome: ReferenceLinkEvaluation<SchemaDeclarationNode>,
    ) -> Result<(), SchemaUriLoadBindingError> {
        if uri.trim().is_empty() {
            return Err(SchemaUriLoadBindingError::EmptyUri);
        }
        if !matches!(control.node(), CemAstNode::Element { .. }) {
            return Err(SchemaUriLoadBindingError::InvalidControl);
        }
        if !self
            .scopes
            .iter()
            .any(|scope| Arc::ptr_eq(scope.tree.ast_owner(), control.document()))
        {
            return Err(SchemaUriLoadBindingError::UnregisteredOwner);
        }
        self.schema_uri_generations.remove(&key(control, uri));
        self.schema_uri_loads.insert(key(control, uri), outcome);
        Ok(())
    }

    /// Remove this control/URI snapshot and supersede its outstanding acquisition.
    /// No source nodes or scopes change.
    pub fn clear_schema_uri_load(&mut self, control: &SchemaDeclarationNode, uri: &str) -> bool {
        self.schema_uri_generations.remove(&key(control, uri));
        self.schema_uri_loads.remove(&key(control, uri)).is_some()
    }

    pub(super) fn schema_uri_load_for_control(
        &self,
        control: &SchemaHostControl,
    ) -> Option<ReferenceLinkEvaluation<SchemaDeclarationNode>> {
        let SchemaHostSource::Uri(uri) = &control.source else {
            return None;
        };
        self.schema_uri_loads.get(&key(&control.host, uri)).cloned()
    }
}

// The root is an implicit consumer occurrence on the original control value,
// not a fabricated AST reference or a URL expression in the query evaluator.
#[derive(Clone)]
struct LoadedNode {
    node: CemQlSchemaReferenceNode,
    root: bool,
}
struct LoadedHost<'a, H> {
    host: &'a mut H,
    control: &'a SchemaHostControl,
    outcome: ReferenceLinkEvaluation<SchemaDeclarationNode>,
}
fn map_evaluation<A, B>(
    value: ReferenceLinkEvaluation<A>,
    mut map: impl FnMut(A) -> B,
) -> ReferenceLinkEvaluation<B> {
    match value {
        ReferenceLinkEvaluation::Resolved(nodes) => {
            ReferenceLinkEvaluation::Resolved(nodes.into_iter().map(&mut map).collect())
        }
        ReferenceLinkEvaluation::Pending(reason) => ReferenceLinkEvaluation::Pending(reason),
        ReferenceLinkEvaluation::Unresolved(reason) => ReferenceLinkEvaluation::Unresolved(reason),
        ReferenceLinkEvaluation::Invalid(diagnostics) => {
            ReferenceLinkEvaluation::Invalid(diagnostics)
        }
    }
}
impl<H: SchemaPreparationHost> ReferenceResolutionHost for LoadedHost<'_, H> {
    type Node = LoadedNode;
    type Scope = H::Scope;
    fn prepare_node(&mut self, node: &mut Self::Node) -> Result<(), ReferenceResolutionError> {
        self.host.prepare_node(&mut node.node)
    }
    fn scope(&self, node: &Self::Node) -> Self::Scope {
        self.host.scope(&node.node)
    }
    fn scope_limits(&self, scope: &Self::Scope) -> ReferenceTraversalLimits {
        self.host.scope_limits(scope)
    }
    fn reference_occurrence(&self, node: &Self::Node) -> Option<ReferenceOccurrence> {
        if !node.root {
            return self.host.reference_occurrence(&node.node);
        }
        let source = &self.control.attribute;
        let provenance = match source.node() {
            CemAstNode::Attribute { source, .. }
            | CemAstNode::Element { source, .. }
            | CemAstNode::Text { source, .. } => source,
            _ => unreachable!("decoded URI controls retain their original value handle"),
        };
        Some(ReferenceOccurrence {
            identity: source.identity(),
            node_id: Some(source.node_id()),
            expression: None,
            source_map: provenance.clone(),
        })
    }
    fn unresolved_policy(&self, node: &Self::Node) -> &ReferenceUnresolvedPolicy {
        self.host.unresolved_policy(&node.node)
    }
    fn permits_edge(&self, from: &Self::Node, to: &Self::Node) -> bool {
        self.host.permits_edge(&from.node, &to.node)
    }
    fn evaluate(&mut self, node: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        if node.root {
            map_evaluation(self.outcome.clone(), |source| LoadedNode {
                node: self.host.source_reference(source),
                root: false,
            })
        } else {
            map_evaluation(self.host.evaluate(&node.node), |node| LoadedNode {
                node,
                root: false,
            })
        }
    }
}

pub(super) fn prepare_loaded_schema_scope<H: SchemaPreparationHost>(
    host: &mut H,
    schema_uri: &str,
    control: &SchemaHostControl,
    outcome: ReferenceLinkEvaluation<SchemaDeclarationNode>,
    limits: ReferenceTraversalLimits,
) -> Result<SchemaScopePreparation, ReferenceResolutionError> {
    let root = LoadedNode {
        node: host.source_reference(control.attribute.clone()),
        root: true,
    };
    let selected = resolve_reference(
        root,
        &mut LoadedHost {
            host,
            control,
            outcome,
        },
        limits,
    )?;
    let selection = ReferenceResolution {
        nodes: selected.nodes.into_iter().map(|node| node.node).collect(),
        state: selected.state,
        failed: selected.failed,
        diagnostics: selected.diagnostics,
        work_used: selected.work_used,
        issues: selected
            .issues
            .into_iter()
            .map(|issue| ReferenceResolutionIssue {
                reference: issue.reference.node,
                kind: issue.kind,
                occurrence: issue.occurrence,
                reason: issue.reason,
            })
            .collect(),
    };
    prepare_schema_scope_selection(host, schema_uri, selection, limits)
}
