//! Explicit consumer preparation before any schema-region activation.
use super::{CemQlSchemaDeclarationHost, CemQlSchemaReferenceNode, LexicalScopeHandoffError};
use cem_ml::{
    parser::ExpandedName,
    schema::{
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        document_model::SchemaDocumentModel,
        machine::LexicallyScopedDocument,
        reference_traversal::ReferenceTraversalLimits,
        scope_references::{
            admit_schema_scope_target, compile_schema_scope_target, SchemaScopeTarget,
            SchemaScopeTargetError,
        },
    },
    value::reference_resolution::{
        resolve_reference, ReferenceResolution, ReferenceResolutionError,
    },
};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaScopePreparationIssue {
    TargetCount(usize),
    TargetHasNoSourceHandle,
    TargetContextNotReady,
    TargetAdmission(SchemaScopeTargetError),
}

/// Inspection retains the bounded selection, original target and any compiled
/// candidate model. Absence of an admission issue does not imply readiness;
/// pending selection/dependencies and hard compile diagnostics also block it.
#[derive(Debug, Clone)]
pub struct SchemaScopePreparation {
    pub selection: ReferenceResolution<CemQlSchemaReferenceNode>,
    pub target: Option<SchemaScopeTarget>,
    /// Shared immutable candidate for inspection and consuming region placement.
    pub model: Option<Arc<SchemaDocumentModel>>,
    pub issue: Option<SchemaScopePreparationIssue>,
}
impl SchemaScopePreparation {
    pub fn is_ready(&self) -> bool {
        self.selection.is_complete()
            && !self.selection.failed
            && self.issue.is_none()
            && self.target.is_some()
            && self.model.as_ref().is_some_and(|model| {
                model.is_ready_for_validation()
                    && !model
                        .compile_diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.severity.is_hard_violation())
            })
    }
}

impl CemQlSchemaDeclarationHost {
    /// Attach immutable namespace metadata to a registered original arena.
    /// Repeat handoff is idempotent. No AST, tree view, context or scope changes.
    pub fn attach_captured_names(
        &mut self,
        captured: &LexicallyScopedDocument,
    ) -> Result<(), LexicalScopeHandoffError> {
        let owner = captured.document();
        if !self
            .scopes
            .iter()
            .any(|scope| Arc::ptr_eq(scope.tree.ast_owner(), owner))
        {
            return Err(LexicalScopeHandoffError::UnregisteredOwner);
        }
        let key = Arc::as_ptr(owner) as usize;
        self.captured_names.entry(key).or_insert_with(|| {
            (0..owner.nodes.len())
                .filter_map(|id| {
                    let id = id as cem_ml::parser::AstNodeId;
                    captured
                        .expanded_name(owner, id)
                        .cloned()
                        .map(|name| (id, name))
                })
                .collect()
        });
        Ok(())
    }

    /// Captured names never fall back to lexical prefixes in raw source nodes.
    pub fn captured_expanded_name(&self, source: &SchemaDeclarationNode) -> Option<&ExpandedName> {
        self.captured_names
            .get(&(Arc::as_ptr(source.document()) as usize))?
            .get(&source.node_id())
    }

    /// Explicit lifecycle consumer of one native reference occurrence. Selection
    /// and declaration compilation use their existing bounded consumer stages,
    /// runtime contexts and directed grants. This does not recognize selector
    /// syntax, assign scopes, install policies or choose an inherited fallback.
    pub fn prepare_schema_scope(
        &mut self,
        schema_uri: &str,
        reference: SchemaDeclarationNode,
        limits: ReferenceTraversalLimits,
    ) -> Result<SchemaScopePreparation, ReferenceResolutionError> {
        let root = self.source_reference(reference);
        self.prepare_schema_scope_node(schema_uri, root, limits)
    }

    pub(super) fn prepare_schema_scope_node(
        &mut self,
        schema_uri: &str,
        root: CemQlSchemaReferenceNode,
        limits: ReferenceTraversalLimits,
    ) -> Result<SchemaScopePreparation, ReferenceResolutionError> {
        let selection = resolve_reference(root, self, limits)?;
        let mut prepared = SchemaScopePreparation {
            selection,
            target: None,
            model: None,
            issue: None,
        };
        if !prepared.selection.is_complete() || prepared.selection.failed {
            return Ok(prepared);
        }
        if prepared.selection.nodes.len() != 1 {
            prepared.issue = Some(SchemaScopePreparationIssue::TargetCount(
                prepared.selection.nodes.len(),
            ));
            return Ok(prepared);
        }
        let Some(source) = self.declaration_node(&prepared.selection.nodes[0]) else {
            prepared.issue = Some(SchemaScopePreparationIssue::TargetHasNoSourceHandle);
            return Ok(prepared);
        };
        let target = match admit_schema_scope_target(source, |node| {
            self.captured_expanded_name(node).cloned()
        }) {
            Ok(target) => target,
            Err(issue) => {
                prepared.issue = Some(SchemaScopePreparationIssue::TargetAdmission(issue));
                return Ok(prepared);
            }
        };
        // A declaration's lifecycle context can remain unavailable even when
        // selection from the requesting context completed and needs no inputs.
        if self
            .source_scope(&target.declaration)
            .and_then(|scope| self.scope_record(scope))
            .and_then(|scope| scope.context.as_ref())
            .is_none()
        {
            prepared.target = Some(target);
            prepared.issue = Some(SchemaScopePreparationIssue::TargetContextNotReady);
            return Ok(prepared);
        }
        prepared.model = Some(Arc::new(compile_schema_scope_target(
            schema_uri, &target, self, limits,
        )?));
        prepared.target = Some(target);
        Ok(prepared)
    }
}
