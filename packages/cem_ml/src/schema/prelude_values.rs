//! Owner-checked views of original typed directive slots. No evaluation, source
//! rewriting, synthetic attributes, or implicit namespace/schema activation.
use super::{
    declaration_references::SchemaDeclarationNode,
    input_references::native_attribute_expression,
    machine::{LexicallyScopedDocument, SchemaElementForm},
    scope_controls::SchemaScopeControlExtent,
};
use crate::{
    parser::CemAstNode,
    tokenizer::cem::{TypedPreludeKind, TypedPreludeRole},
};
use std::sync::Arc;

/// Validate required metadata independently of lexical/runtime capture. Binary
/// decoding and canonical output cannot reinterpret a missing slot as a literal.
pub fn validate_document_slots(
    document: &crate::parser::document::CemDocument,
) -> Result<(), String> {
    for (&id, syntax) in &document.typed_preludes {
        validate_source_slot(document, id, syntax)?;
    }
    for node in &document.nodes {
        if let CemAstNode::Element {
            node_id,
            expanded_name,
            children,
            ..
        } = node
        {
            if expanded_name.local_name.starts_with('@')
                && children.iter().any(|id| matches!(document.get(*id), Some(CemAstNode::Reference { .. }))
                    || matches!(document.get(*id), Some(CemAstNode::Element { expanded_name, .. }) if expanded_name.local_name == "$"))
                && !document.typed_preludes.contains_key(node_id)
            { return Err("typed directive requires original slot metadata".into()); }
        }
    }
    Ok(())
}

pub(crate) fn validate_source_slot(
    document: &crate::parser::document::CemDocument,
    id: crate::parser::AstNodeId,
    syntax: &crate::tokenizer::cem::TypedPreludeValue,
) -> Result<crate::parser::AstNodeId, String> {
    let fail = || "invalid typed directive source metadata".to_owned();
    let CemAstNode::Element {
        expanded_name,
        attributes,
        children,
        ..
    } = document.get(id).ok_or_else(fail)?
    else {
        return Err(fail());
    };
    let [value] = children.as_slice() else {
        return Err(fail());
    };
    let name = match syntax.role {
        TypedPreludeRole::SchemaSelector if syntax.prefix.is_none() => "@schema",
        TypedPreludeRole::Namespace
            if syntax.prefix.as_ref().is_some_and(|p| {
                p.chars()
                    .next()
                    .is_some_and(|c| c.is_alphabetic() || c == '_')
                    && p.chars()
                        .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-'))
            }) =>
        {
            "@ns"
        }
        TypedPreludeRole::DefaultNamespace if syntax.prefix.as_deref() == Some("") => "@default",
        _ => return Err(fail()),
    };
    if syntax.error.is_some()
        || !attributes.is_empty()
        || expanded_name.local_name != name
        || syntax.value_range.start > syntax.payload_range.start
        || syntax.payload_range.end() > syntax.value_range.end()
        || syntax.role_range.end() > syntax.value_range.start
        || syntax
            .prefix_range
            .is_some_and(|range| range.end() > syntax.value_range.start)
        || (syntax.role == TypedPreludeRole::Namespace) != syntax.prefix_range.is_some()
        || (syntax.kind == TypedPreludeKind::Reference && !syntax.expression.starts_with('#'))
        || syntax.expression.is_empty()
        || syntax.expression.contains(['\r', '\n'])
    {
        return Err(fail());
    }
    match (syntax.kind, document.get(*value)) {
        (
            TypedPreludeKind::Reference,
            Some(CemAstNode::Reference {
                context,
                expression,
                ..
            }),
        ) if *context == id && expression == &syntax.expression => {}
        (
            TypedPreludeKind::Expression,
            Some(CemAstNode::Element {
                expanded_name,
                attributes,
                children,
                ..
            }),
        ) if expanded_name.local_name == "$" && attributes.is_empty() => {
            let mut text = String::new();
            for child in children {
                let Some(CemAstNode::Text { data, .. }) = document.get(*child) else {
                    return Err(fail());
                };
                text.push_str(data);
            }
            if text != syntax.expression {
                return Err(fail());
            }
        }
        _ => return Err(fail()),
    }
    Ok(*value)
}

/// Canonical directive payload only; the role, prefix and constructor stay
/// distinct from quoted literal text. No target graph or query is evaluated.
pub fn canonical_prelude_body(
    syntax: &crate::tokenizer::cem::TypedPreludeValue,
) -> Result<String, String> {
    if syntax.error.is_some() || syntax.expression.contains(['\n', '\r']) {
        return Err("typed prelude output requires a valid one-line value".into());
    }
    let header = match syntax.role {
        TypedPreludeRole::SchemaSelector => "select=".to_owned(),
        TypedPreludeRole::Namespace => format!(
            "{} = ",
            syntax.prefix.as_deref().ok_or("missing namespace prefix")?
        ),
        TypedPreludeRole::DefaultNamespace => String::new(),
    };
    let marker = if syntax.kind == TypedPreludeKind::Expression {
        "$ "
    } else {
        ""
    };
    Ok(format!("{header}{{{marker}{}}}", syntax.expression))
}

#[derive(Debug, Clone)]
pub struct NativePreludeValue {
    declaration: SchemaDeclarationNode,
    value: SchemaDeclarationNode,
    role: TypedPreludeRole,
    kind: TypedPreludeKind,
    prefix: Option<String>,
}
impl NativePreludeValue {
    pub fn declaration(&self) -> &SchemaDeclarationNode {
        &self.declaration
    }
    pub fn value(&self) -> &SchemaDeclarationNode {
        &self.value
    }
    pub fn role(&self) -> TypedPreludeRole {
        self.role
    }
    pub fn kind(&self) -> TypedPreludeKind {
        self.kind
    }
    pub fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePreludeValueError {
    OwnerMismatch,
    MetadataNotReady,
    InvalidValue,
}

/// Admission uses the parser's finite role and original owning edge, never a
/// raw name or expression-looking text alone. Quoted directives have no view.
pub fn decode_native_prelude_value(
    declaration: SchemaDeclarationNode,
    captured: &LexicallyScopedDocument,
) -> Result<NativePreludeValue, NativePreludeValueError> {
    use NativePreludeValueError::*;
    if !Arc::ptr_eq(declaration.document(), captured.document()) {
        return Err(OwnerMismatch);
    }
    let slot = captured
        .typed_prelude(declaration.document(), declaration.node_id())
        .ok_or(MetadataNotReady)?;
    if slot.syntax.error.is_some()
        || slot.form != SchemaElementForm::Prelude
        || slot.extent != SchemaScopeControlExtent::Following
        || slot.required_version != super::ir::SemVer::new(1, 1, 0)
        || (slot.syntax.role == TypedPreludeRole::SchemaSelector
            && captured.schema_element_form(declaration.document(), slot.directive)
                != Some(slot.form))
    {
        return Err(InvalidValue);
    }
    let CemAstNode::Element {
        expanded_name,
        attributes,
        children,
        ..
    } = declaration.node()
    else {
        return Err(InvalidValue);
    };
    let (name, valid_prefix) = match slot.syntax.role {
        TypedPreludeRole::SchemaSelector => ("@schema", slot.syntax.prefix.is_none()),
        TypedPreludeRole::Namespace => (
            "@ns",
            slot.syntax.prefix.as_ref().is_some_and(|p| !p.is_empty()),
        ),
        TypedPreludeRole::DefaultNamespace => {
            ("@default", slot.syntax.prefix.as_deref() == Some(""))
        }
    };
    if expanded_name.local_name != name
        || !valid_prefix
        || !attributes.is_empty()
        || children.as_slice() != [slot.value]
        || captured
            .snapshot(declaration.document(), slot.value)
            .is_none()
    {
        return Err(InvalidValue);
    }
    let value = SchemaDeclarationNode::new(declaration.document().clone(), slot.value)
        .ok_or(InvalidValue)?;
    let valid = match (slot.syntax.kind, value.node()) {
        (
            TypedPreludeKind::Reference,
            CemAstNode::Reference {
                context,
                expression,
                ..
            },
        ) => *context == slot.directive && expression == &slot.syntax.expression,
        (TypedPreludeKind::Expression, CemAstNode::Element { .. }) => {
            native_attribute_expression(&value).is_some_and(|occurrence| {
                occurrence.expression.as_deref() == Some(slot.syntax.expression.as_str())
            })
        }
        _ => false,
    };
    if !valid {
        return Err(InvalidValue);
    }
    Ok(NativePreludeValue {
        declaration,
        value,
        role: slot.syntax.role,
        kind: slot.syntax.kind,
        prefix: slot.syntax.prefix.clone(),
    })
}
