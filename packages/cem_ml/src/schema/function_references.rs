//! Passive, retained function selection. A selected declaration is not an
//! executable registration: signature/profile binding and activation are separate.
use super::{
    declaration_references::{
        DeclarationReferenceIssue, DeclarationReferenceResolution, SchemaDeclarationHost,
        SchemaDeclarationNode,
    },
    document_model::source_stack_for_node,
    reference_policy::ReferenceOccurrence,
    reference_traversal::ReferenceTraversalLimits,
    value_contracts::{ValueContractError, ValueContractSource},
};
use crate::{
    diagnostics::{Diagnostic, Severity},
    parser::CemAstNode,
    value::reference_resolution::{
        resolve_reference, ReferenceLinkEvaluation, ReferenceResolutionIssueKind,
        ReferenceResolutionState,
    },
};
use std::collections::BTreeMap;

mod catalog;
mod resolver;
mod assembly;
pub use catalog::{FunctionCatalog, FunctionDeclaration};

/// The caller shares this allowance across collection, lookup, selection and
/// selected signature/body inspection.
/// Resolver destination policies still apply independently on every invocation.
#[derive(Debug)]
pub struct FunctionSelectionBudget {
    limits: ReferenceTraversalLimits,
    used: usize,
}
/// One compilation allowance may span function and datatype consumers.
pub type ScalarCompilationBudget = FunctionSelectionBudget;
impl FunctionSelectionBudget {
    pub fn new(limits: ReferenceTraversalLimits) -> Result<Self, ValueContractError> {
        if limits.max_work == 0 || limits.max_depth == 0 {
            return Err(ValueContractError::new("invalid-function-bounds"));
        }
        Ok(Self { limits, used: 0 })
    }
    pub fn work_used(&self) -> usize {
        self.used
    }
    pub fn remaining_limits(&self) -> ReferenceTraversalLimits {
        ReferenceTraversalLimits {
            max_depth: self.limits.max_depth,
            max_work: self.limits.max_work - self.used,
        }
    }
    /// Account work performed by another admitted compilation phase. Exhaustion
    /// consumes the remaining allowance and retains the responsible source.
    pub fn spend(
        &mut self,
        count: usize,
        source: &SchemaDeclarationNode,
    ) -> Result<(), ValueContractError> {
        if count > self.limits.max_work - self.used {
            self.used = self.limits.max_work;
            return Err(ValueContractError::at("function-work-limit", source));
        }
        self.used += count;
        Ok(())
    }
    /// Bound passive signature/body inspection before the existing profile
    /// validators walk original source fields. No AST expansion or execution.
    pub(crate) fn inspect(
        &mut self,
        root: &SchemaDeclarationNode,
    ) -> Result<(), ValueContractError> {
        let mut stack = vec![(root.node_id(), 0)];
        while let Some((id, depth)) = stack.pop() {
            let node = SchemaDeclarationNode::new(root.document().clone(), id)
                .ok_or_else(|| ValueContractError::at("invalid-function-source-edge", root))?;
            if depth >= self.limits.max_depth {
                return Err(ValueContractError::at("function-depth-limit", &node));
            }
            self.spend(1, &node)?;
            match node.node() {
                CemAstNode::Element {
                    attributes,
                    children,
                    ..
                } => {
                    self.spend(attributes.len().saturating_add(children.len()), &node)?;
                    stack.extend(attributes.iter().chain(children).map(|id| (*id, depth + 1)));
                }
                CemAstNode::Attribute {
                    value, value_nodes, ..
                } => {
                    self.spend(
                        value
                            .as_ref()
                            .map_or(0, String::len)
                            .saturating_add(value_nodes.len()),
                        &node,
                    )?;
                    stack.extend(value_nodes.iter().map(|id| (*id, depth + 1)));
                }
                CemAstNode::Text { data, .. } | CemAstNode::RawText { data, .. } => {
                    self.spend(data.len(), &node)?
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FunctionSelection {
    pub attribute: SchemaDeclarationNode,
    pub resolution: DeclarationReferenceResolution,
    selected: Option<FunctionDeclaration>,
    caller: SchemaDeclarationNode,
    caller_source: ValueContractSource,
    binding_attribute: SchemaDeclarationNode,
}
impl FunctionSelection {
    /// Complete selection only; this grants no invocation authority or readiness
    /// to the enclosing behavior/schema package.
    pub fn target(&self) -> Option<&FunctionDeclaration> {
        (self.resolution.state == ReferenceResolutionState::Resolved && !self.resolution.failed)
            .then_some(self.selected.as_ref())
            .flatten()
    }
    pub(crate) fn binding(
        &self,
    ) -> Result<
        (
            &ValueContractSource,
            &SchemaDeclarationNode,
            &SchemaDeclarationNode,
            &FunctionDeclaration,
        ),
        ValueContractError,
    > {
        let target = self.target().ok_or_else(|| {
            ValueContractError::at("function-selection-incomplete", &self.binding_attribute)
        })?;
        Ok((
            &self.caller_source,
            &self.caller,
            &self.binding_attribute,
            target,
        ))
    }
}

impl FunctionCatalog {
    pub fn select<H: SchemaDeclarationHost>(
        &self,
        caller: &SchemaDeclarationNode,
        host: &mut H,
        budget: &mut FunctionSelectionBudget,
    ) -> Result<FunctionSelection, ValueContractError> {
        let source = self
            .behaviors
            .get(&caller.identity())
            .ok_or_else(|| ValueContractError::at("function-caller-unavailable", caller))?;
        let fields = fields(
            source,
            caller,
            &["name", "implementation", "function"],
            budget,
        )?;
        required_name(&fields, caller)?;
        if literal(required(&fields, "implementation", caller)?)? != "function" {
            return Err(ValueContractError::at(
                "function-implementation-required",
                caller,
            ));
        }
        let attribute = required(&fields, "function", caller)?.clone();
        let CemAstNode::Attribute {
            value_nodes, value, ..
        } = attribute.node()
        else {
            unreachable!()
        };
        let (root, lookup) = if value_nodes.is_empty() {
            let name = value
                .as_deref()
                .map(str::trim)
                .filter(|name| qname(name))
                .ok_or_else(|| ValueContractError::at("invalid-function-name", &attribute))?;
            let lookup = self.lookup(source, caller, name, &attribute, budget)?;
            let lookup = match lookup {
                ReferenceLinkEvaluation::Resolved(nodes) => ReferenceLinkEvaluation::Resolved(
                    nodes
                        .into_iter()
                        .map(|node| host.source_reference(node))
                        .collect(),
                ),
                ReferenceLinkEvaluation::Pending(reason) => {
                    ReferenceLinkEvaluation::Pending(reason)
                }
                ReferenceLinkEvaluation::Unresolved(reason) => {
                    ReferenceLinkEvaluation::Unresolved(reason)
                }
                ReferenceLinkEvaluation::Invalid(diagnostics) => ReferenceLinkEvaluation::Invalid(
                    diagnostics
                        .into_iter()
                        .map(|diagnostic| host.structural_diagnostic(&attribute, diagnostic))
                        .collect(),
                ),
            };
            (
                resolver::Node::Literal(host.source_reference(attribute.clone())),
                Some(lookup),
            )
        } else {
            let [id] = value_nodes.as_slice() else {
                return Err(ValueContractError::at(
                    "function-slot-reference-required",
                    &attribute,
                ));
            };
            if value.as_deref().is_some_and(|s| !s.is_empty())
                || !matches!(
                    attribute.document().get(*id),
                    Some(CemAstNode::Reference { .. })
                )
            {
                return Err(ValueContractError::at(
                    "function-slot-reference-required",
                    &attribute,
                ));
            }
            let reference = SchemaDeclarationNode::new(attribute.document().clone(), *id).unwrap();
            (
                resolver::Node::Value(host.source_reference(reference)),
                None,
            )
        };
        budget.spend(1, &attribute)?;
        let remaining = budget.limits.max_work - budget.used;
        if remaining == 0 {
            return Err(ValueContractError::at("function-work-limit", &attribute));
        }
        let origin = occurrence(&attribute);
        let mut adapter = resolver::Host {
            host,
            literal: lookup,
            origin: origin.clone(),
        };
        let result = resolve_reference(
            root,
            &mut adapter,
            ReferenceTraversalLimits {
                max_depth: budget.limits.max_depth,
                max_work: remaining,
            },
        )
        .map_err(|_| ValueContractError::at("function-resolution-failed", &attribute))?;
        budget.spend(result.work_used, &attribute)?;
        let count = result.nodes.len();
        let mut resolution = DeclarationReferenceResolution {
            nodes: vec![],
            state: result.state,
            failed: result.failed,
            issues: result
                .issues
                .into_iter()
                .map(|issue| DeclarationReferenceIssue {
                    occurrence: issue.occurrence,
                    kind: issue.kind,
                    reason: issue.reason,
                })
                .collect(),
            diagnostics: result.diagnostics,
            work_used: result.work_used,
        };
        let mut selected = None;
        let evaluator_diagnostics = resolution.diagnostics.len();
        if resolution.state == ReferenceResolutionState::Resolved && count != 1 {
            invalidate(&mut resolution, &attribute, "function-cardinality", None);
        }
        for node in result.nodes {
            let target = host.declaration_node(node.value());
            let Some(target) = target else {
                invalidate(
                    &mut resolution,
                    &attribute,
                    "function-target-required",
                    None,
                );
                continue;
            };
            resolution.nodes.push(target.clone());
            if !matches!(target.node(), CemAstNode::Element { .. }) {
                invalidate(
                    &mut resolution,
                    &attribute,
                    "function-target-required",
                    Some(&target),
                );
                continue;
            }
            if let Err(error) = budget.spend(self.sources.len(), &target) {
                invalidate(&mut resolution, &attribute, error.code, Some(&target));
                break;
            }
            let target_source = self.source_for(&target);
            if target_source.is_none() {
                if host.input_expanded_name(&target).is_some_and(|name| {
                    name.local_name != "function"
                        || (!name.namespace_uri.is_empty()
                            && name.namespace_uri != super::registry::CEM_SCHEMA_URI)
                }) {
                    invalidate(
                        &mut resolution,
                        &attribute,
                        "function-target-required",
                        Some(&target),
                    );
                } else {
                    defer(&mut resolution, &origin, "function-source-unavailable");
                }
                continue;
            }
            let target_name = target_source.and_then(|source| source.name(&target));
            if let Some(name) = target_name {
                if name.namespace_uri != super::registry::CEM_SCHEMA_URI
                    || name.local_name != "function"
                {
                    invalidate(
                        &mut resolution,
                        &attribute,
                        "function-target-required",
                        Some(&target),
                    );
                    continue;
                }
            } else {
                defer(&mut resolution, &origin, "function-name-pending");
                continue;
            }
            let Some(declaration) = self.functions.get(&target.identity()) else {
                defer(&mut resolution, &origin, "function-source-unavailable");
                continue;
            };
            match declaration.check(caller, source, budget) {
                Ok(()) => selected = Some(declaration.clone()),
                Err(error) => {
                    invalidate(
                        &mut resolution,
                        &attribute,
                        error.code,
                        error.source.as_ref(),
                    );
                }
            }
        }
        for diagnostic in &mut resolution.diagnostics[evaluator_diagnostics..] {
            *diagnostic = host.structural_diagnostic(&attribute, diagnostic.clone());
        }
        Ok(FunctionSelection {
            caller: caller.clone(),
            caller_source: source.clone(),
            binding_attribute: attribute.clone(),
            attribute,
            selected: if resolution.state == ReferenceResolutionState::Resolved
                && !resolution.failed
            {
                selected
            } else {
                None
            },
            resolution,
        })
    }
}

fn occurrence(source: &SchemaDeclarationNode) -> ReferenceOccurrence {
    ReferenceOccurrence {
        identity: format!("function-slot:{}", source.identity()),
        node_id: Some(source.node_id()),
        expression: None,
        source_map: source_stack_for_node(source.node()).clone(),
    }
}
fn diagnostic(
    source: &SchemaDeclarationNode,
    code: &str,
    target: Option<&SchemaDeclarationNode>,
) -> Diagnostic {
    Diagnostic {
        code: format!("cem.schema_definition.{code}"),
        severity: Severity::Error,
        message: format!("Function selection failed: {code}"),
        node: Some(source.identity()),
        source_map: Some(source_stack_for_node(source.node()).clone()),
        details: target.map(|t| {
            serde_json::json!({
                "target": t.identity(), "targetSource": source_stack_for_node(t.node()),
            })
        }),
        ..Default::default()
    }
}
fn invalidate(
    result: &mut DeclarationReferenceResolution,
    source: &SchemaDeclarationNode,
    code: &str,
    target: Option<&SchemaDeclarationNode>,
) {
    result.state = ReferenceResolutionState::Invalid;
    result.failed = true;
    result.diagnostics.push(diagnostic(source, code, target));
}
fn defer(result: &mut DeclarationReferenceResolution, origin: &ReferenceOccurrence, reason: &str) {
    if result.state == ReferenceResolutionState::Resolved {
        result.state = ReferenceResolutionState::Pending;
    }
    result.issues.push(DeclarationReferenceIssue {
        occurrence: origin.clone(),
        kind: ReferenceResolutionIssueKind::Pending,
        reason: reason.into(),
    });
}
fn children(
    source: &SchemaDeclarationNode,
    budget: &mut FunctionSelectionBudget,
) -> Result<Vec<SchemaDeclarationNode>, ValueContractError> {
    let CemAstNode::Element { children, .. } = source.node() else {
        return Err(ValueContractError::at("function-element-required", source));
    };
    budget.spend(children.len(), source)?;
    Ok(children
        .iter()
        .map(|id| SchemaDeclarationNode::new(source.document().clone(), *id).unwrap())
        .collect())
}
fn fields(
    source: &ValueContractSource,
    node: &SchemaDeclarationNode,
    names: &[&str],
    budget: &mut FunctionSelectionBudget,
) -> Result<BTreeMap<String, SchemaDeclarationNode>, ValueContractError> {
    let CemAstNode::Element { attributes, .. } = node.node() else {
        return Err(ValueContractError::at("function-element-required", node));
    };
    budget.spend(attributes.len(), node)?;
    let mut result = BTreeMap::new();
    for id in attributes {
        let attr = SchemaDeclarationNode::new(node.document().clone(), *id).unwrap();
        let name = source
            .name(&attr)
            .ok_or_else(|| ValueContractError::at("function-name-pending", &attr))?;
        if names.contains(&name.local_name.as_str())
            && (name.namespace_uri.is_empty()
                || name.namespace_uri == super::registry::CEM_SCHEMA_URI)
            && result
                .insert(name.local_name.clone(), attr.clone())
                .is_some()
        {
            return Err(ValueContractError::at("duplicate-function-field", &attr));
        }
    }
    Ok(result)
}
fn required<'a>(
    fields: &'a BTreeMap<String, SchemaDeclarationNode>,
    name: &str,
    node: &SchemaDeclarationNode,
) -> Result<&'a SchemaDeclarationNode, ValueContractError> {
    fields
        .get(name)
        .ok_or_else(|| ValueContractError::at("missing-function-field", node))
}
fn literal(node: &SchemaDeclarationNode) -> Result<&str, ValueContractError> {
    match node.node() {
        CemAstNode::Attribute {
            value: Some(value),
            value_nodes,
            ..
        } if value_nodes.is_empty() => Ok(value.trim()),
        _ => Err(ValueContractError::at("function-literal-required", node)),
    }
}
fn required_name(
    fields: &BTreeMap<String, SchemaDeclarationNode>,
    node: &SchemaDeclarationNode,
) -> Result<String, ValueContractError> {
    let field = required(fields, "name", node)?;
    let name = literal(field)?;
    if !identifier(name) {
        return Err(ValueContractError::at("invalid-function-name", field));
    }
    Ok(name.into())
}
fn identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
}
fn qname(value: &str) -> bool {
    match value.split_once(':') {
        Some((prefix, local)) => identifier(prefix) && identifier(local),
        None => identifier(value),
    }
}

/// Consumer-owned executable registrations retained by a package publication.
/// Implementations must retain the original selected functions and their owners.
pub trait CompiledFunctionBindings: std::fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
}
