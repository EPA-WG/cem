//! Explicit prepared child regions; source syntax and runtime policy installation
//! remain separate lifecycle stages.
use super::{
    CemQlSchemaDeclarationHost, CemQlSchemaReferenceNode, SchemaScopePreparation,
    SchemaScopePreparationIssue,
};
use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    parser::{tree::RetainedCemTree, AstNodeId, CemAstNode},
    schema::{
        declaration_references::SchemaDeclarationNode,
        document_model::SchemaDocumentModel,
        input_references::{
            validate_structural_input_regions_references, InputSchemaRegion,
            StructuralInputValidation,
        },
        reference_traversal::ReferenceTraversalLimits,
        scope_references::SchemaScopeTargetError,
    },
    value::reference_resolution::ReferenceResolutionError,
};
use std::{collections::HashSet, sync::Arc};

/// One caller-selected host and a preparation snapshot. Callers repeat
/// preparation when lifecycle inputs change before validating a new snapshot.
#[derive(Debug, Clone)]
pub struct SchemaInputRegion<'a> {
    pub host: SchemaDeclarationNode,
    pub preparation: &'a SchemaScopePreparation,
}

impl CemQlSchemaDeclarationHost {
    /// Keep host contracts on the enclosing model and validate descendants only
    /// with a ready prepared model. Incomplete overrides block their body, retain
    /// inspection diagnostics and never fall back to inherited validation.
    pub fn validate_input_regions(
        &mut self,
        source: Arc<RetainedCemTree>,
        roots: &[AstNodeId],
        model: &SchemaDocumentModel,
        regions: &[SchemaInputRegion<'_>],
        limits: ReferenceTraversalLimits,
    ) -> Result<StructuralInputValidation<CemQlSchemaReferenceNode>, ReferenceResolutionError> {
        let boundaries: Vec<_> = regions
            .iter()
            .map(|region| InputSchemaRegion {
                host: region.host.clone(),
                model: region
                    .preparation
                    .is_ready()
                    .then(|| region.preparation.model.as_ref().unwrap()),
            })
            .collect();
        let mut report = validate_structural_input_regions_references(
            source.ast_owner().clone(),
            roots,
            model,
            &boundaries,
            self,
            limits,
        )?;
        let entered: HashSet<_> = report
            .nodes
            .iter()
            .map(|node| {
                (
                    Arc::as_ptr(node.source.document()) as usize,
                    node.source.node_id(),
                )
            })
            .collect();
        for region in regions {
            let prepared = region.preparation;
            if prepared.is_ready()
                || !entered.contains(&(
                    Arc::as_ptr(region.host.document()) as usize,
                    region.host.node_id(),
                ))
            {
                continue;
            }
            append_preparation_diagnostics(&mut report, &region.host, prepared);
        }
        report.failed |= report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity.is_hard_violation());
        Ok(report)
    }
}

pub(super) fn append_preparation_diagnostics(
    report: &mut StructuralInputValidation<CemQlSchemaReferenceNode>,
    host: &SchemaDeclarationNode,
    prepared: &SchemaScopePreparation,
) {
    report.failed |= prepared.selection.failed;
    report
        .diagnostics
        .extend(prepared.selection.diagnostics.iter().cloned());
    if let Some(model) = &prepared.model {
        report
            .diagnostics
            .extend(model.compile_diagnostics.iter().cloned());
    }
    let message = match prepared.issue {
        Some(SchemaScopePreparationIssue::TargetCount(count)) => Some(format!(
            "Child schema override requires one target; selected {count}"
        )),
        Some(SchemaScopePreparationIssue::TargetHasNoSourceHandle) => {
            Some("Child schema override requires an original declaration handle".into())
        }
        Some(SchemaScopePreparationIssue::TargetAdmission(
            SchemaScopeTargetError::InvalidKindOrName,
        )) => Some("Child schema override selected an invalid schema target".into()),
        Some(SchemaScopePreparationIssue::TargetAdmission(
            SchemaScopeTargetError::DeclarationCount(count),
        )) => Some(format!(
            "Child schema wrapper requires one direct declaration; found {count}"
        )),
        _ => None, // Context and name readiness are pending, not violations.
    };
    if let Some(message) = message {
        let CemAstNode::Element { source, .. } = host.node() else {
            unreachable!("region host descriptors were validated before traversal")
        };
        report.diagnostics.push(Diagnostic {
            code: "cem.schema_scope.invalid_override".into(),
            severity: Severity::Error,
            message,
            node: Some(host.identity()),
            source_map: Some(source.clone()),
            ..Default::default()
        });
    }
    report.failed |= report
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation());
}
