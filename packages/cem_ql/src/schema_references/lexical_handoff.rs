//! Explicit lifecycle handoff of parser-captured original expression bindings.
use super::{CemQlSchemaDeclarationHost, DeclarationScope};
use crate::api::StandaloneExpressionContext;
use cem_ml::{
    parser::AstNodeId,
    schema::{
        declaration_references::SchemaDeclarationNode,
        machine::{LexicalScopeSnapshot, LexicallyScopedDocument},
        reference_policy::{ReferenceScopePolicy, ReferenceScopePolicyOverrides},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexicalScopeHandoffError {
    UnregisteredOwner,
    OccurrenceAlreadyAssigned(AstNodeId),
}

impl CemQlSchemaDeclarationHost {
    /// Attach original expression occurrences at an explicit consumer lifecycle
    /// stage. The caller prepares runtime inputs/readiness and effective policy
    /// from each saved binding; None context means pending, never inherited input.
    /// Each occurrence preserves its nearest existing relationship boundary.
    /// Invalid owners/repeat attachments reject before preparation or mutation.
    /// All preparation finishes before any scopes are attached. No compilation,
    /// selector execution, grant creation or source-target mutation happens here.
    pub fn attach_captured_lexical_scopes<F>(
        &mut self,
        captured: &LexicallyScopedDocument,
        mut prepare: F,
    ) -> Result<Vec<(AstNodeId, DeclarationScope)>, LexicalScopeHandoffError>
    where
        F: FnMut(
            &SchemaDeclarationNode,
            &LexicalScopeSnapshot,
            DeclarationScope,
        ) -> (Option<StandaloneExpressionContext>, ReferenceScopePolicy),
    {
        self.attach_captured_lexical_scopes_with_policy_overrides(
            captured,
            |source, snapshot, parent| {
                let (context, policy) = prepare(source, snapshot, parent);
                (context, ReferenceScopePolicyOverrides::explicit(policy))
            },
        )
    }

    /// Preserve local policy declarations separately from inherited settings.
    /// All contexts/overrides are prepared before source occurrences are attached;
    /// missing context is pending, regardless of available parent inputs.
    pub fn attach_captured_lexical_scopes_with_policy_overrides<F>(
        &mut self,
        captured: &LexicallyScopedDocument,
        mut prepare: F,
    ) -> Result<Vec<(AstNodeId, DeclarationScope)>, LexicalScopeHandoffError>
    where
        F: FnMut(
            &SchemaDeclarationNode,
            &LexicalScopeSnapshot,
            DeclarationScope,
        ) -> (
            Option<StandaloneExpressionContext>,
            ReferenceScopePolicyOverrides,
        ),
    {
        let owner = captured.document();
        if !self
            .scopes
            .iter()
            .any(|scope| std::sync::Arc::ptr_eq(scope.tree.ast_owner(), owner))
        {
            return Err(LexicalScopeHandoffError::UnregisteredOwner);
        }
        let key = std::sync::Arc::as_ptr(owner) as usize;
        let mut parents = Vec::new();
        for node in captured.occurrences() {
            if self.node_scopes.contains_key(&(key, node)) {
                return Err(LexicalScopeHandoffError::OccurrenceAlreadyAssigned(node));
            }
            let source = SchemaDeclarationNode::new(owner.clone(), node)
                .expect("captured original expression node");
            let parent = self
                .source_scope(&source)
                .ok_or(LexicalScopeHandoffError::UnregisteredOwner)?;
            parents.push((source, parent));
        }
        let prepared: Vec<_> = parents
            .into_iter()
            .map(|(source, parent)| {
                let snapshot = captured
                    .snapshot(owner, source.node_id())
                    .expect("captured owner checked above");
                let (context, local_policy) = prepare(&source, snapshot, parent);
                (source.node_id(), parent, context, local_policy)
            })
            .collect();
        self.attach_captured_names(captured)?;
        Ok(prepared
            .into_iter()
            .map(|(node, parent, context, policy)| {
                let scope = self
                    .register_lexical_scope_with_policy_overrides(parent, context, policy)
                    .expect("preflight checked parent in this host");
                self.node_scopes.insert((key, node), scope);
                (node, scope)
            })
            .collect())
    }
}
