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
use cem_ml::engine::InputFormat;
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;
use cem_ml::tokenizer::{SchemaToken, SchemaTokenizer};

use cem_ql::api::CompileContext;
use cem_ql::template::{
    compile_embedding, extract_embeddings, DefaultAttributeClassifier, EmbeddedExpression,
};

/// Run the cem-ql template pass against `bytes` when the input format
/// is CEM-native; HTML / XML inputs do not host cem-ql embeddings (the
/// canonical CEM-ML surface owns the `$` / template-attribute
/// vocabulary per AC-T-7) and short-circuit to an empty vector.
pub fn run(bytes: &[u8], from_format: InputFormat, uri: Option<&str>) -> Vec<Diagnostic> {
    if !matches!(from_format, InputFormat::Cem) {
        return Vec::new();
    }
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
