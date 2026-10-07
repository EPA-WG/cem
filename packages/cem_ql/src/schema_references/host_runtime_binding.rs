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
        document_model::{SchemaBehaviorEvaluator, SchemaDocumentModel},
        input_references::{
            validate_structural_input_discovering_regions_references_with_behavior_evaluator,
            DiscoveredInputSchemaRegion, StructuralInputValidation,
        },
        reference_policy::{ReferenceOccurrence, ReferenceUnresolvedPolicy},
        reference_traversal::ReferenceTraversalLimits,
        scope_controls::{validate_schema_scope_controls, SchemaScopeControlExtent},
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
/// None stays pending. Body inputs apply to inherited region frames; inspect the
/// contract extent for body versus following regions. An original local frame
/// requests its own context instead of borrowing the selected region context.
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
    following: BTreeMap<SourceKey, BTreeMap<usize, SourceKey>>,
    blocked: BTreeMap<SourceKey, DeclarationScope>,
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
    // Original owning order chooses a following scope; the switch itself stays
    // enclosing. Nearest body boundaries override enclosing following scopes.
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
            if let Some(boundary) = CemQlSchemaDeclarationHost::source_children(tree, parent)
                .and_then(|children| children.iter().position(|id| *id == child))
                .and_then(|index| self.following.get(&candidate)?.range(..=index).next_back())
                .map(|(_, boundary)| *boundary)
            {
                return Some(boundary);
            }
            if self.bodies.get(&candidate).is_some_and(|binding| {
                binding.child.inputs().region().contract.extent() == SchemaScopeControlExtent::Body
            }) && CemQlSchemaDeclarationHost::source_children(tree, parent)
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
        let contract = validate_schema_scope_controls(
            source.clone(),
            |source| self.host.consuming_expanded_name(source).cloned(),
            self.host.captured_schema_element_form(&source),
        );
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
        let following = inputs.region().contract.extent() == SchemaScopeControlExtent::Following;
        if following {
            let tree = self
                .host
                .scopes
                .iter()
                .find(|scope| Arc::ptr_eq(scope.tree.ast_owner(), source.document()))
                .map(|scope| &scope.tree)
                .ok_or(ReferenceResolutionError::InvalidScopeHandoff)?;
            let parent = tree
                .source_parent(source.node_id())
                .ok_or(ReferenceResolutionError::InvalidScopeHandoff)?;
            let position = CemQlSchemaDeclarationHost::source_children(tree, parent)
                .and_then(|children| children.iter().position(|id| *id == source.node_id()))
                .ok_or(ReferenceResolutionError::InvalidScopeHandoff)?;
            self.following
                .entry((key(&source).0, parent))
                .or_default()
                .insert(position + 1, key(&source));
            if !ready {
                let parent = self
                    .host
                    .source_scope(&source)
                    .ok_or(ReferenceResolutionError::InvalidScopeHandoff)?;
                let pending = self
                    .host
                    .register_lexical_scope_with_policy_overrides(parent, None, Default::default())
                    .ok_or(ReferenceResolutionError::InvalidScopeHandoff)?;
                self.blocked.insert(key(&source), pending);
            }
        }
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
        if let Some(pending) = self.blocked.get(&boundary) {
            self.host.node_scopes.insert(key(source), *pending);
            node.scope = Some(*pending);
            return Ok(());
        }
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
    fn input_consumed_namespace_attribute(&self, source: &SchemaDeclarationNode) -> bool {
        SchemaDeclarationHost::input_consumed_namespace_attribute(self.host, source)
    }
    fn input_expanded_name<'a>(
        &'a self,
        source: &'a SchemaDeclarationNode,
    ) -> Option<&'a ExpandedName> {
        SchemaDeclarationHost::input_expanded_name(self.host, source)
    }
    fn input_source_tree(
        &self,
        source: &SchemaDeclarationNode,
    ) -> Option<Arc<RetainedCemTree>> {
        SchemaDeclarationHost::input_source_tree(self.host, source)
    }
    fn source_reference(&self, source: SchemaDeclarationNode) -> Self::Node {
        self.host.source_reference(source)
    }
    fn declaration_node(&self, target: &Self::Node) -> Option<SchemaDeclarationNode> {
        self.host.declaration_node(target)
    }
    fn declaration_schema(&self, target: &SchemaDeclarationNode) -> Option<SchemaDeclarationNode> {
        self.host.declaration_schema(target)
    }
    fn structural_diagnostic(
        &self,
        source: &SchemaDeclarationNode,
        diagnostic: cem_ml::diagnostics::Diagnostic,
    ) -> cem_ml::diagnostics::Diagnostic {
        self.host.structural_diagnostic(source, diagnostic)
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
    fn consuming_expanded_name(&self, source: &SchemaDeclarationNode) -> Option<&ExpandedName> {
        self.host.consuming_expanded_name(source)
    }
    fn schema_uri_load(
        &self,
        control: &cem_ml::schema::scope_controls::SchemaHostControl,
    ) -> Option<ReferenceLinkEvaluation<SchemaDeclarationNode>> {
        self.host.schema_uri_load_for_control(control)
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
    /// Discover host/wrapping controls and bind ready child bodies at an explicit lifecycle
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
        self.validate_input_runtime_host_regions_with_behavior_evaluator(
            schema_uri, source, roots, model, limits, context, None,
        )
    }

    /// Run each consuming model's retained behavior stage over its placements
    /// before restoring invocation assignments. Original node/attribute handles,
    /// consumed relationships and completed attribute values stay shared; the
    /// behavior stage cannot repeat reference consumption or borrow a child model.
    pub fn validate_input_runtime_host_regions_with_behavior_evaluator<F>(
        &mut self,
        schema_uri: &str,
        source: Arc<RetainedCemTree>,
        roots: &[AstNodeId],
        model: &SchemaDocumentModel,
        limits: ReferenceTraversalLimits,
        context: F,
        evaluator: Option<&dyn SchemaBehaviorEvaluator>,
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
            following: BTreeMap::new(),
            blocked: BTreeMap::new(),
            frames: BTreeMap::new(),
            inputs: vec![],
            scopes: vec![],
            occurrences: vec![],
        };
        let mut validation =
            validate_structural_input_discovering_regions_references_with_behavior_evaluator(
                source.ast_owner().clone(),
                roots,
                model,
                &mut host,
                limits,
                |host, source, limits| host.discover(schema_uri, source, limits),
                evaluator,
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
