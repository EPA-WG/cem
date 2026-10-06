//! Explicit structural validation over retained source and selected subtrees.
//! Runtime hosts choose context and timing; source arenas are never rewritten.
use super::{
    declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
    document_model::{self, SchemaBehaviorEvaluator, SchemaDocumentModel},
    reference_traversal::ReferenceTraversalLimits,
};
use crate::{
    diagnostics::{Diagnostic, Severity},
    parser::{document::CemDocument, CemAstNode},
    value::reference_resolution::{
        ReferenceResolutionError, ReferenceResolutionHost, ReferenceResolutionState,
        ReferenceStructureResolution,
    },
};
use std::{collections::HashSet, sync::Arc};

mod consumed_walk;
mod behaviors;
mod regions;
pub use regions::{DiscoveredInputSchemaRegion, InputSchemaRegion};
use regions::RegionModels;

pub const INVALID_STRUCTURAL_TARGET: &str = "cem.schema_validation.invalid_reference_target";

/// Retained-node adaptation is independent of schema-declaration lookup.
pub trait InputReferenceHost: ReferenceResolutionHost {
    fn source_node(&self, source: SchemaDeclarationNode) -> Self::Node;
    fn retained_node(&self, node: &Self::Node) -> Option<SchemaDeclarationNode>;
    /// Explicit lifecycle evaluation of an authored native expression slot.
    /// Return retained nodes/references, or preserve pending context readiness.
    fn evaluate_input_expression(
        &mut self,
        _expression: &Self::Node,
    ) -> crate::value::reference_resolution::ReferenceLinkEvaluation<Self::Node> {
        crate::value::reference_resolution::ReferenceLinkEvaluation::Pending(
            "input-expression-consumer-not-ready".into(),
        )
    }
    /// Original lexical schema, when the host exposes one. Never substitute the
    /// consuming model's aliases for the selected node's declaring context.
    fn declaring_schema(&self, _node: &SchemaDeclarationNode) -> Option<SchemaDeclarationNode> {
        None
    }
}
impl<H: SchemaDeclarationHost> InputReferenceHost for H {
    fn source_node(&self, source: SchemaDeclarationNode) -> Self::Node {
        self.source_reference(source)
    }
    fn retained_node(&self, node: &Self::Node) -> Option<SchemaDeclarationNode> {
        self.declaration_node(node)
    }
    fn declaring_schema(&self, node: &SchemaDeclarationNode) -> Option<SchemaDeclarationNode> {
        self.declaration_schema(node)
    }
    fn evaluate_input_expression(
        &mut self,
        expression: &Self::Node,
    ) -> crate::value::reference_resolution::ReferenceLinkEvaluation<Self::Node> {
        SchemaDeclarationHost::evaluate_input_expression(self, expression)
    }
}

/// Native authored expression metadata, without parsing or executing its text.
/// Returned expression-looking targets remain ordinary nodes; only source slots
/// are passed to the lifecycle hook.
pub fn native_attribute_expression(
    source: &SchemaDeclarationNode,
) -> Option<super::reference_policy::ReferenceOccurrence> {
    let CemAstNode::Element {
        expanded_name,
        attributes,
        children,
        source: provenance,
        ..
    } = source.node()
    else {
        return None;
    };
    if expanded_name.local_name != "$" || !attributes.is_empty() {
        return None;
    }
    let mut expression = String::new();
    for child in children {
        let CemAstNode::Text { data, .. } = source.document().get(*child)? else {
            return None;
        };
        expression.push_str(data);
    }
    Some(super::reference_policy::ReferenceOccurrence {
        identity: source.identity(),
        node_id: Some(source.node_id()),
        expression: Some(expression),
        source_map: provenance.clone(),
    })
}

#[derive(Debug, Clone)]
pub struct StructuralValidationNode {
    pub source: SchemaDeclarationNode,
    /// Filled by the explicit behavior stage; structural-only validation does
    /// not ask the host to establish additional lexical schema context.
    pub declaring_schema: Option<SchemaDeclarationNode>,
    pub children: Vec<usize>,
    pub children_complete: bool,
    pub attribute_values: Vec<super::attribute_references::ConsumedAttributeValue>,
}
#[derive(Debug, Clone)]
pub struct StructuralInputValidation<N> {
    pub source: Arc<CemDocument>,
    pub nodes: Vec<StructuralValidationNode>,
    pub roots: Vec<usize>,
    pub references: Vec<ReferenceStructureResolution<N>>,
    pub diagnostics: Vec<Diagnostic>,
    /// Completeness is independent of schema violations in available nodes.
    pub complete: bool,
    pub failed: bool,
}

/// Read-only consumer graph over original source arenas. Indices identify
/// placements, so repeated selections remain distinct without cloning nodes.
/// Original ancestry/lexical schema is available through retained owner handles;
/// consumed parent/child relationships are represented by graph edges.
#[derive(Debug, Clone, Copy)]
pub struct RetainedValidationStructure<'a> {
    /// Overall readiness of the explicit consumer stage, including pending roots.
    pub complete: bool,
    pub source: &'a Arc<CemDocument>,
    pub nodes: &'a [StructuralValidationNode],
    pub roots: &'a [usize],
}

/// A consuming model's execution domain over the complete authorized forest.
/// Indices are placements, not original AST IDs. Navigation keeps all retained
/// edges; only these placements are eligible for this model's behavior execution.
#[derive(Debug, Clone, Copy)]
pub struct RetainedBehaviorRegion<'a> {
    pub structure: RetainedValidationStructure<'a>,
    pub placements: &'a [usize],
}

/// Deferred/unsupported behavior is incomplete, independently of violations.
#[derive(Debug, Clone, Default)]
pub struct RetainedBehaviorValidation {
    pub diagnostics: Vec<Diagnostic>,
    pub complete: bool,
}

impl<N> StructuralInputValidation<N> {
    pub fn structure(&self) -> RetainedValidationStructure<'_> {
        RetainedValidationStructure {
            complete: self.complete,
            source: &self.source,
            nodes: &self.nodes,
            roots: &self.roots,
        }
    }
}

/// Explicit retained behavior stage after complete structural selection. This
/// never routes selected nodes through the legacy whole-document hook.
pub fn validate_structural_input_references_with_behavior_evaluator<H: InputReferenceHost>(
    source: Arc<CemDocument>,
    model: &SchemaDocumentModel,
    host: &mut H,
    limits: ReferenceTraversalLimits,
    evaluator: Option<&dyn SchemaBehaviorEvaluator>,
) -> Result<StructuralInputValidation<H::Node>, ReferenceResolutionError> {
    let mut report = validate_structural_input_references(source, model, host, limits)?;
    if report.complete {
        if let Some(evaluator) = evaluator {
            for node in &mut report.nodes {
                node.declaring_schema = host.declaring_schema(&node.source);
            }
            report.diagnostics.extend(evaluator.compile_model(model));
            let behavior = evaluator.validate_retained_structure(report.structure(), model);
            report.complete &= behavior.complete;
            report.diagnostics.extend(behavior.diagnostics);
            report.failed |= report
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation());
        }
    }
    Ok(report)
}

/// Validate structural children and selected subtrees under the consuming
/// schema. Each authored reference starts one bounded traversal request; its
/// nested selected subtree shares that request's budgets and active identities.
/// Non-element content stays retained and contributes no element count, just
/// like authored content. Attribute/document/error targets are invalid here.
/// Incomplete child selections defer field contracts rather than looking empty.
/// This API performs structural checks; it does not schedule engine requests or
/// run whole-document behavior hooks against a synthetic expanded document.
pub fn validate_structural_input_references<H: InputReferenceHost>(
    source: Arc<CemDocument>,
    model: &SchemaDocumentModel,
    host: &mut H,
    limits: ReferenceTraversalLimits,
) -> Result<StructuralInputValidation<H::Node>, ReferenceResolutionError> {
    let roots = match source.root() {
        Some(CemAstNode::Document { root_children, .. }) => root_children.clone(),
        _ => vec![],
    };
    validate_structural_input_roots_references(source, &roots, model, host, limits)
}

/// Validate an explicit forest of original structural roots. Root IDs are arena
/// addresses in `source`, not authored IDs. This shares the full-document walk,
/// readiness gates, reference bounds and retained placement reporting; no source
/// subtree is copied or installed as a synthetic document. It neither recognizes
/// scope syntax nor assigns parent/child schema ownership at a region boundary.
pub fn validate_structural_input_roots_references<H: InputReferenceHost>(
    source: Arc<CemDocument>,
    roots: &[crate::parser::AstNodeId],
    model: &SchemaDocumentModel,
    host: &mut H,
    limits: ReferenceTraversalLimits,
) -> Result<StructuralInputValidation<H::Node>, ReferenceResolutionError> {
    validate_structural_input_regions_references(source, roots, model, &[], host, limits)
}

/// Validate caller-declared child schema regions in one retained placement walk.
/// Hosts keep their enclosing attribute and direct child-sequence contracts;
/// descendants use the child model, and unavailable overrides block their body.
/// Region lookup uses original owners/addresses, including selected subtrees.
/// Syntax recognition and effective runtime reference policies remain host stages.
pub fn validate_structural_input_regions_references<H: InputReferenceHost>(
    source: Arc<CemDocument>,
    roots: &[crate::parser::AstNodeId],
    model: &SchemaDocumentModel,
    regions: &[InputSchemaRegion<'_>],
    host: &mut H,
    limits: ReferenceTraversalLimits,
) -> Result<StructuralInputValidation<H::Node>, ReferenceResolutionError> {
    validate_structural_input_controlled_regions_references(
        source,
        roots,
        model,
        regions,
        &[],
        host,
        limits,
    )
}

/// Shared schema controls validate their own shape and readiness. Only their
/// original owner-checked attributes bypass application type/unknown-attribute
/// checks and ordinary native-value consumption. Unclassified attributes defer
/// while metadata is pending. Authored presence stays visible in both cases.
pub fn validate_structural_input_controlled_regions_references<H: InputReferenceHost>(
    source: Arc<CemDocument>,
    roots: &[crate::parser::AstNodeId],
    model: &SchemaDocumentModel,
    regions: &[InputSchemaRegion<'_>],
    controls: &[super::scope_controls::SchemaHostControlContract],
    host: &mut H,
    limits: ReferenceTraversalLimits,
) -> Result<StructuralInputValidation<H::Node>, ReferenceResolutionError> {
    validate_input_regions_scheduled(
        source,
        roots,
        model,
        regions,
        controls,
        host,
        limits,
        false,
        &mut |_: &mut H, _: SchemaDeclarationNode, _: ReferenceTraversalLimits| Ok(None),
        None,
    )
}

/// Discover host controls only when entering original structural placements.
/// Scheduling is explicit lifecycle work over stable caller inputs. Selected
/// subtrees retain the same structural reference traversal; no pre-scan or retry
/// walk evaluates input references twice. Each original host is scheduled once
/// per invocation, even if selected into several consuming placements.
pub fn validate_structural_input_discovering_regions_references<H, D>(
    source: Arc<CemDocument>,
    roots: &[crate::parser::AstNodeId],
    model: &SchemaDocumentModel,
    host: &mut H,
    limits: ReferenceTraversalLimits,
    mut scheduler: D,
) -> Result<StructuralInputValidation<H::Node>, ReferenceResolutionError>
where
    H: InputReferenceHost,
    D: FnMut(
        &mut H,
        SchemaDeclarationNode,
        ReferenceTraversalLimits,
    ) -> Result<Option<DiscoveredInputSchemaRegion>, ReferenceResolutionError>,
{
    validate_input_regions_scheduled(
        source,
        roots,
        model,
        &[],
        &[],
        host,
        limits,
        true,
        &mut scheduler,
        None,
    )
}
/// Complete structural consumption followed by model-specific retained behavior
/// execution, while the host's invocation frames are still installed. Pending
/// forests defer this entire behavior stage. Navigation remains the authorized
/// full forest; execution eligibility is the supplied consuming-model domain.
pub fn validate_structural_input_discovering_regions_references_with_behavior_evaluator<H, D>(
    source: Arc<CemDocument>,
    roots: &[crate::parser::AstNodeId],
    model: &SchemaDocumentModel,
    host: &mut H,
    limits: ReferenceTraversalLimits,
    mut scheduler: D,
    evaluator: Option<&dyn SchemaBehaviorEvaluator>,
) -> Result<StructuralInputValidation<H::Node>, ReferenceResolutionError>
where
    H: InputReferenceHost,
    D: FnMut(
        &mut H,
        SchemaDeclarationNode,
        ReferenceTraversalLimits,
    ) -> Result<Option<DiscoveredInputSchemaRegion>, ReferenceResolutionError>,
{
    validate_input_regions_scheduled(
        source,
        roots,
        model,
        &[],
        &[],
        host,
        limits,
        true,
        &mut scheduler,
        evaluator,
    )
}

fn validate_input_regions_scheduled<H, D>(
    source: Arc<CemDocument>,
    roots: &[crate::parser::AstNodeId],
    model: &SchemaDocumentModel,
    regions: &[InputSchemaRegion<'_>],
    controls: &[super::scope_controls::SchemaHostControlContract],
    host: &mut H,
    limits: ReferenceTraversalLimits,
    discovering: bool,
    scheduler: &mut D,
    evaluator: Option<&dyn SchemaBehaviorEvaluator>,
) -> Result<StructuralInputValidation<H::Node>, ReferenceResolutionError>
where
    H: InputReferenceHost,
    D: FnMut(&mut H, SchemaDeclarationNode, ReferenceTraversalLimits)
        -> Result<Option<DiscoveredInputSchemaRegion>, ReferenceResolutionError>,
{
    let mut report = StructuralInputValidation {
        source: source.clone(),
        nodes: vec![],
        roots: vec![],
        references: vec![],
        diagnostics: vec![],
        complete: true,
        failed: false,
    };
    if !model.is_ready_for_validation()
        || model
            .compile_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        report.complete = false;
        report.failed = true;
        report.diagnostics = model.validation_blocker_diagnostics();
        return Ok(report);
    }
    if limits.max_depth == 0 || limits.max_work == 0 {
        return Err(ReferenceResolutionError::InvalidBounds);
    }
    let mut models =
        match RegionModels::new(model, regions).and_then(|models| models.with_controls(controls)) {
            Ok(models) => models,
            Err(diagnostics) => {
                report.complete = false;
                report.failed = true;
                report.diagnostics = diagnostics;
                return Ok(report);
            }
        };
    if discovering {
        models.enable_discovery();
    }
    let mut node_models = vec![];
    let mut seen_roots = HashSet::new();
    for id in roots {
        let node = source.get(*id);
        if !seen_roots.insert(*id)
            || !node.is_some_and(|node| {
                structural_target(node) || matches!(node, CemAstNode::Reference { .. })
            })
        {
            report.complete = false;
            report.failed = true;
            let provenance = node
                .or_else(|| source.root())
                .map(document_model::source_stack_for_node)
                .cloned()
                .unwrap_or_default();
            report.diagnostics.push(invalid_target(
                "Expected distinct original structural root handles",
                provenance,
            ));
        }
    }
    if !report.complete || (model.is_empty() && regions.is_empty() && !discovering) {
        return Ok(report);
    }
    let mut pending: Vec<_> = roots.iter().rev().map(|id| (*id, None, 0)).collect();
    let mut owned = HashSet::new();
    while let Some((id, parent, model_index)) = pending.pop() {
        let Some(handle) = SchemaDeclarationNode::new(source.clone(), id) else {
            continue;
        };
        if !owned.insert(id) {
            report.complete = false;
            report.failed = true;
            report.diagnostics.push(invalid_target(
                "Invalid repeated owning child edge",
                document_model::source_stack_for_node(handle.node()).clone(),
            ));
            mark_incomplete(&mut report, parent);
            continue;
        }
        let mut entered = host.source_node(handle.clone());
        // Direct owning placements have no reference resolver entry hook.
        // References are handed off by their bounded consuming walk instead.
        if !matches!(handle.node(), CemAstNode::Reference { .. }) {
            host.prepare_node(&mut entered)?;
        }
        if matches!(handle.node(), CemAstNode::Reference { .. }) {
            let consumed = consumed_walk::walk_regions_scheduled(
                entered,
                &mut models,
                model_index,
                host,
                limits,
                None,
                scheduler,
            )?;
            let mut resolved = consumed.structure;
            let mut indices = vec![];
            for (index, target) in resolved.resolution.nodes.iter().enumerate() {
                let retained = host
                    .retained_node(target)
                    .filter(|n| structural_target(n.node()));
                if let Some(retained) = retained {
                    let position = report.nodes.len();
                    report.nodes.push(StructuralValidationNode {
                        declaring_schema: None,
                        source: retained,
                        children: vec![],
                        children_complete: resolved.children_complete[index],
                        attribute_values: vec![],
                    });
                    node_models.push(consumed.models[index]);
                    indices.push(Some(position));
                } else {
                    resolved.resolution.state = ReferenceResolutionState::Invalid;
                    resolved.resolution.failed = true;
                    let diagnostic = invalid_target(
                        "Expected an original structural child node",
                        resolved.origins[index].source_map.clone(),
                    );
                    resolved.resolution.diagnostics.push(diagnostic);
                    if let Some(parent) = resolved.parents[index] {
                        resolved.children_complete[parent] = false;
                    } else {
                        resolved.roots_complete = false;
                    }
                    indices.push(None);
                }
            }
            for (index, position) in indices.iter().enumerate() {
                if let Some(position) = position {
                    report.nodes[*position].children = resolved.children[index]
                        .iter()
                        .filter_map(|child| indices[*child])
                        .collect();
                    report.nodes[*position].children_complete = resolved.children_complete[index];
                }
            }
            for (owner, value) in consumed.attributes {
                report.complete &= value.complete;
                if let Some(position) = owner.and_then(|owner| indices[owner]) {
                    report.nodes[position].attribute_values.push(value);
                }
            }
            for index in &resolved.roots {
                if let Some(position) = indices[*index] {
                    attach(&mut report, parent, position);
                }
            }
            if !resolved.roots_complete {
                mark_incomplete(&mut report, parent);
            }
            report.complete &= resolved.resolution.is_complete();
            report.failed |= resolved.resolution.failed;
            report
                .diagnostics
                .extend(resolved.resolution.diagnostics.iter().cloned());
            report.references.push(resolved);
        } else {
            models.discover(&handle, host, limits, scheduler)?;
            let model = models.model(model_index);
            let position = report.nodes.len();
            let (children, child_model, children_complete) = models.children(model_index, &handle);
            let children = children.to_vec();
            report.complete &= children_complete;
            report.diagnostics.extend(models.blockers(&handle));
            let mut attribute_values = vec![];
            if let CemAstNode::Element {
                expanded_name,
                attributes,
                ..
            } = handle.node()
            {
                if let Some(element) = model.element(&expanded_name.local_name) {
                    for id in attributes {
                        if models
                            .control_attributes(&handle)
                            .is_some_and(|attributes| attributes.contains(id))
                        {
                            continue;
                        }
                        let Some(attribute) =
                            SchemaDeclarationNode::new(handle.document().clone(), *id)
                        else {
                            continue;
                        };
                        let CemAstNode::Attribute {
                            expanded_name: name,
                            value_nodes,
                            ..
                        } = attribute.node()
                        else {
                            continue;
                        };
                        if value_nodes.is_empty() {
                            continue;
                        }
                        let eligible = element
                            .allows_attribute(&name.namespace_uri, &name.local_name)
                            && model
                                .attributes
                                .get(&name.local_name)
                                .is_some_and(|contract| contract.is_node_valued());
                        let root = if eligible {
                            consumed_walk::value_request_root(&attribute)
                        } else {
                            None
                        };
                        if let Some(root) = root {
                            let consumed = consumed_walk::walk(
                                host.source_node(root),
                                model,
                                host,
                                limits,
                                Some((attribute, expanded_name.local_name.clone())),
                            )?;
                            report.failed |= consumed.structure.resolution.failed;
                            report
                                .diagnostics
                                .extend(consumed.structure.resolution.diagnostics);
                            for (_, value) in consumed.attributes {
                                report.complete &= value.complete;
                                attribute_values.push(value);
                            }
                        } else {
                            report.complete = false;
                        }
                    }
                }
            }
            report.nodes.push(StructuralValidationNode {
                declaring_schema: None,
                source: handle,
                children: vec![],
                children_complete,
                attribute_values,
            });
            node_models.push(model_index);
            attach(&mut report, parent, position);
            pending.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|id| (id, Some(position), child_model)),
            );
        }
    }
    let mut pending: Vec<_> = report.roots.iter().rev().map(|id| (*id, false)).collect();
    while let Some((index, allows_any)) = pending.pop() {
        let current = &report.nodes[index];
        let model = models.model(node_models[index]);
        let boundary = models.is_boundary(&current.source);
        let sequence: Vec<_> = current
            .children
            .iter()
            .filter_map(|child| {
                document_model::structural_child_name(report.nodes[*child].source.node())
                    .map(str::to_owned)
            })
            .collect();
        let element = if model.is_empty() {
            None
        } else {
            document_model::validate_element_shallow_with_controls(
                current.source.document(),
                model,
                current.source.node_id(),
                allows_any,
                current.children_complete.then_some(sequence.as_slice()),
                models.control_attributes(&current.source),
                &mut report.diagnostics,
            )
        };
        for child in &current.children {
            document_model::validate_child_relationship(
                model,
                current.source.node(),
                report.nodes[*child].source.node(),
                &mut report.diagnostics,
            );
        }
        pending.extend(current.children.iter().rev().map(|child| {
            (
                *child,
                !boundary && element.is_some_and(|element| element.allow_any_child),
            )
        }));
    }
    if let Some(evaluator) = evaluator {
        behaviors::validate(&mut report, &node_models, &models, host, evaluator);
    }
    report.failed |= report
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation());
    Ok(report)
}

fn consumable_children<'a>(
    node: &'a CemAstNode,
    model: &SchemaDocumentModel,
) -> Option<&'a [crate::parser::AstNodeId]> {
    let name = document_model::structural_child_name(node)?;
    model.element(name)?;
    match node {
        CemAstNode::Element { children, .. } => Some(children),
        _ => None,
    }
}
fn structural_target(node: &CemAstNode) -> bool {
    matches!(
        node,
        CemAstNode::Element { .. }
            | CemAstNode::Text { .. }
            | CemAstNode::Whitespace { .. }
            | CemAstNode::Comment { .. }
            | CemAstNode::ProcessingInstruction { .. }
            | CemAstNode::Cdata { .. }
            | CemAstNode::RawText { .. }
    )
}
fn attach<N>(report: &mut StructuralInputValidation<N>, parent: Option<usize>, child: usize) {
    if let Some(parent) = parent {
        report.nodes[parent].children.push(child);
    } else {
        report.roots.push(child);
    }
}
fn mark_incomplete<N>(report: &mut StructuralInputValidation<N>, parent: Option<usize>) {
    if let Some(parent) = parent {
        report.nodes[parent].children_complete = false;
    }
}
fn invalid_target(message: &str, source_map: crate::source_map::SourceMapStack) -> Diagnostic {
    Diagnostic {
        uri: None,
        line: None,
        column: None,
        byte_offset: None,
        code: INVALID_STRUCTURAL_TARGET.into(),
        severity: Severity::Error,
        message: message.into(),
        node: None,
        details: None,
        source_map: Some(source_map),
    }
}
