//! Schema-owned diagnostics for CEM-ML parser facts.
//!
//! Native tokenizer/parser layers extract byte-accurate facts. This module
//! interprets those facts through `schema/cem-ml-generic.cem` bindings so the
//! schema package owns diagnostic code, severity, behavior, and policy.

use crate::diagnostics::{Diagnostic, Severity};
use crate::parser::format;
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::registry::CEM_ML_SCHEMA_URI;
use crate::source_map::SourceMapStack;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub(crate) const CEM_ML_PACKAGE_ID: &str = "cem-ml";
pub(crate) const CEM_ML_AST_REPORT_BEHAVIOR: &str = "cem-ml-ast-report-fact";
pub(crate) const CEM_ML_DOC_REPORT_BEHAVIOR: &str = "cem-ml-doc-report-fact";
pub(crate) const CEM_ML_TOKENIZER_REPORT_BEHAVIOR: &str = "cem-ml-tokenizer-report-fact";

#[cfg(test)]
pub(crate) const AST_UNBALANCED_CLOSE_CONTRACT: &str = "ast-unbalanced-close";
#[cfg(test)]
pub(crate) const AST_UNCLOSED_SCOPE_CONTRACT: &str = "ast-unclosed-scope";
#[cfg(test)]
pub(crate) const AST_UNRESOLVED_REFERENCE_CONTRACT: &str = "ast-unresolved-reference";
#[cfg(test)]
pub(crate) const DOC_VERSION_MISSING_CONTRACT: &str = "doc-version-missing";
#[cfg(test)]
pub(crate) const DOC_SEMVER_INVALID_CONTRACT: &str = "doc-semver-invalid";
#[cfg(test)]
pub(crate) const DOC_FORMAT_UNKNOWN_CONTRACT: &str = "doc-format-unknown";
#[cfg(test)]
pub(crate) const DOC_VERSION_UNSUPPORTED_CONTRACT: &str = "doc-version-unsupported";
#[cfg(test)]
pub(crate) const DOC_PRERELEASE_UNMATCHED_CONTRACT: &str = "doc-prerelease-unmatched";
#[cfg(test)]
pub(crate) const DOC_VERSION_RESOLVED_CONTRACT: &str = "doc-version-resolved";
#[cfg(test)]
pub(crate) const TOKENIZER_UNTERMINATED_BLOCK_COMMENT_CONTRACT: &str =
    "tokenizer-unterminated-block-comment";
#[cfg(test)]
pub(crate) const TOKENIZER_BARE_BRACE_TEXT_CONTRACT: &str = "tokenizer-bare-brace-text";
#[cfg(test)]
pub(crate) const TOKENIZER_UNTERMINATED_STRING_CONTRACT: &str = "tokenizer-unterminated-string";
#[cfg(test)]
pub(crate) const TOKENIZER_UNTERMINATED_AVT_SPAN_CONTRACT: &str = "tokenizer-unterminated-avt-span";
#[cfg(test)]
pub(crate) const TOKENIZER_UNTERMINATED_NODE_CONTRACT: &str = "tokenizer-unterminated-node";
#[cfg(test)]
pub(crate) const TOKENIZER_INVALID_PROCESSING_INSTRUCTION_CONTRACT: &str =
    "tokenizer-invalid-processing-instruction";
#[cfg(test)]
pub(crate) const TOKENIZER_UNTERMINATED_PROCESSING_INSTRUCTION_CONTRACT: &str =
    "tokenizer-unterminated-processing-instruction";
#[cfg(test)]
pub(crate) const TOKENIZER_UNTERMINATED_EXPRESSION_CONTRACT: &str =
    "tokenizer-unterminated-expression";
#[cfg(test)]
pub(crate) const TOKENIZER_UNTERMINATED_RICH_CONTENT_CONTRACT: &str =
    "tokenizer-unterminated-rich-content";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CemMlParserFact {
    pub kind: CemMlParserFactKind,
    pub byte_offset: Option<u64>,
    pub message: String,
    pub source_map: Option<SourceMapStack>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CemMlParserFactKind {
    AstUnbalancedClose,
    AstUnclosedScope,
    AstUnresolvedReference,
    DocVersionMissing,
    DocSemverInvalid,
    DocFormatUnknown,
    DocVersionUnsupported,
    DocPrereleaseUnmatched,
    DocVersionResolved,
    TokenizerUnterminatedBlockComment,
    TokenizerBareBraceText,
    TokenizerUnterminatedString,
    TokenizerUnterminatedAvtSpan,
    TokenizerUnterminatedNode,
    TokenizerInvalidProcessingInstruction,
    TokenizerUnterminatedProcessingInstruction,
    TokenizerUnterminatedExpression,
    TokenizerUnterminatedRichContent,
}

impl CemMlParserFactKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::AstUnbalancedClose => "ast-unbalanced-close",
            Self::AstUnclosedScope => "ast-unclosed-scope",
            Self::AstUnresolvedReference => "ast-unresolved-reference",
            Self::DocVersionMissing => "doc-version-missing",
            Self::DocSemverInvalid => "doc-semver-invalid",
            Self::DocFormatUnknown => "doc-format-unknown",
            Self::DocVersionUnsupported => "doc-version-unsupported",
            Self::DocPrereleaseUnmatched => "doc-prerelease-unmatched",
            Self::DocVersionResolved => "doc-version-resolved",
            Self::TokenizerUnterminatedBlockComment => "tokenizer-unterminated-block-comment",
            Self::TokenizerBareBraceText => "tokenizer-bare-brace-text",
            Self::TokenizerUnterminatedString => "tokenizer-unterminated-string",
            Self::TokenizerUnterminatedAvtSpan => "tokenizer-unterminated-avt-span",
            Self::TokenizerUnterminatedNode => "tokenizer-unterminated-node",
            Self::TokenizerInvalidProcessingInstruction => {
                "tokenizer-invalid-processing-instruction"
            }
            Self::TokenizerUnterminatedProcessingInstruction => {
                "tokenizer-unterminated-processing-instruction"
            }
            Self::TokenizerUnterminatedExpression => "tokenizer-unterminated-expression",
            Self::TokenizerUnterminatedRichContent => "tokenizer-unterminated-rich-content",
        }
    }

    pub(crate) fn from_doc_directive_error(error: &format::DocDirectiveError) -> Self {
        match error {
            format::DocDirectiveError::SemverInvalid { .. } => Self::DocSemverInvalid,
            format::DocDirectiveError::FormatUnknown { .. } => Self::DocFormatUnknown,
            format::DocDirectiveError::VersionUnsupported { .. } => Self::DocVersionUnsupported,
            format::DocDirectiveError::PrereleaseUnmatched { .. } => Self::DocPrereleaseUnmatched,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CemMlParserDiagnosticCatalog {
    fact_bindings: BTreeMap<String, CemMlParserDiagnosticBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CemMlParserDiagnosticBinding {
    pub fact_kind: String,
    pub contract: String,
    pub behavior: Option<String>,
    pub diagnostic_code: String,
    pub severity: Severity,
    pub policy: Option<String>,
}

impl CemMlParserDiagnosticCatalog {
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
                let behavior = constraint.behavior.as_deref()?.trim();
                if !matches!(
                    behavior,
                    CEM_ML_AST_REPORT_BEHAVIOR
                        | CEM_ML_DOC_REPORT_BEHAVIOR
                        | CEM_ML_TOKENIZER_REPORT_BEHAVIOR
                ) {
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
                    CemMlParserDiagnosticBinding {
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
        kind: CemMlParserFactKind,
    ) -> Option<&CemMlParserDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

pub(crate) fn builtin_cem_ml_parser_diagnostic_catalog() -> &'static CemMlParserDiagnosticCatalog {
    static CATALOG: OnceLock<CemMlParserDiagnosticCatalog> = OnceLock::new();
    CATALOG.get_or_init(CemMlParserDiagnosticCatalog::from_builtin)
}

pub(crate) fn cem_ml_parser_fact_diagnostic(
    fact: &CemMlParserFact,
    catalog: Option<&CemMlParserDiagnosticCatalog>,
) -> Option<Diagnostic> {
    let binding = if let Some(catalog) = catalog {
        catalog.binding_for_fact(fact.kind).cloned()
    } else {
        builtin_cem_ml_parser_diagnostic_catalog()
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
            "sourceRange": {
                "byteOffset": fact.byte_offset,
            },
        })),
        source_map: fact.source_map.clone(),
    })
}
