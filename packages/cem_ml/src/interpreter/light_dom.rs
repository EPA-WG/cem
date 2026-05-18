//! Light-DOM transform.
//!
//! Walks a validated `CemDocument` and emits HTML compatible with
//! `@epa-wg/custom-element`: each AST element renders as its native HTML
//! open/close tag, with CEM annotations preserved as `cem:*` attributes so
//! the custom-element loader can attach behavior to the host node. No
//! shadow DOM is constructed; output is light-DOM markup ready to drop
//! into a page.
//!
//! Every emitted byte range is recorded in [`TransformOutput::output_spans`]
//! paired with the source-map stack of the originating AST node, satisfying
//! the "preserve transform source-map frames for generated custom-element
//! markup" deliverable.

use crate::diagnostics::Diagnostic;
use crate::interpreter::{
    Interpreter, OutputSpan, OutputTarget, TransformContext, TransformOutput,
};
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::source::ByteRange;
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
    "source", "track", "wbr",
];

fn is_void(local: &str) -> bool {
    VOID_ELEMENTS.contains(&local)
}

#[derive(Default)]
pub struct LightDomInterpreter;

impl LightDomInterpreter {
    pub fn new() -> Self {
        Self
    }

    /// Convenience entry point for renderers that operate on a built
    /// `CemDocument`. The trait-level [`Interpreter::transform`] wraps this.
    pub fn render(&self, doc: &CemDocument) -> TransformOutput {
        let mut r = Renderer::default();
        if let Some(root) = doc.root() {
            r.render_node(doc, root);
        }
        TransformOutput {
            target: OutputTarget::LightDomCustomElements,
            rendered: r.out,
            diagnostics: r.diagnostics,
            source_map: SourceMapStack {
                frames: vec![SourceMapFrame {
                    source_id: crate::source::SourceId(0),
                    span: FrameSpan::Single(ByteRange::new(0, 0)),
                    transform: TransformKind::InterpreterRender,
                }],
            },
            output_spans: r.spans,
        }
    }
}

impl Interpreter for LightDomInterpreter {
    fn transform(&self, _nodes: &[CemAstNode], _ctx: &TransformContext) -> TransformOutput {
        // Tier A drives transform off a `CemDocument`; calling the trait
        // method with a raw slice constructs a minimal document.
        let doc = CemDocument {
            nodes: _nodes.to_vec(),
            ..CemDocument::default()
        };
        self.render(&doc)
    }
}

#[derive(Default)]
struct Renderer {
    out: String,
    spans: Vec<OutputSpan>,
    diagnostics: Vec<Diagnostic>,
}

impl Renderer {
    fn render_node(&mut self, doc: &CemDocument, node: &CemAstNode) {
        match node {
            CemAstNode::Document {
                root_children,
                source,
                ..
            } => {
                let span_start = self.out.len() as u64;
                let _ = source;
                for child_id in root_children {
                    if let Some(child) = doc.get(*child_id) {
                        self.render_node(doc, child);
                    }
                }
                self.record_span(span_start, source);
            }
            CemAstNode::Element {
                expanded_name,
                attributes,
                children,
                source,
                ..
            } => {
                let local = expanded_name.local_name.as_str();
                let is_directive = local.starts_with('@');
                if is_directive {
                    // Directives don't render into the light-DOM output;
                    // they configured upstream layers and are stripped here.
                    return;
                }
                let open_start = self.out.len() as u64;
                self.out.push('<');
                self.out.push_str(local);
                let attr_ids = sorted_attribute_ids(doc, attributes);
                for attr_id in &attr_ids {
                    if let Some(CemAstNode::Attribute {
                        expanded_name,
                        value,
                        source,
                        ..
                    }) = doc.get(*attr_id)
                    {
                        self.render_attribute(expanded_name, value.as_deref(), source);
                    }
                }
                if is_void(local) && children.is_empty() {
                    // HTML5 self-close style: omit `/`; just close the tag.
                    self.out.push('>');
                    self.record_span(open_start, source);
                    return;
                }
                self.out.push('>');
                self.record_span(open_start, source);

                for child_id in children {
                    if let Some(child) = doc.get(*child_id) {
                        self.render_node(doc, child);
                    }
                }

                let close_start = self.out.len() as u64;
                self.out.push_str("</");
                self.out.push_str(local);
                self.out.push('>');
                self.record_span(close_start, source);
            }
            CemAstNode::Text { data, source, .. } => {
                let start = self.out.len() as u64;
                escape_text_into(&mut self.out, data);
                self.record_span(start, source);
            }
            CemAstNode::Whitespace { data, source, .. } => {
                if data.is_empty() {
                    return;
                }
                let start = self.out.len() as u64;
                self.out.push_str(data);
                self.record_span(start, source);
            }
            CemAstNode::Comment { data, source, .. } => {
                let start = self.out.len() as u64;
                self.out.push_str("<!--");
                self.out.push_str(data);
                self.out.push_str("-->");
                self.record_span(start, source);
            }
            CemAstNode::ProcessingInstruction {
                target,
                data,
                source,
                ..
            } => {
                let start = self.out.len() as u64;
                self.out.push_str("<?");
                self.out.push_str(target);
                if !data.is_empty() {
                    self.out.push(' ');
                    self.out.push_str(data);
                }
                self.out.push_str("?>");
                self.record_span(start, source);
            }
            CemAstNode::Cdata { data, source, .. } => {
                let start = self.out.len() as u64;
                self.out.push_str("<![CDATA[");
                self.out.push_str(data);
                self.out.push_str("]]>");
                self.record_span(start, source);
            }
            CemAstNode::RawText { data, source, .. } => {
                let start = self.out.len() as u64;
                self.out.push_str(data);
                self.record_span(start, source);
            }
            CemAstNode::Attribute { .. } => {
                // Attributes don't render as top-level children — they're
                // emitted by the owning element. Ignore here.
            }
            CemAstNode::Error { source, .. } => {
                let _ = source;
                // Tier A drops error nodes from rendered output; they
                // remain in the diagnostic list.
            }
        }
    }

    fn render_attribute(
        &mut self,
        expanded: &crate::parser::ExpandedName,
        value: Option<&str>,
        source: &SourceMapStack,
    ) {
        let start = self.out.len() as u64;
        self.out.push(' ');
        if !expanded.namespace_uri.is_empty() {
            self.out.push_str(&expanded.namespace_uri);
            self.out.push(':');
        }
        self.out.push_str(&expanded.local_name);
        if let Some(v) = value {
            self.out.push_str("=\"");
            escape_attribute_into(&mut self.out, v);
            self.out.push('"');
        }
        self.record_span(start, source);
    }

    fn record_span(&mut self, start: u64, origin: &SourceMapStack) {
        let end = self.out.len() as u64;
        if end <= start {
            return;
        }
        let mut origin = origin.clone();
        origin.push(SourceMapFrame {
            source_id: origin
                .frames
                .last()
                .map(|f| f.source_id)
                .unwrap_or(crate::source::SourceId(0)),
            span: FrameSpan::Single(ByteRange::new(start, (end - start) as u32)),
            transform: TransformKind::InterpreterRender,
        });
        self.spans.push(OutputSpan {
            output_range: ByteRange::new(start, (end - start) as u32),
            origin,
        });
    }
}

fn sorted_attribute_ids(doc: &CemDocument, attributes: &[AstNodeId]) -> Vec<AstNodeId> {
    // Deterministic order: namespace ("" first), then local_name. Stable
    // for ties.
    let mut ids: Vec<AstNodeId> = attributes.to_vec();
    ids.sort_by(|a, b| {
        let (na, la) = name_of(doc, *a);
        let (nb, lb) = name_of(doc, *b);
        na.cmp(&nb).then_with(|| la.cmp(&lb))
    });
    ids
}

fn name_of(doc: &CemDocument, id: AstNodeId) -> (String, String) {
    match doc.get(id) {
        Some(CemAstNode::Attribute { expanded_name, .. }) => (
            expanded_name.namespace_uri.clone(),
            expanded_name.local_name.clone(),
        ),
        _ => (String::new(), String::new()),
    }
}

fn escape_text_into(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

fn escape_attribute_into(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            _ => out.push(c),
        }
    }
}

/// Convenience renderer: parse + transform in one call. Diagnostics from
/// upstream layers are merged into the returned `TransformOutput`.
pub fn render_html(input: &str) -> TransformOutput {
    use crate::events::cem::CemEventNormalizer;
    use crate::parser::builder::CemAstBuilder;
    use crate::source::{BytesSource, SourceId};
    use crate::tokenizer::cem::CemTokenizer;

    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let mut tok = CemTokenizer::from_source(src);
    let tok_diags = tok.take_diagnostics();
    let normalizer = CemEventNormalizer::new(tok);
    let mut doc = CemAstBuilder::new(normalizer).build();
    doc.diagnostics.extend(tok_diags);
    let mut out = LightDomInterpreter::new().render(&doc);
    out.diagnostics.extend(doc.diagnostics);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> TransformOutput {
        render_html(input)
    }

    #[test]
    fn simple_element_renders_to_html() {
        let out = render("{p | Hello}");
        assert_eq!(out.rendered, "<p>Hello</p>");
    }

    #[test]
    fn attributes_render_in_namespace_then_local_order() {
        let out = render(r#"{button @type=submit @cem:action=primary | Save}"#);
        // Empty namespace attrs first (`type`), then `cem:action`.
        assert_eq!(
            out.rendered,
            "<button type=\"submit\" cem:action=\"primary\">Save</button>"
        );
    }

    #[test]
    fn boolean_attribute_renders_without_equals() {
        let out = render("{input @id=email @required}");
        assert!(out.rendered.contains(" required"));
        assert!(out.rendered.contains(" id=\"email\""));
    }

    #[test]
    fn void_element_omits_closing_tag() {
        let out = render("{input @type=email @name=email}");
        assert_eq!(out.rendered, r#"<input name="email" type="email">"#);
    }

    #[test]
    fn nested_elements_render() {
        let out = render("{a | {b | x}}");
        assert_eq!(out.rendered, "<a><b>x</b></a>");
    }

    #[test]
    fn text_is_html_escaped() {
        let out = render("{p | a < b & c}");
        assert_eq!(out.rendered, "<p>a &lt; b &amp; c</p>");
    }

    #[test]
    fn attribute_value_is_html_escaped() {
        let out = render(r#"{a @title="x & <b>"}"#);
        assert!(out.rendered.contains("&amp;"));
        assert!(out.rendered.contains("&lt;b"));
        assert!(!out.rendered.contains(" & "));
    }

    #[test]
    fn directives_do_not_render() {
        let out = render("@doc cem-ml 1\n{p | Hi}");
        assert_eq!(out.rendered, "<p>Hi</p>");
    }

    #[test]
    fn output_spans_cover_emitted_bytes() {
        let out = render("{p | Hello}");
        assert!(!out.output_spans.is_empty());
        // Total byte coverage equals output length (with possible
        // overlap; check at minimum each emitted byte falls in at least
        // one span).
        let mut covered = vec![false; out.rendered.len()];
        for s in &out.output_spans {
            for i in s.output_range.start as usize
                ..(s.output_range.start as usize + s.output_range.len as usize)
            {
                if i < covered.len() {
                    covered[i] = true;
                }
            }
        }
        assert!(covered.iter().all(|c| *c), "every output byte must be covered by a span");
    }

    #[test]
    fn output_spans_carry_interpreter_render_frame() {
        let out = render("{p | Hello}");
        let span = &out.output_spans[0];
        assert!(matches!(
            span.origin.frames.last().unwrap().transform,
            TransformKind::InterpreterRender
        ));
    }

    #[test]
    fn output_spans_trace_back_to_source_byte_range() {
        let out = render("{p | Hello}");
        // Find the span for "Hello".
        let hello_pos = out.rendered.find("Hello").unwrap() as u64;
        let span = out
            .output_spans
            .iter()
            .find(|s| s.output_range.start == hello_pos)
            .expect("expected a span at the text byte position");
        // The origin's first frame (origin-first) should point to the
        // source bytes for "Hello" in the input.
        let origin_frame = span.origin.frames.first().unwrap();
        if let FrameSpan::Single(r) = &origin_frame.span {
            let input = "{p | Hello}";
            let bytes = &input.as_bytes()[r.start as usize..(r.start + r.len as u64) as usize];
            assert!(
                std::str::from_utf8(bytes).unwrap().contains("Hello"),
                "origin byte range should point at source text"
            );
        } else {
            panic!("expected single span");
        }
    }
}
