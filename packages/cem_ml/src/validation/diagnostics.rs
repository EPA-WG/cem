//! Schema-owned diagnostics for CEM-ML semantic validation facts.
//!
//! Validation rules extract document/AST facts. This module interprets those
//! facts through `schema/cem-ml-generic.cem` bindings so schema packages own
//! diagnostic code, severity, behavior, and policy for semantic rules without
//! routing those concerns through parser diagnostics.

use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::registry::CEM_ML_SCHEMA_URI;
use crate::source_map::SourceMapStack;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub(crate) const CEM_ML_PACKAGE_ID: &str = "cem-ml";
pub(crate) const CEM_ML_SEMANTIC_REPORT_BEHAVIOR: &str = "cem-ml-semantic-report-fact";

#[cfg(test)]
pub(crate) const SEMANTIC_UNBOUND_PREFIX_CONTRACT: &str = "semantic-unbound-prefix";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CemMlSemanticFact {
    pub kind: CemMlSemanticFactKind,
    pub byte_offset: Option<u64>,
    pub message: String,
    pub source_map: Option<SourceMapStack>,
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CemMlSemanticFactKind {
    UnboundPrefix,
}

impl CemMlSemanticFactKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::UnboundPrefix => "semantic-unbound-prefix",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CemMlSemanticDiagnosticCatalog {
    fact_bindings: BTreeMap<String, CemMlSemanticDiagnosticBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CemMlSemanticDiagnosticBinding {
    pub fact_kind: String,
    pub contract: String,
    pub behavior: Option<String>,
    pub diagnostic_code: String,
    pub severity: Severity,
    pub policy: Option<String>,
}

impl CemMlSemanticDiagnosticCatalog {
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
                if constraint.behavior.as_deref()?.trim() != CEM_ML_SEMANTIC_REPORT_BEHAVIOR {
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
                    CemMlSemanticDiagnosticBinding {
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
        kind: CemMlSemanticFactKind,
    ) -> Option<&CemMlSemanticDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

pub(crate) fn builtin_cem_ml_semantic_diagnostic_catalog() -> &'static CemMlSemanticDiagnosticCatalog
{
    static CATALOG: OnceLock<CemMlSemanticDiagnosticCatalog> = OnceLock::new();
    CATALOG.get_or_init(CemMlSemanticDiagnosticCatalog::from_builtin)
}

pub(crate) fn cem_ml_semantic_fact_diagnostic(
    fact: &CemMlSemanticFact,
    catalog: Option<&CemMlSemanticDiagnosticCatalog>,
) -> Option<Diagnostic> {
    let binding = if let Some(catalog) = catalog {
        catalog.binding_for_fact(fact.kind).cloned()
    } else {
        builtin_cem_ml_semantic_diagnostic_catalog()
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
        details: Some(json!({
            "contract": binding.contract,
            "behavior": binding.behavior,
            "factKind": fact.kind.as_str(),
            "policy": binding.policy,
            "semantic": fact.details,
            "sourceRange": {
                "byteOffset": fact.byte_offset,
            },
        })),
        source_map: fact.source_map.clone(),
    })
}
