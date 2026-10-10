//! Decode established host controls without evaluating or assigning a scope.
use super::{
    declaration_references::SchemaDeclarationNode, input_references::native_attribute_expression,
};
mod prelude;

use crate::parser::{CemAstNode, ExpandedName};

const CORE_NAMESPACE: &str = "https://cem.dev/ns/core/1";

#[derive(Debug, Clone)]
pub enum SchemaHostSource {
    Uri(String),
    LiteralSelector(String),
    NativeSelector(SchemaDeclarationNode),
}

/// Selection metadata only. A following switch retains its enclosing model and
/// context; activation applies after the original control in its parent scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SchemaScopeControlExtent {
    Body,
    Following,
}

/// The host retains its enclosing contracts; extent identifies the governed region.
/// Names and values belong to the original arena, with no scope installation.
#[derive(Debug, Clone)]
pub struct SchemaHostControl {
    pub host: SchemaDeclarationNode,
    /// Original source site: an attribute, literal prelude text, or typed directive.
    pub attribute: SchemaDeclarationNode,
    pub source: SchemaHostSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaHostControlIssue {
    NamesNotReady,
    BodyFormNotReady,
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
    decode_body_control(host, &mut resolved_name, false)
}

fn decode_body_control<F>(
    host: SchemaDeclarationNode,
    resolved_name: &mut F,
    wrapping: bool,
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
        if !is_body_control_name(&name, wrapping) {
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
            ("schema-src" | "src", Some(value), []) if !value.trim().is_empty() => {
                SchemaHostSource::Uri(value.clone())
            }
            ("schema-select" | "select", Some(value), []) if !value.trim().is_empty() => {
                SchemaHostSource::LiteralSelector(value.clone())
            }
            ("schema-select" | "select", None, [id]) => {
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
    extent: SchemaScopeControlExtent,
}
impl SchemaHostControlContract {
    pub fn extent(&self) -> SchemaScopeControlExtent {
        self.extent
    }
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
            SchemaHostControlIssue::NamesNotReady | SchemaHostControlIssue::BodyFormNotReady => return vec![],
            SchemaHostControlIssue::InvalidHost => "Schema controls require an original element host",
            SchemaHostControlIssue::ConflictingSources => "Schema body source controls are mutually exclusive and cannot repeat",
            SchemaHostControlIssue::InvalidValue => "Schema body src requires a nonempty URI literal; select requires a nonempty selector or one native expression slot",
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
    validate_body_controls(host, &mut resolved_name, false, false, None)
}

/// Established host attributes and wrapping schema elements use one body
/// contract. Named declarations without a source do not switch. No-body schema
/// elements require following-region scheduling and are not body overrides.
/// Source-form metadata must come from the original parser/importer, including
/// empty explicit bodies; missing metadata is pending, never a guessed form.
pub fn validate_schema_body_controls<F>(
    host: SchemaDeclarationNode,
    mut resolved_name: F,
    form: Option<super::machine::SchemaElementForm>,
) -> SchemaHostControlContract
where
    F: FnMut(&SchemaDeclarationNode) -> Option<ExpandedName>,
{
    validate_body_controls(host, &mut resolved_name, true, false, form)
}

/// Decode established body, no-body sibling and literal prelude controls with
/// source form. No evaluation, scope installation or readiness is implied.
pub fn validate_schema_scope_controls<F>(
    host: SchemaDeclarationNode,
    mut resolved_name: F,
    form: Option<super::machine::SchemaElementForm>,
) -> SchemaHostControlContract
where
    F: FnMut(&SchemaDeclarationNode) -> Option<ExpandedName>,
{
    validate_body_controls(host, &mut resolved_name, true, true, form)
}

/// Decode an original typed schema directive using its owner-checked capture.
/// Missing or invalid metadata leaves an unavailable following override; it
/// never falls back to literal decoding or an inherited schema.
pub fn validate_typed_schema_prelude(
    host: SchemaDeclarationNode,
    captured: &super::machine::LexicallyScopedDocument,
) -> SchemaHostControlContract {
    prelude::validate_typed(host, captured)
}

fn validate_body_controls<F>(
    host: SchemaDeclarationNode,
    resolved_name: &mut F,
    include_wrappers: bool,
    include_following: bool,
    form: Option<super::machine::SchemaElementForm>,
) -> SchemaHostControlContract
where
    F: FnMut(&SchemaDeclarationNode) -> Option<ExpandedName>,
{
    if include_following
        && matches!(host.node(), CemAstNode::Element { expanded_name, .. } if expanded_name.local_name == "@schema")
    {
        return prelude::validate(host, form);
    }
    let mut names = std::collections::HashMap::new();
    let mut attributes = vec![];
    let mut pending_attributes = vec![];
    let mut missing = None;
    let element_name = include_wrappers.then(|| resolved_name(&host)).flatten();
    let wrapper = include_wrappers
        && element_name.as_ref().is_some_and(|name| {
            name.local_name == "schema"
                && (name.namespace_uri.is_empty() || name.namespace_uri == CORE_NAMESPACE)
        });
    let following =
        wrapper && include_following && form == Some(super::machine::SchemaElementForm::Following);
    let wrapping =
        wrapper && (form == Some(super::machine::SchemaElementForm::Wrapping) || following);
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
                if is_body_control_name(&name, wrapping) {
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
    if include_wrappers && element_name.is_none() && names.values().any(is_wrapper_source_name) {
        // A missing element name cannot distinguish a wrapper from ordinary
        // data. Its source-like attributes remain unclassified until capture.
        missing = Some(SchemaHostControlError {
            source: host.clone(),
            issue: SchemaHostControlIssue::NamesNotReady,
        });
    }
    if wrapper && form.is_none() && names.values().any(is_wrapper_source_name) {
        missing = Some(SchemaHostControlError {
            source: host.clone(),
            issue: SchemaHostControlIssue::BodyFormNotReady,
        });
    }
    let decoded = match missing {
        Some(issue) => Err(issue),
        None => decode_body_control(
            host.clone(),
            &mut |source: &SchemaDeclarationNode| names.get(&source.node_id()).cloned(),
            wrapping,
        ),
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
        extent: if following {
            SchemaScopeControlExtent::Following
        } else {
            SchemaScopeControlExtent::Body
        },
    }
}
fn is_body_control_name(name: &ExpandedName, wrapping: bool) -> bool {
    is_host_control_name(name) || (wrapping && is_wrapper_source_name(name))
}
fn is_wrapper_source_name(name: &ExpandedName) -> bool {
    name.namespace_uri.is_empty() && matches!(name.local_name.as_str(), "src" | "select")
}
fn is_host_control_name(name: &ExpandedName) -> bool {
    (name.namespace_uri.is_empty() || name.namespace_uri == CORE_NAMESPACE)
        && matches!(name.local_name.as_str(), "schema-src" | "schema-select")
}
