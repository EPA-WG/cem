//! Explicit publication of ready namespace values within one input snapshot.
use super::{CemQlSchemaDeclarationHost, NamespacePropertyPreparation};
use cem_ml::schema::{
    declaration_references::SchemaDeclarationNode,
    namespace_references::{decode_native_namespace_property, NamespaceScopeTarget},
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

pub(super) fn next_snapshot() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
#[derive(Debug, Clone)]
pub(super) struct NamespacePublicationProof {
    pub snapshot: u64,
    pub dependencies: Vec<(SchemaDeclarationNode, Option<super::DeclarationScope>)>,
    pub declaration: SchemaDeclarationNode,
    pub target: NamespaceScopeTarget,
    pub selected: SchemaDeclarationNode,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespacePublicationError {
    NotReady,
    DifferentSnapshot,
    ReportMismatch,
    ConflictingResult,
}
fn same_node(left: &SchemaDeclarationNode, right: &SchemaDeclarationNode) -> bool {
    left.node_id() == right.node_id() && Arc::ptr_eq(left.document(), right.document())
}
impl NamespacePublicationProof {
    pub(super) fn matches(&self, host: &CemQlSchemaDeclarationHost) -> bool {
        host.check_operation().is_ok()
            && self.snapshot == host.namespace_input_snapshot
            && self
                .dependencies
                .iter()
                .all(|(source, scope)| host.source_scope(source) == *scope)
    }
}
impl CemQlSchemaDeclarationHost {
    pub(super) fn invalidate_namespace_publications(&mut self) {
        self.namespace_input_snapshot = next_snapshot();
        self.namespace_publications.clear();
    }
    /// Publish a ready result produced by this input snapshot. This does not
    /// evaluate slots, complete names, activate scopes or grant crossings.
    /// Context changes expire all results; a changed scope assignment on any
    /// evaluated dependency makes that result/proof unavailable. A matching repeat
    /// is idempotent; another execution or mismatched/stale report cannot publish.
    /// Each later target selection still runs its own bounded authorized walker.
    pub fn publish_namespace_property(
        &mut self,
        report: &NamespacePropertyPreparation,
    ) -> Result<(), NamespacePublicationError> {
        let proof = self.validate_namespace_property_report(report)?;
        let key = (
            Arc::as_ptr(report.declaration.document()) as usize,
            report.declaration.node_id(),
        );
        if let Some(existing) = self
            .namespace_publications
            .get(&key)
            .filter(|proof| proof.matches(self))
        {
            if !same_node(
                existing.target.binding_declaration(),
                proof.target.binding_declaration(),
            ) || existing.target.namespace_uri() != proof.target.namespace_uri()
            {
                return Err(NamespacePublicationError::ConflictingResult);
            }
        } else {
            self.namespace_publications.insert(key, proof.clone());
        }
        Ok(())
    }

    pub(super) fn validate_namespace_property_report<'a>(
        &self,
        report: &'a NamespacePropertyPreparation,
    ) -> Result<&'a NamespacePublicationProof, NamespacePublicationError> {
        if !report.is_ready() {
            return Err(NamespacePublicationError::NotReady);
        }
        let proof = report
            .publication
            .as_ref()
            .ok_or(NamespacePublicationError::NotReady)?;
        if !proof.matches(self) {
            return Err(NamespacePublicationError::DifferentSnapshot);
        }
        let property = report.property.as_ref().unwrap();
        let key = (
            Arc::as_ptr(report.declaration.document()) as usize,
            report.declaration.node_id(),
        );
        let captured = self
            .captured_namespaces
            .get(&key.0)
            .ok_or(NamespacePublicationError::ReportMismatch)?;
        let original = decode_native_namespace_property(report.declaration.clone(), captured)
            .map_err(|_| NamespacePublicationError::ReportMismatch)?;
        let target = report
            .preparation
            .as_ref()
            .unwrap()
            .target
            .as_ref()
            .unwrap();
        if !same_node(&report.declaration, &proof.declaration)
            || !same_node(&property.declaration, &proof.declaration)
            || !same_node(&property.value, &original.value)
            || property.prefix != original.prefix
            || !same_node(&target.selected, &proof.selected)
            || !same_node(
                target.binding_declaration(),
                proof.target.binding_declaration(),
            )
            || target.namespace_uri() != proof.target.namespace_uri()
        {
            return Err(NamespacePublicationError::ReportMismatch);
        }
        Ok(proof)
    }
}
