//! Decode captured native namespace properties without interpreting their values.
use super::{LexicallyScopedDocument, PendingNamespaceValue, SchemaDeclarationNode};
use crate::{parser::CemAstNode, schema::input_references::native_attribute_expression};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct NativeNamespaceProperty {
    pub declaration: SchemaDeclarationNode,
    /// Destination prefix, independent of the selected declaration's prefix.
    pub prefix: String,
    pub value: SchemaDeclarationNode,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeNamespacePropertyError {
    OwnerMismatch,
    NotNativeNamespaceDeclaration,
    ValueCount(usize),
    InvalidValue,
}

/// Require the original parser-captured native declaration and its single owning
/// expression slot. General expressions keep their original `$` wrapper. Literal
/// namespace declarations/default aliases, ordinary attributes and XML expression-
/// looking literals are not native properties. This creates no context or result,
/// evaluates no text, grants no crossing and changes no source metadata.
pub fn decode_native_namespace_property(
    declaration: SchemaDeclarationNode,
    captured: &LexicallyScopedDocument,
) -> Result<NativeNamespaceProperty, NativeNamespacePropertyError> {
    if !Arc::ptr_eq(declaration.document(), captured.document()) {
        return Err(NativeNamespacePropertyError::OwnerMismatch);
    }
    let pending = captured
        .pending_namespace_declaration(declaration.document(), declaration.node_id())
        .filter(|pending| matches!(pending.value, PendingNamespaceValue::Native))
        .ok_or(NativeNamespacePropertyError::NotNativeNamespaceDeclaration)?;
    let CemAstNode::Attribute { value_nodes, .. } = declaration.node() else {
        return Err(NativeNamespacePropertyError::NotNativeNamespaceDeclaration);
    };
    let [value] = value_nodes.as_slice() else {
        return Err(NativeNamespacePropertyError::ValueCount(value_nodes.len()));
    };
    let value = SchemaDeclarationNode::new(declaration.document().clone(), *value)
        .ok_or(NativeNamespacePropertyError::InvalidValue)?;
    if captured
        .snapshot(value.document(), value.node_id())
        .is_none()
        || (!matches!(value.node(), CemAstNode::Reference { .. })
            && native_attribute_expression(&value).is_none())
    {
        return Err(NativeNamespacePropertyError::InvalidValue);
    }
    Ok(NativeNamespaceProperty {
        declaration,
        prefix: pending.prefix.clone(),
        value,
    })
}
