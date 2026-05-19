//! Semantic-rule catalog + Tier A registry.
//!
//! Per the validation block of `docs/todo.md` and AC-V-* / AC-X-*: every
//! semantic rule has a stable `RuleId`, a clearly named trigger layer, a
//! declared set of required inputs, a default severity, and a policy
//! override hook. Rules are extensible — adding a CSS / JS / XML / JSON /
//! plugin rule means implementing the [`SemanticRule`] trait and
//! registering with [`RuleRegistry`].
//!
//! Tier A rule catalog (see `tier_a_rules`):
//!
//! - `cem.ref.unresolved_reference` — `id` / `for` / `aria-*` reference
//!   integrity.
//! - `cem.a11y.accessible_name_missing` — interactive elements must carry
//!   text content or `aria-label` / `aria-labelledby`.
//! - `cem.state.invalid_combination` — disallowed state combinations on
//!   the active CEM annotation (e.g. `disabled` + `loading`).
//! - `cem.unsafe.javascript_url` — `href` / `src` / `action` attributes
//!   carrying a `javascript:` URL.
//! - `cem.unsafe.event_handler_attribute` — `on*` / DOM-style event
//!   handler attributes.
//! - `cem.struct.unknown_annotation` — re-surface schema-machine "unknown
//!   annotation" diagnostics so the validation layer is a single entry
//!   point.
//! - `cem.lint.relaxed_content_boundary` — recommend the explicit `|` /
//!   `▷` content boundary on elements that carry content.

pub mod rules;

use crate::diagnostics::{Diagnostic, Severity};
use crate::parser::document::CemDocument;

/// Stable identifier for a semantic rule, scoped to its owning schema /
/// content type. Serialized form is `<scope>.<rule>` (e.g.
/// `cem.a11y.accessible_name_missing`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleId(pub String);

impl RuleId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Where the rule fires in the layered runtime. Used to schedule rules
/// and to surface in trace output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerLayer {
    /// Lexical / tokenizer level — runs against raw tokens.
    Tokenizer,
    /// Schema-machine level — runs against `SchemaFrame` transitions.
    SchemaMachine,
    /// Built `CemDocument` — runs after AST construction. Tier A's
    /// default for semantic rules.
    Document,
    /// Cross-document phase — reserved for Tier B.
    CrossDocument,
}

/// Inputs a rule declares it needs. Tier A rules consume the AST; future
/// rules may consume the event stream, the schema frame snapshots, or
/// external policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleInput {
    CemDocument,
    SchemaFrames,
    NormalizedEvents,
    Policy,
}

#[derive(Debug, Clone)]
pub struct RuleDescriptor {
    pub id: RuleId,
    /// Schema or content type this rule is owned by. CEM Core rules use
    /// `"cem-core"`; HTML/SVG accessibility rules use `"cem-a11y"`;
    /// unsafe-content rules use `"cem-policy"`.
    pub owning_scope: &'static str,
    pub content_type: Option<&'static str>,
    pub trigger_layer: TriggerLayer,
    pub required_inputs: &'static [RuleInput],
    pub default_severity: Severity,
    /// Hint for the policy layer: this severity MAY be overridden by the
    /// active `ScopePolicy` for the document or scope. Tier A respects
    /// the default; AC-F-1 policy overrides land with the policy layer.
    pub policy_overridable: bool,
}

/// Inputs the registry hands to a rule during a single Document-layer run.
pub struct RuleContext<'a> {
    pub document: &'a CemDocument,
    /// Diagnostics emitted by upstream layers (decoder, tokenizer, schema
    /// machine, AST builder). Rules may consult this list to skip
    /// downstream work when an upstream layer already failed.
    pub upstream_diagnostics: &'a [Diagnostic],
}

pub trait SemanticRule: Send + Sync {
    fn descriptor(&self) -> &RuleDescriptor;
    fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic>;
}

#[derive(Default)]
pub struct RuleRegistry {
    rules: Vec<Box<dyn SemanticRule>>,
}

impl RuleRegistry {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn register(&mut self, rule: Box<dyn SemanticRule>) {
        self.rules.push(rule);
    }

    /// Register the Tier A catalog of CEM UI + policy rules.
    pub fn with_tier_a_rules() -> Self {
        let mut r = Self::new();
        r.register(Box::new(rules::ReferenceIntegrityRule));
        r.register(Box::new(rules::AccessibleNameRule));
        r.register(Box::new(rules::StateCombinationRule));
        r.register(Box::new(rules::JavaScriptUrlRule));
        r.register(Box::new(rules::EventHandlerAttributeRule));
        r.register(Box::new(rules::UnboundPrefixRule));
        r.register(Box::new(rules::NoncanonicalDelimiterRule));
        r.register(Box::new(rules::SuspiciousContentTypeSwitchRule));
        r.register(Box::new(rules::RelaxedBoundaryRule));
        r
    }

    pub fn descriptors(&self) -> Vec<&RuleDescriptor> {
        self.rules.iter().map(|r| r.descriptor()).collect()
    }

    pub fn run(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for rule in &self.rules {
            out.extend(rule.run(ctx));
        }
        out
    }
}

/// Aggregated validation outcome.
#[derive(Debug, Default)]
pub struct ValidationReport {
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    pub fn hard_violations(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
            .count()
    }

    pub fn has_code(&self, code: &str) -> bool {
        self.diagnostics.iter().any(|d| d.code == code)
    }
}

/// End-to-end Tier A validation: tokenize → normalize → schema-validate →
/// build AST → run rule registry → return merged diagnostics.
///
/// Each layer's diagnostics carry `byteOffset` and a `sourceMap` projection
/// where available, per `cem-ml-cli-contract.md` §Output Shapes.
pub fn run(input: &str) -> ValidationReport {
    use crate::events::cem::CemEventNormalizer;
    use crate::parser::builder::CemAstBuilder;
    use crate::schema::machine::CemSchemaMachine;
    use crate::schema::vocab::CompiledSchema;
    use crate::source::{BytesSource, SourceId};
    use crate::tokenizer::cem::CemTokenizer;

    // Layer 4 — schema machine (consumes its own event stream).
    let schema_outcome = {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer).run()
    };

    // Layer 6 — AST builder (separate parse to keep the boundary clean).
    let mut document = {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let mut tok = CemTokenizer::from_source(src);
        let tok_diags = tok.take_diagnostics();
        let normalizer = CemEventNormalizer::new(tok);
        let mut doc = CemAstBuilder::new(normalizer).build();
        // Fold tokenizer-level diagnostics into the AST's diagnostic list
        // so the validation report carries every layer.
        doc.diagnostics.extend(tok_diags);
        doc
    };
    document.diagnostics.extend(schema_outcome.diagnostics);

    let registry = RuleRegistry::with_tier_a_rules();
    let rule_diags = registry.run(&RuleContext {
        document: &document,
        upstream_diagnostics: &document.diagnostics,
    });

    let mut all = document.diagnostics;
    all.extend(rule_diags);
    ValidationReport { diagnostics: all }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_descriptors_carry_stable_ids() {
        let r = RuleRegistry::with_tier_a_rules();
        let codes: Vec<&str> = r.descriptors().iter().map(|d| d.id.as_str()).collect();
        assert!(codes.contains(&"cem.ref.unresolved_reference"));
        assert!(codes.contains(&"cem.a11y.accessible_name_missing"));
        assert!(codes.contains(&"cem.state.invalid_combination"));
        assert!(codes.contains(&"cem.unsafe.javascript_url"));
        assert!(codes.contains(&"cem.unsafe.event_handler_attribute"));
        assert!(codes.contains(&"cem.lint.unbound_prefix"));
        assert!(codes.contains(&"cem.lint.noncanonical_delimiter"));
        assert!(codes.contains(&"cem.lint.suspicious_content_type_switch"));
        assert!(codes.contains(&"cem.lint.relaxed_content_boundary"));
    }

    #[test]
    fn every_canonical_fixture_validates_clean() {
        let dir =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/cem-ml");
        let mut checked = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("cem") {
                continue;
            }
            let input = std::fs::read_to_string(&path).unwrap();
            let report = run(&input);
            assert_eq!(
                report.hard_violations(),
                0,
                "fixture `{}` produced hard violations: {:?}",
                path.display(),
                report
                    .diagnostics
                    .iter()
                    .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
                    .collect::<Vec<_>>()
            );
            checked += 1;
        }
        assert!(checked >= 5);
    }
}
