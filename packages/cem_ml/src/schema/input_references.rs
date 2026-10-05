//! Explicit structural validation over retained source and selected subtrees.
//! Runtime hosts choose context and timing; source arenas are never rewritten.
use super::{
    declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
    document_model::{self, SchemaDocumentModel},
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

pub const INVALID_STRUCTURAL_TARGET: &str = "cem.schema_validation.invalid_reference_target";

/// Retained-node adaptation is independent of schema-declaration lookup.
pub trait InputReferenceHost: ReferenceResolutionHost {
    fn source_node(&self, source: SchemaDeclarationNode) -> Self::Node;
    fn retained_node(&self, node: &Self::Node) -> Option<SchemaDeclarationNode>;
}
impl<H: SchemaDeclarationHost> InputReferenceHost for H {
    fn source_node(&self, source: SchemaDeclarationNode) -> Self::Node {
        self.source_reference(source)
    }
    fn retained_node(&self, node: &Self::Node) -> Option<SchemaDeclarationNode> {
        self.declaration_node(node)
    }
}

#[derive(Debug, Clone)]
pub struct StructuralValidationNode {
    pub source: SchemaDeclarationNode,
    pub children: Vec<usize>,
    pub children_complete: bool,
}
#[derive(Debug, Clone)]
pub struct StructuralInputValidation<N> {
    pub nodes: Vec<StructuralValidationNode>,
    pub roots: Vec<usize>,
    pub references: Vec<ReferenceStructureResolution<N>>,
    pub diagnostics: Vec<Diagnostic>,
    /// Completeness is independent of schema violations in available nodes.
    pub complete: bool,
    pub failed: bool,
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
        nodes: vec![],
        roots: vec![],
        references: vec![],
        diagnostics: vec![],
        complete: true,
        failed: false,
    };
    if !model.is_ready_for_validation() {
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
            let mut resolved = resolve_reference_structure(
                host.source_node(handle),
                host,
                limits,
                |host, target| {
                    let retained = host.retained_node(target)?;
                    let children = consumable_children(retained.node(), model)?;
                    Some(
                        children
                            .iter()
                            .filter_map(|id| {
                                SchemaDeclarationNode::new(retained.document().clone(), *id)
                            })
                            .map(|child| host.source_node(child))
                            .collect(),
                    )
                },
            )?;
            let mut indices = vec![];
            for (index, target) in resolved.resolution.nodes.iter().enumerate() {
                let retained = host
                    .retained_node(target)
                    .filter(|n| structural_target(n.node()));
                if let Some(retained) = retained {
                    let position = report.nodes.len();
                    report.nodes.push(StructuralValidationNode {
                        source: retained,
                        children: vec![],
                        children_complete: resolved.children_complete[index],
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
            report.nodes.push(StructuralValidationNode {
                source: handle,
                children: vec![],
                children_complete: true,
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
