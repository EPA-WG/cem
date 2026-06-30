//! Host → cem-ql template-embedding bridge for the CLI (AC-T-7).
//!
//! `cem_ml`'s engine returns parsed AST + diagnostics, but the cem-ql
//! parse of embedded expressions (`{$ ... }` content expressions,
//! whole-expression attributes, template-aware `{...}` AVT spans) is
//! deferred to this layer. The CLI tokenizes the input independently,
//! hands the token stream to `cem_ql::template::extract_embeddings`,
//! compiles each embedding through cem-ql's front end, and surfaces
//! the resulting diagnostics alongside the engine's diagnostics. The
//! primary JSON projection is unchanged.

use cem_ml::diagnostics::Diagnostic;
use cem_ml::engine::{FormatIdentity, InputFormat};
use cem_ml::schema::registry::{
    content_type_essence, CEM_NATIVE_TEMPLATE_CONTENT_TYPE, CEM_NATIVE_TEMPLATE_SCHEMA_URI,
    CEM_TRANSFORM_CONTENT_TYPE, CEM_TRANSFORM_SCHEMA_URI,
};
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;
use cem_ml::tokenizer::{SchemaToken, SchemaTokenizer};

use cem_ql::api::CompileContext;
use cem_ql::render::{compile_template, CompileTemplateOptions};
use cem_ql::template::{
    compile_embedding, extract_embeddings, DefaultAttributeClassifier, EmbeddedExpression,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct TemplatePassIdentity<'a> {
    pub content_type: Option<&'a str>,
    pub schema: Option<&'a str>,
}

impl<'a> From<Option<&'a FormatIdentity>> for TemplatePassIdentity<'a> {
    fn from(identity: Option<&'a FormatIdentity>) -> Self {
        Self {
            content_type: identity.and_then(|identity| identity.content_type.as_deref()),
            schema: identity.and_then(|identity| identity.schema.as_deref()),
        }
    }
}

/// Run the cem-ql template pass against `bytes` when the input format
/// is CEM-native; HTML / XML inputs do not host cem-ql embeddings (the
/// canonical CEM-ML surface owns the `$` / template-attribute
/// vocabulary per AC-T-7) and short-circuit to an empty vector.
pub fn run_with_identity(
    bytes: &[u8],
    from_format: InputFormat,
    uri: Option<&str>,
    identity: TemplatePassIdentity<'_>,
) -> Vec<Diagnostic> {
    if !matches!(from_format, InputFormat::Cem) {
        return Vec::new();
    }
    if is_template_family_identity(identity) {
        return run_context_aware_template_pass(bytes, uri);
    }
    run_raw_embedding_pass(bytes, uri)
}

fn run_raw_embedding_pass(bytes: &[u8], uri: Option<&str>) -> Vec<Diagnostic> {
    let tokens = tokenize(bytes);
    let classifier = DefaultAttributeClassifier;
    let mut diagnostics = Vec::new();
    let ctx = CompileContext::default();
    for embedding in extract_embeddings(&tokens, &classifier) {
        let (_, diags) = compile_embedding(&embedding, &ctx);
        for diagnostic in diags {
            diagnostics.push(annotate_uri(diagnostic, uri, &embedding));
        }
    }
    diagnostics
}

fn run_context_aware_template_pass(bytes: &[u8], uri: Option<&str>) -> Vec<Diagnostic> {
    let source = std::str::from_utf8(bytes).unwrap_or("");
    let artifact = compile_template(
        source,
        &CompileTemplateOptions {
            host_bindings: Vec::new(),
        },
    );
    artifact
        .diagnostics
        .into_iter()
        .map(|diagnostic| annotate_template_uri(diagnostic, uri))
        .collect()
}

fn is_template_family_identity(identity: TemplatePassIdentity<'_>) -> bool {
    identity.schema.is_some_and(|schema| {
        matches!(
            schema.trim(),
            CEM_NATIVE_TEMPLATE_SCHEMA_URI | CEM_TRANSFORM_SCHEMA_URI
        )
    }) || identity.content_type.is_some_and(|content_type| {
        matches!(
            content_type_essence(content_type).as_str(),
            CEM_NATIVE_TEMPLATE_CONTENT_TYPE | CEM_TRANSFORM_CONTENT_TYPE
        )
    })
}

fn tokenize(bytes: &[u8]) -> Vec<SchemaToken> {
    let src = BytesSource::new(SourceId(1), bytes.to_vec());
    let mut tokenizer = CemTokenizer::from_source(src);
    let _ = tokenizer.take_diagnostics();
    let mut out = Vec::new();
    while let Some(token) = tokenizer.next_token() {
        out.push(token);
    }
    out
}

fn annotate_uri(
    mut diagnostic: Diagnostic,
    uri: Option<&str>,
    _embedding: &EmbeddedExpression,
) -> Diagnostic {
    if diagnostic.uri.is_none() {
        diagnostic.uri = uri.map(str::to_owned);
    }
    diagnostic
}

fn annotate_template_uri(mut diagnostic: Diagnostic, uri: Option<&str>) -> Diagnostic {
    if diagnostic.uri.is_none() {
        diagnostic.uri = uri.map(str::to_owned);
    }
    diagnostic
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_cem_pass_still_reports_broken_embeddings() {
        let diagnostics = run_with_identity(
            b"{p | {$ 1 + }}",
            InputFormat::Cem,
            Some("broken.cem"),
            TemplatePassIdentity::default(),
        );

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.starts_with("cem.ql.")));
        assert!(diagnostics
            .iter()
            .all(|diagnostic| diagnostic.uri.as_deref() == Some("broken.cem")));
    }

    #[test]
    fn transform_pass_uses_context_aware_template_bindings() {
        let diagnostics = run_with_identity(
            include_bytes!(
                "../../cem_ml/schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt"
            ),
            InputFormat::Cem,
            Some("dom-to-html.cemt"),
            TemplatePassIdentity {
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI),
            },
        );

        assert!(
            diagnostics.is_empty(),
            "context-aware CEMT validation should accept loop/call bindings: {diagnostics:?}"
        );
    }
}
