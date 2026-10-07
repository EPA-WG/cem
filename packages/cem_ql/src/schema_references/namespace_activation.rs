//! Explicit activation of prepared namespace values over an original selected forest.
use super::{
    CemQlSchemaDeclarationHost, DeclarationScope, NamespaceLexicalScopeHandoffError,
    NamespacePropertyPreparation,
};
use crate::api::StandaloneExpressionContext;
use cem_ml::{
    parser::AstNodeId,
    schema::{
        declaration_references::SchemaDeclarationNode,
        machine::LexicallyScopedDocument,
        namespace_references::{
            decode_native_namespace_property, NamespaceLexicalSnapshot, NamespaceNameCompletion,
            NamespaceNameCompletionError,
        },
        reference_policy::ReferenceScopePolicyOverrides,
    },
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespacePropertyActivationError {
    OwnerMismatch,
    DuplicateDeclaration(AstNodeId),
    PropertyMismatch(AstNodeId),
    PropertyNotReady(AstNodeId),
    Namespace(NamespaceNameCompletionError),
    Handoff(NamespaceLexicalScopeHandoffError),
}

/// This execution's completed names and installed original occurrence scopes.
/// The completion can also supply shared native query ingress with the original
/// retained source. Source captures and other executions remain unchanged.
#[derive(Debug)]
pub struct NamespacePropertyActivation {
    pub completion: Arc<NamespaceNameCompletion>,
    pub scopes: Vec<(AstNodeId, DeclarationScope)>,
}

impl CemQlSchemaDeclarationHost {
    /// Activate caller-selected ready property results at an explicit lifecycle
    /// stage. Every supplied report must be ready and identify one distinct
    /// original property in this capture. Missing selected name/lexical binding
    /// dependencies and existing occurrence assignments reject before callbacks
    /// or mutation. The caller supplies contexts and local policy overrides; None
    /// remains pending. The callback can bind a native view of this completion.
    ///
    /// Results keep the bounded consumer's admitted target and original property
    /// destination identity. This does not reevaluate selectors, restart budgets,
    /// activate unrelated forests, grant crossings or change source bindings.
    /// A selector handed off earlier can be excluded from the selected roots;
    /// its original declaration remains available as a binding dependency.
    pub fn activate_namespace_properties<F>(
        &mut self,
        captured: Arc<LexicallyScopedDocument>,
        roots: &[AstNodeId],
        properties: &[NamespacePropertyPreparation],
        mut prepare: F,
    ) -> Result<NamespacePropertyActivation, NamespacePropertyActivationError>
    where
        F: FnMut(
            &SchemaDeclarationNode,
            &NamespaceLexicalSnapshot,
            DeclarationScope,
            &Arc<NamespaceNameCompletion>,
        ) -> (
            Option<StandaloneExpressionContext>,
            ReferenceScopePolicyOverrides,
        ),
    {
        let mut targets = BTreeMap::new();
        for result in properties {
            if !Arc::ptr_eq(result.declaration.document(), captured.document()) {
                return Err(NamespacePropertyActivationError::OwnerMismatch);
            }
            let id = result.declaration.node_id();
            if targets.contains_key(&id) {
                return Err(NamespacePropertyActivationError::DuplicateDeclaration(id));
            }
            if !result.is_ready() {
                return Err(NamespacePropertyActivationError::PropertyNotReady(id));
            }
            let original = decode_native_namespace_property(result.declaration.clone(), &captured)
                .map_err(|_| NamespacePropertyActivationError::PropertyMismatch(id))?;
            let property = result.property.as_ref().expect("ready property report");
            if original.prefix != property.prefix
                || original.declaration.node_id() != property.declaration.node_id()
                || !Arc::ptr_eq(
                    original.declaration.document(),
                    property.declaration.document(),
                )
                || original.value.node_id() != property.value.node_id()
                || !Arc::ptr_eq(original.value.document(), property.value.document())
            {
                return Err(NamespacePropertyActivationError::PropertyMismatch(id));
            }
            let target = result
                .preparation
                .as_ref()
                .and_then(|preparation| preparation.target.as_ref())
                .expect("ready admitted namespace target");
            targets.insert(id, target.clone());
        }
        let completion = Arc::new(
            NamespaceNameCompletion::new(captured, roots, targets)
                .map_err(NamespacePropertyActivationError::Namespace)?,
        );
        let scopes = self
            .attach_completed_namespace_lexical_scopes(&completion, |source, snapshot, parent| {
                prepare(source, snapshot, parent, &completion)
            })
            .map_err(NamespacePropertyActivationError::Handoff)?;
        Ok(NamespacePropertyActivation { completion, scopes })
    }
}
