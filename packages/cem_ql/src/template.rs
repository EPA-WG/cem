//! Host → cem-ql template-embedding boundary per AC-T-7.
//!
//! `cem_ml`'s tokenizer recognizes three host-owned surfaces that may
//! carry cem-ql source:
//!
//! - `{$ ... }` **content expressions** — emitted as
//!   `SchemaTokenKind::ExpressionNode(body)`. The whole token body is
//!   cem-ql source.
//! - **Whole-expression attributes** — `select` / `match` / `test` /
//!   `use` / `group-by` etc. The attribute value (after quote
//!   stripping) is cem-ql source. The classifier identifies them by
//!   name; schemas may extend the set in the future.
//! - **Template-aware attribute values** — attributes whose value
//!   carries one or more `{...}` AVT spans. The tokenizer detects the
//!   spans and preserves the literal braces in the value string; this
//!   module extracts each span and treats it as cem-ql source.
//!
//! For every surface this module produces an `EmbeddedExpression` that
//! the caller hands to [`compile_embedding`]. The compiled query (or
//! the parse diagnostics, if the cem-ql parser rejected the body)
//! carries a `TransformKind::TemplateEmbedding { host }` source-map
//! frame on top of the host token's stack, satisfying the AC-P-7
//! "preserve both the host span and the cem-ql sub-span" rule.

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::source::ByteRange;
use cem_ml::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use cem_ml::tokenizer::{SchemaToken, SchemaTokenKind};

use crate::api::{compile, CompileContext};
use crate::ir::CompiledQuery;

/// Classification AC-T-7 gives a host attribute. The default
/// classifier maps the documented `select` / `match` / `test` /
/// `use` / `group-by` names to `WholeExpression`; everything else is
/// `Opaque` unless a `{...}` AVT span is detected at extraction time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeKind {
    /// Whole attribute value is cem-ql source.
    WholeExpression,
    /// Attribute value MAY contain `{...}` AVT spans interleaved with
    /// literal text. Each span is one cem-ql expression.
    TemplateAware,
    /// Attribute carries no embedded cem-ql.
    Opaque,
}

/// Names the default classifier treats as whole-expression attributes.
/// Sourced from `cem-ml-ac.md` AC-T-7 normative list ("`select`,
/// `match`, `test`, `use`, `group-by`, etc."). The `cem:`-prefixed
/// schema-switch attributes (`cem:schema-select`) are also covered.
pub const WHOLE_EXPRESSION_ATTRIBUTES: &[&str] = &[
    "select",
    "match",
    "test",
    "use",
    "group-by",
    "cem:schema-select",
    "schema-select",
];

/// Classifier the extractor consults for each attribute name.
pub trait AttributeClassifier {
    fn classify(&self, attribute_name: &str) -> AttributeKind;
}

#[derive(Debug, Default, Clone)]
pub struct DefaultAttributeClassifier;

impl AttributeClassifier for DefaultAttributeClassifier {
    fn classify(&self, attribute_name: &str) -> AttributeKind {
        if WHOLE_EXPRESSION_ATTRIBUTES
            .iter()
            .any(|name| name.eq_ignore_ascii_case(attribute_name))
        {
            AttributeKind::WholeExpression
        } else {
            AttributeKind::TemplateAware
        }
    }
}

/// Origin of an embedded cem-ql expression. Tells diagnostics which
/// AC-T-7 surface they refer to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingKind {
    /// `{$ ... }` content expression.
    ContentExpression,
    /// Whole attribute value is one cem-ql expression.
    WholeExpressionAttribute,
    /// One `{...}` AVT span lifted out of a template-aware attribute
    /// value. The AVT scanner unescapes literal `{{` / `}}` pairs.
    AvtSpan,
}

/// One cem-ql source body the host parser handed off to cem-ql,
/// together with the byte range the host owned. `host_range` is what
/// the `TemplateEmbedding` frame records as `host`.
#[derive(Debug, Clone)]
pub struct EmbeddedExpression {
    pub kind: EmbeddingKind,
    pub source: String,
    pub host_range: ByteRange,
    /// Host stack the embedding inherits from the token it was lifted
    /// from. The compile helper pushes the `TemplateEmbedding` frame
    /// on top of this stack.
    pub host_stack: SourceMapStack,
    /// Original attribute name when `kind` is `WholeExpressionAttribute`
    /// or `AvtSpan`. `None` for `ContentExpression`. Diagnostics use
    /// this to label the surface.
    pub attribute: Option<String>,
}

/// Walk a `cem_ml` token stream and yield every cem-ql embedding the
/// host hands off, in document order.
pub fn extract_embeddings(
    tokens: &[SchemaToken],
    classifier: &dyn AttributeClassifier,
) -> Vec<EmbeddedExpression> {
    let mut out = Vec::new();
    for token in tokens {
        match &token.kind {
            SchemaTokenKind::ExpressionNode(body) => {
                out.push(EmbeddedExpression {
                    kind: EmbeddingKind::ContentExpression,
                    source: body.clone(),
                    host_range: token.byte_range.clone(),
                    host_stack: token.source_map.clone(),
                    attribute: None,
                });
            }
            SchemaTokenKind::Attribute {
                name,
                value: Some(value),
                value_range: Some(value_range),
                ..
            } => match classifier.classify(name) {
                AttributeKind::WholeExpression => {
                    out.push(EmbeddedExpression {
                        kind: EmbeddingKind::WholeExpressionAttribute,
                        source: strip_avt_braces(value).to_owned(),
                        host_range: value_range.clone(),
                        host_stack: token.source_map.clone(),
                        attribute: Some(name.clone()),
                    });
                }
                AttributeKind::TemplateAware => {
                    for span in extract_avt_spans(value) {
                        out.push(EmbeddedExpression {
                            kind: EmbeddingKind::AvtSpan,
                            source: span,
                            host_range: value_range.clone(),
                            host_stack: token.source_map.clone(),
                            attribute: Some(name.clone()),
                        });
                    }
                }
                AttributeKind::Opaque => {}
            },
            _ => {}
        }
    }
    out
}

/// Scan `value` for `{...}` AVT spans, unescaping literal `{{` / `}}`
/// per AC-T-7. Returns the cem-ql source inside each span (braces
/// stripped). Unbalanced trailing `{` is ignored — the cem-ml
/// tokenizer already emitted `cem.tokenizer.unterminated_avt_span`
/// for the host-level break.
fn extract_avt_spans(value: &str) -> Vec<String> {
    let mut spans = Vec::new();
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => {
                if chars.peek() == Some(&'{') {
                    // Escaped literal `{{` — not a span boundary.
                    chars.next();
                    continue;
                }
                let mut depth = 1u32;
                let mut body = String::new();
                while let Some(next) = chars.next() {
                    match next {
                        '{' => {
                            depth += 1;
                            body.push('{');
                        }
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                            body.push('}');
                        }
                        other => body.push(other),
                    }
                }
                if depth == 0 {
                    spans.push(body);
                }
            }
            '}' if chars.peek() == Some(&'}') => {
                // Escaped literal `}}` outside any span.
                chars.next();
            }
            _ => {}
        }
    }
    spans
}

/// Strip the leading `{` and trailing `}` the cem-ml tokenizer wraps
/// around a value that is exactly one AVT span. Other shapes are
/// returned verbatim so whole-expression attributes carrying plain
/// cem-ql source (no surrounding braces) work too.
fn strip_avt_braces(value: &str) -> &str {
    if value.starts_with('{') && value.ends_with('}') {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

/// Compile one embedding through the cem-ql front end, with a
/// `TransformKind::TemplateEmbedding { host }` frame pushed on top of
/// the host source-map stack so any diagnostic the parser emits
/// preserves both the host span and the cem-ql sub-span (AC-P-7).
/// Returns `(Some(query), diagnostics)` on a clean parse,
/// `(None, diagnostics)` when the cem-ql parser rejected the body.
pub fn compile_embedding(
    embedding: &EmbeddedExpression,
    ctx: &CompileContext,
) -> (Option<CompiledQuery>, Vec<Diagnostic>) {
    let embedded_ctx = with_embedding_frame(ctx, &embedding.host_stack, embedding.host_range.clone());
    match compile(&embedding.source, &embedded_ctx) {
        Ok(query) => (Some(query), Vec::new()),
        Err(err) => (None, vec![embedding_diagnostic(embedding, &err.code, err.message)]),
    }
}

fn with_embedding_frame(
    ctx: &CompileContext,
    host_stack: &SourceMapStack,
    host: ByteRange,
) -> CompileContext {
    let mut base = host_stack.clone();
    let source_id = base
        .current()
        .map(|frame| frame.source_id)
        .or_else(|| ctx.source_map_base.current().map(|frame| frame.source_id))
        .unwrap_or(cem_ml::source::SourceId(0));
    base.push(SourceMapFrame {
        source_id,
        span: FrameSpan::Single(host.clone()),
        transform: TransformKind::TemplateEmbedding { host },
    });
    CompileContext {
        schema_frame: ctx.schema_frame.clone(),
        overlay: ctx.overlay.clone(),
        diagnostics: ctx.diagnostics.clone(),
        source_map_base: base,
        policy_bindings: ctx.policy_bindings.clone(),
    }
}

fn embedding_diagnostic(
    embedding: &EmbeddedExpression,
    code: &str,
    message: String,
) -> Diagnostic {
    let surface = match embedding.kind {
        EmbeddingKind::ContentExpression => "content-expression `{$ ... }`".to_string(),
        EmbeddingKind::WholeExpressionAttribute => format!(
            "whole-expression attribute `{}=`",
            embedding.attribute.as_deref().unwrap_or("?")
        ),
        EmbeddingKind::AvtSpan => format!(
            "AVT span in attribute `{}=`",
            embedding.attribute.as_deref().unwrap_or("?")
        ),
    };
    let mut stack = embedding.host_stack.clone();
    let source_id = stack
        .current()
        .map(|frame| frame.source_id)
        .unwrap_or(cem_ml::source::SourceId(0));
    stack.push(SourceMapFrame {
        source_id,
        span: FrameSpan::Single(embedding.host_range.clone()),
        transform: TransformKind::TemplateEmbedding {
            host: embedding.host_range.clone(),
        },
    });
    Diagnostic {
        uri: None,
        line: None,
        column: None,
        byte_offset: Some(embedding.host_range.start),
        code: code.to_string(),
        severity: Severity::Error,
        message: format!("{surface}: {message}"),
        node: None,
        source_map: Some(stack),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifier_recognizes_documented_whole_expression_attributes() {
        let cls = DefaultAttributeClassifier;
        for name in WHOLE_EXPRESSION_ATTRIBUTES {
            assert_eq!(cls.classify(name), AttributeKind::WholeExpression, "{name}");
        }
        assert_eq!(cls.classify("class"), AttributeKind::TemplateAware);
        assert_eq!(cls.classify("Select"), AttributeKind::WholeExpression);
    }

    #[test]
    fn avt_extractor_unescapes_double_braces_and_splits_spans() {
        let spans = extract_avt_spans("Hello {.name}, you have {{literal}} {.count} items");
        assert_eq!(spans, vec![".name".to_string(), ".count".to_string()]);
    }

    #[test]
    fn avt_extractor_handles_nested_braces() {
        let spans = extract_avt_spans("{ if .x { .y } else { .z } }");
        assert_eq!(spans, vec![" if .x { .y } else { .z } ".to_string()]);
    }

    #[test]
    fn strip_braces_only_strips_outer_pair() {
        assert_eq!(strip_avt_braces("{.busy}"), ".busy");
        assert_eq!(strip_avt_braces("count(.items)"), "count(.items)");
    }
}
