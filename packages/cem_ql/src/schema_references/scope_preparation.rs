//! Explicit consumer preparation before any schema-region activation.
use super::{CemQlSchemaDeclarationHost, CemQlSchemaReferenceNode, LexicalScopeHandoffError};
use cem_ml::{
    parser::ExpandedName,
    schema::{
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        document_model::SchemaDocumentModel,
        machine::LexicallyScopedDocument,
        reference_traversal::ReferenceTraversalLimits,
        scope_controls::SchemaHostControl,
        scope_references::{
            admit_schema_scope_target, compile_schema_scope_target, SchemaScopeTarget,
            SchemaScopeTargetError,
        },
    },
    value::reference_resolution::{
        resolve_reference, ReferenceLinkEvaluation, ReferenceResolution, ReferenceResolutionError,
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
    /// Attach immutable namespace and schema-element form metadata to a registered
    /// original arena.
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
        self.captured_schema_forms.entry(key).or_insert_with(|| {
            (0..owner.nodes.len())
                .filter_map(|id| {
                    let id = id as cem_ml::parser::AstNodeId;
                    captured
                        .schema_element_form(owner, id)
                        .map(|form| (id, form))
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

    /// Original parser/importer distinction between wrapping and following
    /// schema elements, including empty bodies. Generic AST defaults do not
    /// establish this lifecycle boundary.
    pub fn captured_schema_element_form(
        &self,
        source: &SchemaDeclarationNode,
    ) -> Option<cem_ml::schema::machine::SchemaElementForm> {
        self.captured_schema_forms
            .get(&(Arc::as_ptr(source.document()) as usize))?
            .get(&source.node_id())
            .copied()
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
        prepare_schema_scope_node(self, schema_uri, root, limits)
    }
}

pub(super) trait SchemaPreparationHost:
    SchemaDeclarationHost<Node = CemQlSchemaReferenceNode>
{
    fn captured_expanded_name(&self, source: &SchemaDeclarationNode) -> Option<&ExpandedName>;
    fn schema_uri_load(
        &self,
        control: &SchemaHostControl,
    ) -> Option<ReferenceLinkEvaluation<SchemaDeclarationNode>>;
    fn target_context_is_ready(
        &mut self,
        target: &SchemaDeclarationNode,
    ) -> Result<bool, ReferenceResolutionError>;
}
impl SchemaPreparationHost for CemQlSchemaDeclarationHost {
    fn schema_uri_load(
        &self,
        control: &SchemaHostControl,
    ) -> Option<ReferenceLinkEvaluation<SchemaDeclarationNode>> {
        self.schema_uri_load_for_control(control)
    }
    fn captured_expanded_name(&self, source: &SchemaDeclarationNode) -> Option<&ExpandedName> {
        self.captured_expanded_name(source)
    }
    fn target_context_is_ready(
        &mut self,
        target: &SchemaDeclarationNode,
    ) -> Result<bool, ReferenceResolutionError> {
        Ok(self
            .source_scope(target)
            .and_then(|scope| self.scope_record(scope))
            .is_some_and(|scope| scope.context.is_some()))
    }
}

pub(super) fn prepare_schema_scope_node<H: SchemaPreparationHost>(
    host: &mut H,
    schema_uri: &str,
    root: CemQlSchemaReferenceNode,
    limits: ReferenceTraversalLimits,
) -> Result<SchemaScopePreparation, ReferenceResolutionError> {
    let selection = resolve_reference(root, host, limits)?;
    prepare_schema_scope_selection(host, schema_uri, selection, limits)
}

pub(super) fn prepare_schema_scope_selection<H: SchemaPreparationHost>(
    host: &mut H,
    schema_uri: &str,
    selection: ReferenceResolution<CemQlSchemaReferenceNode>,
    limits: ReferenceTraversalLimits,
) -> Result<SchemaScopePreparation, ReferenceResolutionError> {
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
    let Some(source) = host.declaration_node(&prepared.selection.nodes[0]) else {
        prepared.issue = Some(SchemaScopePreparationIssue::TargetHasNoSourceHandle);
        return Ok(prepared);
    };
    let target = match admit_schema_scope_target(source, |node| {
        host.captured_expanded_name(node).cloned()
    }) {
        Ok(target) => target,
        Err(issue) => {
            prepared.issue = Some(SchemaScopePreparationIssue::TargetAdmission(issue));
            return Ok(prepared);
        }
    };
    // A declaration's lifecycle context can remain unavailable even when
    // selection from the requesting context completed and needs no inputs.
    if !host.target_context_is_ready(&target.declaration)? {
        prepared.target = Some(target);
        prepared.issue = Some(SchemaScopePreparationIssue::TargetContextNotReady);
        return Ok(prepared);
    }
    prepared.model = Some(Arc::new(compile_schema_scope_target(
        schema_uri, &target, host, limits,
    )?));
    prepared.target = Some(target);
    Ok(prepared)
}
