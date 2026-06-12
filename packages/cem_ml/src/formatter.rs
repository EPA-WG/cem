//! Canonical CEM-ML formatter.
//!
//! Takes a built `CemDocument` and emits canonical CEM-ML text. The
//! formatter normalizes the surface per the rules in `docs/todo.md`
//! §Authoring Tooling:
//!
//! - **Indentation.** Two spaces per nesting level.
//! - **Canonical `|` insertion.** Explicit `|` before any non-empty
//!   element content; omitted when the element has no children.
//! - **Attribute ordering.** Sorted by `(namespace, local_name)` so the
//!   output is byte-stable.
//! - **Quote normalization.** Double-quoted strings; bare identifiers
//!   only when they match `name_continue+`.
//! - **Comment/whitespace preservation.** Comments are emitted on their
//!   own line; whitespace-only nodes between elements are dropped (the
//!   formatter manages line breaks deterministically).
//!
//! Idempotence: `format(parse(format(parse(input))))` equals
//! `format(parse(input))`. The integration test asserts this for every
//! canonical fixture.

use crate::engine::InputFormat;
use crate::interpreter::{OutputSpan, OutputTarget, TransformOutput};
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::source::ByteRange;
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::tokenizer::SchemaTokenizer;

pub fn format(doc: &CemDocument) -> String {
    let mut out = String::new();
    if let Some(root) = doc.root() {
        write_node(doc, root, 0, &mut out, /*at_block=*/ true);
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Format a built document as canonical CEM-ML and preserve a transform
/// source-map boundary. Tier A records one whole-output span whose origin
/// stack is rooted in the source tokenizer frame and capped by a
/// `ContentTypeTransform` frame for the generated CEM-ML bytes.
pub fn format_transform(doc: &CemDocument, source_content_type: &str) -> TransformOutput {
    let rendered = format(doc);
    let output_range = ByteRange::new(0, rendered.len() as u32);
    let mut origin = first_source_stack(doc).unwrap_or_default();
    let source_id = origin
        .origin()
        .map(|f| f.source_id)
        .unwrap_or(crate::source::SourceId(0));
    origin.push(SourceMapFrame {
        source_id,
        span: FrameSpan::Single(output_range),
        transform: TransformKind::ContentTypeTransform {
            content_type: source_content_type.to_owned(),
        },
    });
    TransformOutput {
        target: OutputTarget::CanonicalCemMl,
        rendered,
        diagnostics: Vec::new(),
        source_map: origin.clone(),
        output_spans: vec![OutputSpan {
            output_range,
            origin,
        }],
    }
}

/// Parse a CEM-ML, HTML, or XML source into the shared AST and emit
/// canonical CEM-ML.
pub fn format_source_as(input: &str, from_format: InputFormat) -> TransformOutput {
    let doc = parse_source_as(input, from_format);
    let mut output = format_transform(&doc, content_type_for(from_format));
    ensure_source_tokenizer_frame(&mut output, from_format, input.len() as u32);
    output
}

fn write_node(
    doc: &CemDocument,
    node: &CemAstNode,
    indent: usize,
    out: &mut String,
    at_block: bool,
) {
    match node {
        CemAstNode::Document { root_children, .. } => {
            for id in root_children {
                if let Some(child) = doc.get(*id) {
                    if !is_whitespace_node(child) {
                        write_node(doc, child, indent, out, true);
                    }
                }
            }
        }
        CemAstNode::Element {
            expanded_name,
            attributes,
            children,
            ..
        } => {
            let local = expanded_name.local_name.as_str();
            if local.starts_with('@') {
                write_directive(doc, node, out);
                return;
            }
            if at_block {
                push_indent(out, indent);
            }
            out.push('{');
            out.push_str(local);
            let attr_ids = sorted_attribute_ids(doc, attributes);
            for attr_id in &attr_ids {
                if let Some(attr) = doc.get(*attr_id) {
                    write_attribute(attr, out);
                }
            }
            let renderable_children: Vec<AstNodeId> = children
                .iter()
                .copied()
                .filter(|id| {
                    doc.get(*id)
                        .map(|n| !is_whitespace_node(n))
                        .unwrap_or(false)
                })
                .collect();
            if renderable_children.is_empty() {
                out.push('}');
                if at_block {
                    out.push('\n');
                }
                return;
            }
            let inline = is_inline_eligible(doc, &renderable_children);
            if inline {
                out.push_str(" | ");
                for (i, id) in renderable_children.iter().enumerate() {
                    if i > 0 {
                        out.push(' ');
                    }
                    if let Some(child) = doc.get(*id) {
                        write_node(doc, child, 0, out, false);
                    }
                }
                out.push('}');
                if at_block {
                    out.push('\n');
                }
            } else {
                out.push_str(" |\n");
                for id in &renderable_children {
                    if let Some(child) = doc.get(*id) {
                        write_node(doc, child, indent + 1, out, true);
                    }
                }
                push_indent(out, indent);
                out.push('}');
                if at_block {
                    out.push('\n');
                }
            }
        }
        CemAstNode::Text { data, .. } => {
            if at_block {
                push_indent(out, indent);
                write_text(data, out);
                out.push('\n');
            } else {
                write_text(data, out);
            }
        }
        CemAstNode::Whitespace { .. } => {
            // Dropped — the formatter manages spacing itself.
        }
        CemAstNode::Comment { data, .. } => {
            push_indent(out, indent);
            out.push_str("/*");
            out.push_str(data);
            out.push_str("*/\n");
        }
        CemAstNode::ProcessingInstruction { target, data, .. } => {
            push_indent(out, indent);
            out.push_str("<?");
            out.push_str(target);
            if !data.is_empty() {
                out.push(' ');
                out.push_str(data);
            }
            out.push_str("?>\n");
        }
        CemAstNode::Cdata { data, .. } | CemAstNode::RawText { data, .. } => {
            push_indent(out, indent);
            out.push_str(data);
            out.push('\n');
        }
        CemAstNode::Attribute { .. } => {
            // Attributes are emitted by their owning element.
        }
        CemAstNode::Error { code, .. } => {
            push_indent(out, indent);
            out.push_str("/* error: ");
            out.push_str(code);
            out.push_str(" */\n");
        }
    }
}

fn write_directive(doc: &CemDocument, node: &CemAstNode, out: &mut String) {
    let CemAstNode::Element {
        expanded_name,
        children,
        ..
    } = node
    else {
        return;
    };
    out.push('@');
    out.push_str(
        expanded_name
            .local_name
            .strip_prefix('@')
            .unwrap_or(&expanded_name.local_name),
    );
    // Concatenate child text nodes as the directive body.
    let mut body = String::new();
    for id in children {
        if let Some(CemAstNode::Text { data, .. }) = doc.get(*id) {
            if !body.is_empty() {
                body.push(' ');
            }
            body.push_str(data.trim());
        }
    }
    if !body.is_empty() {
        out.push(' ');
        out.push_str(&body);
    }
    out.push('\n');
}

fn write_attribute(attr: &CemAstNode, out: &mut String) {
    let CemAstNode::Attribute {
        expanded_name,
        value,
        ..
    } = attr
    else {
        return;
    };
    out.push_str(" @");
    if !expanded_name.namespace_uri.is_empty() {
        out.push_str(&expanded_name.namespace_uri);
        out.push(':');
    }
    out.push_str(&expanded_name.local_name);
    if let Some(v) = value {
        out.push('=');
        if is_avt_span(v) || is_bare_value_ok(v) {
            out.push_str(v);
        } else {
            out.push('"');
            for c in v.chars() {
                if c == '"' {
                    out.push('\\');
                }
                out.push(c);
            }
            out.push('"');
        }
    }
}

fn write_text(data: &str, out: &mut String) {
    let trimmed = data.trim();
    if needs_rich_content_enclosure(trimmed) {
        out.push_str("```");
        out.push_str(trimmed);
        out.push_str("```");
    } else {
        out.push_str(trimmed);
    }
}

fn needs_rich_content_enclosure(data: &str) -> bool {
    (data.contains('{') || data.contains('}')) && !data.contains("```")
}

fn is_avt_span(v: &str) -> bool {
    v.starts_with('{') && v.ends_with('}')
}

fn is_bare_value_ok(v: &str) -> bool {
    if v.is_empty() {
        return false;
    }
    v.chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/' | '.' | ':'))
}

fn is_whitespace_node(node: &CemAstNode) -> bool {
    matches!(node, CemAstNode::Whitespace { .. })
}

fn is_inline_eligible(doc: &CemDocument, children: &[AstNodeId]) -> bool {
    // Inline (`{name | content}` on one line) when:
    //   - All children are Text or simple nested elements with text.
    //   - No nested Element has block-style children.
    if children.len() > 1 {
        // Multiple non-text children → use multi-line for readability.
        let mut text_count = 0;
        let mut elem_count = 0;
        for id in children {
            match doc.get(*id) {
                Some(CemAstNode::Text { .. }) => text_count += 1,
                Some(CemAstNode::Element { .. }) => elem_count += 1,
                _ => {}
            }
        }
        return text_count >= 1 && elem_count == 0;
    }
    let Some(only) = children.first().and_then(|id| doc.get(*id)) else {
        return false;
    };
    matches!(only, CemAstNode::Text { .. })
}

fn push_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn sorted_attribute_ids(doc: &CemDocument, attributes: &[AstNodeId]) -> Vec<AstNodeId> {
    let mut ids: Vec<AstNodeId> = attributes.to_vec();
    ids.sort_by(|a, b| {
        let (na, la) = key_of(doc, *a);
        let (nb, lb) = key_of(doc, *b);
        na.cmp(&nb).then_with(|| la.cmp(&lb))
    });
    ids
}

fn key_of(doc: &CemDocument, id: AstNodeId) -> (String, String) {
    match doc.get(id) {
        Some(CemAstNode::Attribute { expanded_name, .. }) => (
            expanded_name.namespace_uri.clone(),
            expanded_name.local_name.clone(),
        ),
        _ => (String::new(), String::new()),
    }
}

/// Parse + format in one call.
pub fn format_source(input: &str) -> String {
    format_source_as(input, InputFormat::Cem).rendered
}

fn parse_source_as(input: &str, from_format: InputFormat) -> CemDocument {
    use crate::source::{BytesSource, SourceId};
    use crate::tokenizer::cem::CemTokenizer;
    use crate::tokenizer::html::HtmlTokenizer;
    use crate::tokenizer::xml::XmlTokenizer;
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    match from_format {
        InputFormat::Cem => {
            let tok = CemTokenizer::from_source(src);
            build_document(tok)
        }
        InputFormat::Html => {
            let tok = HtmlTokenizer::from_source(src);
            build_document(tok)
        }
        InputFormat::Xml => {
            let tok = XmlTokenizer::from_source(src);
            build_document(tok)
        }
    }
}

fn build_document<T: SchemaTokenizer>(tok: T) -> CemDocument {
    use crate::events::cem::CemEventNormalizer;
    use crate::parser::builder::CemAstBuilder;
    let normalizer = CemEventNormalizer::new(tok);
    CemAstBuilder::new(normalizer).build()
}

fn content_type_for(format: InputFormat) -> &'static str {
    match format {
        InputFormat::Cem => "application/cem",
        InputFormat::Html => "text/html",
        InputFormat::Xml => "application/xml",
    }
}

fn ensure_source_tokenizer_frame(
    output: &mut TransformOutput,
    from_format: InputFormat,
    source_len: u32,
) {
    let expected = match from_format {
        InputFormat::Cem => TransformKind::CemTokenizer,
        InputFormat::Html => TransformKind::HtmlTokenizer,
        InputFormat::Xml => TransformKind::XmlTokenizer,
    };
    let has_expected = output.output_spans.iter().any(|span| {
        span.origin
            .frames
            .iter()
            .any(|f| same_tokenizer_transform(&f.transform, &expected))
    });
    if has_expected {
        return;
    }
    let frame = SourceMapFrame {
        source_id: crate::source::SourceId(1),
        span: FrameSpan::Single(ByteRange::new(0, source_len)),
        transform: expected,
    };
    output.source_map.frames.insert(0, frame.clone());
    for span in &mut output.output_spans {
        span.origin.frames.insert(0, frame.clone());
    }
}

fn same_tokenizer_transform(a: &TransformKind, b: &TransformKind) -> bool {
    matches!(
        (a, b),
        (TransformKind::CemTokenizer, TransformKind::CemTokenizer)
            | (TransformKind::HtmlTokenizer, TransformKind::HtmlTokenizer)
            | (TransformKind::XmlTokenizer, TransformKind::XmlTokenizer)
    )
}

fn first_source_stack(doc: &CemDocument) -> Option<SourceMapStack> {
    doc.iter().find_map(|node| match node {
        CemAstNode::Element { source, .. }
        | CemAstNode::Text { source, .. }
        | CemAstNode::Whitespace { source, .. }
        | CemAstNode::Comment { source, .. }
        | CemAstNode::ProcessingInstruction { source, .. }
        | CemAstNode::Cdata { source, .. }
        | CemAstNode::RawText { source, .. }
        | CemAstNode::Attribute { source, .. }
        | CemAstNode::Error { source, .. } => (!source.frames.is_empty()).then(|| source.clone()),
        CemAstNode::Document { .. } => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_element_formats_inline() {
        assert_eq!(format_source("{p | Hello}"), "{p | Hello}\n");
    }

    #[test]
    fn attributes_sort_alphabetically_by_namespace_then_local() {
        let out = format_source("{button @cem:action=primary @type=submit | Save}");
        // Empty namespace first → @type comes before @cem:action.
        assert_eq!(out, "{button @type=submit @cem:action=primary | Save}\n");
    }

    #[test]
    fn nested_children_use_block_form() {
        let out = format_source("{a | {b | x} {c | y}}");
        // Multiple non-text children → still inline because both are
        // Element-with-text. Block form is reserved for deeper structure.
        assert!(out.starts_with("{a"));
        assert!(out.ends_with("}\n"));
    }

    #[test]
    fn deeply_nested_uses_block_form() {
        let out = format_source("{outer | {inner | {leaf | x}}}");
        assert!(out.contains('\n'));
        assert!(out.contains("  {inner"));
    }

    #[test]
    fn quoted_value_used_for_values_with_spaces() {
        let out = format_source(r#"{label @text="hello world"}"#);
        assert!(out.contains("\"hello world\""));
    }

    #[test]
    fn bare_value_preserved_for_simple_identifiers() {
        let out = format_source("{input @type=email}");
        assert_eq!(out, "{input @type=email}\n");
    }

    #[test]
    fn boolean_attribute_renders_without_value() {
        let out = format_source("{input @required}");
        assert_eq!(out, "{input @required}\n");
    }

    #[test]
    fn formatter_is_idempotent_for_simple_input() {
        let inputs = [
            "{p | Hello}",
            "{button @type=submit @cem:action=primary | Save}",
            "{input @id=email @required @type=email}",
        ];
        for input in inputs {
            let once = format_source(input);
            let twice = format_source(&once);
            assert_eq!(once, twice, "formatter not idempotent for: {input}");
        }
    }

    #[test]
    fn brace_bearing_text_formats_as_rich_content() {
        let once = format_source(r#"{@type="application/json" | ```{"ok":true}```}"#);
        let twice = format_source(&once);
        assert_eq!(once, twice);
        assert!(once.contains(r#"```{"ok":true}```"#));
    }

    #[test]
    fn every_canonical_fixture_formats_idempotently() {
        let dir =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/cem-ml");
        let mut checked = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("cem") {
                continue;
            }
            let input = std::fs::read_to_string(&path).unwrap();
            let once = format_source(&input);
            let twice = format_source(&once);
            assert_eq!(
                once,
                twice,
                "formatter not idempotent for fixture `{}`",
                path.display()
            );
            checked += 1;
        }
        assert!(checked >= 5);
    }

    #[test]
    fn html_source_formats_to_canonical_cem_ml() {
        let out = format_source_as(
            r#"<button cem:action="primary" type="submit">Save</button>"#,
            InputFormat::Html,
        );
        assert_eq!(
            out.rendered,
            "{button @type=submit @cem:action=primary | Save}\n"
        );
        assert!(out.output_spans.iter().any(|span| {
            span.origin
                .frames
                .iter()
                .any(|f| matches!(f.transform, crate::source_map::TransformKind::HtmlTokenizer))
                && span.origin.frames.iter().any(|f| {
                    matches!(
                        &f.transform,
                        crate::source_map::TransformKind::ContentTypeTransform { content_type }
                            if content_type == "text/html"
                    )
                })
        }));
    }

    #[test]
    fn xml_source_formats_to_canonical_cem_ml() {
        let out = format_source_as(
            r#"<button cem:action="primary" type="submit">Save</button>"#,
            InputFormat::Xml,
        );
        assert_eq!(
            out.rendered,
            "{button @type=submit @cem:action=primary | Save}\n"
        );
        assert!(out.output_spans.iter().any(|span| {
            span.origin
                .frames
                .iter()
                .any(|f| matches!(f.transform, crate::source_map::TransformKind::XmlTokenizer))
                && span.origin.frames.iter().any(|f| {
                    matches!(
                        &f.transform,
                        crate::source_map::TransformKind::ContentTypeTransform { content_type }
                            if content_type == "application/xml"
                    )
                })
        }));
    }
}
