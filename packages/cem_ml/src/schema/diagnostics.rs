//! Schema-owned diagnostics for CEM-ML schema-machine facts.
//!
//! The schema machine extracts stream facts while `schema/cem-ml-generic.cem`
//! owns the diagnostic code, severity, behavior, and policy attached to those
//! facts. This keeps schema validation and handoff reporting on the same
//! neutral-fact boundary as the parser and document-validation lints.

use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::registry::CEM_ML_SCHEMA_URI;
use crate::source_map::SourceMapStack;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub(crate) const CEM_ML_PACKAGE_ID: &str = "cem-ml";
pub(crate) const CEM_ML_SCHEMA_REPORT_BEHAVIOR: &str = "cem-ml-schema-report-fact";

#[cfg(test)]
pub(crate) const SCHEMA_UNBALANCED_CLOSE_CONTRACT: &str = "schema-unbalanced-close";
#[cfg(test)]
pub(crate) const SCHEMA_UNCLOSED_SCOPE_CONTRACT: &str = "schema-unclosed-scope";
#[cfg(test)]
pub(crate) const SCHEMA_UNRESOLVED_NAMESPACE_REJECT_CONTRACT: &str =
    "schema-unresolved-namespace-reject";
#[cfg(test)]
pub(crate) const SCHEMA_UNRESOLVED_NAMESPACE_ALLOW_CONTRACT: &str =
    "schema-unresolved-namespace-allow";
#[cfg(test)]
pub(crate) const SCHEMA_UNRESOLVED_NAMESPACE_IGNORE_CONTRACT: &str =
    "schema-unresolved-namespace-ignore";
#[cfg(test)]
pub(crate) const HANDOFF_XSLT_DISPATCHED_CONTRACT: &str = "handoff-xslt-dispatched";
#[cfg(test)]
pub(crate) const HANDOFF_XSLT_VERSION_INVALID_CONTRACT: &str = "handoff-xslt-version-invalid";
#[cfg(test)]
pub(crate) const HANDOFF_CHILD_PARSER_DEFERRED_CONTRACT: &str = "handoff-child-parser-deferred";
#[cfg(test)]
pub(crate) const HANDOFF_UNSUPPORTED_CONTENT_TYPE_CONTRACT: &str =
    "handoff-unsupported-content-type";
#[cfg(test)]
pub(crate) const SCHEMA_UNKNOWN_ANNOTATION_CONTRACT: &str = "schema-unknown-annotation";
#[cfg(test)]
pub(crate) const SCHEMA_UNKNOWN_ANNOTATION_VALUE_CONTRACT: &str = "schema-unknown-annotation-value";
#[cfg(test)]
pub(crate) const SCHEMA_DISALLOWED_STATE_CONTRACT: &str = "schema-disallowed-state";
#[cfg(test)]
pub(crate) const SCHEMA_STATE_NOT_ALLOWED_FOR_ROLE_CONTRACT: &str =
    "schema-state-not-allowed-for-role";
#[cfg(test)]
pub(crate) const SCHEMA_SCOPING_EXCLUSIVE_SRC_SELECT_CONTRACT: &str =
    "schema-scoping-exclusive-src-select";
#[cfg(test)]
pub(crate) const SCHEMA_SCOPING_MISSING_SOURCE_CONTRACT: &str = "schema-scoping-missing-source";
#[cfg(test)]
pub(crate) const SCHEMA_INVALID_NS_DIRECTIVE_CONTRACT: &str = "schema-invalid-ns-directive";
#[cfg(test)]
pub(crate) const SCHEMA_UNSUPPORTED_CONSTRAINT_CONTRACT: &str = "schema-unsupported-constraint";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CemMlSchemaFact {
    pub kind: CemMlSchemaFactKind,
    pub byte_offset: Option<u64>,
    pub message: String,
    pub source_map: Option<SourceMapStack>,
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CemMlSchemaFactKind {
    SchemaUnbalancedClose,
    SchemaUnclosedScope,
    SchemaUnresolvedNamespaceReject,
    SchemaUnresolvedNamespaceAllow,
    SchemaUnresolvedNamespaceIgnore,
    HandoffXsltDispatched,
    HandoffXsltVersionInvalid,
    HandoffChildParserDeferred,
    HandoffUnsupportedContentType,
    SchemaUnknownAnnotation,
    SchemaUnknownAnnotationValue,
    SchemaDisallowedState,
    SchemaStateNotAllowedForRole,
    SchemaScopingExclusiveSrcSelect,
    SchemaScopingMissingSource,
    SchemaInvalidNsDirective,
    SchemaUnsupportedConstraint,
}

impl CemMlSchemaFactKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SchemaUnbalancedClose => "schema-unbalanced-close",
            Self::SchemaUnclosedScope => "schema-unclosed-scope",
            Self::SchemaUnresolvedNamespaceReject => "schema-unresolved-namespace-reject",
            Self::SchemaUnresolvedNamespaceAllow => "schema-unresolved-namespace-allow",
            Self::SchemaUnresolvedNamespaceIgnore => "schema-unresolved-namespace-ignore",
            Self::HandoffXsltDispatched => "handoff-xslt-dispatched",
            Self::HandoffXsltVersionInvalid => "handoff-xslt-version-invalid",
            Self::HandoffChildParserDeferred => "handoff-child-parser-deferred",
            Self::HandoffUnsupportedContentType => "handoff-unsupported-content-type",
            Self::SchemaUnknownAnnotation => "schema-unknown-annotation",
            Self::SchemaUnknownAnnotationValue => "schema-unknown-annotation-value",
            Self::SchemaDisallowedState => "schema-disallowed-state",
            Self::SchemaStateNotAllowedForRole => "schema-state-not-allowed-for-role",
            Self::SchemaScopingExclusiveSrcSelect => "schema-scoping-exclusive-src-select",
            Self::SchemaScopingMissingSource => "schema-scoping-missing-source",
            Self::SchemaInvalidNsDirective => "schema-invalid-ns-directive",
            Self::SchemaUnsupportedConstraint => "schema-unsupported-constraint",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CemMlSchemaDiagnosticCatalog {
    fact_bindings: BTreeMap<String, CemMlSchemaDiagnosticBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CemMlSchemaDiagnosticBinding {
    pub fact_kind: String,
    pub contract: String,
    pub behavior: Option<String>,
    pub diagnostic_code: String,
    pub severity: Severity,
    pub policy: Option<String>,
}

impl CemMlSchemaDiagnosticCatalog {
    pub(crate) fn from_builtin() -> Self {
        let source =
            crate::schema::package_sources::builtin_schema_package_source(CEM_ML_PACKAGE_ID)
                .expect("built-in CEM-ML schema package source must be registered");
        Self::from_schema_source(source.schema_source)
    }

    pub(crate) fn from_schema_source(schema_source: &str) -> Self {
        let model = compile_schema_document_model(CEM_ML_SCHEMA_URI, schema_source);
        let fact_bindings = model
            .constraints
            .values()
            .filter_map(|constraint| {
                if constraint.behavior.as_deref()?.trim() != CEM_ML_SCHEMA_REPORT_BEHAVIOR {
                    return None;
                }
                let fact_kind = constraint.fact_kind.as_deref()?.trim();
                if fact_kind.is_empty() {
                    return None;
                }
                let diagnostic_code = constraint.diagnostic.as_deref()?.trim();
                if diagnostic_code.is_empty() {
                    return None;
                }
                let diagnostic = model.diagnostics.get(diagnostic_code)?;
                Some((
                    fact_kind.to_owned(),
                    CemMlSchemaDiagnosticBinding {
                        fact_kind: fact_kind.to_owned(),
                        contract: constraint.kind.clone(),
                        behavior: constraint.behavior.clone(),
                        diagnostic_code: diagnostic.code.clone(),
                        severity: diagnostic.severity,
                        policy: constraint.policy.clone(),
                    },
                ))
            })
            .collect();

        Self { fact_bindings }
    }

    pub(crate) fn binding_for_fact(
        &self,
        kind: CemMlSchemaFactKind,
    ) -> Option<&CemMlSchemaDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

pub(crate) fn builtin_cem_ml_schema_diagnostic_catalog() -> &'static CemMlSchemaDiagnosticCatalog {
    static CATALOG: OnceLock<CemMlSchemaDiagnosticCatalog> = OnceLock::new();
    CATALOG.get_or_init(CemMlSchemaDiagnosticCatalog::from_builtin)
}

pub(crate) fn cem_ml_schema_fact_diagnostic(
    fact: &CemMlSchemaFact,
    catalog: Option<&CemMlSchemaDiagnosticCatalog>,
) -> Option<Diagnostic> {
    let binding = if let Some(catalog) = catalog {
        catalog.binding_for_fact(fact.kind).cloned()
    } else {
        builtin_cem_ml_schema_diagnostic_catalog()
            .binding_for_fact(fact.kind)
            .cloned()
    }?;
    Some(Diagnostic {
        uri: None,
        line: None,
        column: None,
        byte_offset: fact.byte_offset,
        code: binding.diagnostic_code.clone(),
        severity: binding.severity,
        message: fact.message.clone(),
        node: None,
        details: Some(schema_fact_details(&binding, fact)),
        source_map: fact.source_map.clone(),
    })
}

fn schema_fact_details(binding: &CemMlSchemaDiagnosticBinding, fact: &CemMlSchemaFact) -> Value {
    let mut details = Map::new();
    details.insert("contract".to_owned(), json!(binding.contract));
    details.insert("behavior".to_owned(), json!(binding.behavior));
    details.insert("factKind".to_owned(), json!(fact.kind.as_str()));
    details.insert("policy".to_owned(), json!(binding.policy));

    match fact.details.as_ref() {
        Some(Value::Object(extra)) => {
            for (key, value) in extra {
                details.insert(key.clone(), value.clone());
            }
        }
        Some(value) => {
            details.insert("schema".to_owned(), value.clone());
        }
        None => {}
    }

    match details.get_mut("sourceRange") {
        Some(Value::Object(source_range)) => {
            source_range
                .entry("byteOffset".to_owned())
                .or_insert_with(|| json!(fact.byte_offset));
        }
        Some(_) => {}
        None => {
            details.insert(
                "sourceRange".to_owned(),
                json!({
                    "byteOffset": fact.byte_offset,
                }),
            );
        }
    }

    Value::Object(details)
}
