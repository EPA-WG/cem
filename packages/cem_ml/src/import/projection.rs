//! Explicit external projection admission. Original artifacts are retained;
//! reported source maps, schema IDs and runtime handles supply no authority.
use super::*;
use crate::schema::registry::*;
mod binary;
mod json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionKind {
    Dom,
    Ast,
    Events,
}
impl ProjectionKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Dom => "dom",
            Self::Ast => "ast",
            Self::Events => "events",
        }
    }
    pub fn tag(self) -> u8 {
        match self {
            Self::Dom => 1,
            Self::Ast => 2,
            Self::Events => 3,
        }
    }
    pub fn schema(self) -> &'static str {
        match self {
            Self::Dom => CEM_DOM_PROJECTION_SCHEMA_URI,
            Self::Ast => CEM_AST_PROJECTION_SCHEMA_URI,
            Self::Events => CEM_EVENTS_PROJECTION_SCHEMA_URI,
        }
    }
    pub fn binary_mime(self) -> &'static str {
        match self {
            Self::Dom => CEM_DOM_PROJECTION_CONTENT_TYPE,
            Self::Ast => CEM_AST_PROJECTION_CONTENT_TYPE,
            Self::Events => CEM_EVENTS_PROJECTION_CONTENT_TYPE,
        }
    }
    pub fn json_mime(self) -> &'static str {
        match self {
            Self::Dom => CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
            Self::Ast => CEM_AST_JSON_PROJECTION_CONTENT_TYPE,
            Self::Events => CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
        }
    }
    pub fn identity(schema: Option<&str>, mime: Option<&str>) -> Option<Self> {
        [Self::Dom, Self::Ast, Self::Events]
            .into_iter()
            .find(|kind| {
                schema.is_some_and(|s| {
                    s.trim() == kind.schema()
                        || *kind == Self::Dom
                            && s.trim() == crate::lifecycle::DOM_JSON_PROJECTION_SCHEMA
                }) || mime.is_some_and(|m| {
                    let m = content_type_essence(m);
                    m == kind.binary_mime() || m == kind.json_mime()
                })
            })
    }
}
#[derive(Debug)]
pub struct ProjectionDocument {
    pub kind: ProjectionKind,
    pub uri: String,
    pub content_type: String,
    pub tree: Arc<RetainedCemTree>,
}
#[derive(Debug)]
struct ProjectionSourceOwner {
    _bytes: Arc<[u8]>,
    _json: Option<Arc<crate::validation::json::JsonDocumentAst>>,
}
#[derive(Debug)]
pub struct ProjectionError {
    pub suffix: &'static str,
    pub message: String,
}
impl ProjectionError {
    fn new(suffix: &'static str, message: impl Into<String>) -> Self {
        Self {
            suffix,
            message: message.into(),
        }
    }
    pub fn diagnostic(self, kind: ProjectionKind, uri: &str) -> Diagnostic {
        Diagnostic {
            uri: Some(uri.into()),
            code: format!("cem.projection.{}.{}", kind.name(), self.suffix),
            message: self.message,
            severity: crate::diagnostics::Severity::Error,
            ..Default::default()
        }
    }
}
impl std::fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.suffix, self.message)
    }
}
type Result<T> = std::result::Result<T, ProjectionError>;
const LIMIT: usize = MAX_DOCUMENT_VALUES;
const DEPTH: usize = 64;

pub fn load(
    bytes: &[u8],
    uri: &str,
    content_type: Option<&str>,
    kind: ProjectionKind,
) -> Result<Arc<ProjectionDocument>> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(ProjectionError::new(
            "source_limit",
            "Projection exceeds the 16 MiB source limit",
        ));
    }
    let mime = content_type.map(content_type_essence);
    let binary = match mime.as_deref() {
        Some(m) if m == kind.binary_mime() => true,
        Some(m) if m == kind.json_mime() || matches!(m, "application/json" | "text/json") => false,
        None => bytes.starts_with(b"CEMPROJ\0"),
        _ => {
            return Err(ProjectionError::new(
                "projection_mismatch",
                "Projection source MIME does not match its selected family",
            ))
        }
    };
    if !binary {
        for parameter in content_type.unwrap_or("").split(';').skip(1) {
            if let Some((key, value)) = parameter.trim().split_once('=') {
                if key.trim().eq_ignore_ascii_case("charset")
                    && !value.trim().trim_matches('"').eq_ignore_ascii_case("utf-8")
                {
                    return Err(ProjectionError::new(
                        "json_parse_error",
                        "Projection JSON requires UTF-8",
                    ));
                }
            }
        }
    }
    let (ast, json_owner) = if binary {
        (binary::decode(bytes, kind, uri)?, None)
    } else {
        json::decode(bytes, kind, uri)?
    };
    let owner = Arc::new(ProjectionSourceOwner {
        _bytes: Arc::from(bytes),
        _json: json_owner,
    });
    let mut semantics = CemTreeSemantics {
        source_fingerprint: Some(source_fingerprint(
            uri,
            bytes,
            &["projection-input/1", kind.schema()],
        )),
        ..Default::default()
    };
    if binary {
        semantics
            .unknown_source_lines
            .extend(0..ast.nodes.len() as u32);
    }
    let tree = RetainedCemTree::new(
        ast,
        uri,
        if binary {
            ""
        } else {
            std::str::from_utf8(bytes)
                .map_err(|e| ProjectionError::new("json_parse_error", e.to_string()))?
        },
        semantics,
        Some(owner),
    )
    .map_err(|e| {
        ProjectionError::new(
            if binary {
                "binary_payload"
            } else {
                "json_shape"
            },
            e,
        )
    })?;
    Ok(Arc::new(ProjectionDocument {
        kind,
        uri: uri.into(),
        content_type: content_type
            .unwrap_or(if binary {
                kind.binary_mime()
            } else {
                kind.json_mime()
            })
            .into(),
        tree,
    }))
}
fn source(uri: &str, start: usize, length: usize) -> SourceMapStack {
    SourceMapStack {
        frames: vec![SourceMapFrame {
            source_id: SourceId(1),
            span: FrameSpan::Single(ByteRange::new(start as u64, length as u32)),
            transform: TransformKind::SourceBoundary { uri: uri.into() },
        }],
    }
}
fn set_source(node: &mut CemAstNode, map: SourceMapStack) {
    let target = match node {
        CemAstNode::Document { source, .. }
        | CemAstNode::Element { source, .. }
        | CemAstNode::Attribute { source, .. }
        | CemAstNode::Reference { source, .. }
        | CemAstNode::Text { source, .. }
        | CemAstNode::Whitespace { source, .. }
        | CemAstNode::Comment { source, .. }
        | CemAstNode::ProcessingInstruction { source, .. }
        | CemAstNode::Cdata { source, .. }
        | CemAstNode::RawText { source, .. }
        | CemAstNode::Error { source, .. } => source,
    };
    *target = map;
}
fn name(namespace: String, local_name: String) -> ExpandedName {
    ExpandedName {
        namespace_uri: namespace,
        local_name,
        schema_id: None,
    }
}
fn check_tree(ast: &CemDocument) -> Result<()> {
    if ast.nodes.is_empty() || ast.nodes.len() > LIMIT {
        return Err(ProjectionError::new(
            "source_limit",
            "Projection node count exceeds admission limits",
        ));
    }
    let mut edges = 0usize;
    for node in &ast.nodes {
        edges = edges.saturating_add(match node {
            CemAstNode::Document { root_children, .. } => root_children.len(),
            CemAstNode::Element {
                attributes,
                children,
                ..
            } => attributes.len().saturating_add(children.len()),
            CemAstNode::Attribute { value_nodes, .. } => value_nodes.len(),
            _ => 0,
        });
        if edges >= ast.nodes.len() {
            return Err(ProjectionError::new(
                "binary_payload",
                "Projection owning edges exceed a tree",
            ));
        }
    }
    let mut seen = vec![false; ast.nodes.len()];
    let mut todo = vec![(0u32, 0usize, false)];
    while let Some((id, depth, attribute)) = todo.pop() {
        if depth > DEPTH {
            return Err(ProjectionError::new(
                "source_limit",
                "Projection exceeds 64 levels",
            ));
        }
        let Some(node) = ast.get(id) else {
            return Err(ProjectionError::new(
                "binary_payload",
                "Projection has a dangling node ID",
            ));
        };
        if seen[id as usize] {
            return Err(ProjectionError::new(
                "binary_payload",
                "Projection has cyclic/shared owning edges",
            ));
        }
        seen[id as usize] = true;
        if matches!(node, CemAstNode::Attribute { .. }) != attribute {
            return Err(ProjectionError::new(
                "binary_payload",
                "Invalid child/attribute edge kind",
            ));
        }
        let children = match node {
            CemAstNode::Document { root_children, .. } if id == 0 => root_children,
            CemAstNode::Document { .. } => {
                return Err(ProjectionError::new(
                    "binary_payload",
                    "Only node zero may be a document",
                ))
            }
            CemAstNode::Element {
                attributes,
                children,
                ..
            } => {
                todo.extend(attributes.iter().map(|id| (*id, depth + 1, true)));
                children
            }
            CemAstNode::Attribute { value_nodes, .. } => value_nodes,
            _ => continue,
        };
        todo.extend(children.iter().map(|id| (*id, depth + 1, false)));
    }
    if seen.iter().any(|v| !v) {
        return Err(ProjectionError::new(
            "binary_payload",
            "Projection has unreachable nodes",
        ));
    }
    Ok(())
}
