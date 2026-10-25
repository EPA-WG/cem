//! Markdown parsing consumes the selected schema's retained diagnostic contract.
use super::*;
use crate::{
    engine::{EngineInput, FormatIdentity, InputFormat},
    lifecycle::{markdown_lifecycle_adapter_diagnostics, LoadedInput, LoadedInputAstStream},
    schema::{
        document_model::{SchemaDocumentModel, INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE},
        parser_contracts::NativeParserContract,
    },
};
use std::sync::Arc;

#[derive(Debug)]
struct MarkdownParserContract(MarkdownDiagnosticCatalog);

impl NativeParserContract for MarkdownParserContract {
    fn load(
        &self,
        input: &EngineInput,
        identity: &FormatIdentity,
        adapter_id: &'static str,
    ) -> LoadedInput {
        let content_type = identity
            .content_type
            .as_deref()
            .or(input.root_scope.default_content_type.as_deref())
            .unwrap_or(MARKDOWN_CONTENT_TYPE);
        let (document, diagnostics) = markdown_document_ast_from_source_bytes_with_contracts(
            MarkdownSourceValidationRequest {
                bytes: &input.bytes,
                source_uri: &input.uri,
                content_type: Some(content_type),
            },
            &self.0,
        );
        LoadedInput {
            bytes: input.bytes.clone(),
            from_format: input.from_format.unwrap_or(InputFormat::Cem),
            ast_stream: document.map(LoadedInputAstStream::MarkdownDocument),
            diagnostics: markdown_lifecycle_adapter_diagnostics(adapter_id, diagnostics),
            adapter_id: Some(adapter_id),
        }
    }
}

pub(crate) fn compile_parser_contract(
    model: &SchemaDocumentModel,
) -> (Option<Arc<dyn NativeParserContract>>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    // These are the closed facts emitted by the existing native Markdown
    // parser. Advisory severities remain schema-owned; unsafe source decoding
    // and embedded HTML must retain their hard refusal.
    for (code, hard) in [
        (MARKDOWN_UNSUPPORTED_ENCODING_CODE, true),
        (MARKDOWN_CHARSET_MISSING_CODE, false),
        (MARKDOWN_UNKNOWN_VARIANT_CODE, false),
        (MARKDOWN_EMBEDDED_HTML_REJECTED_CODE, true),
    ] {
        let declared = model.diagnostics.get(code);
        if declared.is_none()
            || declared.is_some_and(|diagnostic| {
                diagnostic.behavior.is_some() || hard && !diagnostic.severity.is_hard_violation()
            })
        {
            diagnostics.push(Diagnostic {
                code: INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE.into(),
                severity: Severity::Error,
                message: format!("Native Markdown parsing requires a declared{} diagnostic `{code}` without replacement behavior",
                    if hard { " hard" } else { "" }),
                details: Some(json!({"schemaUri": model.schema_uri, "checkKind": "markdown-parser-diagnostic-contract", "diagnostic": code})),
                ..Default::default()
            });
        }
    }
    let implementation = diagnostics.is_empty().then(|| {
        Arc::new(MarkdownParserContract(
            MarkdownDiagnosticCatalog::from_document_model(model),
        )) as Arc<dyn NativeParserContract>
    });
    (implementation, diagnostics)
}

#[cfg(test)]
mod tests;
