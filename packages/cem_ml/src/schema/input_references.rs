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
        resolve_reference_structure, ReferenceResolutionError, ReferenceResolutionHost,
        ReferenceResolutionState, ReferenceStructureResolution,
    },
};
use std::{collections::HashSet, sync::Arc};

mod consumed_walk;

pub const INVALID_STRUCTURAL_TARGET: &str = "cem.schema_validation.invalid_reference_target";

/// Retained-node adaptation is independent of schema-declaration lookup.
pub trait InputReferenceHost: ReferenceResolutionHost {
    fn source_node(&self, source: SchemaDeclarationNode) -> Self::Node;
    fn retained_node(&self, node: &Self::Node) -> Option<SchemaDeclarationNode>;
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
    if model.is_empty() {
        return Ok(report);
    }
    let Some(CemAstNode::Document { root_children, .. }) = source.root() else {
        return Ok(report);
    };
    let mut pending: Vec<_> = root_children.iter().rev().map(|id| (*id, None)).collect();
    let mut owned = HashSet::new();
    while let Some((id, parent)) = pending.pop() {
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
        if matches!(handle.node(), CemAstNode::Reference { .. }) {
            let consumed =
                consumed_walk::walk(host.source_node(handle), model, host, limits, None)?;
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
            let position = report.nodes.len();
            let children = consumable_children(handle.node(), model)
                .unwrap_or(&[])
                .to_vec();
            let mut attribute_values = vec![];
            if let CemAstNode::Element {
                expanded_name,
                attributes,
                ..
            } = handle.node()
            {
                if let Some(element) = model.element(&expanded_name.local_name) {
                    for id in attributes {
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
                        let root = if eligible && value_nodes.len() == 1 {
                            SchemaDeclarationNode::new(attribute.document().clone(), value_nodes[0])
                                .filter(|node| matches!(node.node(), CemAstNode::Reference { .. }))
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
                children_complete: true,
                attribute_values,
            });
            attach(&mut report, parent, position);
            pending.extend(children.into_iter().rev().map(|id| (id, Some(position))));
        }
    }
    let mut pending: Vec<_> = report.roots.iter().rev().map(|id| (*id, false)).collect();
    while let Some((index, allows_any)) = pending.pop() {
        let current = &report.nodes[index];
        let sequence: Vec<_> = current
            .children
            .iter()
            .filter_map(|child| {
                document_model::structural_child_name(report.nodes[*child].source.node())
                    .map(str::to_owned)
            })
            .collect();
        let Some(element) = document_model::validate_element_shallow(
            current.source.document(),
            model,
            current.source.node_id(),
            allows_any,
            current.children_complete.then_some(sequence.as_slice()),
            &mut report.diagnostics,
        ) else {
            continue;
        };
        for child in &current.children {
            document_model::validate_child_relationship(
                model,
                current.source.node(),
                report.nodes[*child].source.node(),
                &mut report.diagnostics,
            );
        }
        pending.extend(
            current
                .children
                .iter()
                .rev()
                .map(|child| (*child, element.allow_any_child)),
        );
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
