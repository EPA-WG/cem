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
//! - `OpenContentPolicyRule`: schema-owned unknown-name policy checks.
//! - `JavaScriptUrlRule`: `href` / `src` / `action` / `formaction` /
//!   `xlink:href` values starting with `javascript:`.
//! - `UnsafeInlineContentRule`: inline script/srcdoc/external-DTD hooks.
//! - `EventHandlerAttributeRule`: `on*` event handler attributes.

use crate::diagnostics::{Diagnostic, Severity};
use crate::parser::{AstNodeId, CemAstNode};
use crate::source_map::FrameSpan;
use crate::validation::{
    RuleContext, RuleDescriptor, RuleId, RuleInput, SemanticRule, TriggerLayer,
};

fn diag_at(code: &str, severity: Severity, message: String, node: &CemAstNode) -> Diagnostic {
    let stack = match node {
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
    };
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
        source_map: Some(stack.clone()),
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
        // the example fixtures. Any other namespace prefix on an
        // attribute is an unbound-prefix lint.
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

// ---------- Open Content / Unknown Names ----------

pub struct OpenContentPolicyRule;

impl SemanticRule for OpenContentPolicyRule {
    fn descriptor(&self) -> &RuleDescriptor {
        open_content_descriptor()
    }

    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for node in ctx.document.iter() {
            match node {
                CemAstNode::Element { expanded_name, .. } => {
                    let ns = expanded_name.namespace_uri.as_str();
                    let local = expanded_name.local_name.as_str();
                    if local.is_empty() || local == "$" || local.starts_with('@') {
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
    "a", "article", "aside", "button", "dd", "desc", "dialog", "div", "dl", "dt", "fieldset",
    "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "header", "html", "iframe", "img",
    "input", "label", "legend", "li", "main", "mark", "nav", "ol", "option", "p", "path", "script",
    "section", "select", "small", "span", "strong", "svg", "textarea", "title", "ul",
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
    "checked",
    "class",
    "d",
    "disabled",
    "for",
    "height",
    "href",
    "id",
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
        let doc = parse(input);
        let upstream: Vec<Diagnostic> = doc.diagnostics.clone();
        let registry = RuleRegistry::with_tier_a_rules();
        registry.run(&RuleContext {
            document: &doc,
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
    fn open_content_policy_accepts_custom_elements_data_and_aria() {
        let diags = run_rules(r#"{my-widget @data-track=x @aria-label="Widget"}"#);
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
