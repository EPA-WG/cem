//! Explicit XML expression intent is interpreted only at the shared import boundary.
use super::*;
use std::collections::BTreeSet;
const CONTROL_NS: &str = "https://cem.dev/ns/core/1";

pub(super) fn selected(event: &xml::XmlEventAst) -> Result<BTreeSet<usize>, String> {
    let Some(marker) = event.attributes.iter().find(|a| {
        a.namespace_uri.as_deref() == Some(CONTROL_NS) && a.local_name == "expression-attributes"
    }) else {
        return Ok(BTreeSet::new());
    };
    let value = marker
        .entity_decoded_value
        .as_deref()
        .ok_or("Unresolved expression attribute marker.")?;
    let mut selected = BTreeSet::new();
    for name in value.split_whitespace() {
        let (index, attribute) = event
            .attributes
            .iter()
            .enumerate()
            .find(|(_, a)| a.qualified_name == name)
            .ok_or_else(|| format!("Expression attribute `{name}` is absent from its element."))?;
        if attribute.qualified_name == "xmlns"
            || matches!(attribute.prefix.as_deref(), Some("xmlns") | Some("xml"))
            || (attribute.namespace_uri.as_deref() == Some(CONTROL_NS)
                && attribute.local_name == "expression-attributes")
        {
            return Err(format!(
                "Expression marker cannot select reserved attribute `{name}`."
            ));
        }
        if !selected.insert(index) {
            return Err(format!("Duplicate expression attribute `{name}`."));
        }
    }
    Ok(selected)
}

// Check that exactly one outer slot encloses the value. Quote/comment bodies
// cannot close it. Query semantics and reference evaluation remain consumer work.
fn payload(lexical: &str) -> Result<(&str, usize), String> {
    if lexical.len() > MAX_BYTES {
        return Err("XML expression attribute exceeds source limit.".into());
    }
    let leading = lexical.len() - lexical.trim_start().len();
    let slot = lexical.trim();
    if !slot.starts_with('{') || !slot.ends_with('}') {
        return Err("Expression attribute requires one enclosing `{...}` slot.".into());
    }
    let mut chars = slot.char_indices().peekable();
    let mut depth = 0usize;
    let mut quote = None;
    let mut comment = 0usize;
    let mut block = false;
    let mut line = false;
    while let Some((offset, c)) = chars.next() {
        let next = chars.peek().map(|(_, c)| *c);
        if let Some(q) = quote {
            if c == '\\' {
                chars.next();
            } else if c == q {
                if next == Some(q) {
                    chars.next();
                } else {
                    quote = None;
                }
            }
            continue;
        }
        if line {
            if matches!(c, '\n' | '\r') {
                line = false;
            }
            continue;
        }
        if block {
            if c == '*' && next == Some('/') {
                chars.next();
                block = false;
            }
            continue;
        }
        if comment > 0 {
            if c == '(' && next == Some(':') {
                chars.next();
                comment += 1;
            } else if c == ':' && next == Some(')') {
                chars.next();
                comment -= 1;
            }
            continue;
        }
        if matches!(c, '\'' | '"') {
            quote = Some(c);
        } else if c == '(' && next == Some(':') {
            chars.next();
            comment = 1;
        } else if c == '/' && next == Some('*') {
            chars.next();
            block = true;
        } else if c == '/' && next == Some('/') {
            chars.next();
            line = true;
        } else if c == '{' {
            depth += 1;
            if depth > MAX_DEPTH {
                return Err("XML expression slot nesting exceeds limit.".into());
            }
        } else if c == '}' {
            depth = depth
                .checked_sub(1)
                .ok_or("Unbalanced XML expression slot.")?;
            if depth == 0 && offset + 1 != slot.len() {
                return Err("Expression attribute requires one enclosing slot.".into());
            }
        }
    }
    if depth != 0 || quote.is_some() || comment != 0 || block || line {
        return Err("Unclosed XML expression slot.".into());
    }
    let body = &slot[1..slot.len() - 1];
    let offset = leading + 1 + body.len() - body.trim_start().len();
    let body = body.trim();
    if body.is_empty() {
        return Err("XML expression slot is empty.".into());
    }
    Ok((body, offset))
}

pub(super) fn append(
    builder: &mut ImportBuilder,
    attribute_id: AstNodeId,
    context: AstNodeId,
    attribute: &xml::XmlAttributeAst,
    semantics: &mut CemTreeSemantics,
) -> Result<AstNodeId, String> {
    let lexical = attribute
        .entity_decoded_value
        .as_deref()
        .ok_or("Unresolved expression attribute.")?;
    let (body, start) = payload(lexical)?;
    let mapping = attribute
        .entity_decoded_source_map
        .as_ref()
        .ok_or("Missing expression attribute source map.")?;
    let range = mapping
        .project_range(ByteRange::new(start as u64, body.len() as u32))
        .ok_or("Missing expression payload range.")?;
    let mut source = range.source_map();
    let source_id = source.origin().map(|f| f.source_id).unwrap_or(SourceId(1));
    for span in mapping.spans().iter().filter(|span| {
        span.decoded_byte_range.start >= start as u64
            && span.decoded_byte_range.end() <= (start + body.len()) as u64
    }) {
        source.frames.push(SourceMapFrame {
            source_id,
            span: FrameSpan::Single(ByteRange::new(
                span.source_range.start.byte_offset,
                span.source_range.byte_length as u32,
            )),
            transform: TransformKind::ExpressionEmbedding {
                expression: ByteRange::new(
                    span.decoded_byte_range.start - start as u64,
                    span.decoded_byte_range.len,
                ),
            },
        });
    }
    let id = builder.ast.nodes.len() as AstNodeId;
    if body.starts_with('#') {
        builder.ast.nodes.push(CemAstNode::Reference {
            node_id: id,
            expression: body.into(),
            context,
            targets: None,
            source: source.clone(),
        });
    } else {
        builder.ast.nodes.push(CemAstNode::Element {
            node_id: id,
            expanded_name: ExpandedName {
                namespace_uri: String::new(),
                local_name: "$".into(),
                schema_id: None,
            },
            attributes: vec![],
            children: vec![id + 1],
            has_explicit_boundary: true,
            source: source.clone(),
        });
        builder.ast.nodes.push(CemAstNode::Text {
            node_id: id + 1,
            data: body.into(),
            source: source.clone(),
        });
        semantics.sources.insert(id + 1, source.clone());
        semantics.ranges.insert(id + 1, xml_range(range));
    }
    semantics.sources.insert(id, source);
    semantics.ranges.insert(id, xml_range(range));
    semantics.values.remove(&attribute_id);
    if let CemAstNode::Attribute {
        value, value_nodes, ..
    } = &mut builder.ast.nodes[attribute_id as usize]
    {
        *value = None;
        value_nodes.push(id);
    }
    Ok(id)
}
