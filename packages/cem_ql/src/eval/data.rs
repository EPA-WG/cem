//! Format-neutral import into the existing typed CEM AST. No presentation policy.
use super::{AtomValue, Item, QueryItemView, QueryItemViewKind};
use cem_ml::{
    diagnostics::Diagnostic,
    parser::{document::CemDocument, AstNodeId, CemAstNode, ExpandedName},
    source::{ByteRange, SourceId},
    source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind},
    validation::{
        csv,
        generic_data::{GenericDataDocumentAst, GenericDataValueAst as Value},
        json, json_xml, xml, yaml,
    },
};
use std::{
    any::Any,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
};

const MAX_BYTES: usize = 32768;
const MAX_DEPTH: usize = 64;
const MAX_VALUES: usize = 4096;
const DATA_NS: &str = "cem:generic-data";

#[derive(Debug)]
enum Owner {
    Xml(xml::XmlDocumentAst),
    Json(json::JsonDocumentAst),
    Data(GenericDataDocumentAst),
}

#[derive(Debug)]
struct Imported {
    // Preserve the source-format owner alongside its typed CEM projection.
    _native: Option<Owner>,
    ast: CemDocument,
    source: String,
    identity: String,
    error: String,
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

fn parse(source: &str, format: &str) -> Result<Owner, String> {
    if source.len() > MAX_BYTES {
        return Err("Source exceeds the 32 KiB import limit.".into());
    }
    let bytes = source.as_bytes();
    let source_uri = "data:read/source";
    Ok(match format {
        "xml" | "application/xml" | "text/xml" => {
            let doc = checked(xml::xml_document_ast_from_source_bytes(
                xml::XmlSourceValidationRequest {
                    bytes,
                    source_uri,
                    content_type: Some("application/xml"),
                },
            ))?;
            if doc.events.len() > MAX_VALUES
                || doc.events.iter().any(|e| {
                    e.depth > MAX_DEPTH
                        || (e.depth == MAX_DEPTH
                            && matches!(
                                e.kind,
                                xml::XmlEventKind::StartElement | xml::XmlEventKind::EmptyElement
                            ))
                })
            {
                return Err("XML exceeds the 64-level / 4096-event import limit.".into());
            }
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
            Owner::Xml(doc)
        }
        "csv" | "text/csv" => Owner::Data(
            checked(csv::csv_document_ast_from_source_bytes(
                csv::CsvSourceValidationRequest {
                    bytes,
                    source_uri,
                    content_type: Some("text/csv;header=present"),
                },
            ))?
            .to_generic_data_ast(),
        ),
        "json" | "application/json" => Owner::Json(checked(
            json::json_document_ast_from_source_bytes(json::JsonSourceValidationRequest {
                bytes,
                source_uri,
                content_type: Some("application/json"),
            }),
        )?),
        "yaml" | "application/yaml" => {
            let doc = checked(yaml::yaml_document_ast_from_source_bytes(
                yaml::YamlSourceValidationRequest {
                    bytes,
                    source_uri,
                    content_type: Some("application/yaml"),
                },
            ))?;
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
                    return Err("YAML exceeds the 64-level / 4096-value import limit.".into());
                }
                if node.alias.is_some() || node.anchor_id.is_some() || node.tag.is_some() {
                    return Err("YAML aliases, anchors and explicit tags are not supported.".into());
                }
                pending.extend(node.sequence.iter().map(|n| (n, depth + 1)));
                pending.extend(
                    node.mapping
                        .iter()
                        .flat_map(|p| [(&p.key, depth + 1), (&p.value, depth + 1)]),
                );
            }
            Owner::Data(doc.to_generic_data_ast())
        }
        _ => return Err("Choose xml, csv, yaml or json explicitly.".into()),
    })
}

pub(super) fn read(source: &str, format: &str, projection: &str) -> Item {
    let mut hash = DefaultHasher::new();
    source.hash(&mut hash);
    format.hash(&mut hash);
    // Preserve existing default identities; different native views never alias.
    if projection != "cem" {
        projection.hash(&mut hash);
    }
    let mut imported = Imported {
        _native: None,
        ast: CemDocument::default(),
        source: source.into(),
        identity: format!("{:016x}", hash.finish()),
        error: String::new(),
    };
    match parse(source, format) {
        Err(error) => imported.error = error,
        Ok(owner) => {
            match project(&owner, projection) {
                Ok(ast) => imported.ast = ast,
                Err(error) => imported.error = error,
            }
            imported._native = Some(owner);
        }
    }
    Item::native(CemAstView {
        owner: Arc::new(imported),
        node: None,
    })
}

fn project(owner: &Owner, projection: &str) -> Result<CemDocument, String> {
    match projection {
        "json-to-xml" => match owner {
            Owner::Json(doc) => json_xml::project_json_to_xml(
                doc,
                &json_xml::JsonXmlProjectionOptions {
                    max_depth: MAX_DEPTH,
                    max_values: MAX_VALUES,
                    ..Default::default()
                },
            )
            .map_err(|error| error.to_string()),
            _ => Err("The json-to-xml projection requires JSON input.".into()),
        },
        "cem" => {
            let mut builder = ImportBuilder::new();
            match owner {
                Owner::Xml(doc) => builder.xml(doc)?,
                Owner::Json(doc) => builder.generic_data(&doc.to_generic_data_ast())?,
                Owner::Data(doc) => builder.generic_data(doc)?,
            }
            Ok(builder.ast)
        }
        _ => Err("Choose the cem or json-to-xml projection explicitly.".into()),
    }
}

struct ImportBuilder {
    ast: CemDocument,
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
        Self { ast, visited: 0 }
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
        if let Some(scalar) = scalar {
            self.text(id, scalar, source.clone());
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
    fn xml(&mut self, document: &xml::XmlDocumentAst) -> Result<(), String> {
        use xml::XmlEventKind::*;
        let mut stack = vec![0];
        for event in &document.events {
            let parent = *stack.last().ok_or("Unbalanced XML document.")?;
            let source = event.source_range.source_map();
            let data = event.value.clone().unwrap_or_else(|| event.lexeme.clone());
            let node_id = self.ast.nodes.len() as AstNodeId;
            match event.kind {
                StartElement | EmptyElement => {
                    let id = self.element(
                        parent,
                        event.namespace_uri.as_deref().unwrap_or(""),
                        event.local_name.as_deref().unwrap_or(""),
                        source.clone(),
                    );
                    for attr in &event.attributes {
                        self.attribute(
                            id,
                            attr.namespace_uri.as_deref().unwrap_or(""),
                            &attr.local_name,
                            attr.entity_decoded_value
                                .clone()
                                .ok_or("Unresolved XML attribute reference.")?,
                            attr.value_source_range
                                .as_ref()
                                .map(|r| r.source_map())
                                .unwrap_or_else(|| source.clone()),
                        );
                    }
                    if event.kind == StartElement {
                        stack.push(id);
                    }
                }
                EndElement => {
                    stack.pop();
                }
                Text | EntityReference => {
                    let data = if event.kind == EntityReference {
                        xml::xml_decode_entity_reference(event.value.as_deref().unwrap_or(""))
                            .ok_or("Unresolved XML reference.")?
                            .to_string()
                    } else {
                        data
                    };
                    let previous = match &self.ast.nodes[parent as usize] {
                        CemAstNode::Element { children, .. } => children.last().copied(),
                        _ => None,
                    };
                    if let Some(CemAstNode::Text {
                        data: prior,
                        source: prior_source,
                        ..
                    }) = previous.and_then(|id| self.ast.nodes.get_mut(id as usize))
                    {
                        prior.push_str(&data);
                        prior_source.frames.extend(source.frames);
                    } else if event.whitespace_only {
                        self.push(
                            parent,
                            CemAstNode::Whitespace {
                                node_id,
                                data,
                                source,
                            },
                        );
                    } else {
                        self.text(parent, data, source);
                    }
                }
                Cdata => {
                    self.push(
                        parent,
                        CemAstNode::Cdata {
                            node_id,
                            data,
                            source,
                        },
                    );
                }
                Comment => {
                    self.push(
                        parent,
                        CemAstNode::Comment {
                            node_id,
                            data,
                            source,
                        },
                    );
                }
                ProcessingInstruction | Declaration => {
                    self.push(
                        parent,
                        CemAstNode::ProcessingInstruction {
                            node_id,
                            target: event.qualified_name.clone().unwrap_or_else(|| "xml".into()),
                            data,
                            source,
                        },
                    );
                }
                Doctype => return Err("DTDs are not supported.".into()),
            }
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
fn string(value: impl Into<String>) -> Vec<Item> {
    vec![Item::Atomic(AtomValue::String(value.into()))]
}
fn node_source(node: &CemAstNode) -> &SourceMapStack {
    match node {
        CemAstNode::Document { source, .. }
        | CemAstNode::Element { source, .. }
        | CemAstNode::Attribute { source, .. }
        | CemAstNode::Text { source, .. }
        | CemAstNode::Whitespace { source, .. }
        | CemAstNode::Comment { source, .. }
        | CemAstNode::ProcessingInstruction { source, .. }
        | CemAstNode::Cdata { source, .. }
        | CemAstNode::RawText { source, .. }
        | CemAstNode::Error { source, .. } => source,
    }
}
fn children(node: &CemAstNode) -> &[AstNodeId] {
    match node {
        CemAstNode::Document { root_children, .. } => root_children,
        CemAstNode::Element { children, .. } => children,
        _ => &[],
    }
}

#[derive(Debug, Clone)]
struct CemAstView {
    owner: Arc<Imported>,
    node: Option<AstNodeId>,
}
impl CemAstView {
    fn item(&self, id: AstNodeId) -> Item {
        Item::native(Self {
            owner: self.owner.clone(),
            node: Some(id),
        })
    }
}
impl QueryItemView for CemAstView {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.ql.imported-cem-ast"
    }
    fn identity(&self) -> String {
        format!("{}:{:?}", self.owner.identity, self.node)
    }
    fn kind(&self) -> QueryItemViewKind {
        if self.node.is_some() {
            QueryItemViewKind::Node
        } else {
            QueryItemViewKind::Record
        }
    }
    fn source_map(&self) -> Option<SourceMapStack> {
        Some(node_source(self.owner.ast.get(self.node?)?).clone())
    }
    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let Some(id) = self.node else {
            return match name {
                "error" => Some(string(&self.owner.error)),
                "root" => Some(if self.owner.error.is_empty() {
                    vec![self.item(0)]
                } else {
                    vec![]
                }),
                _ => None,
            };
        };
        let node = self.owner.ast.get(id)?;
        if name == "id" {
            return Some(string(self.identity()));
        }
        if name == "line" {
            let offset = node_source(node)
                .origin()
                .map(|f| match &f.span {
                    FrameSpan::Single(r) => r.start,
                    FrameSpan::Multi(rs) => rs.first().map_or(0, |r| r.start),
                })
                .unwrap_or(0);
            return Some(string(
                (1 + self
                    .owner
                    .source
                    .as_bytes()
                    .iter()
                    .take(offset as usize)
                    .filter(|b| **b == b'\n')
                    .count())
                .to_string(),
            ));
        }
        if name == "children" {
            return Some(children(node).iter().map(|id| self.item(*id)).collect());
        }
        if name == "descendants" {
            let mut pending: Vec<_> = children(node).iter().rev().copied().collect();
            let mut out = vec![];
            while let Some(id) = pending.pop() {
                out.push(self.item(id));
                pending.extend(children(self.owner.ast.get(id)?).iter().rev().copied());
            }
            return Some(out);
        }
        Some(match (node, name) {
            (CemAstNode::Document { .. }, "kind") => string("document"),
            (CemAstNode::Element { .. }, "kind") => string("element"),
            (CemAstNode::Attribute { .. }, "kind") => string("attribute"),
            (
                CemAstNode::Element { expanded_name, .. }
                | CemAstNode::Attribute { expanded_name, .. },
                "name",
            ) => string(&expanded_name.local_name),
            (
                CemAstNode::Element { expanded_name, .. }
                | CemAstNode::Attribute { expanded_name, .. },
                "namespace",
            ) => string(&expanded_name.namespace_uri),
            (CemAstNode::Element { attributes, .. }, "attributes") => {
                attributes.iter().map(|id| self.item(*id)).collect()
            }
            (CemAstNode::Attribute { value, .. }, "value") => {
                value.as_ref().map(string).unwrap_or_default()
            }
            (CemAstNode::Text { .. }, "kind") => string("text"),
            (CemAstNode::Whitespace { .. }, "kind") => string("whitespace"),
            (CemAstNode::Comment { .. }, "kind") => string("comment"),
            (CemAstNode::Cdata { .. }, "kind") => string("cdata"),
            (CemAstNode::RawText { .. }, "kind") => string("raw-text"),
            (CemAstNode::ProcessingInstruction { .. }, "kind") => string("processing-instruction"),
            (CemAstNode::ProcessingInstruction { target, .. }, "name" | "target") => string(target),
            (
                CemAstNode::Text { data, .. }
                | CemAstNode::Whitespace { data, .. }
                | CemAstNode::Comment { data, .. }
                | CemAstNode::Cdata { data, .. }
                | CemAstNode::RawText { data, .. }
                | CemAstNode::ProcessingInstruction { data, .. },
                "value" | "data",
            ) => string(data),
            (CemAstNode::Error { .. }, "kind") => string("error"),
            (CemAstNode::Error { code, .. }, "code") => string(code),
            _ => vec![],
        })
    }
}
