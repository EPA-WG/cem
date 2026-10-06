//! Invocation-local child body handoff over entered original handles.
use super::{
    host_controls::prepare_decoded_host_control, region_validation::append_preparation_diagnostics,
    scope_preparation::SchemaPreparationHost, CemQlSchemaDeclarationHost, CemQlSchemaReferenceNode,
    DeclarationScope, SchemaHostRegionPreparation, SchemaHostRuntimeInputIssue,
    SchemaHostRuntimeInputs, SchemaHostRuntimeScope,
};
use crate::api::StandaloneExpressionContext;
use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    parser::{tree::RetainedCemTree, AstNodeId, ExpandedName},
    schema::{
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        document_model::SchemaDocumentModel,
        input_references::{
            validate_structural_input_discovering_regions_references, DiscoveredInputSchemaRegion,
            StructuralInputValidation,
        },
        reference_policy::{ReferenceOccurrence, ReferenceUnresolvedPolicy},
        reference_traversal::ReferenceTraversalLimits,
        scope_controls::validate_schema_host_controls,
    },
    value::reference_resolution::{
        ReferenceLinkEvaluation, ReferenceResolutionError, ReferenceResolutionHost,
    },
};
use std::{collections::BTreeMap, sync::Arc};

type SourceKey = (usize, AstNodeId);
fn key(source: &SchemaDeclarationNode) -> SourceKey {
    (Arc::as_ptr(source.document()) as usize, source.node_id())
}

/// The caller supplies inputs at consumption time from its captured bindings.
/// None stays pending. Body inputs apply to inherited body frames; an original
/// local frame requests its own context instead of borrowing the body context.
pub enum SchemaHostRuntimeContextRequest<'a> {
    Body(&'a SchemaHostRegionPreparation),
    Occurrence {
        source: &'a SchemaDeclarationNode,
        original_scope: DeclarationScope,
        child: &'a SchemaHostRuntimeScope,
    },
}

/// One invocation's readiness, retained validation and immutable frame handles.
/// Source assignments are restored before returning, including on errors/unwind;
/// registered frames remain inspectable and never overwrite captured records.
#[derive(Debug, Clone)]
pub struct SchemaHostRuntimeValidation {
    pub validation: StructuralInputValidation<CemQlSchemaReferenceNode>,
    pub inputs: Vec<SchemaHostRuntimeInputs>,
    pub scopes: Vec<SchemaHostRuntimeScope>,
    pub occurrences: Vec<(SchemaDeclarationNode, DeclarationScope)>,
}

struct BodyBinding {
    child: SchemaHostRuntimeScope,
    original_enclosing: DeclarationScope,
}
struct RuntimeHost<'a, F> {
    host: &'a mut CemQlSchemaDeclarationHost,
    context: F,
    original_assignments: BTreeMap<SourceKey, DeclarationScope>,
    bodies: BTreeMap<SourceKey, BodyBinding>,
    frames: BTreeMap<(SourceKey, DeclarationScope), DeclarationScope>,
    inputs: Vec<SchemaHostRuntimeInputs>,
    scopes: Vec<SchemaHostRuntimeScope>,
    occurrences: Vec<(SchemaDeclarationNode, DeclarationScope)>,
}
impl<F> Drop for RuntimeHost<'_, F> {
    fn drop(&mut self) {
        self.host.node_scopes = std::mem::take(&mut self.original_assignments);
    }
}
impl<F> RuntimeHost<'_, F>
where
    F: for<'a> FnMut(SchemaHostRuntimeContextRequest<'a>) -> Option<StandaloneExpressionContext>,
{
    // Host attributes retain their enclosing context. Only a branch reached via
    // an original owning child edge is in a host's body; nested/sibling exit
    // therefore restores context by ancestry, without editing earlier frames.
    fn body(&self, source: &SchemaDeclarationNode) -> Option<SourceKey> {
        let tree = &self
            .host
            .scopes
            .iter()
            .find(|scope| Arc::ptr_eq(scope.tree.ast_owner(), source.document()))?
            .tree;
        let owner = key(source).0;
        let mut child = source.node_id();
        while let Some(parent) = tree.source_parent(child) {
            let candidate = (owner, parent);
            if self.bodies.contains_key(&candidate)
                && CemQlSchemaDeclarationHost::source_children(tree, parent)
                    .is_some_and(|children| children.contains(&child))
            {
                return Some(candidate);
            }
            child = parent;
        }
        None
    }

    fn discover(
        &mut self,
        schema_uri: &str,
        source: SchemaDeclarationNode,
        limits: ReferenceTraversalLimits,
    ) -> Result<Option<DiscoveredInputSchemaRegion>, ReferenceResolutionError> {
        let contract = validate_schema_host_controls(source.clone(), |source| {
            self.host.captured_expanded_name(source).cloned()
        });
        if !contract.has_override() {
            return Ok(None);
        }
        let preparation = contract
            .control()
            .map(|control| prepare_decoded_host_control(self, schema_uri, control, limits))
            .transpose()?
            .flatten();
        let region = SchemaHostRegionPreparation {
            contract,
            preparation,
        };
        // Unready controls, selectors and policies do not request body inputs.
        let mut inputs = self.host.prepare_schema_host_runtime_inputs(region, None);
        if matches!(
            inputs.issue(),
            Some(SchemaHostRuntimeInputIssue::ContextNotReady)
        ) {
            let context = (self.context)(SchemaHostRuntimeContextRequest::Body(inputs.region()));
            inputs = inputs.with_context(context);
        }
        let ready = inputs.is_ready();
        let mut diagnostics = vec![];
        if let Some(SchemaHostRuntimeInputIssue::InvalidPolicy(error)) = inputs.issue() {
            diagnostics.push(Diagnostic {
                code: "cem.schema_scope.invalid_reference_policy".into(),
                severity: Severity::Error,
                message: error.message.clone(),
                node: Some(source.identity()),
                source_map: Some(error.source_map.clone()),
                ..Default::default()
            });
        }
        let model = ready.then(|| {
            inputs
                .region()
                .preparation
                .as_ref()
                .unwrap()
                .model
                .as_ref()
                .unwrap()
                .clone()
        });
        if ready {
            let original_enclosing = self
                .host
                .source_scope_with_assignments(&source, &self.original_assignments)
                .ok_or(ReferenceResolutionError::InvalidScopeHandoff)?;
            let child = self
                .host
                .register_schema_host_runtime_scope(inputs.clone())
                .map_err(|_| ReferenceResolutionError::InvalidScopeHandoff)?;
            self.scopes.push(child.clone());
            self.bodies.insert(
                key(&source),
                BodyBinding {
                    child,
                    original_enclosing,
                },
            );
        }
        let result = DiscoveredInputSchemaRegion {
            contract: inputs.region().contract.clone(),
            model,
            diagnostics,
        };
        self.inputs.push(inputs);
        Ok(Some(result))
    }
}
impl<F> ReferenceResolutionHost for RuntimeHost<'_, F>
where
    F: for<'a> FnMut(SchemaHostRuntimeContextRequest<'a>) -> Option<StandaloneExpressionContext>,
{
    type Node = CemQlSchemaReferenceNode;
    type Scope = Option<DeclarationScope>;
    fn prepare_node(&mut self, node: &mut Self::Node) -> Result<(), ReferenceResolutionError> {
        let Some(source) = &node.source else {
            return Ok(());
        };
        let Some(boundary) = self.body(source) else {
            return Ok(());
        };
        let binding = &self.bodies[&boundary];
        let original = self
            .host
            .source_scope_with_assignments(source, &self.original_assignments)
            .ok_or(ReferenceResolutionError::InvalidScopeHandoff)?;
        let scope = if original == binding.original_enclosing {
            binding.child.scope()
        } else if let Some(scope) = self.frames.get(&(boundary, original)) {
            *scope
        } else {
            let context = (self.context)(SchemaHostRuntimeContextRequest::Occurrence {
                source,
                original_scope: original,
                child: &binding.child,
            });
            let scope = self
                .host
                .register_schema_host_occurrence_scope_from(
                    &binding.child,
                    binding.original_enclosing,
                    original,
                    context,
                )
                .map_err(|_| ReferenceResolutionError::InvalidScopeHandoff)?;
            self.frames.insert((boundary, original), scope);
            self.occurrences.push((source.clone(), scope));
            scope
        };
        self.host.node_scopes.insert(key(source), scope);
        node.scope = Some(scope);
        Ok(())
    }
    fn scope(&self, node: &Self::Node) -> Self::Scope {
        self.host.scope(node)
    }
    fn scope_limits(&self, scope: &Self::Scope) -> ReferenceTraversalLimits {
        self.host.scope_limits(scope)
    }
    fn reference_occurrence(&self, node: &Self::Node) -> Option<ReferenceOccurrence> {
        self.host.reference_occurrence(node)
    }
    fn unresolved_policy(&self, node: &Self::Node) -> &ReferenceUnresolvedPolicy {
        self.host.unresolved_policy(node)
    }
    fn permits_edge(&self, from: &Self::Node, to: &Self::Node) -> bool {
        self.host.permits_edge(from, to)
    }
    fn evaluate(&mut self, node: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        self.host.evaluate(node)
    }
}
impl<F> SchemaDeclarationHost for RuntimeHost<'_, F>
where
    F: for<'a> FnMut(SchemaHostRuntimeContextRequest<'a>) -> Option<StandaloneExpressionContext>,
{
    fn source_reference(&self, source: SchemaDeclarationNode) -> Self::Node {
        self.host.source_reference(source)
    }
    fn declaration_node(&self, target: &Self::Node) -> Option<SchemaDeclarationNode> {
        self.host.declaration_node(target)
    }
    fn declaration_schema(&self, target: &SchemaDeclarationNode) -> Option<SchemaDeclarationNode> {
        self.host.declaration_schema(target)
    }
    fn evaluate_input_expression(
        &mut self,
        node: &Self::Node,
    ) -> ReferenceLinkEvaluation<Self::Node> {
        SchemaDeclarationHost::evaluate_input_expression(self.host, node)
    }
}
impl<F> SchemaPreparationHost for RuntimeHost<'_, F>
where
    F: for<'a> FnMut(SchemaHostRuntimeContextRequest<'a>) -> Option<StandaloneExpressionContext>,
{
    fn captured_expanded_name(&self, source: &SchemaDeclarationNode) -> Option<&ExpandedName> {
        self.host.captured_expanded_name(source)
    }
    fn target_context_is_ready(
        &mut self,
        target: &SchemaDeclarationNode,
    ) -> Result<bool, ReferenceResolutionError> {
        let mut node = self.source_reference(target.clone());
        self.prepare_node(&mut node)?;
        Ok(node
            .scope
            .and_then(|scope| self.host.scope_record(scope))
            .is_some_and(|scope| scope.context.is_some()))
    }
}

impl CemQlSchemaDeclarationHost {
    /// Discover controls and bind ready child bodies at an explicit lifecycle
    /// invocation. Original local frames request their own caller inputs as they
    /// are consumed; inactive, blocked and denied descendants are not prepared.
    /// Selected traversal keeps its active references and request/destination
    /// accounting. This creates no grants, evaluates no source on load, and
    /// leaves the original captured mappings intact when the invocation ends.
    pub fn validate_input_runtime_host_regions<F>(
        &mut self,
        schema_uri: &str,
        source: Arc<RetainedCemTree>,
        roots: &[AstNodeId],
        model: &SchemaDocumentModel,
        limits: ReferenceTraversalLimits,
        context: F,
    ) -> Result<SchemaHostRuntimeValidation, ReferenceResolutionError>
    where
        F: for<'a> FnMut(
            SchemaHostRuntimeContextRequest<'a>,
        ) -> Option<StandaloneExpressionContext>,
    {
        let original_assignments = self.node_scopes.clone();
        let mut host = RuntimeHost {
            host: self,
            context,
            original_assignments,
            bodies: BTreeMap::new(),
            frames: BTreeMap::new(),
            inputs: vec![],
            scopes: vec![],
            occurrences: vec![],
        };
        let mut validation = validate_structural_input_discovering_regions_references(
            source.ast_owner().clone(),
            roots,
            model,
            &mut host,
            limits,
            |host, source, limits| host.discover(schema_uri, source, limits),
        )?;
        for inputs in &host.inputs {
            if !inputs.region().is_ready() {
                if let Some(preparation) = &inputs.region().preparation {
                    append_preparation_diagnostics(
                        &mut validation,
                        inputs.region().contract.host(),
                        preparation,
                    );
                }
            }
        }
        validation.failed |= validation
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity.is_hard_violation());
        Ok(SchemaHostRuntimeValidation {
            validation,
            inputs: std::mem::take(&mut host.inputs),
            scopes: std::mem::take(&mut host.scopes),
            occurrences: std::mem::take(&mut host.occurrences),
        })
    }
}
