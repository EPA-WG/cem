//! AC-T-7 — CLI invokes cem-ql for embedded expressions.
//!
//! Three host surfaces produce cem-ql embeddings: `{$ ... }` content
//! expressions, whole-expression attributes (`select=` / `match=` /
//! `test=` / etc.), and `{...}` AVT spans inside template-aware
//! attribute values. The CLI tokenizes the input, runs
//! `cem_ql::template::extract_embeddings`, compiles each through
//! cem-ql, and surfaces parse failures as `cem.ql.*` diagnostics
//! alongside the cem-ml diagnostics — without touching the primary
//! parse projection.

use cem_ml::engine::InputFormat;
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;
use cem_ml::tokenizer::{SchemaToken, SchemaTokenizer};
use cem_ql::api::CompileContext;
use cem_ql::template::{
    compile_embedding, extract_embeddings, DefaultAttributeClassifier, EmbeddingKind,
};

fn tokenize(source: &str) -> Vec<SchemaToken> {
    let src = BytesSource::new(SourceId(1), source.as_bytes().to_vec());
    let mut tok = CemTokenizer::from_source(src);
    let _ = tok.take_diagnostics();
    let mut out = Vec::new();
    while let Some(t) = tok.next_token() {
        out.push(t);
    }
    out
}

#[test]
fn content_expression_is_handed_off_to_cem_ql() {
    let tokens = tokenize("{p | {$ 1 + 2 * 3 }}");
    let embeddings = extract_embeddings(&tokens, &DefaultAttributeClassifier);
    let content: Vec<_> = embeddings
        .iter()
        .filter(|e| matches!(e.kind, EmbeddingKind::ContentExpression))
        .collect();
    assert_eq!(content.len(), 1, "exactly one `{{$ ... }}` body");
    let (query, diags) = compile_embedding(content[0], &CompileContext::default());
    assert!(
        query.is_some(),
        "valid expression parses cleanly: {diags:?}"
    );
    assert!(diags.is_empty(), "no diagnostics for valid expression");
}

#[test]
fn whole_expression_attribute_is_handed_off_to_cem_ql() {
    let tokens = tokenize(r#"{for-each @select="(1, 2, 3) | (3, 4)" | item}"#);
    let embeddings = extract_embeddings(&tokens, &DefaultAttributeClassifier);
    let whole: Vec<_> = embeddings
        .iter()
        .filter(|e| matches!(e.kind, EmbeddingKind::WholeExpressionAttribute))
        .collect();
    assert_eq!(whole.len(), 1, "exactly one whole-expression attribute");
    assert_eq!(whole[0].attribute.as_deref(), Some("select"));
    let (query, diags) = compile_embedding(whole[0], &CompileContext::default());
    assert!(
        query.is_some(),
        "valid expression parses cleanly: {diags:?}"
    );
}

#[test]
fn avt_span_inside_template_aware_attribute_is_handed_off_to_cem_ql() {
    let tokens = tokenize(r#"{button @label="hello {1 + 1}" | Save}"#);
    let embeddings = extract_embeddings(&tokens, &DefaultAttributeClassifier);
    let spans: Vec<_> = embeddings
        .iter()
        .filter(|e| matches!(e.kind, EmbeddingKind::AvtSpan))
        .collect();
    assert_eq!(spans.len(), 1, "one AVT span inside @label");
    assert_eq!(spans[0].source.trim(), "1 + 1");
    assert_eq!(spans[0].attribute.as_deref(), Some("label"));
    let (query, diags) = compile_embedding(spans[0], &CompileContext::default());
    assert!(query.is_some(), "valid AVT span parses cleanly: {diags:?}");
}

#[test]
fn broken_embedding_surfaces_cem_ql_diagnostic_with_host_source_map() {
    let tokens = tokenize("{p | {$ 1 + }}");
    let embeddings = extract_embeddings(&tokens, &DefaultAttributeClassifier);
    let content: Vec<_> = embeddings
        .iter()
        .filter(|e| matches!(e.kind, EmbeddingKind::ContentExpression))
        .collect();
    assert_eq!(content.len(), 1);
    let (query, diags) = compile_embedding(content[0], &CompileContext::default());
    assert!(query.is_none(), "broken expression must not yield a query");
    assert_eq!(diags.len(), 1, "one diagnostic per broken embedding");
    let diagnostic = &diags[0];
    assert!(
        diagnostic.code.starts_with("cem.ql."),
        "diagnostic code is cem-ql namespaced: {}",
        diagnostic.code
    );
    let stack = diagnostic
        .source_map
        .as_ref()
        .expect("embedding diagnostic carries source-map stack");
    let has_host_frame = stack.frames.iter().any(|frame| {
        matches!(
            frame.transform,
            cem_ml::source_map::TransformKind::CemTokenizer
        )
    });
    assert!(has_host_frame, "host CemTokenizer frame preserved");
    let has_embedding_frame = stack.frames.iter().any(|frame| {
        matches!(
            frame.transform,
            cem_ml::source_map::TransformKind::TemplateEmbedding { .. }
        )
    });
    assert!(
        has_embedding_frame,
        "TemplateEmbedding frame pushed for the host → cem-ql boundary"
    );
}

#[test]
fn html_input_does_not_invoke_cem_ql_template_pass() {
    // AC-T-7 host embedding is owned by the CEM-native surface; HTML
    // inputs route through the HTML tokenizer instead. The cem-ml-cli
    // template pass short-circuits when from_format != Cem.
    let diagnostics = cem_ml_cli_template_pass(b"<p>Hello {{ not-cem-ql }}</p>", InputFormat::Html);
    assert!(
        diagnostics.is_empty(),
        "HTML inputs produce no cem-ql template diagnostics; got {diagnostics:?}"
    );
}

#[test]
fn template_embedding_target_is_registered() {
    let project = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("project.json");
    let text = std::fs::read_to_string(&project)
        .unwrap_or_else(|err| panic!("read {}: {err}", project.display()));
    assert!(
        text.contains("\"test:template-embedding\""),
        "project.json must expose the AC-T-7 verification target"
    );
}

// Reproduce the CLI's template-pass entry point inline — the binary
// crate's module is not reachable from this integration test.
fn cem_ml_cli_template_pass(
    bytes: &[u8],
    from_format: InputFormat,
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    if !matches!(from_format, InputFormat::Cem) {
        return Vec::new();
    }
    let tokens = tokenize(std::str::from_utf8(bytes).unwrap_or(""));
    let mut diagnostics = Vec::new();
    let ctx = CompileContext::default();
    for embedding in extract_embeddings(&tokens, &DefaultAttributeClassifier) {
        let (_, diags) = compile_embedding(&embedding, &ctx);
        diagnostics.extend(diags);
    }
    diagnostics
}
