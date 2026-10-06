//! Shared host-control contracts connected to explicit retained regions.
use super::{
    region_validation::append_preparation_diagnostics, CemQlSchemaDeclarationHost,
    CemQlSchemaReferenceNode, SchemaScopePreparation,
};
use cem_ml::{
    parser::{tree::RetainedCemTree, AstNodeId},
    schema::{
        declaration_references::SchemaDeclarationNode,
        document_model::SchemaDocumentModel,
        input_references::{
            validate_structural_input_controlled_regions_references,
            validate_structural_input_discovering_regions_references, DiscoveredInputSchemaRegion,
            InputSchemaRegion, StructuralInputValidation,
        },
        reference_traversal::ReferenceTraversalLimits,
        scope_controls::{validate_schema_host_controls, SchemaHostControlContract},
    },
    value::reference_resolution::ReferenceResolutionError,
};
use std::{collections::HashSet, sync::Arc};

/// One lifecycle snapshot retains shared control validation even if decoding or
/// selection is incomplete/invalid. Repeat preparation when inputs change.
#[derive(Debug, Clone)]
pub struct SchemaHostRegionPreparation {
    pub contract: SchemaHostControlContract,
    pub preparation: Option<SchemaScopePreparation>,
}
impl SchemaHostRegionPreparation {
    pub fn is_ready(&self) -> bool {
        self.contract.issue().is_none()
            && self.contract.control().is_some()
            && self
                .preparation
                .as_ref()
                .is_some_and(SchemaScopePreparation::is_ready)
    }
}

impl CemQlSchemaDeclarationHost {
    /// Validate the shared control shape before evaluating any selector. Invalid
    /// controls retain their source and diagnostics; missing names remain pending.
    /// URI loading, policy installation and runtime context assignment are separate
    /// stages. A host with no control inherits and introduces no child boundary.
    pub fn prepare_schema_host_region(
        &mut self,
        schema_uri: &str,
        host: SchemaDeclarationNode,
        limits: ReferenceTraversalLimits,
    ) -> Result<SchemaHostRegionPreparation, ReferenceResolutionError> {
        let contract = validate_schema_host_controls(host, |source| {
            self.captured_expanded_name(source).cloned()
        });
        let preparation = contract
            .control()
            .map(|control| self.prepare_decoded_host_control(schema_uri, control, limits))
            .transpose()?
            .flatten();
        Ok(SchemaHostRegionPreparation {
            contract,
            preparation,
        })
    }

    /// Original host attributes are split by validated control metadata, never a
    /// copied AST. Ordinary host contracts and direct child checks use the enclosing
    /// model; descendants use only a ready selected model. Failed explicit controls
    /// or unavailable selections/loaders block the body without inherited fallback.
    /// Prepared hosts may live inside selected subtrees in other original arenas.
    pub fn validate_input_host_regions(
        &mut self,
        source: Arc<RetainedCemTree>,
        roots: &[AstNodeId],
        model: &SchemaDocumentModel,
        prepared: &[SchemaHostRegionPreparation],
        limits: ReferenceTraversalLimits,
    ) -> Result<StructuralInputValidation<CemQlSchemaReferenceNode>, ReferenceResolutionError> {
        let regions: Vec<_> = prepared
            .iter()
            .filter(|region| region.contract.has_override())
            .map(|region| InputSchemaRegion {
                host: region.contract.host().clone(),
                model: region.is_ready().then(|| {
                    region
                        .preparation
                        .as_ref()
                        .unwrap()
                        .model
                        .as_deref()
                        .unwrap()
                }),
            })
            .collect();
        let controls: Vec<_> = prepared
            .iter()
            .map(|region| region.contract.clone())
            .collect();
        let mut report = validate_structural_input_controlled_regions_references(
            source.ast_owner().clone(),
            roots,
            model,
            &regions,
            &controls,
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
        for region in prepared {
            if region.is_ready()
                || !entered.contains(&(
                    Arc::as_ptr(region.contract.host().document()) as usize,
                    region.contract.host().node_id(),
                ))
            {
                continue;
            }
            if let Some(preparation) = &region.preparation {
                append_preparation_diagnostics(&mut report, region.contract.host(), preparation);
            }
        }
        Ok(report)
    }
}

/// One invocation's structural outcome and encountered override preparations.
/// Snapshots retain current inputs for inspection; a later call prepares anew.
#[derive(Debug, Clone)]
pub struct SchemaHostRegionValidation {
    pub validation: StructuralInputValidation<CemQlSchemaReferenceNode>,
    pub preparations: Vec<SchemaHostRegionPreparation>,
}

impl CemQlSchemaDeclarationHost {
    /// Discover and prepare host attributes on structural entry using stable
    /// explicitly registered contexts, names and grants. URI loading, effective
    /// policy/context installation and behavior execution remain separate stages.
    /// Selector preparation is its own bounded consumer request; structural
    /// traversal accounting and active references continue without resetting.
    pub fn validate_input_discovering_host_regions(
        &mut self,
        schema_uri: &str,
        source: Arc<RetainedCemTree>,
        roots: &[AstNodeId],
        model: &SchemaDocumentModel,
        limits: ReferenceTraversalLimits,
    ) -> Result<SchemaHostRegionValidation, ReferenceResolutionError> {
        let mut preparations = vec![];
        let mut validation = validate_structural_input_discovering_regions_references(
            source.ast_owner().clone(),
            roots,
            model,
            self,
            limits,
            |host, source, limits| {
                let prepared = host.prepare_schema_host_region(schema_uri, source, limits)?;
                if !prepared.contract.has_override() {
                    return Ok(None);
                }
                let ready = prepared.is_ready();
                let region = DiscoveredInputSchemaRegion {
                    contract: prepared.contract.clone(),
                    model: ready.then(|| {
                        prepared
                            .preparation
                            .as_ref()
                            .unwrap()
                            .model
                            .as_ref()
                            .unwrap()
                            .clone()
                    }),
                    diagnostics: vec![],
                };
                preparations.push(prepared);
                Ok(Some(region))
            },
        )?;
        for prepared in &preparations {
            if !prepared.is_ready() {
                if let Some(preparation) = &prepared.preparation {
                    append_preparation_diagnostics(
                        &mut validation,
                        prepared.contract.host(),
                        preparation,
                    );
                }
            }
        }
        Ok(SchemaHostRegionValidation {
            validation,
            preparations,
        })
    }
}
