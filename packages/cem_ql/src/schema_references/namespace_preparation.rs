//! Explicit bounded namespace selection; no scope activation or URI extraction.
use super::{CemQlSchemaDeclarationHost, CemQlSchemaReferenceNode, LexicalScopeHandoffError};
use cem_ml::{
    schema::{
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        machine::LexicallyScopedDocument,
        namespace_references::{
            admit_namespace_scope_target, NamespaceScopeTarget, NamespaceScopeTargetError,
        },
        reference_traversal::ReferenceTraversalLimits,
    },
    value::reference_resolution::{
        resolve_reference, ReferenceResolution, ReferenceResolutionError,
    },
};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceScopePreparationIssue {
    TargetCount(usize),
    TargetHasNoSourceHandle,
    TargetMetadataNotReady,
    TargetBindingNotReady,
    TargetContextNotReady,
    TargetAdmission(NamespaceScopeTargetError),
}

/// Selection and admission are inspectable independently of readiness. A
/// selected pending declaration is not a usable URI or an inherited fallback.
#[derive(Debug, Clone)]
pub struct NamespaceScopePreparation {
    pub selection: ReferenceResolution<CemQlSchemaReferenceNode>,
    pub target: Option<NamespaceScopeTarget>,
    pub issue: Option<NamespaceScopePreparationIssue>,
}
impl NamespaceScopePreparation {
    pub fn is_ready(&self) -> bool {
        self.selection.is_complete()
            && !self.selection.failed
            && self.issue.is_none()
            && self.target.is_some()
    }
}
impl CemQlSchemaDeclarationHost {
    /// Retain original namespace metadata for a registered arena. Attachment is
    /// idempotent and grants no crossing, readiness or namespace activation.
    pub fn attach_captured_namespaces(
        &mut self,
        captured: Arc<LexicallyScopedDocument>,
    ) -> Result<(), LexicalScopeHandoffError> {
        if !self
            .scopes
            .iter()
            .any(|scope| Arc::ptr_eq(scope.tree.ast_owner(), captured.document()))
        {
            return Err(LexicalScopeHandoffError::UnregisteredOwner);
        }
        let key = Arc::as_ptr(captured.document()) as usize;
        self.captured_namespaces.entry(key).or_insert(captured);
        Ok(())
    }
    /// Explicit consumer lifecycle stage. The shared resolver applies request
    /// and destination limits, source policies and directed relationship grants.
    /// Singleton admission reads only original namespace declaration metadata;
    /// schema/data nodes and their namespace/URI attributes cannot supply it.
    /// This does not evaluate a pending declaration's attribute value, complete
    /// dependent QNames, install a namespace context or write source targets.
    pub fn prepare_namespace_scope(
        &mut self,
        reference: SchemaDeclarationNode,
        limits: ReferenceTraversalLimits,
    ) -> Result<NamespaceScopePreparation, ReferenceResolutionError> {
        let selection = resolve_reference(self.source_reference(reference), self, limits)?;
        let mut prepared = NamespaceScopePreparation {
            selection,
            target: None,
            issue: None,
        };
        if !prepared.selection.is_complete() || prepared.selection.failed {
            return Ok(prepared);
        }
        if prepared.selection.nodes.len() != 1 {
            prepared.issue = Some(NamespaceScopePreparationIssue::TargetCount(
                prepared.selection.nodes.len(),
            ));
            return Ok(prepared);
        }
        let Some(source) = self.declaration_node(&prepared.selection.nodes[0]) else {
            prepared.issue = Some(NamespaceScopePreparationIssue::TargetHasNoSourceHandle);
            return Ok(prepared);
        };
        let Some(captured) = self
            .captured_namespaces
            .get(&(Arc::as_ptr(source.document()) as usize))
        else {
            prepared.issue = Some(NamespaceScopePreparationIssue::TargetMetadataNotReady);
            return Ok(prepared);
        };
        if captured
            .pending_namespace_declaration(source.document(), source.node_id())
            .is_some()
        {
            prepared.issue = Some(NamespaceScopePreparationIssue::TargetBindingNotReady);
            return Ok(prepared);
        }
        let target = match admit_namespace_scope_target(source, captured) {
            Ok(target) => target,
            Err(issue) => {
                prepared.issue = Some(NamespaceScopePreparationIssue::TargetAdmission(issue));
                return Ok(prepared);
            }
        };
        // Match the existing declaration-consumer readiness contract even when
        // this completed literal declaration needs no expression evaluation.
        if !self
            .source_scope(&target.selected)
            .and_then(|scope| self.scope_record(scope))
            .is_some_and(|scope| scope.context.is_some())
        {
            prepared.issue = Some(NamespaceScopePreparationIssue::TargetContextNotReady);
        }
        prepared.target = Some(target);
        Ok(prepared)
    }
}
