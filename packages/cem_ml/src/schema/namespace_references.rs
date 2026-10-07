//! Namespace target admission, independent of schema selection and activation.
use super::{
    declaration_references::SchemaDeclarationNode, machine::LexicallyScopedDocument,
    namespace::NamespaceBinding,
};
use std::sync::Arc;

mod completion;
pub use completion::{NamespaceNameCompletion, NamespaceNameCompletionError};

/// Original declaration dependency, not a stored evaluation or runtime context.
#[derive(Debug, Clone)]
pub enum PendingNamespaceValue {
    /// Evaluate the declaration attribute's original native value slots.
    Native,
    /// An existing literal default-prefix alias selected this earlier binding.
    Alias(crate::parser::AstNodeId),
}
#[derive(Debug, Clone)]
pub struct PendingNamespaceDeclaration {
    pub prefix: String,
    pub value: PendingNamespaceValue,
}
#[derive(Debug, Clone)]
pub struct PendingNamespaceName {
    pub declaration: crate::parser::AstNodeId,
    pub local_name: String,
}

/// Original source declaration plus its completed parser binding. A binding ID
/// identifies a namespace record in its lexical context, never an AST node.
/// The consuming property supplies its own destination prefix and lifecycle.
#[derive(Debug, Clone)]
pub struct NamespaceScopeTarget {
    pub selected: SchemaDeclarationNode,
    binding: NamespaceBinding,
}
impl NamespaceScopeTarget {
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
    Ok(NamespaceScopeTarget { selected, binding })
}
