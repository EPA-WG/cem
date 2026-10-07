//! Namespace target admission, independent of schema selection and activation.
use super::{
    declaration_references::SchemaDeclarationNode, machine::LexicallyScopedDocument,
    namespace::NamespaceBinding,
};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

mod completion;
mod property;
pub use completion::{
    CompletedNamespaceBinding, NamespaceLexicalSnapshot, NamespaceNameCompletion,
    NamespaceNameCompletionError,
};
pub use property::{
    decode_native_namespace_property, NativeNamespaceProperty, NativeNamespacePropertyError,
};

/// Original declaration dependency, not a stored evaluation or runtime context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PendingNamespaceValue {
    /// Evaluate the declaration attribute's original native value slots.
    Native,
    /// An existing literal default-prefix alias selected this earlier binding.
    Alias(crate::parser::AstNodeId),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingNamespaceDeclaration {
    pub prefix: String,
    pub value: PendingNamespaceValue,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingNamespaceName {
    pub declaration: crate::parser::AstNodeId,
    pub local_name: String,
}

/// Selected original declaration plus its original completed binding provider.
/// Publication can select a native property while retaining the literal provider.
/// A binding ID identifies that provider's lexical record, never an AST node.
/// The consuming property supplies its own destination prefix and lifecycle.
#[derive(Debug, Clone)]
pub struct NamespaceScopeTarget {
    pub selected: SchemaDeclarationNode,
    binding: NamespaceBinding,
    binding_declaration: SchemaDeclarationNode,
}
impl NamespaceScopeTarget {
    /// Original completed binding provider, retained even when selection reuses
    /// an execution-completed native property. No synthetic binding ID is made.
    pub fn binding_declaration(&self) -> &SchemaDeclarationNode {
        &self.binding_declaration
    }
    /// Original provider metadata; its prefix is independent of the consuming
    /// property's destination prefix and of a published property's authored name.
    pub fn binding(&self) -> &NamespaceBinding {
        &self.binding
    }
    /// An empty URI is a completed namespace reset, not missing readiness.
    pub fn namespace_uri(&self) -> &str {
        &self.binding.namespace_uri
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceScopeTargetError {
    OwnerMismatch,
    NotNamespaceDeclaration,
}

/// Admit only explicit declarations recorded by the original parser/importer.
/// CEM `@ns`/`@default` and CEM/XML namespace attributes retain their existing
/// literal semantics. Missing/uncompleted bindings and schema/data lookalikes
/// cannot provide a namespace URI. This does not evaluate a reference, enforce
/// selection cardinality, grant a crossing or install/rebind namespace context.
pub fn admit_namespace_scope_target(
    selected: SchemaDeclarationNode,
    captured: &LexicallyScopedDocument,
) -> Result<NamespaceScopeTarget, NamespaceScopeTargetError> {
    if !Arc::ptr_eq(selected.document(), captured.document()) {
        return Err(NamespaceScopeTargetError::OwnerMismatch);
    }
    let binding = captured
        .namespace_binding(selected.document(), selected.node_id())
        .ok_or(NamespaceScopeTargetError::NotNamespaceDeclaration)?
        .clone();
    Ok(NamespaceScopeTarget {
        binding_declaration: selected.clone(),
        selected,
        binding,
    })
}

/// Retain a completed property's original declaration and its original binding
/// provider separately. The caller owns bounded evaluation and snapshot readiness;
/// this creates neither a parser binding nor a runtime/source cache.
pub fn completed_namespace_scope_target(
    property: &NativeNamespaceProperty,
    target: &NamespaceScopeTarget,
) -> NamespaceScopeTarget {
    NamespaceScopeTarget {
        selected: property.declaration.clone(),
        binding: target.binding.clone(),
        binding_declaration: target.binding_declaration.clone(),
    }
}
