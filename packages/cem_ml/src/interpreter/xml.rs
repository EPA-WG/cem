//! XML transform.
//!
//! Walks a validated `CemDocument` and emits deterministic XML markup from
//! the internal AST. This is intentionally conservative: it preserves
//! comments, processing instructions, CDATA, raw text, text escaping,
//! attribute escaping, and source-map output spans without attempting schema
//! version conversion.

use crate::diagnostics::Diagnostic;
use crate::interpreter::{OutputSpan, OutputTarget, TransformOutput};
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::source::ByteRange;
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};

#[derive(Default)]
pub struct XmlInterpreter;

impl XmlInterpreter {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, doc: &CemDocument) -> TransformOutput {
        let mut r = Renderer::default();
        if let Some(root) = doc.root() {
            r.render_node(doc, root);
        }
        TransformOutput {
            target: OutputTarget::Xml,
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

#[derive(Default)]
struct Renderer {
    out: String,
    spans: Vec<OutputSpan>,
    diagnostics: Vec<Diagnostic>,
}

impl Renderer {
    fn render_node(&mut self, doc: &CemDocument, node: &CemAstNode) {
        match node {
            CemAstNode::Document { root_children, .. } => {
                for child_id in root_children {
                    if let Some(child) = doc.get(*child_id) {
                        self.render_node(doc, child);
                    }
                }
            }
            CemAstNode::Element {
                expanded_name,
                attributes,
                children,
                source,
                ..
            } => {
                let local = expanded_name.local_name.as_str();
                if local.starts_with('@') {
                    return;
                }

                let open_start = self.out.len() as u64;
                self.out.push('<');
                self.out.push_str(local);
                for attr_id in sorted_attribute_ids(doc, attributes) {
                    if let Some(CemAstNode::Attribute {
                        expanded_name,
                        value,
                        source,
                        ..
                    }) = doc.get(attr_id)
                    {
                        self.render_attribute(expanded_name, value.as_deref(), source);
                    }
                }

                if children.is_empty() {
                    self.out.push_str("/>");
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
            CemAstNode::Attribute { .. } => {}
            CemAstNode::Text { data, source, .. } | CemAstNode::Whitespace { data, source, .. } => {
                if data.is_empty() {
                    return;
                }
                let start = self.out.len() as u64;
                escape_text_into(&mut self.out, data);
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
            CemAstNode::Error { .. } => {}
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
        self.out.push_str("=\"");
        if let Some(v) = value {
            escape_attribute_into(&mut self.out, v);
        }
        self.out.push('"');
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
            '\'' => out.push_str("&apos;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::cem::CemEventNormalizer;
    use crate::parser::builder::CemAstBuilder;
    use crate::source::{BytesSource, SourceId};
    use crate::tokenizer::cem::CemTokenizer;

    fn render(input: &str) -> TransformOutput {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        let doc = CemAstBuilder::new(normalizer).build();
        XmlInterpreter::new().render(&doc)
    }

    #[test]
    fn simple_element_renders_to_xml() {
        let out = render("{p | Hello}");
        assert_eq!(out.rendered, "<p>Hello</p>");
    }

    #[test]
    fn empty_element_self_closes() {
        let out = render("{input @required}");
        assert_eq!(out.rendered, r#"<input required=""/>"#);
    }

    #[test]
    fn text_and_attributes_are_xml_escaped() {
        let out = render(r#"{p @title="x & <y>" | a < b & c}"#);
        assert_eq!(
            out.rendered,
            r#"<p title="x &amp; &lt;y&gt;">a &lt; b &amp; c</p>"#
        );
    }

    #[test]
    fn output_spans_cover_emitted_bytes() {
        let out = render("{p | Hello}");
        assert!(!out.output_spans.is_empty());
        let mut covered = vec![false; out.rendered.len()];
        for span in &out.output_spans {
            for i in span.output_range.start as usize
                ..(span.output_range.start as usize + span.output_range.len as usize)
            {
                if i < covered.len() {
                    covered[i] = true;
                }
            }
        }
        assert!(covered.iter().all(|covered| *covered));
    }
}
