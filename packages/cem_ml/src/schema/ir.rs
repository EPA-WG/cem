//! Schema IR — `CompiledSchema` and the structural / semantic / open-content
//! shapes that emitter PRs (RELAX NG XML/compact, TypeScript `.d.ts`, Rust
//! `.rs`, URI manifest) consume.
//!
//! Authoritative reference: `docs/cem-ml-stack-design-impl.md` §3.4 and
//! `docs/cem-ml-stack-design.md` §13. Older callers reach `CompiledSchema`
//! via `crate::schema::vocab`; that path re-exports from here for
//! back-compat.
//!
//! The runtime trait `crate::validation::SemanticRule` (rule engine) is
//! distinct from the IR type [`SemanticRule`] (rule catalog entry). The
//! IR struct describes which rules the schema declares and where they
//! run; the trait drives execution.
//!
//! `cem_core()` populates every field with concrete Tier A values for the
//! `cem-core/1` schema. No emitter PR may consume a field whose population
//! is documented as `placeholder` in this file.

use std::collections::BTreeMap;

use crate::parser::ExpandedName;
use crate::source::SourceId;
use crate::source_map::SourceMapStack;

use super::SchemaId;

pub const CEM_CORE_NAMESPACE: &str = "https://cem.dev/ns/core/1";
pub const CEM_CORE_SCHEMA_ID: SchemaId = 1;

// ---------------------------------------------------------------------------
// SemVer (cem-ml minimal subset — full SemVer 2.0 surface is Tier B per AC-V-9)
// ---------------------------------------------------------------------------

/// Numeric SemVer triple with optional prerelease/build tails. Implements
/// `Display` as the canonical SemVer 2.0 string; ordering across
/// prereleases follows the SemVer precedence rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub prerelease: Option<String>,
    pub build: Option<String>,
}

impl SemVer {
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
            prerelease: None,
            build: None,
        }
    }

    /// Canonical SemVer 2.0 string. Used as `fingerprint_input` on
    /// [`SchemaVersionIdentity`] when prerelease/build are present.
    pub fn to_canonical_string(&self) -> String {
        let mut out = format!("{}.{}.{}", self.major, self.minor, self.patch);
        if let Some(pre) = &self.prerelease {
            out.push('-');
            out.push_str(pre);
        }
        if let Some(build) = &self.build {
            out.push('+');
            out.push_str(build);
        }
        out
    }
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_canonical_string())
    }
}

// ---------------------------------------------------------------------------
// Version identity / source provenance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaVersionConstraint {
    Unconstrained,
    Major(u64),
    MajorMinor(u64, u64),
    Full(SemVer),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaVersionMatchRule {
    Unconstrained,
    Major,
    MajorMinor,
    Full,
    PrereleaseExact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaVersionIdentity {
    /// Stable schema URI / author constraint.
    pub uri: String,
    /// Complete descriptor version (full SemVer 2.0). Authoritative per
    /// AC-V-9.
    pub embedded_version: SemVer,
    pub constraint: SchemaVersionConstraint,
    pub match_rule: SchemaVersionMatchRule,
    /// Canonical SemVer string including any prerelease/build segments.
    /// Used for byte-stable fingerprinting in artifact manifests.
    pub fingerprint_input: String,
}

#[derive(Debug, Clone)]
pub struct CemNativeSchemaSource {
    pub uri: String,
    /// Complete SemVer 2.0 descriptor version as authored in the
    /// CEM-native source.
    pub version: String,
    pub source_id: SourceId,
    pub source_map: SourceMapStack,
}

// ---------------------------------------------------------------------------
// Annotation vocabulary (moved from vocab.rs)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AnnotationDef {
    pub local_name: &'static str,
    /// `Some(values)` means the value must be in the enum; `None` means any
    /// non-empty string is accepted (free-form id).
    pub allowed_values: Option<Vec<&'static str>>,
    /// Known values for autocomplete / tooling even when `allowed_values`
    /// is `None`. The compiler does not reject values absent from this
    /// list when `allowed_values` is `None`.
    pub known_values: Vec<&'static str>,
    pub allowed_states: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct NonStreamableConstraint {
    pub annotation: &'static str,
    pub kind: NonStreamableKind,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonStreamableKind {
    AttributeOrderNonAdjacent,
    CrossScopePredicate,
    FullDocumentBuffering,
}

// ---------------------------------------------------------------------------
// Structural IR — Tier A DFA profile + RELAX NG functional parity slot
// ---------------------------------------------------------------------------

pub type SchemaState = String;

#[derive(Debug, Clone)]
pub struct SchemaStateDef {
    pub id: SchemaState,
    /// Annotation this state belongs to (e.g. `"badge"`).
    pub annotation: String,
    /// `Some(values)` mirrors [`AnnotationDef::allowed_values`].
    pub allowed_values: Option<Vec<String>>,
    pub allowed_state_names: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralConstraintKind {
    EnumValue,
    StateEnum,
    ChildSequence,
    AttributeRequired,
    ContentBoundary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnsupportedConstraintPolicy {
    /// Tier A default — emit `cem.schema.unsupported_tier_a_constraint`
    /// at schema compile time and reject the schema instead of silently
    /// weakening validation.
    CompileError(String),
}

#[derive(Debug, Clone)]
pub struct TierAValidationProfile {
    pub supported_constraints: Vec<StructuralConstraintKind>,
    pub unsupported_policy: UnsupportedConstraintPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationEngineKind {
    TierADfa,
    RelaxNgDerivative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedContentDiagnosticMode {
    None,
    DfaFollowSet,
    DerivativeResidual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportCompatibility {
    /// No compatibility guarantee across validation engines.
    EngineVersionLocal,
}

#[derive(Debug, Clone)]
pub struct EngineDiagnosticProfile {
    pub engine: ValidationEngineKind,
    pub expected_content: ExpectedContentDiagnosticMode,
    pub report_compatibility: ReportCompatibility,
}

/// Placeholder for the RELAX NG functional-parity IR. Tier A retains
/// structural semantics RELAX-NG-equivalent in shape; the byte-for-byte
/// RNG mapping is populated when the `rng_xml` / `rng_compact` emitters
/// land (AC-S-2).
#[derive(Debug, Clone, Default)]
pub struct RelaxNgEquivalentIr {
    /// Reserved. Empty until the RELAX NG emitter populates it.
    pub grammar_outline: Vec<String>,
}

/// Placeholder for the residual/derivative representation used by the
/// Tier B derivative engine.
#[derive(Debug, Clone, Default)]
pub struct DerivativeIr {
    pub residuals: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StructuralSchemaIr {
    pub entry_state: SchemaState,
    pub relax_ng_equivalent: RelaxNgEquivalentIr,
    pub tier_a_profile: TierAValidationProfile,
    /// DFA-ready limited structural states for Tier A.
    pub states: Vec<SchemaStateDef>,
    /// Full residual/derivative representation. `None` under Tier A.
    pub derivative: Option<DerivativeIr>,
    pub diagnostics: EngineDiagnosticProfile,
}

// ---------------------------------------------------------------------------
// Open content policy
// ---------------------------------------------------------------------------

pub type ContentModelId = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenContentAction {
    Accept,
    AcceptIgnore,
    DeferToSemanticPass,
    DelegateToRegisteredSchema,
    Diagnostic {
        code: String,
        severity: crate::diagnostics::Severity,
    },
}

#[derive(Debug, Clone)]
pub struct OpenContentRule {
    pub content_model: Option<ContentModelId>,
    pub namespace_uri: Option<String>,
    pub open: bool,
    pub unknown_element: OpenContentAction,
    pub unknown_attribute: OpenContentAction,
    pub source: SourceMapStack,
}

/// The 15 default-action mappings spelled out in
/// `cem-ml-stack-design-impl.md` §3.4. Every field is populated by
/// [`OpenContentDefaults::tier_a`]; missing fields are a compile error.
#[derive(Debug, Clone)]
pub struct OpenContentDefaults {
    pub html_unknown_element: OpenContentAction,
    pub html_unknown_attribute: OpenContentAction,
    pub html_custom_element: OpenContentAction,
    pub cem_html_data_attribute: OpenContentAction,
    pub aria_or_role_attribute: OpenContentAction,
    pub active_cem_unknown_element: OpenContentAction,
    pub active_cem_unknown_attribute: OpenContentAction,
    pub other_registered_schema: OpenContentAction,
    pub unbound_prefix: OpenContentAction,
    pub no_namespace_open_true_element: OpenContentAction,
    pub no_namespace_open_true_attribute: OpenContentAction,
    pub no_namespace_open_false_element: OpenContentAction,
    pub no_namespace_open_false_attribute: OpenContentAction,
    pub vendor_prefixed_html_attribute: OpenContentAction,
}

impl OpenContentDefaults {
    fn tier_a() -> Self {
        use crate::diagnostics::Severity;
        Self {
            html_unknown_element: OpenContentAction::Diagnostic {
                code: "cem.schema.unknown_html_element".to_owned(),
                severity: Severity::Error,
            },
            html_unknown_attribute: OpenContentAction::Diagnostic {
                code: "cem.schema.unknown_html_attribute".to_owned(),
                severity: Severity::Warning,
            },
            html_custom_element: OpenContentAction::Accept,
            cem_html_data_attribute: OpenContentAction::AcceptIgnore,
            aria_or_role_attribute: OpenContentAction::DeferToSemanticPass,
            active_cem_unknown_element: OpenContentAction::Diagnostic {
                code: "cem.schema.unknown_cem_element".to_owned(),
                severity: Severity::Error,
            },
            active_cem_unknown_attribute: OpenContentAction::Diagnostic {
                code: "cem.schema.unknown_cem_attribute".to_owned(),
                severity: Severity::Error,
            },
            other_registered_schema: OpenContentAction::DelegateToRegisteredSchema,
            unbound_prefix: OpenContentAction::Diagnostic {
                code: "cem.schema.unbound_prefix".to_owned(),
                severity: Severity::Error,
            },
            no_namespace_open_true_element: OpenContentAction::Diagnostic {
                code: "cem.schema.extension_element".to_owned(),
                severity: Severity::Warning,
            },
            no_namespace_open_true_attribute: OpenContentAction::Diagnostic {
                code: "cem.schema.extension_attribute".to_owned(),
                severity: Severity::Warning,
            },
            no_namespace_open_false_element: OpenContentAction::Diagnostic {
                code: "cem.schema.unknown_element".to_owned(),
                severity: Severity::Error,
            },
            no_namespace_open_false_attribute: OpenContentAction::Diagnostic {
                code: "cem.schema.unknown_attribute".to_owned(),
                severity: Severity::Error,
            },
            vendor_prefixed_html_attribute: OpenContentAction::Diagnostic {
                code: "cem.schema.unknown_html_attribute".to_owned(),
                severity: Severity::Warning,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpenContentPolicy {
    pub rules: Vec<OpenContentRule>,
    pub defaults: OpenContentDefaults,
}

// ---------------------------------------------------------------------------
// Semantic rules
// ---------------------------------------------------------------------------

pub type SemanticRuleId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticRulePhase {
    CrossReference,
    Contextual,
    Policy,
    Transform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintTier {
    Structural,
    CrossReference,
    SemanticContextual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleExecutionPlacement {
    Tokenizer,
    EventNormalizer,
    SchemaMachine,
    ReferenceResolution,
    AstValidation,
    Transform,
    Policy,
}

/// IR catalog entry for a semantic rule. The runtime execution side
/// lives behind the [`crate::validation::SemanticRule`] trait; this
/// struct documents *which* rules the schema declares and at *which*
/// layer they run, so emitters can project the catalog into RELAX NG
/// annotations, `.d.ts` JSDoc, and `.rs` doc comments.
#[derive(Debug, Clone)]
pub struct SemanticRule {
    pub rule_id: SemanticRuleId,
    pub phase: SemanticRulePhase,
    pub dependency_tier: ConstraintTier,
    pub execution: RuleExecutionPlacement,
    pub applies_to: ExpandedName,
    pub severity: crate::diagnostics::Severity,
    pub source: SourceMapStack,
}

// ---------------------------------------------------------------------------
// Transform plans
// ---------------------------------------------------------------------------

/// Reference to a schema-owned transform plan. Populated by the CEM
/// template renderer (`crate::interpreter::template`) when transform
/// plans are declared; the cem-core/1 vocabulary declares none in Tier
/// A. The full [`TransformPlan`](crate::interpreter::template) shape is
/// dereffed when the renderer needs it.
#[derive(Debug, Clone)]
pub struct TransformPlanRef {
    pub plan_id: String,
    pub applies_to: ExpandedName,
    pub source: SourceMapStack,
}

// ---------------------------------------------------------------------------
// CompiledSchema (extended)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CompiledSchema {
    pub schema_id: SchemaId,
    pub namespace_uri: &'static str,
    pub annotations: BTreeMap<&'static str, AnnotationDef>,
    pub state_matrix: Vec<&'static str>,
    /// Reserved: rules that would require non-streamable evaluation are
    /// rejected at compile time with `cem.schema.unsupported_constraint`.
    /// Tier A authors no such rules.
    pub non_streamable_constraints: Vec<NonStreamableConstraint>,

    // §3.4 IR extensions ------------------------------------------------
    pub version_identity: SchemaVersionIdentity,
    pub source: CemNativeSchemaSource,
    pub structural: StructuralSchemaIr,
    pub semantic_rules: Vec<SemanticRule>,
    pub transform_plans: Vec<TransformPlanRef>,
    pub open_content: OpenContentPolicy,
}

impl CompiledSchema {
    /// Returns the annotation definition for the given local name (i.e.
    /// the part after `cem:`), or `None` if the name is not part of the
    /// active vocabulary.
    pub fn annotation(&self, local: &str) -> Option<&AnnotationDef> {
        self.annotations.get(local)
    }

    pub fn is_known_state(&self, state: &str) -> bool {
        self.state_matrix.contains(&state)
    }

    /// Build the active Tier A schema mirroring `schema/cem-core.md`,
    /// with every §3.4 IR field populated for the `cem-core/1`
    /// namespace.
    pub fn cem_core() -> Self {
        let annotations = build_cem_core_annotations();
        let state_matrix = cem_core_state_matrix();

        let version_identity = SchemaVersionIdentity {
            uri: CEM_CORE_NAMESPACE.to_owned(),
            embedded_version: SemVer::new(1, 0, 0),
            constraint: SchemaVersionConstraint::Major(1),
            match_rule: SchemaVersionMatchRule::Major,
            fingerprint_input: "1.0.0".to_owned(),
        };

        let source = CemNativeSchemaSource {
            uri: CEM_CORE_NAMESPACE.to_owned(),
            version: "1.0.0".to_owned(),
            // SourceId(0) is the synthetic in-process source. The
            // CEM-native markdown loader will swap in a real source id
            // once schema/cem-core.md is parsed at load time.
            source_id: SourceId(0),
            source_map: SourceMapStack::default(),
        };

        let structural = build_cem_core_structural(&annotations);
        let semantic_rules = build_cem_core_semantic_rules();
        let open_content = OpenContentPolicy {
            rules: Vec::new(),
            defaults: OpenContentDefaults::tier_a(),
        };

        Self {
            schema_id: CEM_CORE_SCHEMA_ID,
            namespace_uri: CEM_CORE_NAMESPACE,
            annotations,
            state_matrix,
            non_streamable_constraints: Vec::new(),
            version_identity,
            source,
            structural,
            semantic_rules,
            transform_plans: Vec::new(),
            open_content,
        }
    }
}

// ---------------------------------------------------------------------------
// cem-core factory helpers
// ---------------------------------------------------------------------------

fn build_cem_core_annotations() -> BTreeMap<&'static str, AnnotationDef> {
    let mut annotations: BTreeMap<&'static str, AnnotationDef> = BTreeMap::new();

    annotations.insert(
        "screen",
        AnnotationDef {
            local_name: "screen",
            allowed_values: None,
            known_values: vec![
                "login",
                "registration",
                "profile",
                "assets",
                "message-thread",
            ],
            allowed_states: vec!["default", "loading", "empty"],
        },
    );
    annotations.insert(
        "form",
        AnnotationDef {
            local_name: "form",
            allowed_values: None,
            known_values: vec![
                "sign-in",
                "registration",
                "asset-filter",
                "profile-preferences",
                "message-reply",
            ],
            allowed_states: vec!["default", "disabled", "invalid", "loading"],
        },
    );
    annotations.insert(
        "action",
        AnnotationDef {
            local_name: "action",
            allowed_values: Some(vec!["primary", "secondary"]),
            known_values: vec!["primary", "secondary"],
            allowed_states: vec![
                "default",
                "hover",
                "focus-visible",
                "active",
                "disabled",
                "loading",
            ],
        },
    );
    annotations.insert(
        "badge",
        AnnotationDef {
            local_name: "badge",
            allowed_values: Some(vec!["success", "info", "warning", "error"]),
            known_values: vec!["success", "info", "warning", "error"],
            allowed_states: vec!["default"],
        },
    );
    annotations.insert(
        "card",
        AnnotationDef {
            local_name: "card",
            allowed_values: None,
            known_values: vec!["identity", "preferences", "summary"],
            allowed_states: vec!["default", "selected", "loading", "empty"],
        },
    );
    annotations.insert(
        "list",
        AnnotationDef {
            local_name: "list",
            allowed_values: None,
            known_values: vec!["assets", "results", "notifications"],
            allowed_states: vec!["default", "loading", "empty"],
        },
    );
    annotations.insert(
        "row",
        AnnotationDef {
            local_name: "row",
            allowed_values: None,
            known_values: vec!["asset", "result", "notification"],
            allowed_states: vec!["default", "hover", "focus-visible", "selected", "disabled"],
        },
    );
    annotations.insert(
        "thread",
        AnnotationDef {
            local_name: "thread",
            allowed_values: None,
            known_values: vec!["support", "notifications"],
            allowed_states: vec!["default", "loading", "empty"],
        },
    );
    annotations.insert(
        "message",
        AnnotationDef {
            local_name: "message",
            allowed_values: Some(vec!["sent", "received"]),
            known_values: vec!["sent", "received"],
            allowed_states: vec!["default"],
        },
    );

    annotations
}

fn cem_core_state_matrix() -> Vec<&'static str> {
    vec![
        "default",
        "hover",
        "focus-visible",
        "active",
        "selected",
        "disabled",
        "invalid",
        "required",
        "loading",
        "empty",
    ]
}

fn build_cem_core_structural(
    annotations: &BTreeMap<&'static str, AnnotationDef>,
) -> StructuralSchemaIr {
    // One DFA state per annotation — Tier A's DFA executes a state
    // machine over allowed values + allowed states. BTreeMap iteration
    // is sorted, so this is deterministic.
    let states = annotations
        .values()
        .map(|def| SchemaStateDef {
            id: format!("cem-core/state/{}", def.local_name),
            annotation: def.local_name.to_owned(),
            allowed_values: def
                .allowed_values
                .as_ref()
                .map(|vs| vs.iter().map(|v| (*v).to_owned()).collect()),
            allowed_state_names: def.allowed_states.iter().map(|s| (*s).to_owned()).collect(),
        })
        .collect();

    StructuralSchemaIr {
        entry_state: "cem-core/state/root".to_owned(),
        relax_ng_equivalent: RelaxNgEquivalentIr::default(),
        tier_a_profile: TierAValidationProfile {
            supported_constraints: vec![
                StructuralConstraintKind::EnumValue,
                StructuralConstraintKind::StateEnum,
                StructuralConstraintKind::ChildSequence,
                StructuralConstraintKind::AttributeRequired,
                StructuralConstraintKind::ContentBoundary,
            ],
            unsupported_policy: UnsupportedConstraintPolicy::CompileError(
                "cem.schema.unsupported_tier_a_constraint".to_owned(),
            ),
        },
        states,
        derivative: None,
        diagnostics: EngineDiagnosticProfile {
            engine: ValidationEngineKind::TierADfa,
            expected_content: ExpectedContentDiagnosticMode::DfaFollowSet,
            report_compatibility: ReportCompatibility::EngineVersionLocal,
        },
    }
}

fn cem_rule_target(local: &str) -> ExpandedName {
    ExpandedName {
        namespace_uri: CEM_CORE_NAMESPACE.to_owned(),
        local_name: local.to_owned(),
        schema_id: Some(CEM_CORE_SCHEMA_ID),
    }
}

fn build_cem_core_semantic_rules() -> Vec<SemanticRule> {
    use crate::diagnostics::Severity;

    // One entry per `validation::RuleRegistry::with_tier_a_rules`
    // registration. The exec placement reflects where the rule lands
    // in the layered runtime, not where its descriptor's
    // `TriggerLayer` says it runs — the IR documents *schema-declared*
    // execution placement; the runtime trait documents trigger
    // scheduling.
    vec![
        SemanticRule {
            rule_id: "cem.refs.integrity".to_owned(),
            phase: SemanticRulePhase::CrossReference,
            dependency_tier: ConstraintTier::CrossReference,
            execution: RuleExecutionPlacement::ReferenceResolution,
            applies_to: cem_rule_target("ref"),
            severity: Severity::Error,
            source: SourceMapStack::default(),
        },
        SemanticRule {
            rule_id: "cem.a11y.accessible_name".to_owned(),
            phase: SemanticRulePhase::Contextual,
            dependency_tier: ConstraintTier::SemanticContextual,
            execution: RuleExecutionPlacement::AstValidation,
            applies_to: cem_rule_target("action"),
            severity: Severity::Error,
            source: SourceMapStack::default(),
        },
        SemanticRule {
            rule_id: "cem.state.invalid_combination".to_owned(),
            phase: SemanticRulePhase::Contextual,
            dependency_tier: ConstraintTier::SemanticContextual,
            execution: RuleExecutionPlacement::SchemaMachine,
            applies_to: cem_rule_target("state"),
            severity: Severity::Error,
            source: SourceMapStack::default(),
        },
        SemanticRule {
            rule_id: "cem.policy.javascript_url".to_owned(),
            phase: SemanticRulePhase::Policy,
            dependency_tier: ConstraintTier::SemanticContextual,
            execution: RuleExecutionPlacement::Policy,
            applies_to: cem_rule_target("href"),
            severity: Severity::Error,
            source: SourceMapStack::default(),
        },
        SemanticRule {
            rule_id: "cem.policy.event_handler_attribute".to_owned(),
            phase: SemanticRulePhase::Policy,
            dependency_tier: ConstraintTier::SemanticContextual,
            execution: RuleExecutionPlacement::Policy,
            applies_to: cem_rule_target("on*"),
            severity: Severity::Error,
            source: SourceMapStack::default(),
        },
        SemanticRule {
            rule_id: "cem.schema.unbound_prefix".to_owned(),
            phase: SemanticRulePhase::CrossReference,
            dependency_tier: ConstraintTier::CrossReference,
            execution: RuleExecutionPlacement::Tokenizer,
            applies_to: cem_rule_target("*"),
            severity: Severity::Error,
            source: SourceMapStack::default(),
        },
        SemanticRule {
            rule_id: "cem.lint.noncanonical_delimiter".to_owned(),
            phase: SemanticRulePhase::Contextual,
            dependency_tier: ConstraintTier::SemanticContextual,
            execution: RuleExecutionPlacement::Tokenizer,
            applies_to: cem_rule_target("*"),
            severity: Severity::Warning,
            source: SourceMapStack::default(),
        },
        SemanticRule {
            rule_id: "cem.lint.suspicious_content_type_switch".to_owned(),
            phase: SemanticRulePhase::Contextual,
            dependency_tier: ConstraintTier::SemanticContextual,
            execution: RuleExecutionPlacement::EventNormalizer,
            applies_to: cem_rule_target("*"),
            severity: Severity::Warning,
            source: SourceMapStack::default(),
        },
        SemanticRule {
            rule_id: "cem.lint.relaxed_content_boundary".to_owned(),
            phase: SemanticRulePhase::Contextual,
            dependency_tier: ConstraintTier::SemanticContextual,
            execution: RuleExecutionPlacement::EventNormalizer,
            applies_to: cem_rule_target("*"),
            severity: Severity::Warning,
            source: SourceMapStack::default(),
        },
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Legacy vocab tests — kept here so the move from vocab.rs to ir.rs
    // does not lose coverage.

    #[test]
    fn cem_core_includes_every_fixture_annotation() {
        let s = CompiledSchema::cem_core();
        for name in [
            "screen", "form", "action", "badge", "card", "list", "row", "thread", "message",
        ] {
            assert!(s.annotation(name).is_some(), "missing annotation: {name}");
        }
    }

    #[test]
    fn enum_annotations_carry_allowed_values() {
        let s = CompiledSchema::cem_core();
        let action = s.annotation("action").unwrap();
        assert_eq!(
            action.allowed_values.as_deref(),
            Some(["primary", "secondary"].as_slice())
        );
    }

    #[test]
    fn state_matrix_matches_component_mvp() {
        let s = CompiledSchema::cem_core();
        for state in [
            "default",
            "hover",
            "focus-visible",
            "active",
            "selected",
            "disabled",
            "invalid",
            "required",
            "loading",
            "empty",
        ] {
            assert!(s.is_known_state(state), "state matrix missing: {state}");
        }
    }

    #[test]
    fn tier_a_declares_no_non_streamable_constraints() {
        let s = CompiledSchema::cem_core();
        assert!(s.non_streamable_constraints.is_empty());
    }

    // §3.4 IR-extension coverage --------------------------------------

    #[test]
    fn version_identity_uri_matches_active_namespace() {
        let s = CompiledSchema::cem_core();
        assert_eq!(s.version_identity.uri, CEM_CORE_NAMESPACE);
        assert_eq!(s.version_identity.embedded_version, SemVer::new(1, 0, 0));
        assert_eq!(
            s.version_identity.constraint,
            SchemaVersionConstraint::Major(1)
        );
        assert_eq!(s.version_identity.match_rule, SchemaVersionMatchRule::Major);
        assert_eq!(s.version_identity.fingerprint_input, "1.0.0");
    }

    #[test]
    fn semver_canonical_string_round_trip() {
        let v = SemVer {
            major: 1,
            minor: 2,
            patch: 3,
            prerelease: Some("rc.1".to_owned()),
            build: Some("sha.abc".to_owned()),
        };
        assert_eq!(v.to_canonical_string(), "1.2.3-rc.1+sha.abc");
        assert_eq!(SemVer::new(1, 0, 0).to_canonical_string(), "1.0.0");
    }

    #[test]
    fn source_provenance_carries_active_namespace() {
        let s = CompiledSchema::cem_core();
        assert_eq!(s.source.uri, CEM_CORE_NAMESPACE);
        assert_eq!(s.source.version, "1.0.0");
    }

    #[test]
    fn open_content_defaults_populate_all_15_branches() {
        use crate::diagnostics::Severity;
        let d = &CompiledSchema::cem_core().open_content.defaults;

        // Diagnostic branches — assert code + severity per §3.4.
        match &d.html_unknown_element {
            OpenContentAction::Diagnostic { code, severity } => {
                assert_eq!(code, "cem.schema.unknown_html_element");
                assert_eq!(*severity, Severity::Error);
            }
            other => panic!("expected Diagnostic, got {other:?}"),
        }
        match &d.html_unknown_attribute {
            OpenContentAction::Diagnostic { code, severity } => {
                assert_eq!(code, "cem.schema.unknown_html_attribute");
                assert_eq!(*severity, Severity::Warning);
            }
            other => panic!("expected Diagnostic, got {other:?}"),
        }
        assert_eq!(d.html_custom_element, OpenContentAction::Accept);
        assert_eq!(d.cem_html_data_attribute, OpenContentAction::AcceptIgnore);
        assert_eq!(
            d.aria_or_role_attribute,
            OpenContentAction::DeferToSemanticPass
        );

        match &d.active_cem_unknown_element {
            OpenContentAction::Diagnostic { code, severity } => {
                assert_eq!(code, "cem.schema.unknown_cem_element");
                assert_eq!(*severity, Severity::Error);
            }
            other => panic!("expected Diagnostic, got {other:?}"),
        }
        match &d.active_cem_unknown_attribute {
            OpenContentAction::Diagnostic { code, severity } => {
                assert_eq!(code, "cem.schema.unknown_cem_attribute");
                assert_eq!(*severity, Severity::Error);
            }
            other => panic!("expected Diagnostic, got {other:?}"),
        }
        assert_eq!(
            d.other_registered_schema,
            OpenContentAction::DelegateToRegisteredSchema
        );
        match &d.unbound_prefix {
            OpenContentAction::Diagnostic { code, severity } => {
                assert_eq!(code, "cem.schema.unbound_prefix");
                assert_eq!(*severity, Severity::Error);
            }
            other => panic!("expected Diagnostic, got {other:?}"),
        }

        // The four no-namespace branches (open=true|false × element|attribute).
        match &d.no_namespace_open_true_element {
            OpenContentAction::Diagnostic { code, severity } => {
                assert_eq!(code, "cem.schema.extension_element");
                assert_eq!(*severity, Severity::Warning);
            }
            other => panic!("expected Diagnostic, got {other:?}"),
        }
        match &d.no_namespace_open_true_attribute {
            OpenContentAction::Diagnostic { code, severity } => {
                assert_eq!(code, "cem.schema.extension_attribute");
                assert_eq!(*severity, Severity::Warning);
            }
            other => panic!("expected Diagnostic, got {other:?}"),
        }
        match &d.no_namespace_open_false_element {
            OpenContentAction::Diagnostic { code, severity } => {
                assert_eq!(code, "cem.schema.unknown_element");
                assert_eq!(*severity, Severity::Error);
            }
            other => panic!("expected Diagnostic, got {other:?}"),
        }
        match &d.no_namespace_open_false_attribute {
            OpenContentAction::Diagnostic { code, severity } => {
                assert_eq!(code, "cem.schema.unknown_attribute");
                assert_eq!(*severity, Severity::Error);
            }
            other => panic!("expected Diagnostic, got {other:?}"),
        }
        match &d.vendor_prefixed_html_attribute {
            OpenContentAction::Diagnostic { code, severity } => {
                assert_eq!(code, "cem.schema.unknown_html_attribute");
                assert_eq!(*severity, Severity::Warning);
            }
            other => panic!("expected Diagnostic, got {other:?}"),
        }
    }

    #[test]
    fn structural_states_cover_every_annotation() {
        let s = CompiledSchema::cem_core();
        let state_anns: std::collections::BTreeSet<&str> = s
            .structural
            .states
            .iter()
            .map(|st| st.annotation.as_str())
            .collect();
        for name in [
            "screen", "form", "action", "badge", "card", "list", "row", "thread", "message",
        ] {
            assert!(
                state_anns.contains(name),
                "structural.states missing entry for annotation: {name}"
            );
        }
    }

    #[test]
    fn structural_tier_a_profile_lists_supported_constraints() {
        let s = CompiledSchema::cem_core();
        for kind in [
            StructuralConstraintKind::EnumValue,
            StructuralConstraintKind::StateEnum,
            StructuralConstraintKind::ChildSequence,
            StructuralConstraintKind::AttributeRequired,
            StructuralConstraintKind::ContentBoundary,
        ] {
            assert!(
                s.structural
                    .tier_a_profile
                    .supported_constraints
                    .contains(&kind),
                "tier_a_profile missing constraint: {kind:?}"
            );
        }
        assert_eq!(
            s.structural.diagnostics.engine,
            ValidationEngineKind::TierADfa
        );
    }

    #[test]
    fn semantic_rules_cover_every_tier_a_registration() {
        let s = CompiledSchema::cem_core();
        let ids: std::collections::BTreeSet<&str> = s
            .semantic_rules
            .iter()
            .map(|r| r.rule_id.as_str())
            .collect();
        for expected in [
            "cem.refs.integrity",
            "cem.a11y.accessible_name",
            "cem.state.invalid_combination",
            "cem.policy.javascript_url",
            "cem.policy.event_handler_attribute",
            "cem.schema.unbound_prefix",
            "cem.lint.noncanonical_delimiter",
            "cem.lint.suspicious_content_type_switch",
            "cem.lint.relaxed_content_boundary",
        ] {
            assert!(
                ids.contains(expected),
                "semantic_rules missing rule_id: {expected}"
            );
        }
    }

    #[test]
    fn tier_a_declares_no_transform_plans() {
        let s = CompiledSchema::cem_core();
        assert!(s.transform_plans.is_empty());
    }

    #[test]
    fn open_content_rules_start_empty_for_cem_core() {
        let s = CompiledSchema::cem_core();
        assert!(s.open_content.rules.is_empty());
    }
}
