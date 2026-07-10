//! Tier A semantic rule catalog.
//!
//! Each rule maps a category from `docs/todo.md` §Validation:
//!
//! - `ReferenceIntegrityRule`: `id` / `for` / `aria-*` integrity.
//! - `AccessibleNameRule`: interactive elements (button, a, input,
//!   textarea, select) must have accessible name material.
//! - `AriaCompatibilityRule`: role/ARIA attribute compatibility.
//! - `SvgAccessibilityRule`: SVG-in-HTML naming and focus boundaries.
//! - `StateCombinationRule`: disallow incompatible `cem:state` combos.
//! - `StateTransitionRule`: disallow impossible static state transitions.
//! - `SchemaDocumentModelRule`: schema-package structural checks.
//! - `OpenContentPolicyRule`: schema-owned unknown-name policy checks.
//! - `JavaScriptUrlRule`: `href` / `src` / `action` / `formaction` /
//!   `xlink:href` values starting with `javascript:`.
//! - `UnsafeInlineContentRule`: inline script/srcdoc/external-DTD hooks.
//! - `EventHandlerAttributeRule`: `on*` event handler attributes.

use crate::conversion::DomProjectionParityCemtAdapter;
use crate::diagnostics::{Diagnostic, Severity};
use crate::engine::{
    FormatIdentity, TemplateInput, TransformExecutionPolicy, TransformTemplateEntrypoint,
};
use crate::parser::{AstNodeId, CemAstNode};
use crate::resolver::{has_uri_scheme, is_windows_drive_path, parse_local_file_uri};
use crate::run_config::ScopeConfig;
use crate::schema::document_model::{
    load_builtin_document_model_for_identity, validate_document_model,
};
use crate::schema::registry::{
    content_type_essence, SchemaRegistry, CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI,
    CEM_NATIVE_TEMPLATE_CONTENT_TYPE, CEM_NATIVE_TEMPLATE_SCHEMA_URI, CEM_SCHEMA_CONTENT_TYPE,
    CEM_SCHEMA_PACKAGE_CONTENT_TYPE, CEM_SCHEMA_PACKAGE_URI, CEM_SCHEMA_URI,
    CEM_TRANSFORM_CONTENT_TYPE, CEM_TRANSFORM_SCHEMA_URI,
};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack};
use crate::transform_template::{
    parse_cem_native_template_module_options,
    validate_transform_template_artifact_function_contract, TransformTemplateAdapter,
    TransformTemplateArtifactFunctionContract, TransformTemplateArtifactFunctionContractMismatch,
    TransformTemplateCompileRequest, TransformTemplateModuleOptions,
    TransformTemplateModuleParseRequest, TransformTemplateModulePreflight,
};
use crate::validation::{
    RuleContext, RuleDescriptor, RuleId, RuleInput, RuleResourceRead, SemanticRule, TriggerLayer,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub(crate) const SCHEMA_PACKAGE_CONVERTER_CONSTRAINT_DIAGNOSTICS: &[(&str, &str)] = &[
    (
        "cem.schema_package.converter_check",
        "cemt-template-identity-required",
    ),
    (
        "cem.schema_package.converter_check",
        "converter-template-output-stage-contract",
    ),
    (
        "cem.schema_package.converter_check",
        "converter-endpoint-schema-content-type-match",
    ),
    (
        "cem.schema_package.artifact_check",
        "artifact-output-stage-contract",
    ),
    (
        "cem.schema_package.example_content_type_mismatch",
        "example-contract",
    ),
];

fn diag_at(code: &str, severity: Severity, message: String, node: &CemAstNode) -> Diagnostic {
    let stack = source_stack_for_node(node);
    let byte_offset = stack.frames.first().and_then(|f| match &f.span {
        FrameSpan::Single(r) => Some(r.start),
        FrameSpan::Multi(rs) => rs.first().map(|r| r.start),
    });
    Diagnostic {
        uri: None,
        line: None,
        column: None,
        byte_offset,
        code: code.to_owned(),
        severity,
        message,
        node: None,
        details: None,
        source_map: Some(stack.clone()),
    }
}

fn diag_at_with_details(
    code: &str,
    severity: Severity,
    message: String,
    node: &CemAstNode,
    details: serde_json::Value,
) -> Diagnostic {
    let mut diagnostic = diag_at(code, severity, message, node);
    diagnostic.details = Some(details);
    diagnostic
}

fn source_stack_for_node(node: &CemAstNode) -> &SourceMapStack {
    match node {
        CemAstNode::Document { source, .. }
        | CemAstNode::Element { source, .. }
        | CemAstNode::Attribute { source, .. }
        | CemAstNode::Text { source, .. }
        | CemAstNode::Whitespace { source, .. }
        | CemAstNode::Comment { source, .. }
        | CemAstNode::ProcessingInstruction { source, .. }
        | CemAstNode::Cdata { source, .. }
        | CemAstNode::RawText { source, .. }
        | CemAstNode::Error { source, .. } => source,
    }
}

fn node_source_range_details(node: &CemAstNode) -> Option<serde_json::Value> {
    source_stack_for_node(node)
        .current()
        .map(source_frame_range_details)
}

fn source_frame_range_details(frame: &SourceMapFrame) -> serde_json::Value {
    serde_json::json!({
        "sourceId": frame.source_id.0,
        "span": frame_span_details(&frame.span),
    })
}

fn frame_span_details(span: &FrameSpan) -> serde_json::Value {
    match span {
        FrameSpan::Single(range) => serde_json::json!({
            "kind": "single",
            "start": range.start,
            "len": range.len,
            "end": range.end(),
        }),
        FrameSpan::Multi(ranges) => serde_json::json!({
            "kind": "multi",
            "ranges": ranges
                .iter()
                .map(|range| {
                    serde_json::json!({
                        "start": range.start,
                        "len": range.len,
                        "end": range.end(),
                    })
                })
                .collect::<Vec<_>>(),
        }),
    }
}

fn element_attributes<'a>(
    doc: &'a crate::parser::document::CemDocument,
    element: &'a CemAstNode,
) -> impl Iterator<Item = &'a CemAstNode> {
    let ids: &[AstNodeId] = match element {
        CemAstNode::Element { attributes, .. } => attributes,
        _ => &[],
    };
    ids.iter().filter_map(move |id| doc.get(*id))
}

fn element_local_name(node: &CemAstNode) -> Option<&str> {
    match node {
        CemAstNode::Element { expanded_name, .. } => Some(expanded_name.local_name.as_str()),
        _ => None,
    }
}

fn element_node_id(node: &CemAstNode) -> Option<AstNodeId> {
    match node {
        CemAstNode::Element { node_id, .. } => Some(*node_id),
        _ => None,
    }
}

fn attribute_parts(node: &CemAstNode) -> Option<(&str, &str, Option<&str>)> {
    if let CemAstNode::Attribute {
        expanded_name,
        value,
        ..
    } = node
    {
        Some((
            expanded_name.namespace_uri.as_str(),
            expanded_name.local_name.as_str(),
            value.as_deref(),
        ))
    } else {
        None
    }
}

fn attr_value<'a>(
    doc: &'a crate::parser::document::CemDocument,
    element: &'a CemAstNode,
    name: &str,
) -> Option<&'a str> {
    element_attributes(doc, element).find_map(|attr| {
        let (_, local, value) = attribute_parts(attr)?;
        (local == name).then_some(value).flatten()
    })
}

fn element_attribute_values(
    doc: &crate::parser::document::CemDocument,
    element: &CemAstNode,
) -> BTreeMap<String, String> {
    element_attributes(doc, element)
        .filter_map(|attr| {
            let (_, local, value) = attribute_parts(attr)?;
            Some((local.to_owned(), value.unwrap_or_default().to_owned()))
        })
        .collect()
}

fn element_child_ids_by_local_name(
    doc: &crate::parser::document::CemDocument,
    element: &CemAstNode,
    name: &str,
) -> Vec<AstNodeId> {
    let CemAstNode::Element { children, .. } = element else {
        return Vec::new();
    };
    children
        .iter()
        .copied()
        .filter(|child_id| {
            matches!(
                doc.get(*child_id),
                Some(CemAstNode::Element { expanded_name, .. })
                    if expanded_name.local_name == name
            )
        })
        .collect()
}

// ---------- Reference Integrity ----------

pub struct ReferenceIntegrityRule;

impl SemanticRule for ReferenceIntegrityRule {
    fn descriptor(&self) -> &RuleDescriptor {
        ref_integrity_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for node in ctx.document.iter() {
            let CemAstNode::Element { attributes, .. } = node else {
                continue;
            };
            for attr_id in attributes {
                let Some(attr) = ctx.document.get(*attr_id) else {
                    continue;
                };
                let Some((_, local, value)) = attribute_parts(attr) else {
                    continue;
                };
                let Some(value) = value else { continue };
                let is_reference = matches!(
                    local,
                    "for" | "aria-labelledby" | "aria-describedby" | "aria-controls" | "aria-owns"
                );
                if !is_reference {
                    continue;
                }
                let targets: Vec<&str> = if local == "for" {
                    vec![value]
                } else {
                    value.split_whitespace().collect()
                };
                for target in targets {
                    if target.is_empty() || ctx.document.id_table.contains_key(target) {
                        continue;
                    }
                    out.push(diag_at(
                        "cem.ref.unresolved_reference",
                        Severity::Warning,
                        format!("`{local}` reference `{target}` does not match any element id"),
                        attr,
                    ));
                }
            }
        }
        out
    }
}

fn ref_integrity_descriptor() -> &'static RuleDescriptor {
    use std::sync::OnceLock;
    static D: OnceLock<RuleDescriptor> = OnceLock::new();
    D.get_or_init(|| RuleDescriptor {
        id: RuleId::new("cem.ref.unresolved_reference"),
        owning_scope: "cem-a11y",
        content_type: None,
        trigger_layer: TriggerLayer::Document,
        required_inputs: &[RuleInput::CemDocument],
        default_severity: Severity::Warning,
        policy_overridable: true,
    })
}

// ---------- Accessible Name ----------

pub struct AccessibleNameRule;

impl SemanticRule for AccessibleNameRule {
    fn descriptor(&self) -> &RuleDescriptor {
        accessible_name_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for node in ctx.document.iter() {
            let Some(local) = element_local_name(node) else {
                continue;
            };
            if !is_interactive_element(local) {
                continue;
            }
            if has_accessible_name(ctx.document, node) {
                continue;
            }
            out.push(diag_at(
                "cem.a11y.accessible_name_missing",
                Severity::Warning,
                format!(
                    "interactive element `{local}` has no accessible name (text, `aria-label`, or `aria-labelledby` required)"
                ),
                node,
            ));
        }
        out
    }
}

fn accessible_name_descriptor() -> &'static RuleDescriptor {
    use std::sync::OnceLock;
    static D: OnceLock<RuleDescriptor> = OnceLock::new();
    D.get_or_init(|| RuleDescriptor {
        id: RuleId::new("cem.a11y.accessible_name_missing"),
        owning_scope: "cem-a11y",
        content_type: Some("text/html"),
        trigger_layer: TriggerLayer::Document,
        required_inputs: &[RuleInput::CemDocument],
        default_severity: Severity::Warning,
        policy_overridable: true,
    })
}

fn is_interactive_element(local: &str) -> bool {
    matches!(
        local,
        "button" | "a" | "input" | "select" | "textarea" | "summary"
    )
}

fn has_accessible_name(doc: &crate::parser::document::CemDocument, element: &CemAstNode) -> bool {
    // ARIA labelling attributes.
    for attr in element_attributes(doc, element) {
        let Some((_, local, value)) = attribute_parts(attr) else {
            continue;
        };
        if matches!(local, "aria-label" | "aria-labelledby" | "title")
            && value.map(|v| !v.trim().is_empty()).unwrap_or(false)
        {
            return true;
        }
    }
    if let Some(id) = attr_value(doc, element, "id") {
        if doc.iter().any(|node| {
            element_local_name(node) == Some("label")
                && attr_value(doc, node, "for")
                    .map(|value| value.split_whitespace().any(|target| target == id))
                    .unwrap_or(false)
                && has_visible_text(doc, node)
        }) {
            return true;
        }
    }
    if let Some(id) = element_node_id(element) {
        if doc.iter().any(|node| {
            element_local_name(node) == Some("label") && label_wraps_node_with_text(doc, node, id)
        }) {
            return true;
        }
    }
    // Text content (direct or in descendants).
    has_visible_text(doc, element)
}

fn has_visible_text(doc: &crate::parser::document::CemDocument, node: &CemAstNode) -> bool {
    match node {
        CemAstNode::Text { data, .. } => !data.trim().is_empty(),
        CemAstNode::Element { children, .. }
        | CemAstNode::Document {
            root_children: children,
            ..
        } => children.iter().any(|c| {
            doc.get(*c)
                .map(|n| has_visible_text(doc, n))
                .unwrap_or(false)
        }),
        _ => false,
    }
}

fn label_wraps_node_with_text(
    doc: &crate::parser::document::CemDocument,
    label: &CemAstNode,
    target: AstNodeId,
) -> bool {
    let CemAstNode::Element { children, .. } = label else {
        return false;
    };
    children.iter().any(|id| {
        *id == target
            || doc
                .get(*id)
                .map(|n| contains_node(doc, n, target))
                .unwrap_or(false)
    }) && has_visible_text(doc, label)
}

fn contains_node(
    doc: &crate::parser::document::CemDocument,
    node: &CemAstNode,
    target: AstNodeId,
) -> bool {
    match node {
        CemAstNode::Element {
            node_id, children, ..
        } => {
            *node_id == target
                || children.iter().any(|id| {
                    doc.get(*id)
                        .map(|n| contains_node(doc, n, target))
                        .unwrap_or(false)
                })
        }
        _ => false,
    }
}

// ---------- ARIA Compatibility ----------

pub struct AriaCompatibilityRule;

impl SemanticRule for AriaCompatibilityRule {
    fn descriptor(&self) -> &RuleDescriptor {
        aria_compat_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for node in ctx.document.iter() {
            let Some(local) = element_local_name(node) else {
                continue;
            };
            let role = attr_value(ctx.document, node, "role");
            if let Some(role) = role {
                if !KNOWN_ROLES.contains(&role) {
                    out.push(diag_at(
                        "cem.a11y.aria_incompatible",
                        Severity::Warning,
                        format!("ARIA role `{role}` is not in the Tier A compatibility table"),
                        node,
                    ));
                }
            }
            for attr in element_attributes(ctx.document, node) {
                let Some((_, attr_name, _)) = attribute_parts(attr) else {
                    continue;
                };
                let Some(allowed_roles) = aria_role_requirements(attr_name) else {
                    continue;
                };
                if role.map(|r| allowed_roles.contains(&r)).unwrap_or(false)
                    || native_allows_aria(local, attr_name, ctx.document, node)
                {
                    continue;
                }
                out.push(diag_at(
                    "cem.a11y.aria_incompatible",
                    Severity::Warning,
                    format!(
                        "`{attr_name}` is not compatible with `{local}` without one of roles: {}",
                        allowed_roles.join(", ")
                    ),
                    attr,
                ));
            }
        }
        out
    }
}

fn aria_compat_descriptor() -> &'static RuleDescriptor {
    use std::sync::OnceLock;
    static D: OnceLock<RuleDescriptor> = OnceLock::new();
    D.get_or_init(|| RuleDescriptor {
        id: RuleId::new("cem.a11y.aria_incompatible"),
        owning_scope: "cem-a11y",
        content_type: Some("text/html"),
        trigger_layer: TriggerLayer::Document,
        required_inputs: &[RuleInput::CemDocument],
        default_severity: Severity::Warning,
        policy_overridable: true,
    })
}

const KNOWN_ROLES: &[&str] = &[
    "alert",
    "button",
    "checkbox",
    "combobox",
    "dialog",
    "gridcell",
    "link",
    "listbox",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "option",
    "progressbar",
    "radio",
    "row",
    "scrollbar",
    "search",
    "slider",
    "spinbutton",
    "status",
    "switch",
    "tab",
    "tabpanel",
    "treeitem",
];

fn aria_role_requirements(attr: &str) -> Option<&'static [&'static str]> {
    match attr {
        "aria-checked" => Some(&[
            "checkbox",
            "menuitemcheckbox",
            "menuitemradio",
            "radio",
            "switch",
        ]),
        "aria-selected" => Some(&["gridcell", "option", "row", "tab"]),
        "aria-valuenow" | "aria-valuemin" | "aria-valuemax" => {
            Some(&["progressbar", "scrollbar", "slider", "spinbutton"])
        }
        "aria-expanded" => Some(&["button", "combobox", "link", "menuitem", "treeitem"]),
        _ => None,
    }
}

fn native_allows_aria(
    local: &str,
    attr: &str,
    doc: &crate::parser::document::CemDocument,
    node: &CemAstNode,
) -> bool {
    match (local, attr) {
        ("button" | "summary", "aria-expanded") => true,
        ("a", "aria-expanded") => attr_value(doc, node, "href").is_some(),
        ("input", "aria-checked") => {
            matches!(attr_value(doc, node, "type"), Some("checkbox" | "radio"))
        }
        _ => false,
    }
}

// ---------- SVG Accessibility ----------

pub struct SvgAccessibilityRule;

impl SemanticRule for SvgAccessibilityRule {
    fn descriptor(&self) -> &RuleDescriptor {
        svg_accessibility_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for node in ctx.document.iter() {
            if element_local_name(node) != Some("svg") {
                continue;
            }
            let hidden = attr_value(ctx.document, node, "aria-hidden")
                .map(|v| v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            let focusable = attr_value(ctx.document, node, "focusable")
                .map(|v| v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            if hidden && focusable {
                out.push(diag_at(
                    "cem.a11y.svg_focusable_hidden",
                    Severity::Warning,
                    "`svg` is both `aria-hidden=true` and focusable".to_owned(),
                    node,
                ));
            }
            if hidden || svg_has_accessible_name(ctx.document, node) {
                continue;
            }
            out.push(diag_at(
                "cem.a11y.svg_accessible_name_missing",
                Severity::Warning,
                "`svg` content must be `aria-hidden=true` or provide title/desc/ARIA name material"
                    .to_owned(),
                node,
            ));
        }
        out
    }
}

fn svg_accessibility_descriptor() -> &'static RuleDescriptor {
    use std::sync::OnceLock;
    static D: OnceLock<RuleDescriptor> = OnceLock::new();
    D.get_or_init(|| RuleDescriptor {
        id: RuleId::new("cem.a11y.svg_accessible_name_missing"),
        owning_scope: "cem-a11y",
        content_type: Some("image/svg+xml"),
        trigger_layer: TriggerLayer::Document,
        required_inputs: &[RuleInput::CemDocument],
        default_severity: Severity::Warning,
        policy_overridable: true,
    })
}

fn svg_has_accessible_name(doc: &crate::parser::document::CemDocument, node: &CemAstNode) -> bool {
    if attr_value(doc, node, "aria-label")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
        || attr_value(doc, node, "aria-labelledby")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    {
        return true;
    }
    let CemAstNode::Element { children, .. } = node else {
        return false;
    };
    children.iter().any(|id| {
        doc.get(*id)
            .map(|child| {
                matches!(element_local_name(child), Some("title" | "desc"))
                    && has_visible_text(doc, child)
            })
            .unwrap_or(false)
    })
}

// ---------- State Combination ----------

pub struct StateCombinationRule;

impl SemanticRule for StateCombinationRule {
    fn descriptor(&self) -> &RuleDescriptor {
        state_combo_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for node in ctx.document.iter() {
            let CemAstNode::Element { .. } = node else {
                continue;
            };
            let mut states: Vec<String> = Vec::new();
            for attr in element_attributes(ctx.document, node) {
                let Some((ns, local, value)) = attribute_parts(attr) else {
                    continue;
                };
                if ns == "cem" && local == "state" {
                    if let Some(v) = value {
                        for tok in v.split_whitespace() {
                            states.push(tok.to_owned());
                        }
                    }
                }
            }
            if states.is_empty() {
                continue;
            }
            for (a, b) in DISALLOWED_PAIRS {
                if states.iter().any(|s| s == a) && states.iter().any(|s| s == b) {
                    out.push(diag_at(
                        "cem.state.invalid_combination",
                        Severity::Error,
                        format!(
                            "states `{a}` and `{b}` cannot apply to the same element simultaneously"
                        ),
                        node,
                    ));
                }
            }
        }
        out
    }
}

fn state_combo_descriptor() -> &'static RuleDescriptor {
    use std::sync::OnceLock;
    static D: OnceLock<RuleDescriptor> = OnceLock::new();
    D.get_or_init(|| RuleDescriptor {
        id: RuleId::new("cem.state.invalid_combination"),
        owning_scope: "cem-core",
        content_type: None,
        trigger_layer: TriggerLayer::Document,
        required_inputs: &[RuleInput::CemDocument],
        default_severity: Severity::Error,
        policy_overridable: false,
    })
}

const DISALLOWED_PAIRS: &[(&str, &str)] = &[
    ("disabled", "loading"),
    ("disabled", "active"),
    ("disabled", "hover"),
    ("disabled", "focus-visible"),
    ("disabled", "selected"),
    ("empty", "loading"),
    ("default", "hover"),
    ("default", "focus-visible"),
    ("default", "active"),
    ("default", "selected"),
    ("default", "disabled"),
    ("default", "invalid"),
    ("default", "required"),
    ("default", "loading"),
    ("default", "empty"),
];

pub struct StateTransitionRule;

impl SemanticRule for StateTransitionRule {
    fn descriptor(&self) -> &RuleDescriptor {
        state_transition_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for node in ctx.document.iter() {
            let CemAstNode::Element { .. } = node else {
                continue;
            };
            let Some(local) = element_local_name(node) else {
                continue;
            };
            let Some(states) = cem_state_tokens(ctx.document, node) else {
                continue;
            };
            for state in states {
                if matches!(state, "required" | "invalid") && !is_form_state_host(local) {
                    out.push(diag_at(
                        "cem.state.invalid_transition",
                        Severity::Warning,
                        format!("state `{state}` is only valid on form-associated host elements"),
                        node,
                    ));
                }
            }
        }
        out
    }
}

fn state_transition_descriptor() -> &'static RuleDescriptor {
    use std::sync::OnceLock;
    static D: OnceLock<RuleDescriptor> = OnceLock::new();
    D.get_or_init(|| RuleDescriptor {
        id: RuleId::new("cem.state.invalid_transition"),
        owning_scope: "cem-core",
        content_type: None,
        trigger_layer: TriggerLayer::Document,
        required_inputs: &[RuleInput::CemDocument],
        default_severity: Severity::Warning,
        policy_overridable: true,
    })
}

fn cem_state_tokens<'a>(
    doc: &'a crate::parser::document::CemDocument,
    node: &'a CemAstNode,
) -> Option<Vec<&'a str>> {
    element_attributes(doc, node).find_map(|attr| {
        let (ns, local, value) = attribute_parts(attr)?;
        (ns == "cem" && local == "state").then(|| value.unwrap_or("").split_whitespace().collect())
    })
}

fn is_form_state_host(local: &str) -> bool {
    matches!(
        local,
        "button"
            | "fieldset"
            | "form"
            | "input"
            | "meter"
            | "option"
            | "output"
            | "progress"
            | "select"
            | "textarea"
    )
}

// ---------- Unsafe Content ----------

pub struct JavaScriptUrlRule;

impl SemanticRule for JavaScriptUrlRule {
    fn descriptor(&self) -> &RuleDescriptor {
        js_url_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for node in ctx.document.iter() {
            let CemAstNode::Attribute {
                expanded_name,
                value: Some(value),
                ..
            } = node
            else {
                continue;
            };
            if !is_url_bearing_attribute(&expanded_name.local_name) {
                continue;
            }
            let trimmed = value.trim_start().to_ascii_lowercase();
            if trimmed.starts_with("javascript:") {
                out.push(diag_at(
                    "cem.unsafe.javascript_url",
                    Severity::Error,
                    format!(
                        "`{}` attribute carries a `javascript:` URL, which is policy-rejected",
                        expanded_name.local_name
                    ),
                    node,
                ));
            }
        }
        out
    }
}

fn js_url_descriptor() -> &'static RuleDescriptor {
    use std::sync::OnceLock;
    static D: OnceLock<RuleDescriptor> = OnceLock::new();
    D.get_or_init(|| RuleDescriptor {
        id: RuleId::new("cem.unsafe.javascript_url"),
        owning_scope: "cem-policy",
        content_type: Some("text/html"),
        trigger_layer: TriggerLayer::Document,
        required_inputs: &[RuleInput::CemDocument],
        default_severity: Severity::Error,
        policy_overridable: false,
    })
}

fn is_url_bearing_attribute(local: &str) -> bool {
    matches!(
        local,
        "href" | "src" | "action" | "formaction" | "xlink:href" | "ping" | "data"
    )
}

// ---------- Authoring Lints ----------

pub struct UnboundPrefixRule;

impl SemanticRule for UnboundPrefixRule {
    fn descriptor(&self) -> &RuleDescriptor {
        unbound_prefix_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        // The active CEM Core schema binds the `cem:` prefix; the Tier A
        // tokenizer also recognizes the lexical `html` and `svg` hints in
        // the example fixtures. Any other namespace prefix on an attribute is
        // an unbound-prefix lint unless the active schema owns that prefixed
        // attribute family.
        const KNOWN_PREFIXES: &[&str] = &["cem", "html", "svg", "xml", "xmlns", "aria", "xlink"];
        let mut out = Vec::new();
        for node in ctx.document.iter() {
            let CemAstNode::Attribute { expanded_name, .. } = node else {
                continue;
            };
            let prefix = &expanded_name.namespace_uri;
            if prefix.is_empty() {
                continue;
            }
            if KNOWN_PREFIXES.contains(&prefix.as_str()) {
                continue;
            }
            if prefix == "with" && is_template_family_language_document(ctx) {
                continue;
            }
            out.push(diag_at(
                "cem.lint.unbound_prefix",
                Severity::Warning,
                format!(
                    "namespace prefix `{prefix}` on `@{prefix}:{}` is not bound by any active `@ns` declaration",
                    expanded_name.local_name
                ),
                node,
            ));
        }
        out
    }
}

fn unbound_prefix_descriptor() -> &'static RuleDescriptor {
    use std::sync::OnceLock;
    static D: OnceLock<RuleDescriptor> = OnceLock::new();
    D.get_or_init(|| RuleDescriptor {
        id: RuleId::new("cem.lint.unbound_prefix"),
        owning_scope: "cem-lint",
        content_type: None,
        trigger_layer: TriggerLayer::Document,
        required_inputs: &[RuleInput::CemDocument],
        default_severity: Severity::Warning,
        policy_overridable: true,
    })
}

pub struct NoncanonicalDelimiterRule;

impl SemanticRule for NoncanonicalDelimiterRule {
    fn descriptor(&self) -> &RuleDescriptor {
        noncanonical_delimiter_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        // The Unicode content-boundary `▷` is accepted by the tokenizer
        // but the canonical CEM-ML surface uses ASCII `|`. We can't
        // detect the literal character at AST level reliably, but we
        // *can* flag attribute values whose canonical form would have
        // been a bare identifier yet were quoted. That's a noncanonical
        // delimiter choice the formatter would normalize.
        let mut out = Vec::new();
        for node in ctx.document.iter() {
            let CemAstNode::Attribute {
                expanded_name,
                value: Some(v),
                ..
            } = node
            else {
                continue;
            };
            // Quoted single-identifier values without whitespace should
            // be bare in canonical form. The tokenizer strips the
            // surrounding quotes before placing into `value`, so we
            // can't see the quotes here directly; we approximate by
            // flagging values that *would* be bare-eligible but appear
            // to have been authored with leading/trailing whitespace
            // (a hint they were quoted unnecessarily).
            if v != v.trim() && is_bare_eligible(v.trim()) {
                out.push(diag_at(
                    "cem.lint.noncanonical_delimiter",
                    Severity::Info,
                    format!(
                        "attribute `@{}=\"{}\"` has surrounding whitespace; the canonical form is `@{}={}`",
                        expanded_name.local_name,
                        v,
                        expanded_name.local_name,
                        v.trim()
                    ),
                    node,
                ));
            }
        }
        out
    }
}

fn noncanonical_delimiter_descriptor() -> &'static RuleDescriptor {
    use std::sync::OnceLock;
    static D: OnceLock<RuleDescriptor> = OnceLock::new();
    D.get_or_init(|| RuleDescriptor {
        id: RuleId::new("cem.lint.noncanonical_delimiter"),
        owning_scope: "cem-lint",
        content_type: None,
        trigger_layer: TriggerLayer::Document,
        required_inputs: &[RuleInput::CemDocument],
        default_severity: Severity::Info,
        policy_overridable: true,
    })
}

fn is_bare_eligible(v: &str) -> bool {
    !v.is_empty()
        && v.chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/' | '.' | ':'))
}

pub struct SuspiciousContentTypeSwitchRule;

impl SemanticRule for SuspiciousContentTypeSwitchRule {
    fn descriptor(&self) -> &RuleDescriptor {
        suspicious_content_type_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        // A `@type="..."` attribute on an anonymous scope is a
        // content-type handoff (`cem-ml-syntax.md`
        // §"Content-Type Handoffs Stay Schema-Owned"). On a *named*
        // element it's an ordinary HTML attribute; on anything other
        // than an anonymous scope or an `<input>`/`<button>`/`<source>`-
        // family node with a known `type=` enum, a non-MIME value is a
        // lint warning because it might have been intended as a handoff.
        let mut out = Vec::new();
        const MIME_HOSTS: &[&str] = &[
            "script", "style", "link", "source", "embed", "object", "audio", "video",
        ];
        for node in ctx.document.iter() {
            let CemAstNode::Element {
                expanded_name,
                attributes,
                ..
            } = node
            else {
                continue;
            };
            let local = expanded_name.local_name.as_str();
            if local.is_empty() {
                continue; // anonymous scopes handled by schema machine
            }
            for attr_id in attributes {
                let Some(attr) = ctx.document.get(*attr_id) else {
                    continue;
                };
                let Some((ns, name, val)) = attribute_parts(attr) else {
                    continue;
                };
                if !ns.is_empty() || name != "type" {
                    continue;
                }
                let Some(v) = val else { continue };
                // A MIME-style value (`text/*`, `application/*`,
                // `image/*`, etc.) on a non-MIME-host element is the
                // suspicious case.
                if !MIME_HOSTS.contains(&local) && looks_like_mime(v) {
                    out.push(diag_at(
                        "cem.lint.suspicious_content_type_switch",
                        Severity::Warning,
                        format!(
                            "`<{local} type=\"{v}\">` looks like a content-type handoff but `{local}` is not a known MIME host; did you mean to wrap in an anonymous scope `{{@type=\"{v}\" | ...}}`?"
                        ),
                        attr,
                    ));
                }
            }
        }
        out
    }
}

fn suspicious_content_type_descriptor() -> &'static RuleDescriptor {
    use std::sync::OnceLock;
    static D: OnceLock<RuleDescriptor> = OnceLock::new();
    D.get_or_init(|| RuleDescriptor {
        id: RuleId::new("cem.lint.suspicious_content_type_switch"),
        owning_scope: "cem-lint",
        content_type: None,
        trigger_layer: TriggerLayer::Document,
        required_inputs: &[RuleInput::CemDocument],
        default_severity: Severity::Warning,
        policy_overridable: true,
    })
}

fn looks_like_mime(v: &str) -> bool {
    v.contains('/') && v.chars().all(|c| !c.is_whitespace())
}

pub struct EventHandlerAttributeRule;

impl SemanticRule for EventHandlerAttributeRule {
    fn descriptor(&self) -> &RuleDescriptor {
        event_handler_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for node in ctx.document.iter() {
            let CemAstNode::Attribute { expanded_name, .. } = node else {
                continue;
            };
            let local = &expanded_name.local_name;
            // DOM-style event handlers: `on*` with at least one trailing char.
            if local.len() > 2
                && local.starts_with("on")
                && local.chars().nth(2).unwrap().is_alphabetic()
            {
                out.push(diag_at(
                    "cem.unsafe.event_handler_attribute",
                    Severity::Error,
                    format!(
                        "event-handler attribute `{local}` is policy-rejected; use CEM action annotations instead"
                    ),
                    node,
                ));
            }
        }
        out
    }
}

fn event_handler_descriptor() -> &'static RuleDescriptor {
    use std::sync::OnceLock;
    static D: OnceLock<RuleDescriptor> = OnceLock::new();
    D.get_or_init(|| RuleDescriptor {
        id: RuleId::new("cem.unsafe.event_handler_attribute"),
        owning_scope: "cem-policy",
        content_type: Some("text/html"),
        trigger_layer: TriggerLayer::Document,
        required_inputs: &[RuleInput::CemDocument],
        default_severity: Severity::Error,
        policy_overridable: false,
    })
}

pub struct UnsafeInlineContentRule;

impl SemanticRule for UnsafeInlineContentRule {
    fn descriptor(&self) -> &RuleDescriptor {
        unsafe_inline_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for node in ctx.document.iter() {
            match node {
                CemAstNode::Element {
                    expanded_name,
                    children,
                    ..
                } if expanded_name.local_name == "script"
                    && has_significant_content(ctx.document, children) =>
                {
                    out.push(diag_at(
                        "cem.unsafe.inline_script",
                        Severity::Error,
                        "inline `script` content is policy-rejected in Tier A semantic documents"
                            .to_owned(),
                        node,
                    ));
                }
                CemAstNode::Attribute { expanded_name, .. }
                    if expanded_name.local_name == "srcdoc" =>
                {
                    out.push(diag_at(
                        "cem.unsafe.srcdoc",
                        Severity::Error,
                        "`srcdoc` embeds an inline HTML document and is policy-gated".to_owned(),
                        node,
                    ));
                }
                CemAstNode::ProcessingInstruction { target, data, .. }
                    if target.eq_ignore_ascii_case("DOCTYPE")
                        && (data.contains("SYSTEM") || data.contains("PUBLIC")) =>
                {
                    out.push(diag_at(
                        "cem.unsafe.external_dtd",
                        Severity::Error,
                        "external DTD declarations are policy-rejected".to_owned(),
                        node,
                    ));
                }
                _ => {}
            }
        }
        out
    }
}

fn unsafe_inline_descriptor() -> &'static RuleDescriptor {
    use std::sync::OnceLock;
    static D: OnceLock<RuleDescriptor> = OnceLock::new();
    D.get_or_init(|| RuleDescriptor {
        id: RuleId::new("cem.unsafe.inline_content"),
        owning_scope: "cem-policy",
        content_type: None,
        trigger_layer: TriggerLayer::Document,
        required_inputs: &[RuleInput::CemDocument],
        default_severity: Severity::Error,
        policy_overridable: false,
    })
}

// ---------- Schema Document Model ----------

pub struct SchemaDocumentModelRule;

impl SemanticRule for SchemaDocumentModelRule {
    fn descriptor(&self) -> &RuleDescriptor {
        schema_document_model_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        let Some(model) =
            load_builtin_document_model_for_identity(ctx.schema_uri, ctx.content_type)
        else {
            return Vec::new();
        };

        validate_document_model(ctx.document, &model)
    }
}

fn schema_document_model_descriptor() -> &'static RuleDescriptor {
    use std::sync::OnceLock;
    static D: OnceLock<RuleDescriptor> = OnceLock::new();
    D.get_or_init(|| RuleDescriptor {
        id: RuleId::new("cem.schema_model.document_model"),
        owning_scope: "cem-core",
        content_type: None,
        trigger_layer: TriggerLayer::Document,
        required_inputs: &[RuleInput::CemDocument],
        default_severity: Severity::Error,
        policy_overridable: false,
    })
}

// ---------- Schema Package Converter Contract ----------

pub struct SchemaPackageConverterContractRule;

impl SemanticRule for SchemaPackageConverterContractRule {
    fn descriptor(&self) -> &RuleDescriptor {
        schema_package_converter_contract_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        if !is_schema_package_manifest_document(ctx) {
            return Vec::new();
        }

        let registry = SchemaRegistry::with_builtin_schemas();
        let mut out = Vec::new();
        for node in ctx.document.iter() {
            match element_local_name(node) {
                Some("converter") => {
                    validate_schema_package_converter(ctx, node, &registry, &mut out)
                }
                Some("artifact") => validate_schema_package_artifact(ctx, node, &mut out),
                Some("example") => {
                    validate_schema_package_example(ctx.document, node, &registry, &mut out)
                }
                _ => {}
            }
        }
        out
    }
}

fn validate_schema_package_converter(
    ctx: &RuleContext<'_>,
    node: &CemAstNode,
    registry: &SchemaRegistry,
    out: &mut Vec<Diagnostic>,
) {
    let doc = ctx.document;
    let converter_id = attr_value(doc, node, "id")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("<missing>");

    match attr_value(doc, node, "implementation")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some("cemt") => validate_cemt_converter_contract(ctx, node, converter_id, out),
        Some("rust") => {}
        Some(_) => {}
        None => {}
    }

    validate_converter_endpoint_content_type_contract(
        doc,
        node,
        converter_id,
        "from",
        registry,
        out,
    );
    validate_converter_endpoint_content_type_contract(doc, node, converter_id, "to", registry, out);
}

fn validate_cemt_converter_contract(
    ctx: &RuleContext<'_>,
    node: &CemAstNode,
    converter_id: &str,
    out: &mut Vec<Diagnostic>,
) {
    let doc = ctx.document;
    let template_path = attr_value(doc, node, "template")
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let Some(template_content_type) = attr_value(doc, node, "template-content-type")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    let template_content_type_valid =
        content_type_essence(template_content_type) == CEM_TRANSFORM_CONTENT_TYPE;

    let claims_formatter_coloring_pipeline =
        cemt_converter_claims_formatter_coloring_output_pipeline(doc, node);
    let template_schema = attr_value(doc, node, "template-schema")
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mut template_schema_valid = false;
    if let Some(template_schema) = template_schema {
        template_schema_valid = template_schema == CEM_TRANSFORM_SCHEMA_URI;
    }

    if claims_formatter_coloring_pipeline && template_content_type_valid && template_schema_valid {
        validate_cemt_converter_template_source_contract(
            ctx,
            node,
            converter_id,
            template_path,
            template_content_type,
            template_schema,
            out,
        );
    }
}

fn cemt_converter_claims_formatter_coloring_output_pipeline(
    doc: &crate::parser::document::CemDocument,
    node: &CemAstNode,
) -> bool {
    attr_value(doc, node, "output-syntax")
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
        && attr_value(doc, node, "encoding-category")
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        && (attr_value(doc, node, "formatter-profile")
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
            || attr_value(doc, node, "color-profile")
                .map(str::trim)
                .is_some_and(|value| !value.is_empty()))
        && element_child_ids_by_local_name(doc, node, "to")
            .into_iter()
            .filter_map(|endpoint_id| doc.get(endpoint_id))
            .any(|endpoint| {
                attr_value(doc, endpoint, "schema")
                    .map(str::trim)
                    .is_some_and(|value| !value.is_empty())
            })
}

fn validate_cemt_converter_template_source_contract(
    ctx: &RuleContext<'_>,
    node: &CemAstNode,
    converter_id: &str,
    template_path: Option<&str>,
    template_content_type: &str,
    template_schema: Option<&str>,
    out: &mut Vec<Diagnostic>,
) {
    let Some(template_path) = template_path else {
        return;
    };
    let Some(source) =
        read_schema_package_converter_template_source(ctx, node, template_path, converter_id, out)
    else {
        return;
    };

    let template_input = TemplateInput {
        uri: source.uri,
        bytes: source.bytes,
        identity: Some(FormatIdentity {
            content_type: Some(content_type_essence(template_content_type)),
            schema: template_schema.map(str::to_owned),
            ..FormatIdentity::default()
        }),
        root_scope: ScopeConfig::default(),
    };
    let entrypoint = attr_value(ctx.document, node, "template-entrypoint")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(TransformTemplateEntrypoint::named)
        .unwrap_or_else(TransformTemplateEntrypoint::implicit);
    let params = BTreeMap::new();
    let data_bindings = vec!["input".to_owned()];
    let adapter = DomProjectionParityCemtAdapter;
    let compile_response = match adapter.compile(TransformTemplateCompileRequest {
        template: &template_input,
        entrypoint: &entrypoint,
        params: &params,
        data_bindings: &data_bindings,
        module_options: TransformTemplateModuleOptions::default(),
        module_preflight: TransformTemplateModulePreflight::default(),
        execution_policy: TransformExecutionPolicy::default(),
    }) {
        Ok(response) => response,
        Err(error) => {
            out.push(schema_package_converter_template_contract_failed_diag(
                ctx.document,
                node,
                converter_id,
                template_path,
                error.to_string(),
                None,
            ));
            return;
        }
    };

    if let Some(diagnostic) = compile_response
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        out.push(schema_package_converter_template_contract_failed_diag(
            ctx.document,
            node,
            converter_id,
            template_path,
            format!(
                "template compile emitted hard diagnostic `{}`",
                diagnostic.code
            ),
            Some(diagnostic),
        ));
    }
}

fn read_schema_package_converter_template_source(
    ctx: &RuleContext<'_>,
    node: &CemAstNode,
    template_path: &str,
    converter_id: &str,
    out: &mut Vec<Diagnostic>,
) -> Option<RuleResourceRead> {
    if let Some(source_path) = resolve_schema_package_resource_path(ctx.source_uri, template_path) {
        return match std::fs::read(&source_path) {
            Ok(bytes) => Some(RuleResourceRead {
                uri: source_path.display().to_string(),
                bytes,
                content_type: attr_value(ctx.document, node, "template-content-type")
                    .map(str::to_owned),
            }),
            Err(error) => {
                out.push(schema_package_converter_template_source_read_failed_diag(
                    ctx.document,
                    node,
                    template_path,
                    converter_id,
                    error.to_string(),
                ));
                None
            }
        };
    }

    let Some(resource_reader) = ctx.resource_reader else {
        return None;
    };
    match resource_reader(
        template_path,
        ctx.source_uri,
        attr_value(ctx.document, node, "template-content-type"),
    ) {
        Ok(source) => Some(source),
        Err(error) => {
            out.push(schema_package_converter_template_source_read_failed_diag(
                ctx.document,
                node,
                template_path,
                converter_id,
                error,
            ));
            None
        }
    }
}

fn schema_package_converter_template_source_read_failed_diag(
    doc: &crate::parser::document::CemDocument,
    node: &CemAstNode,
    template_path: &str,
    converter_id: &str,
    error: impl AsRef<str>,
) -> Diagnostic {
    diag_at_with_details(
        "cem.schema_package.converter_check",
        Severity::Error,
        format!(
            "template `{template_path}` referenced by CEMT converter `{converter_id}` could not be read: {}",
            error.as_ref()
        ),
        node,
        serde_json::json!({
            "schemaUri": CEM_SCHEMA_PACKAGE_URI,
            "element": "converter",
            "contract": "converter-template-output-stage-contract",
            "target": "converter",
            "diagnostic": "cem.schema_package.converter_check",
            "checkKind": "converter-template-source-readable",
            "converterId": converter_id,
            "invalidFields": ["template"],
            "invalidValues": {
                "template": template_path,
            },
            "error": error.as_ref(),
            "actualValues": element_attribute_values(doc, node),
            "sourceRange": node_source_range_details(node),
        }),
    )
}

fn schema_package_converter_template_contract_failed_diag(
    doc: &crate::parser::document::CemDocument,
    node: &CemAstNode,
    converter_id: &str,
    template_path: &str,
    error: impl AsRef<str>,
    source_diagnostic: Option<&Diagnostic>,
) -> Diagnostic {
    let source_diagnostic_details = source_diagnostic.map(|diagnostic| {
        serde_json::json!({
            "code": diagnostic.code,
            "severity": format!("{:?}", diagnostic.severity),
            "message": diagnostic.message,
        })
    });

    diag_at_with_details(
        "cem.schema_package.converter_check",
        Severity::Error,
        format!(
            "CEMT converter `{converter_id}` formatter/coloring output pipeline requires a template that can render a formatted CEM tree before the writer: {}",
            error.as_ref()
        ),
        node,
        serde_json::json!({
            "schemaUri": CEM_SCHEMA_PACKAGE_URI,
            "element": "converter",
            "contract": "converter-template-output-stage-contract",
            "target": "converter",
            "diagnostic": "cem.schema_package.converter_check",
            "checkKind": "converter-template-contract",
            "converterId": converter_id,
            "invalidFields": ["template"],
            "invalidValues": {
                "template": template_path,
            },
            "error": error.as_ref(),
            "sourceDiagnostic": source_diagnostic_details,
            "actualValues": element_attribute_values(doc, node),
            "sourceRange": node_source_range_details(node),
        }),
    )
}

fn validate_converter_endpoint_content_type_contract(
    doc: &crate::parser::document::CemDocument,
    node: &CemAstNode,
    converter_id: &str,
    endpoint_name: &str,
    registry: &SchemaRegistry,
    out: &mut Vec<Diagnostic>,
) {
    for endpoint_id in element_child_ids_by_local_name(doc, node, endpoint_name) {
        let Some(endpoint) = doc.get(endpoint_id) else {
            continue;
        };
        validate_endpoint_schema_content_type(
            doc,
            endpoint,
            converter_id,
            endpoint_name,
            registry,
            out,
        );
    }
}

fn validate_endpoint_schema_content_type(
    doc: &crate::parser::document::CemDocument,
    endpoint: &CemAstNode,
    converter_id: &str,
    endpoint_name: &str,
    registry: &SchemaRegistry,
    out: &mut Vec<Diagnostic>,
) {
    let Some(content_type) = attr_value(doc, endpoint, "content-type")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    let Some(schema_uri) = attr_value(doc, endpoint, "schema")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    let Some(schema) = registry.schema(schema_uri) else {
        return;
    };
    let essence = content_type_essence(content_type);
    let allowed_content_types = schema
        .content_type_essences()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if !allowed_content_types
        .iter()
        .any(|allowed| allowed == &essence)
    {
        out.push(
            schema_package_converter_endpoint_content_type_mismatch_diag(
                doc,
                endpoint,
                converter_id,
                endpoint_name,
                content_type,
                schema_uri,
                &allowed_content_types,
            ),
        );
    }
}

fn schema_package_converter_endpoint_content_type_mismatch_diag(
    doc: &crate::parser::document::CemDocument,
    endpoint: &CemAstNode,
    converter_id: &str,
    endpoint_name: &str,
    content_type: &str,
    schema_uri: &str,
    allowed_content_types: &[String],
) -> Diagnostic {
    diag_at_with_details(
        "cem.schema_package.converter_check",
        Severity::Error,
        format!(
            "converter `{converter_id}` `{endpoint_name}` endpoint content type `{content_type}` is not declared by schema `{schema_uri}`"
        ),
        endpoint,
        serde_json::json!({
            "schemaUri": CEM_SCHEMA_PACKAGE_URI,
            "element": endpoint_name,
            "contract": "converter-endpoint-schema-content-type-match",
            "target": endpoint_name,
            "diagnostic": "cem.schema_package.converter_check",
            "checkKind": "endpoint-content-type-schema",
            "converterId": converter_id,
            "endpoint": endpoint_name,
            "schema": schema_uri,
            "invalidFields": ["content-type"],
            "expectedValues": {
                "content-type": allowed_content_types,
            },
            "invalidValues": {
                "content-type": content_type,
            },
            "actualValues": element_attribute_values(doc, endpoint),
            "sourceRange": node_source_range_details(endpoint),
        }),
    )
}

fn validate_schema_package_artifact(
    ctx: &RuleContext<'_>,
    node: &CemAstNode,
    out: &mut Vec<Diagnostic>,
) {
    let Some(function_name) = attr_value(ctx.document, node, "function-name")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    let Some(path) = attr_value(ctx.document, node, "path")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    let Some(source) = read_schema_package_artifact_source(ctx, node, path, function_name, out)
    else {
        return;
    };

    let parse_response =
        parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
            template: TemplateInput {
                uri: source.uri,
                bytes: source.bytes,
                identity: Some(FormatIdentity {
                    content_type: attr_value(ctx.document, node, "content-type")
                        .map(content_type_essence),
                    schema: attr_value(ctx.document, node, "schema")
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned),
                    ..FormatIdentity::default()
                }),
                root_scope: ScopeConfig::default(),
            },
        });
    if let Some(diagnostic) = parse_response
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        out.push(schema_package_artifact_cemt_parse_failed_diag(
            ctx.document,
            node,
            path,
            function_name,
            diagnostic,
        ));
        return;
    }

    let Some(function) = parse_response
        .module_options
        .output_functions
        .iter()
        .find(|function| function.name == function_name)
    else {
        let declared_functions = parse_response
            .module_options
            .output_functions
            .iter()
            .map(|function| function.name.clone())
            .collect::<Vec<_>>();
        out.push(schema_package_artifact_function_lookup_failed_diag(
            ctx.document,
            node,
            path,
            function_name,
            declared_functions,
        ));
        return;
    };

    out.extend(
        validate_transform_template_artifact_function_contract(
            function,
            TransformTemplateArtifactFunctionContract {
                artifact_kind: attr_value(ctx.document, node, "kind"),
                target_content_type: attr_value(ctx.document, node, "target-content-type"),
                target_schema: attr_value(ctx.document, node, "target-schema"),
                target_category: attr_value(ctx.document, node, "target-category"),
                function_profile: attr_value(ctx.document, node, "function-profile"),
            },
        )
        .into_iter()
        .map(|mismatch| {
            schema_package_artifact_contract_mismatch_diag(ctx.document, node, path, mismatch)
        }),
    );
}

fn read_schema_package_artifact_source(
    ctx: &RuleContext<'_>,
    node: &CemAstNode,
    path: &str,
    function_name: &str,
    out: &mut Vec<Diagnostic>,
) -> Option<RuleResourceRead> {
    if let Some(source_path) = resolve_schema_package_resource_path(ctx.source_uri, path) {
        return match std::fs::read(&source_path) {
            Ok(bytes) => Some(RuleResourceRead {
                uri: source_path.display().to_string(),
                bytes,
                content_type: attr_value(ctx.document, node, "content-type").map(str::to_owned),
            }),
            Err(error) => {
                out.push(schema_package_artifact_source_read_failed_diag(
                    ctx.document,
                    node,
                    path,
                    function_name,
                    error.to_string(),
                ));
                None
            }
        };
    }

    let Some(resource_reader) = ctx.resource_reader else {
        return None;
    };
    match resource_reader(
        path,
        ctx.source_uri,
        attr_value(ctx.document, node, "content-type"),
    ) {
        Ok(source) => Some(source),
        Err(error) => {
            out.push(schema_package_artifact_source_read_failed_diag(
                ctx.document,
                node,
                path,
                function_name,
                error,
            ));
            None
        }
    }
}

fn schema_package_artifact_source_read_failed_diag(
    doc: &crate::parser::document::CemDocument,
    node: &CemAstNode,
    path: &str,
    function_name: &str,
    error: impl AsRef<str>,
) -> Diagnostic {
    diag_at_with_details(
        "cem.schema_package.artifact_check",
        Severity::Error,
        format!(
            "artifact `{path}` referenced by CEMT function `{function_name}` could not be read: {}",
            error.as_ref()
        ),
        node,
        serde_json::json!({
            "schemaUri": CEM_SCHEMA_PACKAGE_URI,
            "element": "artifact",
            "contract": "artifact-output-stage-contract",
            "target": "artifact",
            "diagnostic": "cem.schema_package.artifact_check",
            "checkKind": "artifact-source-readable",
            "path": path,
            "functionName": function_name,
            "invalidFields": ["path"],
            "invalidValues": {
                "path": path,
            },
            "error": error.as_ref(),
            "actualValues": element_attribute_values(doc, node),
            "sourceRange": node_source_range_details(node),
        }),
    )
}

fn schema_package_artifact_cemt_parse_failed_diag(
    doc: &crate::parser::document::CemDocument,
    node: &CemAstNode,
    path: &str,
    function_name: &str,
    diagnostic: &Diagnostic,
) -> Diagnostic {
    diag_at_with_details(
        "cem.schema_package.artifact_check",
        Severity::Error,
        format!(
            "artifact `{path}` CEMT source is invalid: {}",
            diagnostic.message
        ),
        node,
        serde_json::json!({
            "schemaUri": CEM_SCHEMA_PACKAGE_URI,
            "element": "artifact",
            "contract": "artifact-output-stage-contract",
            "target": "artifact",
            "diagnostic": "cem.schema_package.artifact_check",
            "checkKind": "artifact-cemt-valid",
            "path": path,
            "functionName": function_name,
            "invalidFields": ["path"],
            "invalidValues": {
                "path": path,
            },
            "sourceDiagnostic": {
                "code": diagnostic.code,
                "severity": format!("{:?}", diagnostic.severity),
                "message": diagnostic.message,
            },
            "actualValues": element_attribute_values(doc, node),
            "sourceRange": node_source_range_details(node),
        }),
    )
}

fn schema_package_artifact_function_lookup_failed_diag(
    doc: &crate::parser::document::CemDocument,
    node: &CemAstNode,
    path: &str,
    function_name: &str,
    declared_functions: Vec<String>,
) -> Diagnostic {
    diag_at_with_details(
        "cem.schema_package.artifact_check",
        Severity::Error,
        format!("artifact `{path}` does not declare CEMT output function `{function_name}`"),
        node,
        serde_json::json!({
            "schemaUri": CEM_SCHEMA_PACKAGE_URI,
            "element": "artifact",
            "contract": "artifact-output-stage-contract",
            "target": "artifact",
            "diagnostic": "cem.schema_package.artifact_check",
            "checkKind": "artifact-function-declared",
            "path": path,
            "functionName": function_name,
            "invalidFields": ["function-name"],
            "expectedValues": {
                "function-name": function_name,
            },
            "invalidValues": {
                "function-name": "<not declared>",
            },
            "declaredFunctions": declared_functions,
            "actualValues": element_attribute_values(doc, node),
            "sourceRange": node_source_range_details(node),
        }),
    )
}

fn resolve_schema_package_resource_path(
    source_uri: Option<&str>,
    resource_path: &str,
) -> Option<PathBuf> {
    let resource_path = resource_path.trim();
    if resource_path.is_empty() {
        return None;
    }
    if let Some(parsed) = parse_local_file_uri(resource_path) {
        return parsed.ok();
    }
    if has_uri_scheme(resource_path) && !is_windows_drive_path(resource_path) {
        return None;
    }
    let resource_path = PathBuf::from(resource_path);
    if resource_path.is_absolute() {
        return Some(resource_path);
    }

    let source_uri = source_uri?;
    let manifest_path = local_path_from_validation_source_uri(source_uri)?;
    manifest_path
        .parent()
        .map(|parent| parent.join(resource_path))
}

fn local_path_from_validation_source_uri(source_uri: &str) -> Option<PathBuf> {
    match parse_local_file_uri(source_uri) {
        Some(Ok(path)) => Some(path),
        Some(Err(_)) => None,
        None if !has_uri_scheme(source_uri) || is_windows_drive_path(source_uri) => {
            Some(Path::new(source_uri).to_path_buf())
        }
        None => None,
    }
}

fn schema_package_artifact_contract_mismatch_diag(
    doc: &crate::parser::document::CemDocument,
    node: &CemAstNode,
    path: &str,
    mismatch: TransformTemplateArtifactFunctionContractMismatch,
) -> Diagnostic {
    let field_name = artifact_contract_mismatch_field_name(mismatch.field);
    let invalid_fields = field_name
        .map(|field| vec![field.to_owned()])
        .unwrap_or_default();
    let expected_values = field_name
        .map(|field| BTreeMap::from([(field.to_owned(), mismatch.expected.clone())]))
        .unwrap_or_default();
    let invalid_values = field_name
        .map(|field| BTreeMap::from([(field.to_owned(), mismatch.actual.clone())]))
        .unwrap_or_default();

    diag_at_with_details(
        "cem.schema_package.artifact_check",
        Severity::Error,
        format!(
            "artifact `{path}` failed schema-owned artifact output stage contract: {} metadata expected `{}`, CEMT declares `{}`",
            mismatch.field, mismatch.expected, mismatch.actual
        ),
        node,
        serde_json::json!({
            "schemaUri": CEM_SCHEMA_PACKAGE_URI,
            "element": "artifact",
            "contract": "artifact-output-stage-contract",
            "target": "artifact",
            "diagnostic": "cem.schema_package.artifact_check",
            "checkKind": "artifact-function-contract",
            "path": path,
            "field": mismatch.field,
            "invalidFields": invalid_fields,
            "expectedValues": expected_values,
            "invalidValues": invalid_values,
            "actualValues": element_attribute_values(doc, node),
            "sourceRange": node_source_range_details(node),
        }),
    )
}

fn artifact_contract_mismatch_field_name(field: &str) -> Option<&'static str> {
    match field {
        "function kind" => Some("kind"),
        "target content type" => Some("target-content-type"),
        "target schema" => Some("target-schema"),
        "target category" => Some("target-category"),
        "function profile" => Some("function-profile"),
        _ => None,
    }
}

fn validate_schema_package_example(
    doc: &crate::parser::document::CemDocument,
    node: &CemAstNode,
    registry: &SchemaRegistry,
    out: &mut Vec<Diagnostic>,
) {
    let example_id = attr_value(doc, node, "id")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("<missing>");

    let Some(content_type) = attr_value(doc, node, "content-type")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    let Some(schema_uri) = attr_value(doc, node, "schema")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    let Some(schema) = registry.schema(schema_uri) else {
        return;
    };
    let essence = content_type_essence(content_type);
    if !schema
        .content_type_essences()
        .any(|allowed| allowed == essence)
    {
        out.push(diag_at(
            "cem.schema_package.example_content_type_mismatch",
            Severity::Error,
            format!(
                "schema-package example `{example_id}` content type `{content_type}` is not declared by schema `{schema_uri}`"
            ),
            node,
        ));
    }
}

fn is_schema_package_manifest_document(ctx: &RuleContext<'_>) -> bool {
    ctx.schema_uri == Some(CEM_SCHEMA_PACKAGE_URI)
        || ctx.content_type.is_some_and(|content_type| {
            content_type_essence(content_type) == CEM_SCHEMA_PACKAGE_CONTENT_TYPE
        })
}

fn schema_package_converter_contract_descriptor() -> &'static RuleDescriptor {
    use std::sync::OnceLock;
    static D: OnceLock<RuleDescriptor> = OnceLock::new();
    D.get_or_init(|| RuleDescriptor {
        id: RuleId::new("cem.schema_package.converter_contract"),
        owning_scope: "cem-core",
        content_type: Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
        trigger_layer: TriggerLayer::Document,
        required_inputs: &[RuleInput::CemDocument],
        default_severity: Severity::Error,
        policy_overridable: false,
    })
}

// ---------- Open Content / Unknown Names ----------

pub struct OpenContentPolicyRule;

impl SemanticRule for OpenContentPolicyRule {
    fn descriptor(&self) -> &RuleDescriptor {
        open_content_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        if is_schema_language_document(ctx) {
            return Vec::new();
        }

        let mut out = Vec::new();
        for node in ctx.document.iter() {
            match node {
                CemAstNode::Element { expanded_name, .. } => {
                    let ns = expanded_name.namespace_uri.as_str();
                    let local = expanded_name.local_name.as_str();
                    if local.is_empty() || local == "$" || local.starts_with('@') {
                        continue;
                    }
                    if is_schema_language_namespace(ns) {
                        continue;
                    }
                    if ns == "cem" {
                        if !KNOWN_CEM_ELEMENTS.contains(&local) {
                            out.push(diag_at(
                                "cem.schema.unknown_cem_element",
                                Severity::Error,
                                format!("CEM schema element `{local}` is not declared by the active schema"),
                                node,
                            ));
                        }
                    } else if !is_custom_element_name(local)
                        && !KNOWN_HTML_SVG_ELEMENTS.contains(&local)
                    {
                        out.push(diag_at(
                            "cem.schema.unknown_html_element",
                            Severity::Error,
                            format!("element `{local}` is not accepted by the Tier A HTML/SVG open-content policy"),
                            node,
                        ));
                    }
                }
                CemAstNode::Attribute { expanded_name, .. } => {
                    let ns = expanded_name.namespace_uri.as_str();
                    let local = expanded_name.local_name.as_str();
                    if is_schema_language_namespace(ns) {
                        continue;
                    }
                    if ns == "cem" {
                        if !KNOWN_CEM_ATTRIBUTES.contains(&local) {
                            out.push(diag_at(
                                "cem.schema.unknown_cem_attribute",
                                Severity::Error,
                                format!("CEM annotation `cem:{local}` is not declared by the active schema"),
                                node,
                            ));
                        }
                    } else if !known_open_attribute(ns, local) {
                        out.push(diag_at(
                            "cem.schema.unknown_html_attribute",
                            Severity::Warning,
                            format!("attribute `{local}` is not declared by the Tier A HTML/SVG open-content policy"),
                            node,
                        ));
                    }
                }
                _ => {}
            }
        }
        out
    }
}

fn is_schema_language_namespace(ns: &str) -> bool {
    matches!(
        ns,
        CEM_ML_SCHEMA_URI
            | CEM_SCHEMA_URI
            | CEM_SCHEMA_PACKAGE_URI
            | CEM_NATIVE_TEMPLATE_SCHEMA_URI
            | CEM_TRANSFORM_SCHEMA_URI
    )
}

fn is_schema_language_document(ctx: &RuleContext<'_>) -> bool {
    ctx.schema_uri
        .map(is_schema_language_namespace)
        .unwrap_or(false)
        || ctx
            .content_type
            .map(is_schema_language_content_type)
            .unwrap_or(false)
}

fn is_template_family_language_document(ctx: &RuleContext<'_>) -> bool {
    matches!(
        ctx.schema_uri,
        Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI | CEM_TRANSFORM_SCHEMA_URI)
    ) || ctx.content_type.is_some_and(|content_type| {
        matches!(
            content_type_essence(content_type).as_str(),
            CEM_NATIVE_TEMPLATE_CONTENT_TYPE | CEM_TRANSFORM_CONTENT_TYPE
        )
    })
}

fn is_schema_language_content_type(content_type: &str) -> bool {
    matches!(
        content_type_essence(content_type).as_str(),
        CEM_ML_CONTENT_TYPE
            | CEM_SCHEMA_CONTENT_TYPE
            | CEM_SCHEMA_PACKAGE_CONTENT_TYPE
            | CEM_NATIVE_TEMPLATE_CONTENT_TYPE
            | CEM_TRANSFORM_CONTENT_TYPE
    )
}

fn open_content_descriptor() -> &'static RuleDescriptor {
    use std::sync::OnceLock;
    static D: OnceLock<RuleDescriptor> = OnceLock::new();
    D.get_or_init(|| RuleDescriptor {
        id: RuleId::new("cem.schema.open_content_policy"),
        owning_scope: "cem-core",
        content_type: None,
        trigger_layer: TriggerLayer::Document,
        required_inputs: &[RuleInput::CemDocument, RuleInput::Policy],
        default_severity: Severity::Warning,
        policy_overridable: true,
    })
}

const KNOWN_CEM_ELEMENTS: &[&str] = &[
    "schema",
    "for-each",
    "if",
    "choose",
    "when",
    "otherwise",
    "variable",
];

const KNOWN_CEM_ATTRIBUTES: &[&str] = &[
    "screen",
    "form",
    "action",
    "badge",
    "card",
    "list",
    "row",
    "thread",
    "message",
    "state",
    "name",
    "schema",
    "schema-src",
    "schema-select",
    "for-each",
    "if",
    "choose",
    "when",
    "otherwise",
    "variable",
];

const KNOWN_HTML_SVG_ELEMENTS: &[&str] = &[
    "a", "article", "aside", "body", "button", "dd", "desc", "dialog", "div", "dl", "dt",
    "fieldset", "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "head", "header", "html",
    "iframe", "img", "input", "label", "legend", "li", "main", "mark", "meta", "nav", "ol",
    "option", "p", "path", "script", "section", "select", "small", "span", "strong", "svg",
    "textarea", "title", "ul",
];

const KNOWN_HTML_SVG_ATTRIBUTES: &[&str] = &[
    "action",
    "alt",
    "aria-hidden",
    "aria-label",
    "aria-labelledby",
    "aria-describedby",
    "aria-controls",
    "aria-owns",
    "autocomplete",
    "charset",
    "checked",
    "class",
    "d",
    "disabled",
    "for",
    "height",
    "href",
    "id",
    "lang",
    "method",
    "name",
    "required",
    "role",
    "rows",
    "src",
    "srcdoc",
    "title",
    "type",
    "value",
    "viewBox",
    "width",
    "xmlns",
];

fn known_open_attribute(ns: &str, local: &str) -> bool {
    ns == "xmlns"
        || ns == "xlink"
        || local.starts_with("data-")
        || local.starts_with("aria-")
        || KNOWN_HTML_SVG_ATTRIBUTES.contains(&local)
}

fn is_custom_element_name(local: &str) -> bool {
    local.contains('-')
}

// ---------- Relaxed Content Boundary ----------

/// `cem.lint.relaxed_content_boundary` — recommend the explicit `|` /
/// `▷` content-boundary marker on every element that carries content.
///
/// `cem-ml-syntax.md` §"Content Runs" allows the relaxed form (content
/// begins at the first non-attribute token), but the canonical surface
/// keeps `|` for clarity. This rule runs at the document layer and
/// inspects the AST flag set by the parser (no reliance on tokenizer
/// proxies like `cem.tokenizer.unterminated_node` or
/// `cem.tokenizer.bare_brace_text`).
pub struct RelaxedBoundaryRule;

impl SemanticRule for RelaxedBoundaryRule {
    fn descriptor(&self) -> &RuleDescriptor {
        relaxed_boundary_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for node in ctx.document.iter() {
            let CemAstNode::Element {
                expanded_name,
                children,
                has_explicit_boundary,
                ..
            } = node
            else {
                continue;
            };
            if *has_explicit_boundary {
                continue;
            }
            // Directives lower to `Element` with a leading `@`; expression
            // nodes use `$`. Neither participates in the `|` content-
            // boundary rule.
            let local = expanded_name.local_name.as_str();
            if local.starts_with('@') || local == "$" {
                continue;
            }
            if !has_significant_content(ctx.document, children) {
                continue;
            }
            out.push(diag_at(
                "cem.lint.relaxed_content_boundary",
                Severity::Warning,
                format!(
                    "element `{}` uses the relaxed content boundary; insert `|` (or `▷`) before the content for canonical CEM-ML",
                    qualified_name(expanded_name),
                ),
                node,
            ));
        }
        out
    }
}

fn qualified_name(name: &crate::parser::ExpandedName) -> String {
    if name.namespace_uri.is_empty() {
        name.local_name.clone()
    } else {
        format!("{}:{}", name.namespace_uri, name.local_name)
    }
}

fn has_significant_content(
    doc: &crate::parser::document::CemDocument,
    children: &[AstNodeId],
) -> bool {
    children.iter().any(|id| {
        matches!(
            doc.get(*id),
            Some(CemAstNode::Element { .. })
                | Some(CemAstNode::Text { .. })
                | Some(CemAstNode::Cdata { .. })
                | Some(CemAstNode::RawText { .. })
                | Some(CemAstNode::ProcessingInstruction { .. })
        )
    })
}

fn relaxed_boundary_descriptor() -> &'static RuleDescriptor {
    use std::sync::OnceLock;
    static D: OnceLock<RuleDescriptor> = OnceLock::new();
    D.get_or_init(|| RuleDescriptor {
        id: RuleId::new("cem.lint.relaxed_content_boundary"),
        owning_scope: "cem-lint",
        content_type: None,
        trigger_layer: TriggerLayer::Document,
        required_inputs: &[RuleInput::CemDocument],
        default_severity: Severity::Warning,
        policy_overridable: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{run, RuleRegistry};

    fn parse(input: &str) -> crate::parser::document::CemDocument {
        use crate::events::cem::CemEventNormalizer;
        use crate::parser::builder::CemAstBuilder;
        use crate::source::{BytesSource, SourceId};
        use crate::tokenizer::cem::CemTokenizer;
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        CemAstBuilder::new(normalizer).build()
    }

    fn run_rules(input: &str) -> Vec<Diagnostic> {
        run_rules_with_identity(input, None, None)
    }

    fn run_rules_with_identity(
        input: &str,
        schema_uri: Option<&str>,
        content_type: Option<&str>,
    ) -> Vec<Diagnostic> {
        run_rules_with_identity_and_source_uri(input, schema_uri, content_type, None)
    }

    fn run_rules_with_identity_and_source_uri(
        input: &str,
        schema_uri: Option<&str>,
        content_type: Option<&str>,
        source_uri: Option<&str>,
    ) -> Vec<Diagnostic> {
        let doc = parse(input);
        let upstream: Vec<Diagnostic> = doc.diagnostics.clone();
        let registry = RuleRegistry::with_tier_a_rules();
        registry.run(&RuleContext {
            document: &doc,
            schema_uri,
            content_type,
            source_uri,
            resource_reader: None,
            upstream_diagnostics: &upstream,
        })
    }

    #[test]
    fn reference_integrity_flags_unresolved_for_attribute() {
        let diags = run_rules(r#"{label @for=missing | Missing}"#);
        assert!(diags
            .iter()
            .any(|d| d.code == "cem.ref.unresolved_reference"));
    }

    #[test]
    fn reference_integrity_clean_when_target_present() {
        let diags = run_rules(r#"{form | {label @for=email | E} {input @id=email}}"#);
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.ref.unresolved_reference"));
    }

    #[test]
    fn reference_integrity_splits_aria_idrefs() {
        let diags =
            run_rules(r#"{main @aria-labelledby="title missing" | {h1 @id=title | Title}}"#);
        assert!(diags
            .iter()
            .any(|d| d.code == "cem.ref.unresolved_reference" && d.message.contains("missing")));
    }

    #[test]
    fn accessible_name_flags_button_without_label_or_text() {
        let diags = run_rules("{button @type=submit}");
        assert!(diags
            .iter()
            .any(|d| d.code == "cem.a11y.accessible_name_missing"));
    }

    #[test]
    fn accessible_name_clean_when_text_content_present() {
        let diags = run_rules("{button | Save}");
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.a11y.accessible_name_missing"));
    }

    #[test]
    fn accessible_name_clean_when_aria_label_present() {
        let diags = run_rules(r#"{button @aria-label="Save"}"#);
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.a11y.accessible_name_missing"));
    }

    #[test]
    fn input_accessible_name_resolves_label_for() {
        let diags = run_rules(r#"{form | {label @for=email | Email} {input @id=email}}"#);
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.a11y.accessible_name_missing"));
    }

    #[test]
    fn input_accessible_name_resolves_wrapping_label() {
        let diags = run_rules(r#"{label | {input @type=checkbox} Email updates}"#);
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.a11y.accessible_name_missing"));
    }

    #[test]
    fn aria_role_attribute_compatibility_flags_mismatch() {
        let diags = run_rules(r#"{div @aria-checked=true | Toggle}"#);
        assert!(diags.iter().any(|d| d.code == "cem.a11y.aria_incompatible"));
    }

    #[test]
    fn aria_role_attribute_compatibility_accepts_matching_role() {
        let diags = run_rules(r#"{div @role=checkbox @aria-checked=true | Toggle}"#);
        assert!(diags.iter().all(|d| d.code != "cem.a11y.aria_incompatible"));
    }

    #[test]
    fn svg_requires_name_when_visible() {
        let diags = run_rules(r#"{svg | {path @d="M0 0h1"}}"#);
        assert!(diags
            .iter()
            .any(|d| d.code == "cem.a11y.svg_accessible_name_missing"));
    }

    #[test]
    fn svg_hidden_or_titled_is_clean() {
        let hidden = run_rules(r#"{svg @aria-hidden=true | {path @d="M0 0h1"}}"#);
        assert!(hidden
            .iter()
            .all(|d| d.code != "cem.a11y.svg_accessible_name_missing"));
        let titled = run_rules(r#"{svg | {title | Download} {path @d="M0 0h1"}}"#);
        assert!(titled
            .iter()
            .all(|d| d.code != "cem.a11y.svg_accessible_name_missing"));
    }

    #[test]
    fn state_combination_flags_disabled_plus_loading() {
        let diags =
            run_rules(r#"{button @cem:action=primary @cem:state="disabled loading" | Save}"#);
        assert!(diags
            .iter()
            .any(|d| d.code == "cem.state.invalid_combination"));
    }

    #[test]
    fn state_combination_clean_when_single_state() {
        let diags = run_rules(r#"{button @cem:action=primary @cem:state="disabled" | Save}"#);
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.state.invalid_combination"));
    }

    #[test]
    fn state_default_cannot_combine_with_transient_state() {
        let diags = run_rules(r#"{button @cem:state="default active" | Save}"#);
        assert!(diags
            .iter()
            .any(|d| d.code == "cem.state.invalid_combination"));
    }

    #[test]
    fn state_transition_flags_form_state_on_non_form_host() {
        let diags = run_rules(r#"{span @cem:state=invalid | Bad}"#);
        assert!(diags
            .iter()
            .any(|d| d.code == "cem.state.invalid_transition"));
    }

    #[test]
    fn javascript_url_flagged() {
        let diags = run_rules(r#"{a @href="javascript:void(0)" | Click}"#);
        assert!(diags.iter().any(|d| d.code == "cem.unsafe.javascript_url"));
    }

    #[test]
    fn javascript_url_case_insensitive_match() {
        let diags = run_rules(r#"{a @href="  JavaScript:alert(1)" | Click}"#);
        assert!(diags.iter().any(|d| d.code == "cem.unsafe.javascript_url"));
    }

    #[test]
    fn safe_url_passes() {
        let diags = run_rules(r#"{a @href="/dashboard" | Dashboard}"#);
        assert!(diags.iter().all(|d| d.code != "cem.unsafe.javascript_url"));
    }

    #[test]
    fn event_handler_attribute_flagged() {
        let diags = run_rules(r#"{button @onclick="boom()" | Boom}"#);
        assert!(diags
            .iter()
            .any(|d| d.code == "cem.unsafe.event_handler_attribute"));
    }

    #[test]
    fn unsafe_inline_script_and_srcdoc_are_flagged() {
        let script = run_rules(r#"{script | ```alert(1)```}"#);
        assert!(script.iter().any(|d| d.code == "cem.unsafe.inline_script"));
        let srcdoc = run_rules(r#"{iframe @srcdoc="<p>x</p>"}"#);
        assert!(srcdoc.iter().any(|d| d.code == "cem.unsafe.srcdoc"));
    }

    #[test]
    fn ordinary_attributes_starting_with_on_are_not_misflagged() {
        // `once` and `online` shouldn't trigger the rule (length > 2 OK,
        // but the third char must be alphabetic which it is; the rule is
        // intentionally conservative — it flags any `on*` attribute as
        // a policy violation). Accept that false positive in Tier A and
        // document it in the rule comment.
        // This test asserts that the *built-in* `on` (length 2) doesn't
        // panic; in Tier A we accept the broader flag-rule.
        let diags = run_rules(r#"{input @on="weird"}"#);
        // `@on` has length 2 and our rule requires > 2, so no diag fires.
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.unsafe.event_handler_attribute"));
    }

    #[test]
    fn validation_run_end_to_end_clean_on_canonical_fixture() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/cem-ml/login.cem");
        let input = std::fs::read_to_string(path).unwrap();
        let report = run(&input);
        assert_eq!(
            report.hard_violations(),
            0,
            "login fixture should validate clean: {:?}",
            report
                .diagnostics
                .iter()
                .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn unbound_prefix_flagged_on_unknown_namespace() {
        let diags = run_rules(r#"{main @bogus:role="x" | hi}"#);
        assert!(diags.iter().any(|d| d.code == "cem.lint.unbound_prefix"));
    }

    #[test]
    fn known_namespace_prefixes_not_flagged() {
        let diags = run_rules(r#"{button @cem:action=primary @aria-label="Save"}"#);
        assert!(diags.iter().all(|d| d.code != "cem.lint.unbound_prefix"));
    }

    #[test]
    fn native_template_with_attributes_are_not_unbound_prefix_lints() {
        let diags = run_rules_with_identity(
            r#"{module | {template @name=page | {body | {call @template=hero @with:title=heading}}}}"#,
            Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI),
            Some(CEM_NATIVE_TEMPLATE_CONTENT_TYPE),
        );
        assert!(diags.iter().all(|d| d.code != "cem.lint.unbound_prefix"));
    }

    #[test]
    fn transform_with_attributes_are_not_unbound_prefix_lints() {
        let diags = run_rules_with_identity(
            r#"{module | {template @name=page | {body | {call @template=hero @with:title=heading}}}}"#,
            Some(CEM_TRANSFORM_SCHEMA_URI),
            Some(CEM_TRANSFORM_CONTENT_TYPE),
        );
        assert!(diags.iter().all(|d| d.code != "cem.lint.unbound_prefix"));
    }

    #[test]
    fn open_content_policy_flags_unknown_names() {
        let element_diags = run_rules(r#"{nothtml | hi}"#);
        assert!(element_diags
            .iter()
            .any(|d| d.code == "cem.schema.unknown_html_element"));
        let attr_diags = run_rules(r#"{button @madeup=value | Save}"#);
        assert!(attr_diags
            .iter()
            .any(|d| d.code == "cem.schema.unknown_html_attribute"));
    }

    #[test]
    fn open_content_policy_skips_schema_language_documents() {
        let diags = run_rules_with_identity(
            r#"{schema @name=note | {elements | {element @name=note}}}"#,
            Some(CEM_SCHEMA_URI),
            Some(CEM_SCHEMA_CONTENT_TYPE),
        );
        assert!(diags.iter().all(|d| {
            d.code != "cem.schema.unknown_html_element"
                && d.code != "cem.schema.unknown_html_attribute"
        }));
    }

    #[test]
    fn open_content_policy_skips_native_template_language_documents() {
        let diags = run_rules_with_identity(
            r#"{module | {template @name=page | {body | {call @template=hero}}}}"#,
            Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI),
            Some(CEM_NATIVE_TEMPLATE_CONTENT_TYPE),
        );
        assert!(diags.iter().all(|d| {
            d.code != "cem.schema.unknown_html_element"
                && d.code != "cem.schema.unknown_html_attribute"
                && d.code != "cem.schema.unresolved_namespace"
        }));
    }

    #[test]
    fn open_content_policy_skips_transform_language_documents() {
        let diags = run_rules_with_identity(
            r#"{module | {template @name=main | {body | {call @template=row}}}}"#,
            Some(CEM_TRANSFORM_SCHEMA_URI),
            Some(CEM_TRANSFORM_CONTENT_TYPE),
        );
        assert!(diags.iter().all(|d| {
            d.code != "cem.schema.unknown_html_element"
                && d.code != "cem.schema.unknown_html_attribute"
                && d.code != "cem.schema.unresolved_namespace"
        }));
    }

    #[test]
    fn schema_document_model_rule_flags_missing_required_attribute() {
        let diags = run_rules_with_identity(
            r#"{schema @name=note @version="1.0.0"}"#,
            Some(CEM_SCHEMA_URI),
            Some(CEM_SCHEMA_CONTENT_TYPE),
        );
        assert!(diags
            .iter()
            .any(|d| d.code == "cem.schema_model.missing_required_attribute"));
    }

    #[test]
    fn schema_package_converter_contract_accepts_valid_cemt_converter() {
        let diags = run_rules_with_identity(
            r#"{package @id=demo @version="1.0.0" |
                {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
                {content-type @value="application/vnd.example.demo+cem" @primary=true}
                {converter
                    @id="demo-to-html"
                    @implementation="cemt"
                    @template="converters/demo-to-html.cemt"
                    @template-content-type="application/vnd.cem.transform+cem"
                    @template-schema="https://cem.dev/ns/transform/cem/1"
                    @streamable=true
                    @output-syntax="html"
                    @encoding-category="html-document"
                    @formatter-profile="canonical"
                    @color-profile="classes"
                    @parity="parse-equivalent"
                    @implicit=false
                    @cost=25 |
                    {from @content-type="application/vnd.example.demo+cem" @schema="https://example.test/ns/demo/1"}
                    {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
                }
            }"#,
            Some(CEM_SCHEMA_PACKAGE_URI),
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
        );

        assert!(
            diags
                .iter()
                .all(|d| !d.code.starts_with("cem.schema_package.converter_")),
            "unexpected converter diagnostics: {diags:?}"
        );
    }

    #[test]
    fn schema_package_converter_contract_flags_invalid_cemt_metadata() {
        let diags = run_rules_with_identity(
            r#"{package @id=demo @version="1.0.0" |
                {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
                {content-type @value="application/vnd.example.demo+cem" @primary=true}
                {converter
                    @id="bad-cemt"
                    @implementation="cemt"
                    @template-content-type="text/cem-ml"
                    @template-schema="https://cem.dev/ns/schema/1"
                    @rust-symbol="DemoHtmlFallback"
                    @streamable=maybe
                    @lossiness="hand-wave"
                    @output-syntax="pdf"
                    @parity="mostly-equal"
                    @readiness=later
                    @explicit-only=true
                    @implicit=true
                    @cost=0 |
                    {from @content-type="text/html" @schema="https://cem.dev/ns/data/xml/1"}
                    {from @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
                }
            }"#,
            Some(CEM_SCHEMA_PACKAGE_URI),
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
        );

        for code in [
            "cem.schema_package.converter_check",
            "cem.schema_model.invalid_attribute_type",
            "cem.schema_model.invalid_attribute_value",
            "cem.schema_model.invalid_attribute_datatype_param",
        ] {
            assert!(
                diags.iter().any(|d| d.code == code),
                "missing {code}; diagnostics: {diags:?}"
            );
        }

        for check_kind in ["endpoint-content-type-schema"] {
            assert!(
                diags.iter().any(|d| {
                    d.code == "cem.schema_package.converter_check"
                        && d.details.as_ref().and_then(|details| {
                            details.get("checkKind").and_then(serde_json::Value::as_str)
                        }) == Some(check_kind)
                }),
                "missing converter_check diagnostic with checkKind `{check_kind}`: {diags:?}"
            );
        }

        for removed_code in [
            "cem.schema_package.converter_boolean_invalid",
            "cem.schema_package.converter_cost_invalid",
            "cem.schema_package.converter_template_missing",
            "cem.schema_package.converter_template_content_type_missing",
            "cem.schema_package.converter_template_content_type_mismatch",
            "cem.schema_package.converter_template_schema_missing",
            "cem.schema_package.converter_template_schema_mismatch",
            "cem.schema_package.converter_template_source_unreadable",
            "cem.schema_package.converter_template_contract_invalid",
            "cem.schema_package.converter_rust_symbol_missing",
            "cem.schema_package.converter_fallback_reason_missing",
            "cem.schema_package.converter_content_type_mismatch",
            "cem.schema_package.converter_readiness_unknown",
            "cem.schema_package.converter_lossiness_unknown",
            "cem.schema_package.converter_output_syntax_unknown",
            "cem.schema_package.converter_parity_unknown",
            "cem.schema_package.converter_selection_conflict",
            "cem.schema_package.converter_endpoint_duplicate",
            "cem.schema_package.converter_endpoint_missing",
        ] {
            assert!(
                diags.iter().all(|d| d.code != removed_code),
                "legacy converter diagnostic `{removed_code}` should be covered by generic schema-model validation: {diags:?}"
            );
        }

        let cemt_identity = diags
            .iter()
            .find(|d| {
                d.code == "cem.schema_package.converter_check"
                    && d.details.as_ref().and_then(|details| {
                        details.get("contract").and_then(serde_json::Value::as_str)
                    }) == Some("converter-cemt-template-identity")
                    && d.details.as_ref().and_then(|details| {
                        details.get("checkKind").and_then(serde_json::Value::as_str)
                    }) == Some("required-fields")
            })
            .expect("schema-owned CEMT template identity field contract diagnostic");
        let details = cemt_identity
            .details
            .as_ref()
            .expect("CEMT template identity details");
        assert_eq!(details["missingFields"], serde_json::json!(["template"]));
        assert_eq!(
            details["condition"],
            serde_json::json!({
                "attribute": "implementation",
                "values": ["cemt"],
                "presentAttributes": [],
            })
        );

        let fallback_reason = diags
            .iter()
            .find(|d| {
                d.code == "cem.schema_package.converter_check"
                    && d.details.as_ref().and_then(|details| {
                        details.get("contract").and_then(serde_json::Value::as_str)
                    }) == Some("converter-cemt-fallback-reason")
            })
            .expect("schema-owned CEMT fallback reason field contract diagnostic");
        let details = fallback_reason
            .details
            .as_ref()
            .expect("CEMT fallback reason details");
        assert_eq!(
            details["missingFields"],
            serde_json::json!(["fallback-reason"])
        );
        assert_eq!(
            details["condition"],
            serde_json::json!({
                "attribute": "implementation",
                "values": ["cemt"],
                "presentAttributes": ["rust-symbol"],
            })
        );

        let endpoint_occurrence = diags
            .iter()
            .find(|d| {
                d.code == "cem.schema_package.converter_check"
                    && d.details.as_ref().and_then(|details| {
                        details.get("contract").and_then(serde_json::Value::as_str)
                    }) == Some("converter-from-to-endpoints")
            })
            .expect("schema-owned converter endpoint occurrence field contract diagnostic");
        let details = endpoint_occurrence
            .details
            .as_ref()
            .expect("converter endpoint occurrence details");
        assert_eq!(details["missingChildren"], serde_json::json!(["to"]));
        assert_eq!(details["duplicateChildren"], serde_json::json!(["from"]));
        assert_eq!(
            details["requiredChildren"],
            serde_json::json!(["from", "to"])
        );
        assert_eq!(details["maxOneChildren"], serde_json::json!(["from", "to"]));
        assert_eq!(
            details["childCounts"],
            serde_json::json!({
                "from": 2,
            })
        );

        let planner_state = diags
            .iter()
            .find(|d| {
                d.code == "cem.schema_package.converter_check"
                    && d.details.as_ref().and_then(|details| {
                        details.get("contract").and_then(serde_json::Value::as_str)
                    }) == Some("converter-planner-state")
            })
            .expect("schema-owned converter planner state field contract diagnostic");
        let details = planner_state
            .details
            .as_ref()
            .expect("converter planner state details");
        assert_eq!(details["missingFields"], serde_json::json!([]));
        assert_eq!(details["invalidFields"], serde_json::json!(["implicit"]));
        assert_eq!(
            details["forbiddenAttributeValues"],
            serde_json::json!({
                "implicit": ["true"],
            })
        );
        assert_eq!(
            details["invalidValues"],
            serde_json::json!({
                "implicit": "true",
            })
        );
        assert_eq!(
            details["condition"],
            serde_json::json!({
                "attribute": "explicit-only",
                "values": ["true"],
                "presentAttributes": [],
            })
        );

        for attribute_name in [
            "template-content-type",
            "template-schema",
            "readiness",
            "lossiness",
            "output-syntax",
            "parity",
        ] {
            assert!(
                diags.iter().any(|d| {
                    d.code == "cem.schema_model.invalid_attribute_value"
                        && d.details.as_ref().and_then(|details| {
                            details.get("attribute").and_then(serde_json::Value::as_str)
                        }) == Some(attribute_name)
                }),
                "missing generic value-vocabulary diagnostic for `{attribute_name}`: {diags:?}"
            );
        }
    }

    #[test]
    fn schema_package_example_contract_accepts_valid_examples() {
        let diags = run_rules_with_identity(
            r#"{package @id=demo @version="1.0.0" |
                {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
                {content-type @value="application/vnd.example.demo+cem" @primary=true}
                {example
                    @id="basic"
                    @path="examples/basic.html"
                    @content-type="text/html"
                    @schema="https://cem.dev/ns/data/html/1"
                    @expected-result="pass"
                }
                {example
                    @id="invalid"
                    @path="examples/invalid.html"
                    @content-type="text/html"
                    @schema="https://cem.dev/ns/data/html/1"
                    @expected-result="fail"
                    @expected-diagnostics="cem.html.script_rejected"
                }
            }"#,
            Some(CEM_SCHEMA_PACKAGE_URI),
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
        );

        assert!(
            diags
                .iter()
                .all(|d| !d.code.starts_with("cem.schema_package.example_")),
            "unexpected example diagnostics: {diags:?}"
        );
    }

    #[test]
    fn schema_package_example_contract_flags_invalid_metadata() {
        let diags = run_rules_with_identity(
            r#"{package @id=demo @version="1.0.0" |
                {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
                {content-type @value="application/vnd.example.demo+cem" @primary=true}
                {example
                    @id="wrong-result"
                    @path="examples/wrong-result.html"
                    @content-type="text/html"
                    @schema="https://cem.dev/ns/data/html/1"
                    @expected-result="maybe"
                }
                {example
                    @id="wrong-content-type"
                    @path="examples/wrong-content-type.html"
                    @content-type="text/html"
                    @schema="https://cem.dev/ns/data/xml/1"
                    @expected-result="pass"
                }
                {example
                    @id="missing-diagnostics"
                    @path="examples/missing-diagnostics.html"
                    @content-type="text/html"
                    @schema="https://cem.dev/ns/data/html/1"
                    @expected-result="fail"
                }
                {example
                    @id="missing-required"
                    @expected-result="pass"
                }
            }"#,
            Some(CEM_SCHEMA_PACKAGE_URI),
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
        );

        for code in [
            "cem.schema_model.invalid_attribute_value",
            "cem.schema_package.example_content_type_mismatch",
            "cem.schema_package.example_check",
        ] {
            assert!(
                diags.iter().any(|d| d.code == code),
                "missing {code}; diagnostics: {diags:?}"
            );
        }

        for removed_code in [
            "cem.schema_package.example_result_unknown",
            "cem.schema_package.example_expected_diagnostics_missing",
        ] {
            assert!(
                diags.iter().all(|d| d.code != removed_code),
                "legacy example diagnostic `{removed_code}` should be covered by generic schema-owned validation: {diags:?}"
            );
        }

        let missing_required = diags
            .iter()
            .find(|d| {
                d.code == "cem.schema_package.example_check"
                    && d.details.as_ref().and_then(|details| {
                        details.get("contract").and_then(serde_json::Value::as_str)
                    }) == Some("example-required-metadata")
            })
            .expect("schema-owned example required metadata field contract diagnostic");
        let details = missing_required
            .details
            .as_ref()
            .expect("example required metadata details");
        assert_eq!(
            details["missingFields"],
            serde_json::json!(["content-type", "path", "schema"])
        );
        assert_eq!(details["checkKind"], serde_json::json!("required-fields"));

        let failing_diagnostics = diags
            .iter()
            .find(|d| {
                d.code == "cem.schema_package.example_check"
                    && d.details.as_ref().and_then(|details| {
                        details.get("contract").and_then(serde_json::Value::as_str)
                    }) == Some("example-failing-diagnostics")
            })
            .expect("schema-owned failing example diagnostics field contract diagnostic");
        let details = failing_diagnostics
            .details
            .as_ref()
            .expect("failing example diagnostics details");
        assert_eq!(
            details["missingFields"],
            serde_json::json!(["expected-diagnostics"])
        );
        assert_eq!(
            details["condition"],
            serde_json::json!({
                "attribute": "expected-result",
                "values": ["fail"],
                "presentAttributes": [],
            })
        );
    }

    #[test]
    fn schema_package_converter_contract_flags_unsupported_output_pipeline_template() {
        let dir = schema_package_artifact_contract_fixture_dir(
            "converter-template-contract-invalid",
            &[(
                "converters/demo-to-html.cemt",
                r#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {template @name="main" | {text "not a supported DOM converter"}}
}
"#,
            )],
        );
        let package_uri = dir.join("package.cem").display().to_string();
        let diags = run_rules_with_identity_and_source_uri(
            r#"{package @id=demo @version="1.0.0" |
                {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
                {content-type @value="application/vnd.example.demo+cem" @primary=true}
                {converter
                    @id="demo-to-html"
                    @implementation="cemt"
                    @template="converters/demo-to-html.cemt"
                    @template-content-type="application/vnd.cem.transform+cem"
                    @template-schema="https://cem.dev/ns/transform/cem/1"
                    @output-syntax="html"
                    @encoding-category="html-document"
                    @formatter-profile="canonical"
                    @color-profile="classes" |
                    {from @content-type="application/vnd.example.demo+cem" @schema="https://example.test/ns/demo/1"}
                    {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
                }
            }"#,
            Some(CEM_SCHEMA_PACKAGE_URI),
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
            Some(&package_uri),
        );

        let diagnostic = diags
            .iter()
            .find(|d| {
                d.code == "cem.schema_package.converter_check"
                    && d.details.as_ref().and_then(|details| {
                        details.get("checkKind").and_then(serde_json::Value::as_str)
                    }) == Some("converter-template-contract")
            })
            .expect("converter template contract diagnostic");
        assert!(diagnostic.message.contains("demo-to-html"));
        assert!(diagnostic
            .message
            .contains("formatted CEM tree before the writer"));
        assert!(diagnostic
            .message
            .contains("supported DOM projection converter"));
        let details = diagnostic
            .details
            .as_ref()
            .expect("converter template contract details");
        assert_eq!(
            details["contract"],
            serde_json::json!("converter-template-output-stage-contract")
        );
        assert_eq!(
            details["diagnostic"],
            serde_json::json!("cem.schema_package.converter_check")
        );
        assert_eq!(details["invalidFields"], serde_json::json!(["template"]));
    }

    #[test]
    fn schema_package_converter_contract_flags_unreadable_output_pipeline_template() {
        let dir =
            schema_package_artifact_contract_fixture_dir("converter-template-source-missing", &[]);
        let package_uri = dir.join("package.cem").display().to_string();
        let diags = run_rules_with_identity_and_source_uri(
            r#"{package @id=demo @version="1.0.0" |
                {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
                {content-type @value="application/vnd.example.demo+cem" @primary=true}
                {converter
                    @id="demo-to-html"
                    @implementation="cemt"
                    @template="converters/missing.cemt"
                    @template-content-type="application/vnd.cem.transform+cem"
                    @template-schema="https://cem.dev/ns/transform/cem/1"
                    @output-syntax="html"
                    @encoding-category="html-document"
                    @formatter-profile="canonical" |
                    {from @content-type="application/vnd.example.demo+cem" @schema="https://example.test/ns/demo/1"}
                    {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
                }
            }"#,
            Some(CEM_SCHEMA_PACKAGE_URI),
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
            Some(&package_uri),
        );

        let diagnostic = diags
            .iter()
            .find(|d| {
                d.code == "cem.schema_package.converter_check"
                    && d.details.as_ref().and_then(|details| {
                        details.get("checkKind").and_then(serde_json::Value::as_str)
                    }) == Some("converter-template-source-readable")
            })
            .expect("converter template source-readable diagnostic");
        assert_eq!(
            diagnostic.details.as_ref().expect("source details")["contract"],
            serde_json::json!("converter-template-output-stage-contract")
        );
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.schema_package.converter_template_source_unreadable"));
    }

    #[test]
    fn schema_package_converter_contract_flags_rust_converter_without_symbol() {
        let diags = run_rules_with_identity(
            r#"{package @id=demo @version="1.0.0" |
                {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
                {content-type @value="application/vnd.example.demo+cem" @primary=true}
                {converter @id="bad-rust" @implementation="rust" |
                    {from @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
                    {to @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
                }
            }"#,
            Some(CEM_SCHEMA_PACKAGE_URI),
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
        );

        assert!(diags
            .iter()
            .any(|d| d.code == "cem.schema_package.converter_check"));
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.schema_package.converter_rust_symbol_missing"));
        let diagnostic = diags
            .iter()
            .find(|d| {
                d.code == "cem.schema_package.converter_check"
                    && d.details.as_ref().and_then(|details| {
                        details.get("contract").and_then(serde_json::Value::as_str)
                    }) == Some("converter-rust-symbol")
            })
            .expect("schema-owned Rust symbol field contract diagnostic");
        assert_eq!(
            diagnostic.details.as_ref().expect("Rust symbol details")["missingFields"],
            serde_json::json!(["rust-symbol"])
        );
    }

    #[test]
    fn schema_package_converter_contract_does_not_require_fallback_reason_for_rust_converter() {
        let diags = run_rules_with_identity(
            r#"{package @id=demo @version="1.0.0" |
                {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
                {content-type @value="application/vnd.example.demo+cem" @primary=true}
                {converter
                    @id="rust-converter"
                    @implementation="rust"
                    @rust-symbol="demo_convert" |
                    {from @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
                    {to @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
                }
            }"#,
            Some(CEM_SCHEMA_PACKAGE_URI),
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
        );

        assert!(
            diags.iter().all(|d| {
                d.details
                    .as_ref()
                    .and_then(|details| details.get("contract").and_then(serde_json::Value::as_str))
                    != Some("converter-cemt-fallback-reason")
            }),
            "Rust converter should not trigger CEMT fallback field contract: {diags:?}"
        );
    }

    #[test]
    fn schema_package_converter_contract_allows_explicit_only_with_implicit_false() {
        let diags = run_rules_with_identity(
            r#"{package @id=demo @version="1.0.0" |
                {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
                {content-type @value="application/vnd.example.demo+cem" @primary=true}
                {converter
                    @id="rust-explicit"
                    @implementation="rust"
                    @rust-symbol="demo_convert"
                    @explicit-only=true
                    @implicit=false |
                    {from @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
                    {to @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
                }
            }"#,
            Some(CEM_SCHEMA_PACKAGE_URI),
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
        );

        assert!(
            diags.iter().all(|d| {
                d.details
                    .as_ref()
                    .and_then(|details| details.get("contract").and_then(serde_json::Value::as_str))
                    != Some("converter-planner-state")
            }),
            "implicit=false should not trigger converter planner-state field contract: {diags:?}"
        );
        assert!(
            diags
                .iter()
                .all(|d| d.code != "cem.schema_package.converter_selection_conflict"),
            "legacy converter selection conflict diagnostic should not emit: {diags:?}"
        );
    }

    #[test]
    fn schema_package_artifact_contract_flags_cemt_function_metadata_mismatch() {
        let dir = schema_package_artifact_contract_fixture_dir(
            "artifact-contract-mismatch",
            &[("formatters/demo.cemt", demo_format_cemt_source())],
        );
        let package_uri = dir.join("package.cem").display().to_string();
        let diags = run_rules_with_identity_and_source_uri(
            r#"{package @id=demo @version="1.0.0" |
                {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
                {content-type @value="application/vnd.example.demo+cem" @primary=true}
                {artifact
                    @kind="formatter"
                    @path="formatters/demo.cemt"
                    @content-type="application/vnd.cem.transform+cem"
                    @schema="https://cem.dev/ns/transform/cem/1"
                    @target-content-type="application/cem"
                    @target-schema="https://cem.dev/ns/cem-ml/1"
                    @target-category="wrong-tree"
                    @function-name="demo.format"
                    @formatter-profile="cem.format-tree"
                }
            }"#,
            Some(CEM_SCHEMA_PACKAGE_URI),
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
            Some(&package_uri),
        );

        let diagnostic = diags
            .iter()
            .find(|d| {
                d.code == "cem.schema_package.artifact_check"
                    && d.details.as_ref().and_then(|details| {
                        details.get("checkKind").and_then(serde_json::Value::as_str)
                    }) == Some("artifact-function-contract")
            })
            .expect("schema-owned artifact function metadata mismatch diagnostic");
        assert!(diagnostic
            .message
            .contains("target category metadata expected `wrong-tree`"));
        assert!(diagnostic.message.contains("CEMT declares `cem-tree`"));
        let details = diagnostic
            .details
            .as_ref()
            .expect("artifact function contract details");
        assert_eq!(
            details["contract"],
            serde_json::json!("artifact-output-stage-contract")
        );
        assert_eq!(
            details["diagnostic"],
            serde_json::json!("cem.schema_package.artifact_check")
        );
        assert_eq!(
            details["invalidFields"],
            serde_json::json!(["target-category"])
        );
        assert_eq!(
            details["expectedValues"],
            serde_json::json!({
                "target-category": "wrong-tree",
            })
        );
        assert_eq!(
            details["invalidValues"],
            serde_json::json!({
                "target-category": "cem-tree",
            })
        );
        assert_eq!(
            details["actualValues"]["function-name"],
            serde_json::json!("demo.format")
        );
        assert!(details["sourceRange"]["span"]["start"].is_u64());
    }

    #[test]
    fn schema_package_artifact_contract_flags_formatter_and_colorizer_stage_layouts() {
        let dir = schema_package_artifact_contract_fixture_dir(
            "artifact-layout-invalid",
            &[
                ("transforms/demo-format.cemt", demo_format_cemt_source()),
                ("formatters/demo-color.cemt", demo_color_cemt_source()),
            ],
        );
        let package_uri = dir.join("package.cem").display().to_string();
        let diags = run_rules_with_identity_and_source_uri(
            r#"{package @id=demo @version="1.0.0" |
                {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
                {content-type @value="application/vnd.example.demo+cem" @primary=true}
                {artifact
                    @kind="formatter"
                    @path="transforms/demo-format.cemt"
                    @content-type="application/vnd.cem.transform+cem"
                    @schema="https://cem.dev/ns/transform/cem/1"
                    @target-content-type="application/cem"
                    @target-schema="https://cem.dev/ns/cem-ml/1"
                    @target-category="cem-tree"
                    @function-name="demo.format"
                    @formatter-profile="cem.format-tree"
                }
                {artifact
                    @kind="formatter-helper"
                    @path="transforms/demo-format.cemt"
                    @content-type="application/vnd.cem.transform+cem"
                    @schema="https://cem.dev/ns/transform/cem/1"
                    @target-content-type="application/cem"
                    @target-schema="https://cem.dev/ns/cem-ml/1"
                    @target-category="cem-tree"
                    @function-name="demo.format"
                    @formatter-profile="cem.format-tree"
                }
                {artifact
                    @kind="colorizer"
                    @path="formatters/demo-color.cemt"
                    @content-type="application/vnd.cem.transform+cem"
                    @schema="https://cem.dev/ns/transform/cem/1"
                    @target-content-type="application/cem"
                    @target-schema="https://cem.dev/ns/cem-ml/1"
                    @target-category="cem-tree"
                    @function-name="demo.color"
                    @function-profile="classes"
                    @color-profile="classes"
                }
                {artifact
                    @kind="colorizer-helper"
                    @path="formatters/demo-color.cemt"
                    @content-type="application/vnd.cem.transform+cem"
                    @schema="https://cem.dev/ns/transform/cem/1"
                    @target-content-type="application/cem"
                    @target-schema="https://cem.dev/ns/cem-ml/1"
                    @target-category="cem-tree"
                    @function-name="demo.color"
                    @function-profile="classes"
                    @color-profile="classes"
                }
            }"#,
            Some(CEM_SCHEMA_PACKAGE_URI),
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
            Some(&package_uri),
        );

        let layout_diags: Vec<_> = diags
            .iter()
            .filter(|d| {
                d.code == "cem.schema_package.artifact_check"
                    && d.details.as_ref().and_then(|details| {
                        details.get("checkKind").and_then(serde_json::Value::as_str)
                    }) == Some("path-layout")
            })
            .collect();
        assert_eq!(
            layout_diags.len(),
            4,
            "expected formatter/colorizer entrypoint and helper layout diagnostics: {diags:?}"
        );
        assert!(layout_diags.iter().all(|d| d.details.is_some()));
        assert!(layout_diags.iter().any(|d| {
            d.details
                .as_ref()
                .and_then(|details| details.get("contract").and_then(serde_json::Value::as_str))
                == Some("artifact-formatter-layout")
                && d.details.as_ref().and_then(|details| {
                    details
                        .get("invalidValues")
                        .and_then(|invalid| invalid.get("path"))
                        .and_then(serde_json::Value::as_str)
                }) == Some("transforms/demo-format.cemt")
        }));
        assert!(layout_diags.iter().any(|d| {
            d.details
                .as_ref()
                .and_then(|details| details.get("contract").and_then(serde_json::Value::as_str))
                == Some("artifact-colorizer-layout")
                && d.details.as_ref().and_then(|details| {
                    details
                        .get("invalidValues")
                        .and_then(|invalid| invalid.get("path"))
                        .and_then(serde_json::Value::as_str)
                }) == Some("formatters/demo-color.cemt")
        }));
        let formatter_layout = layout_diags
            .iter()
            .find(|d| {
                d.details
                    .as_ref()
                    .and_then(|details| details.get("contract").and_then(serde_json::Value::as_str))
                    == Some("artifact-formatter-layout")
            })
            .expect("formatter path-layout diagnostic");
        assert_eq!(
            formatter_layout
                .details
                .as_ref()
                .expect("formatter layout details")["pathLayout"],
            serde_json::json!({
                "attributes": ["path"],
                "prefix": "formatters",
                "extension": "cemt",
                "relative": true,
                "cleanSegments": true,
            })
        );
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.schema_package.artifact_layout_invalid"));
    }

    #[test]
    fn schema_package_artifact_contract_flags_missing_stage_metadata_from_schema_contract() {
        let dir = schema_package_artifact_contract_fixture_dir(
            "artifact-metadata-missing",
            &[("formatters/demo.cemt", demo_format_cemt_source())],
        );
        let package_uri = dir.join("package.cem").display().to_string();
        let diags = run_rules_with_identity_and_source_uri(
            r#"{package @id=demo @version="1.0.0" |
                {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
                {content-type @value="application/vnd.example.demo+cem" @primary=true}
                {artifact
                    @kind="formatter"
                    @path="formatters/demo.cemt"
                    @content-type="application/vnd.cem.transform+cem"
                    @target-content-type="application/cem"
                    @target-category="cem-tree"
                }
            }"#,
            Some(CEM_SCHEMA_PACKAGE_URI),
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
            Some(&package_uri),
        );

        let diagnostic = diags
            .iter()
            .find(|d| d.code == "cem.schema_package.artifact_check")
            .expect("artifact check diagnostic");
        assert!(diagnostic.message.contains("artifact-stage-metadata"));
        assert!(diagnostic.message.contains("schema"));
        assert!(diagnostic.message.contains("target-schema"));
        assert!(diagnostic.message.contains("function-name"));
        let details = diagnostic.details.as_ref().expect("artifact check details");
        assert_eq!(
            details["schemaUri"],
            serde_json::json!(CEM_SCHEMA_PACKAGE_URI)
        );
        assert_eq!(details["element"], serde_json::json!("artifact"));
        assert_eq!(
            details["contract"],
            serde_json::json!("artifact-stage-metadata")
        );
        assert_eq!(details["checkKind"], serde_json::json!("required-fields"));
        assert_eq!(
            details["requiredFields"],
            serde_json::json!([
                "content-type",
                "function-name",
                "schema",
                "target-category",
                "target-content-type",
                "target-schema"
            ])
        );
        assert_eq!(
            details["missingFields"],
            serde_json::json!(["function-name", "schema", "target-schema"])
        );
        assert_eq!(
            details["actualValues"]["kind"],
            serde_json::json!("formatter")
        );
        assert!(details["sourceRange"]["span"]["start"].is_u64());
        assert!(diags.iter().any(|d| {
            d.code == "cem.schema_package.artifact_check"
                && d.message.contains("artifact-formatter-profile")
                && d.message.contains("formatter-profile")
        }));
    }

    #[test]
    fn schema_package_artifact_contract_flags_missing_colorizer_profile_from_schema_contract() {
        let dir = schema_package_artifact_contract_fixture_dir(
            "artifact-colorizer-profile-missing",
            &[("colorizers/demo.cemt", demo_color_cemt_source())],
        );
        let package_uri = dir.join("package.cem").display().to_string();
        let diags = run_rules_with_identity_and_source_uri(
            r#"{package @id=demo @version="1.0.0" |
                {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
                {content-type @value="application/vnd.example.demo+cem" @primary=true}
                {artifact
                    @kind="colorizer"
                    @path="colorizers/demo.cemt"
                    @content-type="application/vnd.cem.transform+cem"
                    @schema="https://cem.dev/ns/transform/cem/1"
                    @target-content-type="application/cem"
                    @target-schema="https://cem.dev/ns/cem-ml/1"
                    @target-category="cem-tree"
                    @function-name="demo.color"
                    @function-profile="classes"
                }
            }"#,
            Some(CEM_SCHEMA_PACKAGE_URI),
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
            Some(&package_uri),
        );

        let diagnostic = diags
            .iter()
            .find(|d| {
                d.code == "cem.schema_package.artifact_check"
                    && d.details.as_ref().and_then(|details| {
                        details.get("contract").and_then(serde_json::Value::as_str)
                    }) == Some("artifact-colorizer-profile")
            })
            .expect("schema-owned colorizer profile field contract diagnostic");
        assert!(diagnostic.message.contains("color-profile"));
        let details = diagnostic
            .details
            .as_ref()
            .expect("colorizer profile details");
        assert_eq!(
            details["missingFields"],
            serde_json::json!(["color-profile"])
        );
        assert_eq!(details["checkKind"], serde_json::json!("required-fields"));
    }

    #[test]
    fn schema_package_artifact_contract_flags_unreadable_cemt_source() {
        let dir = schema_package_artifact_contract_fixture_dir("artifact-source-missing", &[]);
        let package_uri = dir.join("package.cem").display().to_string();
        let diags = run_rules_with_identity_and_source_uri(
            r#"{package @id=demo @version="1.0.0" |
                {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
                {content-type @value="application/vnd.example.demo+cem" @primary=true}
                {artifact
                    @kind="formatter"
                    @path="formatters/missing.cemt"
                    @content-type="application/vnd.cem.transform+cem"
                    @schema="https://cem.dev/ns/transform/cem/1"
                    @target-content-type="application/cem"
                    @target-schema="https://cem.dev/ns/cem-ml/1"
                    @target-category="cem-tree"
                    @function-name="demo.format"
                    @formatter-profile="cem.format-tree"
                }
            }"#,
            Some(CEM_SCHEMA_PACKAGE_URI),
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
            Some(&package_uri),
        );

        let diagnostic = diags
            .iter()
            .find(|d| {
                d.code == "cem.schema_package.artifact_check"
                    && d.details.as_ref().and_then(|details| {
                        details.get("checkKind").and_then(serde_json::Value::as_str)
                    }) == Some("artifact-source-readable")
            })
            .expect("schema-owned artifact source readability diagnostic");
        assert!(diagnostic
            .message
            .contains("referenced by CEMT function `demo.format` could not be read"));
        let details = diagnostic
            .details
            .as_ref()
            .expect("artifact source readability details");
        assert_eq!(
            details["contract"],
            serde_json::json!("artifact-output-stage-contract")
        );
        assert_eq!(details["invalidFields"], serde_json::json!(["path"]));
        assert_eq!(
            details["invalidValues"],
            serde_json::json!({
                "path": "formatters/missing.cemt",
            })
        );
        let error = details["error"]
            .as_str()
            .expect("artifact source readability error");
        assert!(error.contains("No such file") || error.contains("os error"));
    }

    #[test]
    fn schema_package_artifact_contract_flags_invalid_cemt_source_from_schema_contract() {
        let dir = schema_package_artifact_contract_fixture_dir(
            "artifact-cemt-invalid",
            &[("formatters/invalid.cemt", invalid_cemt_source())],
        );
        let package_uri = dir.join("package.cem").display().to_string();
        let diags = run_rules_with_identity_and_source_uri(
            r#"{package @id=demo @version="1.0.0" |
                {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
                {content-type @value="application/vnd.example.demo+cem" @primary=true}
                {artifact
                    @kind="formatter"
                    @path="formatters/invalid.cemt"
                    @content-type="application/vnd.cem.transform+cem"
                    @schema="https://cem.dev/ns/transform/cem/1"
                    @target-content-type="application/cem"
                    @target-schema="https://cem.dev/ns/cem-ml/1"
                    @target-category="cem-tree"
                    @function-name="demo.format"
                    @formatter-profile="cem.format-tree"
                }
            }"#,
            Some(CEM_SCHEMA_PACKAGE_URI),
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
            Some(&package_uri),
        );

        let diagnostic = diags
            .iter()
            .find(|d| {
                d.code == "cem.schema_package.artifact_check"
                    && d.details.as_ref().and_then(|details| {
                        details.get("checkKind").and_then(serde_json::Value::as_str)
                    }) == Some("artifact-cemt-valid")
            })
            .expect("schema-owned artifact CEMT validity diagnostic");
        assert!(diagnostic.message.contains("CEMT source is invalid"));
        let details = diagnostic
            .details
            .as_ref()
            .expect("artifact CEMT validity details");
        assert_eq!(
            details["sourceDiagnostic"]["severity"],
            serde_json::json!("Fatal")
        );
        assert!(details["sourceDiagnostic"]["message"]
            .as_str()
            .is_some_and(|message| !message.is_empty()));
        assert_eq!(
            details["invalidValues"],
            serde_json::json!({
                "path": "formatters/invalid.cemt",
            })
        );
    }

    #[test]
    fn schema_package_artifact_contract_flags_missing_cemt_function_from_schema_contract() {
        let dir = schema_package_artifact_contract_fixture_dir(
            "artifact-function-missing",
            &[("formatters/demo.cemt", demo_format_cemt_source())],
        );
        let package_uri = dir.join("package.cem").display().to_string();
        let diags = run_rules_with_identity_and_source_uri(
            r#"{package @id=demo @version="1.0.0" |
                {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
                {content-type @value="application/vnd.example.demo+cem" @primary=true}
                {artifact
                    @kind="formatter"
                    @path="formatters/demo.cemt"
                    @content-type="application/vnd.cem.transform+cem"
                    @schema="https://cem.dev/ns/transform/cem/1"
                    @target-content-type="application/cem"
                    @target-schema="https://cem.dev/ns/cem-ml/1"
                    @target-category="cem-tree"
                    @function-name="demo.missing"
                    @formatter-profile="cem.format-tree"
                }
            }"#,
            Some(CEM_SCHEMA_PACKAGE_URI),
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE),
            Some(&package_uri),
        );

        let diagnostic = diags
            .iter()
            .find(|d| {
                d.code == "cem.schema_package.artifact_check"
                    && d.details.as_ref().and_then(|details| {
                        details.get("checkKind").and_then(serde_json::Value::as_str)
                    }) == Some("artifact-function-declared")
            })
            .expect("schema-owned artifact function declaration diagnostic");
        assert!(diagnostic
            .message
            .contains("does not declare CEMT output function `demo.missing`"));
        let details = diagnostic
            .details
            .as_ref()
            .expect("artifact function declaration details");
        assert_eq!(
            details["invalidFields"],
            serde_json::json!(["function-name"])
        );
        assert_eq!(
            details["expectedValues"],
            serde_json::json!({
                "function-name": "demo.missing",
            })
        );
        assert_eq!(
            details["declaredFunctions"],
            serde_json::json!(["demo.format"])
        );
    }

    fn schema_package_artifact_contract_fixture_dir(
        label: &str,
        files: &[(&str, &str)],
    ) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "cem-ml-schema-package-{label}-{}-{unique}",
            std::process::id()
        ));
        for (path, source) in files {
            let path = dir.join(path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create CEMT fixture directory");
            }
            std::fs::write(path, source).expect("write CEMT fixture");
        }
        dir
    }

    fn demo_format_cemt_source() -> &'static str {
        r#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {format-function
        @name="demo.format"
        @category="cem-tree"
        @subject="cem-ast-node"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @canonical=true
        @deterministic=true
        @streamable=true
    }
}
"#
    }

    fn demo_color_cemt_source() -> &'static str {
        r#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {color-function
        @name="demo.color"
        @category="cem-tree"
        @subject="cem-tree"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @profile="classes"
        @canonical=false
        @deterministic=true
        @streamable=true
    }
}
"#
    }

    fn invalid_cemt_source() -> &'static str {
        r#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {format-function @name="demo.format"
"#
    }

    #[test]
    fn open_content_policy_accepts_custom_elements_data_and_aria() {
        let diags = run_rules(r#"{my-widget @data-track=x @aria-label="Widget"}"#);
        assert!(diags.iter().all(|d| {
            d.code != "cem.schema.unknown_html_element"
                && d.code != "cem.schema.unknown_html_attribute"
        }));
    }

    #[test]
    fn open_content_policy_accepts_html_parity_document_wrapper() {
        let diags = run_rules(
            r#"{html @lang=en | {head | {meta @charset=utf-8} {title | Demo}} {body | {main | Hi}}}"#,
        );
        assert!(diags.iter().all(|d| {
            d.code != "cem.schema.unknown_html_element"
                && d.code != "cem.schema.unknown_html_attribute"
        }));
    }

    #[test]
    fn suspicious_content_type_switch_flagged_on_non_mime_host() {
        let diags = run_rules(r#"{section @type="text/html" | hi}"#);
        assert!(diags
            .iter()
            .any(|d| d.code == "cem.lint.suspicious_content_type_switch"));
    }

    #[test]
    fn content_type_switch_not_flagged_on_known_mime_host() {
        let diags = run_rules(r#"{script @type="application/json" | {{}}}"#);
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.lint.suspicious_content_type_switch"));
    }

    #[test]
    fn input_type_attribute_not_flagged() {
        // `<input type=email>` is the canonical HTML attribute, not a
        // content-type handoff. The tokenizer's `@type=email` value is a
        // bare identifier, not MIME-shaped.
        let diags = run_rules(r#"{input @type=email}"#);
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.lint.suspicious_content_type_switch"));
    }

    #[test]
    fn relaxed_boundary_flagged_when_marker_omitted_with_content() {
        // `{p Hello}` is the relaxed form — content follows attributes
        // (or, here, the name) without the canonical `|` marker.
        let diags = run_rules("{p Hello}");
        assert!(
            diags
                .iter()
                .any(|d| d.code == "cem.lint.relaxed_content_boundary"),
            "expected relaxed-boundary lint, got {diags:?}"
        );
    }

    #[test]
    fn relaxed_boundary_flagged_on_child_element_without_marker() {
        let diags = run_rules("{section {p | hi}}");
        assert!(diags
            .iter()
            .any(|d| d.code == "cem.lint.relaxed_content_boundary"));
    }

    #[test]
    fn relaxed_boundary_clean_when_marker_present() {
        let diags = run_rules("{p | Hello}");
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.lint.relaxed_content_boundary"));
    }

    #[test]
    fn relaxed_boundary_clean_for_element_with_no_content() {
        // No content children, so the boundary marker would be
        // redundant. `{input @required}` must not fire the rule.
        let diags = run_rules("{input @required}");
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.lint.relaxed_content_boundary"));
    }

    #[test]
    fn relaxed_boundary_clean_for_unicode_marker() {
        let diags = run_rules("{p ▷ Hello}");
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.lint.relaxed_content_boundary"));
    }

    #[test]
    fn diagnostics_carry_source_map_for_byte_offset_projection() {
        let diags = run_rules("{button @type=submit}");
        let d = diags
            .iter()
            .find(|d| d.code == "cem.a11y.accessible_name_missing")
            .expect("expected a11y diag");
        assert!(d.byte_offset.is_some(), "byteOffset should be projected");
        assert!(d.source_map.is_some(), "sourceMap should be attached");
    }
}
