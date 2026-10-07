//! Selected-forest lifecycle handoff of completed original namespace bindings.
use super::{CemQlSchemaDeclarationHost, DeclarationScope, LexicalScopeHandoffError};
use crate::api::StandaloneExpressionContext;
use cem_ml::{
    parser::AstNodeId,
    schema::{
        declaration_references::SchemaDeclarationNode,
        namespace_references::{
            NamespaceLexicalSnapshot, NamespaceNameCompletion, NamespaceNameCompletionError,
        },
        reference_policy::ReferenceScopePolicyOverrides,
    },
};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceLexicalScopeHandoffError {
    Lexical(LexicalScopeHandoffError),
    Namespace(NamespaceNameCompletionError),
}
impl From<LexicalScopeHandoffError> for NamespaceLexicalScopeHandoffError {
    fn from(error: LexicalScopeHandoffError) -> Self {
        Self::Lexical(error)
    }
}
impl From<NamespaceNameCompletionError> for NamespaceLexicalScopeHandoffError {
    fn from(error: NamespaceNameCompletionError) -> Self {
        Self::Namespace(error)
    }
}

impl CemQlSchemaDeclarationHost {
    /// Explicit consumer handoff for only the completion's selected occurrences.
    /// Every saved pending prefix must be ready, even if no selected QName uses it.
    /// Owner, assignment and namespace readiness checks finish before any callback
    /// or mutation, so incomplete attempts are retryable without partial contexts.
    /// The caller supplies runtime inputs (None stays pending) and local policy
    /// overrides. It can bind the matching NamespaceQueryTree in those contexts;
    /// completed names never replace this host's original captured-name metadata.
    /// Existing relationship boundaries/grants remain unchanged. No compilation,
    /// reference evaluation, scope activation or source-target writeback occurs.
    pub fn attach_completed_namespace_lexical_scopes<F>(
        &mut self,
        completion: &NamespaceNameCompletion,
        mut prepare: F,
    ) -> Result<Vec<(AstNodeId, DeclarationScope)>, NamespaceLexicalScopeHandoffError>
    where
        F: FnMut(
            &SchemaDeclarationNode,
            &NamespaceLexicalSnapshot,
            DeclarationScope,
        ) -> (
            Option<StandaloneExpressionContext>,
            ReferenceScopePolicyOverrides,
        ),
    {
        let captured = completion.captured();
        let owner = captured.document();
        if !self
            .scopes
            .iter()
            .any(|scope| Arc::ptr_eq(scope.tree.ast_owner(), owner))
        {
            return Err(LexicalScopeHandoffError::UnregisteredOwner.into());
        }
        let key = Arc::as_ptr(owner) as usize;
        let mut selected = Vec::new();
        for node in captured
            .occurrences()
            .filter(|node| completion.contains(*node))
        {
            if self.node_scopes.contains_key(&(key, node)) {
                return Err(LexicalScopeHandoffError::OccurrenceAlreadyAssigned(node).into());
            }
            let source = SchemaDeclarationNode::new(owner.clone(), node)
                .expect("captured original occurrence");
            let parent = self
                .source_scope(&source)
                .ok_or(LexicalScopeHandoffError::UnregisteredOwner)?;
            selected.push((source, parent));
        }
        let snapshots: Vec<_> = selected
            .into_iter()
            .map(|(source, parent)| {
                completion
                    .lexical_snapshot(&source)
                    .map(|snapshot| (source, parent, snapshot))
            })
            .collect::<Result<_, _>>()?;
        let prepared: Vec<_> = snapshots
            .into_iter()
            .map(|(source, parent, snapshot)| {
                let (context, policy) = prepare(&source, &snapshot, parent);
                (source.node_id(), parent, context, policy)
            })
            .collect();
        self.attach_captured_names(captured)?;
        Ok(prepared
            .into_iter()
            .map(|(node, parent, context, policy)| {
                let scope = self
                    .register_lexical_scope_with_policy_overrides(parent, context, policy)
                    .expect("preflight checked original parent in this host");
                self.node_scopes.insert((key, node), scope);
                (node, scope)
            })
            .collect())
    }
}
