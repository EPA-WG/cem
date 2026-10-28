//! Compiler imports use the exact immutable contract admitted for source loading.
use super::*;
use crate::{engine::EngineContext, schema::parser_contracts::CompiledParserContract};
use std::sync::Arc;

/// Select existing native authority; never construct it from a control record.
/// An inactive/invalid explicit candidate cannot fall back to embedded bodies.
pub fn xslt_parser_contract_from_context(
    context: &EngineContext,
    source_uri: &str,
) -> Result<Option<Arc<CompiledParserContract>>, Vec<Diagnostic>> {
    #[cfg(feature = "external-schema-assets")]
    if let Some(message) = crate::schema::package_sources::missing_external_source(
        Some(context),
        Some(XSLT_SCHEMA_URI),
        Some(XSLT_CONTENT_TYPE),
    ) {
        return Err(vec![failure(
            source_uri,
            "cem.schema_asset.missing",
            message,
        )]);
    }
    let models = &context.schema_document_models;
    let model = models
        .resolve_for_identity(Some(XSLT_SCHEMA_URI), None, None)
        .or_else(|| models.inspect_for_identity(Some(XSLT_SCHEMA_URI), None, None));
    let Some(model) = model else { return Ok(None) };
    if model.is_ready_for_validation()
        && !model
            .compile_diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
    {
        return Ok(model.parser_contract.clone());
    }
    let mut diagnostics = model.validation_blocker_diagnostics();
    for diagnostic in &mut diagnostics {
        if diagnostic.uri.is_none() {
            diagnostic.uri = Some(source_uri.into());
        }
    }
    Err(diagnostics)
}

/// Parse authoring source with exact preflighted import edges. This grants no
/// I/O, document() or result-document authority. Other parser kinds refuse.
pub fn xslt_stylesheet_ast_from_source_bytes_with_modules_and_parser_contract(
    request: XsltSourceValidationRequest<'_>,
    hrefs: &[&str],
    contract: Option<&CompiledParserContract>,
) -> (Option<XsltStylesheetAst>, Vec<Diagnostic>) {
    let Some(contract) = contract else {
        return xslt_stylesheet_ast_from_source_bytes_with_modules(request, hrefs);
    };
    let uri = request.source_uri;
    contract
        .parse_xslt_with_modules(request, hrefs)
        .unwrap_or_else(|| {
            (
                None,
                vec![failure(
                    uri,
                    crate::schema::document_model::MODEL_NOT_READY_CODE,
                    "XSLT compilation requires an original-owned XSLT parser contract".into(),
                )],
            )
        })
}

/// Discovery defers only the native external-URI fact until exact import edges
/// have been resolved. Original schemas may rename its diagnostic code.
pub fn xslt_diagnostic_requires_resolved_modules(diagnostic: &Diagnostic) -> bool {
    diagnostic
        .details
        .as_ref()
        .and_then(|details| details.pointer("/xslt/factKind"))
        .and_then(serde_json::Value::as_str)
        == Some(XsltFactKind::ExternalUriRejected.as_str())
}

fn failure(uri: &str, code: &str, message: String) -> Diagnostic {
    Diagnostic {
        uri: Some(uri.into()),
        code: code.into(),
        severity: Severity::Error,
        message,
        ..Default::default()
    }
}
