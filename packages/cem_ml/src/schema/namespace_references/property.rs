//! Decode captured native namespace properties without interpreting their values.
use super::{LexicallyScopedDocument, PendingNamespaceValue, SchemaDeclarationNode};
use crate::{parser::CemAstNode, schema::input_references::native_attribute_expression};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct NativeNamespaceProperty {
    /// Original namespace attribute or typed prelude directive, never synthetic.
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
/// looking literals are not native properties. Typed directives require their
/// original captured role and owning value edge. This creates no context or result,
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
    if matches!(declaration.node(), CemAstNode::Element { .. }) {
        use crate::{
            schema::prelude_values::decode_native_prelude_value, tokenizer::cem::TypedPreludeRole,
        };
        let slot = decode_native_prelude_value(declaration.clone(), captured)
            .map_err(|_| NativeNamespacePropertyError::InvalidValue)?;
        let prefix = match slot.role() {
            TypedPreludeRole::Namespace => slot.prefix().unwrap(),
            TypedPreludeRole::DefaultNamespace => "",
            _ => return Err(NativeNamespacePropertyError::NotNativeNamespaceDeclaration),
        };
        if prefix != pending.prefix {
            return Err(NativeNamespacePropertyError::InvalidValue);
        }
        return Ok(NativeNamespaceProperty {
            declaration,
            prefix: prefix.to_owned(),
            value: slot.value().clone(),
        });
    }
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
