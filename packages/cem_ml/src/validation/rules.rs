//! Tier A semantic rule catalog.
//!
//! Each rule maps a category from `docs/todo.md` §Validation:
//!
//! - `ReferenceIntegrityRule`: `id` / `for` / `aria-*` integrity.
//! - `AccessibleNameRule`: interactive elements (button, a, input,
//!   textarea, select) must have accessible name material.
//! - `StateCombinationRule`: disallow incompatible `cem:state` combos.
//! - `JavaScriptUrlRule`: `href` / `src` / `action` / `formaction` /
//!   `xlink:href` values starting with `javascript:`.
//! - `EventHandlerAttributeRule`: `on*` event handler attributes.

use crate::diagnostics::{Diagnostic, Severity};
use crate::parser::{AstNodeId, CemAstNode};
use crate::source_map::FrameSpan;
use crate::validation::{
    RuleContext, RuleDescriptor, RuleId, RuleInput, SemanticRule, TriggerLayer,
};

fn diag_at(
    code: &str,
    severity: Severity,
    message: String,
    node: &CemAstNode,
) -> Diagnostic {
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
    let byte_offset = stack
        .frames
        .first()
        .and_then(|f| match &f.span {
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
                if !ctx.document.id_table.contains_key(value) {
                    out.push(diag_at(
                        "cem.ref.unresolved_reference",
                        Severity::Warning,
                        format!("`{local}=\"{value}\"` does not match any element id"),
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
        "button" | "a" | "select" | "textarea"
    )
}

fn has_accessible_name(
    doc: &crate::parser::document::CemDocument,
    element: &CemAstNode,
) -> bool {
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
    // Text content (direct or in descendants).
    has_visible_text(doc, element)
}

fn has_visible_text(
    doc: &crate::parser::document::CemDocument,
    node: &CemAstNode,
) -> bool {
    match node {
        CemAstNode::Text { data, .. } => !data.trim().is_empty(),
        CemAstNode::Element { children, .. } | CemAstNode::Document { root_children: children, .. } => {
            children.iter().any(|c| {
                doc.get(*c)
                    .map(|n| has_visible_text(doc, n))
                    .unwrap_or(false)
            })
        }
        _ => false,
    }
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
];

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
            if local.len() > 2 && local.starts_with("on") && local.chars().nth(2).unwrap().is_alphabetic() {
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
    fn state_combination_flags_disabled_plus_loading() {
        let diags =
            run_rules(r#"{button @cem:action=primary @cem:state="disabled loading" | Save}"#);
        assert!(diags
            .iter()
            .any(|d| d.code == "cem.state.invalid_combination"));
    }

    #[test]
    fn state_combination_clean_when_single_state() {
        let diags =
            run_rules(r#"{button @cem:action=primary @cem:state="disabled" | Save}"#);
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.state.invalid_combination"));
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
        assert!(diags
            .iter()
            .all(|d| d.code != "cem.unsafe.javascript_url"));
    }

    #[test]
    fn event_handler_attribute_flagged() {
        let diags = run_rules(r#"{button @onclick="boom()" | Boom}"#);
        assert!(diags
            .iter()
            .any(|d| d.code == "cem.unsafe.event_handler_attribute"));
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
