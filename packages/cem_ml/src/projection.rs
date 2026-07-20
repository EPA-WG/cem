//! Projection artifacts and debug/interchange views for AST / events / DOM.
//!
//! The `*_binary_artifact` functions produce hash-addressed CEM binary chunks
//! for runtime/cache handoff. The JSON projection functions remain
//! consumer-friendly debug/interchange views for `cem-ml parse --format
//! dom-json|ast|events` and the same projections for `convert` / `inspect`.

use crate::engine::InputFormat;
use crate::events::{
    cem::CemEventNormalizer, EventNormalizer, NormalizedEvent, ScalarValue, SeparatorKind,
    Synthesis, TriviaKind,
};
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode, ExpandedName};
use crate::schema::registry::{
    CEM_AST_PROJECTION_CONTENT_TYPE, CEM_AST_PROJECTION_SCHEMA_URI,
    CEM_DOM_PROJECTION_CONTENT_TYPE, CEM_DOM_PROJECTION_SCHEMA_URI,
    CEM_EVENTS_PROJECTION_CONTENT_TYPE, CEM_EVENTS_PROJECTION_SCHEMA_URI,
};
use crate::source::{ByteRange, BytesSource, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::tokenizer::cem::CemTokenizer;
use crate::tokenizer::html::HtmlTokenizer;
use crate::tokenizer::xml::XmlTokenizer;
use crate::tokenizer::SchemaTokenizer;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

const BINARY_FORMAT_VERSION: &str = "cem-projection-bin/1";
const BINARY_MAGIC: &[u8; 8] = b"CEMPROJ\0";
const BINARY_VERSION: u16 = 1;
const HASH_SCHEME: &str = crate::content_cache::HASH_SCHEME;
const ROOT_CHUNK_ID: &str = "root";

#[derive(Clone, Copy)]
enum BinaryProjectionKind {
    Dom,
    Ast,
    Events,
}

impl BinaryProjectionKind {
    fn tag(self) -> u8 {
        match self {
            Self::Dom => 1,
            Self::Ast => 2,
            Self::Events => 3,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Dom => "dom",
            Self::Ast => "ast",
            Self::Events => "events",
        }
    }

    fn schema(self) -> &'static str {
        match self {
            Self::Dom => CEM_DOM_PROJECTION_SCHEMA_URI,
            Self::Ast => CEM_AST_PROJECTION_SCHEMA_URI,
            Self::Events => CEM_EVENTS_PROJECTION_SCHEMA_URI,
        }
    }

    fn content_type(self) -> &'static str {
        match self {
            Self::Dom => CEM_DOM_PROJECTION_CONTENT_TYPE,
            Self::Ast => CEM_AST_PROJECTION_CONTENT_TYPE,
            Self::Events => CEM_EVENTS_PROJECTION_CONTENT_TYPE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryProjectionArtifact {
    pub projection: String,
    pub schema: String,
    pub content_type: String,
    pub format_version: String,
    pub hash_scheme: String,
    pub hash: String,
    pub bytes: Vec<u8>,
    pub chunk_metadata: Vec<ProjectionChunkMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionChunkMetadata {
    pub id: String,
    pub parent_id: Option<String>,
    pub root_id: Option<String>,
    pub byte_offset: u64,
    pub byte_length: usize,
    pub child_links: Vec<ProjectionChildLink>,
    pub source_map_deltas: Vec<ProjectionSourceMapDelta>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionChildLink {
    pub chunk_id: String,
    pub root_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionSourceMapDelta {
    pub source_id: u32,
    pub byte_offset: u64,
    pub byte_length: u32,
    pub transform: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedProjectionChunk {
    pub id: String,
    pub parent_id: Option<String>,
    pub root_id: Option<String>,
    pub byte_offset: u64,
    pub byte_length: usize,
    pub hash: String,
    pub child_links: Vec<ProjectionChildLink>,
    pub source_map_deltas: Vec<ProjectionSourceMapDelta>,
    bytes: Arc<[u8]>,
}

impl SealedProjectionChunk {
    pub fn bytes(&self) -> &[u8] {
        self.bytes.as_ref()
    }

    pub fn shared_bytes(&self) -> Arc<[u8]> {
        self.bytes.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionChunkStream {
    pub projection: String,
    pub schema: String,
    pub content_type: String,
    pub format_version: String,
    pub hash_scheme: String,
    pub hash: String,
    pub byte_length: usize,
    chunks: Vec<SealedProjectionChunk>,
}

impl ProjectionChunkStream {
    pub fn chunks(&self) -> &[SealedProjectionChunk] {
        &self.chunks
    }

    pub fn chunk_by_id(&self, id: &str) -> Option<&SealedProjectionChunk> {
        self.chunks.iter().find(|chunk| chunk.id == id)
    }

    pub fn chunk_by_root_id(&self, root_id: &str) -> Option<&SealedProjectionChunk> {
        self.chunks
            .iter()
            .find(|chunk| chunk.root_id.as_deref() == Some(root_id))
    }

    pub fn replay_bytes(&self) -> Vec<u8> {
        replay_projection_chunks(&self.chunks, self.byte_length)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionStreamRoute {
    pub sink_id: String,
}

impl ProjectionStreamRoute {
    pub fn new(sink_id: impl Into<String>) -> Self {
        Self {
            sink_id: sink_id.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionRouteMode {
    Deterministic,
    Parallel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutedProjectionStream {
    pub sink_id: String,
    pub projection: String,
    pub schema: String,
    pub content_type: String,
    pub format_version: String,
    pub hash_scheme: String,
    pub hash: String,
    pub byte_length: usize,
    pub chunks: Vec<SealedProjectionChunk>,
}

impl RoutedProjectionStream {
    pub fn concatenated_bytes(&self) -> Vec<u8> {
        replay_projection_chunks(&self.chunks, self.byte_length)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionRouteError {
    WorkerPanicked,
}

impl std::fmt::Display for ProjectionRouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkerPanicked => write!(f, "projection stream route worker panicked"),
        }
    }
}

impl std::error::Error for ProjectionRouteError {}

impl BinaryProjectionArtifact {
    pub fn to_chunk_stream(&self) -> ProjectionChunkStream {
        let metadata = if self.chunk_metadata.is_empty() {
            vec![single_chunk_metadata(
                self.projection.as_str(),
                self.bytes.len(),
            )]
        } else {
            self.chunk_metadata.clone()
        };
        let chunks = metadata
            .into_iter()
            .map(|metadata| sealed_chunk_from_metadata(&self.bytes, metadata))
            .collect::<Vec<_>>();
        ProjectionChunkStream {
            projection: self.projection.clone(),
            schema: self.schema.clone(),
            content_type: self.content_type.clone(),
            format_version: self.format_version.clone(),
            hash_scheme: self.hash_scheme.clone(),
            hash: self.hash.clone(),
            byte_length: self.bytes.len(),
            chunks,
        }
    }

    pub fn to_metadata_json(&self) -> Value {
        json!({
            "kind": "cem-binary-projection",
            "projection": self.projection,
            "schema": self.schema,
            "contentType": self.content_type,
            "formatVersion": self.format_version,
            "hashScheme": self.hash_scheme,
            "hash": self.hash,
            "byteLength": self.bytes.len(),
            "nativeBytes": true,
        })
    }

    pub fn to_json_envelope(&self) -> Value {
        let stream = self.to_chunk_stream();
        json!({
            "kind": "cem-binary-projection",
            "projection": &stream.projection,
            "schema": &stream.schema,
            "contentType": &stream.content_type,
            "formatVersion": &stream.format_version,
            "hashScheme": &stream.hash_scheme,
            "hash": &stream.hash,
            "byteLength": stream.byte_length,
            "chunks": stream.chunks.iter().map(chunk_json_envelope).collect::<Vec<_>>(),
        })
    }
}

fn chunk_json_envelope(chunk: &SealedProjectionChunk) -> Value {
    let mut value = json!({
        "id": &chunk.id,
        "sealed": true,
        "byteOffset": chunk.byte_offset,
        "byteLength": chunk.byte_length,
        "hash": &chunk.hash,
        "dataEncoding": "hex",
        "data": hex_encode(chunk.bytes()),
    });
    if let Some(parent_id) = &chunk.parent_id {
        value["parentId"] = json!(parent_id);
    }
    if let Some(root_id) = &chunk.root_id {
        value["rootId"] = json!(root_id);
    }
    if !chunk.child_links.is_empty() {
        value["childLinks"] = json!(chunk
            .child_links
            .iter()
            .map(|link| json!({
                "chunkId": &link.chunk_id,
                "rootId": &link.root_id,
            }))
            .collect::<Vec<_>>());
    }
    if !chunk.source_map_deltas.is_empty() {
        value["sourceMapDeltas"] = json!(chunk
            .source_map_deltas
            .iter()
            .map(|delta| json!({
                "sourceId": delta.source_id,
                "byteOffset": delta.byte_offset,
                "byteLength": delta.byte_length,
                "transform": &delta.transform,
            }))
            .collect::<Vec<_>>());
    }
    value
}

fn sealed_chunk_from_metadata(
    artifact_bytes: &[u8],
    metadata: ProjectionChunkMetadata,
) -> SealedProjectionChunk {
    let start = metadata.byte_offset as usize;
    let end = start.saturating_add(metadata.byte_length);
    let bytes: Arc<[u8]> = Arc::from(artifact_bytes[start..end].to_vec().into_boxed_slice());
    let hash = projection_chunk_hash(bytes.as_ref());
    SealedProjectionChunk {
        id: metadata.id,
        parent_id: metadata.parent_id,
        root_id: metadata.root_id,
        byte_offset: metadata.byte_offset,
        byte_length: metadata.byte_length,
        hash,
        child_links: metadata.child_links,
        source_map_deltas: metadata.source_map_deltas,
        bytes,
    }
}

fn projection_chunk_hash(bytes: &[u8]) -> String {
    format!("{HASH_SCHEME}:{}", blake3::hash(bytes).to_hex())
}

fn single_chunk_metadata(projection: &str, byte_length: usize) -> ProjectionChunkMetadata {
    ProjectionChunkMetadata {
        id: ROOT_CHUNK_ID.to_owned(),
        parent_id: None,
        root_id: Some(format!("projection:{projection}")),
        byte_offset: 0,
        byte_length,
        child_links: Vec::new(),
        source_map_deltas: Vec::new(),
    }
}

fn replay_projection_chunks(chunks: &[SealedProjectionChunk], byte_length: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(byte_length);
    let mut chunks = chunks.to_vec();
    chunks.sort_by_key(|chunk| chunk.byte_offset);
    for chunk in chunks {
        out.extend_from_slice(chunk.bytes());
    }
    out
}

pub fn route_projection_stream(
    stream: &ProjectionChunkStream,
    routes: &[ProjectionStreamRoute],
    mode: ProjectionRouteMode,
) -> Result<Vec<RoutedProjectionStream>, ProjectionRouteError> {
    match mode {
        ProjectionRouteMode::Deterministic => Ok(routes
            .iter()
            .map(|route| routed_projection_stream(stream, route))
            .collect()),
        ProjectionRouteMode::Parallel => route_projection_stream_parallel(stream, routes),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn route_projection_stream_parallel(
    stream: &ProjectionChunkStream,
    routes: &[ProjectionStreamRoute],
) -> Result<Vec<RoutedProjectionStream>, ProjectionRouteError> {
    std::thread::scope(|scope| {
        let handles = routes
            .iter()
            .map(|route| {
                let route = route.clone();
                scope.spawn(move || routed_projection_stream(stream, &route))
            })
            .collect::<Vec<_>>();
        let mut routed = Vec::with_capacity(handles.len());
        for handle in handles {
            routed.push(
                handle
                    .join()
                    .map_err(|_| ProjectionRouteError::WorkerPanicked)?,
            );
        }
        Ok(routed)
    })
}

#[cfg(target_arch = "wasm32")]
fn route_projection_stream_parallel(
    stream: &ProjectionChunkStream,
    routes: &[ProjectionStreamRoute],
) -> Result<Vec<RoutedProjectionStream>, ProjectionRouteError> {
    Ok(routes
        .iter()
        .map(|route| routed_projection_stream(stream, route))
        .collect())
}

fn routed_projection_stream(
    stream: &ProjectionChunkStream,
    route: &ProjectionStreamRoute,
) -> RoutedProjectionStream {
    RoutedProjectionStream {
        sink_id: route.sink_id.clone(),
        projection: stream.projection.clone(),
        schema: stream.schema.clone(),
        content_type: stream.content_type.clone(),
        format_version: stream.format_version.clone(),
        hash_scheme: stream.hash_scheme.clone(),
        hash: stream.hash.clone(),
        byte_length: stream.byte_length,
        chunks: stream.chunks.clone(),
    }
}

/// Project a built `CemDocument` to the first canonical CEM DOM binary artifact.
///
/// This compatibility helper renders the native binary artifact as a full JSON
/// envelope with hex-encoded chunk data. Native callers should use
/// `dom_binary_projection_artifact` when they need direct byte access.
pub fn dom_binary_artifact(doc: &CemDocument) -> Value {
    dom_binary_projection_artifact(doc).to_json_envelope()
}

/// Project a built `CemDocument` to the first canonical CEM AST binary artifact.
pub fn ast_binary_artifact(doc: &CemDocument) -> Value {
    ast_binary_projection_artifact(doc).to_json_envelope()
}

/// Project the input source to the first canonical CEM event-stream binary
/// artifact.
pub fn events_binary_artifact_as(input: &[u8], from_format: InputFormat) -> Value {
    events_binary_projection_artifact_as(input, from_format).to_json_envelope()
}

pub fn dom_binary_projection_artifact(doc: &CemDocument) -> BinaryProjectionArtifact {
    document_binary_artifact(doc, BinaryProjectionKind::Dom)
}

pub fn ast_binary_projection_artifact(doc: &CemDocument) -> BinaryProjectionArtifact {
    document_binary_artifact(doc, BinaryProjectionKind::Ast)
}

pub fn events_binary_projection_artifact_as(
    input: &[u8],
    from_format: InputFormat,
) -> BinaryProjectionArtifact {
    let encoded = encode_events_binary(input, from_format);
    binary_artifact_with_chunks(
        BinaryProjectionKind::Events,
        encoded.bytes,
        encoded.chunk_metadata,
    )
}

fn document_binary_artifact(
    doc: &CemDocument,
    kind: BinaryProjectionKind,
) -> BinaryProjectionArtifact {
    let encoded = encode_document_binary(doc, kind);
    binary_artifact_with_chunks(kind, encoded.bytes, encoded.chunk_metadata)
}

fn binary_artifact_with_chunks(
    kind: BinaryProjectionKind,
    bytes: Vec<u8>,
    chunk_metadata: Vec<ProjectionChunkMetadata>,
) -> BinaryProjectionArtifact {
    let hash_hex = blake3::hash(&bytes).to_hex().to_string();
    let hash = format!("{HASH_SCHEME}:{hash_hex}");
    BinaryProjectionArtifact {
        projection: kind.name().to_owned(),
        schema: kind.schema().to_owned(),
        content_type: kind.content_type().to_owned(),
        format_version: BINARY_FORMAT_VERSION.to_owned(),
        hash_scheme: HASH_SCHEME.to_owned(),
        hash,
        bytes,
        chunk_metadata,
    }
}

struct EncodedBinaryProjection {
    bytes: Vec<u8>,
    chunk_metadata: Vec<ProjectionChunkMetadata>,
}

#[derive(Debug, Default)]
struct DocumentChunkRelationships {
    roots: Vec<AstNodeId>,
    parents: BTreeMap<AstNodeId, AstNodeId>,
    children: BTreeMap<AstNodeId, Vec<AstNodeId>>,
}

fn document_chunk_relationships(doc: &CemDocument) -> DocumentChunkRelationships {
    let mut relationships = DocumentChunkRelationships::default();
    let mut node_ids = Vec::with_capacity(doc.nodes.len());
    for node in &doc.nodes {
        let node_id = ast_node_id(node);
        node_ids.push(node_id);
        let children = ast_node_children(node);
        for child_id in &children {
            relationships.parents.insert(*child_id, node_id);
        }
        if !children.is_empty() {
            relationships.children.insert(node_id, children);
        }
    }
    relationships.roots = node_ids
        .into_iter()
        .filter(|node_id| !relationships.parents.contains_key(node_id))
        .collect();
    relationships
}

fn ast_node_id(node: &CemAstNode) -> AstNodeId {
    match node {
        CemAstNode::Document { node_id, .. }
        | CemAstNode::Element { node_id, .. }
        | CemAstNode::Attribute { node_id, .. }
        | CemAstNode::Text { node_id, .. }
        | CemAstNode::Whitespace { node_id, .. }
        | CemAstNode::Comment { node_id, .. }
        | CemAstNode::ProcessingInstruction { node_id, .. }
        | CemAstNode::Cdata { node_id, .. }
        | CemAstNode::RawText { node_id, .. }
        | CemAstNode::Error { node_id, .. } => *node_id,
    }
}

fn ast_node_children(node: &CemAstNode) -> Vec<AstNodeId> {
    match node {
        CemAstNode::Document { root_children, .. } => root_children.clone(),
        CemAstNode::Element {
            attributes,
            children,
            ..
        } => attributes
            .iter()
            .chain(children.iter())
            .copied()
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    }
}

fn ast_node_source_map(node: &CemAstNode) -> &SourceMapStack {
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

fn node_chunk_id(node_id: AstNodeId) -> String {
    format!("node-{node_id}")
}

fn node_root_id(node_id: AstNodeId) -> String {
    format!("node:{node_id}")
}

fn event_chunk_id(sequence: u64) -> String {
    format!("event-{sequence}")
}

fn event_root_id(sequence: u64) -> String {
    format!("event:{sequence}")
}

fn encode_document_binary(
    doc: &CemDocument,
    kind: BinaryProjectionKind,
) -> EncodedBinaryProjection {
    let relationships = document_chunk_relationships(doc);
    let mut out = Vec::new();
    write_binary_header(&mut out, kind);
    write_u32(&mut out, doc.nodes.len() as u32);
    let header_len = out.len();

    let mut chunk_metadata = Vec::with_capacity(doc.nodes.len().saturating_add(1));
    chunk_metadata.push(ProjectionChunkMetadata {
        id: ROOT_CHUNK_ID.to_owned(),
        parent_id: None,
        root_id: Some(format!("projection:{}", kind.name())),
        byte_offset: 0,
        byte_length: header_len,
        child_links: relationships
            .roots
            .iter()
            .map(|node_id| ProjectionChildLink {
                chunk_id: node_chunk_id(*node_id),
                root_id: node_root_id(*node_id),
            })
            .collect(),
        source_map_deltas: Vec::new(),
    });

    for node in doc.nodes.iter() {
        let node_id = ast_node_id(node);
        let start = out.len();
        encode_ast_node(&mut out, node);
        let byte_length = out.len() - start;
        let parent_id = relationships
            .parents
            .get(&node_id)
            .map(|parent| node_chunk_id(*parent))
            .unwrap_or_else(|| ROOT_CHUNK_ID.to_owned());
        let child_links = relationships
            .children
            .get(&node_id)
            .map(|children| {
                children
                    .iter()
                    .map(|child_id| ProjectionChildLink {
                        chunk_id: node_chunk_id(*child_id),
                        root_id: node_root_id(*child_id),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        chunk_metadata.push(ProjectionChunkMetadata {
            id: node_chunk_id(node_id),
            parent_id: Some(parent_id),
            root_id: Some(node_root_id(node_id)),
            byte_offset: start as u64,
            byte_length,
            child_links,
            source_map_deltas: source_map_deltas(ast_node_source_map(node)),
        });
    }
    EncodedBinaryProjection {
        bytes: out,
        chunk_metadata,
    }
}

fn encode_events_binary(input: &[u8], from_format: InputFormat) -> EncodedBinaryProjection {
    let src = BytesSource::new(SourceId(1), input.to_vec());
    let mut events = match from_format {
        InputFormat::Cem => collect_normalized_events(CemTokenizer::from_source(src)),
        InputFormat::Html => collect_normalized_events(HtmlTokenizer::from_source(src)),
        InputFormat::Xml => collect_normalized_events(XmlTokenizer::from_source(src)),
    };
    let mut out = Vec::new();
    write_binary_header(&mut out, BinaryProjectionKind::Events);
    write_u32(&mut out, events.len() as u32);
    let header_len = out.len();
    let mut chunk_metadata = Vec::with_capacity(events.len().saturating_add(1));
    chunk_metadata.push(ProjectionChunkMetadata {
        id: ROOT_CHUNK_ID.to_owned(),
        parent_id: None,
        root_id: Some("projection:events".to_owned()),
        byte_offset: 0,
        byte_length: header_len,
        child_links: (0..events.len())
            .map(|sequence| ProjectionChildLink {
                chunk_id: event_chunk_id(sequence as u64),
                root_id: event_root_id(sequence as u64),
            })
            .collect(),
        source_map_deltas: Vec::new(),
    });
    for (sequence, event) in events.drain(..).enumerate() {
        let start = out.len();
        encode_event(&mut out, sequence as u64, &event);
        let byte_length = out.len() - start;
        chunk_metadata.push(ProjectionChunkMetadata {
            id: event_chunk_id(sequence as u64),
            parent_id: Some(ROOT_CHUNK_ID.to_owned()),
            root_id: Some(event_root_id(sequence as u64)),
            byte_offset: start as u64,
            byte_length,
            child_links: Vec::new(),
            source_map_deltas: event_source_map_deltas(&event),
        });
    }
    EncodedBinaryProjection {
        bytes: out,
        chunk_metadata,
    }
}

fn write_binary_header(out: &mut Vec<u8>, kind: BinaryProjectionKind) {
    out.extend_from_slice(BINARY_MAGIC);
    write_u16(out, BINARY_VERSION);
    write_u8(out, kind.tag());
    write_str(out, kind.schema());
    write_str(out, kind.content_type());
}

fn encode_ast_node(out: &mut Vec<u8>, node: &CemAstNode) {
    match node {
        CemAstNode::Document {
            node_id,
            root_children,
            source,
        } => {
            write_u8(out, 1);
            write_u32(out, *node_id);
            write_source_range(out, stack_origin(source));
            write_id_list(out, root_children);
        }
        CemAstNode::Element {
            node_id,
            expanded_name,
            attributes,
            children,
            has_explicit_boundary,
            source,
        } => {
            write_u8(out, 2);
            write_u32(out, *node_id);
            write_source_range(out, stack_origin(source));
            write_expanded_name(out, expanded_name);
            write_bool(out, *has_explicit_boundary);
            write_id_list(out, attributes);
            write_id_list(out, children);
        }
        CemAstNode::Attribute {
            node_id,
            expanded_name,
            value,
            source,
        } => {
            write_u8(out, 3);
            write_u32(out, *node_id);
            write_source_range(out, stack_origin(source));
            write_expanded_name(out, expanded_name);
            write_optional_str(out, value.as_deref());
        }
        CemAstNode::Text {
            node_id,
            data,
            source,
        } => encode_text_node(out, 4, *node_id, data, source),
        CemAstNode::Whitespace {
            node_id,
            data,
            source,
        } => encode_text_node(out, 5, *node_id, data, source),
        CemAstNode::Comment {
            node_id,
            data,
            source,
        } => encode_text_node(out, 6, *node_id, data, source),
        CemAstNode::ProcessingInstruction {
            node_id,
            target,
            data,
            source,
        } => {
            write_u8(out, 7);
            write_u32(out, *node_id);
            write_source_range(out, stack_origin(source));
            write_str(out, target);
            write_str(out, data);
        }
        CemAstNode::Cdata {
            node_id,
            data,
            source,
        } => encode_text_node(out, 8, *node_id, data, source),
        CemAstNode::RawText {
            node_id,
            data,
            source,
        } => encode_text_node(out, 9, *node_id, data, source),
        CemAstNode::Error {
            node_id,
            code,
            source,
        } => encode_text_node(out, 10, *node_id, code, source),
    }
}

fn encode_text_node(
    out: &mut Vec<u8>,
    tag: u8,
    node_id: AstNodeId,
    data: &str,
    source: &SourceMapStack,
) {
    write_u8(out, tag);
    write_u32(out, node_id);
    write_source_range(out, stack_origin(source));
    write_str(out, data);
}

fn write_expanded_name(out: &mut Vec<u8>, name: &ExpandedName) {
    write_str(out, &name.namespace_uri);
    write_str(out, &name.local_name);
    match name.schema_id {
        Some(schema_id) => {
            write_bool(out, true);
            write_u32(out, schema_id);
        }
        None => write_bool(out, false),
    }
}

fn encode_event(out: &mut Vec<u8>, sequence: u64, event: &NormalizedEvent) {
    write_u64(out, sequence);
    match event {
        NormalizedEvent::OpenScope {
            name, byte_range, ..
        } => {
            write_u8(out, 1);
            write_source_range(out, Some(*byte_range));
            write_qname(out, name);
        }
        NormalizedEvent::CloseScope {
            name,
            byte_range,
            synthesis,
            ..
        } => {
            write_u8(out, 2);
            write_source_range(out, Some(*byte_range));
            write_qname(out, name);
            write_u8(out, synthesis_tag(synthesis));
        }
        NormalizedEvent::Name { name, byte_range } => {
            write_u8(out, 3);
            write_source_range(out, Some(*byte_range));
            write_qname(out, name);
        }
        NormalizedEvent::Value { value, byte_range } => {
            write_u8(out, 4);
            write_source_range(out, Some(*byte_range));
            write_scalar_value(out, value);
        }
        NormalizedEvent::Trivia {
            kind,
            data,
            byte_range,
        } => {
            write_u8(out, 5);
            write_source_range(out, Some(*byte_range));
            write_u8(
                out,
                match kind {
                    TriviaKind::Whitespace => 1,
                    TriviaKind::Comment => 2,
                },
            );
            write_str(out, data);
        }
        NormalizedEvent::ProcessingInstruction {
            target,
            data,
            byte_range,
        } => {
            write_u8(out, 6);
            write_source_range(out, Some(*byte_range));
            write_str(out, target);
            write_str(out, data);
        }
        NormalizedEvent::Separator { kind, byte_range } => {
            write_u8(out, 7);
            write_source_range(out, Some(*byte_range));
            write_u8(out, separator_tag(kind));
        }
        NormalizedEvent::ModeSwitch {
            content_type,
            handoff,
            ..
        } => {
            write_u8(out, 8);
            write_source_range(out, Some(handoff.source_span));
            write_str(out, content_type);
        }
        NormalizedEvent::Error {
            code,
            byte_range,
            severity,
        } => {
            write_u8(out, 9);
            write_source_range(out, Some(*byte_range));
            write_str(out, code);
            write_u8(out, severity_tag(*severity));
        }
    }
}

fn write_qname(out: &mut Vec<u8>, name: &crate::events::QName) {
    write_str(out, &name.lexical_name);
    write_optional_str(out, name.prefix.as_deref());
    write_str(out, &name.local_name);
    write_source_range(out, Some(name.source_range));
}

fn write_scalar_value(out: &mut Vec<u8>, value: &ScalarValue) {
    match value {
        ScalarValue::Text(value) => {
            write_u8(out, 1);
            write_str(out, value);
        }
        ScalarValue::Int(value) => {
            write_u8(out, 2);
            out.extend_from_slice(&value.to_be_bytes());
        }
        ScalarValue::Float(value) => {
            write_u8(out, 3);
            out.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        ScalarValue::Bool(value) => {
            write_u8(out, 4);
            write_bool(out, *value);
        }
        ScalarValue::Null => write_u8(out, 5),
    }
}

fn separator_tag(kind: &SeparatorKind) -> u8 {
    match kind {
        SeparatorKind::ElementBoundary => 1,
        SeparatorKind::Comma => 2,
        SeparatorKind::Colon => 3,
        SeparatorKind::Delimiter => 4,
        SeparatorKind::Newline => 5,
    }
}

fn synthesis_tag(synthesis: &Synthesis) -> u8 {
    match synthesis {
        Synthesis::Real => 1,
        Synthesis::SelfClosing => 2,
        Synthesis::VoidElement => 3,
        Synthesis::ImpliedByStartTag => 4,
        Synthesis::ImpliedByAncestorClose => 5,
        Synthesis::ImpliedByEof => 6,
    }
}

fn severity_tag(severity: crate::diagnostics::Severity) -> u8 {
    match severity {
        crate::diagnostics::Severity::Info => 1,
        crate::diagnostics::Severity::Warning => 2,
        crate::diagnostics::Severity::Error => 3,
        crate::diagnostics::Severity::Fatal => 4,
    }
}

fn collect_normalized_events<T: SchemaTokenizer>(tok: T) -> Vec<NormalizedEvent> {
    let mut n = CemEventNormalizer::new(tok);
    let mut out = Vec::new();
    while let Some(ev) = n.next_event() {
        out.push(ev);
    }
    out
}

fn write_id_list(out: &mut Vec<u8>, ids: &[AstNodeId]) {
    write_u32(out, ids.len() as u32);
    for id in ids {
        write_u32(out, *id);
    }
}

fn write_source_range(out: &mut Vec<u8>, range: Option<ByteRange>) {
    match range {
        Some(range) => {
            write_bool(out, true);
            write_u64(out, range.start);
            write_u32(out, range.len);
        }
        None => write_bool(out, false),
    }
}

fn write_optional_str(out: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            write_bool(out, true);
            write_str(out, value);
        }
        None => write_bool(out, false),
    }
}

fn write_str(out: &mut Vec<u8>, value: &str) {
    write_u32(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}

fn write_bool(out: &mut Vec<u8>, value: bool) {
    write_u8(out, u8::from(value));
}

fn write_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Project a built `CemDocument` to a DOM-JSON tree:
///
/// ```json
/// {
///   "kind": "document",
///   "children": [
///     {
///       "kind": "element",
///       "name": "main",
///       "namespace": "",
///       "attributes": [{ "name": "cem:screen", "value": "login", "namespace": "cem" }],
///       "children": [...],
///       "byteRange": { "start": 130, "len": 12 }
///     }
///   ]
/// }
/// ```
pub fn dom_json(doc: &CemDocument) -> Value {
    let root = doc.root().cloned();
    match root {
        Some(CemAstNode::Document { root_children, .. }) => json!({
            "kind": "document",
            "children": root_children.iter().filter_map(|id| project_node(doc, *id)).collect::<Vec<_>>(),
        }),
        _ => Value::Null,
    }
}

/// Project root document children as CEM tree formatter input.
///
/// This intentionally differs from `dom_json`: it preserves the full
/// `sourceMap` stacks that the CEMT formatter/colorizer/writer pipeline uses
/// for output spans, and it emits namespace-qualified names as writer-ready
/// CEM names.
pub fn cem_tree_nodes_json(doc: &CemDocument) -> Value {
    cem_tree_nodes_json_with_source_content_type(doc, None)
}

pub fn cem_tree_nodes_json_with_source_content_type(
    doc: &CemDocument,
    source_content_type: Option<&str>,
) -> Value {
    match doc.root() {
        Some(CemAstNode::Document { root_children, .. }) => Value::Array(
            root_children
                .iter()
                .filter_map(|id| project_cem_tree_node(doc, *id, source_content_type))
                .collect(),
        ),
        _ => Value::Array(Vec::new()),
    }
}

fn project_node(doc: &CemDocument, id: AstNodeId) -> Option<Value> {
    let node = doc.get(id)?;
    let value = match node {
        CemAstNode::Document { root_children, .. } => json!({
            "kind": "document",
            "children": root_children.iter().filter_map(|id| project_node(doc, *id)).collect::<Vec<_>>(),
        }),
        CemAstNode::Element {
            expanded_name,
            attributes,
            children,
            source,
            ..
        } => {
            let attrs: Vec<Value> = attributes
                .iter()
                .filter_map(|aid| match doc.get(*aid)? {
                    CemAstNode::Attribute {
                        expanded_name,
                        value,
                        ..
                    } => Some(json!({
                        "name": expanded_name.local_name,
                        "namespace": expanded_name.namespace_uri,
                        "value": value,
                    })),
                    _ => None,
                })
                .collect();
            json!({
                "kind": "element",
                "name": expanded_name.local_name,
                "namespace": expanded_name.namespace_uri,
                "attributes": attrs,
                "children": children.iter().filter_map(|cid| project_node(doc, *cid)).collect::<Vec<_>>(),
                "byteRange": project_byte_range(source.frames.first().and_then(|f| match &f.span {
                    crate::source_map::FrameSpan::Single(r) => Some(*r),
                    crate::source_map::FrameSpan::Multi(rs) => rs.first().copied(),
                })),
            })
        }
        CemAstNode::Text { data, source, .. } => json!({
            "kind": "text",
            "data": data,
            "byteRange": project_byte_range(stack_origin(source)),
        }),
        CemAstNode::Whitespace { data, source, .. } => json!({
            "kind": "whitespace",
            "data": data,
            "byteRange": project_byte_range(stack_origin(source)),
        }),
        CemAstNode::Comment { data, source, .. } => json!({
            "kind": "comment",
            "data": data,
            "byteRange": project_byte_range(stack_origin(source)),
        }),
        CemAstNode::ProcessingInstruction {
            target,
            data,
            source,
            ..
        } => json!({
            "kind": "processing-instruction",
            "name": target,
            "target": target,
            "data": data,
            "byteRange": project_byte_range(stack_origin(source)),
        }),
        CemAstNode::Cdata { data, source, .. } => json!({
            "kind": "cdata",
            "data": data,
            "byteRange": project_byte_range(stack_origin(source)),
        }),
        CemAstNode::RawText { data, source, .. } => json!({
            "kind": "raw-text",
            "data": data,
            "byteRange": project_byte_range(stack_origin(source)),
        }),
        CemAstNode::Error { code, source, .. } => json!({
            "kind": "error",
            "code": code,
            "byteRange": project_byte_range(stack_origin(source)),
        }),
        CemAstNode::Attribute { .. } => return None,
    };
    Some(value)
}

fn project_cem_tree_node(
    doc: &CemDocument,
    id: AstNodeId,
    source_content_type: Option<&str>,
) -> Option<Value> {
    let node = doc.get(id)?;
    let value = match node {
        CemAstNode::Document {
            root_children,
            source,
            ..
        } => json!({
            "kind": "document",
            "children": root_children.iter().filter_map(|id| project_cem_tree_node(doc, *id, source_content_type)).collect::<Vec<_>>(),
            "sourceMap": source_map_value(source, source_content_type),
        }),
        CemAstNode::Element {
            expanded_name,
            attributes,
            children,
            source,
            ..
        } => {
            let mut attrs = attributes
                .iter()
                .filter_map(|aid| project_cem_tree_node(doc, *aid, source_content_type))
                .collect::<Vec<_>>();
            attrs.sort_by(|left, right| {
                let left_name = left.get("name").and_then(Value::as_str).unwrap_or_default();
                let right_name = right
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                (left_name.contains(':'), left_name).cmp(&(right_name.contains(':'), right_name))
            });
            json!({
                "kind": "element",
                "name": projected_expanded_name(expanded_name),
                "attributes": attrs,
                "children": children.iter().filter_map(|cid| project_cem_tree_node(doc, *cid, source_content_type)).collect::<Vec<_>>(),
                "sourceMap": source_map_value(source, source_content_type),
            })
        }
        CemAstNode::Attribute {
            expanded_name,
            value,
            source,
            ..
        } => json!({
            "kind": "attribute",
            "name": projected_expanded_name(expanded_name),
            "value": value,
            "sourceMap": source_map_value(source, source_content_type),
        }),
        CemAstNode::Text { data, source, .. } => json!({
            "kind": "text",
            "value": data,
            "sourceMap": source_map_value(source, source_content_type),
        }),
        CemAstNode::Whitespace { data, source, .. } => json!({
            "kind": "whitespace",
            "data": data,
            "sourceMap": source_map_value(source, source_content_type),
        }),
        CemAstNode::Comment { data, source, .. } => json!({
            "kind": "comment",
            "data": data,
            "sourceMap": source_map_value(source, source_content_type),
        }),
        CemAstNode::ProcessingInstruction {
            target,
            data,
            source,
            ..
        } => json!({
            "kind": "processing-instruction",
            "name": target,
            "target": target,
            "data": data,
            "sourceMap": source_map_value(source, source_content_type),
        }),
        CemAstNode::Cdata { data, source, .. } => json!({
            "kind": "cdata",
            "data": data,
            "sourceMap": source_map_value(source, source_content_type),
        }),
        CemAstNode::RawText { data, source, .. } => json!({
            "kind": "raw-text",
            "data": data,
            "sourceMap": source_map_value(source, source_content_type),
        }),
        CemAstNode::Error { code, source, .. } => json!({
            "kind": "error",
            "code": code,
            "sourceMap": source_map_value(source, source_content_type),
        }),
    };
    Some(value)
}

fn projected_expanded_name(name: &ExpandedName) -> String {
    if name.namespace_uri.is_empty() {
        name.local_name.clone()
    } else {
        format!("{}:{}", name.namespace_uri, name.local_name)
    }
}

fn source_map_value(source: &SourceMapStack, source_content_type: Option<&str>) -> Value {
    serde_json::to_value(source_map_with_content_type_transform(
        source,
        source_content_type,
    ))
    .unwrap_or(Value::Null)
}

fn source_map_with_content_type_transform(
    source: &SourceMapStack,
    source_content_type: Option<&str>,
) -> SourceMapStack {
    let Some(content_type) = source_content_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return source.clone();
    };
    let mut source_map = source.clone();
    let current = source_map.current().cloned();
    source_map.push(SourceMapFrame {
        source_id: current
            .as_ref()
            .map(|frame| frame.source_id)
            .unwrap_or(SourceId(0)),
        span: current
            .map(|frame| frame.span)
            .unwrap_or_else(|| FrameSpan::Single(ByteRange::new(0, 0))),
        transform: TransformKind::ContentTypeTransform {
            content_type: content_type.to_owned(),
        },
    });
    source_map
}

fn stack_origin(stack: &SourceMapStack) -> Option<ByteRange> {
    stack.frames.first().and_then(|f| match &f.span {
        FrameSpan::Single(r) => Some(*r),
        FrameSpan::Multi(rs) => rs.first().copied(),
    })
}

fn source_map_deltas(stack: &SourceMapStack) -> Vec<ProjectionSourceMapDelta> {
    stack
        .frames
        .iter()
        .flat_map(|frame| {
            let ranges = match &frame.span {
                FrameSpan::Single(range) => vec![*range],
                FrameSpan::Multi(ranges) => ranges.clone(),
            };
            ranges
                .into_iter()
                .map(|range| ProjectionSourceMapDelta {
                    source_id: frame.source_id.0,
                    byte_offset: range.start,
                    byte_length: range.len,
                    transform: transform_kind_label(&frame.transform).to_owned(),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn event_source_map_deltas(event: &NormalizedEvent) -> Vec<ProjectionSourceMapDelta> {
    event_source_range(event)
        .map(|range| {
            vec![ProjectionSourceMapDelta {
                source_id: SourceId(1).0,
                byte_offset: range.start,
                byte_length: range.len,
                transform: "event-normalizer".to_owned(),
            }]
        })
        .unwrap_or_default()
}

fn event_source_range(event: &NormalizedEvent) -> Option<ByteRange> {
    match event {
        NormalizedEvent::OpenScope { byte_range, .. }
        | NormalizedEvent::CloseScope { byte_range, .. }
        | NormalizedEvent::Name { byte_range, .. }
        | NormalizedEvent::Value { byte_range, .. }
        | NormalizedEvent::Trivia { byte_range, .. }
        | NormalizedEvent::ProcessingInstruction { byte_range, .. }
        | NormalizedEvent::Separator { byte_range, .. }
        | NormalizedEvent::Error { byte_range, .. } => Some(*byte_range),
        NormalizedEvent::ModeSwitch { handoff, .. } => Some(handoff.source_span),
    }
}

fn transform_kind_label(transform: &TransformKind) -> &'static str {
    match transform {
        TransformKind::HtmlTokenizer => "html-tokenizer",
        TransformKind::XmlTokenizer => "xml-tokenizer",
        TransformKind::CemTokenizer => "cem-tokenizer",
        TransformKind::EventNormalizer => "event-normalizer",
        TransformKind::SchemaValidation { .. } => "schema-validation",
        TransformKind::CemAstBuilder => "cem-ast-builder",
        TransformKind::HandoffBoundary { .. } => "handoff-boundary",
        TransformKind::ContentTypeTransform { .. } => "content-type-transform",
        TransformKind::InterpreterRender => "interpreter-render",
        TransformKind::Query => "query",
        TransformKind::QueryStep => "query-step",
        TransformKind::TemplateEmbedding { .. } => "template-embedding",
        TransformKind::TemplateTransform { .. } => "template-transform",
    }
}

fn project_byte_range(range: Option<ByteRange>) -> Value {
    match range {
        Some(r) => json!({ "start": r.start, "len": r.len }),
        None => Value::Null,
    }
}

/// Project the parsed AST as a typed-tree JSON (alias for `dom_json` in
/// Tier A; future CEM-specific projections add the `annotations` /
/// `state` fields here).
pub fn ast_json(doc: &CemDocument) -> Value {
    dom_json(doc)
}

/// Project the input source as a flat list of normalized events:
///
/// ```json
/// [
///   { "kind": "open", "name": "main", "byteRange": {...} },
///   { "kind": "name", "name": "cem:screen" },
///   { "kind": "value", "value": "login" },
///   { "kind": "close", "name": "main" }
/// ]
/// ```
pub fn events_json(input: &[u8]) -> Value {
    events_json_as(input, InputFormat::Cem)
}

pub fn events_json_as(input: &[u8], from_format: InputFormat) -> Value {
    let src = BytesSource::new(SourceId(1), input.to_vec());
    match from_format {
        InputFormat::Cem => collect_events(CemTokenizer::from_source(src)),
        InputFormat::Html => collect_events(HtmlTokenizer::from_source(src)),
        InputFormat::Xml => collect_events(XmlTokenizer::from_source(src)),
    }
}

fn collect_events<T: SchemaTokenizer>(tok: T) -> Value {
    let mut n = CemEventNormalizer::new(tok);
    let mut out: Vec<Value> = Vec::new();
    while let Some(ev) = n.next_event() {
        out.push(event_to_json(&ev));
    }
    Value::Array(out)
}

fn event_to_json(ev: &NormalizedEvent) -> Value {
    match ev {
        NormalizedEvent::OpenScope {
            name, byte_range, ..
        } => json!({
            "kind": "open",
            "name": name.lexical_name,
            "byteRange": project_byte_range(Some(*byte_range)),
        }),
        NormalizedEvent::CloseScope {
            name, byte_range, ..
        } => json!({
            "kind": "close",
            "name": name.lexical_name,
            "byteRange": project_byte_range(Some(*byte_range)),
        }),
        NormalizedEvent::Name { name, byte_range } => json!({
            "kind": "name",
            "name": name.lexical_name,
            "byteRange": project_byte_range(Some(*byte_range)),
        }),
        NormalizedEvent::Value { value, byte_range } => {
            let v = match value {
                ScalarValue::Text(t) => Value::String(t.clone()),
                ScalarValue::Int(i) => json!(*i),
                ScalarValue::Float(f) => json!(*f),
                ScalarValue::Bool(b) => Value::Bool(*b),
                ScalarValue::Null => Value::Null,
            };
            json!({
                "kind": "value",
                "value": v,
                "byteRange": project_byte_range(Some(*byte_range)),
            })
        }
        NormalizedEvent::Trivia {
            kind,
            data,
            byte_range,
        } => json!({
            "kind": "trivia",
            "trivia": match kind { TriviaKind::Whitespace => "whitespace", TriviaKind::Comment => "comment" },
            "data": data,
            "byteRange": project_byte_range(Some(*byte_range)),
        }),
        NormalizedEvent::ProcessingInstruction {
            target,
            data,
            byte_range,
        } => json!({
            "kind": "processing-instruction",
            "target": target,
            "data": data,
            "byteRange": project_byte_range(Some(*byte_range)),
        }),
        NormalizedEvent::Separator { byte_range, .. } => json!({
            "kind": "separator",
            "byteRange": project_byte_range(Some(*byte_range)),
        }),
        NormalizedEvent::ModeSwitch { content_type, .. } => json!({
            "kind": "mode-switch",
            "contentType": content_type,
        }),
        NormalizedEvent::Error {
            code, byte_range, ..
        } => json!({
            "kind": "error",
            "code": code,
            "byteRange": project_byte_range(Some(*byte_range)),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::builder::CemAstBuilder;

    fn parse(input: &str) -> CemDocument {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        CemAstBuilder::new(normalizer).build()
    }

    #[test]
    fn dom_json_root_is_document_kind() {
        let doc = parse("{p Hi}");
        let v = dom_json(&doc);
        assert_eq!(v["kind"], "document");
        assert!(v["children"].is_array());
    }

    #[test]
    fn dom_json_element_has_name_and_attributes() {
        let doc = parse(r#"{button @cem:action=primary | Save}"#);
        let v = dom_json(&doc);
        let button = v["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "button")
            .unwrap();
        assert_eq!(button["kind"], "element");
        let attr = &button["attributes"][0];
        assert_eq!(attr["name"], "action");
        assert_eq!(attr["namespace"], "cem");
        assert_eq!(attr["value"], "primary");
    }

    #[test]
    fn cem_tree_nodes_json_preserves_writer_names_and_source_map_transform() {
        let doc = parse(r#"{button @cem:action=primary @type=submit | Save}"#);
        let v = cem_tree_nodes_json_with_source_content_type(&doc, Some("text/html"));
        let nodes = v.as_array().expect("CEM tree projection is node array");
        let button = &nodes[0];

        assert_eq!(button["kind"], "element");
        assert_eq!(button["name"], "button");
        assert_eq!(button["attributes"][0]["name"], "type");
        assert_eq!(button["attributes"][1]["name"], "cem:action");
        let text = button["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|child| child["kind"] == "text")
            .expect("projected text child");
        assert_eq!(text["value"], "Save");
        assert!(text["sourceMap"]["frames"]
            .as_array()
            .unwrap()
            .iter()
            .any(|frame| {
                frame["transform"]["kind"] == "ContentTypeTransform"
                    && frame["transform"]["content_type"] == "text/html"
            }));
    }

    #[test]
    fn dom_json_processing_instruction_exposes_name_alias() {
        let doc = parse(r#"<?xml-stylesheet href="main.css"?>{p Hi}"#);
        let v = dom_json(&doc);
        let pi = v["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|child| child["kind"] == "processing-instruction")
            .unwrap();

        assert_eq!(pi["name"], "xml-stylesheet");
        assert_eq!(pi["target"], "xml-stylesheet");
        assert_eq!(pi["data"], "href=\"main.css\"");
    }

    #[test]
    fn events_json_open_close_round_trip() {
        let v = events_json(b"{p Hi}");
        let arr = v.as_array().unwrap();
        let opens: Vec<&Value> = arr.iter().filter(|e| e["kind"] == "open").collect();
        let closes: Vec<&Value> = arr.iter().filter(|e| e["kind"] == "close").collect();
        assert_eq!(opens.len(), closes.len());
        assert_eq!(opens[0]["name"], "p");
    }

    #[test]
    fn dom_binary_artifact_is_hash_addressed_multi_chunk_envelope() {
        let doc = parse("{p Hi}");
        let v = dom_binary_artifact(&doc);
        let chunks = v["chunks"].as_array().expect("chunks array");
        let root = &chunks[0];
        let document = chunks
            .iter()
            .find(|chunk| chunk["id"] == "node-0")
            .expect("document node chunk");
        let text = chunks
            .iter()
            .find(|chunk| chunk["rootId"] == "node:2")
            .expect("text node chunk");

        assert_eq!(v["kind"], "cem-binary-projection");
        assert_eq!(v["projection"], "dom");
        assert_eq!(v["contentType"], CEM_DOM_PROJECTION_CONTENT_TYPE);
        assert_eq!(v["hashScheme"], HASH_SCHEME);
        assert!(chunks.len() > 1);
        assert_eq!(root["id"], ROOT_CHUNK_ID);
        assert_eq!(root["rootId"], "projection:dom");
        assert_eq!(root["sealed"], true);
        assert_eq!(root["byteOffset"], 0);
        assert!(root["byteLength"].as_u64().unwrap() < v["byteLength"].as_u64().unwrap());
        assert_eq!(root["childLinks"][0]["chunkId"], "node-0");
        assert_eq!(document["parentId"], ROOT_CHUNK_ID);
        assert_eq!(document["childLinks"][0]["chunkId"], "node-1");
        assert_eq!(text["sourceMapDeltas"][0]["transform"], "cem-tokenizer");
        assert!(v["hash"].as_str().unwrap().starts_with("cem-bin/1+blake3:"));
        assert!(root["data"]
            .as_str()
            .unwrap()
            .starts_with("43454d50524f4a00"));
    }

    #[test]
    fn binary_projection_metadata_omits_json_chunk_data() {
        let doc = parse("{p Hi}");
        let artifact = dom_binary_projection_artifact(&doc);
        let v = artifact.to_metadata_json();

        assert_eq!(v["kind"], "cem-binary-projection");
        assert_eq!(v["projection"], "dom");
        assert_eq!(v["contentType"], CEM_DOM_PROJECTION_CONTENT_TYPE);
        assert_eq!(v["nativeBytes"], true);
        assert_eq!(v["byteLength"], artifact.bytes.len());
        assert!(v.get("chunks").is_none());
    }

    #[test]
    fn binary_projection_stream_exposes_stable_multi_chunk_contract() {
        let doc = parse("{p Hi}");
        let artifact = dom_binary_projection_artifact(&doc);
        let stream = artifact.to_chunk_stream();
        let root = &stream.chunks()[0];
        let document = stream.chunk_by_root_id("node:0").expect("document chunk");
        let element = stream.chunk_by_id("node-1").expect("element chunk");
        let text = stream.chunk_by_root_id("node:2").expect("text chunk");

        assert_eq!(stream.projection, "dom");
        assert_eq!(stream.content_type, CEM_DOM_PROJECTION_CONTENT_TYPE);
        assert_eq!(stream.byte_length, artifact.bytes.len());
        assert_eq!(stream.replay_bytes(), artifact.bytes);
        assert!(stream.chunks().len() > 1);
        assert_eq!(root.id, ROOT_CHUNK_ID);
        assert_eq!(root.parent_id, None);
        assert_eq!(root.root_id.as_deref(), Some("projection:dom"));
        assert_eq!(root.byte_offset, 0);
        assert!(root.byte_length < artifact.bytes.len());
        assert_eq!(root.child_links[0].chunk_id, "node-0");
        assert_eq!(document.parent_id.as_deref(), Some(ROOT_CHUNK_ID));
        assert_eq!(document.child_links[0].chunk_id, "node-1");
        assert_eq!(element.parent_id.as_deref(), Some("node-0"));
        assert_eq!(element.child_links[0].chunk_id, "node-2");
        assert_eq!(text.parent_id.as_deref(), Some("node-1"));
        assert!(!text.source_map_deltas.is_empty());
        assert!(text.hash.starts_with("cem-bin/1+blake3:"));
    }

    #[test]
    fn projection_stream_multicast_reuses_sealed_chunk_bytes() {
        let doc = parse("{p Hi}");
        let artifact = dom_binary_projection_artifact(&doc);
        let stream = artifact.to_chunk_stream();
        let routed = route_projection_stream(
            &stream,
            &[
                ProjectionStreamRoute::new("primary"),
                ProjectionStreamRoute::new("cache"),
            ],
            ProjectionRouteMode::Deterministic,
        )
        .unwrap();

        assert_eq!(routed.len(), 2);
        assert_eq!(routed[0].sink_id, "primary");
        assert_eq!(routed[1].sink_id, "cache");
        assert_eq!(routed[0].hash_scheme, HASH_SCHEME);
        assert_eq!(routed[0].concatenated_bytes(), artifact.bytes);
        assert_eq!(routed[1].concatenated_bytes(), artifact.bytes);
        assert!(std::sync::Arc::ptr_eq(
            &routed[0].chunks[0].shared_bytes(),
            &routed[1].chunks[0].shared_bytes()
        ));
    }

    #[test]
    fn projection_stream_parallel_routing_preserves_route_order() {
        let doc = parse("{button @cem:action=primary | Save}");
        let artifact = ast_binary_projection_artifact(&doc);
        let routed = route_projection_stream(
            &artifact.to_chunk_stream(),
            &[
                ProjectionStreamRoute::new("stdout"),
                ProjectionStreamRoute::new("cache"),
                ProjectionStreamRoute::new("observer"),
            ],
            ProjectionRouteMode::Parallel,
        )
        .unwrap();

        let sink_ids = routed
            .iter()
            .map(|route| route.sink_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(sink_ids, vec!["stdout", "cache", "observer"]);
        assert!(routed
            .iter()
            .all(|route| route.concatenated_bytes() == artifact.bytes));
        assert!(std::sync::Arc::ptr_eq(
            &routed[0].chunks[0].shared_bytes(),
            &routed[2].chunks[0].shared_bytes()
        ));
    }

    #[test]
    fn ast_binary_artifact_is_deterministic_for_same_document() {
        let doc = parse("{button @cem:action=primary | Save}");
        let first = ast_binary_artifact(&doc);
        let second = ast_binary_artifact(&doc);

        assert_eq!(first["projection"], "ast");
        assert_eq!(first["contentType"], CEM_AST_PROJECTION_CONTENT_TYPE);
        assert_eq!(first["hash"], second["hash"]);
        assert_eq!(first["chunks"], second["chunks"]);
        assert!(first["chunks"].as_array().unwrap().len() > 1);
    }

    #[test]
    fn events_binary_artifact_encodes_event_stream() {
        let v = events_binary_artifact_as(b"{p Hi}", InputFormat::Cem);
        let chunks = v["chunks"].as_array().expect("chunks array");

        assert_eq!(v["projection"], "events");
        assert_eq!(v["contentType"], CEM_EVENTS_PROJECTION_CONTENT_TYPE);
        assert!(v["byteLength"].as_u64().unwrap() > BINARY_MAGIC.len() as u64);
        assert!(chunks.len() > 1);
        assert_eq!(chunks[0]["rootId"], "projection:events");
        assert_eq!(chunks[0]["childLinks"][0]["chunkId"], "event-0");
        assert_eq!(chunks[1]["parentId"], ROOT_CHUNK_ID);
        assert_eq!(chunks[1]["rootId"], "event:0");
        assert_eq!(
            chunks[1]["sourceMapDeltas"][0]["transform"],
            "event-normalizer"
        );
        assert!(chunks[0]["data"]
            .as_str()
            .unwrap()
            .starts_with("43454d50524f4a00"));
    }
}
