//! Effective scope disposition for consumer-supplied unresolved reference facts.
//! This module neither evaluates expressions nor follows or edits graph edges.

use super::{
    document_model::{compile_schema_document_model, DiagnosticDefinition, SchemaDocumentModel},
    package_sources::builtin_schema_package_source,
    reference_traversal::ReferenceTraversalLimits,
    registry::CEM_ML_SCHEMA_URI,
};
use crate::{
    diagnostics::{Diagnostic, Severity},
    parser::AstNodeId,
    source_map::{FrameSpan, SourceMapStack},
};
use std::{fmt, sync::OnceLock};

pub(crate) const DISPOSITION: &str = "reference-unresolved-disposition";
pub(crate) const DUPLICATE_POLICY_CODE: &str = "cem.schema.reference_policy_duplicate";

pub(crate) fn is_scope_reference_policy(kind: &str) -> bool {
    matches!(
        kind,
        DISPOSITION | "reference-traversal-depth" | "reference-traversal-work"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnresolvedDisposition {
    Neutral,
    Mandatory,
    Warning,
    Ignore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceOccurrence {
    /// Runtime occurrence identity, including its owner. Not an authored ID.
    pub identity: String,
    /// Source arena handle when available; constructed native references need
    /// not belong to a persisted arena or capture a runtime context handle.
    pub node_id: Option<AstNodeId>,
    pub expression: Option<String>,
    pub source_map: SourceMapStack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedReferenceFact {
    pub occurrence: ReferenceOccurrence,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct UnresolvedReferenceTreatment {
    pub fact: UnresolvedReferenceFact,
    pub disposition: UnresolvedDisposition,
    /// Only the mandatory disposition makes this treatment a failure.
    /// False does not imply successful resolution or accepted cardinality.
    pub failed: bool,
    pub diagnostic: Option<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct ReferenceUnresolvedPolicy {
    disposition: UnresolvedDisposition,
    diagnostic: Option<DiagnosticDefinition>,
}

/// Bounds and unresolved-link disposition from one effective scope. The caller
/// owns scope selection and timing; this does not capture a runtime context.
#[derive(Debug, Clone)]
pub struct ReferenceScopePolicy {
    pub limits: ReferenceTraversalLimits,
    pub unresolved: ReferenceUnresolvedPolicy,
}

/// Declared settings keep their origin even when their values equal defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferencePolicyOverrideOrigin {
    Caller,
    Schema(SourceMapStack),
}
#[derive(Debug, Clone)]
pub struct DeclaredReferencePolicySetting<T> {
    value: T,
    origin: ReferencePolicyOverrideOrigin,
}
impl<T> DeclaredReferencePolicySetting<T> {
    pub fn value(&self) -> &T {
        &self.value
    }
    pub fn origin(&self) -> &ReferencePolicyOverrideOrigin {
        &self.origin
    }
}

/// None means inherit, not an explicit declaration of the standard value.
/// Complete legacy caller policies declare all settings; schema construction
/// retains only authored local declarations and validates them atomically.
#[derive(Debug, Clone, Default)]
pub struct ReferenceScopePolicyOverrides {
    depth: Option<DeclaredReferencePolicySetting<usize>>,
    work: Option<DeclaredReferencePolicySetting<usize>>,
    unresolved: Option<DeclaredReferencePolicySetting<ReferenceUnresolvedPolicy>>,
}
impl ReferenceScopePolicyOverrides {
    pub fn explicit(policy: ReferenceScopePolicy) -> Self {
        let caller = |value| DeclaredReferencePolicySetting {
            value,
            origin: ReferencePolicyOverrideOrigin::Caller,
        };
        Self {
            depth: Some(caller(policy.limits.max_depth)),
            work: Some(caller(policy.limits.max_work)),
            unresolved: Some(DeclaredReferencePolicySetting {
                value: policy.unresolved,
                origin: ReferencePolicyOverrideOrigin::Caller,
            }),
        }
    }
    pub fn from_schema(model: &SchemaDocumentModel) -> Result<Self, ReferencePolicyError> {
        let policy = ReferenceScopePolicy::schema_defaults()?.for_scope(model)?;
        let declaration = |kind, value| {
            model
                .constraint(kind)
                .map(|constraint| DeclaredReferencePolicySetting {
                    value,
                    origin: ReferencePolicyOverrideOrigin::Schema(constraint.source_map.clone()),
                })
        };
        Ok(Self {
            depth: declaration("reference-traversal-depth", policy.limits.max_depth),
            work: declaration("reference-traversal-work", policy.limits.max_work),
            unresolved: model.constraint(DISPOSITION).map(|constraint| {
                DeclaredReferencePolicySetting {
                    value: policy.unresolved,
                    origin: ReferencePolicyOverrideOrigin::Schema(constraint.source_map.clone()),
                }
            }),
        })
    }
    pub fn depth(&self) -> Option<&DeclaredReferencePolicySetting<usize>> {
        self.depth.as_ref()
    }
    pub fn work(&self) -> Option<&DeclaredReferencePolicySetting<usize>> {
        self.work.as_ref()
    }
    pub fn unresolved(&self) -> Option<&DeclaredReferencePolicySetting<ReferenceUnresolvedPolicy>> {
        self.unresolved.as_ref()
    }
    /// Combine local declarations in lexical order, preserving the nearest
    /// declaration's origin. Omitted local settings retain earlier declarations.
    pub fn overlay(&self, enclosing: &Self) -> Self {
        Self {
            depth: self.depth.clone().or_else(|| enclosing.depth.clone()),
            work: self.work.clone().or_else(|| enclosing.work.clone()),
            unresolved: self
                .unresolved
                .clone()
                .or_else(|| enclosing.unresolved.clone()),
        }
    }
    /// Replay declared local settings onto a new inherited child policy without
    /// replacing omitted facets or mutating either policy's source metadata.
    pub fn apply_to(&self, enclosing: &ReferenceScopePolicy) -> ReferenceScopePolicy {
        ReferenceScopePolicy {
            limits: ReferenceTraversalLimits {
                max_depth: self
                    .depth
                    .as_ref()
                    .map_or(enclosing.limits.max_depth, |setting| setting.value),
                max_work: self
                    .work
                    .as_ref()
                    .map_or(enclosing.limits.max_work, |setting| setting.value),
            },
            unresolved: self.unresolved.as_ref().map_or_else(
                || enclosing.unresolved.clone(),
                |setting| setting.value.clone(),
            ),
        }
    }
}
impl ReferenceScopePolicy {
    pub fn schema_defaults() -> Result<Self, ReferencePolicyError> {
        Ok(Self {
            limits: ReferenceTraversalLimits::schema_defaults()
                .map_err(|limit| error(&limit.to_string(), &SourceMapStack::default()))?,
            unresolved: ReferenceUnresolvedPolicy::schema_defaults()?,
        })
    }

    /// Validate all overrides before returning a new policy. An error leaves
    /// the enclosing policy unchanged, including its diagnostic definitions.
    pub fn for_scope(&self, model: &SchemaDocumentModel) -> Result<Self, ReferencePolicyError> {
        let unresolved = self.unresolved.for_scope(model)?;
        let limits = self.limits.for_scope(model).map_err(|limit| {
            let source = model
                .constraint(&limit.constraint)
                .map(|constraint| constraint.source_map.clone())
                .unwrap_or_default();
            error(&limit.to_string(), &source)
        })?;
        Ok(Self { limits, unresolved })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferencePolicyError {
    pub message: String,
    pub source_map: SourceMapStack,
}

impl fmt::Display for ReferencePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl std::error::Error for ReferencePolicyError {}

#[derive(Clone)]
struct StandardPolicies {
    default: ReferenceUnresolvedPolicy,
    mandatory: DiagnosticDefinition,
    warning: DiagnosticDefinition,
}

fn standard_policies() -> Result<&'static StandardPolicies, ReferencePolicyError> {
    static STANDARD: OnceLock<Result<StandardPolicies, ReferencePolicyError>> = OnceLock::new();
    STANDARD
        .get_or_init(|| {
            let source = builtin_schema_package_source("cem-ml").expect("embedded CEM-ML schema");
            let model = compile_schema_document_model(CEM_ML_SCHEMA_URI, source.schema_source);
            let constraint = model.constraint(DISPOSITION).ok_or_else(|| {
                error(
                    "Embedded schema omits the unresolved-reference fallback",
                    &SourceMapStack::default(),
                )
            })?;
            let disposition =
                parse_disposition(constraint.value.as_deref(), &constraint.source_map)?;
            let mandatory = model
                .diagnostics
                .get("cem.reference.unresolved_required")
                .ok_or_else(|| {
                    error(
                        "Embedded schema omits the mandatory diagnostic",
                        &constraint.source_map,
                    )
                })?
                .clone();
            let warning = model
                .diagnostics
                .get("cem.reference.unresolved_warning")
                .ok_or_else(|| {
                    error(
                        "Embedded schema omits the warning diagnostic",
                        &constraint.source_map,
                    )
                })?
                .clone();
            validate_severity(UnresolvedDisposition::Mandatory, &mandatory)?;
            validate_severity(UnresolvedDisposition::Warning, &warning)?;
            let diagnostic = match disposition {
                UnresolvedDisposition::Mandatory => Some(mandatory.clone()),
                UnresolvedDisposition::Warning => Some(warning.clone()),
                _ => None,
            };
            Ok(StandardPolicies {
                default: ReferenceUnresolvedPolicy {
                    disposition,
                    diagnostic,
                },
                mandatory,
                warning,
            })
        })
        .as_ref()
        .map_err(Clone::clone)
}

impl ReferenceUnresolvedPolicy {
    /// Explicit host policy using the schema-owned standard diagnostic definitions.
    pub fn standard(disposition: UnresolvedDisposition) -> Result<Self, ReferencePolicyError> {
        let diagnostic = match disposition {
            UnresolvedDisposition::Mandatory => Some(standard_policies()?.mandatory.clone()),
            UnresolvedDisposition::Warning => Some(standard_policies()?.warning.clone()),
            _ => None,
        };
        Ok(Self {
            disposition,
            diagnostic,
        })
    }
    pub fn schema_defaults() -> Result<Self, ReferencePolicyError> {
        Ok(standard_policies()?.default.clone())
    }

    pub fn disposition(&self) -> UnresolvedDisposition {
        self.disposition
    }

    /// An omitted constraint inherits the complete enclosing policy. A declared
    /// constraint replaces it; `neutral` can explicitly clear a mandatory rule.
    /// Custom diagnostic references resolve within the declaring schema model.
    pub fn for_scope(&self, model: &SchemaDocumentModel) -> Result<Self, ReferencePolicyError> {
        if let Some(duplicate) = model
            .compile_diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DUPLICATE_POLICY_CODE)
        {
            return Err(error(
                &duplicate.message,
                duplicate
                    .source_map
                    .as_ref()
                    .unwrap_or(&SourceMapStack::default()),
            ));
        }
        let Some(constraint) = model.constraint(DISPOSITION) else {
            return Ok(self.clone());
        };
        if constraint
            .target
            .as_deref()
            .is_some_and(|target| target != "reference")
        {
            return Err(error(
                "Unresolved-reference disposition must target reference nodes",
                &constraint.source_map,
            ));
        }
        let disposition = parse_disposition(constraint.value.as_deref(), &constraint.source_map)?;
        let emits = matches!(
            disposition,
            UnresolvedDisposition::Mandatory | UnresolvedDisposition::Warning
        );
        if !emits && constraint.diagnostic.is_some() {
            return Err(error(
                "Neutral/ignore reference policies cannot declare a diagnostic",
                &constraint.source_map,
            ));
        }
        let diagnostic = if let Some(code) = &constraint.diagnostic {
            Some(
                model
                    .diagnostics
                    .get(code)
                    .ok_or_else(|| {
                        error(
                            &format!("Unresolved-reference diagnostic `{code}` is not declared"),
                            &constraint.source_map,
                        )
                    })?
                    .clone(),
            )
        } else {
            match disposition {
                UnresolvedDisposition::Mandatory => Some(standard_policies()?.mandatory.clone()),
                UnresolvedDisposition::Warning => Some(standard_policies()?.warning.clone()),
                _ => None,
            }
        };
        if let Some(diagnostic) = &diagnostic {
            validate_severity(disposition, diagnostic)?;
        }
        Ok(Self {
            disposition,
            diagnostic,
        })
    }

    /// Apply only to a fact that the consumer has classified as unresolved.
    /// Pending/invalid outcomes and resolved-empty cardinality are separate.
    pub fn apply(&self, fact: &UnresolvedReferenceFact) -> UnresolvedReferenceTreatment {
        let diagnostic =
            self.diagnostic.as_ref().map(|definition| Diagnostic {
                code: definition.code.clone(),
                severity: definition.severity,
                message: format!(
                    "{}: {} ({})",
                    definition.message.as_deref().unwrap_or(&definition.code),
                    fact.occurrence
                        .expression
                        .as_deref()
                        .unwrap_or(&fact.occurrence.identity),
                    fact.reason
                ),
                node: Some(fact.occurrence.identity.clone()),
                byte_offset: fact.occurrence.source_map.origin().and_then(|frame| {
                    match &frame.span {
                        FrameSpan::Single(range) => Some(range.start),
                        FrameSpan::Multi(ranges) => ranges.first().map(|range| range.start),
                    }
                }),
                source_map: Some(fact.occurrence.source_map.clone()),
                ..Diagnostic::default()
            });
        UnresolvedReferenceTreatment {
            fact: fact.clone(),
            disposition: self.disposition,
            failed: self.disposition == UnresolvedDisposition::Mandatory,
            diagnostic,
        }
    }
}

fn parse_disposition(
    value: Option<&str>,
    source: &SourceMapStack,
) -> Result<UnresolvedDisposition, ReferencePolicyError> {
    match value {
        Some("neutral") => Ok(UnresolvedDisposition::Neutral),
        Some("mandatory") => Ok(UnresolvedDisposition::Mandatory),
        Some("warning") => Ok(UnresolvedDisposition::Warning),
        Some("ignore") => Ok(UnresolvedDisposition::Ignore),
        _ => Err(error(
            "Reference disposition requires neutral, mandatory, warning or ignore",
            source,
        )),
    }
}

fn validate_severity(
    disposition: UnresolvedDisposition,
    diagnostic: &DiagnosticDefinition,
) -> Result<(), ReferencePolicyError> {
    let valid = match disposition {
        UnresolvedDisposition::Mandatory => diagnostic.severity.is_hard_violation(),
        UnresolvedDisposition::Warning => diagnostic.severity == Severity::Warning,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(error(
            "Reference diagnostic severity conflicts with disposition",
            &diagnostic.source_map,
        ))
    }
}

fn error(message: &str, source: &SourceMapStack) -> ReferencePolicyError {
    ReferencePolicyError {
        message: message.into(),
        source_map: source.clone(),
    }
}
