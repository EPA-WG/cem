//! Typed page projection into the same event builder used by native HTML.
use super::*;
use crate::{
    interpreter::OutputSpan,
    projection::CemTreeAstNode,
    ssr_source::AuthoredPageSource,
    tokenizer::{AttributeValueSyntax, TokenizerProfile},
    transform_template::{
        transform_template_encode_html_attribute, transform_template_encode_html_text,
    },
};

pub(crate) fn project(
    nodes: &[CemTreeAstNode],
    authored: AuthoredPageSource,
    authored_bytes: &[u8],
    max_bytes: usize,
) -> Result<(Option<HtmlDocumentAst>, Vec<Diagnostic>), String> {
    let mut writer = Projection {
        max_bytes,
        ..Projection::default()
    };
    writer.push(
        "<!doctype html>",
        SchemaTokenKind::ProcessingInstruction {
            target: "doctype".into(),
            data: "html".into(),
        },
        &SourceMapStack::default(),
    )?;
    for node in nodes {
        writer.node(node, HtmlNamespace::Html, None, 0)?;
    }
    let index = LineIndex::from_utf8(&writer.lexemes);
    let request = HtmlSourceValidationRequest {
        bytes: writer.lexemes.as_bytes(),
        source_uri: &authored.uri,
        content_type: Some(HTML_CONTENT_TYPE),
    };
    let info = HtmlDocumentSource::from_request(request, BTreeMap::new());
    let (mut document, mut diagnostics) = html_document_from_tokens(
        &writer.lexemes,
        info,
        &index,
        &writer.tokens,
        Vec::new(),
        true,
    );
    let origins = crate::source_map::SourceMapIndex::new(writer.spans);
    let authored_index = LineIndex::from_bytes_lossy(authored_bytes);
    for diagnostic in &mut diagnostics {
        let range = diagnostic
            .source_map
            .as_ref()
            .and_then(SourceMapStack::current)
            .and_then(|frame| match frame.span {
                FrameSpan::Single(range) => Some(range),
                _ => None,
            });
        diagnostic.source_map = range.and_then(|range| origins.map_range(range));
        let offset = diagnostic.source_map.as_ref().and_then(|map| {
            map.frames.iter().rev().find_map(|frame| {
                if !matches!(&frame.transform, TransformKind::SourceBoundary { uri } if uri == &authored.uri) {
                    return None;
                }
                match &frame.span {
                    FrameSpan::Single(range) => Some(range.start),
                    FrameSpan::Multi(ranges) => ranges.first().map(|range| range.start),
                }
            })
        });
        let coordinate = offset.map(|offset| authored_index.project(offset));
        diagnostic.byte_offset = offset;
        diagnostic.line = coordinate.map(|coordinate| coordinate.line);
        diagnostic.column = coordinate.map(|coordinate| coordinate.column);
        if let Some(details) = diagnostic.details.as_mut().and_then(Value::as_object_mut) {
            details.remove("sourceRange");
        }
    }
    if let Some(document) = &mut document {
        document.origin_spans = Some(origins);
        document.ssr_authored_source = Some(authored);
    }
    Ok((document, diagnostics))
}

#[derive(Default)]
struct Projection {
    // A lexical sidecar supplies HTML event ranges and template text. The
    // native event builder takes typed tokens directly and never tokenizes it.
    lexemes: String,
    tokens: Vec<SchemaToken>,
    spans: Vec<OutputSpan>,
    max_bytes: usize,
}

impl Projection {
    fn push(
        &mut self,
        lexeme: &str,
        kind: SchemaTokenKind,
        source: &SourceMapStack,
    ) -> Result<(), String> {
        if lexeme.contains('\0') || lexeme.len() > self.max_bytes.saturating_sub(self.lexemes.len())
        {
            return Err("SSR page projection contains NUL or exceeds the byte budget".into());
        }
        let length = u32::try_from(lexeme.len()).map_err(|_| "SSR projection span exceeds u32")?;
        let range = ByteRange::new(self.lexemes.len() as u64, length);
        self.lexemes.push_str(lexeme);
        self.tokens.push(SchemaToken {
            kind,
            byte_range: range,
            profile: TokenizerProfile::Html,
            source_map: SourceMapStack::default(),
        });
        if length > 0 && !source.frames.is_empty() {
            self.spans.push(OutputSpan {
                output_range: range,
                origin: source.clone(),
            });
        }
        Ok(())
    }

    fn node(
        &mut self,
        node: &CemTreeAstNode,
        parent_namespace: HtmlNamespace,
        raw_parent: Option<&str>,
        depth: usize,
    ) -> Result<(), String> {
        if depth > 64 {
            return Err("SSR page projection exceeds the 64-level import limit".into());
        }
        let origin = node.source_map();
        match node {
            CemTreeAstNode::Element {
                name,
                attributes,
                children,
                ..
            } => {
                if raw_parent.is_some() {
                    return Err("SSR raw-text elements require literal text children".into());
                }
                let (declared, local) = expanded_name(name);
                let local = checked_name(local)?.to_ascii_lowercase();
                let namespace = html_element_namespace(parent_namespace, &local);
                if declared.is_some_and(|uri| uri != namespace.uri())
                    || matches!(
                        local.as_str(),
                        "script"
                            | "style"
                            | "iframe"
                            | "object"
                            | "embed"
                            | "frame"
                            | "frameset"
                            | "plaintext"
                            | "xmp"
                            | "noembed"
                            | "noframes"
                    )
                {
                    return Err(format!("SSR page element `{name}` has an unsupported namespace or script capability"));
                }
                if html_is_void_element(&local, namespace) && !children.is_empty() {
                    return Err(format!("SSR void element `{name}` cannot have children"));
                }
                self.push(
                    &format!("<{local}"),
                    SchemaTokenKind::NodeStart {
                        name: local.clone(),
                    },
                    origin,
                )?;
                let mut html_attributes = Vec::with_capacity(attributes.len());
                let mut names = BTreeSet::new();
                for attribute in attributes {
                    let (uri, local) = expanded_name(&attribute.name);
                    let prefix = match uri {
                        None => "",
                        Some("http://www.w3.org/XML/1998/namespace") => "xml:",
                        Some("http://www.w3.org/1999/xlink") => "xlink:",
                        _ => {
                            return Err(format!(
                                "Unsupported SSR attribute namespace in `{}`",
                                attribute.name
                            ))
                        }
                    };
                    let name = format!("{prefix}{}", checked_name(local)?).to_ascii_lowercase();
                    if !attribute.value_nodes.is_empty() || !names.insert(name.clone()) {
                        return Err(format!(
                            "SSR page attribute `{name}` requires a unique literal value"
                        ));
                    }
                    let value = attribute
                        .value
                        .as_deref()
                        .map(transform_template_encode_html_attribute);
                    self.lexemes.push(' ');
                    let start = self.lexemes.len() as u64;
                    let name_range = ByteRange::new(start, name.len() as u32);
                    let value_range = value
                        .as_ref()
                        .map(|v| ByteRange::new(name_range.end() + 2, v.len() as u32));
                    let lexeme = value
                        .as_ref()
                        .map_or_else(|| name.clone(), |v| format!("{name}=\"{v}\""));
                    self.push(
                        &lexeme,
                        SchemaTokenKind::Attribute {
                            name: name.clone(),
                            value,
                            name_range,
                            value_range,
                            value_syntax: AttributeValueSyntax::Literal,
                        },
                        &attribute.source,
                    )?;
                    html_attributes.push(HtmlAttributeAst {
                        lexical_name: name.clone(),
                        local_name: name,
                        value: attribute.value.clone(),
                        lexeme,
                        duplicate: false,
                        source_range: HtmlSourceRange {
                            start: HtmlSourcePosition {
                                line: 0,
                                column: 0,
                                byte_offset: start,
                            },
                            byte_length: 0,
                        },
                    });
                }
                let passive = html_attributes
                    .iter()
                    .filter(|attr| !(local == "base" && attr.local_name == "href"))
                    .map(|attr| {
                        (
                            attr.local_name.as_str(),
                            attr.value.as_deref().unwrap_or_default(),
                        )
                    })
                    .collect::<Vec<_>>();
                validate_passive_html_attributes(&local, &passive).map_err(|kind| {
                    format!("SSR projected page attribute refused: {}", kind.as_str())
                })?;
                if local == "meta"
                    && html_attribute_value(&html_attributes, "http-equiv")
                        .is_some_and(|value| value.eq_ignore_ascii_case("refresh"))
                {
                    return Err("SSR projected pages refuse meta refresh".into());
                }
                self.push(">", SchemaTokenKind::NodeEnd { name: None }, origin)?;
                let child_namespace = html_child_namespace(namespace, &local, &html_attributes);
                let raw = (namespace == HtmlNamespace::Html
                    && matches!(local.as_str(), "title" | "textarea"))
                .then_some(local.as_str());
                for child in children {
                    self.node(child, child_namespace, raw, depth + 1)?;
                }
                if !html_is_void_element(&local, namespace) {
                    self.push(
                        &format!("</{local}>"),
                        SchemaTokenKind::NodeEnd { name: Some(local) },
                        origin,
                    )?;
                }
            }
            CemTreeAstNode::Text { value, .. }
            | CemTreeAstNode::Whitespace { data: value, .. }
            | CemTreeAstNode::RawText { data: value, .. } => {
                let lexeme = transform_template_encode_html_text(value);
                self.push(&lexeme, SchemaTokenKind::Text(lexeme.clone()), origin)?;
            }
            CemTreeAstNode::Comment { data, .. }
                if raw_parent.is_none()
                    && !data.contains("--")
                    && !data.ends_with('-')
                    && !data.starts_with('>') =>
            {
                self.push(
                    &format!("<!--{data}-->"),
                    SchemaTokenKind::Comment(data.clone()),
                    origin,
                )?;
            }
            _ => return Err(format!("Unsupported SSR page node `{}`", node.kind())),
        }
        Ok(())
    }
}

fn expanded_name(name: &str) -> (Option<&str>, &str) {
    name.rsplit_once(':')
        .map_or((None, name), |(uri, local)| (Some(uri), local))
}

fn checked_name(name: &str) -> Result<&str, String> {
    if !name.starts_with(|c: char| c.is_ascii_alphabetic())
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(format!("Unsupported SSR HTML name `{name}`"));
    }
    Ok(name)
}
