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
        if !is_host_control_name(&name) {
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

/// Validated control metadata over one original host. Construction stays in this
/// module so consumers cannot exempt arbitrary application attributes by ID.
#[derive(Debug, Clone)]
pub struct SchemaHostControlContract {
    host: SchemaDeclarationNode,
    attributes: Vec<crate::parser::AstNodeId>,
    pending_attributes: Vec<crate::parser::AstNodeId>,
    control: Option<SchemaHostControl>,
    issue: Option<SchemaHostControlError>,
}
impl SchemaHostControlContract {
    pub fn host(&self) -> &SchemaDeclarationNode {
        &self.host
    }
    pub fn attributes(&self) -> &[crate::parser::AstNodeId] {
        &self.attributes
    }
    pub(crate) fn pending_attributes(&self) -> &[crate::parser::AstNodeId] {
        &self.pending_attributes
    }
    pub fn control(&self) -> Option<&SchemaHostControl> {
        self.control.as_ref()
    }
    pub fn issue(&self) -> Option<&SchemaHostControlError> {
        self.issue.as_ref()
    }
    pub fn has_override(&self) -> bool {
        self.control.is_some() || self.issue.is_some()
    }
    pub fn diagnostics(&self) -> Vec<crate::diagnostics::Diagnostic> {
        use crate::diagnostics::{Diagnostic, Severity};
        let Some(issue) = &self.issue else {
            return vec![];
        };
        let message = match issue.issue {
            SchemaHostControlIssue::NamesNotReady => return vec![],
            SchemaHostControlIssue::InvalidHost => "Schema controls require an original element host",
            SchemaHostControlIssue::ConflictingSources => "Host schema-src and schema-select controls are mutually exclusive and cannot repeat",
            SchemaHostControlIssue::InvalidValue => "Host schema-src requires a nonempty URI literal; schema-select requires a nonempty selector or one native expression slot",
        };
        vec![Diagnostic {
            code: "cem.schema_scope.invalid_control".into(),
            severity: Severity::Error,
            message: message.into(),
            node: Some(issue.source.identity()),
            source_map: Some(
                super::document_model::source_stack_for_node(issue.source.node()).clone(),
            ),
            ..Default::default()
        }]
    }
}

/// Shared control validation retains recognized attributes even on malformed
/// controls. Missing names are incomplete metadata, not guessed exemptions.
/// Resolve each attribute name once; no source mutation or selector evaluation.
pub fn validate_schema_host_controls<F>(
    host: SchemaDeclarationNode,
    mut resolved_name: F,
) -> SchemaHostControlContract
where
    F: FnMut(&SchemaDeclarationNode) -> Option<ExpandedName>,
{
    let mut names = std::collections::HashMap::new();
    let mut attributes = vec![];
    let mut pending_attributes = vec![];
    let mut missing = None;
    if let CemAstNode::Element {
        attributes: source_attributes,
        ..
    } = host.node()
    {
        for id in source_attributes {
            let Some(source) = SchemaDeclarationNode::new(host.document().clone(), *id) else {
                continue;
            };
            if let Some(name) = resolved_name(&source) {
                if is_host_control_name(&name) {
                    attributes.push(*id);
                }
                names.insert(*id, name);
            } else {
                pending_attributes.push(*id);
                if missing.is_none() {
                    missing = Some(SchemaHostControlError {
                        source,
                        issue: SchemaHostControlIssue::NamesNotReady,
                    });
                }
            }
        }
    }
    let decoded = match missing {
        Some(issue) => Err(issue),
        None => {
            decode_schema_host_control(host.clone(), |source| names.get(&source.node_id()).cloned())
        }
    };
    let (control, issue) = match decoded {
        Ok(control) => (control, None),
        Err(issue) => (None, Some(issue)),
    };
    SchemaHostControlContract {
        host,
        attributes,
        pending_attributes,
        control,
        issue,
    }
}
fn is_host_control_name(name: &ExpandedName) -> bool {
    (name.namespace_uri.is_empty() || name.namespace_uri == CORE_NAMESPACE)
        && matches!(name.local_name.as_str(), "schema-src" | "schema-select")
}
