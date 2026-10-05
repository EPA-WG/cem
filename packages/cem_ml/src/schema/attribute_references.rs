//! Explicit schema consumption of a native attribute's reference slot.
//! Lifecycle hosts supply context; authored nodes are never rewritten.
use super::{
    declaration_references::SchemaDeclarationNode,
    document_model::{self, SchemaDocumentModel},
    input_references::InputReferenceHost,
    reference_traversal::ReferenceTraversalLimits,
};
use crate::{
    diagnostics::{Diagnostic, Severity},
    parser::CemAstNode,
    value::reference_resolution::{
        resolve_reference, ReferenceResolution, ReferenceResolutionError,
    },
};

pub const INVALID_NATIVE_TARGET: &str = "cem.schema_validation.invalid_attribute_reference_target";

/// Results belong to this consumption, not to the source attribute or reference.
#[derive(Debug, Clone)]
pub struct NativeAttributeReferenceValidation<N> {
    pub attribute: SchemaDeclarationNode,
    pub targets: Vec<SchemaDeclarationNode>,
    pub resolution: Option<ReferenceResolution<N>>,
    pub diagnostics: Vec<Diagnostic>,
    pub complete: bool,
    pub failed: bool,
}

/// Consume the single native reference slot produced by the current CEM-ML
/// constructor. The reference chain shares request and destination bounds.
/// General expression slots and composite slots require their own consumer and
/// remain incomplete here. Only complete selections undergo count validation.
/// This entry point starts one independent request; callers consuming slots
/// inside an already-selected subtree must use the enclosing traversal instead
/// of restarting its budget. It does not schedule behavior validation.
pub fn validate_native_attribute_reference<H: InputReferenceHost>(
    attribute: SchemaDeclarationNode,
    model: &SchemaDocumentModel,
    element_name: &str,
    host: &mut H,
    limits: ReferenceTraversalLimits,
) -> Result<NativeAttributeReferenceValidation<H::Node>, ReferenceResolutionError> {
    if limits.max_depth == 0 || limits.max_work == 0 {
        return Err(ReferenceResolutionError::InvalidBounds);
    }
    let CemAstNode::Attribute {
        expanded_name,
        value_nodes,
        ..
    } = attribute.node()
    else {
        return Err(ReferenceResolutionError::NotReference);
    };
    if value_nodes.is_empty() {
        return Err(ReferenceResolutionError::NotReference);
    }
    let mut report = NativeAttributeReferenceValidation {
        attribute: attribute.clone(),
        targets: vec![],
        resolution: None,
        diagnostics: vec![],
        complete: false,
        failed: false,
    };
    if !model.is_ready_for_validation()
        || model
            .compile_diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
    {
        report.diagnostics = model.validation_blocker_diagnostics();
        report.failed = true;
        return Ok(report);
    }
    let name = &expanded_name.local_name;
    document_model::validate_native_attribute_contract(
        model,
        element_name,
        name,
        attribute.node(),
        &mut report.diagnostics,
    );
    if !model
        .attributes
        .get(name)
        .is_some_and(|contract| contract.is_node_valued())
    {
        report.failed = report
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation());
        return Ok(report);
    }
    let [root] = value_nodes.as_slice() else {
        return Ok(report);
    };
    let Some(root) = SchemaDeclarationNode::new(attribute.document().clone(), *root) else {
        return Ok(report);
    };
    if !matches!(root.node(), CemAstNode::Reference { .. }) {
        return Ok(report);
    }
    let resolved = resolve_reference(host.source_node(root), host, limits)?;
    report.complete = resolved.is_complete();
    report.failed = resolved.failed;
    report
        .diagnostics
        .extend(resolved.diagnostics.iter().cloned());
    for target in &resolved.nodes {
        if let Some(retained) = host.retained_node(target) {
            report.targets.push(retained);
        } else {
            report.complete = false;
            report.failed = true;
            report.diagnostics.push(Diagnostic {
                code: INVALID_NATIVE_TARGET.into(),
                severity: Severity::Error,
                message: "Expected an original retained native attribute target".into(),
                source_map: Some(document_model::source_stack_for_node(attribute.node()).clone()),
                ..Diagnostic::default()
            });
        }
    }
    if report.complete {
        document_model::validate_native_attribute_count(
            model,
            element_name,
            name,
            report.targets.len(),
            attribute.node(),
            &mut report.diagnostics,
        );
    }
    if model
        .attributes
        .get(name)
        .is_some_and(has_unconsumed_constraints)
    {
        // Resolving nodes does not consume lexical facets. Keep the report
        // pending rather than silently discarding schema-authored constraints.
        report.complete = false;
    }
    report.failed |= report
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation());
    report.resolution = Some(resolved);
    Ok(report)
}

pub(crate) fn has_unconsumed_constraints(attribute: &document_model::AttributeModel) -> bool {
    let mut remaining = attribute.clone();
    remaining.name.clear();
    remaining.value_type = None;
    remaining.default_value = None;
    remaining.item_count = None;
    remaining.min_items = None;
    remaining.max_items = None;
    remaining.values_diagnostic = None;
    remaining.type_diagnostic = None;
    remaining.datatype_param_diagnostic = None;
    remaining.source_map = Default::default();
    remaining != document_model::AttributeModel::default()
}

/// Read-only authorized source subtree, captured under the consuming request.
/// Private edges prevent query adapters from manufacturing wider authority.
#[derive(Debug, Clone)]
pub struct NativeAttributeTargetAccess {
    pub(crate) nodes: Vec<SchemaDeclarationNode>,
    pub(crate) roots: Vec<usize>,
    pub(crate) parents: Vec<Option<usize>>,
    pub(crate) children: Vec<Vec<usize>>,
    pub(crate) complete: bool,
}
impl NativeAttributeTargetAccess {
    pub fn is_complete(&self) -> bool {
        self.complete
    }
    pub fn roots(&self) -> &[usize] {
        &self.roots
    }
    pub fn node(&self, index: usize) -> Option<&SchemaDeclarationNode> {
        self.nodes.get(index)
    }
    pub fn parent(&self, index: usize) -> Option<usize> {
        self.parents.get(index).copied().flatten()
    }
    pub fn children(&self, index: usize) -> Option<&[usize]> {
        self.children.get(index).map(Vec::as_slice)
    }
}
#[derive(Debug, Clone)]
pub struct ConsumedAttributeValue {
    pub attribute: SchemaDeclarationNode,
    pub access: std::sync::Arc<NativeAttributeTargetAccess>,
    pub complete: bool,
}
