//! External document syntax is resolved only here, into retained typed CEM trees.
use crate::{
    diagnostics::Diagnostic,
    lifecycle::LoadedInputAstStream,
    parser::{
        document::CemDocument,
        tree::{CemTreeRange, CemTreeSemantics, RetainedCemTree},
        AstNodeId, CemAstNode, ExpandedName,
    },
    source::{ByteRange, SourceId},
    source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind},
    validation::{
        csv,
        generic_data::{GenericDataDocumentAst, GenericDataValueAst as Value},
        json, json_xml, xml, yaml,
    },
};
use std::sync::Arc;

pub mod documents;
mod strings;
mod string_options;
pub use string_options::{resolve_string_import, ImportStringConfig, ImportStringOption};
pub use crate::validation::json_xml::{JsonXmlDuplicates, JsonXmlProjectionOptions};
pub use strings::{
    import_string, CsvHeader, CsvImportOptions, ImportFailure, ImportFailureKind,
    ImportStringProfile, ImportStringRequest,
};

pub const MAX_BYTES: usize = 32768;
pub const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
const MAX_DEPTH: usize = 64;
const MAX_VALUES: usize = 4096;
const DATA_NS: &str = "cem:generic-data";

// Length-framed fields and fixed-width lengths make tokens deterministic on
// native and WASM hosts. This hashes original input, never a serialized AST.
fn source_fingerprint(source_uri: &str, bytes: &[u8], profile: &[&str]) -> [u8; 32] {
    let mut hash = blake3::Hasher::new_derive_key("cem imported source selection/1");
    for field in std::iter::once(source_uri.as_bytes())
        .chain(profile.iter().map(|field| field.as_bytes()))
        .chain(std::iter::once(bytes))
    {
        hash.update(&(field.len() as u64).to_le_bytes());
        hash.update(field);
    }
    *hash.finalize().as_bytes()
}

/// Source-event correspondence exists only for compatibility import entrypoints.
pub struct XmlCemImport {
    pub ast: CemDocument,
    pub semantics: CemTreeSemantics,
    pub event_nodes: Vec<Option<AstNodeId>>,
    pub attribute_nodes: Vec<Vec<AstNodeId>>,
}

pub struct RetainedXmlImport {
    pub tree: Arc<RetainedCemTree>,
    pub event_nodes: Vec<Option<AstNodeId>>,
    pub attribute_nodes: Vec<Vec<AstNodeId>>,
}

/// Compatibility entrypoint for callers retaining an already parsed XML AST.
pub fn retain_xml(owner: Arc<LoadedInputAstStream>) -> Result<RetainedXmlImport, String> {
    let LoadedInputAstStream::XmlDocument(doc) = owner.as_ref() else {
        return Err("owner-is-not-xml".into());
    };
    let imported = import_xml_ast(doc)?;
    let uri = doc.source.uri.clone();
    let tree = RetainedCemTree::new(imported.ast, uri, "", imported.semantics, Some(owner))?;
    Ok(RetainedXmlImport {
        tree,
        event_nodes: imported.event_nodes,
        attribute_nodes: imported.attribute_nodes,
    })
}

/// Import XML semantic values before merging lexical text and references.
/// Source-oriented CEM fields remain unchanged.
pub fn import_xml_ast(document: &xml::XmlDocumentAst) -> Result<XmlCemImport, String> {
    use xml::XmlEventKind::*;
    if !document.parse_facts.is_empty() {
        let contracts = xml::XmlSchemaContractCatalog::from_builtin();
        if let Some(fact) = document
            .parse_facts
            .iter()
            .find(|fact| contracts.severity_for_fact(fact.kind).is_hard_violation())
        {
            return Err(fact.message.clone());
        }
    }
    let mut b = ImportBuilder::new();
    let mut semantics = CemTreeSemantics::default();
    semantics.sources.insert(
        0,
        SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(1),
                span: FrameSpan::Single(ByteRange::new(
                    0,
                    document.source.byte_length.try_into().unwrap_or(u32::MAX),
                )),
                transform: TransformKind::ContentTypeTransform {
                    content_type: document.source.media_type.clone(),
                },
            }],
        },
    );
    semantics.ranges.insert(
        0,
        CemTreeRange {
            line: 1,
            column: 1,
            offset: 0,
            length: document.source.byte_length as u64,
        },
    );
    let mut stack = vec![0];
    let mut event_nodes = vec![None; document.events.len()];
    let mut attribute_nodes = vec![vec![]; document.events.len()];
    for (index, event) in document.events.iter().enumerate() {
        let parent = *stack.last().ok_or("Unbalanced XML document.")?;
        let source = event.source_range.source_map();
        let data = event.value.clone().unwrap_or_else(|| event.lexeme.clone());
        let range = xml_range(event.source_range);
        let id = match event.kind {
            StartElement | EmptyElement => {
                let id = b.element(
                    parent,
                    event.namespace_uri.as_deref().unwrap_or(""),
                    event.local_name.as_deref().unwrap_or(""),
                    source.clone(),
                );
                for attr in &event.attributes {
                    let aid = b.ast.nodes.len() as AstNodeId;
                    b.attribute(
                        id,
                        attr.namespace_uri.as_deref().unwrap_or(""),
                        &attr.local_name,
                        attr.entity_decoded_value
                            .clone()
                            .ok_or("Unresolved XML attribute reference.")?,
                        attr.value_source_range
                            .map(|r| r.source_map())
                            .unwrap_or_else(|| source.clone()),
                    );
                    semantics
                        .ranges
                        .insert(aid, attr.value_source_range.map(xml_range).unwrap_or(range));
                    if attr.qualified_name == "xmlns" || attr.prefix.as_deref() == Some("xmlns") {
                        semantics.omitted.insert(aid);
                    } else {
                        semantics
                            .values
                            .insert(aid, xml_attribute_value(&attr.value)?);
                    }
                    attribute_nodes[index].push(aid);
                }
                if event.kind == StartElement {
                    stack.push(id);
                }
                id
            }
            EndElement => {
                if stack.len() <= 1 {
                    return Err("Unbalanced XML document.".into());
                }
                stack.pop();
                continue;
            }
            Text | EntityReference => {
                let value = if event.kind == EntityReference {
                    xml::xml_decode_entity_reference(event.value.as_deref().unwrap_or(""))
                        .ok_or("Unresolved XML reference.")?
                        .to_string()
                } else {
                    xml_line_endings(&data)
                };
                let raw = if event.kind == EntityReference {
                    value.clone()
                } else {
                    data
                };
                let previous = match &b.ast.nodes[parent as usize] {
                    CemAstNode::Element { children, .. } => children.last().copied(),
                    _ => None,
                };
                if let Some(previous) = previous
                    .filter(|&id| matches!(b.ast.nodes[id as usize], CemAstNode::Text { .. }))
                {
                    if let CemAstNode::Text {
                        data,
                        source: prior,
                        ..
                    } = &mut b.ast.nodes[previous as usize]
                    {
                        data.push_str(&raw);
                        prior.frames.extend(source.frames.clone());
                    }
                    semantics
                        .values
                        .entry(previous)
                        .or_default()
                        .push_str(&value);
                    crate::parser::tree::merge_source(
                        semantics
                            .sources
                            .get_mut(&previous)
                            .expect("previous imported text"),
                        &source,
                    );
                    if let Some(prior) = semantics.ranges.get_mut(&previous) {
                        prior.length = range.offset + range.length - prior.offset;
                    }
                    event_nodes[index] = Some(previous);
                    continue;
                }
                let id = b.ast.nodes.len() as AstNodeId;
                if event.whitespace_only {
                    b.push(
                        parent,
                        CemAstNode::Whitespace {
                            node_id: id,
                            data: raw,
                            source: source.clone(),
                        },
                    );
                } else {
                    b.text(parent, raw, source.clone());
                }
                semantics.values.insert(id, value);
                if event.depth == 0 {
                    semantics.omitted.insert(id);
                }
                id
            }
            Cdata | Comment => {
                let id = b.ast.nodes.len() as AstNodeId;
                let node = if event.kind == Cdata {
                    CemAstNode::Cdata {
                        node_id: id,
                        data: data.clone(),
                        source: source.clone(),
                    }
                } else {
                    CemAstNode::Comment {
                        node_id: id,
                        data: data.clone(),
                        source: source.clone(),
                    }
                };
                b.push(parent, node);
                semantics.values.insert(id, xml_line_endings(&data));
                if event.kind == Cdata && event.depth == 0 {
                    semantics.omitted.insert(id);
                }
                id
            }
            ProcessingInstruction | Declaration => {
                let id = b.ast.nodes.len() as AstNodeId;
                // The XML parser retains the complete PI body. Resolve its
                // target here once for both source inspection and XPath.
                // Declarations carry a full lexeme rather than a PI value.
                let body = if event.kind == Declaration {
                    data.strip_prefix("<?")
                        .and_then(|value| value.strip_suffix("?>"))
                        .ok_or("Invalid XML declaration lexeme.")?
                } else {
                    &data
                };
                let target = body.split([' ', '\t', '\r', '\n']).next().unwrap_or("");
                let value = body
                    .get(target.len()..)
                    .unwrap_or("")
                    .trim_start_matches([' ', '\t', '\r', '\n']);
                b.push(
                    parent,
                    CemAstNode::ProcessingInstruction {
                        node_id: id,
                        target: target.into(),
                        data: value.into(),
                        source: source.clone(),
                    },
                );
                if event.kind == Declaration {
                    semantics.omitted.insert(id);
                } else {
                    semantics.names.insert(id, expanded("", target));
                    semantics.values.insert(id, xml_line_endings(value));
                }
                id
            }
            Doctype => return Err("DTDs are not supported.".into()),
        };
        semantics.sources.insert(id, source);
        semantics.ranges.insert(id, range);
        event_nodes[index] = Some(id);
    }
    if stack.len() != 1 {
        return Err("Unbalanced XML document.".into());
    }
    let mut imported = XmlCemImport {
        ast: b.ast,
        semantics,
        event_nodes,
        attribute_nodes,
    };
    strings::xml_base_uris(&mut imported, document, Some(&document.source.uri));
    Ok(imported)
}

fn validate_xml_limits(document: &xml::XmlDocumentAst) -> Result<(), ImportFailure> {
    if document.events.len() > MAX_VALUES
        || document.events.iter().any(|event| {
            event.depth > MAX_DEPTH
                || (event.depth == MAX_DEPTH
                    && matches!(
                        event.kind,
                        xml::XmlEventKind::StartElement | xml::XmlEventKind::EmptyElement
                    ))
        })
    {
        return Err(ImportFailure::new(
            ImportFailureKind::Limit,
            "XML exceeds the 64-level / 4096-event import limit.",
        ));
    }
    Ok(())
}

fn xml_range(range: xml::XmlSourceRange) -> CemTreeRange {
    CemTreeRange {
        line: range.start.line,
        column: range.start.column,
        offset: range.start.byte_offset,
        length: range.byte_length,
    }
}
fn xml_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}
fn xml_attribute_value(text: &str) -> Result<String, String> {
    let normalized = xml_line_endings(text);
    let mut rest = normalized.as_str();
    let mut out = String::new();
    while let Some(start) = rest.find('&') {
        out.extend(rest[..start].chars().map(|c| {
            if matches!(c, '\t' | '\n' | '\r') {
                ' '
            } else {
                c
            }
        }));
        let end = rest[start..]
            .find(';')
            .ok_or("Unresolved XML attribute reference.")?
            + start;
        out.push(
            xml::xml_decode_entity_reference(&rest[start + 1..end])
                .ok_or("Unresolved XML attribute reference.")?,
        );
        rest = &rest[end + 1..];
    }
    out.extend(rest.chars().map(|c| {
        if matches!(c, '\t' | '\n' | '\r') {
            ' '
        } else {
            c
        }
    }));
    Ok(out)
}

fn checked<T>((document, diagnostics): (Option<T>, Vec<Diagnostic>)) -> Result<T, String> {
    if let Some(error) = diagnostics.iter().find(|d| d.severity.is_hard_violation()) {
        return Err(format!(
            "{}:{}: {}",
            error.line.unwrap_or(1),
            error.column.unwrap_or(1),
            error.message
        ));
    }
    document.ok_or_else(|| "No document was produced.".into())
}

fn parse(source: &str, format: &str, source_uri: &str) -> Result<LoadedInputAstStream, String> {
    if source.len() > MAX_BYTES {
        return Err("Source exceeds the 32 KiB import limit.".into());
    }
    parse_bytes(source.as_bytes(), format, source_uri)
}

fn parse_bytes(
    bytes: &[u8],
    content_type: &str,
    source_uri: &str,
) -> Result<LoadedInputAstStream, String> {
    let (format, content_type) = import_content_type(content_type)?;
    Ok(match format {
        "xml" | "application/xml" | "text/xml" => {
            let doc = checked(xml::xml_document_ast_from_source_bytes(
                xml::XmlSourceValidationRequest {
                    bytes,
                    source_uri,
                    content_type: Some(&content_type),
                },
            ))?;
            validate_xml_limits(&doc).map_err(|error| error.to_string())?;
            if doc.events.iter().any(|e| {
                e.kind == xml::XmlEventKind::Doctype
                    || (e.kind == xml::XmlEventKind::EntityReference
                        && e.value
                            .as_deref()
                            .and_then(xml::xml_decode_entity_reference)
                            .is_none())
            }) {
                return Err("DTD and unresolved entity references are not supported.".into());
            }
            LoadedInputAstStream::XmlDocument(doc)
        }
        "csv" | "text/csv" => LoadedInputAstStream::CsvDocument(checked(
            csv::csv_document_ast_from_source_bytes(csv::CsvSourceValidationRequest {
                bytes,
                source_uri,
                content_type: Some(&content_type),
            }),
        )?),
        "json" | "application/json" => LoadedInputAstStream::JsonDocument(checked(
            json::json_document_ast_from_source_bytes(json::JsonSourceValidationRequest {
                bytes,
                source_uri,
                content_type: Some(&content_type),
            }),
        )?),
        "yaml" | "application/yaml" => {
            let doc = checked(yaml::yaml_document_ast_from_source_bytes(
                yaml::YamlSourceValidationRequest {
                    bytes,
                    source_uri,
                    content_type: Some(&content_type),
                },
            ))?;
            LoadedInputAstStream::YamlDocument(doc)
        }
        _ => return Err("Choose xml, csv, yaml or json explicitly.".into()),
    })
}

/// Content negotiation and character decoding belong to this import boundary.
fn import_content_type(value: &str) -> Result<(&'static str, String), String> {
    let (mime, parameters) = value.split_once(';').unwrap_or((value, ""));
    let mime = mime.trim().to_ascii_lowercase();
    let (format, canonical) = match mime.as_str() {
        "json" | "application/json" | "text/json" => ("json", "application/json"),
        "xml" | "application/xml" | "text/xml" => ("xml", "application/xml"),
        "yaml" | "application/yaml" | "application/x-yaml" | "text/yaml" => {
            ("yaml", "application/yaml")
        }
        "csv" | "text/csv" => ("csv", "text/csv"),
        mime if mime.ends_with("+json") => ("json", "application/json"),
        mime if mime.ends_with("+xml") => ("xml", "application/xml"),
        _ => return Err(format!("Unsupported CEM import content type `{value}`.")),
    };
    let mut normalized = canonical.to_owned();
    if !parameters.trim().is_empty() {
        normalized.push(';');
        normalized.push_str(parameters);
    }
    if format == "csv" && !parameters.to_ascii_lowercase().contains("header=") {
        normalized.push_str(";header=present");
    }
    Ok((format, normalized))
}

/// Import response bytes without a JavaScript or serialized AST intermediate.
/// Byte readers retain their smaller limit; loaders additionally bound retained owners.
pub fn import_data_bytes(
    bytes: &[u8],
    content_type: &str,
    projection: &str,
    source_uri: &str,
) -> Result<Arc<RetainedCemTree>, String> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err("Source exceeds the 16 MiB document import limit.".into());
    }
    if content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .eq_ignore_ascii_case("text/plain")
    {
        if projection != "cem" {
            return Err("Plain text supports the CEM projection only.".into());
        }
        for parameter in content_type.split(';').skip(1) {
            if let Some((name, value)) = parameter.trim().split_once('=') {
                if name.eq_ignore_ascii_case("charset")
                    && !value.trim_matches('"').eq_ignore_ascii_case("utf-8")
                {
                    return Err("Plain text import requires UTF-8.".into());
                }
            }
        }
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        let source = SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(1),
                span: FrameSpan::Single(ByteRange::new(0, bytes.len() as u32)),
                transform: TransformKind::ContentTypeTransform {
                    content_type: content_type.into(),
                },
            }],
        };
        let mut builder = ImportBuilder::new();
        builder.text(0, text.into(), source.clone());
        builder.semantics.sources.insert(0, source);
        builder.semantics.ranges.insert(0, CemTreeRange {
            line: 1, column: 1, offset: 0, length: bytes.len() as u64,
        });
        builder.semantics.source_fingerprint = Some(source_fingerprint(
            source_uri,
            bytes,
            &["data/1", "text/plain;charset=utf-8", projection],
        ));
        return RetainedCemTree::new(builder.ast, source_uri, text, builder.semantics, None);
    }
    let native = Arc::new(parse_bytes(bytes, content_type, source_uri)?);
    project_native(
        native,
        "",
        projection,
        source_uri,
        content_type,
        bytes.len(),
        Some(source_fingerprint(
            source_uri,
            bytes,
            &["data/1", &import_content_type(content_type)?.1, projection],
        )),
    )
}

/// Import one explicitly selected external data format and tree vocabulary.
pub fn import_data(
    source: &str,
    format: &str,
    projection: &str,
    source_uri: &str,
) -> Result<Arc<RetainedCemTree>, String> {
    let native = Arc::new(parse(source, format, source_uri)?);
    project_native(
        native, source, projection, source_uri, format, source.len(),
        Some(source_fingerprint(
            source_uri,
            source.as_bytes(),
            &["data/1", &import_content_type(format)?.1, projection],
        )),
    )
}

/// Import a retained parser AST into the same tree used by the data reader.
/// `None` identifies an unrelated lifecycle representation, never a failed
/// external import. Consumers must not fall back to a partial tree on errors.
pub fn try_retain_lifecycle(
    native: Arc<LoadedInputAstStream>,
) -> Result<Option<Arc<RetainedCemTree>>, String> {
    if matches!(
        native.as_ref(),
        LoadedInputAstStream::XmlDocument(_)
            | LoadedInputAstStream::JsonDocument(_)
            | LoadedInputAstStream::CsvDocument(_)
            | LoadedInputAstStream::YamlDocument(_)
    ) {
        retain_lifecycle(native).map(Some)
    } else {
        Ok(None)
    }
}

/// Import a retained parser AST into the same tree used by the data reader.
pub fn retain_lifecycle(native: Arc<LoadedInputAstStream>) -> Result<Arc<RetainedCemTree>, String> {
    let (uri, format, length) = match native.as_ref() {
        LoadedInputAstStream::XmlDocument(doc) => (
            doc.source.uri.clone(),
            doc.source.media_type.clone(),
            doc.source.byte_length,
        ),
        LoadedInputAstStream::JsonDocument(doc) => (
            doc.source.uri.clone(),
            doc.source.media_type.clone(),
            doc.source.byte_length,
        ),
        LoadedInputAstStream::CsvDocument(doc) => (
            doc.source.uri.clone(),
            doc.source.media_type.clone(),
            doc.source.byte_length,
        ),
        LoadedInputAstStream::YamlDocument(doc) => (
            doc.source.uri.clone(),
            doc.source.media_type.clone(),
            doc.source.byte_length,
        ),
        _ => return Err("The lifecycle owner has no registered CEM data import.".into()),
    };
    project_native(native, "", "cem", &uri, &format, length, None)
}

fn validate_data_ast(native: &LoadedInputAstStream) -> Result<(), ImportFailure> {
    use ImportFailureKind::*;
    match native {
        LoadedInputAstStream::XmlDocument(doc) => validate_xml_limits(doc)?,
        LoadedInputAstStream::JsonDocument(doc) => {
            if let Some(fact) = doc.parse_facts.iter().find(|fact| fact.fatal) {
                return Err(ImportFailure::new(Malformed, fact.message.clone()));
            }
            let mut pending: Vec<_> = doc.root.iter().map(|value| (value, 0)).collect();
            let mut count = 0;
            while let Some((value, depth)) = pending.pop() {
                count += 1;
                if depth > MAX_DEPTH || count > MAX_VALUES {
                    return Err(ImportFailure::new(
                        Limit,
                        "JSON exceeds the 64-level / 4096-value import limit.",
                    ));
                }
                match value {
                    json::JsonValueAst::Object { members, .. } => {
                        pending.extend(members.iter().map(|m| (&m.value, depth + 1)))
                    }
                    json::JsonValueAst::Array { items, .. } => {
                        pending.extend(items.iter().map(|v| (v, depth + 1)))
                    }
                    _ => {}
                }
            }
        }
        LoadedInputAstStream::YamlDocument(doc) => {
            if let Some(fact) = doc.parse_facts.iter().find(|fact| fact.fatal) {
                return Err(ImportFailure::new(Malformed, fact.message.clone()));
            }
            let mut pending: Vec<_> = doc
                .documents
                .iter()
                .filter_map(|d| d.root.as_ref())
                .map(|n| (n, 0))
                .collect();
            let mut count = 0;
            while let Some((node, depth)) = pending.pop() {
                count += 1;
                if depth > MAX_DEPTH || count > MAX_VALUES {
                    return Err(ImportFailure::new(
                        Limit,
                        "YAML exceeds the 64-level / 4096-value import limit.",
                    ));
                }
                if node.alias.is_some() || node.anchor_id.is_some() || node.tag.is_some() {
                    return Err(ImportFailure::new(
                        Unsupported,
                        "YAML aliases, anchors and explicit tags are not supported.",
                    ));
                }
                pending.extend(node.sequence.iter().map(|n| (n, depth + 1)));
                pending.extend(
                    node.mapping
                        .iter()
                        .flat_map(|p| [(&p.key, depth + 1), (&p.value, depth + 1)]),
                );
            }
        }
        LoadedInputAstStream::CsvDocument(doc) => {
            if let Some(fact) = doc.parse_facts.iter().find(|fact| fact.fatal) {
                return Err(ImportFailure::new(Malformed, fact.message.clone()));
            }
        }
        _ => {}
    }
    Ok(())
}

fn project_native(
    native: Arc<LoadedInputAstStream>,
    source: &str,
    projection: &str,
    source_uri: &str,
    format: &str,
    byte_length: usize,
    fingerprint: Option<[u8; 32]>,
) -> Result<Arc<RetainedCemTree>, String> {
    validate_data_ast(native.as_ref()).map_err(|e| e.to_string())?;
    let (ast, mut semantics) = match (projection, native.as_ref()) {
        ("json-to-xml", LoadedInputAstStream::JsonDocument(doc)) =>
            json_xml::project_json_to_xml_with_semantics(doc, &json_xml::JsonXmlProjectionOptions { max_depth: MAX_DEPTH, max_values: MAX_VALUES, ..Default::default() }).map_err(|e| e.to_string())?,
        ("json-to-xml", _) => return Err("The json-to-xml projection requires JSON input.".into()),
        ("cem" | "xpath", LoadedInputAstStream::XmlDocument(doc)) => { let imported = import_xml_ast(doc)?; (imported.ast, imported.semantics) },
        ("xpath", _) => return Err("The xpath projection requires XML input; use the ordinary CEM import for other formats.".into()),
        ("cem", owner) => {
            let data = match owner {
                LoadedInputAstStream::JsonDocument(doc) => doc.to_generic_data_ast(),
                LoadedInputAstStream::CsvDocument(doc) => doc.to_generic_data_ast(),
                LoadedInputAstStream::YamlDocument(doc) => doc.to_generic_data_ast(),
                _ => return Err("Unsupported CEM data import.".into()),
            };
            let mut builder = ImportBuilder::new();
            builder.generic_data(&data)?;
            (builder.ast, builder.semantics)
        },
        _ => return Err("Choose the cem, json-to-xml or xpath projection explicitly.".into()),
    };
    semantics.source_fingerprint = fingerprint;
    semantics
        .sources
        .entry(0)
        .or_insert_with(|| SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(1),
                span: FrameSpan::Single(ByteRange::new(
                    0,
                    byte_length.try_into().unwrap_or(u32::MAX),
                )),
                transform: TransformKind::ContentTypeTransform {
                    content_type: format.into(),
                },
            }],
        });
    semantics.ranges.entry(0).or_insert(CemTreeRange {
        line: 1,
        column: 1,
        offset: 0,
        length: byte_length as u64,
    });
    RetainedCemTree::new(ast, source_uri, source, semantics, Some(native))
}

struct ImportBuilder {
    ast: CemDocument,
    semantics: CemTreeSemantics,
    visited: usize,
}
impl ImportBuilder {
    fn generic_data(&mut self, document: &GenericDataDocumentAst) -> Result<(), String> {
        for doc in &document.documents {
            if let Some(value) = &doc.root {
                self.value(0, value, 0)?;
            }
        }
        Ok(())
    }

    fn new() -> Self {
        let mut ast = CemDocument::default();
        ast.nodes.push(CemAstNode::Document {
            node_id: 0,
            root_children: vec![],
            source: SourceMapStack::default(),
        });
        Self {
            ast,
            semantics: CemTreeSemantics::default(),
            visited: 0,
        }
    }
    fn push(&mut self, parent: AstNodeId, node: CemAstNode) -> AstNodeId {
        let id = self.ast.nodes.len() as AstNodeId;
        let attribute = matches!(node, CemAstNode::Attribute { .. });
        self.ast.nodes.push(node);
        match &mut self.ast.nodes[parent as usize] {
            CemAstNode::Document { root_children, .. } => root_children.push(id),
            CemAstNode::Element {
                attributes,
                children,
                ..
            } => {
                if attribute {
                    attributes.push(id)
                } else {
                    children.push(id)
                }
            }
            _ => unreachable!("only document/element nodes own children"),
        }
        id
    }
    fn element(
        &mut self,
        parent: AstNodeId,
        namespace: &str,
        name: &str,
        source: SourceMapStack,
    ) -> AstNodeId {
        self.push(
            parent,
            CemAstNode::Element {
                node_id: self.ast.nodes.len() as AstNodeId,
                expanded_name: expanded(namespace, name),
                attributes: vec![],
                children: vec![],
                has_explicit_boundary: true,
                source,
            },
        )
    }
    fn attribute(
        &mut self,
        parent: AstNodeId,
        namespace: &str,
        name: &str,
        value: String,
        source: SourceMapStack,
    ) {
        self.push(
            parent,
            CemAstNode::Attribute {
                node_id: self.ast.nodes.len() as AstNodeId,
                expanded_name: expanded(namespace, name),
                value: Some(value),
                source,
            },
        );
    }
    fn text(&mut self, parent: AstNodeId, data: String, source: SourceMapStack) {
        self.push(
            parent,
            CemAstNode::Text {
                node_id: self.ast.nodes.len() as AstNodeId,
                data,
                source,
            },
        );
    }
    fn value(&mut self, parent: AstNodeId, value: &Value, depth: usize) -> Result<(), String> {
        self.visited += 1;
        if self.visited > MAX_VALUES
            || depth > MAX_DEPTH
            || (depth == MAX_DEPTH
                && matches!(value, Value::Sequence { .. } | Value::Mapping { .. }))
        {
            return Err("Data exceeds the 64-level / 4096-value import limit.".into());
        }
        let range = value.source_range();
        let source = range.source_map.clone().unwrap_or_else(|| SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(0),
                span: FrameSpan::Single(ByteRange::new(
                    range.byte_offset,
                    range.byte_length as u32,
                )),
                transform: TransformKind::CemAstBuilder,
            }],
        });
        let (name, scalar) = match value {
            Value::Mapping { .. } => ("object", None),
            Value::Sequence { .. } => ("array", None),
            Value::String { value, .. } => ("string", Some(value.clone())),
            Value::Number { lexeme, .. } => ("number", Some(lexeme.clone())),
            Value::Boolean { value, .. } => ("boolean", Some(value.to_string())),
            Value::Null { .. } => ("null", None),
            Value::Alias { .. } => return Err("Unresolved aliases are not supported.".into()),
        };
        let id = self.element(parent, DATA_NS, name, source.clone());
        let location = CemTreeRange {
            line: range.line,
            column: range.column,
            offset: range.byte_offset,
            length: range.byte_length,
        };
        self.semantics.ranges.insert(id, location);
        if let Some(scalar) = scalar {
            let text_id = self.ast.nodes.len() as u32;
            self.text(id, scalar, source.clone());
            self.semantics.ranges.insert(text_id, location);
        }
        match value {
            Value::Sequence { items, .. } => {
                for item in items {
                    self.value(id, item, depth + 1)?;
                }
            }
            Value::Mapping { entries, .. } => {
                for entry in entries {
                    let key =
                        match &entry.key {
                            Value::String { value, .. } => value.clone(),
                            Value::Number { lexeme, .. } => lexeme.clone(),
                            Value::Boolean { value, .. } => value.to_string(),
                            Value::Null { .. } => "null".into(),
                            _ => return Err(
                                "Complex mapping keys are not supported by this import profile."
                                    .into(),
                            ),
                        };
                    self.visited += 1;
                    let entry_source = entry
                        .source_range
                        .source_map
                        .clone()
                        .unwrap_or_else(|| source.clone());
                    let property = self.element(id, DATA_NS, "property", entry_source.clone());
                    let property_range = &entry.source_range;
                    self.semantics.ranges.insert(
                        property,
                        CemTreeRange {
                            line: property_range.line,
                            column: property_range.column,
                            offset: property_range.byte_offset,
                            length: property_range.byte_length,
                        },
                    );
                    let key_range = entry.key.source_range();
                    self.semantics.ranges.insert(
                        self.ast.nodes.len() as u32,
                        CemTreeRange {
                            line: key_range.line,
                            column: key_range.column,
                            offset: key_range.byte_offset,
                            length: key_range.byte_length,
                        },
                    );
                    self.attribute(
                        property,
                        "",
                        "name",
                        key,
                        entry
                            .key
                            .source_range()
                            .source_map
                            .clone()
                            .unwrap_or(entry_source),
                    );
                    self.value(property, &entry.value, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn expanded(namespace: &str, name: &str) -> ExpandedName {
    ExpandedName {
        namespace_uri: namespace.into(),
        local_name: name.into(),
        schema_id: None,
    }
}
