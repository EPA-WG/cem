//! Decode established host controls without evaluating or assigning a scope.
use super::{
    declaration_references::SchemaDeclarationNode, input_references::native_attribute_expression,
};
use crate::parser::{CemAstNode, ExpandedName};

const CORE_NAMESPACE: &str = "https://cem.dev/ns/core/1";

#[derive(Debug, Clone)]
pub enum SchemaHostSource {
    Uri(String),
    LiteralSelector(String),
    NativeSelector(SchemaDeclarationNode),
}

/// The host retains its enclosing contracts; this control governs its body.
/// Names and values belong to the original arena, with no scope installation.
#[derive(Debug, Clone)]
pub struct SchemaHostControl {
    pub host: SchemaDeclarationNode,
    pub attribute: SchemaDeclarationNode,
    pub source: SchemaHostSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaHostControlIssue {
    NamesNotReady,
    InvalidHost,
    ConflictingSources,
    InvalidValue,
}
#[derive(Debug, Clone)]
pub struct SchemaHostControlError {
    pub source: SchemaDeclarationNode,
    pub issue: SchemaHostControlIssue,
}

/// Match expanded core names or canonical unqualified host attributes. A foreign
/// prefix/local-name lookalike is ordinary data. Missing name metadata is pending,
/// never permission to guess by the source prefix. Wrapper/sibling constructs and
/// the document prelude are handled by their separate lifecycle stages.
pub fn decode_schema_host_control<F>(
    host: SchemaDeclarationNode,
    mut resolved_name: F,
) -> Result<Option<SchemaHostControl>, SchemaHostControlError>
where
    F: FnMut(&SchemaDeclarationNode) -> Option<ExpandedName>,
{
    let error = |source, issue| SchemaHostControlError { source, issue };
    let CemAstNode::Element { attributes, .. } = host.node() else {
        return Err(error(host, SchemaHostControlIssue::InvalidHost));
    };
    let mut control = None;
    for id in attributes {
        let Some(attribute) = SchemaDeclarationNode::new(host.document().clone(), *id) else {
            return Err(error(host.clone(), SchemaHostControlIssue::InvalidHost));
        };
        let name = resolved_name(&attribute)
            .ok_or_else(|| error(attribute.clone(), SchemaHostControlIssue::NamesNotReady))?;
        if !(name.namespace_uri.is_empty() || name.namespace_uri == CORE_NAMESPACE)
            || !matches!(name.local_name.as_str(), "schema-src" | "schema-select")
        {
            continue;
        }
        if control.is_some() {
            return Err(error(attribute, SchemaHostControlIssue::ConflictingSources));
        }
        let CemAstNode::Attribute {
            value, value_nodes, ..
        } = attribute.node()
        else {
            return Err(error(attribute, SchemaHostControlIssue::InvalidValue));
        };
        let source = match (name.local_name.as_str(), value, value_nodes.as_slice()) {
            ("schema-src", Some(value), []) if !value.trim().is_empty() => {
                SchemaHostSource::Uri(value.clone())
            }
            ("schema-select", Some(value), []) if !value.trim().is_empty() => {
                SchemaHostSource::LiteralSelector(value.clone())
            }
            ("schema-select", None, [id]) => {
                let payload =
                    SchemaDeclarationNode::new(host.document().clone(), *id).ok_or_else(|| {
                        error(attribute.clone(), SchemaHostControlIssue::InvalidValue)
                    })?;
                if !matches!(payload.node(), CemAstNode::Reference { .. })
                    && native_attribute_expression(&payload).is_none()
                {
                    return Err(error(attribute, SchemaHostControlIssue::InvalidValue));
                }
                SchemaHostSource::NativeSelector(payload)
            }
            _ => return Err(error(attribute, SchemaHostControlIssue::InvalidValue)),
        };
        control = Some(SchemaHostControl {
            host: host.clone(),
            attribute,
            source,
        });
    }
    Ok(control)
}
