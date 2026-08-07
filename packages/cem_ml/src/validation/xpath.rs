mod lexer;
mod parser;
mod syntax;

pub use syntax::*;

use crate::diagnostics::{Diagnostic, Severity};
use crate::engine::{FormatIdentity, TransformTemplateKind};
use crate::lifecycle::LoadedInputAstStream;
use crate::resolver::{ResolverPolicy, ResolverRegistry};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::package_sources::builtin_schema_package_source;
use crate::schema::registry::content_type_essence;
pub use crate::schema::registry::{
    XPATH_CONTENT_TYPE, XPATH_RESULT_CONTENT_TYPE, XPATH_SCHEMA_URI,
};
use crate::source::line_index::LineIndex;
use crate::source::{ByteRange, SourceId, SourceRangeProjector};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::transform_artifact::TransformArtifactBody;
use crate::transform_template::{
    TransformTemplateAdapter, TransformTemplateAdapterCapability, TransformTemplateAdapterError,
    TransformTemplateAdapterExecutionPhase, TransformTemplateAdapterResult,
    TransformTemplateCompileRequest, TransformTemplateCompileResponse,
    TransformTemplateCompiledArtifact, TransformTemplateOutputArtifact,
    TransformTemplateRenderRequest, TransformTemplateRenderResponse,
    TransformTemplateRuntimeContext,
};
use crate::validation::xml::{XmlAttributeAst, XmlDocumentAst, XmlEventAst, XmlEventKind};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};
#[cfg(test)]
use xee_xpath_ast::ast as xee_ast;
#[cfg(test)]
use xee_xpath_ast::{Namespaces, VariableNames, XPathParserContext};
#[cfg(test)]
use xee_xpath_lexer::Token as XeeToken;

const XPATH_PACKAGE_ID: &str = "xpath";
const XPATH_FACT_BEHAVIOR: &str = "xpath-report-fact";
pub const XPATH_GRAMMAR_VERSION: &str = "xpath-3.1/cem-ast-1";

#[derive(Debug, Clone, Copy)]
pub struct XPathSourceRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
    /// Optional native projection from these UTF-8 bytes to an owning source.
    /// The scanner and parser consume it directly; no serialized range bridge
    /// or post-parse AST rewrite is performed.
    pub source_range_projector: Option<&'a dyn SourceRangeProjector>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathExpressionSource {
    pub uri: String,
    pub content_type: String,
    pub media_type: String,
    pub parameters: BTreeMap<String, String>,
    pub byte_length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathSourcePosition {
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathSourceRange {
    pub start: XPathSourcePosition,
    pub byte_length: u64,
}

impl XPathSourceRange {
    pub fn new(line: u32, column: u32, byte_offset: u64, byte_length: u64) -> Self {
        Self {
            start: XPathSourcePosition {
                line,
                column,
                byte_offset,
            },
            byte_length,
        }
    }

    fn from_offsets(
        line_index: &LineIndex,
        origin: XPathSourcePosition,
        start: usize,
        end: usize,
    ) -> Self {
        let coordinate = line_index.project(start as u64);
        let line = origin
            .line
            .saturating_add(coordinate.line.saturating_sub(1));
        let column = if coordinate.line == 1 {
            origin
                .column
                .saturating_add(coordinate.column.saturating_sub(1))
        } else {
            coordinate.column
        };
        Self::new(
            line,
            column,
            origin.byte_offset.saturating_add(start as u64),
            end.saturating_sub(start) as u64,
        )
    }

    fn to_cemt_subject(self) -> Value {
        json!({
            "byteOffset": self.start.byte_offset,
            "byteLength": self.byte_length,
            "line": self.start.line,
            "column": self.start.column,
        })
    }

    fn source_map(self, source_id: u32, content_type: &str) -> SourceMapStack {
        SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(source_id),
                span: FrameSpan::Single(ByteRange::new(
                    self.start.byte_offset,
                    u32::try_from(self.byte_length).unwrap_or(u32::MAX),
                )),
                transform: TransformKind::ContentTypeTransform {
                    content_type: content_type.to_owned(),
                },
            }],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct XPathSourceBoundary {
    decoded_byte_offset: usize,
    source_position: XPathSourcePosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct XPathSourceRangeResolver {
    boundaries: Vec<XPathSourceBoundary>,
}

impl XPathSourceRangeResolver {
    fn new(
        source: &str,
        line_index: &LineIndex,
        origin: XPathSourcePosition,
        projector: Option<&dyn SourceRangeProjector>,
    ) -> Option<Self> {
        let mut decoded_boundaries = source
            .char_indices()
            .map(|(offset, _)| offset)
            .collect::<Vec<_>>();
        if decoded_boundaries.last().copied() != Some(source.len()) {
            decoded_boundaries.push(source.len());
        }
        let boundaries = decoded_boundaries
            .into_iter()
            .map(|decoded_byte_offset| {
                let source_position = if let Some(projector) = projector {
                    let projected = projector.project_boundary(decoded_byte_offset as u64)?;
                    XPathSourcePosition {
                        line: projected.line,
                        column: projected.column,
                        byte_offset: projected.byte_offset,
                    }
                } else {
                    XPathSourceRange::from_offsets(
                        line_index,
                        origin,
                        decoded_byte_offset,
                        decoded_byte_offset,
                    )
                    .start
                };
                Some(XPathSourceBoundary {
                    decoded_byte_offset,
                    source_position,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        if boundaries.windows(2).any(|pair| {
            pair[0].decoded_byte_offset >= pair[1].decoded_byte_offset
                || pair[0].source_position.byte_offset >= pair[1].source_position.byte_offset
        }) {
            return None;
        }
        Some(Self { boundaries })
    }

    pub(super) fn range(&self, start: usize, end: usize) -> XPathSourceRange {
        let start_position = self.boundary_for_decoded_offset(start);
        let end_position = self.boundary_for_decoded_offset(end);
        XPathSourceRange {
            start: start_position,
            byte_length: end_position
                .byte_offset
                .checked_sub(start_position.byte_offset)
                .expect("prevalidated XPath source boundaries are monotonic"),
        }
    }

    pub(super) fn decoded_start(&self, range: XPathSourceRange) -> usize {
        self.decoded_offset_for_source_byte(range.start.byte_offset)
    }

    pub(super) fn decoded_end(&self, range: XPathSourceRange) -> usize {
        self.decoded_offset_for_source_byte(
            range
                .start
                .byte_offset
                .checked_add(range.byte_length)
                .expect("XPath source range end must fit in u64"),
        )
    }

    fn boundary_for_decoded_offset(&self, decoded_byte_offset: usize) -> XPathSourcePosition {
        let index = self
            .boundaries
            .binary_search_by_key(&decoded_byte_offset, |boundary| {
                boundary.decoded_byte_offset
            })
            .expect("XPath parser ranges must end on UTF-8 scalar boundaries");
        self.boundaries[index].source_position
    }

    fn decoded_offset_for_source_byte(&self, source_byte_offset: u64) -> usize {
        let index = self
            .boundaries
            .binary_search_by_key(&source_byte_offset, |boundary| {
                boundary.source_position.byte_offset
            })
            .expect("XPath AST ranges must retain projected scalar boundaries");
        self.boundaries[index].decoded_byte_offset
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathTokenKind {
    Keyword,
    Name,
    Number,
    String,
    Operator,
    Punctuation,
    VariableSigil,
    Comment,
    Whitespace,
    Error,
}

impl XPathTokenKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::Name => "name",
            Self::Number => "number",
            Self::String => "string",
            Self::Operator => "operator",
            Self::Punctuation => "punctuation",
            Self::VariableSigil => "variable-sigil",
            Self::Comment => "comment",
            Self::Whitespace => "whitespace",
            Self::Error => "error",
        }
    }

    pub fn role(self) -> &'static str {
        match self {
            Self::Keyword => "syntax.keyword",
            Self::Name => "syntax.name",
            Self::Number => "syntax.number",
            Self::String => "syntax.string",
            Self::Operator => "syntax.operator",
            Self::Punctuation | Self::VariableSigil => "syntax.punctuation",
            Self::Comment => "syntax.comment",
            Self::Whitespace => "syntax.whitespace",
            Self::Error => "syntax.error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathTokenAst {
    pub index: usize,
    pub kind: XPathTokenKind,
    pub lexeme: String,
    pub depth: usize,
    pub source_range: XPathSourceRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathAstEventKind {
    StartExpression,
    Token,
    EndExpression,
}

impl XPathAstEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StartExpression => "start-expression",
            Self::Token => "token",
            Self::EndExpression => "end-expression",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathAstEvent {
    pub index: usize,
    pub kind: XPathAstEventKind,
    pub token_index: Option<usize>,
    pub depth: usize,
    pub source_range: XPathSourceRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathHostNodeKind {
    XmlDocument,
    XmlSubtree,
    XmlElement,
    XmlAttribute,
    XsltAttribute,
    CemtExpressionSlot,
    CemQlExpressionSlot,
}

impl XPathHostNodeKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::XmlDocument => "xml-document",
            Self::XmlSubtree => "xml-subtree",
            Self::XmlElement => "xml-element",
            Self::XmlAttribute => "xml-attribute",
            Self::XsltAttribute => "xslt-attribute",
            Self::CemtExpressionSlot => "cemt-expression-slot",
            Self::CemQlExpressionSlot => "cem-ql-expression-slot",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathHostOwner {
    pub source_id: u32,
    pub source_uri: String,
    pub content_type: Option<String>,
    pub schema_uri: Option<String>,
    pub node_kind: XPathHostNodeKind,
    pub node_id: Option<String>,
    pub source_range: XPathSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathStaticContext {
    pub namespaces: BTreeMap<String, String>,
    pub default_element_namespace: Option<String>,
    pub default_function_namespace: Option<String>,
    pub variable_bindings: BTreeMap<String, String>,
    pub function_bindings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathEvaluationPhase {
    Validate,
    Compile,
    Transform,
    Render,
    Runtime,
}

impl XPathEvaluationPhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Validate => "validate",
            Self::Compile => "compile",
            Self::Transform => "transform",
            Self::Render => "render",
            Self::Runtime => "runtime",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathExpectedResult {
    pub sequence_type: String,
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathExpandedName {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace_uri: Option<String>,
    pub local_name: String,
}

impl XPathExpandedName {
    pub fn new(namespace_uri: Option<impl Into<String>>, local_name: impl Into<String>) -> Self {
        Self {
            namespace_uri: namespace_uri.map(Into::into),
            local_name: local_name.into(),
        }
    }

    pub fn unqualified(local_name: impl Into<String>) -> Self {
        Self {
            namespace_uri: None,
            local_name: local_name.into(),
        }
    }

    fn from_syntax_name(name: &XPathName) -> Self {
        Self {
            namespace_uri: name.namespace_uri.clone(),
            local_name: name.local_name.clone(),
        }
    }

    fn display(&self) -> String {
        self.namespace_uri
            .as_deref()
            .map(|namespace_uri| format!("Q{{{namespace_uri}}}{}", self.local_name))
            .unwrap_or_else(|| self.local_name.clone())
    }
}

pub type XPathVariableBindings = BTreeMap<XPathExpandedName, XPathResultSequence>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum XPathInvocationHost {
    StandaloneTransform,
    Cemt,
    CemQl,
    Xslt,
}

impl XPathInvocationHost {
    fn as_str(self) -> &'static str {
        match self {
            Self::StandaloneTransform => "standalone-transform",
            Self::Cemt => "cemt",
            Self::CemQl => "cem-ql",
            Self::Xslt => "xslt",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum XPathResultItemKind {
    Node,
    Atomic,
    Map,
    Array,
    Function,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum XPathEvaluatorAstInput {
    PackageAst,
    SourceText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum XPathEvaluatorResourceAccess {
    CemResolver,
    Direct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum XPathEvaluatorSourceMapMode {
    ItemOrigins,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathEvaluatorCapabilities {
    pub evaluator_id: String,
    pub evaluator_version: String,
    pub xpath_version: String,
    pub grammar_version: String,
    pub ast_input: XPathEvaluatorAstInput,
    pub resource_access: XPathEvaluatorResourceAccess,
    pub source_map_mode: XPathEvaluatorSourceMapMode,
    pub deterministic: bool,
    pub targets: BTreeSet<String>,
    pub result_item_kinds: BTreeSet<XPathResultItemKind>,
}

impl XPathEvaluatorCapabilities {
    pub fn required(evaluator_id: impl Into<String>, evaluator_version: impl Into<String>) -> Self {
        Self {
            evaluator_id: evaluator_id.into(),
            evaluator_version: evaluator_version.into(),
            xpath_version: "3.1".to_owned(),
            grammar_version: XPATH_GRAMMAR_VERSION.to_owned(),
            ast_input: XPathEvaluatorAstInput::PackageAst,
            resource_access: XPathEvaluatorResourceAccess::CemResolver,
            source_map_mode: XPathEvaluatorSourceMapMode::ItemOrigins,
            deterministic: true,
            targets: BTreeSet::from(["native".to_owned(), "wasm32-unknown-unknown".to_owned()]),
            result_item_kinds: BTreeSet::from([
                XPathResultItemKind::Node,
                XPathResultItemKind::Atomic,
                XPathResultItemKind::Map,
                XPathResultItemKind::Array,
                XPathResultItemKind::Function,
            ]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathEvaluatorIdentity {
    pub evaluator_id: String,
    pub evaluator_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathAtomicValue {
    pub type_name: String,
    pub lexical_value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum XPathResultNodeKind {
    Document,
    Element,
    Attribute,
    Text,
    Comment,
    ProcessingInstruction,
    Namespace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathNativeNodeHandle {
    XmlDocument,
    XmlEvent {
        event_index: usize,
    },
    XmlAttribute {
        event_index: usize,
        attribute_index: usize,
    },
}

#[derive(Clone)]
pub struct XPathNativeNode {
    owner: Arc<LoadedInputAstStream>,
    handle: XPathNativeNodeHandle,
}

impl std::fmt::Debug for XPathNativeNode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("XPathNativeNode")
            .field("owner", &Arc::as_ptr(&self.owner))
            .field("handle", &self.handle)
            .finish()
    }
}

impl PartialEq for XPathNativeNode {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.handle == other.handle
    }
}

impl Eq for XPathNativeNode {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XPathNativeNodeError {
    OwnerIsNotXml,
    XmlEventMissing {
        event_index: usize,
    },
    XmlEventIsNotNode {
        event_index: usize,
    },
    XmlAttributeMissing {
        event_index: usize,
        attribute_index: usize,
    },
}

impl std::fmt::Display for XPathNativeNodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerIsNotXml => write!(formatter, "XPath native node owner is not an XML AST"),
            Self::XmlEventMissing { event_index } => {
                write!(formatter, "XML event `{event_index}` does not exist")
            }
            Self::XmlEventIsNotNode { event_index } => write!(
                formatter,
                "XML event `{event_index}` does not represent an XPath node"
            ),
            Self::XmlAttributeMissing {
                event_index,
                attribute_index,
            } => write!(
                formatter,
                "XML attribute `{attribute_index}` does not exist on event `{event_index}`"
            ),
        }
    }
}

impl std::error::Error for XPathNativeNodeError {}

impl XPathNativeNode {
    pub fn xml_document(owner: Arc<LoadedInputAstStream>) -> Result<Self, XPathNativeNodeError> {
        if !matches!(owner.as_ref(), LoadedInputAstStream::XmlDocument(_)) {
            return Err(XPathNativeNodeError::OwnerIsNotXml);
        }
        Ok(Self {
            owner,
            handle: XPathNativeNodeHandle::XmlDocument,
        })
    }

    pub fn xml_event(
        owner: Arc<LoadedInputAstStream>,
        event_index: usize,
    ) -> Result<Self, XPathNativeNodeError> {
        let LoadedInputAstStream::XmlDocument(document) = owner.as_ref() else {
            return Err(XPathNativeNodeError::OwnerIsNotXml);
        };
        let event = document
            .events
            .get(event_index)
            .ok_or(XPathNativeNodeError::XmlEventMissing { event_index })?;
        if xpath_xml_event_node_kind(event.kind).is_none() {
            return Err(XPathNativeNodeError::XmlEventIsNotNode { event_index });
        }
        Ok(Self {
            owner,
            handle: XPathNativeNodeHandle::XmlEvent { event_index },
        })
    }

    pub fn xml_attribute(
        owner: Arc<LoadedInputAstStream>,
        event_index: usize,
        attribute_index: usize,
    ) -> Result<Self, XPathNativeNodeError> {
        let LoadedInputAstStream::XmlDocument(document) = owner.as_ref() else {
            return Err(XPathNativeNodeError::OwnerIsNotXml);
        };
        let event = document
            .events
            .get(event_index)
            .ok_or(XPathNativeNodeError::XmlEventMissing { event_index })?;
        event
            .attributes
            .get(attribute_index)
            .ok_or(XPathNativeNodeError::XmlAttributeMissing {
                event_index,
                attribute_index,
            })?;
        Ok(Self {
            owner,
            handle: XPathNativeNodeHandle::XmlAttribute {
                event_index,
                attribute_index,
            },
        })
    }

    pub fn owner(&self) -> &Arc<LoadedInputAstStream> {
        &self.owner
    }

    pub fn handle(&self) -> XPathNativeNodeHandle {
        self.handle
    }

    pub fn source_map(&self) -> SourceMapStack {
        match self.handle {
            XPathNativeNodeHandle::XmlDocument => SourceMapStack {
                frames: vec![SourceMapFrame {
                    source_id: SourceId(1),
                    span: FrameSpan::Single(ByteRange::new(
                        0,
                        u32::try_from(self.xml_document_ast().source.byte_length)
                            .unwrap_or(u32::MAX),
                    )),
                    transform: TransformKind::ContentTypeTransform {
                        content_type: self.xml_document_ast().source.media_type.clone(),
                    },
                }],
            },
            XPathNativeNodeHandle::XmlEvent { event_index } => self
                .xml_document_ast()
                .events
                .get(event_index)
                .expect("validated XPath XML event handle")
                .source_range
                .source_map(),
            XPathNativeNodeHandle::XmlAttribute { .. } => self
                .xml_attribute_ast()
                .and_then(|attribute| attribute.value_source_range)
                .unwrap_or_else(|| {
                    self.xml_event_ast()
                        .expect("validated XPath XML attribute owner")
                        .source_range
                })
                .source_map(),
        }
    }

    fn xml_document_ast(&self) -> &XmlDocumentAst {
        let LoadedInputAstStream::XmlDocument(document) = self.owner.as_ref() else {
            unreachable!("XPathNativeNode constructors validate XML owners")
        };
        document
    }

    fn xml_event_ast(&self) -> Option<&XmlEventAst> {
        let event_index = match self.handle {
            XPathNativeNodeHandle::XmlEvent { event_index }
            | XPathNativeNodeHandle::XmlAttribute { event_index, .. } => event_index,
            XPathNativeNodeHandle::XmlDocument => return None,
        };
        self.xml_document_ast().events.get(event_index)
    }

    fn xml_attribute_ast(&self) -> Option<&XmlAttributeAst> {
        let XPathNativeNodeHandle::XmlAttribute {
            event_index,
            attribute_index,
        } = self.handle
        else {
            return None;
        };
        self.xml_document_ast()
            .events
            .get(event_index)?
            .attributes
            .get(attribute_index)
    }

    fn document_root(&self) -> Self {
        Self {
            owner: Arc::clone(&self.owner),
            handle: XPathNativeNodeHandle::XmlDocument,
        }
    }

    fn child_nodes(&self) -> Vec<Self> {
        let document = self.xml_document_ast();
        let (start_index, child_depth, closing_depth) = match self.handle {
            XPathNativeNodeHandle::XmlDocument => (0, 0, None),
            XPathNativeNodeHandle::XmlEvent { event_index } => {
                let Some(event) = document.events.get(event_index) else {
                    return Vec::new();
                };
                if event.kind != XmlEventKind::StartElement {
                    return Vec::new();
                }
                (
                    event_index.saturating_add(1),
                    event.depth.saturating_add(1),
                    Some(event.depth),
                )
            }
            XPathNativeNodeHandle::XmlAttribute { .. } => return Vec::new(),
        };

        let mut children = Vec::new();
        for event in document.events.iter().skip(start_index) {
            if closing_depth
                .is_some_and(|depth| event.kind == XmlEventKind::EndElement && event.depth == depth)
            {
                break;
            }
            if event.depth != child_depth || xpath_xml_event_node_kind(event.kind).is_none() {
                continue;
            }
            if let Ok(node) = Self::xml_event(Arc::clone(&self.owner), event.index) {
                children.push(node);
            }
        }
        children
    }

    fn attribute_nodes(&self) -> Vec<Self> {
        let XPathNativeNodeHandle::XmlEvent { event_index } = self.handle else {
            return Vec::new();
        };
        let Some(event) = self.xml_event_ast() else {
            return Vec::new();
        };
        if !matches!(
            event.kind,
            XmlEventKind::StartElement | XmlEventKind::EmptyElement
        ) {
            return Vec::new();
        }
        event
            .attributes
            .iter()
            .enumerate()
            .filter(|(_, attribute)| {
                attribute.qualified_name != "xmlns" && attribute.prefix.as_deref() != Some("xmlns")
            })
            .filter_map(|(attribute_index, _)| {
                Self::xml_attribute(Arc::clone(&self.owner), event_index, attribute_index).ok()
            })
            .collect()
    }

    fn parent_node(&self) -> Option<Self> {
        let event_index = match self.handle {
            XPathNativeNodeHandle::XmlDocument => return None,
            XPathNativeNodeHandle::XmlAttribute { event_index, .. } => {
                return Self::xml_event(Arc::clone(&self.owner), event_index).ok();
            }
            XPathNativeNodeHandle::XmlEvent { event_index } => event_index,
        };
        let event = self.xml_document_ast().events.get(event_index)?;
        if event.depth == 0 {
            return Some(self.document_root());
        }
        self.xml_document_ast().events[..event_index]
            .iter()
            .rev()
            .find(|candidate| {
                candidate.kind == XmlEventKind::StartElement
                    && candidate.depth.saturating_add(1) == event.depth
            })
            .and_then(|parent| Self::xml_event(Arc::clone(&self.owner), parent.index).ok())
    }

    fn descendant_nodes(&self) -> Vec<Self> {
        let mut descendants = Vec::new();
        let mut pending = self.child_nodes();
        pending.reverse();
        while let Some(node) = pending.pop() {
            let mut children = node.child_nodes();
            children.reverse();
            pending.extend(children);
            descendants.push(node);
        }
        descendants
    }

    fn ancestor_nodes(&self) -> Vec<Self> {
        let mut ancestors = Vec::new();
        let mut current = self.parent_node();
        while let Some(node) = current {
            current = node.parent_node();
            ancestors.push(node);
        }
        ancestors
    }

    fn following_sibling_nodes(&self) -> Vec<Self> {
        if matches!(
            self.handle,
            XPathNativeNodeHandle::XmlDocument | XPathNativeNodeHandle::XmlAttribute { .. }
        ) {
            return Vec::new();
        }
        let Some(parent) = self.parent_node() else {
            return Vec::new();
        };
        let siblings = parent.child_nodes();
        siblings
            .iter()
            .position(|candidate| candidate == self)
            .map(|index| siblings.into_iter().skip(index.saturating_add(1)).collect())
            .unwrap_or_default()
    }

    fn preceding_sibling_nodes(&self) -> Vec<Self> {
        if matches!(
            self.handle,
            XPathNativeNodeHandle::XmlDocument | XPathNativeNodeHandle::XmlAttribute { .. }
        ) {
            return Vec::new();
        }
        let Some(parent) = self.parent_node() else {
            return Vec::new();
        };
        let siblings = parent.child_nodes();
        siblings
            .iter()
            .position(|candidate| candidate == self)
            .map(|index| siblings.into_iter().take(index).rev().collect())
            .unwrap_or_default()
    }

    fn is_ancestor_of(&self, other: &Self) -> bool {
        let mut current = other.parent_node();
        while let Some(node) = current {
            if node == *self {
                return true;
            }
            current = node.parent_node();
        }
        false
    }

    fn following_nodes(&self) -> Vec<Self> {
        let context_order = self.document_order_key();
        self.document_root()
            .descendant_nodes()
            .into_iter()
            .filter(|candidate| {
                candidate.document_order_key() > context_order && !self.is_ancestor_of(candidate)
            })
            .collect()
    }

    fn preceding_nodes(&self) -> Vec<Self> {
        let context_order = self.document_order_key();
        let mut nodes = self
            .document_root()
            .descendant_nodes()
            .into_iter()
            .filter(|candidate| {
                candidate.document_order_key() < context_order && !candidate.is_ancestor_of(self)
            })
            .collect::<Vec<_>>();
        nodes.reverse();
        nodes
    }

    fn document_order_key(&self) -> (usize, usize, usize) {
        match self.handle {
            XPathNativeNodeHandle::XmlDocument => (0, 0, 0),
            XPathNativeNodeHandle::XmlEvent { event_index } => {
                (event_index.saturating_add(1), 0, 0)
            }
            XPathNativeNodeHandle::XmlAttribute {
                event_index,
                attribute_index,
            } => (
                event_index.saturating_add(1),
                1,
                attribute_index.saturating_add(1),
            ),
        }
    }

    pub fn string_value(&self) -> String {
        if let Some(attribute) = self.xml_attribute_ast() {
            return attribute
                .entity_decoded_value
                .clone()
                .unwrap_or_else(|| attribute.value.clone());
        }
        match self.result_node_kind() {
            XPathResultNodeKind::Document | XPathResultNodeKind::Element => self
                .descendant_nodes()
                .into_iter()
                .filter(|node| node.result_node_kind() == XPathResultNodeKind::Text)
                .filter_map(|node| node.xml_event_ast().and_then(|event| event.value.clone()))
                .collect(),
            XPathResultNodeKind::Text
            | XPathResultNodeKind::Comment
            | XPathResultNodeKind::ProcessingInstruction => self
                .xml_event_ast()
                .and_then(|event| event.value.clone())
                .unwrap_or_default(),
            XPathResultNodeKind::Attribute => unreachable!("attributes return above"),
            XPathResultNodeKind::Namespace => String::new(),
        }
    }

    fn result_node_kind(&self) -> XPathResultNodeKind {
        match self.handle {
            XPathNativeNodeHandle::XmlDocument => XPathResultNodeKind::Document,
            XPathNativeNodeHandle::XmlEvent { .. } => xpath_xml_event_node_kind(
                self.xml_event_ast()
                    .expect("validated XPath XML event handle")
                    .kind,
            )
            .expect("validated XPath XML event node kind"),
            XPathNativeNodeHandle::XmlAttribute { .. } => XPathResultNodeKind::Attribute,
        }
    }

    fn source_range(&self) -> XPathSourceRange {
        match self.handle {
            XPathNativeNodeHandle::XmlDocument => {
                XPathSourceRange::new(1, 1, 0, self.xml_document_ast().source.byte_length as u64)
            }
            XPathNativeNodeHandle::XmlEvent { .. } => {
                let range = self
                    .xml_event_ast()
                    .expect("validated XPath XML event handle")
                    .source_range;
                XPathSourceRange::new(
                    range.start.line,
                    range.start.column,
                    range.start.byte_offset,
                    range.byte_length,
                )
            }
            XPathNativeNodeHandle::XmlAttribute { .. } => {
                let range = self
                    .xml_attribute_ast()
                    .and_then(|attribute| attribute.value_source_range)
                    .unwrap_or_else(|| {
                        self.xml_event_ast()
                            .expect("validated XPath XML attribute owner")
                            .source_range
                    });
                XPathSourceRange::new(
                    range.start.line,
                    range.start.column,
                    range.start.byte_offset,
                    range.byte_length,
                )
            }
        }
    }

    fn node_id(&self) -> String {
        match self.handle {
            XPathNativeNodeHandle::XmlDocument => "xml:document".to_owned(),
            XPathNativeNodeHandle::XmlEvent { event_index } => {
                format!("xml:event:{event_index}")
            }
            XPathNativeNodeHandle::XmlAttribute {
                event_index,
                attribute_index,
            } => format!("xml:event:{event_index}:attribute:{attribute_index}"),
        }
    }

    fn expanded_name(&self) -> Option<String> {
        let (local_name, namespace_uri) = if let Some(attribute) = self.xml_attribute_ast() {
            (
                attribute.local_name.as_str(),
                attribute.namespace_uri.as_deref(),
            )
        } else {
            let event = self.xml_event_ast()?;
            (event.local_name.as_deref()?, event.namespace_uri.as_deref())
        };
        Some(match namespace_uri {
            Some(namespace_uri) => format!("{{{namespace_uri}}}{local_name}"),
            None => local_name.to_owned(),
        })
    }

    fn matches_node_test(&self, node_test: &XPathNodeTest) -> bool {
        match node_test {
            XPathNodeTest::Name(name_test) => {
                let (local_name, node_namespace_uri) =
                    if let Some(attribute) = self.xml_attribute_ast() {
                        (
                            attribute.local_name.as_str(),
                            attribute.namespace_uri.as_deref(),
                        )
                    } else {
                        let Some(event) = self.xml_event_ast() else {
                            return false;
                        };
                        if !matches!(
                            event.kind,
                            XmlEventKind::StartElement | XmlEventKind::EmptyElement
                        ) {
                            return false;
                        }
                        let Some(local_name) = event.local_name.as_deref() else {
                            return false;
                        };
                        (local_name, event.namespace_uri.as_deref())
                    };
                match name_test {
                    XPathNameTest::Name(name) => {
                        local_name == name.local_name.as_str()
                            && node_namespace_uri == name.namespace_uri.as_deref()
                    }
                    XPathNameTest::Any => true,
                    XPathNameTest::AnyNamespace {
                        local_name: expected_local_name,
                    } => local_name == expected_local_name,
                    XPathNameTest::Namespace {
                        namespace_uri: expected_namespace_uri,
                    } => node_namespace_uri == Some(expected_namespace_uri.as_str()),
                }
            }
            XPathNodeTest::Kind { kind, .. } => match kind {
                XPathKindTest::Document => self.result_node_kind() == XPathResultNodeKind::Document,
                XPathKindTest::Element | XPathKindTest::SchemaElement => {
                    self.result_node_kind() == XPathResultNodeKind::Element
                }
                XPathKindTest::Attribute | XPathKindTest::SchemaAttribute => {
                    self.result_node_kind() == XPathResultNodeKind::Attribute
                }
                XPathKindTest::ProcessingInstruction => {
                    self.result_node_kind() == XPathResultNodeKind::ProcessingInstruction
                }
                XPathKindTest::Comment => self.result_node_kind() == XPathResultNodeKind::Comment,
                XPathKindTest::Text => self.result_node_kind() == XPathResultNodeKind::Text,
                XPathKindTest::NamespaceNode => {
                    self.result_node_kind() == XPathResultNodeKind::Namespace
                }
                XPathKindTest::AnyNode => true,
            },
        }
    }
}

fn xpath_xml_event_node_kind(kind: XmlEventKind) -> Option<XPathResultNodeKind> {
    match kind {
        XmlEventKind::StartElement | XmlEventKind::EmptyElement => {
            Some(XPathResultNodeKind::Element)
        }
        XmlEventKind::Text | XmlEventKind::Cdata | XmlEventKind::EntityReference => {
            Some(XPathResultNodeKind::Text)
        }
        XmlEventKind::Comment => Some(XPathResultNodeKind::Comment),
        XmlEventKind::ProcessingInstruction => Some(XPathResultNodeKind::ProcessingInstruction),
        XmlEventKind::Declaration | XmlEventKind::EndElement | XmlEventKind::Doctype => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathMapEntry {
    pub key: XPathAtomicValue,
    pub value: XPathResultSequence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathResultSequence {
    pub sequence_type: String,
    pub items: Vec<XPathResultItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum XPathResultItem {
    Node {
        node_kind: XPathResultNodeKind,
        source_id: u32,
        source_uri: String,
        node_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expanded_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source_range: Option<XPathSourceRange>,
        source_map: SourceMapStack,
        #[serde(skip)]
        native_node: Option<XPathNativeNode>,
    },
    Atomic {
        value: XPathAtomicValue,
        source_map: SourceMapStack,
    },
    Map {
        entries: Vec<XPathMapEntry>,
        source_map: SourceMapStack,
    },
    Array {
        members: Vec<XPathResultSequence>,
        source_map: SourceMapStack,
    },
    Function {
        evaluator_id: String,
        function_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        arity: usize,
        signature: String,
        source_map: SourceMapStack,
    },
}

impl XPathResultItem {
    pub fn from_native_node(native_node: XPathNativeNode) -> Self {
        let source_map = native_node.source_map();
        let source_id = source_map
            .frames
            .first()
            .map_or(0, |frame| frame.source_id.0);
        Self::Node {
            node_kind: native_node.result_node_kind(),
            source_id,
            source_uri: native_node.xml_document_ast().source.uri.clone(),
            node_id: native_node.node_id(),
            expanded_name: native_node.expanded_name(),
            source_range: Some(native_node.source_range()),
            source_map,
            native_node: Some(native_node),
        }
    }

    pub fn native_node(&self) -> Option<&XPathNativeNode> {
        match self {
            Self::Node { native_node, .. } => native_node.as_ref(),
            _ => None,
        }
    }

    pub fn kind(&self) -> XPathResultItemKind {
        match self {
            Self::Node { .. } => XPathResultItemKind::Node,
            Self::Atomic { .. } => XPathResultItemKind::Atomic,
            Self::Map { .. } => XPathResultItemKind::Map,
            Self::Array { .. } => XPathResultItemKind::Array,
            Self::Function { .. } => XPathResultItemKind::Function,
        }
    }

    fn source_map(&self) -> &SourceMapStack {
        match self {
            Self::Node { source_map, .. }
            | Self::Atomic { source_map, .. }
            | Self::Map { source_map, .. }
            | Self::Array { source_map, .. }
            | Self::Function { source_map, .. } => source_map,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathResultArtifact {
    pub content_type: String,
    pub schema_uri: String,
    pub xpath_version: String,
    pub grammar_version: String,
    pub invocation_host: XPathInvocationHost,
    pub evaluator: XPathEvaluatorIdentity,
    pub expression_uri: String,
    pub static_context: XPathStaticContext,
    pub resolver_policy_stamp: String,
    pub safety_policy_stamp: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_result: Option<XPathExpectedResult>,
    pub sequence: XPathResultSequence,
    pub source_map: SourceMapStack,
}

#[derive(Debug, Clone, Default)]
pub struct XPathDynamicContext {
    pub context_item: Option<XPathResultItem>,
    pub variable_bindings: XPathVariableBindings,
}

pub struct XPathEvaluationRequest<'a> {
    pub invocation_host: XPathInvocationHost,
    pub expression: &'a XPathExpressionAst,
    pub dynamic_context: XPathDynamicContext,
    pub static_context: XPathStaticContext,
    pub expected_result: Option<XPathExpectedResult>,
    pub resolver_registry: &'a ResolverRegistry,
    pub resolver_policy: &'a ResolverPolicy,
    pub safety_policy_stamp: &'a str,
}

pub trait XPathEvaluatorAdapter: Send + Sync {
    fn capabilities(&self) -> &XPathEvaluatorCapabilities;

    fn evaluate(
        &self,
        request: XPathEvaluationRequest<'_>,
    ) -> Result<XPathResultArtifact, Vec<Diagnostic>>;
}

#[derive(Debug, Clone)]
pub struct CemXPathEvaluator {
    capabilities: XPathEvaluatorCapabilities,
}

impl Default for CemXPathEvaluator {
    fn default() -> Self {
        Self {
            capabilities: XPathEvaluatorCapabilities::required("cem.xpath.native", "0.1.0"),
        }
    }
}

impl XPathEvaluatorAdapter for CemXPathEvaluator {
    fn capabilities(&self) -> &XPathEvaluatorCapabilities {
        &self.capabilities
    }

    fn evaluate(
        &self,
        request: XPathEvaluationRequest<'_>,
    ) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
        let Some(syntax) = request.expression.syntax_ast.as_ref() else {
            return Err(vec![xpath_evaluation_diagnostic(
                request.expression,
                "cem.xpath.evaluation_ast_missing",
                "XPath evaluation requires the package-owned typed syntax AST",
                None,
            )]);
        };
        let sequence = xpath_evaluate_expression_sequence(
            request.expression,
            &syntax.root,
            XPathFocus::outer(request.dynamic_context.context_item.as_ref()),
            &request.dynamic_context.variable_bindings,
        )
        .map_err(|error| vec![error.into_diagnostic(request.expression)])?;
        let artifact = XPathResultArtifact {
            content_type: XPATH_RESULT_CONTENT_TYPE.to_owned(),
            schema_uri: XPATH_SCHEMA_URI.to_owned(),
            xpath_version: "3.1".to_owned(),
            grammar_version: XPATH_GRAMMAR_VERSION.to_owned(),
            invocation_host: request.invocation_host,
            evaluator: XPathEvaluatorIdentity {
                evaluator_id: self.capabilities.evaluator_id.clone(),
                evaluator_version: self.capabilities.evaluator_version.clone(),
            },
            expression_uri: request.expression.source.uri.clone(),
            static_context: request.static_context,
            resolver_policy_stamp: request.resolver_policy.cache_stamp(),
            safety_policy_stamp: request.safety_policy_stamp.to_owned(),
            expected_result: request.expected_result,
            sequence,
            source_map: syntax.root.source_range.source_map(
                request.expression.attachment.source_id(),
                request.expression.source.media_type.as_str(),
            ),
        };
        let violations = validate_xpath_result_artifact(&artifact, &self.capabilities);
        if violations.is_empty() {
            Ok(artifact)
        } else {
            Err(violations
                .into_iter()
                .map(|violation| {
                    xpath_evaluation_diagnostic(
                        request.expression,
                        "cem.xpath.result_artifact_invalid",
                        violation.message,
                        None,
                    )
                })
                .collect())
        }
    }
}

pub const XPATH_TRANSFORM_TEMPLATE_ADAPTER_ID: &str = "cem.xpath-transform-template";

#[derive(Debug, Clone, Default)]
pub struct XPathTransformTemplateAdapter;

#[derive(Debug, Clone)]
struct XPathCompiledTransformPayload {
    expression: Arc<XPathExpressionAst>,
    static_context: XPathStaticContext,
}

impl TransformTemplateAdapter for XPathTransformTemplateAdapter {
    fn id(&self) -> &'static str {
        XPATH_TRANSFORM_TEMPLATE_ADAPTER_ID
    }

    fn kind(&self) -> TransformTemplateKind {
        TransformTemplateKind::XPath
    }

    fn capability(&self) -> TransformTemplateAdapterCapability {
        TransformTemplateAdapterCapability::Executable
    }

    fn matches_template(&self, identity: &FormatIdentity) -> bool {
        identity
            .content_type
            .as_deref()
            .map(content_type_essence)
            .is_some_and(|content_type| {
                matches!(content_type.as_str(), XPATH_CONTENT_TYPE | "text/xpath")
            })
            || identity.schema.as_deref() == Some(XPATH_SCHEMA_URI)
    }

    fn compile(
        &self,
        request: TransformTemplateCompileRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateCompileResponse> {
        if !request.entrypoint.is_implicit() {
            return Err(TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Compile,
                "XPath transforms use the expression root as the implicit entrypoint",
            ));
        }
        if !request.params.is_empty() {
            return Err(TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Compile,
                "XPath parameter-to-XDM bindings are not defined for the standalone transform slice",
            ));
        }

        let expression = Arc::new(xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: &request.template.bytes,
                source_uri: &request.template.uri,
                content_type: request
                    .template
                    .identity
                    .as_ref()
                    .and_then(|identity| identity.content_type.as_deref()),
                source_range_projector: None,
            },
            XPathAttachment::Standalone { source_id: 1 },
        ));
        let diagnostics = validate_xpath_expression_ast(
            expression.as_ref(),
            &XPathSchemaContractCatalog::from_builtin(),
        );
        let static_context = request
            .template
            .identity
            .as_ref()
            .map(|identity| XPathStaticContext {
                namespaces: identity.namespaces.clone(),
                default_element_namespace: identity.default_namespace.clone(),
                default_function_namespace: None,
                variable_bindings: BTreeMap::new(),
                function_bindings: BTreeMap::new(),
            })
            .unwrap_or_default();

        Ok(TransformTemplateCompileResponse {
            artifact: TransformTemplateCompiledArtifact::new(
                self.id(),
                self.kind(),
                request.template.uri.clone(),
                request.template.identity.clone(),
                request.entrypoint.clone(),
                Value::Null,
            )
            .with_parameters(request.params.clone())
            .with_native_payload(XPathCompiledTransformPayload {
                expression,
                static_context,
            }),
            diagnostics,
        })
    }

    fn render_with_runtime(
        &self,
        request: TransformTemplateRenderRequest<'_>,
        runtime: TransformTemplateRuntimeContext<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateRenderResponse> {
        if !request.secondary_inputs.is_empty() {
            return Err(TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Render,
                "secondary XPath transform inputs require an explicit context or variable binding contract",
            ));
        }
        if !request.compiled.parameters().is_empty() {
            return Err(TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Render,
                "XPath parameter-to-XDM bindings are not defined for the standalone transform slice",
            ));
        }
        let payload = request
            .compiled
            .native_payload::<XPathCompiledTransformPayload>()
            .ok_or_else(|| {
                TransformTemplateAdapterError::failed(
                    self.id(),
                    TransformTemplateAdapterExecutionPhase::Render,
                    "compiled XPath template is missing its package-owned expression AST",
                )
            })?;
        let TransformArtifactBody::Lifecycle(owner) = &request.primary_input.body else {
            return Err(TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Render,
                format!(
                    "XPath standalone transforms require a lifecycle XML AST, got `{}`",
                    request.primary_input.body.representation_id()
                ),
            ));
        };
        let context_node = XPathNativeNode::xml_document(Arc::clone(owner)).map_err(|error| {
            TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Render,
                error.to_string(),
            )
        })?;
        let safety_policy_stamp = format!(
            "cem.xpath.transform-safety/1;phase={:?};scope-policy={}",
            request.execution_policy.runtime_phase,
            request.target_scope.policy.as_deref().unwrap_or("default")
        );
        let result = CemXPathEvaluator::default()
            .evaluate(XPathEvaluationRequest {
                invocation_host: XPathInvocationHost::StandaloneTransform,
                expression: payload.expression.as_ref(),
                dynamic_context: XPathDynamicContext {
                    context_item: Some(XPathResultItem::from_native_node(context_node)),
                    variable_bindings: BTreeMap::new(),
                },
                static_context: payload.static_context.clone(),
                expected_result: None,
                resolver_registry: runtime.resolver_registry,
                resolver_policy: runtime.resolver_policy,
                safety_policy_stamp: &safety_policy_stamp,
            })
            .map_err(|diagnostics| {
                TransformTemplateAdapterError::failed(
                    self.id(),
                    TransformTemplateAdapterExecutionPhase::Render,
                    diagnostics
                        .iter()
                        .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
                        .collect::<Vec<_>>()
                        .join("; "),
                )
            })?;
        let source_map = result.source_map.clone();
        let identity = request.target.cloned().unwrap_or_else(|| FormatIdentity {
            content_type: Some(XPATH_RESULT_CONTENT_TYPE.to_owned()),
            schema: Some(XPATH_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        });

        Ok(TransformTemplateRenderResponse {
            output: TransformTemplateOutputArtifact::new(
                request.primary_input.uri.clone(),
                Some(identity),
                TransformArtifactBody::XPathResult(Arc::new(result)),
            )
            .with_metadata(Some(source_map), Vec::new()),
            diagnostics: Vec::new(),
        })
    }
}

pub trait XPathInvocationAdapter: Send + Sync {
    fn host(&self) -> XPathInvocationHost;

    fn invoke(
        &self,
        request: XPathEvaluationRequest<'_>,
    ) -> Result<XPathResultArtifact, Vec<Diagnostic>>;
}

#[derive(Debug, Clone, Default)]
pub struct CemtXPathInvocationAdapter;

impl XPathInvocationAdapter for CemtXPathInvocationAdapter {
    fn host(&self) -> XPathInvocationHost {
        XPathInvocationHost::Cemt
    }

    fn invoke(
        &self,
        request: XPathEvaluationRequest<'_>,
    ) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
        let attachment_matches = matches!(
            &request.expression.attachment,
            XPathAttachment::Host(host)
                if host.owner.node_kind == XPathHostNodeKind::CemtExpressionSlot
        );
        if request.invocation_host != self.host() || !attachment_matches {
            return Err(vec![xpath_evaluation_diagnostic(
                request.expression,
                "cem.xpath.invocation_host_mismatch",
                format!(
                    "XPath {} invocation requires a CEMT-owned typed expression slot",
                    self.host().as_str()
                ),
                request
                    .expression
                    .syntax_ast
                    .as_ref()
                    .map(|syntax| syntax.root.source_range),
            )]);
        }
        CemXPathEvaluator::default().evaluate(request)
    }
}

#[derive(Debug, Clone)]
struct XPathEvaluationError {
    code: &'static str,
    message: String,
    source_range: Option<XPathSourceRange>,
}

impl XPathEvaluationError {
    fn unsupported(message: impl Into<String>, source_range: XPathSourceRange) -> Self {
        Self {
            code: "cem.xpath.evaluation_unsupported",
            message: message.into(),
            source_range: Some(source_range),
        }
    }

    fn dynamic(
        code: &'static str,
        message: impl Into<String>,
        source_range: XPathSourceRange,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            source_range: Some(source_range),
        }
    }

    fn into_diagnostic(self, expression: &XPathExpressionAst) -> Diagnostic {
        xpath_evaluation_diagnostic(expression, self.code, self.message, self.source_range)
    }
}

#[derive(Debug, Clone, Copy)]
struct XPathFocus<'a> {
    context_item: Option<&'a XPathResultItem>,
    position: usize,
    size: usize,
}

impl<'a> XPathFocus<'a> {
    fn outer(context_item: Option<&'a XPathResultItem>) -> Self {
        Self {
            context_item,
            position: usize::from(context_item.is_some()),
            size: usize::from(context_item.is_some()),
        }
    }

    fn item(context_item: &'a XPathResultItem, position: usize, size: usize) -> Self {
        Self {
            context_item: Some(context_item),
            position,
            size,
        }
    }
}

fn xpath_evaluation_diagnostic(
    expression: &XPathExpressionAst,
    code: impl Into<String>,
    message: impl Into<String>,
    source_range: Option<XPathSourceRange>,
) -> Diagnostic {
    let source_range = source_range.or_else(|| {
        expression
            .syntax_ast
            .as_ref()
            .map(|syntax| syntax.root.source_range)
    });
    Diagnostic {
        uri: Some(expression.source.uri.clone()),
        line: source_range.map(|range| range.start.line),
        column: source_range.map(|range| range.start.column),
        byte_offset: source_range.map(|range| range.start.byte_offset),
        code: code.into(),
        severity: Severity::Error,
        message: message.into(),
        source_map: source_range.map(|range| {
            range.source_map(
                expression.attachment.source_id(),
                expression.source.media_type.as_str(),
            )
        }),
        ..Diagnostic::default()
    }
}

fn xpath_evaluate_expression_sequence(
    expression: &XPathExpressionAst,
    sequence: &XPathExpressionSequence,
    focus: XPathFocus<'_>,
    variable_bindings: &XPathVariableBindings,
) -> Result<XPathResultSequence, XPathEvaluationError> {
    debug_assert!(focus.position <= focus.size);
    debug_assert_eq!(focus.context_item.is_some(), focus.size > 0);
    let mut items = Vec::new();
    for node in &sequence.expressions {
        items.extend(xpath_evaluate_expression_node(
            expression,
            node,
            focus,
            variable_bindings,
        )?);
    }
    Ok(XPathResultSequence {
        sequence_type: xpath_result_sequence_type(&items),
        items,
    })
}

fn xpath_evaluate_expression_node(
    expression: &XPathExpressionAst,
    node: &XPathExpressionNode,
    focus: XPathFocus<'_>,
    variable_bindings: &XPathVariableBindings,
) -> Result<Vec<XPathResultItem>, XPathEvaluationError> {
    match &node.expression {
        XPathExpression::Path(path) => {
            xpath_evaluate_path(expression, path, focus, variable_bindings)
        }
        XPathExpression::Binary {
            operator,
            left,
            right,
        } if matches!(operator, XPathBinaryOperator::And | XPathBinaryOperator::Or) => {
            let left_items =
                xpath_evaluate_expression_node(expression, left, focus, variable_bindings)?;
            let left_value = xpath_effective_boolean_value(&left_items, left.source_range)?;
            let value = match operator {
                XPathBinaryOperator::And if !left_value => false,
                XPathBinaryOperator::Or if left_value => true,
                XPathBinaryOperator::And | XPathBinaryOperator::Or => {
                    let right_items = xpath_evaluate_expression_node(
                        expression,
                        right,
                        focus,
                        variable_bindings,
                    )?;
                    xpath_effective_boolean_value(&right_items, right.source_range)?
                }
                _ => unreachable!("logical operator guard restricts native evaluation"),
            };
            Ok(vec![xpath_boolean_result_item(
                expression,
                node.source_range,
                value,
            )])
        }
        XPathExpression::Binary {
            operator,
            left,
            right,
        } if xpath_comparison_operator(*operator).is_some() => {
            let left = xpath_evaluate_expression_node(expression, left, focus, variable_bindings)?;
            let right =
                xpath_evaluate_expression_node(expression, right, focus, variable_bindings)?;
            let (mode, relation) = xpath_comparison_operator(*operator)
                .expect("comparison operator guard resolves comparison semantics");
            let value = match mode {
                XPathComparisonMode::General => Some(xpath_general_compare(
                    &left,
                    &right,
                    relation,
                    node.source_range,
                )?),
                XPathComparisonMode::Value => {
                    xpath_value_compare(&left, &right, relation, node.source_range)?
                }
            };
            Ok(value
                .map(|value| xpath_boolean_result_item(expression, node.source_range, value))
                .into_iter()
                .collect())
        }
        XPathExpression::Binary { operator, .. } => Err(XPathEvaluationError::unsupported(
            format!("XPath operator `{operator:?}` is outside the first native evaluator slice"),
            node.source_range,
        )),
        XPathExpression::For { .. } => Err(XPathEvaluationError::unsupported(
            "XPath for expressions are outside the first native evaluator slice",
            node.source_range,
        )),
        XPathExpression::Unsupported { production } => Err(XPathEvaluationError::unsupported(
            format!("XPath production `{production}` is not executable yet"),
            node.source_range,
        )),
    }
}

fn xpath_evaluate_path(
    expression: &XPathExpressionAst,
    path: &XPathPathExpression,
    focus: XPathFocus<'_>,
    variable_bindings: &XPathVariableBindings,
) -> Result<Vec<XPathResultItem>, XPathEvaluationError> {
    let mut current = match path.root {
        XPathPathRoot::Relative => Vec::new(),
        XPathPathRoot::Rooted | XPathPathRoot::RootedDescendant => {
            let context_node = xpath_context_native_node(focus.context_item, path.source_range)?;
            vec![XPathResultItem::from_native_node(
                context_node.document_root(),
            )]
        }
    };

    if path.root == XPathPathRoot::RootedDescendant {
        let root = current
            .first()
            .and_then(XPathResultItem::native_node)
            .expect("rooted XPath path initializes a native document node");
        let mut nodes = vec![root.clone()];
        nodes.extend(root.descendant_nodes());
        current = nodes
            .into_iter()
            .map(XPathResultItem::from_native_node)
            .collect();
    }

    for (index, step) in path.steps.iter().enumerate() {
        match &step.step {
            XPathStep::Axis {
                axis,
                node_test,
                predicates,
            } => {
                if current.is_empty() && path.root == XPathPathRoot::Relative && index == 0 {
                    current.push(focus.context_item.cloned().ok_or_else(|| {
                        XPathEvaluationError {
                            code: "cem.xpath.context_item_missing",
                            message: "relative XPath path requires a context item".to_owned(),
                            source_range: Some(step.source_range),
                        }
                    })?);
                }
                let mut combined = Vec::new();
                for item in &current {
                    let candidates =
                        xpath_evaluate_axis(*axis, node_test, item, step.source_range)?;
                    combined.extend(xpath_apply_predicates(
                        expression,
                        predicates,
                        candidates,
                        variable_bindings,
                    )?);
                }
                current = xpath_normalize_node_results(combined, step.source_range)?;
            }
            XPathStep::Primary(primary) => {
                if path.root == XPathPathRoot::Relative && index == 0 {
                    current = xpath_evaluate_primary(
                        expression,
                        primary,
                        focus,
                        variable_bindings,
                        step.source_range,
                    )?;
                } else {
                    let size = current.len();
                    let mut combined = Vec::new();
                    for (position, item) in current.iter().enumerate() {
                        combined.extend(xpath_evaluate_primary(
                            expression,
                            primary,
                            XPathFocus::item(item, position.saturating_add(1), size),
                            variable_bindings,
                            step.source_range,
                        )?);
                    }
                    current = xpath_normalize_path_results(combined, step.source_range)?;
                }
            }
            XPathStep::Postfix { primary, postfixes } => {
                if path.root == XPathPathRoot::Relative && index == 0 {
                    current = xpath_evaluate_postfix(
                        expression,
                        primary,
                        postfixes,
                        focus,
                        variable_bindings,
                        step.source_range,
                    )?;
                } else {
                    let size = current.len();
                    let mut combined = Vec::new();
                    for (position, item) in current.iter().enumerate() {
                        combined.extend(xpath_evaluate_postfix(
                            expression,
                            primary,
                            postfixes,
                            XPathFocus::item(item, position.saturating_add(1), size),
                            variable_bindings,
                            step.source_range,
                        )?);
                    }
                    current = xpath_normalize_path_results(combined, step.source_range)?;
                }
            }
        }
    }
    Ok(current)
}

fn xpath_evaluate_postfix(
    expression: &XPathExpressionAst,
    primary: &XPathPrimaryExpression,
    postfixes: &[XPathPostfixExpression],
    focus: XPathFocus<'_>,
    variable_bindings: &XPathVariableBindings,
    source_range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>, XPathEvaluationError> {
    let mut current =
        xpath_evaluate_primary(expression, primary, focus, variable_bindings, source_range)?;
    for postfix in postfixes {
        match postfix {
            XPathPostfixExpression::Predicate(predicate) => {
                current = xpath_apply_predicates(
                    expression,
                    std::slice::from_ref(predicate),
                    current,
                    variable_bindings,
                )?;
            }
            XPathPostfixExpression::ArgumentList(_) => {
                return Err(XPathEvaluationError::unsupported(
                    "XPath dynamic function calls are outside the native evaluator slice",
                    source_range,
                ));
            }
            XPathPostfixExpression::Lookup { .. } => {
                return Err(XPathEvaluationError::unsupported(
                    "XPath postfix lookups are outside the native evaluator slice",
                    source_range,
                ));
            }
        }
    }
    Ok(current)
}

fn xpath_evaluate_primary(
    expression: &XPathExpressionAst,
    primary: &XPathPrimaryExpression,
    focus: XPathFocus<'_>,
    variable_bindings: &XPathVariableBindings,
    source_range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>, XPathEvaluationError> {
    match primary {
        XPathPrimaryExpression::Literal(literal) => Ok(vec![XPathResultItem::Atomic {
            value: XPathAtomicValue {
                type_name: match literal.kind {
                    XPathLiteralKind::Integer => "xs:integer",
                    XPathLiteralKind::Decimal => "xs:decimal",
                    XPathLiteralKind::Double => "xs:double",
                    XPathLiteralKind::String => "xs:string",
                }
                .to_owned(),
                lexical_value: match literal.kind {
                    XPathLiteralKind::String => literal.value.clone(),
                    _ => literal.lexical.clone(),
                },
                namespace_uri: None,
                local_name: None,
            },
            source_map: source_range.source_map(
                expression.attachment.source_id(),
                expression.source.media_type.as_str(),
            ),
        }]),
        XPathPrimaryExpression::VariableReference(name) => variable_bindings
            .get(&XPathExpandedName::from_syntax_name(name))
            .map(|sequence| sequence.items.clone())
            .ok_or_else(|| XPathEvaluationError {
                code: "cem.xpath.variable_unbound",
                message: format!(
                    "XPath variable `${}` ({}) is not bound",
                    name.lexical,
                    XPathExpandedName::from_syntax_name(name).display()
                ),
                source_range: Some(name.source_range),
            }),
        XPathPrimaryExpression::Parenthesized(None) => Ok(Vec::new()),
        XPathPrimaryExpression::Parenthesized(Some(sequence)) => {
            xpath_evaluate_expression_sequence(expression, sequence, focus, variable_bindings)
                .map(|sequence| sequence.items)
        }
        XPathPrimaryExpression::ContextItem => focus
            .context_item
            .cloned()
            .map(|item| vec![item])
            .ok_or_else(|| XPathEvaluationError {
                code: "cem.xpath.context_item_missing",
                message: "XPath context item is not available".to_owned(),
                source_range: Some(source_range),
            }),
        XPathPrimaryExpression::FunctionCall { name, arguments }
            if name.namespace_uri.as_deref() == Some("http://www.w3.org/2005/xpath-functions")
                && arguments.is_empty()
                && matches!(name.local_name.as_str(), "position" | "last") =>
        {
            if focus.context_item.is_none() {
                return Err(XPathEvaluationError::dynamic(
                    "cem.xpath.context_item_missing",
                    format!(
                        "XPath focus function `{}` requires an available focus",
                        name.lexical
                    ),
                    source_range,
                ));
            }
            let value = match name.local_name.as_str() {
                "position" => focus.position,
                "last" => focus.size,
                _ => unreachable!("guard restricts native focus functions"),
            };
            Ok(vec![XPathResultItem::Atomic {
                value: XPathAtomicValue {
                    type_name: "xs:integer".to_owned(),
                    lexical_value: value.to_string(),
                    namespace_uri: None,
                    local_name: None,
                },
                source_map: source_range.source_map(
                    expression.attachment.source_id(),
                    expression.source.media_type.as_str(),
                ),
            }])
        }
        XPathPrimaryExpression::FunctionCall { name, .. } => {
            Err(XPathEvaluationError::unsupported(
                format!(
                    "XPath function call `{}` is outside the native evaluator slice",
                    name.lexical
                ),
                source_range,
            ))
        }
        XPathPrimaryExpression::MapConstructor { .. } => Err(XPathEvaluationError::unsupported(
            "XPath map constructors are outside the first native evaluator slice",
            source_range,
        )),
        XPathPrimaryExpression::ArrayConstructor(_) => Err(XPathEvaluationError::unsupported(
            "XPath array constructors are outside the first native evaluator slice",
            source_range,
        )),
        XPathPrimaryExpression::Unsupported { production } => {
            Err(XPathEvaluationError::unsupported(
                format!("XPath production `{production}` is not executable yet"),
                source_range,
            ))
        }
    }
}

fn xpath_context_native_node(
    context_item: Option<&XPathResultItem>,
    source_range: XPathSourceRange,
) -> Result<&XPathNativeNode, XPathEvaluationError> {
    context_item
        .and_then(XPathResultItem::native_node)
        .ok_or_else(|| XPathEvaluationError {
            code: "cem.xpath.context_item_native_node_required",
            message: "XPath path evaluation requires a native AST node context".to_owned(),
            source_range: Some(source_range),
        })
}

fn xpath_evaluate_axis(
    axis: XPathAxis,
    node_test: &XPathNodeTest,
    input: &XPathResultItem,
    source_range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>, XPathEvaluationError> {
    let native_node = input.native_node().ok_or_else(|| XPathEvaluationError {
        code: "cem.xpath.context_item_native_node_required",
        message: "XPath axis evaluation requires native AST node items".to_owned(),
        source_range: Some(source_range),
    })?;
    let candidates = match axis {
        XPathAxis::Ancestor => native_node.ancestor_nodes(),
        XPathAxis::AncestorOrSelf => {
            let mut nodes = vec![native_node.clone()];
            nodes.extend(native_node.ancestor_nodes());
            nodes
        }
        XPathAxis::Attribute => native_node.attribute_nodes(),
        XPathAxis::Child => native_node.child_nodes(),
        XPathAxis::Descendant => native_node.descendant_nodes(),
        XPathAxis::DescendantOrSelf => {
            let mut nodes = vec![native_node.clone()];
            nodes.extend(native_node.descendant_nodes());
            nodes
        }
        XPathAxis::Following => native_node.following_nodes(),
        XPathAxis::FollowingSibling => native_node.following_sibling_nodes(),
        XPathAxis::Namespace => {
            return Err(XPathEvaluationError::unsupported(
                "XPath axis `Namespace` is optional, deprecated, and not provided by this host language",
                source_range,
            ));
        }
        XPathAxis::Parent => native_node.parent_node().into_iter().collect(),
        XPathAxis::Preceding => native_node.preceding_nodes(),
        XPathAxis::PrecedingSibling => native_node.preceding_sibling_nodes(),
        XPathAxis::SelfAxis => vec![native_node.clone()],
    };
    Ok(candidates
        .into_iter()
        .filter(|node| xpath_axis_matches_node_test(axis, node, node_test))
        .map(XPathResultItem::from_native_node)
        .collect())
}

fn xpath_axis_matches_node_test(
    axis: XPathAxis,
    node: &XPathNativeNode,
    node_test: &XPathNodeTest,
) -> bool {
    if matches!(node_test, XPathNodeTest::Name(_)) {
        let principal_node_kind = match axis {
            XPathAxis::Attribute => XPathResultNodeKind::Attribute,
            XPathAxis::Namespace => XPathResultNodeKind::Namespace,
            _ => XPathResultNodeKind::Element,
        };
        if node.result_node_kind() != principal_node_kind {
            return false;
        }
    }
    node.matches_node_test(node_test)
}

fn xpath_apply_predicates(
    expression: &XPathExpressionAst,
    predicates: &[XPathExpressionSequence],
    mut input: Vec<XPathResultItem>,
    variable_bindings: &XPathVariableBindings,
) -> Result<Vec<XPathResultItem>, XPathEvaluationError> {
    for predicate in predicates {
        let size = input.len();
        let mut filtered = Vec::new();
        for (index, item) in input.into_iter().enumerate() {
            let predicate_focus = XPathFocus::item(&item, index.saturating_add(1), size);
            let result = xpath_evaluate_expression_sequence(
                expression,
                predicate,
                predicate_focus,
                variable_bindings,
            )?;
            if xpath_predicate_truth_value(
                &result.items,
                predicate_focus.position,
                predicate.source_range,
            )? {
                filtered.push(item);
            }
        }
        input = filtered;
    }
    Ok(input)
}

fn xpath_predicate_truth_value(
    items: &[XPathResultItem],
    position: usize,
    source_range: XPathSourceRange,
) -> Result<bool, XPathEvaluationError> {
    if let [XPathResultItem::Atomic { value, .. }] = items {
        if let Some(result) = xpath_numeric_equals_position(value, position, source_range)? {
            return Ok(result);
        }
    }
    xpath_effective_boolean_value(items, source_range)
}

fn xpath_numeric_equals_position(
    value: &XPathAtomicValue,
    position: usize,
    source_range: XPathSourceRange,
) -> Result<Option<bool>, XPathEvaluationError> {
    let result = match value.type_name.as_str() {
        "xs:integer" => XPathExactDecimal::parse(&value.lexical_value, false).map(|number| {
            number.compare(&XPathExactDecimal::from_usize(position)) == Ordering::Equal
        }),
        "xs:decimal" => XPathExactDecimal::parse(&value.lexical_value, true).map(|number| {
            number.compare(&XPathExactDecimal::from_usize(position)) == Ordering::Equal
        }),
        "xs:float" => {
            xpath_parse_float(&value.lexical_value).map(|number| number == position as f32)
        }
        "xs:double" => {
            xpath_parse_double(&value.lexical_value).map(|number| number == position as f64)
        }
        _ => return Ok(None),
    }
    .ok_or_else(|| {
        XPathEvaluationError::dynamic(
            "cem.xpath.numeric_value_invalid",
            format!(
                "XPath numeric value `{}` is not a valid {}",
                value.lexical_value, value.type_name
            ),
            source_range,
        )
    })?;
    Ok(Some(result))
}

fn xpath_parse_float(lexical: &str) -> Option<f32> {
    match lexical.trim() {
        "INF" => Some(f32::INFINITY),
        "-INF" => Some(f32::NEG_INFINITY),
        "NaN" => Some(f32::NAN),
        lexical => lexical.parse::<f32>().ok(),
    }
}

fn xpath_parse_double(lexical: &str) -> Option<f64> {
    match lexical.trim() {
        "INF" => Some(f64::INFINITY),
        "-INF" => Some(f64::NEG_INFINITY),
        "NaN" => Some(f64::NAN),
        lexical => lexical.parse::<f64>().ok(),
    }
}

fn xpath_effective_boolean_value(
    items: &[XPathResultItem],
    source_range: XPathSourceRange,
) -> Result<bool, XPathEvaluationError> {
    let Some(first) = items.first() else {
        return Ok(false);
    };
    if matches!(first, XPathResultItem::Node { .. }) {
        return Ok(true);
    }
    let [XPathResultItem::Atomic { value, .. }] = items else {
        return Err(XPathEvaluationError::dynamic(
            "cem.xpath.effective_boolean_value_type_error",
            "XPath effective boolean value requires a node sequence or one supported atomic value",
            source_range,
        ));
    };
    match value.type_name.as_str() {
        "xs:boolean" => match value.lexical_value.as_str() {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            _ => Err(XPathEvaluationError::dynamic(
                "cem.xpath.boolean_value_invalid",
                format!(
                    "XPath boolean value `{}` is not a valid xs:boolean",
                    value.lexical_value
                ),
                source_range,
            )),
        },
        "xs:string" | "xs:anyURI" | "xs:untypedAtomic" => Ok(!value.lexical_value.is_empty()),
        "xs:integer" => XPathExactDecimal::parse(&value.lexical_value, false)
            .map(|number| !number.is_zero())
            .ok_or_else(|| {
                XPathEvaluationError::dynamic(
                    "cem.xpath.numeric_value_invalid",
                    format!(
                        "XPath numeric value `{}` is not a valid xs:integer",
                        value.lexical_value
                    ),
                    source_range,
                )
            }),
        "xs:decimal" => XPathExactDecimal::parse(&value.lexical_value, true)
            .map(|number| !number.is_zero())
            .ok_or_else(|| {
                XPathEvaluationError::dynamic(
                    "cem.xpath.numeric_value_invalid",
                    format!(
                        "XPath numeric value `{}` is not a valid xs:decimal",
                        value.lexical_value
                    ),
                    source_range,
                )
            }),
        "xs:float" => xpath_parse_float(&value.lexical_value)
            .map(|number| number != 0.0 && !number.is_nan())
            .ok_or_else(|| {
                XPathEvaluationError::dynamic(
                    "cem.xpath.numeric_value_invalid",
                    format!(
                        "XPath numeric value `{}` is not a valid {}",
                        value.lexical_value, value.type_name
                    ),
                    source_range,
                )
            }),
        "xs:double" => xpath_parse_double(&value.lexical_value)
            .map(|number| number != 0.0 && !number.is_nan())
            .ok_or_else(|| {
                XPathEvaluationError::dynamic(
                    "cem.xpath.numeric_value_invalid",
                    format!(
                        "XPath numeric value `{}` is not a valid {}",
                        value.lexical_value, value.type_name
                    ),
                    source_range,
                )
            }),
        _ => Err(XPathEvaluationError::dynamic(
            "cem.xpath.effective_boolean_value_type_error",
            format!(
                "XPath effective boolean value does not support `{}` yet",
                value.type_name
            ),
            source_range,
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum XPathComparisonMode {
    Value,
    General,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum XPathComparisonRelation {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

fn xpath_comparison_operator(
    operator: XPathBinaryOperator,
) -> Option<(XPathComparisonMode, XPathComparisonRelation)> {
    let comparison = match operator {
        XPathBinaryOperator::ValueEqual => {
            (XPathComparisonMode::Value, XPathComparisonRelation::Equal)
        }
        XPathBinaryOperator::ValueNotEqual => (
            XPathComparisonMode::Value,
            XPathComparisonRelation::NotEqual,
        ),
        XPathBinaryOperator::ValueLessThan => (
            XPathComparisonMode::Value,
            XPathComparisonRelation::LessThan,
        ),
        XPathBinaryOperator::ValueLessThanOrEqual => (
            XPathComparisonMode::Value,
            XPathComparisonRelation::LessThanOrEqual,
        ),
        XPathBinaryOperator::ValueGreaterThan => (
            XPathComparisonMode::Value,
            XPathComparisonRelation::GreaterThan,
        ),
        XPathBinaryOperator::ValueGreaterThanOrEqual => (
            XPathComparisonMode::Value,
            XPathComparisonRelation::GreaterThanOrEqual,
        ),
        XPathBinaryOperator::GeneralEqual => {
            (XPathComparisonMode::General, XPathComparisonRelation::Equal)
        }
        XPathBinaryOperator::GeneralNotEqual => (
            XPathComparisonMode::General,
            XPathComparisonRelation::NotEqual,
        ),
        XPathBinaryOperator::GeneralLessThan => (
            XPathComparisonMode::General,
            XPathComparisonRelation::LessThan,
        ),
        XPathBinaryOperator::GeneralLessThanOrEqual => (
            XPathComparisonMode::General,
            XPathComparisonRelation::LessThanOrEqual,
        ),
        XPathBinaryOperator::GeneralGreaterThan => (
            XPathComparisonMode::General,
            XPathComparisonRelation::GreaterThan,
        ),
        XPathBinaryOperator::GeneralGreaterThanOrEqual => (
            XPathComparisonMode::General,
            XPathComparisonRelation::GreaterThanOrEqual,
        ),
        _ => return None,
    };
    Some(comparison)
}

fn xpath_boolean_result_item(
    expression: &XPathExpressionAst,
    source_range: XPathSourceRange,
    value: bool,
) -> XPathResultItem {
    XPathResultItem::Atomic {
        value: XPathAtomicValue {
            type_name: "xs:boolean".to_owned(),
            lexical_value: value.to_string(),
            namespace_uri: None,
            local_name: None,
        },
        source_map: source_range.source_map(
            expression.attachment.source_id(),
            expression.source.media_type.as_str(),
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct XPathExactDecimal {
    negative: bool,
    coefficient: Vec<u8>,
    scale: usize,
}

impl XPathExactDecimal {
    fn parse(lexical: &str, allow_fraction: bool) -> Option<Self> {
        let lexical = lexical.trim();
        let (negative, unsigned) = match lexical.as_bytes().first() {
            Some(b'-') => (true, &lexical[1..]),
            Some(b'+') => (false, &lexical[1..]),
            _ => (false, lexical),
        };
        if unsigned.is_empty() {
            return None;
        }
        let mut parts = unsigned.split('.');
        let integer = parts.next()?;
        let fraction = parts.next();
        if parts.next().is_some()
            || fraction.is_some_and(|_| !allow_fraction)
            || (integer.is_empty() && fraction.is_none_or(str::is_empty))
            || !integer.bytes().all(|byte| byte.is_ascii_digit())
            || fraction.is_some_and(|digits| !digits.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return None;
        }

        let fraction = fraction.unwrap_or("").trim_end_matches('0');
        let scale = fraction.len();
        let mut coefficient = integer
            .bytes()
            .chain(fraction.bytes())
            .map(|byte| byte.saturating_sub(b'0'))
            .skip_while(|digit| *digit == 0)
            .collect::<Vec<_>>();
        if coefficient.is_empty() {
            coefficient.push(0);
            return Some(Self {
                negative: false,
                coefficient,
                scale: 0,
            });
        }
        Some(Self {
            negative,
            coefficient,
            scale,
        })
    }

    fn from_usize(value: usize) -> Self {
        Self::parse(&value.to_string(), false).expect("usize has a valid integer representation")
    }

    fn is_zero(&self) -> bool {
        self.coefficient == [0]
    }

    fn compare(&self, other: &Self) -> Ordering {
        if self.negative != other.negative {
            return if self.negative {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        let magnitude = self.compare_magnitude(other);
        if self.negative {
            magnitude.reverse()
        } else {
            magnitude
        }
    }

    fn compare_magnitude(&self, other: &Self) -> Ordering {
        let common_scale = self.scale.max(other.scale);
        let left_length = self
            .coefficient
            .len()
            .saturating_add(common_scale.saturating_sub(self.scale));
        let right_length = other
            .coefficient
            .len()
            .saturating_add(common_scale.saturating_sub(other.scale));
        match left_length.cmp(&right_length) {
            Ordering::Equal => {}
            ordering => return ordering,
        }
        for index in 0..left_length {
            let left = self.coefficient.get(index).copied().unwrap_or(0);
            let right = other.coefficient.get(index).copied().unwrap_or(0);
            match left.cmp(&right) {
                Ordering::Equal => {}
                ordering => return ordering,
            }
        }
        Ordering::Equal
    }

    fn to_lexical(&self) -> String {
        let sign_length = usize::from(self.negative);
        let decimal_overhead = usize::from(self.scale > 0).saturating_add(
            self.scale
                .saturating_sub(self.coefficient.len())
                .saturating_add(1),
        );
        let mut lexical = String::with_capacity(
            sign_length
                .saturating_add(self.coefficient.len())
                .saturating_add(decimal_overhead),
        );
        if self.negative {
            lexical.push('-');
        }
        if self.scale == 0 {
            for digit in &self.coefficient {
                lexical.push(char::from(b'0'.saturating_add(*digit)));
            }
            return lexical;
        }
        if self.coefficient.len() > self.scale {
            let integer_length = self.coefficient.len().saturating_sub(self.scale);
            for (index, digit) in self.coefficient.iter().enumerate() {
                if index == integer_length {
                    lexical.push('.');
                }
                lexical.push(char::from(b'0'.saturating_add(*digit)));
            }
        } else {
            lexical.push_str("0.");
            for _ in 0..self.scale.saturating_sub(self.coefficient.len()) {
                lexical.push('0');
            }
            for digit in &self.coefficient {
                lexical.push(char::from(b'0'.saturating_add(*digit)));
            }
        }
        lexical
    }
}

#[derive(Debug, Clone)]
enum XPathComparableAtomic {
    Untyped(String),
    String(String),
    Boolean(bool),
    Decimal(XPathExactDecimal),
    Float(f32),
    Double(f64),
}

impl XPathComparableAtomic {
    fn is_numeric(&self) -> bool {
        matches!(self, Self::Decimal(_) | Self::Float(_) | Self::Double(_))
    }

    fn type_name(&self) -> &'static str {
        match self {
            Self::Untyped(_) => "xs:untypedAtomic",
            Self::String(_) => "xs:string",
            Self::Boolean(_) => "xs:boolean",
            Self::Decimal(_) => "xs:decimal",
            Self::Float(_) => "xs:float",
            Self::Double(_) => "xs:double",
        }
    }
}

fn xpath_value_compare(
    left: &[XPathResultItem],
    right: &[XPathResultItem],
    relation: XPathComparisonRelation,
    source_range: XPathSourceRange,
) -> Result<Option<bool>, XPathEvaluationError> {
    let mut left = xpath_atomize_sequence(left, source_range)?;
    let mut right = xpath_atomize_sequence(right, source_range)?;
    if left.is_empty() || right.is_empty() {
        return Ok(None);
    }
    if left.len() != 1 || right.len() != 1 {
        return Err(XPathEvaluationError::dynamic(
            "cem.xpath.value_comparison_cardinality",
            "XPath value comparison operands must atomize to zero or one value",
            source_range,
        ));
    }
    let left = xpath_untyped_to_string(left.pop().expect("singleton value operand"));
    let right = xpath_untyped_to_string(right.pop().expect("singleton value operand"));
    xpath_compare_atomic(&left, &right, relation, source_range).map(Some)
}

fn xpath_general_compare(
    left: &[XPathResultItem],
    right: &[XPathResultItem],
    relation: XPathComparisonRelation,
    source_range: XPathSourceRange,
) -> Result<bool, XPathEvaluationError> {
    let left = xpath_atomize_sequence(left, source_range)?;
    let right = xpath_atomize_sequence(right, source_range)?;
    for left_value in &left {
        for right_value in &right {
            let (left_value, right_value) =
                xpath_prepare_general_pair(left_value.clone(), right_value.clone(), source_range)?;
            if xpath_compare_atomic(&left_value, &right_value, relation, source_range)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn xpath_atomize_sequence(
    items: &[XPathResultItem],
    source_range: XPathSourceRange,
) -> Result<Vec<XPathComparableAtomic>, XPathEvaluationError> {
    items
        .iter()
        .map(|item| xpath_atomize_item(item, source_range))
        .collect()
}

fn xpath_atomize_item(
    item: &XPathResultItem,
    source_range: XPathSourceRange,
) -> Result<XPathComparableAtomic, XPathEvaluationError> {
    match item {
        XPathResultItem::Node {
            native_node: Some(node),
            ..
        } => Ok(XPathComparableAtomic::Untyped(node.string_value())),
        XPathResultItem::Node { .. } => Err(XPathEvaluationError::dynamic(
            "cem.xpath.native_node_missing",
            "XPath node atomization requires its retained native node handle",
            source_range,
        )),
        XPathResultItem::Atomic { value, .. } => xpath_comparable_atomic(value, source_range),
        item => Err(XPathEvaluationError::unsupported(
            format!(
                "XPath atomization for result item kind `{:?}` is outside the native atomic slice",
                item.kind()
            ),
            source_range,
        )),
    }
}

fn xpath_comparable_atomic(
    value: &XPathAtomicValue,
    source_range: XPathSourceRange,
) -> Result<XPathComparableAtomic, XPathEvaluationError> {
    let invalid = || {
        XPathEvaluationError::dynamic(
            "cem.xpath.atomic_value_invalid",
            format!(
                "XPath atomic value `{}` is not valid for `{}`",
                value.lexical_value, value.type_name
            ),
            source_range,
        )
    };
    match value.type_name.as_str() {
        "xs:untypedAtomic" => Ok(XPathComparableAtomic::Untyped(value.lexical_value.clone())),
        "xs:string" | "xs:anyURI" => Ok(XPathComparableAtomic::String(value.lexical_value.clone())),
        "xs:boolean" => xpath_parse_boolean(&value.lexical_value)
            .map(XPathComparableAtomic::Boolean)
            .ok_or_else(invalid),
        "xs:integer" => XPathExactDecimal::parse(&value.lexical_value, false)
            .map(XPathComparableAtomic::Decimal)
            .ok_or_else(invalid),
        "xs:decimal" => XPathExactDecimal::parse(&value.lexical_value, true)
            .map(XPathComparableAtomic::Decimal)
            .ok_or_else(invalid),
        "xs:float" => xpath_parse_float(&value.lexical_value)
            .map(XPathComparableAtomic::Float)
            .ok_or_else(invalid),
        "xs:double" => xpath_parse_double(&value.lexical_value)
            .map(XPathComparableAtomic::Double)
            .ok_or_else(invalid),
        _ => Err(XPathEvaluationError::unsupported(
            format!(
                "XPath comparison for atomic type `{}` is outside the native atomic slice",
                value.type_name
            ),
            source_range,
        )),
    }
}

fn xpath_prepare_general_pair(
    mut left: XPathComparableAtomic,
    mut right: XPathComparableAtomic,
    source_range: XPathSourceRange,
) -> Result<(XPathComparableAtomic, XPathComparableAtomic), XPathEvaluationError> {
    if let (
        XPathComparableAtomic::Untyped(left_value),
        XPathComparableAtomic::Untyped(right_value),
    ) = (&left, &right)
    {
        return Ok((
            XPathComparableAtomic::String(left_value.clone()),
            XPathComparableAtomic::String(right_value.clone()),
        ));
    }
    if let XPathComparableAtomic::Untyped(value) = &left {
        left = xpath_cast_untyped_for_general(value, &right, source_range)?;
    }
    if let XPathComparableAtomic::Untyped(value) = &right {
        right = xpath_cast_untyped_for_general(value, &left, source_range)?;
    }
    Ok((left, right))
}

fn xpath_cast_untyped_for_general(
    value: &str,
    other: &XPathComparableAtomic,
    source_range: XPathSourceRange,
) -> Result<XPathComparableAtomic, XPathEvaluationError> {
    let cast_invalid = || {
        XPathEvaluationError::dynamic(
            "cem.xpath.atomic_cast_invalid",
            format!(
                "XPath untyped atomic value `{value}` cannot be cast for comparison with `{}`",
                other.type_name()
            ),
            source_range,
        )
    };
    match other {
        XPathComparableAtomic::String(_) => Ok(XPathComparableAtomic::String(value.to_owned())),
        XPathComparableAtomic::Boolean(_) => xpath_parse_boolean(value)
            .map(XPathComparableAtomic::Boolean)
            .ok_or_else(cast_invalid),
        other if other.is_numeric() => xpath_parse_double(value)
            .map(XPathComparableAtomic::Double)
            .ok_or_else(cast_invalid),
        XPathComparableAtomic::Untyped(_) => {
            unreachable!("two untyped general-comparison operands are converted together")
        }
        _ => Err(cast_invalid()),
    }
}

fn xpath_untyped_to_string(value: XPathComparableAtomic) -> XPathComparableAtomic {
    match value {
        XPathComparableAtomic::Untyped(value) => XPathComparableAtomic::String(value),
        value => value,
    }
}

fn xpath_compare_atomic(
    left: &XPathComparableAtomic,
    right: &XPathComparableAtomic,
    relation: XPathComparisonRelation,
    source_range: XPathSourceRange,
) -> Result<bool, XPathEvaluationError> {
    match (left, right) {
        (XPathComparableAtomic::String(left), XPathComparableAtomic::String(right)) => {
            Ok(xpath_ordering_matches(left.cmp(right), relation))
        }
        (XPathComparableAtomic::Boolean(left), XPathComparableAtomic::Boolean(right)) => {
            Ok(xpath_ordering_matches(left.cmp(right), relation))
        }
        (left, right) if left.is_numeric() && right.is_numeric() => {
            xpath_compare_numeric(left, right, relation, source_range)
        }
        _ => Err(XPathEvaluationError::dynamic(
            "cem.xpath.comparison_type_error",
            format!(
                "XPath comparison does not define a relationship between `{}` and `{}`",
                left.type_name(),
                right.type_name()
            ),
            source_range,
        )),
    }
}

fn xpath_compare_numeric(
    left: &XPathComparableAtomic,
    right: &XPathComparableAtomic,
    relation: XPathComparisonRelation,
    source_range: XPathSourceRange,
) -> Result<bool, XPathEvaluationError> {
    if matches!(left, XPathComparableAtomic::Double(_))
        || matches!(right, XPathComparableAtomic::Double(_))
    {
        let left = xpath_numeric_as_double(left, source_range)?;
        let right = xpath_numeric_as_double(right, source_range)?;
        return Ok(xpath_double_matches(left, right, relation));
    }
    if matches!(left, XPathComparableAtomic::Float(_))
        || matches!(right, XPathComparableAtomic::Float(_))
    {
        let left = xpath_numeric_as_float(left, source_range)?;
        let right = xpath_numeric_as_float(right, source_range)?;
        return Ok(xpath_float_matches(left, right, relation));
    }
    let (XPathComparableAtomic::Decimal(left), XPathComparableAtomic::Decimal(right)) =
        (left, right)
    else {
        unreachable!("numeric comparison ranks exhaust supported numeric values")
    };
    Ok(xpath_ordering_matches(left.compare(right), relation))
}

fn xpath_numeric_as_double(
    value: &XPathComparableAtomic,
    source_range: XPathSourceRange,
) -> Result<f64, XPathEvaluationError> {
    match value {
        XPathComparableAtomic::Double(value) => Ok(*value),
        XPathComparableAtomic::Float(value) => Ok(f64::from(*value)),
        XPathComparableAtomic::Decimal(value) => xpath_parse_double(&value.to_lexical())
            .ok_or_else(|| xpath_numeric_promotion_error(value, "xs:double", source_range)),
        _ => unreachable!("numeric conversion requires a numeric atomic value"),
    }
}

fn xpath_numeric_as_float(
    value: &XPathComparableAtomic,
    source_range: XPathSourceRange,
) -> Result<f32, XPathEvaluationError> {
    match value {
        XPathComparableAtomic::Float(value) => Ok(*value),
        XPathComparableAtomic::Decimal(value) => xpath_parse_float(&value.to_lexical())
            .ok_or_else(|| xpath_numeric_promotion_error(value, "xs:float", source_range)),
        _ => unreachable!("float promotion accepts decimal or float values"),
    }
}

fn xpath_numeric_promotion_error(
    value: &XPathExactDecimal,
    target_type: &str,
    source_range: XPathSourceRange,
) -> XPathEvaluationError {
    XPathEvaluationError::dynamic(
        "cem.xpath.numeric_promotion_invalid",
        format!(
            "XPath decimal value `{}` cannot be promoted to `{target_type}`",
            value.to_lexical()
        ),
        source_range,
    )
}

fn xpath_ordering_matches(ordering: Ordering, relation: XPathComparisonRelation) -> bool {
    match relation {
        XPathComparisonRelation::Equal => ordering == Ordering::Equal,
        XPathComparisonRelation::NotEqual => ordering != Ordering::Equal,
        XPathComparisonRelation::LessThan => ordering == Ordering::Less,
        XPathComparisonRelation::LessThanOrEqual => ordering != Ordering::Greater,
        XPathComparisonRelation::GreaterThan => ordering == Ordering::Greater,
        XPathComparisonRelation::GreaterThanOrEqual => ordering != Ordering::Less,
    }
}

fn xpath_float_matches(left: f32, right: f32, relation: XPathComparisonRelation) -> bool {
    match relation {
        XPathComparisonRelation::Equal => left == right,
        XPathComparisonRelation::NotEqual => left != right,
        XPathComparisonRelation::LessThan => left < right,
        XPathComparisonRelation::LessThanOrEqual => left <= right,
        XPathComparisonRelation::GreaterThan => left > right,
        XPathComparisonRelation::GreaterThanOrEqual => left >= right,
    }
}

fn xpath_double_matches(left: f64, right: f64, relation: XPathComparisonRelation) -> bool {
    match relation {
        XPathComparisonRelation::Equal => left == right,
        XPathComparisonRelation::NotEqual => left != right,
        XPathComparisonRelation::LessThan => left < right,
        XPathComparisonRelation::LessThanOrEqual => left <= right,
        XPathComparisonRelation::GreaterThan => left > right,
        XPathComparisonRelation::GreaterThanOrEqual => left >= right,
    }
}

fn xpath_parse_boolean(lexical: &str) -> Option<bool> {
    match lexical.trim() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

fn xpath_normalize_path_results(
    items: Vec<XPathResultItem>,
    source_range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>, XPathEvaluationError> {
    let node_count = items
        .iter()
        .filter(|item| matches!(item, XPathResultItem::Node { .. }))
        .count();
    if node_count == items.len() {
        xpath_normalize_node_results(items, source_range)
    } else if node_count == 0 {
        Ok(items)
    } else {
        Err(XPathEvaluationError::dynamic(
            "cem.xpath.path_mixed_result",
            "XPath path step returned a mixture of nodes and non-nodes",
            source_range,
        ))
    }
}

fn xpath_normalize_node_results(
    mut items: Vec<XPathResultItem>,
    source_range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>, XPathEvaluationError> {
    let mut owners = Vec::<Arc<LoadedInputAstStream>>::new();
    for item in &items {
        let node = item.native_node().ok_or_else(|| {
            XPathEvaluationError::dynamic(
                "cem.xpath.native_node_missing",
                "XPath node ordering requires retained native node handles",
                source_range,
            )
        })?;
        if !owners.iter().any(|owner| Arc::ptr_eq(owner, node.owner())) {
            owners.push(Arc::clone(node.owner()));
        }
    }
    items.sort_by_key(|item| {
        let node = item
            .native_node()
            .expect("XPath node results validated before ordering");
        let owner_index = owners
            .iter()
            .position(|owner| Arc::ptr_eq(owner, node.owner()))
            .expect("XPath node owner registered before ordering");
        (owner_index, node.document_order_key())
    });
    items.dedup_by(|right, left| {
        right
            .native_node()
            .zip(left.native_node())
            .is_some_and(|(right, left)| right == left)
    });
    Ok(items)
}

fn xpath_result_sequence_type(items: &[XPathResultItem]) -> String {
    match items {
        [] => "empty-sequence()".to_owned(),
        [XPathResultItem::Atomic { value, .. }] => value.type_name.clone(),
        [XPathResultItem::Node { .. }] => "node()".to_owned(),
        [XPathResultItem::Map { .. }] => "map(*)".to_owned(),
        [XPathResultItem::Array { .. }] => "array(*)".to_owned(),
        [XPathResultItem::Function { .. }] => "function(*)".to_owned(),
        items
            if items
                .iter()
                .all(|item| matches!(item, XPathResultItem::Node { .. })) =>
        {
            "node()*".to_owned()
        }
        _ => "item()*".to_owned(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathEvaluationContractViolation {
    pub code: &'static str,
    pub message: String,
}

pub fn validate_xpath_evaluator_capabilities(
    capabilities: &XPathEvaluatorCapabilities,
) -> Vec<XPathEvaluationContractViolation> {
    let mut violations = Vec::new();
    if capabilities.xpath_version != "3.1" {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-evaluator-version-unsupported",
            "XPath evaluator must implement XPath 3.1",
        );
    }
    if capabilities.grammar_version != XPATH_GRAMMAR_VERSION {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-evaluator-grammar-mismatch",
            "XPath evaluator grammar must match the package-owned syntax AST",
        );
    }
    if capabilities.ast_input != XPathEvaluatorAstInput::PackageAst {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-evaluator-ast-reparse-forbidden",
            "XPath evaluator must consume XPathExpressionAst without reparsing source text",
        );
    }
    if capabilities.resource_access != XPathEvaluatorResourceAccess::CemResolver {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-evaluator-resolver-bypass",
            "XPath evaluator resource reads must use the CEM resolver boundary",
        );
    }
    if capabilities.source_map_mode != XPathEvaluatorSourceMapMode::ItemOrigins {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-evaluator-source-map-missing",
            "XPath evaluator must retain item-level source-map origins",
        );
    }
    if !capabilities.deterministic {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-evaluator-nondeterministic",
            "XPath evaluator must materialize deterministic result artifacts",
        );
    }
    for target in ["native", "wasm32-unknown-unknown"] {
        if !capabilities.targets.contains(target) {
            push_xpath_contract_violation(
                &mut violations,
                "xpath-evaluator-target-missing",
                format!("XPath evaluator does not support required target `{target}`"),
            );
        }
    }
    for kind in [
        XPathResultItemKind::Node,
        XPathResultItemKind::Atomic,
        XPathResultItemKind::Map,
        XPathResultItemKind::Array,
        XPathResultItemKind::Function,
    ] {
        if !capabilities.result_item_kinds.contains(&kind) {
            push_xpath_contract_violation(
                &mut violations,
                "xpath-evaluator-result-kind-missing",
                format!("XPath evaluator does not support `{kind:?}` result items"),
            );
        }
    }
    violations
}

pub fn validate_xpath_result_artifact(
    artifact: &XPathResultArtifact,
    capabilities: &XPathEvaluatorCapabilities,
) -> Vec<XPathEvaluationContractViolation> {
    let mut violations = validate_xpath_evaluator_capabilities(capabilities);
    if artifact.content_type != XPATH_RESULT_CONTENT_TYPE
        || artifact.schema_uri != XPATH_SCHEMA_URI
        || artifact.xpath_version != "3.1"
        || artifact.grammar_version != XPATH_GRAMMAR_VERSION
    {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-result-identity-invalid",
            "XPath result artifact identity does not match the package contract",
        );
    }
    if artifact.evaluator.evaluator_id != capabilities.evaluator_id
        || artifact.evaluator.evaluator_version != capabilities.evaluator_version
    {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-result-evaluator-mismatch",
            "XPath result artifact evaluator identity does not match the selected adapter",
        );
    }
    if artifact.resolver_policy_stamp.trim().is_empty()
        || artifact.safety_policy_stamp.trim().is_empty()
    {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-result-policy-stamp-missing",
            "XPath result artifact must retain resolver and safety policy stamps",
        );
    }
    if artifact.source_map.frames.is_empty() {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-result-source-map-required",
            "XPath result artifact must retain its expression source-map origin",
        );
    }
    if let Some(expected) = &artifact.expected_result {
        let item_count = artifact.sequence.items.len();
        if expected
            .min_items
            .is_some_and(|minimum| item_count < minimum)
            || expected
                .max_items
                .is_some_and(|maximum| item_count > maximum)
        {
            push_xpath_contract_violation(
                &mut violations,
                "xpath-result-cardinality-mismatch",
                "XPath result sequence does not satisfy the expected result contract",
            );
        }
    }
    validate_xpath_result_sequence(
        &artifact.sequence,
        &artifact.evaluator.evaluator_id,
        capabilities,
        &mut violations,
    );
    violations
}

fn validate_xpath_result_sequence(
    sequence: &XPathResultSequence,
    evaluator_id: &str,
    capabilities: &XPathEvaluatorCapabilities,
    violations: &mut Vec<XPathEvaluationContractViolation>,
) {
    if sequence.sequence_type.trim().is_empty() {
        push_xpath_contract_violation(
            violations,
            "xpath-result-sequence-type-missing",
            "XPath result sequence must retain its sequence type",
        );
    }
    for item in &sequence.items {
        if !capabilities.result_item_kinds.contains(&item.kind()) {
            push_xpath_contract_violation(
                violations,
                "xpath-result-item-kind-unsupported",
                format!(
                    "XPath result item kind `{:?}` is not supported",
                    item.kind()
                ),
            );
        }
        if item.source_map().frames.is_empty() {
            push_xpath_contract_violation(
                violations,
                "xpath-result-source-map-required",
                format!(
                    "XPath `{:?}` result item has no source-map origin",
                    item.kind()
                ),
            );
        }
        match item {
            XPathResultItem::Node {
                source_uri,
                node_id,
                ..
            } => {
                if source_uri.trim().is_empty() || node_id.trim().is_empty() {
                    push_xpath_contract_violation(
                        violations,
                        "xpath-result-node-identity-missing",
                        "XPath node result must retain source and node identity",
                    );
                }
            }
            XPathResultItem::Atomic { value, .. } => {
                validate_xpath_atomic_value(value, violations);
            }
            XPathResultItem::Map { entries, .. } => {
                for entry in entries {
                    validate_xpath_atomic_value(&entry.key, violations);
                    validate_xpath_result_sequence(
                        &entry.value,
                        evaluator_id,
                        capabilities,
                        violations,
                    );
                }
            }
            XPathResultItem::Array { members, .. } => {
                for member in members {
                    validate_xpath_result_sequence(member, evaluator_id, capabilities, violations);
                }
            }
            XPathResultItem::Function {
                evaluator_id: function_evaluator_id,
                function_id,
                signature,
                ..
            } => {
                if function_evaluator_id != evaluator_id
                    || function_id.trim().is_empty()
                    || signature.trim().is_empty()
                {
                    push_xpath_contract_violation(
                        violations,
                        "xpath-result-function-scope-invalid",
                        "XPath function result must be an evaluator-scoped typed handle",
                    );
                }
            }
        }
    }
}

fn validate_xpath_atomic_value(
    value: &XPathAtomicValue,
    violations: &mut Vec<XPathEvaluationContractViolation>,
) {
    if value.type_name.trim().is_empty() {
        push_xpath_contract_violation(
            violations,
            "xpath-result-atomic-type-missing",
            "XPath atomic result must retain its type name",
        );
    }
    if value.type_name == "xs:QName"
        && (value.namespace_uri.is_none() || value.local_name.as_deref().is_none_or(str::is_empty))
    {
        push_xpath_contract_violation(
            violations,
            "xpath-result-qname-identity-missing",
            "XPath QName result must retain expanded-name identity",
        );
    }
}

fn push_xpath_contract_violation(
    violations: &mut Vec<XPathEvaluationContractViolation>,
    code: &'static str,
    message: impl Into<String>,
) {
    violations.push(XPathEvaluationContractViolation {
        code,
        message: message.into(),
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathHostAttachment {
    pub owner: XPathHostOwner,
    pub expression_range: XPathSourceRange,
    pub static_context: XPathStaticContext,
    pub expected_result: Option<XPathExpectedResult>,
    pub evaluation_phase: XPathEvaluationPhase,
    pub resolver_policy_stamp: Option<String>,
    pub safety_policy_stamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XPathAttachment {
    Standalone { source_id: u32 },
    Host(XPathHostAttachment),
}

impl XPathAttachment {
    fn source_id(&self) -> u32 {
        match self {
            Self::Standalone { source_id } => *source_id,
            Self::Host(attachment) => attachment.owner.source_id,
        }
    }

    fn expression_origin(&self) -> XPathSourcePosition {
        match self {
            Self::Standalone { .. } => XPathSourcePosition {
                line: 1,
                column: 1,
                byte_offset: 0,
            },
            Self::Host(attachment) => attachment.expression_range.start,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum XPathFactKind {
    InvalidUtf8,
    LexicalError,
    ParseError,
    UnknownNamespacePrefix,
    UnclosedDelimiter,
    MismatchedDelimiter,
    HostAssociationInvalid,
    ExternalResourceDenied,
    SourceMapUnavailable,
    EventLifecycleInvalid,
    Parsed,
    HostAssociationObserved,
}

impl XPathFactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidUtf8 => "invalid-utf8",
            Self::LexicalError => "lexical-error",
            Self::ParseError => "parse-error",
            Self::UnknownNamespacePrefix => "unknown-namespace-prefix",
            Self::UnclosedDelimiter => "unclosed-delimiter",
            Self::MismatchedDelimiter => "mismatched-delimiter",
            Self::HostAssociationInvalid => "host-association-invalid",
            Self::ExternalResourceDenied => "external-resource-denied",
            Self::SourceMapUnavailable => "source-map-unavailable",
            Self::EventLifecycleInvalid => "event-lifecycle-invalid",
            Self::Parsed => "parsed",
            Self::HostAssociationObserved => "host-association-observed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathFact {
    pub kind: XPathFactKind,
    pub source_range: Option<XPathSourceRange>,
    pub message: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathDiagnosticBinding {
    pub fact_kind: String,
    pub contract: String,
    pub behavior: Option<String>,
    pub diagnostic_code: String,
    pub severity: Severity,
    pub policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathSchemaContractCatalog {
    pub fact_bindings: BTreeMap<String, XPathDiagnosticBinding>,
}

impl XPathSchemaContractCatalog {
    pub fn from_builtin() -> &'static Self {
        static CATALOG: OnceLock<XPathSchemaContractCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            let source = builtin_schema_package_source(XPATH_PACKAGE_ID)
                .expect("built-in XPath schema package source must be registered");
            Self::from_schema_source(source.schema_source)
        })
    }

    pub fn from_schema_source(schema_source: &str) -> Self {
        let model = compile_schema_document_model(XPATH_SCHEMA_URI, schema_source);
        let fact_bindings = model
            .constraints
            .values()
            .filter_map(|constraint| {
                if constraint.behavior.as_deref()?.trim() != XPATH_FACT_BEHAVIOR {
                    return None;
                }
                let fact_kind = constraint.fact_kind.as_deref()?.trim();
                let diagnostic_code = constraint.diagnostic.as_deref()?.trim();
                if fact_kind.is_empty() || diagnostic_code.is_empty() {
                    return None;
                }
                let diagnostic = model.diagnostics.get(diagnostic_code)?;
                Some((
                    fact_kind.to_owned(),
                    XPathDiagnosticBinding {
                        fact_kind: fact_kind.to_owned(),
                        contract: constraint.kind.clone(),
                        behavior: constraint.behavior.clone(),
                        diagnostic_code: diagnostic.code.clone(),
                        severity: diagnostic.severity,
                        policy: constraint.policy.clone(),
                    },
                ))
            })
            .collect();
        Self { fact_bindings }
    }

    fn binding_for_fact(&self, kind: XPathFactKind) -> Option<&XPathDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathExpressionAst {
    pub source: XPathExpressionSource,
    pub source_text: Option<String>,
    pub tokens: Vec<XPathTokenAst>,
    pub events: Vec<XPathAstEvent>,
    pub syntax_ast: Option<XPathSyntaxAst>,
    pub facts: Vec<XPathFact>,
    pub attachment: XPathAttachment,
    pub line_ending: Option<String>,
}

impl XPathExpressionAst {
    pub fn to_cemt_subject(&self) -> Value {
        let source_id = self.attachment.source_id();
        json!({
            "kind": "xpath-expression",
            "contentType": self.source.media_type,
            "schema": XPATH_SCHEMA_URI,
            "category": "xpath-expression",
            "grammarVersion": XPATH_GRAMMAR_VERSION,
            "source": {
                "uri": self.source.uri,
                "contentType": self.source.content_type,
                "mediaType": self.source.media_type,
                "parameters": self.source.parameters,
                "byteLength": self.source.byte_length,
            },
            "sourceText": self.source_text,
            "tokens": self.tokens.iter().map(|token| json!({
                "index": token.index,
                "kind": token.kind.as_str(),
                "lexeme": token.lexeme,
                "depth": token.depth,
                "role": token.kind.role(),
                "sourceRange": token.source_range.to_cemt_subject(),
                "sourceMap": token.source_range.source_map(source_id, &self.source.media_type),
            })).collect::<Vec<_>>(),
            "events": self.events.iter().map(|event| json!({
                "index": event.index,
                "kind": event.kind.as_str(),
                "tokenIndex": event.token_index,
                "depth": event.depth,
                "sourceRange": event.source_range.to_cemt_subject(),
                "sourceMap": event.source_range.source_map(source_id, &self.source.media_type),
            })).collect::<Vec<_>>(),
            "syntaxAst": self.syntax_ast.as_ref().map(|syntax| xpath_syntax_to_cemt_subject(
                syntax,
                source_id,
                &self.source.media_type,
            )),
            "parseFacts": self.facts.iter().map(|fact| json!({
                "kind": fact.kind.as_str(),
                "sourceRange": fact.source_range.map(XPathSourceRange::to_cemt_subject),
                "message": fact.message,
                "value": fact.value,
            })).collect::<Vec<_>>(),
            "attachment": xpath_attachment_to_cemt_subject(&self.attachment),
            "lineEnding": self.line_ending,
        })
    }
}

pub fn validate_xpath_source_bytes(request: XPathSourceRequest<'_>) -> Vec<Diagnostic> {
    let ast = xpath_expression_ast_from_source_bytes(
        request,
        XPathAttachment::Standalone { source_id: 1 },
    );
    validate_xpath_expression_ast(&ast, XPathSchemaContractCatalog::from_builtin())
}

pub fn validate_xpath_expression_ast(
    ast: &XPathExpressionAst,
    contracts: &XPathSchemaContractCatalog,
) -> Vec<Diagnostic> {
    let source_id = ast.attachment.source_id();
    ast.facts
        .iter()
        .filter_map(|fact| {
            let binding = contracts.binding_for_fact(fact.kind)?;
            let (line, column, byte_offset) = fact
                .source_range
                .map(|range| {
                    (
                        Some(range.start.line),
                        Some(range.start.column),
                        Some(range.start.byte_offset),
                    )
                })
                .unwrap_or((None, None, None));
            Some(Diagnostic {
                uri: Some(ast.source.uri.clone()),
                line,
                column,
                byte_offset,
                code: binding.diagnostic_code.clone(),
                severity: binding.severity,
                message: fact.message.clone(),
                details: Some(json!({
                    "xpath": {
                        "phase": "parse",
                        "factKind": fact.kind.as_str(),
                        "contract": binding.contract,
                        "behavior": binding.behavior,
                        "policy": binding.policy,
                        "schema": XPATH_SCHEMA_URI,
                        "schemaPackage": XPATH_PACKAGE_ID,
                        "contentType": ast.source.content_type,
                        "mediaType": ast.source.media_type,
                        "byteLength": fact.source_range.map(|range| range.byte_length),
                        "value": fact.value,
                        "attachmentKind": match ast.attachment {
                            XPathAttachment::Standalone { .. } => "standalone",
                            XPathAttachment::Host(_) => "host",
                        },
                    }
                })),
                source_map: fact
                    .source_range
                    .map(|range| range.source_map(source_id, ast.source.media_type.as_str())),
                ..Diagnostic::default()
            })
        })
        .collect()
}

pub fn xpath_expression_ast_from_source_bytes(
    request: XPathSourceRequest<'_>,
    attachment: XPathAttachment,
) -> XPathExpressionAst {
    let content_type = request.content_type.unwrap_or(XPATH_CONTENT_TYPE);
    let source = XPathExpressionSource {
        uri: request.source_uri.to_owned(),
        content_type: content_type.to_owned(),
        media_type: content_type_essence(content_type),
        parameters: content_type_parameters(request.content_type),
        byte_length: request.bytes.len(),
    };
    let origin = attachment.expression_origin();
    let source_text = match std::str::from_utf8(request.bytes) {
        Ok(source_text) => source_text,
        Err(error) => {
            let line_index = LineIndex::from_bytes_lossy(request.bytes);
            let start = error.valid_up_to();
            let end = start.saturating_add(error.error_len().unwrap_or(1));
            return XPathExpressionAst {
                source,
                source_text: None,
                tokens: Vec::new(),
                events: Vec::new(),
                syntax_ast: None,
                facts: vec![XPathFact {
                    kind: XPathFactKind::InvalidUtf8,
                    source_range: request.source_range_projector.is_none().then(|| {
                        XPathSourceRange::from_offsets(
                            &line_index,
                            origin,
                            start,
                            end.min(request.bytes.len()),
                        )
                    }),
                    message: format!("XPath source must be valid UTF-8: {error}"),
                    value: Some(start.to_string()),
                }],
                attachment,
                line_ending: detect_line_ending_style_bytes(request.bytes).map(str::to_owned),
            };
        }
    };

    let line_index = LineIndex::from_utf8(source_text);
    let Some(range_resolver) = XPathSourceRangeResolver::new(
        source_text,
        &line_index,
        origin,
        request.source_range_projector,
    ) else {
        return XPathExpressionAst {
            source,
            source_text: Some(source_text.to_owned()),
            tokens: Vec::new(),
            events: Vec::new(),
            syntax_ast: None,
            facts: vec![XPathFact {
                kind: XPathFactKind::SourceMapUnavailable,
                source_range: None,
                message: "XPath source projector does not cover every UTF-8 scalar boundary"
                    .to_owned(),
                value: None,
            }],
            attachment,
            line_ending: detect_line_ending_style_bytes(request.bytes).map(str::to_owned),
        };
    };
    let lexical_tokens = lexer::xpath_lexical_tokens(source_text);
    let mut tokens = xpath_lossless_tokens(source_text, &lexical_tokens, &range_resolver);
    let mut facts = xpath_delimiter_facts(&mut tokens);
    if let XPathAttachment::Host(host) = &attachment {
        let projected_expression_range = range_resolver.range(0, source_text.len());
        facts.push(XPathFact {
            kind: XPathFactKind::HostAssociationObserved,
            source_range: Some(projected_expression_range),
            message: "XPath expression is associated with a host AST node".to_owned(),
            value: Some("host".to_owned()),
        });
        facts.extend(xpath_host_attachment_facts(
            request,
            host,
            projected_expression_range,
        ));
    }
    facts.extend(xpath_external_resource_facts(&tokens, &attachment));

    let has_lexical_error = tokens
        .iter()
        .any(|token| token.kind == XPathTokenKind::Error);
    for token in tokens
        .iter()
        .filter(|token| token.kind == XPathTokenKind::Error)
    {
        facts.push(XPathFact {
            kind: XPathFactKind::LexicalError,
            source_range: Some(token.source_range),
            message: format!("XPath lexical error at `{}`", token.lexeme),
            value: Some(token.lexeme.clone()),
        });
    }

    let syntax_ast = if has_lexical_error {
        None
    } else {
        match parser::parse_xpath(source_text, &lexical_tokens, &range_resolver, &attachment) {
            Ok(syntax_ast) => {
                facts.push(XPathFact {
                    kind: XPathFactKind::Parsed,
                    source_range: Some(range_resolver.range(0, source_text.len())),
                    message: "XPath 3.1 expression parsed successfully".to_owned(),
                    value: Some("xpath-3.1".to_owned()),
                });
                Some(syntax_ast)
            }
            Err(error) => {
                let start = error.start.min(source_text.len());
                let end = error.end.min(source_text.len()).max(start);
                let kind = match error.kind {
                    parser::XPathParseErrorKind::UnknownNamespacePrefix => {
                        XPathFactKind::UnknownNamespacePrefix
                    }
                    parser::XPathParseErrorKind::Syntax => XPathFactKind::ParseError,
                };
                facts.push(XPathFact {
                    kind,
                    source_range: Some(range_resolver.range(start, end)),
                    message: error.message(),
                    value: Some(format!("{error:?}")),
                });
                None
            }
        }
    };

    let events = xpath_ast_events(&tokens, &range_resolver, source_text.len());
    facts.extend(xpath_stream_invariant_facts(&tokens, &events));
    XPathExpressionAst {
        source,
        source_text: Some(source_text.to_owned()),
        tokens,
        events,
        syntax_ast,
        facts,
        attachment,
        line_ending: detect_line_ending_style_bytes(request.bytes).map(str::to_owned),
    }
}

fn xpath_host_attachment_facts(
    request: XPathSourceRequest<'_>,
    host: &XPathHostAttachment,
    projected_expression_range: XPathSourceRange,
) -> Vec<XPathFact> {
    let expression_start = host.expression_range.start.byte_offset;
    let expression_end = expression_start.saturating_add(host.expression_range.byte_length);
    let owner_start = host.owner.source_range.start.byte_offset;
    let owner_end = owner_start.saturating_add(host.owner.source_range.byte_length);
    let mut failures = Vec::new();
    if host.expression_range != projected_expression_range {
        failures.push(format!(
            "expression range {:?} does not match projected source range {:?}",
            host.expression_range, projected_expression_range
        ));
    }
    if expression_start < owner_start || expression_end > owner_end {
        failures.push("expression range is outside the owning host node range".to_owned());
    }
    if host.owner.source_uri != request.source_uri {
        failures.push(format!(
            "owner source URI `{}` does not match expression source URI `{}`",
            host.owner.source_uri, request.source_uri
        ));
    }
    if failures.is_empty() {
        Vec::new()
    } else {
        vec![XPathFact {
            kind: XPathFactKind::HostAssociationInvalid,
            source_range: Some(host.expression_range),
            message: format!("Invalid XPath host association: {}", failures.join("; ")),
            value: host.owner.node_id.clone(),
        }]
    }
}

fn xpath_external_resource_facts(
    tokens: &[XPathTokenAst],
    attachment: &XPathAttachment,
) -> Vec<XPathFact> {
    let resolver_policy_present = matches!(
        attachment,
        XPathAttachment::Host(XPathHostAttachment {
            resolver_policy_stamp: Some(stamp),
            ..
        }) if !stamp.trim().is_empty()
    );
    if resolver_policy_present {
        return Vec::new();
    }

    let significant = tokens
        .iter()
        .filter(|token| {
            !matches!(
                token.kind,
                XPathTokenKind::Whitespace | XPathTokenKind::Comment
            )
        })
        .collect::<Vec<_>>();
    significant
        .windows(2)
        .filter_map(|pair| {
            let name = pair[0].lexeme.rsplit(':').next().unwrap_or_default();
            let is_external_function = matches!(
                name,
                "collection"
                    | "doc"
                    | "json-doc"
                    | "unparsed-text"
                    | "unparsed-text-available"
                    | "unparsed-text-lines"
                    | "uri-collection"
            );
            (is_external_function && pair[1].lexeme == "(").then(|| XPathFact {
                kind: XPathFactKind::ExternalResourceDenied,
                source_range: Some(pair[0].source_range),
                message: format!(
                    "XPath function `{}` requires an explicit resolver policy",
                    pair[0].lexeme
                ),
                value: Some(pair[0].lexeme.clone()),
            })
        })
        .collect()
}

fn xpath_stream_invariant_facts(
    tokens: &[XPathTokenAst],
    events: &[XPathAstEvent],
) -> Vec<XPathFact> {
    let mut facts = Vec::new();
    if tokens
        .iter()
        .any(|token| token.source_range.byte_length == 0)
    {
        facts.push(XPathFact {
            kind: XPathFactKind::SourceMapUnavailable,
            source_range: None,
            message: "XPath token stream contains a token without an exact source range".to_owned(),
            value: None,
        });
    }
    let lifecycle_valid = events.len() == tokens.len() + 2
        && events
            .first()
            .is_some_and(|event| event.kind == XPathAstEventKind::StartExpression)
        && events
            .last()
            .is_some_and(|event| event.kind == XPathAstEventKind::EndExpression)
        && events[1..events.len().saturating_sub(1)]
            .iter()
            .zip(tokens)
            .all(|(event, token)| {
                event.kind == XPathAstEventKind::Token
                    && event.token_index == Some(token.index)
                    && event.source_range == token.source_range
            });
    if !lifecycle_valid {
        facts.push(XPathFact {
            kind: XPathFactKind::EventLifecycleInvalid,
            source_range: None,
            message: "XPath AST event lifecycle does not match the lossless token stream"
                .to_owned(),
            value: None,
        });
    }
    facts
}

fn xpath_ast_events(
    tokens: &[XPathTokenAst],
    range_resolver: &XPathSourceRangeResolver,
    source_len: usize,
) -> Vec<XPathAstEvent> {
    let mut events = Vec::with_capacity(tokens.len() + 2);
    events.push(XPathAstEvent {
        index: 0,
        kind: XPathAstEventKind::StartExpression,
        token_index: None,
        depth: 0,
        source_range: range_resolver.range(0, 0),
    });
    events.extend(tokens.iter().map(|token| XPathAstEvent {
        index: token.index + 1,
        kind: XPathAstEventKind::Token,
        token_index: Some(token.index),
        depth: token.depth,
        source_range: token.source_range,
    }));
    events.push(XPathAstEvent {
        index: tokens.len() + 1,
        kind: XPathAstEventKind::EndExpression,
        token_index: None,
        depth: 0,
        source_range: range_resolver.range(source_len, source_len),
    });
    events
}

fn xpath_lossless_tokens(
    source: &str,
    lexical_tokens: &[lexer::XPathLexicalToken<'_>],
    range_resolver: &XPathSourceRangeResolver,
) -> Vec<XPathTokenAst> {
    let mut tokens = Vec::new();
    for token in lexical_tokens {
        xpath_push_token(
            source,
            range_resolver,
            token.start,
            token.end,
            token.kind.presentation_kind(),
            &mut tokens,
        );
    }
    tokens
}

fn xpath_push_token(
    source: &str,
    range_resolver: &XPathSourceRangeResolver,
    start: usize,
    end: usize,
    kind: XPathTokenKind,
    tokens: &mut Vec<XPathTokenAst>,
) {
    if start >= end || end > source.len() {
        return;
    }
    tokens.push(XPathTokenAst {
        index: tokens.len(),
        kind,
        lexeme: source[start..end].to_owned(),
        depth: 0,
        source_range: range_resolver.range(start, end),
    });
}

#[cfg(test)]
fn xpath_token_kind(token: &XeeToken<'_>) -> XPathTokenKind {
    use XeeToken::*;
    match token {
        Error => XPathTokenKind::Error,
        IntegerLiteral(_) | DecimalLiteral(_) | DoubleLiteral(_) => XPathTokenKind::Number,
        StringLiteral(_) => XPathTokenKind::String,
        PrefixedQName(_)
        | URIQualifiedName(_)
        | LocalNameWildcard(_)
        | PrefixWildcard(_)
        | BracedURILiteralWildcard(_)
        | NCName(_)
        | BracedURILiteral(_) => XPathTokenKind::Name,
        Dollar => XPathTokenKind::VariableSigil,
        Whitespace => XPathTokenKind::Whitespace,
        CommentStart => XPathTokenKind::Comment,
        ExclamationMark | NotEqual | Asterisk | Plus | Minus | Slash | DoubleSlash | LessThan
        | Precedes | LessThanEqual | Equal | Arrow | GreaterThan | GreaterThanEqual | Follows
        | Pipe | DoublePipe | ColonEqual | And | Or | Div | Idiv | Mod | Eq | Ne | Lt | Le | Gt
        | Ge | Is | To | Union | Intersect | Except => XPathTokenKind::Operator,
        Ancestor
        | AncestorOrSelf
        | Array
        | As
        | Attribute
        | Cast
        | Castable
        | Child
        | Comment
        | Descendant
        | DescendantOrSelf
        | DocumentNode
        | Element
        | Else
        | EmptySequence
        | Every
        | Following
        | FollowingSibling
        | For
        | Function
        | If
        | In
        | Instance
        | Item
        | Let
        | Map
        | Namespace
        | NamespaceNode
        | Node
        | Of
        | Parent
        | Preceding
        | PrecedingSibling
        | ProcessingInstruction
        | Return
        | Satisfies
        | SchemaAttribute
        | SchemaElement
        | Self_
        | Some
        | Text
        | Then
        | Treat
        | Switch
        | Typeswitch => XPathTokenKind::Keyword,
        _ => XPathTokenKind::Punctuation,
    }
}

fn xpath_delimiter_facts(tokens: &mut [XPathTokenAst]) -> Vec<XPathFact> {
    let mut stack = Vec::<(char, XPathSourceRange)>::new();
    let mut facts = Vec::new();
    for token in tokens {
        let delimiter = match token.lexeme.as_str() {
            "(" | "[" | "{" | ")" | "]" | "}" => token.lexeme.chars().next(),
            _ => None,
        };
        match delimiter {
            Some(close @ (')' | ']' | '}')) => {
                let expected = matching_open_delimiter(close);
                if stack.last().is_some_and(|(open, _)| *open == expected) {
                    stack.pop();
                } else {
                    facts.push(XPathFact {
                        kind: XPathFactKind::MismatchedDelimiter,
                        source_range: Some(token.source_range),
                        message: format!(
                            "XPath closing delimiter `{close}` has no matching `{expected}`"
                        ),
                        value: Some(close.to_string()),
                    });
                }
                token.depth = stack.len();
            }
            Some(open @ ('(' | '[' | '{')) => {
                token.depth = stack.len();
                stack.push((open, token.source_range));
            }
            _ => token.depth = stack.len(),
        }
    }
    for (open, source_range) in stack.into_iter().rev() {
        facts.push(XPathFact {
            kind: XPathFactKind::UnclosedDelimiter,
            source_range: Some(source_range),
            message: format!("XPath opening delimiter `{open}` is not closed"),
            value: Some(open.to_string()),
        });
    }
    facts
}

fn matching_open_delimiter(close: char) -> char {
    match close {
        ')' => '(',
        ']' => '[',
        '}' => '{',
        _ => close,
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
enum XPathNameUse {
    Element,
    Attribute,
    Function,
    Variable,
}

#[cfg(test)]
struct XPathSyntaxLowerer<'a> {
    source: &'a str,
    line_index: &'a LineIndex,
    origin: XPathSourcePosition,
    attachment: &'a XPathAttachment,
}

#[cfg(test)]
impl<'a> XPathSyntaxLowerer<'a> {
    fn new(
        source: &'a str,
        line_index: &'a LineIndex,
        origin: XPathSourcePosition,
        attachment: &'a XPathAttachment,
    ) -> Self {
        Self {
            source,
            line_index,
            origin,
            attachment,
        }
    }

    fn lower(&self, xpath: &xee_ast::XPath) -> XPathSyntaxAst {
        XPathSyntaxAst::new(self.lower_expr_s(&xpath.0))
    }

    fn lower_expr_s(&self, expression: &xee_ast::ExprS) -> XPathExpressionSequence {
        self.lower_expr(
            &expression.value,
            expression.span.start,
            expression.span.end,
        )
    }

    fn lower_expr(
        &self,
        expression: &xee_ast::Expr,
        start: usize,
        end: usize,
    ) -> XPathExpressionSequence {
        XPathExpressionSequence {
            expressions: expression
                .0
                .iter()
                .map(|expression| self.lower_expr_single(expression))
                .collect(),
            source_range: self.range(start, end),
        }
    }

    fn lower_expr_single(&self, expression: &xee_ast::ExprSingleS) -> XPathExpressionNode {
        let source_range = self.range(expression.span.start, expression.span.end);
        if let xee_ast::ExprSingle::Path(path) = &expression.value {
            if let Some(inner) = synthetic_wrapped_expression(path) {
                let mut lowered = self.lower_expr_single(inner);
                lowered.source_range = source_range;
                return lowered;
            }
        }
        let expression = match &expression.value {
            xee_ast::ExprSingle::Path(path) => XPathExpression::Path(self.lower_path(
                path,
                expression.span.start,
                expression.span.end,
            )),
            xee_ast::ExprSingle::Binary(binary) => XPathExpression::Binary {
                operator: self.lower_binary_operator(binary.operator),
                left: Box::new(self.lower_path_node(
                    &binary.left,
                    expression.span.start,
                    expression.span.end,
                )),
                right: Box::new(self.lower_path_node(
                    &binary.right,
                    expression.span.start,
                    expression.span.end,
                )),
            },
            xee_ast::ExprSingle::For(for_expression) => XPathExpression::For {
                binding: self.lower_name_s(&for_expression.var_name, XPathNameUse::Variable),
                binding_expression: Box::new(self.lower_expr_single(&for_expression.var_expr)),
                return_expression: Box::new(self.lower_expr_single(&for_expression.return_expr)),
            },
            xee_ast::ExprSingle::Apply(_) => XPathExpression::Unsupported {
                production: "apply-expression".to_owned(),
            },
            xee_ast::ExprSingle::Let(_) => XPathExpression::Unsupported {
                production: "let-expression".to_owned(),
            },
            xee_ast::ExprSingle::If(_) => XPathExpression::Unsupported {
                production: "if-expression".to_owned(),
            },
            xee_ast::ExprSingle::Quantified(_) => XPathExpression::Unsupported {
                production: "quantified-expression".to_owned(),
            },
        };
        XPathExpressionNode {
            expression,
            source_range,
        }
    }

    fn lower_path_node(
        &self,
        path: &xee_ast::PathExpr,
        fallback_start: usize,
        fallback_end: usize,
    ) -> XPathExpressionNode {
        if let Some(inner) = synthetic_wrapped_expression(path) {
            return self.lower_expr_single(inner);
        }
        let path = self.lower_path(path, fallback_start, fallback_end);
        XPathExpressionNode {
            source_range: path.source_range,
            expression: XPathExpression::Path(path),
        }
    }

    fn lower_path(
        &self,
        path: &xee_ast::PathExpr,
        fallback_start: usize,
        fallback_end: usize,
    ) -> XPathPathExpression {
        let (start, end) = self.path_bounds(path, fallback_start, fallback_end);
        let lexical = self.slice(start, end).trim_start();
        let root = if lexical.starts_with("//") {
            XPathPathRoot::RootedDescendant
        } else if lexical.starts_with('/') {
            XPathPathRoot::Rooted
        } else {
            XPathPathRoot::Relative
        };
        let synthetic_steps = match root {
            XPathPathRoot::Relative => 0,
            XPathPathRoot::Rooted => 1,
            XPathPathRoot::RootedDescendant => 2,
        };
        XPathPathExpression {
            root,
            steps: path
                .steps
                .iter()
                .skip(synthetic_steps)
                .map(|step| self.lower_step(step))
                .collect(),
            source_range: self.range(start, end),
        }
    }

    fn path_bounds(
        &self,
        path: &xee_ast::PathExpr,
        fallback_start: usize,
        fallback_end: usize,
    ) -> (usize, usize) {
        let mut spans = path
            .steps
            .iter()
            .filter(|step| step.span.end > step.span.start);
        let Some(first) = spans.next() else {
            return (fallback_start, fallback_end);
        };
        let mut end = first.span.end;
        for step in spans {
            end = end.max(step.span.end);
        }
        (first.span.start, end)
    }

    fn lower_step(&self, step: &xee_ast::StepExprS) -> XPathStepNode {
        let source_range = self.range(step.span.start, step.span.end);
        let step = match &step.value {
            xee_ast::StepExpr::AxisStep(axis_step) => XPathStep::Axis {
                axis: self.lower_axis(&axis_step.axis),
                node_test: self.lower_node_test(
                    &axis_step.node_test,
                    &axis_step.axis,
                    step.span.start,
                    step.span.end,
                ),
                predicates: axis_step
                    .predicates
                    .iter()
                    .map(|predicate| self.lower_expr_s(predicate))
                    .collect(),
            },
            xee_ast::StepExpr::PrimaryExpr(primary) => {
                XPathStep::Primary(self.lower_primary(primary))
            }
            xee_ast::StepExpr::PostfixExpr { primary, postfixes } => XPathStep::Postfix {
                primary: self.lower_primary(primary),
                postfixes: postfixes
                    .iter()
                    .map(|postfix| self.lower_postfix(postfix))
                    .collect(),
            },
        };
        XPathStepNode { step, source_range }
    }

    fn lower_primary(&self, primary: &xee_ast::PrimaryExprS) -> XPathPrimaryExpression {
        match &primary.value {
            xee_ast::PrimaryExpr::Literal(literal) => XPathPrimaryExpression::Literal(
                self.lower_literal(literal, primary.span.start, primary.span.end),
            ),
            xee_ast::PrimaryExpr::VarRef(_) => {
                let lexical = self.slice(primary.span.start, primary.span.end);
                let name_start = primary
                    .span
                    .start
                    .saturating_add(lexical.find('$').map_or(0, |index| index + 1));
                XPathPrimaryExpression::VariableReference(self.lower_name_range(
                    name_start,
                    primary.span.end,
                    XPathNameUse::Variable,
                ))
            }
            xee_ast::PrimaryExpr::Expr(expression) => {
                XPathPrimaryExpression::Parenthesized(expression.value.as_ref().map(|expression| {
                    Box::new(
                        self.lower_expr(
                            expression,
                            expression
                                .0
                                .first()
                                .map_or(primary.span.start, |item| item.span.start),
                            expression
                                .0
                                .last()
                                .map_or(primary.span.end, |item| item.span.end),
                        ),
                    )
                }))
            }
            xee_ast::PrimaryExpr::ContextItem => XPathPrimaryExpression::ContextItem,
            xee_ast::PrimaryExpr::FunctionCall(function) => XPathPrimaryExpression::FunctionCall {
                name: self.lower_name_s(&function.name, XPathNameUse::Function),
                arguments: function
                    .arguments
                    .iter()
                    .map(|argument| self.lower_expr_single(argument))
                    .collect(),
            },
            xee_ast::PrimaryExpr::MapConstructor(map) => XPathPrimaryExpression::MapConstructor {
                entries: map
                    .entries
                    .iter()
                    .map(|entry| XPathMapConstructorEntry {
                        source_range: self.range(entry.key.span.start, entry.value.span.end),
                        key: self.lower_expr_single(&entry.key),
                        value: self.lower_expr_single(&entry.value),
                    })
                    .collect(),
            },
            xee_ast::PrimaryExpr::ArrayConstructor(array) => {
                XPathPrimaryExpression::ArrayConstructor(match array {
                    xee_ast::ArrayConstructor::Square(expression) => {
                        XPathArrayConstructor::Square(self.lower_expr_s(expression))
                    }
                    xee_ast::ArrayConstructor::Curly(expression) => {
                        XPathArrayConstructor::Curly(expression.value.as_ref().map(|expression| {
                            Box::new(
                                self.lower_expr(
                                    expression,
                                    expression
                                        .0
                                        .first()
                                        .map_or(primary.span.start, |item| item.span.start),
                                    expression
                                        .0
                                        .last()
                                        .map_or(primary.span.end, |item| item.span.end),
                                ),
                            )
                        }))
                    }
                })
            }
            xee_ast::PrimaryExpr::NamedFunctionRef(_) => XPathPrimaryExpression::Unsupported {
                production: "named-function-reference".to_owned(),
            },
            xee_ast::PrimaryExpr::InlineFunction(_) => XPathPrimaryExpression::Unsupported {
                production: "inline-function-expression".to_owned(),
            },
            xee_ast::PrimaryExpr::UnaryLookup(_) => XPathPrimaryExpression::Unsupported {
                production: "unary-lookup".to_owned(),
            },
        }
    }

    fn lower_postfix(&self, postfix: &xee_ast::Postfix) -> XPathPostfixExpression {
        match postfix {
            xee_ast::Postfix::Predicate(expression) => {
                XPathPostfixExpression::Predicate(self.lower_expr_s(expression))
            }
            xee_ast::Postfix::ArgumentList(arguments) => XPathPostfixExpression::ArgumentList(
                arguments
                    .iter()
                    .map(|argument| self.lower_expr_single(argument))
                    .collect(),
            ),
            xee_ast::Postfix::Lookup(key) => XPathPostfixExpression::Lookup {
                lexical: self.key_specifier_lexical(key),
            },
        }
    }

    fn key_specifier_lexical(&self, key: &xee_ast::KeySpecifier) -> String {
        match key {
            xee_ast::KeySpecifier::NcName(name) => name.clone(),
            xee_ast::KeySpecifier::Integer(integer) => integer.to_string(),
            xee_ast::KeySpecifier::Expr(expression) => self
                .slice(expression.span.start, expression.span.end)
                .to_owned(),
            xee_ast::KeySpecifier::Star => "*".to_owned(),
        }
    }

    fn lower_literal(&self, literal: &xee_ast::Literal, start: usize, end: usize) -> XPathLiteral {
        let (kind, value) = match literal {
            xee_ast::Literal::Integer(value) => (XPathLiteralKind::Integer, value.to_string()),
            xee_ast::Literal::Decimal(value) => (XPathLiteralKind::Decimal, value.to_string()),
            xee_ast::Literal::Double(value) => (XPathLiteralKind::Double, value.to_string()),
            xee_ast::Literal::String(value) => (XPathLiteralKind::String, value.clone()),
        };
        XPathLiteral {
            kind,
            lexical: self.slice(start, end).to_owned(),
            value,
        }
    }

    fn lower_axis(&self, axis: &xee_ast::Axis) -> XPathAxis {
        match axis {
            xee_ast::Axis::Ancestor => XPathAxis::Ancestor,
            xee_ast::Axis::AncestorOrSelf => XPathAxis::AncestorOrSelf,
            xee_ast::Axis::Attribute => XPathAxis::Attribute,
            xee_ast::Axis::Child => XPathAxis::Child,
            xee_ast::Axis::Descendant => XPathAxis::Descendant,
            xee_ast::Axis::DescendantOrSelf => XPathAxis::DescendantOrSelf,
            xee_ast::Axis::Following => XPathAxis::Following,
            xee_ast::Axis::FollowingSibling => XPathAxis::FollowingSibling,
            xee_ast::Axis::Namespace => XPathAxis::Namespace,
            xee_ast::Axis::Parent => XPathAxis::Parent,
            xee_ast::Axis::Preceding => XPathAxis::Preceding,
            xee_ast::Axis::PrecedingSibling => XPathAxis::PrecedingSibling,
            xee_ast::Axis::Self_ => XPathAxis::SelfAxis,
        }
    }

    fn lower_node_test(
        &self,
        node_test: &xee_ast::NodeTest,
        axis: &xee_ast::Axis,
        start: usize,
        end: usize,
    ) -> XPathNodeTest {
        match node_test {
            xee_ast::NodeTest::NameTest(name_test) => XPathNodeTest::Name(match name_test {
                xee_ast::NameTest::Name(name) => XPathNameTest::Name(self.lower_name_s(
                    name,
                    if matches!(axis, xee_ast::Axis::Attribute) {
                        XPathNameUse::Attribute
                    } else {
                        XPathNameUse::Element
                    },
                )),
                xee_ast::NameTest::Star => XPathNameTest::Any,
                xee_ast::NameTest::LocalName(local_name) => XPathNameTest::AnyNamespace {
                    local_name: local_name.clone(),
                },
                xee_ast::NameTest::Namespace(namespace_uri) => XPathNameTest::Namespace {
                    namespace_uri: namespace_uri.clone(),
                },
            }),
            xee_ast::NodeTest::KindTest(kind_test) => XPathNodeTest::Kind {
                kind: self.lower_kind_test(kind_test),
                lexical: self.node_test_lexical(start, end),
            },
        }
    }

    fn node_test_lexical(&self, start: usize, end: usize) -> String {
        let step = self.slice(start, end);
        let test = step.rsplit_once("::").map_or(step, |(_, test)| test);
        test.split('[').next().unwrap_or(test).trim().to_owned()
    }

    fn lower_kind_test(&self, test: &xee_ast::KindTest) -> XPathKindTest {
        match test {
            xee_ast::KindTest::Document(_) => XPathKindTest::Document,
            xee_ast::KindTest::Element(_) => XPathKindTest::Element,
            xee_ast::KindTest::Attribute(_) => XPathKindTest::Attribute,
            xee_ast::KindTest::SchemaElement(_) => XPathKindTest::SchemaElement,
            xee_ast::KindTest::SchemaAttribute(_) => XPathKindTest::SchemaAttribute,
            xee_ast::KindTest::PI(_) => XPathKindTest::ProcessingInstruction,
            xee_ast::KindTest::Comment => XPathKindTest::Comment,
            xee_ast::KindTest::Text => XPathKindTest::Text,
            xee_ast::KindTest::NamespaceNode => XPathKindTest::NamespaceNode,
            xee_ast::KindTest::Any => XPathKindTest::AnyNode,
        }
    }

    fn lower_binary_operator(&self, operator: xee_ast::BinaryOperator) -> XPathBinaryOperator {
        match operator {
            xee_ast::BinaryOperator::Or => XPathBinaryOperator::Or,
            xee_ast::BinaryOperator::And => XPathBinaryOperator::And,
            xee_ast::BinaryOperator::ValueEq => XPathBinaryOperator::ValueEqual,
            xee_ast::BinaryOperator::ValueNe => XPathBinaryOperator::ValueNotEqual,
            xee_ast::BinaryOperator::ValueLt => XPathBinaryOperator::ValueLessThan,
            xee_ast::BinaryOperator::ValueLe => XPathBinaryOperator::ValueLessThanOrEqual,
            xee_ast::BinaryOperator::ValueGt => XPathBinaryOperator::ValueGreaterThan,
            xee_ast::BinaryOperator::ValueGe => XPathBinaryOperator::ValueGreaterThanOrEqual,
            xee_ast::BinaryOperator::GenEq => XPathBinaryOperator::GeneralEqual,
            xee_ast::BinaryOperator::GenNe => XPathBinaryOperator::GeneralNotEqual,
            xee_ast::BinaryOperator::GenLt => XPathBinaryOperator::GeneralLessThan,
            xee_ast::BinaryOperator::GenLe => XPathBinaryOperator::GeneralLessThanOrEqual,
            xee_ast::BinaryOperator::GenGt => XPathBinaryOperator::GeneralGreaterThan,
            xee_ast::BinaryOperator::GenGe => XPathBinaryOperator::GeneralGreaterThanOrEqual,
            xee_ast::BinaryOperator::Is => XPathBinaryOperator::NodeIs,
            xee_ast::BinaryOperator::Precedes => XPathBinaryOperator::NodePrecedes,
            xee_ast::BinaryOperator::Follows => XPathBinaryOperator::NodeFollows,
            xee_ast::BinaryOperator::Concat => XPathBinaryOperator::Concatenate,
            xee_ast::BinaryOperator::Range => XPathBinaryOperator::Range,
            xee_ast::BinaryOperator::Add => XPathBinaryOperator::Add,
            xee_ast::BinaryOperator::Sub => XPathBinaryOperator::Subtract,
            xee_ast::BinaryOperator::Mul => XPathBinaryOperator::Multiply,
            xee_ast::BinaryOperator::Div => XPathBinaryOperator::Divide,
            xee_ast::BinaryOperator::IntDiv => XPathBinaryOperator::IntegerDivide,
            xee_ast::BinaryOperator::Mod => XPathBinaryOperator::Modulo,
            xee_ast::BinaryOperator::Union => XPathBinaryOperator::Union,
            xee_ast::BinaryOperator::Intersect => XPathBinaryOperator::Intersect,
            xee_ast::BinaryOperator::Except => XPathBinaryOperator::Except,
            xee_ast::BinaryOperator::Comma => XPathBinaryOperator::Sequence,
        }
    }

    fn lower_name_s(&self, name: &xee_ast::NameS, name_use: XPathNameUse) -> XPathName {
        self.lower_name_range(name.span.start, name.span.end, name_use)
    }

    fn lower_name_range(&self, start: usize, end: usize, name_use: XPathNameUse) -> XPathName {
        let lexical = self.slice(start, end).trim().trim_start_matches('$');
        let (prefix, local_name, explicit_namespace) =
            if let Some(rest) = lexical.strip_prefix("Q{") {
                if let Some((namespace, local_name)) = rest.split_once('}') {
                    (None, local_name.to_owned(), Some(namespace.to_owned()))
                } else {
                    (None, lexical.to_owned(), None)
                }
            } else if let Some((prefix, local_name)) = lexical.split_once(':') {
                (Some(prefix.to_owned()), local_name.to_owned(), None)
            } else {
                (None, lexical.to_owned(), None)
            };
        let namespace_uri = explicit_namespace.or_else(|| match prefix.as_deref() {
            Some(prefix) => self.namespace_for_prefix(prefix),
            None => self.default_namespace(name_use),
        });
        XPathName {
            lexical: lexical.to_owned(),
            prefix,
            local_name,
            namespace_uri,
            source_range: self.range(start, end),
        }
    }

    fn namespace_for_prefix(&self, prefix: &str) -> Option<String> {
        let host_namespace = match self.attachment {
            XPathAttachment::Host(host) => host.static_context.namespaces.get(prefix).cloned(),
            XPathAttachment::Standalone { .. } => None,
        };
        host_namespace.or_else(|| {
            match prefix {
                "xml" => Some("http://www.w3.org/XML/1998/namespace"),
                "xs" => Some("http://www.w3.org/2001/XMLSchema"),
                "fn" => Some("http://www.w3.org/2005/xpath-functions"),
                "math" => Some("http://www.w3.org/2005/xpath-functions/math"),
                "map" => Some("http://www.w3.org/2005/xpath-functions/map"),
                "array" => Some("http://www.w3.org/2005/xpath-functions/array"),
                "err" => Some("http://www.w3.org/2005/xqt-errors"),
                "output" => Some("http://www.w3.org/2010/xslt-xquery-serialization"),
                _ => None,
            }
            .map(str::to_owned)
        })
    }

    fn default_namespace(&self, name_use: XPathNameUse) -> Option<String> {
        let static_context = match self.attachment {
            XPathAttachment::Host(host) => Some(&host.static_context),
            XPathAttachment::Standalone { .. } => None,
        };
        match name_use {
            XPathNameUse::Element => {
                static_context.and_then(|context| context.default_element_namespace.clone())
            }
            XPathNameUse::Function => static_context
                .and_then(|context| context.default_function_namespace.clone())
                .or_else(|| Some("http://www.w3.org/2005/xpath-functions".to_owned())),
            XPathNameUse::Attribute | XPathNameUse::Variable => None,
        }
    }

    fn range(&self, start: usize, end: usize) -> XPathSourceRange {
        XPathSourceRange::from_offsets(self.line_index, self.origin, start, end)
    }

    fn slice(&self, start: usize, end: usize) -> &str {
        self.source.get(start..end).unwrap_or_default()
    }
}

#[cfg(test)]
fn synthetic_wrapped_expression(path: &xee_ast::PathExpr) -> Option<&xee_ast::ExprSingleS> {
    let [step] = path.steps.as_slice() else {
        return None;
    };
    let xee_ast::StepExpr::PrimaryExpr(primary) = &step.value else {
        return None;
    };
    let xee_ast::PrimaryExpr::Expr(wrapped) = &primary.value else {
        return None;
    };
    let expression = wrapped.value.as_ref()?;
    let [inner] = expression.0.as_slice() else {
        return None;
    };
    (step.span == primary.span && primary.span == wrapped.span && inner.span == step.span)
        .then_some(inner)
}

#[cfg(test)]
fn xpath_parser_context(attachment: &XPathAttachment) -> XPathParserContext {
    let mut namespaces = Namespaces::default();
    if let XPathAttachment::Host(host) = attachment {
        if let Some(namespace) = &host.static_context.default_element_namespace {
            namespaces.default_element_namespace = namespace.clone();
        }
        if let Some(namespace) = &host.static_context.default_function_namespace {
            namespaces.default_function_namespace = namespace.clone();
        }
        let pairs = host
            .static_context
            .namespaces
            .iter()
            .map(|(prefix, namespace)| (prefix.as_str(), namespace.as_str()))
            .collect::<Vec<_>>();
        namespaces.add(&pairs);
    }
    XPathParserContext::new(namespaces, VariableNames::default())
}

fn xpath_syntax_to_cemt_subject(
    syntax: &XPathSyntaxAst,
    source_id: u32,
    content_type: &str,
) -> Value {
    json!({
        "kind": "xpath-syntax-ast",
        "grammarVersion": XPATH_GRAMMAR_VERSION,
        "rootKind": XPathSyntaxNodeKind::ExpressionSequence.as_str(),
        "sourceRange": syntax.root.source_range.to_cemt_subject(),
        "sourceMap": syntax.root.source_range.source_map(source_id, content_type),
        "events": syntax.events.iter().map(|event| json!({
            "index": event.index,
            "kind": event.kind.as_str(),
            "nodeKind": event.node_kind.as_str(),
            "depth": event.depth,
            "sourceRange": event.source_range.to_cemt_subject(),
            "sourceMap": event.source_range.source_map(source_id, content_type),
        })).collect::<Vec<_>>(),
    })
}

fn xpath_attachment_to_cemt_subject(attachment: &XPathAttachment) -> Value {
    match attachment {
        XPathAttachment::Standalone { source_id } => json!({
            "kind": "standalone",
            "sourceId": source_id,
        }),
        XPathAttachment::Host(host) => json!({
            "kind": "host",
            "owner": {
                "sourceId": host.owner.source_id,
                "sourceUri": host.owner.source_uri,
                "contentType": host.owner.content_type,
                "schema": host.owner.schema_uri,
                "nodeKind": host.owner.node_kind.as_str(),
                "nodeId": host.owner.node_id,
                "sourceRange": host.owner.source_range.to_cemt_subject(),
            },
            "expressionRange": host.expression_range.to_cemt_subject(),
            "staticContext": {
                "namespaces": host.static_context.namespaces,
                "defaultElementNamespace": host.static_context.default_element_namespace,
                "defaultFunctionNamespace": host.static_context.default_function_namespace,
                "variableBindings": host.static_context.variable_bindings,
                "functionBindings": host.static_context.function_bindings,
            },
            "expectedResult": host.expected_result.as_ref().map(|result| json!({
                "sequenceType": result.sequence_type,
                "minItems": result.min_items,
                "maxItems": result.max_items,
            })),
            "evaluationPhase": host.evaluation_phase.as_str(),
            "resolverPolicyStamp": host.resolver_policy_stamp,
            "safetyPolicyStamp": host.safety_policy_stamp,
        }),
    }
}

fn content_type_parameters(content_type: Option<&str>) -> BTreeMap<String, String> {
    let Some(content_type) = content_type else {
        return BTreeMap::new();
    };
    content_type
        .split(';')
        .skip(1)
        .filter_map(|part| {
            let (name, value) = part.split_once('=')?;
            Some((
                name.trim().to_ascii_lowercase(),
                value.trim().trim_matches('"').to_owned(),
            ))
        })
        .collect()
}

fn detect_line_ending_style_bytes(source: &[u8]) -> Option<&'static str> {
    let has_crlf = source.windows(2).any(|pair| pair == b"\r\n");
    let has_lone_cr = source
        .iter()
        .enumerate()
        .any(|(index, byte)| *byte == b'\r' && source.get(index + 1).copied() != Some(b'\n'));
    let has_lf = source.iter().enumerate().any(|(index, byte)| {
        *byte == b'\n'
            && index
                .checked_sub(1)
                .and_then(|previous| source.get(previous))
                != Some(&b'\r')
    });
    match (has_crlf, has_lf, has_lone_cr) {
        (false, false, false) => None,
        (true, false, false) => Some("crlf"),
        (false, true, false) => Some("lf"),
        _ => Some("mixed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{SourceProjectionPosition, SourceRangeProjector};

    #[derive(Debug)]
    struct ExpandingEntityProjector;

    impl SourceRangeProjector for ExpandingEntityProjector {
        fn project_boundary(&self, decoded_byte_offset: u64) -> Option<SourceProjectionPosition> {
            if decoded_byte_offset > 10 {
                return None;
            }
            let expansion = u64::from(decoded_byte_offset > 6) * 3;
            Some(SourceProjectionPosition {
                line: 3,
                column: 15 + u32::try_from(decoded_byte_offset + expansion).ok()?,
                byte_offset: 42 + decoded_byte_offset + expansion,
            })
        }
    }

    #[derive(Debug)]
    struct IncompleteProjector;

    impl SourceRangeProjector for IncompleteProjector {
        fn project_boundary(&self, decoded_byte_offset: u64) -> Option<SourceProjectionPosition> {
            (decoded_byte_offset != 6).then_some(SourceProjectionPosition {
                line: 1,
                column: u32::try_from(decoded_byte_offset).ok()?.saturating_add(1),
                byte_offset: decoded_byte_offset,
            })
        }
    }

    fn parse_cem_contract(source: &str) -> crate::parser::document::CemDocument {
        let source = crate::source::BytesSource::new(SourceId(91), source.as_bytes().to_vec());
        let tokenizer = crate::tokenizer::cem::CemTokenizer::from_source(source);
        let events = crate::events::cem::CemEventNormalizer::new(tokenizer);
        crate::parser::builder::CemAstBuilder::new(events).build()
    }

    fn contract_element_ids(
        document: &crate::parser::document::CemDocument,
        local_name: &str,
    ) -> Vec<crate::parser::AstNodeId> {
        document
            .iter()
            .filter_map(|node| match node {
                crate::parser::CemAstNode::Element {
                    node_id,
                    expanded_name,
                    ..
                } if expanded_name.local_name == local_name => Some(*node_id),
                _ => None,
            })
            .collect()
    }

    fn contract_attributes(
        document: &crate::parser::document::CemDocument,
        node_id: crate::parser::AstNodeId,
    ) -> BTreeMap<String, String> {
        let Some(crate::parser::CemAstNode::Element { attributes, .. }) = document.get(node_id)
        else {
            return BTreeMap::new();
        };
        attributes
            .iter()
            .filter_map(|attribute_id| match document.get(*attribute_id) {
                Some(crate::parser::CemAstNode::Attribute {
                    expanded_name,
                    value,
                    ..
                }) => value
                    .as_ref()
                    .map(|value| (expanded_name.local_name.clone(), value.clone())),
                _ => None,
            })
            .collect()
    }

    fn result_source_map(source_id: u32, byte_offset: u64) -> SourceMapStack {
        SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(source_id),
                span: FrameSpan::Single(ByteRange::new(byte_offset, 1)),
                transform: TransformKind::Query,
            }],
        }
    }

    fn parse(source: &str) -> XPathExpressionAst {
        xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: source.as_bytes(),
                source_uri: "memory://expression.xpath",
                content_type: Some(XPATH_CONTENT_TYPE),
                source_range_projector: None,
            },
            XPathAttachment::Standalone { source_id: 7 },
        )
    }

    fn evaluate_for_test(
        source: &str,
        context_item: Option<XPathResultItem>,
        variable_bindings: XPathVariableBindings,
    ) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
        let expression = parse(source);
        let resolver_registry = ResolverRegistry::new();
        let resolver_policy = ResolverPolicy::new();
        CemXPathEvaluator::default().evaluate(XPathEvaluationRequest {
            invocation_host: XPathInvocationHost::StandaloneTransform,
            expression: &expression,
            dynamic_context: XPathDynamicContext {
                context_item,
                variable_bindings,
            },
            static_context: XPathStaticContext::default(),
            expected_result: None,
            resolver_registry: &resolver_registry,
            resolver_policy: &resolver_policy,
            safety_policy_stamp: "xpath-safety/1;pure",
        })
    }

    fn atomic_test_item(type_name: &str, lexical_value: &str) -> XPathResultItem {
        XPathResultItem::Atomic {
            value: XPathAtomicValue {
                type_name: type_name.to_owned(),
                lexical_value: lexical_value.to_owned(),
                namespace_uri: None,
                local_name: None,
            },
            source_map: result_source_map(9, 1),
        }
    }

    fn singleton_test_binding(type_name: &str, lexical_value: &str) -> XPathResultSequence {
        XPathResultSequence {
            sequence_type: type_name.to_owned(),
            items: vec![atomic_test_item(type_name, lexical_value)],
        }
    }

    fn parsed_syntax(source: &str) -> XPathSyntaxAst {
        parse(source)
            .syntax_ast
            .unwrap_or_else(|| panic!("expected parsed XPath syntax for `{source}`"))
    }

    #[test]
    fn xpath_single_parse_projects_every_native_ast_range_to_original_source() {
        let source = "price < 10";
        let projector = ExpandingEntityProjector;
        let expression_range = XPathSourceRange::new(3, 15, 42, 13);
        let ast = xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: source.as_bytes(),
                source_uri: "memory://stylesheet.xsl",
                content_type: Some(XPATH_CONTENT_TYPE),
                source_range_projector: Some(&projector),
            },
            XPathAttachment::Host(XPathHostAttachment {
                owner: XPathHostOwner {
                    source_id: 11,
                    source_uri: "memory://stylesheet.xsl".to_owned(),
                    content_type: Some("application/xslt+xml".to_owned()),
                    schema_uri: Some("https://cem.dev/ns/transform/xslt/1".to_owned()),
                    node_kind: XPathHostNodeKind::XsltAttribute,
                    node_id: Some("event:4@select".to_owned()),
                    source_range: XPathSourceRange::new(3, 7, 34, 30),
                },
                expression_range,
                static_context: XPathStaticContext::default(),
                expected_result: None,
                evaluation_phase: XPathEvaluationPhase::Transform,
                resolver_policy_stamp: None,
                safety_policy_stamp: None,
            }),
        );

        let less_than = ast
            .tokens
            .iter()
            .find(|token| token.lexeme == "<")
            .expect("projected less-than token");
        assert_eq!(less_than.source_range, XPathSourceRange::new(3, 21, 48, 4));
        assert_eq!(
            ast.tokens
                .iter()
                .find(|token| token.lexeme == " " && token.source_range.start.byte_offset > 48)
                .expect("whitespace after expanded entity")
                .source_range
                .start
                .byte_offset,
            52
        );
        assert_eq!(
            ast.syntax_ast
                .as_ref()
                .expect("projected syntax AST")
                .root
                .source_range,
            expression_range
        );
        assert_eq!(
            ast.events.first().unwrap().source_range.start.byte_offset,
            42
        );
        assert_eq!(
            ast.events.last().unwrap().source_range.start.byte_offset,
            55
        );
        assert!(ast.facts.iter().any(|fact| {
            fact.kind == XPathFactKind::HostAssociationObserved
                && fact.source_range == Some(expression_range)
        }));
        assert!(!ast
            .facts
            .iter()
            .any(|fact| fact.kind == XPathFactKind::SourceMapUnavailable));
    }

    #[test]
    fn xpath_incomplete_projector_fails_closed_before_scanning() {
        let source = "price < 10";
        let projector = IncompleteProjector;
        let ast = xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: source.as_bytes(),
                source_uri: "memory://projected.xpath",
                content_type: Some(XPATH_CONTENT_TYPE),
                source_range_projector: Some(&projector),
            },
            XPathAttachment::Standalone { source_id: 7 },
        );

        assert_eq!(ast.source_text.as_deref(), Some(source));
        assert!(ast.tokens.is_empty());
        assert!(ast.events.is_empty());
        assert!(ast.syntax_ast.is_none());
        assert_eq!(ast.facts.len(), 1);
        assert_eq!(ast.facts[0].kind, XPathFactKind::SourceMapUnavailable);
        assert_eq!(ast.facts[0].source_range, None);
    }

    fn xee_lexical_projection(source: &str) -> Vec<(XPathTokenKind, String)> {
        xee_xpath_lexer::lexer(source)
            .map(|(token, span)| (xpath_token_kind(&token), source[span].to_owned()))
            .collect()
    }

    fn cem_lexical_projection(source: &str) -> Vec<(XPathTokenKind, String)> {
        lexer::xpath_lexical_tokens(source)
            .into_iter()
            .filter_map(|token| {
                let kind = token.kind.presentation_kind();
                (!matches!(kind, XPathTokenKind::Comment | XPathTokenKind::Whitespace))
                    .then(|| (kind, token.lexeme.to_owned()))
            })
            .collect()
    }

    fn cem_parser_syntax(source: &str) -> Result<XPathSyntaxAst, parser::XPathParseError> {
        let line_index = LineIndex::from_utf8(source);
        let attachment = XPathAttachment::Standalone { source_id: 7 };
        let tokens = lexer::xpath_lexical_tokens(source);
        let range_resolver = XPathSourceRangeResolver::new(
            source,
            &line_index,
            XPathSourcePosition {
                line: 1,
                column: 1,
                byte_offset: 0,
            },
            None,
        )
        .expect("standalone XPath range resolver");
        parser::parse_xpath(source, &tokens, &range_resolver, &attachment)
    }

    fn xee_parser_syntax(source: &str) -> XPathSyntaxAst {
        let line_index = LineIndex::from_utf8(source);
        let attachment = XPathAttachment::Standalone { source_id: 7 };
        let parsed = xpath_parser_context(&attachment)
            .parse_xpath(source)
            .unwrap_or_else(|error| panic!("Xee oracle failed for `{source}`: {error:?}"));
        XPathSyntaxLowerer::new(
            source,
            &line_index,
            XPathSourcePosition {
                line: 1,
                column: 1,
                byte_offset: 0,
            },
            &attachment,
        )
        .lower(&parsed)
    }

    #[test]
    fn xpath_syntax_ast_lowers_paths_predicates_names_and_ranges_to_cem_types() {
        let source = "/catalog/book[@lang = \"en\"]/title";
        let syntax = parsed_syntax(source);

        assert_eq!(syntax.root.expressions.len(), 1);
        assert_eq!(
            syntax.root.source_range,
            XPathSourceRange::new(1, 1, 0, source.len() as u64)
        );
        let XPathExpression::Path(path) = &syntax.root.expressions[0].expression else {
            panic!("expected a typed path expression");
        };
        assert_eq!(path.root, XPathPathRoot::Rooted);
        let book_step = path
            .steps
            .iter()
            .find_map(|step| match &step.step {
                XPathStep::Axis {
                    node_test: XPathNodeTest::Name(XPathNameTest::Name(name)),
                    predicates,
                    ..
                } if name.local_name == "book" => Some((name, predicates)),
                _ => None,
            })
            .expect("book axis step");
        assert_eq!(book_step.0.lexical, "book");
        assert_eq!(book_step.0.namespace_uri, None);
        assert_eq!(book_step.1.len(), 1);
        assert!(matches!(
            book_step.1[0].expressions[0].expression,
            XPathExpression::Binary {
                operator: XPathBinaryOperator::GeneralEqual,
                ..
            }
        ));
    }

    #[test]
    fn xpath_cem_parser_matches_xee_lowered_ast_for_passing_package_examples() {
        for source in [
            include_str!("../../schema-packages/xpath/v1/examples/basic-path.xpath"),
            include_str!("../../schema-packages/xpath/v1/examples/functions-and-variables.xpath"),
            include_str!("../../schema-packages/xpath/v1/examples/maps-arrays-and-comments.xpath"),
            include_str!("../../schema-packages/xpath/v1/examples/unicode-qname.xpath"),
            include_str!(
                "../../schema-packages/xpath/v1/examples/explicit-axes-and-escaped-string.xpath"
            ),
            include_str!("../../schema-packages/xpath/v1/examples/external-resource-denied.xpath"),
        ] {
            assert_eq!(
                cem_parser_syntax(source).expect("CEM parser must accept package example"),
                xee_parser_syntax(source),
                "CEM parser AST diverged from the pinned parser oracle for `{source}`"
            );
        }
    }

    #[test]
    fn xpath_cem_parser_applies_precedence_and_preserves_real_parentheses() {
        let syntax = cem_parser_syntax("1 + 2 * 3").expect("precedence expression");
        let XPathExpression::Binary {
            operator: XPathBinaryOperator::Add,
            right,
            ..
        } = &syntax.root.expressions[0].expression
        else {
            panic!("expected additive root");
        };
        assert!(matches!(
            right.expression,
            XPathExpression::Binary {
                operator: XPathBinaryOperator::Multiply,
                ..
            }
        ));

        let parenthesized = cem_parser_syntax("(1 + 2) * 3").expect("parenthesized precedence");
        let XPathExpression::Binary {
            operator: XPathBinaryOperator::Multiply,
            left,
            ..
        } = &parenthesized.root.expressions[0].expression
        else {
            panic!("expected multiplicative root");
        };
        let XPathExpression::Path(path) = &left.expression else {
            panic!("expected parenthesized primary path");
        };
        assert!(matches!(
            path.steps[0].step,
            XPathStep::Primary(XPathPrimaryExpression::Parenthesized(Some(_)))
        ));

        let associative = cem_parser_syntax("10 - 3 - 2").expect("left-associative expression");
        let XPathExpression::Binary {
            operator: XPathBinaryOperator::Subtract,
            left,
            ..
        } = &associative.root.expressions[0].expression
        else {
            panic!("expected subtractive root");
        };
        assert!(matches!(
            left.expression,
            XPathExpression::Binary {
                operator: XPathBinaryOperator::Subtract,
                ..
            }
        ));
    }

    #[test]
    fn xpath_cem_parser_lowers_eqname_and_wildcard_name_tests() {
        let source = "/Q{urn:catalog}catalog/*:book/Q{urn:app}*";
        let syntax = cem_parser_syntax(source).expect("EQName and wildcard path");
        assert_eq!(syntax, xee_parser_syntax(source));

        let XPathExpression::Path(path) = &syntax.root.expressions[0].expression else {
            panic!("expected path");
        };
        assert!(matches!(
            path.steps[0].step,
            XPathStep::Axis {
                node_test: XPathNodeTest::Name(XPathNameTest::Name(XPathName {
                    ref namespace_uri,
                    ref local_name,
                    ..
                })),
                ..
            } if namespace_uri.as_deref() == Some("urn:catalog") && local_name == "catalog"
        ));
        assert!(matches!(
            path.steps[1].step,
            XPathStep::Axis {
                node_test: XPathNodeTest::Name(XPathNameTest::AnyNamespace {
                    ref local_name,
                }),
                ..
            } if local_name == "book"
        ));
        assert!(matches!(
            path.steps[2].step,
            XPathStep::Axis {
                node_test: XPathNodeTest::Name(XPathNameTest::Namespace {
                    ref namespace_uri,
                }),
                ..
            } if namespace_uri == "urn:app"
        ));
    }

    #[test]
    fn xpath_cem_parser_distinguishes_roots_sequences_and_argument_lists() {
        for (source, expected_root) in [
            ("a/b", XPathPathRoot::Relative),
            ("/a", XPathPathRoot::Rooted),
            ("//a", XPathPathRoot::RootedDescendant),
        ] {
            let syntax = cem_parser_syntax(source).expect("path expression");
            let XPathExpression::Path(path) = &syntax.root.expressions[0].expression else {
                panic!("expected path for `{source}`");
            };
            assert_eq!(path.root, expected_root);
        }

        let syntax = cem_parser_syntax("concat((1, 2), 3)").expect("function arguments");
        assert_eq!(syntax.root.expressions.len(), 1);
        let XPathExpression::Path(path) = &syntax.root.expressions[0].expression else {
            panic!("expected function path");
        };
        let XPathStep::Primary(XPathPrimaryExpression::FunctionCall { arguments, .. }) =
            &path.steps[0].step
        else {
            panic!("expected function call");
        };
        assert_eq!(arguments.len(), 2);
        let XPathExpression::Path(first_argument) = &arguments[0].expression else {
            panic!("expected parenthesized first argument");
        };
        let XPathStep::Primary(XPathPrimaryExpression::Parenthesized(Some(sequence))) =
            &first_argument.steps[0].step
        else {
            panic!("expected parenthesized sequence");
        };
        assert_eq!(sequence.expressions.len(), 2);
    }

    #[test]
    fn xpath_cem_parser_returns_typed_expected_found_and_namespace_errors() {
        let unclosed = cem_parser_syntax("/book[").expect_err("unclosed predicate must fail");
        assert_eq!(unclosed.kind, parser::XPathParseErrorKind::Syntax);
        assert!(unclosed.expected.iter().any(|expected| expected == "]"));
        assert_eq!(unclosed.found, None);
        assert_eq!(unclosed.start, 6);
        assert_eq!(unclosed.end, 6);

        let mismatched = cem_parser_syntax("/book[1)").expect_err("mismatched predicate must fail");
        assert_eq!(mismatched.kind, parser::XPathParseErrorKind::Syntax);
        assert!(mismatched.expected.iter().any(|expected| expected == "]"));
        assert_eq!(mismatched.found.as_deref(), Some(")"));
        assert_eq!((mismatched.start, mismatched.end), (7, 8));

        let unknown = cem_parser_syntax("/catalog/ns:book").expect_err("unknown prefix must fail");
        assert_eq!(
            unknown.kind,
            parser::XPathParseErrorKind::UnknownNamespacePrefix
        );
        assert_eq!(unknown.namespace_prefix.as_deref(), Some("ns"));
        assert_eq!((unknown.start, unknown.end), (9, 16));
    }

    #[test]
    fn xpath_cem_parser_retains_unmodeled_xpath_as_typed_ranged_nodes() {
        let source = "let $x := 1 return $x";
        let syntax = cem_parser_syntax(source).expect("recognized unsupported production");
        assert!(matches!(
            syntax.root.expressions[0].expression,
            XPathExpression::Unsupported { ref production }
                if production == "let-expression"
        ));
        assert_eq!(
            syntax.root.expressions[0].source_range,
            XPathSourceRange::new(1, 1, 0, source.len() as u64)
        );

        let inline = "function($x) { $x }";
        let syntax = cem_parser_syntax(inline).expect("recognized inline-function production");
        assert_eq!(syntax, xee_parser_syntax(inline));
        let XPathExpression::Path(path) = &syntax.root.expressions[0].expression else {
            panic!("expected inline function primary path");
        };
        assert!(matches!(
            path.steps[0].step,
            XPathStep::Primary(XPathPrimaryExpression::Unsupported { ref production })
                if production == "inline-function-expression"
        ));
        assert_eq!(
            path.source_range,
            XPathSourceRange::new(1, 1, 0, inline.len() as u64)
        );
    }

    #[test]
    fn xpath_syntax_ast_lowers_for_variables_and_function_calls() {
        let source = "for $book in /catalog/book return normalize-space($book/title)";
        let syntax = parsed_syntax(source);
        let XPathExpression::For {
            binding,
            binding_expression,
            return_expression,
        } = &syntax.root.expressions[0].expression
        else {
            panic!("expected a typed for expression");
        };

        assert_eq!(binding.lexical, "book");
        assert_eq!(binding.local_name, "book");
        assert!(matches!(
            binding_expression.expression,
            XPathExpression::Path(_)
        ));
        let XPathExpression::Path(return_path) = &return_expression.expression else {
            panic!("expected for return path");
        };
        let function = return_path.steps.iter().find_map(|step| match &step.step {
            XPathStep::Primary(XPathPrimaryExpression::FunctionCall { name, arguments })
            | XPathStep::Postfix {
                primary: XPathPrimaryExpression::FunctionCall { name, arguments },
                ..
            } => Some((name, arguments)),
            _ => None,
        });
        let (name, arguments) = function.expect("normalize-space function call");
        assert_eq!(name.lexical, "normalize-space");
        assert_eq!(
            name.namespace_uri.as_deref(),
            Some("http://www.w3.org/2005/xpath-functions")
        );
        assert_eq!(arguments.len(), 1);
    }

    #[test]
    fn xpath_syntax_ast_lowers_map_and_array_constructors() {
        let source = "map { \"titles\": array { /catalog/book/title/string() }, \"count\": count(/catalog/book) }";
        let syntax = parsed_syntax(source);
        let XPathExpression::Path(path) = &syntax.root.expressions[0].expression else {
            panic!("expected constructor path wrapper");
        };
        let entries = path.steps.iter().find_map(|step| match &step.step {
            XPathStep::Primary(XPathPrimaryExpression::MapConstructor { entries })
            | XPathStep::Postfix {
                primary: XPathPrimaryExpression::MapConstructor { entries },
                ..
            } => Some(entries),
            _ => None,
        });
        let entries = entries.expect("map constructor");
        assert_eq!(entries.len(), 2);
        let XPathExpression::Path(value_path) = &entries[0].value.expression else {
            panic!("expected array value path");
        };
        assert!(value_path.steps.iter().any(|step| matches!(
            step.step,
            XPathStep::Primary(XPathPrimaryExpression::ArrayConstructor(_))
                | XPathStep::Postfix {
                    primary: XPathPrimaryExpression::ArrayConstructor(_),
                    ..
                }
        )));
    }

    #[test]
    fn xpath_syntax_events_are_balanced_and_cemt_projection_is_explicit() {
        let ast = parse("/catalog/book[@lang = \"en\"]/title");
        let syntax = ast.syntax_ast.as_ref().expect("typed syntax AST");
        let mut stack = Vec::new();
        for event in &syntax.events {
            match event.kind {
                XPathSyntaxEventKind::StartNode => stack.push(event.node_kind),
                XPathSyntaxEventKind::EndNode => {
                    assert_eq!(stack.pop(), Some(event.node_kind));
                }
            }
        }
        assert!(stack.is_empty());
        assert_eq!(
            syntax.events.first().map(|event| event.node_kind),
            Some(XPathSyntaxNodeKind::ExpressionSequence)
        );
        assert_eq!(
            syntax.events.last().map(|event| event.node_kind),
            Some(XPathSyntaxNodeKind::ExpressionSequence)
        );

        let subject = ast.to_cemt_subject();
        assert_eq!(subject["grammarVersion"], XPATH_GRAMMAR_VERSION);
        assert_eq!(subject["syntaxAst"]["kind"], "xpath-syntax-ast");
        assert_eq!(subject["syntaxAst"]["events"][0]["kind"], "start-node");
        assert!(subject["syntaxAst"].get("root").is_none());
    }

    #[test]
    fn xpath_result_artifact_preserves_ordered_mixed_sequences_and_item_origins() {
        let capabilities = XPathEvaluatorCapabilities::required("test.xpath", "1.0.0");
        let sequence = XPathResultSequence {
            sequence_type: "item()*".to_owned(),
            items: vec![
                XPathResultItem::Node {
                    node_kind: XPathResultNodeKind::Element,
                    source_id: 9,
                    source_uri: "memory://catalog.xml".to_owned(),
                    node_id: "node:12".to_owned(),
                    expanded_name: Some("{urn:catalog}book".to_owned()),
                    source_range: Some(XPathSourceRange::new(2, 3, 18, 24)),
                    source_map: result_source_map(9, 18),
                    native_node: None,
                },
                XPathResultItem::Atomic {
                    value: XPathAtomicValue {
                        type_name: "xs:decimal".to_owned(),
                        lexical_value: "42.50".to_owned(),
                        namespace_uri: None,
                        local_name: None,
                    },
                    source_map: result_source_map(7, 4),
                },
                XPathResultItem::Map {
                    entries: vec![XPathMapEntry {
                        key: XPathAtomicValue {
                            type_name: "xs:string".to_owned(),
                            lexical_value: "title".to_owned(),
                            namespace_uri: None,
                            local_name: None,
                        },
                        value: XPathResultSequence {
                            sequence_type: "xs:string".to_owned(),
                            items: vec![XPathResultItem::Atomic {
                                value: XPathAtomicValue {
                                    type_name: "xs:string".to_owned(),
                                    lexical_value: "CEM".to_owned(),
                                    namespace_uri: None,
                                    local_name: None,
                                },
                                source_map: result_source_map(7, 8),
                            }],
                        },
                    }],
                    source_map: result_source_map(7, 6),
                },
                XPathResultItem::Array {
                    members: vec![XPathResultSequence {
                        sequence_type: "xs:boolean".to_owned(),
                        items: vec![XPathResultItem::Atomic {
                            value: XPathAtomicValue {
                                type_name: "xs:boolean".to_owned(),
                                lexical_value: "true".to_owned(),
                                namespace_uri: None,
                                local_name: None,
                            },
                            source_map: result_source_map(7, 10),
                        }],
                    }],
                    source_map: result_source_map(7, 9),
                },
                XPathResultItem::Function {
                    evaluator_id: "test.xpath".to_owned(),
                    function_id: "function:5".to_owned(),
                    name: Some("fn:string".to_owned()),
                    arity: 1,
                    signature: "function(item()?) as xs:string".to_owned(),
                    source_map: result_source_map(7, 12),
                },
            ],
        };
        let artifact = XPathResultArtifact {
            content_type: XPATH_RESULT_CONTENT_TYPE.to_owned(),
            schema_uri: XPATH_SCHEMA_URI.to_owned(),
            xpath_version: "3.1".to_owned(),
            grammar_version: XPATH_GRAMMAR_VERSION.to_owned(),
            invocation_host: XPathInvocationHost::StandaloneTransform,
            evaluator: XPathEvaluatorIdentity {
                evaluator_id: "test.xpath".to_owned(),
                evaluator_version: "1.0.0".to_owned(),
            },
            expression_uri: "memory://query.xpath".to_owned(),
            static_context: XPathStaticContext::default(),
            resolver_policy_stamp: "resolver-policy/1;test".to_owned(),
            safety_policy_stamp: "xpath-safety/1;pure".to_owned(),
            expected_result: Some(XPathExpectedResult {
                sequence_type: "item()*".to_owned(),
                min_items: Some(0),
                max_items: None,
            }),
            sequence,
            source_map: result_source_map(7, 0),
        };

        assert!(
            validate_xpath_result_artifact(&artifact, &capabilities).is_empty(),
            "valid mixed sequence must satisfy the package contract"
        );
        let value = serde_json::to_value(&artifact).expect("result artifact serializes");
        assert_eq!(value["contentType"], XPATH_RESULT_CONTENT_TYPE);
        assert_eq!(value["invocationHost"], "standalone-transform");
        assert_eq!(value["evaluator"]["evaluatorId"], "test.xpath");
        assert_eq!(value["evaluator"]["evaluatorVersion"], "1.0.0");
        assert_eq!(value["sequence"]["items"][0]["kind"], "node");
        assert_eq!(value["sequence"]["items"][1]["kind"], "atomic");
        assert_eq!(value["sequence"]["items"][2]["kind"], "map");
        assert_eq!(value["sequence"]["items"][3]["kind"], "array");
        assert_eq!(value["sequence"]["items"][4]["kind"], "function");
        assert_eq!(
            value["sequence"]["items"][0]["sourceMap"]["frames"][0]["source_id"],
            9
        );
        assert_eq!(value["sequence"]["items"][4]["functionId"], "function:5");
        assert!(value["sequence"]["items"][4].get("closure").is_none());

        let mut invalid = artifact.clone();
        if let XPathResultItem::Atomic { source_map, .. } = &mut invalid.sequence.items[1] {
            source_map.frames.clear();
        }
        if let XPathResultItem::Function { evaluator_id, .. } = &mut invalid.sequence.items[4] {
            *evaluator_id = "other.xpath".to_owned();
        }
        let violations = validate_xpath_result_artifact(&invalid, &capabilities);
        assert!(violations
            .iter()
            .any(|violation| violation.code == "xpath-result-source-map-required"));
        assert!(violations
            .iter()
            .any(|violation| violation.code == "xpath-result-function-scope-invalid"));
    }

    #[test]
    fn xpath_evaluator_capabilities_forbid_reparse_resolver_bypass_and_missing_wasm() {
        let required = XPathEvaluatorCapabilities::required("test.xpath", "1.0.0");
        assert!(validate_xpath_evaluator_capabilities(&required).is_empty());

        let mut incompatible = required;
        incompatible.ast_input = XPathEvaluatorAstInput::SourceText;
        incompatible.resource_access = XPathEvaluatorResourceAccess::Direct;
        incompatible.targets.remove("wasm32-unknown-unknown");
        let violations = validate_xpath_evaluator_capabilities(&incompatible);

        for code in [
            "xpath-evaluator-ast-reparse-forbidden",
            "xpath-evaluator-resolver-bypass",
            "xpath-evaluator-target-missing",
        ] {
            assert!(
                violations.iter().any(|violation| violation.code == code),
                "missing capability violation {code}: {violations:?}"
            );
        }
    }

    #[test]
    fn xpath_evaluation_request_carries_package_ast_and_cem_resolver_boundary() {
        let expression = parse("/catalog/book");
        let resolver_registry = ResolverRegistry::new();
        let resolver_policy = ResolverPolicy::new();
        let request = XPathEvaluationRequest {
            invocation_host: XPathInvocationHost::StandaloneTransform,
            expression: &expression,
            dynamic_context: XPathDynamicContext::default(),
            static_context: XPathStaticContext::default(),
            expected_result: None,
            resolver_registry: &resolver_registry,
            resolver_policy: &resolver_policy,
            safety_policy_stamp: "xpath-safety/1;pure",
        };

        assert!(request.expression.syntax_ast.is_some());
        assert_eq!(
            request.resolver_policy.cache_stamp(),
            resolver_policy.cache_stamp()
        );
        assert!(std::ptr::eq(request.resolver_registry, &resolver_registry));
    }

    #[test]
    fn xpath_cemt_invocation_uses_owned_ast_native_context_and_expanded_qname_bindings() {
        use crate::lifecycle::LoadedInputAstStream;
        use crate::validation::xml::{
            xml_document_ast_from_source_bytes, XmlSourceValidationRequest,
        };
        use std::sync::Arc;

        let source = br#"<root><n>2</n><n>10</n></root>"#;
        let (document, diagnostics) =
            xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
                bytes: source,
                source_uri: "memory://cemt-context.xml",
                content_type: Some("application/xml"),
            });
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let owner = Arc::new(LoadedInputAstStream::XmlDocument(
            document.expect("typed XML document"),
        ));
        let context_item = XPathResultItem::from_native_node(
            XPathNativeNode::xml_document(Arc::clone(&owner)).expect("native document node"),
        );

        let expression_source = "$vars:limit = /root/n, /root/n[$vars:index]";
        let expression_range = XPathSourceRange::new(4, 12, 96, expression_source.len() as u64);
        let static_context = XPathStaticContext {
            namespaces: BTreeMap::from([("vars".to_owned(), "urn:cem:variables".to_owned())]),
            variable_bindings: BTreeMap::from([
                ("vars:limit".to_owned(), "xs:integer".to_owned()),
                ("vars:index".to_owned(), "xs:integer".to_owned()),
            ]),
            ..XPathStaticContext::default()
        };
        let expression = xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: expression_source.as_bytes(),
                source_uri: "memory://formatter.cemt",
                content_type: Some(XPATH_CONTENT_TYPE),
                source_range_projector: None,
            },
            XPathAttachment::Host(XPathHostAttachment {
                owner: XPathHostOwner {
                    source_id: 41,
                    source_uri: "memory://formatter.cemt".to_owned(),
                    content_type: Some("application/vnd.cem.transform+cem".to_owned()),
                    schema_uri: Some("https://cem.dev/ns/transform/cem/1".to_owned()),
                    node_kind: XPathHostNodeKind::CemtExpressionSlot,
                    node_id: Some("function:catalog@select".to_owned()),
                    source_range: XPathSourceRange::new(4, 1, 80, 64),
                },
                expression_range,
                static_context: static_context.clone(),
                expected_result: None,
                evaluation_phase: XPathEvaluationPhase::Render,
                resolver_policy_stamp: Some("resolver:none".to_owned()),
                safety_policy_stamp: Some("xpath:pure".to_owned()),
            }),
        );
        assert!(expression.syntax_ast.is_some(), "{:?}", expression.facts);

        let variable_bindings = BTreeMap::from([
            (
                XPathExpandedName::new(Some("urn:cem:variables"), "limit"),
                singleton_test_binding("xs:integer", "2"),
            ),
            (
                XPathExpandedName::new(Some("urn:cem:variables"), "index"),
                singleton_test_binding("xs:integer", "2"),
            ),
        ]);
        let resolver_registry = ResolverRegistry::new();
        let resolver_policy = ResolverPolicy::new();
        let result = CemtXPathInvocationAdapter::default()
            .invoke(XPathEvaluationRequest {
                invocation_host: XPathInvocationHost::Cemt,
                expression: &expression,
                dynamic_context: XPathDynamicContext {
                    context_item: Some(context_item),
                    variable_bindings,
                },
                static_context,
                expected_result: None,
                resolver_registry: &resolver_registry,
                resolver_policy: &resolver_policy,
                safety_policy_stamp: "xpath-safety/1;cemt-render",
            })
            .expect("CEMT invokes the native evaluator over typed bindings");

        let [XPathResultItem::Atomic { value, .. }, XPathResultItem::Node { native_node, .. }] =
            result.sequence.items.as_slice()
        else {
            panic!("expected boolean and native node result: {result:?}");
        };
        assert_eq!(value.type_name, "xs:boolean");
        assert_eq!(value.lexical_value, "true");
        assert_eq!(result.invocation_host, XPathInvocationHost::Cemt);
        assert!(Arc::ptr_eq(
            native_node.as_ref().expect("native result node").owner(),
            &owner
        ));
    }

    #[test]
    fn xpath_cemt_invocation_rejects_non_cemt_owners_and_lexical_binding_fallbacks() {
        let expression = parse("$vars:item");
        let resolver_registry = ResolverRegistry::new();
        let resolver_policy = ResolverPolicy::new();
        let diagnostics = CemtXPathInvocationAdapter::default()
            .invoke(XPathEvaluationRequest {
                invocation_host: XPathInvocationHost::Cemt,
                expression: &expression,
                dynamic_context: XPathDynamicContext {
                    context_item: None,
                    variable_bindings: BTreeMap::from([(
                        XPathExpandedName::unqualified("vars:item"),
                        singleton_test_binding("xs:string", "not-an-expanded-name"),
                    )]),
                },
                static_context: XPathStaticContext::default(),
                expected_result: None,
                resolver_registry: &resolver_registry,
                resolver_policy: &resolver_policy,
                safety_policy_stamp: "xpath-safety/1;cemt-render",
            })
            .expect_err("a CEMT adapter only accepts CEMT-owned XPath AST slots");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].code, "cem.xpath.invocation_host_mismatch");

        let variable_source = "$vars:item";
        let static_context = XPathStaticContext {
            namespaces: BTreeMap::from([("vars".to_owned(), "urn:cem:variables".to_owned())]),
            variable_bindings: BTreeMap::from([("vars:item".to_owned(), "xs:string".to_owned())]),
            ..XPathStaticContext::default()
        };
        let expression = xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: variable_source.as_bytes(),
                source_uri: "memory://binding.cemt",
                content_type: Some(XPATH_CONTENT_TYPE),
                source_range_projector: None,
            },
            XPathAttachment::Host(XPathHostAttachment {
                owner: XPathHostOwner {
                    source_id: 42,
                    source_uri: "memory://binding.cemt".to_owned(),
                    content_type: Some("application/vnd.cem.transform+cem".to_owned()),
                    schema_uri: Some("https://cem.dev/ns/transform/cem/1".to_owned()),
                    node_kind: XPathHostNodeKind::CemtExpressionSlot,
                    node_id: Some("function:binding@select".to_owned()),
                    source_range: XPathSourceRange::new(1, 1, 0, variable_source.len() as u64),
                },
                expression_range: XPathSourceRange::new(1, 1, 0, variable_source.len() as u64),
                static_context: static_context.clone(),
                expected_result: None,
                evaluation_phase: XPathEvaluationPhase::Render,
                resolver_policy_stamp: Some("resolver:none".to_owned()),
                safety_policy_stamp: Some("xpath:pure".to_owned()),
            }),
        );
        let diagnostics = CemtXPathInvocationAdapter::default()
            .invoke(XPathEvaluationRequest {
                invocation_host: XPathInvocationHost::Cemt,
                expression: &expression,
                dynamic_context: XPathDynamicContext {
                    context_item: None,
                    variable_bindings: BTreeMap::from([(
                        XPathExpandedName::unqualified("vars:item"),
                        singleton_test_binding("xs:string", "not-an-expanded-name"),
                    )]),
                },
                static_context,
                expected_result: None,
                resolver_registry: &resolver_registry,
                resolver_policy: &resolver_policy,
                safety_policy_stamp: "xpath-safety/1;cemt-render",
            })
            .expect_err("lexical binding keys do not alias expanded QNames");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].code, "cem.xpath.variable_unbound");

        let source = include_str!("xpath.rs");
        let adapter = source
            .split_once("pub struct CemtXPathInvocationAdapter")
            .expect("CEMT XPath adapter source boundary")
            .1
            .split_once("struct XPathEvaluationError")
            .expect("CEMT XPath adapter source boundary")
            .0;
        for forbidden in [
            "serde_json",
            "source_text",
            "xpath_expression_ast_from_source_bytes",
            "xml_document_ast_from_source_bytes",
        ] {
            assert!(
                !adapter.contains(forbidden),
                "CEMT XPath invocation must not cross `{forbidden}`"
            );
        }
    }

    #[test]
    fn xpath_native_evaluator_retains_lifecycle_xml_owner_node_identity_and_source_map() {
        use crate::lifecycle::LoadedInputAstStream;
        use crate::validation::xml::{
            xml_document_ast_from_source_bytes, XmlSourceValidationRequest,
        };
        use std::sync::Arc;

        let source = br#"<catalog><book id="a">Alpha</book><book id="b">Beta</book></catalog>"#;
        let (document, diagnostics) =
            xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
                bytes: source,
                source_uri: "memory://catalog.xml",
                content_type: Some("application/xml"),
            });
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let owner = Arc::new(LoadedInputAstStream::XmlDocument(
            document.expect("typed XML document"),
        ));
        let context_node =
            XPathNativeNode::xml_document(Arc::clone(&owner)).expect("native XML document node");
        let expression = parse("/catalog/book");
        let resolver_registry = ResolverRegistry::new();
        let resolver_policy = ResolverPolicy::new();
        let evaluator = CemXPathEvaluator::default();

        let result = evaluator
            .evaluate(XPathEvaluationRequest {
                invocation_host: XPathInvocationHost::StandaloneTransform,
                expression: &expression,
                dynamic_context: XPathDynamicContext {
                    context_item: Some(XPathResultItem::from_native_node(context_node)),
                    variable_bindings: BTreeMap::new(),
                },
                static_context: XPathStaticContext::default(),
                expected_result: None,
                resolver_registry: &resolver_registry,
                resolver_policy: &resolver_policy,
                safety_policy_stamp: "xpath-safety/1;pure",
            })
            .expect("native path evaluation");

        assert_eq!(result.sequence.items.len(), 2);
        for item in &result.sequence.items {
            let XPathResultItem::Node {
                node_kind,
                expanded_name,
                source_map,
                native_node,
                ..
            } = item
            else {
                panic!("expected native XML node result: {item:?}");
            };
            assert_eq!(*node_kind, XPathResultNodeKind::Element);
            assert_eq!(expanded_name.as_deref(), Some("book"));
            assert!(!source_map.frames.is_empty());
            let native_node = native_node.as_ref().expect("native owner reference");
            assert!(Arc::ptr_eq(native_node.owner(), &owner));
            assert!(matches!(
                native_node.handle(),
                XPathNativeNodeHandle::XmlEvent { .. }
            ));
            assert_eq!(native_node.source_map(), *source_map);
        }
    }

    #[test]
    fn xpath_native_evaluator_filters_attributes_with_general_equality_and_ebv() {
        use crate::lifecycle::LoadedInputAstStream;
        use crate::validation::xml::{
            xml_document_ast_from_source_bytes, XmlSourceValidationRequest,
        };
        use std::sync::Arc;

        let source = br#"<catalog><book lang="en"><title>Alpha</title></book><book><title>Beta</title></book><book lang="e&#110;"><title>Gamma</title></book></catalog>"#;
        let (document, diagnostics) =
            xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
                bytes: source,
                source_uri: "memory://catalog.xml",
                content_type: Some("application/xml"),
            });
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let owner = Arc::new(LoadedInputAstStream::XmlDocument(
            document.expect("typed XML document"),
        ));
        let context_node =
            XPathNativeNode::xml_document(Arc::clone(&owner)).expect("native XML document node");
        let resolver_registry = ResolverRegistry::new();
        let resolver_policy = ResolverPolicy::new();

        for (source, expected_titles) in [
            ("/catalog/book[@lang = \"en\"]/title", 2),
            ("/catalog/book[@lang]/title", 2),
        ] {
            let expression = parse(source);
            let result = CemXPathEvaluator::default()
                .evaluate(XPathEvaluationRequest {
                    invocation_host: XPathInvocationHost::StandaloneTransform,
                    expression: &expression,
                    dynamic_context: XPathDynamicContext {
                        context_item: Some(XPathResultItem::from_native_node(context_node.clone())),
                        variable_bindings: BTreeMap::new(),
                    },
                    static_context: XPathStaticContext::default(),
                    expected_result: None,
                    resolver_registry: &resolver_registry,
                    resolver_policy: &resolver_policy,
                    safety_policy_stamp: "xpath-safety/1;pure",
                })
                .unwrap_or_else(|diagnostics| panic!("`{source}` failed: {diagnostics:?}"));

            assert_eq!(result.sequence.items.len(), expected_titles, "`{source}`");
            let handles = result
                .sequence
                .items
                .iter()
                .map(|item| {
                    let native_node = item.native_node().expect("native title node");
                    assert!(Arc::ptr_eq(native_node.owner(), &owner));
                    assert_eq!(
                        item,
                        &XPathResultItem::from_native_node(native_node.clone())
                    );
                    native_node.handle()
                })
                .collect::<Vec<_>>();
            assert!(handles.windows(2).all(|pair| match pair {
                [
                    XPathNativeNodeHandle::XmlEvent { event_index: left },
                    XPathNativeNodeHandle::XmlEvent { event_index: right },
                ] => left < right,
                _ => false,
            }));
        }
    }

    #[test]
    fn xpath_native_evaluator_applies_axis_focus_and_deduplicates_in_document_order() {
        use crate::lifecycle::LoadedInputAstStream;
        use crate::validation::xml::{
            xml_document_ast_from_source_bytes, XmlSourceValidationRequest,
        };
        use std::sync::Arc;

        let source = br#"<catalog><shelf><book id="a"/><book id="b"/></shelf><shelf><book id="c"/><book id="d"/></shelf></catalog>"#;
        let (document, diagnostics) =
            xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
                bytes: source,
                source_uri: "memory://catalog.xml",
                content_type: Some("application/xml"),
            });
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let owner = Arc::new(LoadedInputAstStream::XmlDocument(
            document.expect("typed XML document"),
        ));
        let context_node =
            XPathNativeNode::xml_document(Arc::clone(&owner)).expect("native XML document node");
        let resolver_registry = ResolverRegistry::new();
        let resolver_policy = ResolverPolicy::new();

        let evaluate = |source: &str| {
            let expression = parse(source);
            CemXPathEvaluator::default()
                .evaluate(XPathEvaluationRequest {
                    invocation_host: XPathInvocationHost::StandaloneTransform,
                    expression: &expression,
                    dynamic_context: XPathDynamicContext {
                        context_item: Some(XPathResultItem::from_native_node(context_node.clone())),
                        variable_bindings: BTreeMap::new(),
                    },
                    static_context: XPathStaticContext::default(),
                    expected_result: None,
                    resolver_registry: &resolver_registry,
                    resolver_policy: &resolver_policy,
                    safety_policy_stamp: "xpath-safety/1;pure",
                })
                .unwrap_or_else(|diagnostics| panic!("`{source}` failed: {diagnostics:?}"))
        };

        for source in [
            "//shelf/book[2]/@id",
            "//shelf/book[2.0]/@id",
            "//shelf/book[2e0]/@id",
        ] {
            let second_ids = evaluate(source);
            assert_eq!(second_ids.sequence.items.len(), 2, "`{source}`");
            assert_eq!(
                second_ids
                    .sequence
                    .items
                    .iter()
                    .map(|item| {
                        let XPathResultItem::Node {
                            node_kind,
                            expanded_name,
                            source_map,
                            native_node: Some(native_node),
                            ..
                        } = item
                        else {
                            panic!("expected native attribute node: {item:?}");
                        };
                        assert_eq!(*node_kind, XPathResultNodeKind::Attribute);
                        assert_eq!(expanded_name.as_deref(), Some("id"));
                        assert!(Arc::ptr_eq(native_node.owner(), &owner));
                        assert_eq!(native_node.source_map(), *source_map);
                        match native_node.handle() {
                            XPathNativeNodeHandle::XmlAttribute { .. } => {}
                            handle => panic!("expected attribute handle, got {handle:?}"),
                        }
                        native_node.string_value()
                    })
                    .collect::<Vec<_>>(),
                ["b", "d"],
                "`{source}`"
            );
        }

        let deduplicated = evaluate("//book/../descendant::book");
        assert_eq!(deduplicated.sequence.items.len(), 4);
        let handles = deduplicated
            .sequence
            .items
            .iter()
            .map(|item| item.native_node().expect("native book node").handle())
            .collect::<Vec<_>>();
        assert!(handles.windows(2).all(|pair| match pair {
            [
                XPathNativeNodeHandle::XmlEvent { event_index: left },
                XPathNativeNodeHandle::XmlEvent { event_index: right },
            ] => left < right,
            _ => false,
        }));
    }

    #[test]
    fn xpath_native_evaluator_preserves_forward_reverse_and_filter_focus_order() {
        use crate::lifecycle::LoadedInputAstStream;
        use crate::validation::xml::{
            xml_document_ast_from_source_bytes, XmlSourceValidationRequest,
        };
        use std::sync::Arc;

        let source = br#"<catalog id="root"><shelf id="s1"><book id="a"/><book id="b"/></shelf><shelf id="s2"><book id="c"/><book id="d"/></shelf></catalog>"#;
        let (document, diagnostics) =
            xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
                bytes: source,
                source_uri: "memory://catalog.xml",
                content_type: Some("application/xml"),
            });
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let owner = Arc::new(LoadedInputAstStream::XmlDocument(
            document.expect("typed XML document"),
        ));
        let context_node =
            XPathNativeNode::xml_document(Arc::clone(&owner)).expect("native XML document node");
        let resolver_registry = ResolverRegistry::new();
        let resolver_policy = ResolverPolicy::new();

        let evaluate_ids = |xpath_source: &str| {
            let expression = parse(xpath_source);
            let result = CemXPathEvaluator::default()
                .evaluate(XPathEvaluationRequest {
                    invocation_host: XPathInvocationHost::StandaloneTransform,
                    expression: &expression,
                    dynamic_context: XPathDynamicContext {
                        context_item: Some(XPathResultItem::from_native_node(context_node.clone())),
                        variable_bindings: BTreeMap::new(),
                    },
                    static_context: XPathStaticContext::default(),
                    expected_result: None,
                    resolver_registry: &resolver_registry,
                    resolver_policy: &resolver_policy,
                    safety_policy_stamp: "xpath-safety/1;pure",
                })
                .unwrap_or_else(|diagnostics| panic!("`{xpath_source}` failed: {diagnostics:?}"));

            result
                .sequence
                .items
                .iter()
                .map(|item| {
                    let XPathResultItem::Node {
                        node_kind,
                        expanded_name,
                        source_map,
                        native_node: Some(native_node),
                        ..
                    } = item
                    else {
                        panic!("expected native attribute result: {item:?}");
                    };
                    assert_eq!(*node_kind, XPathResultNodeKind::Attribute);
                    assert_eq!(expanded_name.as_deref(), Some("id"));
                    assert!(Arc::ptr_eq(native_node.owner(), &owner));
                    assert_eq!(native_node.source_map(), *source_map);
                    native_node.string_value()
                })
                .collect::<Vec<_>>()
        };

        for (xpath_source, expected_ids) in [
            ("//book/ancestor::*[1]/@id", vec!["s1", "s2"]),
            ("(//book/ancestor::*)[1]/@id", vec!["root"]),
            (
                "//book/ancestor-or-self::*[1]/@id",
                vec!["a", "b", "c", "d"],
            ),
            ("//book/preceding-sibling::*[1]/@id", vec!["a", "c"]),
            ("//book/following-sibling::*[1]/@id", vec!["b", "d"]),
            ("/catalog/shelf[1]/following::*[1]/@id", vec!["s2"]),
            ("/catalog/shelf[2]/preceding::*[1]/@id", vec!["b"]),
            ("//shelf/book[last()]/@id", vec!["b", "d"]),
            ("//shelf/book[fn:last()]/@id", vec!["b", "d"]),
            ("//shelf/book[position()]/@id", vec!["a", "b", "c", "d"]),
            ("(//book)[2]/@id", vec!["b"]),
            ("(//book)[last()]/@id", vec!["d"]),
            ("//@id/self::*", Vec::new()),
        ] {
            assert_eq!(evaluate_ids(xpath_source), expected_ids, "`{xpath_source}`");
        }
    }

    #[test]
    fn xpath_native_evaluator_keeps_optional_namespace_axis_explicitly_unsupported() {
        use crate::lifecycle::LoadedInputAstStream;
        use crate::validation::xml::{
            xml_document_ast_from_source_bytes, XmlSourceValidationRequest,
        };
        use std::sync::Arc;

        let (document, diagnostics) =
            xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
                bytes: br#"<root xmlns="urn:example"/>"#,
                source_uri: "memory://namespaces.xml",
                content_type: Some("application/xml"),
            });
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let owner = Arc::new(LoadedInputAstStream::XmlDocument(
            document.expect("typed XML document"),
        ));
        let context_node = XPathNativeNode::xml_document(owner).expect("native XML document node");
        let expression = parse("/*/namespace::*");
        let resolver_registry = ResolverRegistry::new();
        let resolver_policy = ResolverPolicy::new();
        let diagnostics = CemXPathEvaluator::default()
            .evaluate(XPathEvaluationRequest {
                invocation_host: XPathInvocationHost::StandaloneTransform,
                expression: &expression,
                dynamic_context: XPathDynamicContext {
                    context_item: Some(XPathResultItem::from_native_node(context_node)),
                    variable_bindings: BTreeMap::new(),
                },
                static_context: XPathStaticContext::default(),
                expected_result: None,
                resolver_registry: &resolver_registry,
                resolver_policy: &resolver_policy,
                safety_policy_stamp: "xpath-safety/1;pure",
            })
            .expect_err("the optional namespace axis remains unsupported");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].code, "cem.xpath.evaluation_unsupported");
        assert!(diagnostics[0].message.contains("Namespace"));
    }

    #[test]
    fn xpath_native_evaluator_uses_package_ast_for_literals_variables_and_context() {
        let expression = parse("$prefix, ., 42, 'native'");
        let resolver_registry = ResolverRegistry::new();
        let resolver_policy = ResolverPolicy::new();
        let context = XPathResultItem::Atomic {
            value: XPathAtomicValue {
                type_name: "xs:string".to_owned(),
                lexical_value: "context".to_owned(),
                namespace_uri: None,
                local_name: None,
            },
            source_map: result_source_map(9, 2),
        };
        let variables = BTreeMap::from([(
            XPathExpandedName::unqualified("prefix"),
            XPathResultSequence {
                sequence_type: "xs:string".to_owned(),
                items: vec![XPathResultItem::Atomic {
                    value: XPathAtomicValue {
                        type_name: "xs:string".to_owned(),
                        lexical_value: "variable".to_owned(),
                        namespace_uri: None,
                        local_name: None,
                    },
                    source_map: result_source_map(9, 1),
                }],
            },
        )]);

        let result = CemXPathEvaluator::default()
            .evaluate(XPathEvaluationRequest {
                invocation_host: XPathInvocationHost::StandaloneTransform,
                expression: &expression,
                dynamic_context: XPathDynamicContext {
                    context_item: Some(context),
                    variable_bindings: variables,
                },
                static_context: XPathStaticContext::default(),
                expected_result: None,
                resolver_registry: &resolver_registry,
                resolver_policy: &resolver_policy,
                safety_policy_stamp: "xpath-safety/1;pure",
            })
            .expect("native scalar evaluation");

        let lexical_values = result
            .sequence
            .items
            .iter()
            .map(|item| match item {
                XPathResultItem::Atomic { value, .. } => value.lexical_value.as_str(),
                _ => panic!("expected atomic result: {item:?}"),
            })
            .collect::<Vec<_>>();
        assert_eq!(lexical_values, ["variable", "context", "42", "native"]);
    }

    #[test]
    fn xpath_native_evaluator_compares_exact_atomic_values_without_bounded_numbers() {
        let assert_boolean = |source: &str, expected: bool| {
            let result = evaluate_for_test(source, None, BTreeMap::new())
                .unwrap_or_else(|diagnostics| panic!("`{source}` failed: {diagnostics:?}"));
            let [XPathResultItem::Atomic { value, source_map }] = result.sequence.items.as_slice()
            else {
                panic!("`{source}` did not return one atomic value: {result:?}");
            };
            assert_eq!(value.type_name, "xs:boolean", "`{source}`");
            assert_eq!(value.lexical_value, expected.to_string(), "`{source}`");
            assert!(!source_map.frames.is_empty(), "`{source}`");
        };

        for (source, expected) in [
            (
                "999999999999999999999999999999999999 = 999999999999999999999999999999999999",
                true,
            ),
            (
                "999999999999999999999999999999999999 < 1000000000000000000000000000000000000",
                true,
            ),
            ("1.20 eq 1.2", true),
            ("1.0000000000000000000000001 gt 1.0", true),
            ("1 = 1.0", true),
            ("1 = 1e0", true),
            ("'alpha' lt 'beta'", true),
            ("(1, 2) = (2, 3)", true),
            ("(1, 2) != (2, 3)", true),
            ("(1, 2) = 3", false),
        ] {
            assert_boolean(source, expected);
        }

        let empty = evaluate_for_test("() eq 1", None, BTreeMap::new())
            .expect("an empty value-comparison operand returns the empty sequence");
        assert!(empty.sequence.items.is_empty());

        let diagnostics = evaluate_for_test("(1, 2) eq 1", None, BTreeMap::new())
            .expect_err("value comparisons require singleton atomized operands");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(
            diagnostics[0].code,
            "cem.xpath.value_comparison_cardinality"
        );
    }

    #[test]
    fn xpath_native_evaluator_executes_logical_operators_with_ebv_and_short_circuiting() {
        use crate::lifecycle::LoadedInputAstStream;
        use crate::validation::xml::{
            xml_document_ast_from_source_bytes, XmlSourceValidationRequest,
        };
        use std::sync::Arc;

        let (document, diagnostics) =
            xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
                bytes: br#"<root><n/></root>"#,
                source_uri: "memory://logical-context.xml",
                content_type: Some("application/xml"),
            });
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let owner = Arc::new(LoadedInputAstStream::XmlDocument(
            document.expect("typed XML document"),
        ));
        let context_node = XPathNativeNode::xml_document(owner).expect("native XML document node");

        let assert_boolean = |source: &str, expected: bool| {
            let result = evaluate_for_test(
                source,
                Some(XPathResultItem::from_native_node(context_node.clone())),
                BTreeMap::new(),
            )
            .unwrap_or_else(|diagnostics| panic!("`{source}` failed: {diagnostics:?}"));
            let [XPathResultItem::Atomic { value, source_map }] = result.sequence.items.as_slice()
            else {
                panic!("`{source}` did not return one atomic value: {result:?}");
            };
            assert_eq!(value.type_name, "xs:boolean", "`{source}`");
            assert_eq!(value.lexical_value, expected.to_string(), "`{source}`");
            assert_eq!(source_map.frames.len(), 1, "`{source}`");
            assert_eq!(source_map.frames[0].source_id, SourceId(7), "`{source}`");
            assert_eq!(
                source_map.frames[0].span,
                FrameSpan::Single(ByteRange::new(0, source.len() as u32)),
                "`{source}`"
            );
        };

        for (source, expected) in [
            ("1 = 1 and 2 = 2", true),
            ("1 = 1 and 2 = 3", false),
            ("1 = 2 or 2 = 2", true),
            ("1 = 2 or 2 = 3", false),
            ("/root/n and 'non-empty'", true),
            ("/root/m or ''", false),
            ("1 = 2 and $missing", false),
            ("1 = 1 or $missing", true),
        ] {
            assert_boolean(source, expected);
        }

        for source in ["1 = 1 and $missing", "1 = 2 or $missing"] {
            let diagnostics = evaluate_for_test(
                source,
                Some(XPathResultItem::from_native_node(context_node.clone())),
                BTreeMap::new(),
            )
            .expect_err("a required logical operand must be evaluated");
            assert_eq!(diagnostics[0].code, "cem.xpath.variable_unbound");
            assert_eq!(
                diagnostics[0].byte_offset,
                Some(source.find("missing").expect("missing variable offset") as u64),
                "`{source}`"
            );
        }

        for source in ["(1, 2) and 1 = 1", "1 = 2 or (1, 2)"] {
            let diagnostics = evaluate_for_test(
                source,
                Some(XPathResultItem::from_native_node(context_node.clone())),
                BTreeMap::new(),
            )
            .expect_err("logical operands require a defined effective boolean value");
            assert_eq!(
                diagnostics[0].code,
                "cem.xpath.effective_boolean_value_type_error"
            );
            assert!(diagnostics[0].source_map.is_some(), "`{source}`");
        }
    }

    #[test]
    fn xpath_native_evaluator_applies_typed_promotion_and_untyped_atomization() {
        use crate::lifecycle::LoadedInputAstStream;
        use crate::validation::xml::{
            xml_document_ast_from_source_bytes, XmlSourceValidationRequest,
        };
        use std::sync::Arc;

        let source = br#"<root><n>2</n><n>10</n><flag>true</flag><bad>not-a-number</bad></root>"#;
        let (document, diagnostics) =
            xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
                bytes: source,
                source_uri: "memory://atomic-values.xml",
                content_type: Some("application/xml"),
            });
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let owner = Arc::new(LoadedInputAstStream::XmlDocument(
            document.expect("typed XML document"),
        ));
        let context_node = XPathNativeNode::xml_document(owner).expect("native XML document node");

        let variables = BTreeMap::from([
            (
                XPathExpandedName::unqualified("truth"),
                singleton_test_binding("xs:boolean", "true"),
            ),
            (
                XPathExpandedName::unqualified("falsehood"),
                singleton_test_binding("xs:boolean", "false"),
            ),
            (
                XPathExpandedName::unqualified("float"),
                singleton_test_binding("xs:float", "0.1"),
            ),
            (
                XPathExpandedName::unqualified("double"),
                singleton_test_binding("xs:double", "0.1"),
            ),
            (
                XPathExpandedName::unqualified("decimal"),
                singleton_test_binding("xs:decimal", "0.1"),
            ),
            (
                XPathExpandedName::unqualified("negative"),
                singleton_test_binding(
                    "xs:decimal",
                    "-999999999999999999999999999999999999.00000000000000000001",
                ),
            ),
            (
                XPathExpandedName::unqualified("nan"),
                singleton_test_binding("xs:double", "NaN"),
            ),
            (
                XPathExpandedName::unqualified("uri"),
                singleton_test_binding("xs:anyURI", "urn:example"),
            ),
            (
                XPathExpandedName::unqualified("string"),
                singleton_test_binding("xs:string", "urn:example"),
            ),
        ]);

        let assert_boolean = |xpath_source: &str, expected: bool| {
            let result = evaluate_for_test(
                xpath_source,
                Some(XPathResultItem::from_native_node(context_node.clone())),
                variables.clone(),
            )
            .unwrap_or_else(|diagnostics| panic!("`{xpath_source}` failed: {diagnostics:?}"));
            let [XPathResultItem::Atomic { value, .. }] = result.sequence.items.as_slice() else {
                panic!("`{xpath_source}` did not return one atomic value: {result:?}");
            };
            assert_eq!(value.type_name, "xs:boolean", "`{xpath_source}`");
            assert_eq!(
                value.lexical_value,
                expected.to_string(),
                "`{xpath_source}`"
            );
        };

        for (xpath_source, expected) in [
            ("/root/n = 2", true),
            ("/root/n < 3", true),
            ("/root/n = '10'", true),
            ("/root/n[1] = /root/n[2]", false),
            ("/root/n[1] eq '2'", true),
            ("/root/flag = $truth", true),
            ("$falsehood lt $truth", true),
            ("$float eq $decimal", true),
            ("$float eq $double", false),
            ("$negative lt 0", true),
            ("$nan eq $nan", false),
            ("$nan ne $nan", true),
            ("$uri eq $string", true),
        ] {
            assert_boolean(xpath_source, expected);
        }

        let diagnostics = evaluate_for_test(
            "/root/n eq 2",
            Some(XPathResultItem::from_native_node(context_node.clone())),
            variables.clone(),
        )
        .expect_err("value comparison rejects a multi-value atomized operand");
        assert_eq!(
            diagnostics[0].code,
            "cem.xpath.value_comparison_cardinality"
        );

        let diagnostics = evaluate_for_test(
            "/root/n[1] eq 2",
            Some(XPathResultItem::from_native_node(context_node.clone())),
            variables.clone(),
        )
        .expect_err("value comparison casts untyped atomic values to strings");
        assert_eq!(diagnostics[0].code, "cem.xpath.comparison_type_error");

        let diagnostics = evaluate_for_test(
            "/root/bad = 2",
            Some(XPathResultItem::from_native_node(context_node)),
            variables,
        )
        .expect_err("general comparison must report an invalid untyped numeric cast");
        assert_eq!(diagnostics[0].code, "cem.xpath.atomic_cast_invalid");
    }

    #[test]
    fn xpath_native_evaluator_rejects_unsupported_semantics_without_projection() {
        let expression = parse("1 + 2");
        let resolver_registry = ResolverRegistry::new();
        let resolver_policy = ResolverPolicy::new();
        let diagnostics = CemXPathEvaluator::default()
            .evaluate(XPathEvaluationRequest {
                invocation_host: XPathInvocationHost::StandaloneTransform,
                expression: &expression,
                dynamic_context: XPathDynamicContext::default(),
                static_context: XPathStaticContext::default(),
                expected_result: None,
                resolver_registry: &resolver_registry,
                resolver_policy: &resolver_policy,
                safety_policy_stamp: "xpath-safety/1;pure",
            })
            .expect_err("operators are outside the first evaluator slice");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].code, "cem.xpath.evaluation_unsupported");

        let source = include_str!("xpath.rs");
        let evaluator = source
            .split_once("pub struct CemXPathEvaluator")
            .expect("native evaluator source boundary")
            .1
            .split_once("pub fn validate_xpath_evaluator_capabilities")
            .expect("native evaluator source boundary")
            .0;
        for forbidden in [
            "serde_json",
            "to_cemt_subject",
            "source_text",
            "xml_document_ast_from_source_bytes",
        ] {
            assert!(
                !evaluator.contains(forbidden),
                "native evaluator must not cross `{forbidden}`"
            );
        }
    }

    #[test]
    fn xpath_transform_adapter_routes_lifecycle_xml_owner_to_typed_result_without_projection() {
        use crate::engine::{
            FormatIdentity, TemplateInput, TransformExecutionPolicy, TransformRuntimePhase,
            TransformTemplateEntrypoint, TransformTemplateKind,
        };
        use crate::lifecycle::LoadedInputAstStream;
        use crate::resolver::{ResolverPolicy, ResolverRegistry};
        use crate::run_config::ScopeConfig;
        use crate::schema::registry::{XML_CONTENT_TYPE, XML_SCHEMA_URI};
        use crate::transform_artifact::{TransformArtifactBody, TransformDataArtifact};
        use crate::transform_template::{
            TransformTemplateAdapter, TransformTemplateCompileRequest,
            TransformTemplateModuleOptions, TransformTemplateModulePreflight,
            TransformTemplateParameterArena, TransformTemplateRenderRequest,
            TransformTemplateRuntimeContext,
        };
        use crate::validation::xml::{
            xml_document_ast_from_source_bytes, XmlSourceValidationRequest,
        };
        use std::sync::Arc;

        let (document, diagnostics) =
            xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
                bytes: b"<catalog><book/><book/></catalog>",
                source_uri: "memory://catalog.xml",
                content_type: Some(XML_CONTENT_TYPE),
            });
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let owner = Arc::new(LoadedInputAstStream::XmlDocument(
            document.expect("typed XML document"),
        ));
        let template = TemplateInput {
            uri: "memory://books.xpath".to_owned(),
            bytes: b"/catalog/book".to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(XPATH_CONTENT_TYPE.to_owned()),
                schema: Some(XPATH_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let params = TransformTemplateParameterArena::default();
        let entrypoint = TransformTemplateEntrypoint::implicit();
        let execution_policy = TransformExecutionPolicy {
            runtime_phase: TransformRuntimePhase::XPath,
            ..TransformExecutionPolicy::default()
        };
        let adapter = XPathTransformTemplateAdapter::default();
        assert_eq!(adapter.kind(), TransformTemplateKind::XPath);
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &entrypoint,
                params: &params,
                data_bindings: &["input".to_owned()],
                module_options: TransformTemplateModuleOptions::default(),
                module_preflight: TransformTemplateModulePreflight::default(),
                execution_policy,
            })
            .expect("XPath transform compile");
        assert!(
            compiled.diagnostics.is_empty(),
            "{:?}",
            compiled.diagnostics
        );

        let primary_input = TransformDataArtifact::new(
            "input",
            Some("memory://catalog.xml".to_owned()),
            Some(FormatIdentity {
                content_type: Some(XML_CONTENT_TYPE.to_owned()),
                schema: Some(XML_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            TransformArtifactBody::Lifecycle(Arc::clone(&owner)),
        );
        let secondary_inputs = BTreeMap::new();
        let target_scope = ScopeConfig::default();
        let target = FormatIdentity {
            content_type: Some(XPATH_RESULT_CONTENT_TYPE.to_owned()),
            schema: Some(XPATH_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let resolver_registry = ResolverRegistry::new();
        let resolver_policy = ResolverPolicy::new();
        let rendered = adapter
            .render_with_runtime(
                TransformTemplateRenderRequest {
                    compiled: &compiled.artifact,
                    primary_input: &primary_input,
                    secondary_inputs: &secondary_inputs,
                    target: Some(&target),
                    target_scope: &target_scope,
                    execution_policy,
                },
                TransformTemplateRuntimeContext {
                    resolver_registry: &resolver_registry,
                    resolver_policy: &resolver_policy,
                },
            )
            .expect("XPath transform render");
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
        let TransformArtifactBody::XPathResult(result) = rendered.output.body else {
            panic!("XPath transform must return a typed XPath result artifact")
        };
        assert_eq!(result.sequence.items.len(), 2);
        for item in &result.sequence.items {
            let native_node = item.native_node().expect("native XPath result node");
            assert!(Arc::ptr_eq(native_node.owner(), &owner));
        }
    }

    #[test]
    fn xpath_schema_declares_evaluation_result_and_capability_contracts() {
        let source = builtin_schema_package_source(XPATH_PACKAGE_ID)
            .expect("XPath package source")
            .schema_source;
        let model = compile_schema_document_model(XPATH_SCHEMA_URI, source);

        for element in [
            "evaluation-request",
            "evaluator-capabilities",
            "result-artifact",
            "sequence",
            "node-item",
            "atomic-item",
            "map-item",
            "array-item",
            "function-item",
            "source-map-frame",
            "source-range",
        ] {
            assert!(
                model.elements.contains_key(element),
                "XPath schema must own `{element}`"
            );
        }
        let result = model.elements.get("result-artifact").unwrap();
        assert!(result.required_attributes.contains("content-type"));
        assert!(result.required_attributes.contains("host-language"));
        assert!(result.required_attributes.contains("evaluator-id"));
        assert!(result.required_attributes.contains("resolver-policy-stamp"));
        assert!(result.required_attributes.contains("safety-policy-stamp"));
        assert!(result.child_elements.contains("sequence"));
        let request = model.elements.get("evaluation-request").unwrap();
        assert!(request.required_attributes.contains("host-language"));
        let variable_value = model.elements.get("variable-value").unwrap();
        assert!(variable_value.optional_attributes.contains("namespace-uri"));
        assert!(variable_value.optional_attributes.contains("local-name"));
        for contract in [
            "xpath-evaluator-package-ast",
            "xpath-evaluator-resource-access",
            "xpath-evaluator-runtime-targets",
            "xpath-evaluation-feature-support",
            "xpath-invocation-host-association",
            "xpath-invocation-variable-expanded-name",
            "xpath-result-item-order",
            "xpath-result-node-identity",
            "xpath-result-function-scope",
            "xpath-result-policy-stamps",
        ] {
            assert!(
                model.constraints.contains_key(contract),
                "XPath schema must own `{contract}`"
            );
        }
        for diagnostic in [
            "cem.xpath.evaluation_ast_missing",
            "cem.xpath.evaluation_unsupported",
            "cem.xpath.invocation_host_mismatch",
            "cem.xpath.context_item_missing",
            "cem.xpath.context_item_native_node_required",
            "cem.xpath.variable_unbound",
        ] {
            assert!(
                model.diagnostics.contains_key(diagnostic),
                "XPath schema must own `{diagnostic}`"
            );
        }
    }

    #[test]
    fn xpath_full_conformance_matrix_is_schema_owned_and_actionable() {
        let source = include_str!("../../schema-packages/xpath/v1/tests/xpath-3.1-conformance.cem");
        let document = parse_cem_contract(source);
        assert!(
            document.diagnostics.is_empty(),
            "conformance matrix must parse as CEM: {:?}",
            document.diagnostics
        );

        let model = compile_schema_document_model(
            XPATH_SCHEMA_URI,
            builtin_schema_package_source(XPATH_PACKAGE_ID)
                .expect("XPath package source")
                .schema_source,
        );
        let diagnostics = crate::schema::document_model::validate_document_model(&document, &model);
        assert!(
            diagnostics.is_empty(),
            "conformance matrix must satisfy the XPath schema: {diagnostics:?}"
        );

        let profiles = contract_element_ids(&document, "conformance-profile");
        assert_eq!(
            profiles.len(),
            1,
            "one XPath conformance profile is required"
        );
        let profile = contract_attributes(&document, profiles[0]);
        assert_eq!(
            profile.get("xpath-version").map(String::as_str),
            Some("3.1")
        );
        assert_eq!(profile.get("destination").map(String::as_str), Some("full"));
        assert_eq!(profile.get("delivery").map(String::as_str), Some("staged"));
        assert_eq!(profile.get("qt3-version").map(String::as_str), Some("3.1"));

        let references = contract_element_ids(&document, "normative-reference")
            .into_iter()
            .map(|node_id| contract_attributes(&document, node_id))
            .collect::<Vec<_>>();
        let reference_ids = references
            .iter()
            .filter_map(|reference| reference.get("id").cloned())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            reference_ids,
            BTreeSet::from([
                "fo31".to_owned(),
                "qt3-31".to_owned(),
                "xdm31".to_owned(),
                "xpath31".to_owned(),
            ])
        );
        assert!(references.iter().all(|reference| {
            reference
                .get("uri")
                .is_some_and(|uri| uri.starts_with("https://www.w3.org/"))
        }));

        let implementation_reference_ids =
            contract_element_ids(&document, "implementation-reference");
        assert_eq!(implementation_reference_ids.len(), 1);
        let implementation_reference =
            contract_attributes(&document, implementation_reference_ids[0]);
        assert_eq!(
            implementation_reference.get("usage").map(String::as_str),
            Some("reference-only")
        );
        assert_eq!(
            implementation_reference.get("commit").map(String::as_str),
            Some("200b1e3356ea9d6dd2901d67bd941b779df7e5b7")
        );

        let slices = contract_element_ids(&document, "conformance-slice")
            .into_iter()
            .map(|node_id| contract_attributes(&document, node_id))
            .collect::<Vec<_>>();
        assert!(
            slices.len() >= 10,
            "full XPath requires a complete slice inventory"
        );
        let slice_ids = slices
            .iter()
            .map(|slice| slice.get("id").expect("slice id").clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(slice_ids.len(), slices.len(), "slice ids must be unique");
        for required in [
            "syntax-and-static-context",
            "expressions-and-control-flow",
            "paths-and-node-tests",
            "operators",
            "type-system",
            "function-items",
            "maps-and-arrays",
            "functions-and-operators",
            "dynamic-context-and-resources",
            "schema-aware-evaluation",
            "xdm-results-and-serialization",
        ] {
            assert!(
                slice_ids.contains(required),
                "missing conformance slice `{required}`"
            );
        }
        for slice in &slices {
            let status = slice.get("status").expect("slice status");
            assert!(
                matches!(
                    status.as_str(),
                    "complete" | "transitional" | "partial" | "contract-only" | "planned"
                ),
                "unsupported slice status `{status}`"
            );
            if status != "complete" {
                assert!(
                    slice.get("gap").is_some_and(|gap| !gap.trim().is_empty()),
                    "non-complete slice requires a gap: {slice:?}"
                );
                assert!(
                    slice
                        .get("todo")
                        .is_some_and(|todo| !todo.trim().is_empty()),
                    "non-complete slice requires a todo reference: {slice:?}"
                );
            }
            assert!(
                slice.get("qt3-status").is_some(),
                "slice must declare QT3 mapping state: {slice:?}"
            );
        }
    }

    #[test]
    fn xpath_ast_is_lossless_for_unicode_names_nested_comments_and_xpath_31_structures() {
        let source = "for $\u{03c0} in /catalog/book[@lang = \"en\"] (: outer (: nested :) :) return map { \"title\": $\u{03c0}/title }";
        let ast = parse(source);

        assert_eq!(
            ast.tokens
                .iter()
                .map(|token| token.lexeme.as_str())
                .collect::<String>(),
            source
        );
        assert!(ast.syntax_ast.is_some());
        assert!(ast
            .facts
            .iter()
            .any(|fact| fact.kind == XPathFactKind::Parsed));
        assert!(ast.tokens.iter().any(|token| {
            token.kind == XPathTokenKind::Comment && token.lexeme.contains("(: nested :)")
        }));
        assert!(ast.tokens.iter().any(|token| {
            token.kind == XPathTokenKind::Name
                && token.lexeme == "\u{03c0}"
                && token.source_range.byte_length == 2
        }));
        assert!(ast
            .tokens
            .windows(2)
            .all(
                |pair| pair[0].source_range.start.byte_offset + pair[0].source_range.byte_length
                    == pair[1].source_range.start.byte_offset
            ));
        assert_eq!(ast.events.len(), ast.tokens.len() + 2);
        assert_eq!(
            ast.events.first().map(|event| event.kind),
            Some(XPathAstEventKind::StartExpression)
        );
        assert_eq!(
            ast.events.last().map(|event| event.kind),
            Some(XPathAstEventKind::EndExpression)
        );
        assert!(ast.events[1..ast.events.len() - 1]
            .iter()
            .zip(&ast.tokens)
            .all(|(event, token)| {
                event.kind == XPathAstEventKind::Token
                    && event.token_index == Some(token.index)
                    && event.depth == token.depth
                    && event.source_range == token.source_range
            }));
    }

    #[test]
    fn xpath_cem_scanner_matches_xee_reference_boundaries_and_presentation_kinds() {
        let sources = [
            include_str!("../../schema-packages/xpath/v1/examples/basic-path.xpath"),
            include_str!("../../schema-packages/xpath/v1/examples/functions-and-variables.xpath"),
            include_str!("../../schema-packages/xpath/v1/examples/maps-arrays-and-comments.xpath"),
            include_str!("../../schema-packages/xpath/v1/examples/unicode-qname.xpath"),
            include_str!(
                "../../schema-packages/xpath/v1/examples/explicit-axes-and-escaped-string.xpath"
            ),
            include_str!("../../schema-packages/xpath/v1/examples/external-resource-denied.xpath"),
            include_str!("../../schema-packages/xpath/v1/examples/invalid-token.xpath"),
            include_str!(
                "../../schema-packages/xpath/v1/examples/invalid-unclosed-predicate.xpath"
            ),
            include_str!("../../schema-packages/xpath/v1/examples/mismatched-delimiter.xpath"),
            include_str!("../../schema-packages/xpath/v1/examples/unknown-prefix.xpath"),
            "1eq 1 and 1.25e+2 ge .5",
            "1.2.3 + .1.1",
            r#""a""b" = 'c''d'"#,
            "Q{urn:test}element | app:book | app:* | *:book",
            "for$pi in/child::book return$pi",
            "(: outer (: nested :) :) /book",
            "/book[",
        ];

        for source in sources {
            assert_eq!(
                cem_lexical_projection(source),
                xee_lexical_projection(source),
                "CEM scanner diverged from the pinned Xee lexical oracle for `{source}`"
            );
        }
    }

    #[test]
    fn xpath_cem_scanner_retains_trivia_nested_comments_and_utf8_byte_ranges() {
        let source =
            "for $\u{03c0}\n(: outer (: nested :) :) return Q{urn:test}\u{00e9}l\u{00e9}ment";
        let tokens = lexer::xpath_lexical_tokens(source);

        assert_eq!(
            tokens.iter().map(|token| token.lexeme).collect::<String>(),
            source
        );
        assert_eq!(tokens.first().map(|token| token.start), Some(0));
        assert_eq!(tokens.last().map(|token| token.end), Some(source.len()));
        assert!(tokens.windows(2).all(|pair| pair[0].end == pair[1].start));
        assert!(tokens.iter().any(|token| {
            token.kind.presentation_kind() == XPathTokenKind::Comment
                && token.lexeme == "(: outer (: nested :) :)"
        }));
        assert!(tokens.iter().any(|token| {
            token.kind.presentation_kind() == XPathTokenKind::Name
                && token.lexeme == "Q{urn:test}\u{00e9}l\u{00e9}ment"
                && token.end - token.start == token.lexeme.len()
        }));
    }

    #[test]
    fn xpath_cem_scanner_retains_parser_ready_lexical_categories() {
        use lexer::XPathLexicalTokenKind as Kind;

        let tokens = lexer::xpath_lexical_tokens(
            "1 1. .5 1e2 \"s\" name Q{urn:test}name *:name for and + ( $",
        )
        .into_iter()
        .filter(|token| token.kind != Kind::Whitespace)
        .map(|token| token.kind)
        .collect::<Vec<_>>();

        assert_eq!(
            tokens,
            vec![
                Kind::IntegerLiteral,
                Kind::DecimalLiteral,
                Kind::DecimalLiteral,
                Kind::DoubleLiteral,
                Kind::StringLiteral,
                Kind::Name,
                Kind::Name,
                Kind::DelimitingName,
                Kind::Keyword,
                Kind::WordOperator,
                Kind::SymbolOperator,
                Kind::Punctuation,
                Kind::VariableSigil,
            ]
        );
    }

    #[test]
    fn xpath_cem_scanner_preserves_malformed_lexemes_as_errors() {
        for source in ["(: unclosed", "'unclosed", "\"unclosed", "\u{00a7}"] {
            let tokens = lexer::xpath_lexical_tokens(source);
            assert_eq!(
                tokens.iter().map(|token| token.lexeme).collect::<String>(),
                source
            );
            assert!(
                tokens
                    .iter()
                    .any(|token| token.kind.presentation_kind() == XPathTokenKind::Error),
                "malformed lexical input must retain an error token: `{source}`"
            );
        }
    }

    #[test]
    fn xpath_ast_preserves_malformed_source_and_reports_parser_and_delimiter_facts() {
        let source = "/catalog/book[1";
        let ast = parse(source);

        assert_eq!(
            ast.tokens
                .iter()
                .map(|token| token.lexeme.as_str())
                .collect::<String>(),
            source
        );
        assert!(ast.syntax_ast.is_none());
        assert!(ast
            .facts
            .iter()
            .any(|fact| fact.kind == XPathFactKind::ParseError));
        assert!(ast
            .facts
            .iter()
            .any(|fact| fact.kind == XPathFactKind::UnclosedDelimiter));
    }

    #[test]
    fn xpath_ast_offsets_embedded_tokens_and_retains_owner_static_context() {
        let expression = "item/@id";
        let expression_range = XPathSourceRange::new(3, 15, 42, expression.len() as u64);
        let attachment = XPathAttachment::Host(XPathHostAttachment {
            owner: XPathHostOwner {
                source_id: 11,
                source_uri: "memory://stylesheet.xsl".to_owned(),
                content_type: Some("application/xslt+xml".to_owned()),
                schema_uri: Some("https://cem.dev/ns/transform/xslt/1".to_owned()),
                node_kind: XPathHostNodeKind::XsltAttribute,
                node_id: Some("event:4@select".to_owned()),
                source_range: XPathSourceRange::new(3, 7, 34, 19),
            },
            expression_range,
            static_context: XPathStaticContext {
                namespaces: BTreeMap::from([(
                    "app".to_owned(),
                    "https://example.test/app".to_owned(),
                )]),
                default_element_namespace: Some("https://example.test/app".to_owned()),
                variable_bindings: BTreeMap::from([("item".to_owned(), "element()".to_owned())]),
                ..XPathStaticContext::default()
            },
            expected_result: Some(XPathExpectedResult {
                sequence_type: "attribute()*".to_owned(),
                min_items: Some(0),
                max_items: None,
            }),
            evaluation_phase: XPathEvaluationPhase::Transform,
            resolver_policy_stamp: Some("resolver:none".to_owned()),
            safety_policy_stamp: Some("xpath:pure".to_owned()),
        });
        let ast = xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: expression.as_bytes(),
                source_uri: "memory://stylesheet.xsl",
                content_type: Some(XPATH_CONTENT_TYPE),
                source_range_projector: None,
            },
            attachment,
        );

        assert_eq!(ast.tokens[0].source_range.start, expression_range.start);
        assert_eq!(
            ast.syntax_ast
                .as_ref()
                .expect("typed host XPath syntax")
                .root
                .source_range
                .start,
            expression_range.start
        );
        let syntax = ast.syntax_ast.as_ref().expect("typed host XPath syntax");
        let XPathExpression::Path(path) = &syntax.root.expressions[0].expression else {
            panic!("expected host path expression");
        };
        let names = path
            .steps
            .iter()
            .filter_map(|step| match &step.step {
                XPathStep::Axis {
                    node_test: XPathNodeTest::Name(XPathNameTest::Name(name)),
                    ..
                } => Some(name),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(names[0].local_name, "item");
        assert_eq!(
            names[0].namespace_uri.as_deref(),
            Some("https://example.test/app")
        );
        assert_eq!(names[1].local_name, "id");
        assert_eq!(names[1].namespace_uri, None);
        assert!(ast
            .facts
            .iter()
            .any(|fact| fact.kind == XPathFactKind::HostAssociationObserved));
        let XPathAttachment::Host(host) = &ast.attachment else {
            panic!("expected host XPath attachment");
        };
        assert_eq!(host.owner.node_kind, XPathHostNodeKind::XsltAttribute);
        assert_eq!(
            host.static_context
                .namespaces
                .get("app")
                .map(String::as_str),
            Some("https://example.test/app")
        );
        assert_eq!(host.evaluation_phase, XPathEvaluationPhase::Transform);
        let subject = ast.to_cemt_subject();
        assert_eq!(subject["events"][0]["kind"], "start-expression");
        assert_eq!(subject["attachment"]["owner"]["nodeId"], "event:4@select");
        assert_eq!(subject["attachment"]["evaluationPhase"], "transform");
    }

    #[test]
    fn xpath_package_examples_match_declared_parse_expectations() {
        let passing = [
            (
                "basic-path",
                include_str!("../../schema-packages/xpath/v1/examples/basic-path.xpath"),
            ),
            (
                "functions-and-variables",
                include_str!(
                    "../../schema-packages/xpath/v1/examples/functions-and-variables.xpath"
                ),
            ),
            (
                "maps-arrays-and-comments",
                include_str!(
                    "../../schema-packages/xpath/v1/examples/maps-arrays-and-comments.xpath"
                ),
            ),
            (
                "unicode-qname",
                include_str!("../../schema-packages/xpath/v1/examples/unicode-qname.xpath"),
            ),
            (
                "explicit-axes-and-escaped-string",
                include_str!(
                    "../../schema-packages/xpath/v1/examples/explicit-axes-and-escaped-string.xpath"
                ),
            ),
        ];

        for (name, source) in passing {
            let ast = parse(source);
            assert_eq!(
                ast.tokens
                    .iter()
                    .map(|token| token.lexeme.as_str())
                    .collect::<String>(),
                source,
                "{name} must retain every source byte"
            );
            assert!(
                ast.syntax_ast.is_some(),
                "{name} must parse: {:?}",
                ast.facts
            );
            assert!(
                ast.syntax_ast
                    .as_ref()
                    .is_some_and(|syntax| syntax.events.iter().all(|event| !matches!(
                        event.node_kind,
                        XPathSyntaxNodeKind::UnsupportedExpression
                            | XPathSyntaxNodeKind::UnsupportedPrimary
                    ))),
                "{name} must lower completely into the current CEM AST slice"
            );
            assert!(
                ast.facts
                    .iter()
                    .any(|fact| fact.kind == XPathFactKind::Parsed),
                "{name} must report its parsed fact"
            );
        }

        let invalid = parse(include_str!(
            "../../schema-packages/xpath/v1/examples/invalid-unclosed-predicate.xpath"
        ));
        assert!(invalid.syntax_ast.is_none());
        assert!(invalid
            .facts
            .iter()
            .any(|fact| fact.kind == XPathFactKind::ParseError));
        assert!(invalid
            .facts
            .iter()
            .any(|fact| fact.kind == XPathFactKind::UnclosedDelimiter));

        for (name, source, expected_facts) in [
            (
                "unknown-prefix",
                include_str!("../../schema-packages/xpath/v1/examples/unknown-prefix.xpath"),
                vec![XPathFactKind::UnknownNamespacePrefix],
            ),
            (
                "invalid-token",
                include_str!("../../schema-packages/xpath/v1/examples/invalid-token.xpath"),
                vec![XPathFactKind::LexicalError],
            ),
            (
                "mismatched-delimiter",
                include_str!("../../schema-packages/xpath/v1/examples/mismatched-delimiter.xpath"),
                vec![
                    XPathFactKind::ParseError,
                    XPathFactKind::MismatchedDelimiter,
                    XPathFactKind::UnclosedDelimiter,
                ],
            ),
        ] {
            let ast = parse(source);
            assert!(ast.syntax_ast.is_none(), "{name} must not parse");
            for expected in expected_facts {
                assert!(
                    ast.facts.iter().any(|fact| fact.kind == expected),
                    "{name} must report {expected:?}: {:?}",
                    ast.facts
                );
            }
        }
    }

    #[test]
    fn xpath_schema_contract_binds_reportable_facts_to_diagnostics() {
        let catalog = XPathSchemaContractCatalog::from_builtin();
        for (kind, code, severity) in [
            (
                XPathFactKind::InvalidUtf8,
                "cem.xpath.invalid_utf8",
                crate::diagnostics::Severity::Error,
            ),
            (
                XPathFactKind::LexicalError,
                "cem.xpath.lexical_error",
                crate::diagnostics::Severity::Error,
            ),
            (
                XPathFactKind::ParseError,
                "cem.xpath.parse_error",
                crate::diagnostics::Severity::Error,
            ),
            (
                XPathFactKind::UnknownNamespacePrefix,
                "cem.xpath.unknown_namespace_prefix",
                crate::diagnostics::Severity::Error,
            ),
            (
                XPathFactKind::UnclosedDelimiter,
                "cem.xpath.unclosed_delimiter",
                crate::diagnostics::Severity::Error,
            ),
            (
                XPathFactKind::MismatchedDelimiter,
                "cem.xpath.mismatched_delimiter",
                crate::diagnostics::Severity::Error,
            ),
            (
                XPathFactKind::HostAssociationInvalid,
                "cem.xpath.host_association_invalid",
                crate::diagnostics::Severity::Error,
            ),
            (
                XPathFactKind::ExternalResourceDenied,
                "cem.xpath.external_resource_denied",
                crate::diagnostics::Severity::Error,
            ),
            (
                XPathFactKind::SourceMapUnavailable,
                "cem.xpath.source_map_unavailable",
                crate::diagnostics::Severity::Info,
            ),
            (
                XPathFactKind::EventLifecycleInvalid,
                "cem.xpath.event_lifecycle_invalid",
                crate::diagnostics::Severity::Error,
            ),
        ] {
            let binding = catalog
                .binding_for_fact(kind)
                .unwrap_or_else(|| panic!("schema binding for {}", kind.as_str()));
            assert_eq!(binding.diagnostic_code, code);
            assert_eq!(binding.severity, severity);
            assert_eq!(binding.behavior.as_deref(), Some("xpath-report-fact"));
        }
    }

    #[test]
    fn xpath_unknown_prefix_diagnostic_is_schema_declared() {
        let source = builtin_schema_package_source(XPATH_PACKAGE_ID)
            .expect("XPath package source")
            .schema_source
            .replace(
                r#"{constraint @kind="xpath-static-namespace" @target="static-context" @diagnostic="cem.xpath.unknown_namespace_prefix" @behavior="xpath-report-fact" @fact-kind="unknown-namespace-prefix" @policy="prefixed names resolve through the declared static context"}"#,
                r#"{constraint @kind="xpath-static-namespace" @target="static-context" @diagnostic="example.xpath.unknown_prefix" @behavior="xpath-report-fact" @fact-kind="unknown-namespace-prefix" @policy="prefixed names resolve through the declared static context"}"#,
            )
            .replace(
                r#"{diagnostic @code="cem.xpath.unknown_namespace_prefix" @severity="error"}"#,
                r#"{diagnostic @code="example.xpath.unknown_prefix" @severity="warning"}"#,
            );
        let contracts = XPathSchemaContractCatalog::from_schema_source(&source);
        let ast = parse("/catalog/ns:book");
        let diagnostics = validate_xpath_expression_ast(&ast, &contracts);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "example.xpath.unknown_prefix");
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert_eq!(
            diagnostics[0].details.as_ref().unwrap()["xpath"]["contract"],
            "xpath-static-namespace"
        );
    }

    #[test]
    fn xpath_source_diagnostics_preserve_exact_ranges_maps_and_schema_details() {
        let source = "/catalog/ns:book";
        let diagnostics = validate_xpath_source_bytes(XPathSourceRequest {
            bytes: source.as_bytes(),
            source_uri: "memory://unknown-prefix.xpath",
            content_type: Some("text/xpath; charset=utf-8"),
            source_range_projector: None,
        });
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "cem.xpath.unknown_namespace_prefix")
            .expect("unknown namespace diagnostic");

        assert_eq!(
            diagnostic.uri.as_deref(),
            Some("memory://unknown-prefix.xpath")
        );
        assert_eq!(diagnostic.line, Some(1));
        assert_eq!(diagnostic.column, Some(10));
        assert_eq!(diagnostic.byte_offset, Some(9));
        assert!(diagnostic.source_map.as_ref().is_some_and(|source_map| {
            source_map.frames.iter().any(|frame| {
                frame.source_id == SourceId(1)
                    && matches!(frame.span, FrameSpan::Single(range) if range.start == 9)
            })
        }));
        assert_eq!(
            diagnostic.details.as_ref().unwrap()["xpath"]["factKind"],
            "unknown-namespace-prefix"
        );
        assert_eq!(
            diagnostic.details.as_ref().unwrap()["xpath"]["contract"],
            "xpath-static-namespace"
        );
    }

    #[test]
    fn xpath_validation_denies_external_resource_functions_without_resolver_policy() {
        let diagnostics = validate_xpath_source_bytes(XPathSourceRequest {
            bytes: b"doc(\"catalog.xml\")/catalog",
            source_uri: "memory://external.xpath",
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        });

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.xpath.external_resource_denied"
                && diagnostic.byte_offset == Some(0)
        }));
    }

    #[test]
    fn xpath_validation_reports_invalid_host_association_from_schema_binding() {
        let expression = "item/@id";
        let attachment = XPathAttachment::Host(XPathHostAttachment {
            owner: XPathHostOwner {
                source_id: 11,
                source_uri: "memory://stylesheet.xsl".to_owned(),
                content_type: Some("application/xslt+xml".to_owned()),
                schema_uri: Some("https://cem.dev/ns/transform/xslt/1".to_owned()),
                node_kind: XPathHostNodeKind::XsltAttribute,
                node_id: Some("event:4@select".to_owned()),
                source_range: XPathSourceRange::new(3, 7, 34, 19),
            },
            expression_range: XPathSourceRange::new(3, 15, 42, 2),
            static_context: XPathStaticContext::default(),
            expected_result: None,
            evaluation_phase: XPathEvaluationPhase::Transform,
            resolver_policy_stamp: None,
            safety_policy_stamp: None,
        });
        let ast = xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: expression.as_bytes(),
                source_uri: "memory://stylesheet.xsl",
                content_type: Some(XPATH_CONTENT_TYPE),
                source_range_projector: None,
            },
            attachment,
        );
        let diagnostics =
            validate_xpath_expression_ast(&ast, &XPathSchemaContractCatalog::from_builtin());

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.xpath.host_association_invalid"
                && diagnostic.byte_offset == Some(42)
        }));
    }

    #[test]
    fn xpath_ast_reports_invalid_utf8_without_synthesizing_tokens() {
        let ast = xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: &[b'/', 0xff, b'x'],
                source_uri: "memory://invalid.xpath",
                content_type: Some(XPATH_CONTENT_TYPE),
                source_range_projector: None,
            },
            XPathAttachment::Standalone { source_id: 3 },
        );

        assert!(ast.source_text.is_none());
        assert!(ast.tokens.is_empty());
        assert!(ast.events.is_empty());
        assert!(ast.syntax_ast.is_none());
        assert!(ast
            .facts
            .iter()
            .any(|fact| fact.kind == XPathFactKind::InvalidUtf8));
    }

    #[test]
    fn xpath_public_syntax_contract_has_no_foreign_or_json_representation_dependency() {
        let source = include_str!("xpath/syntax.rs");
        for forbidden in ["xee_", "serde_json", "serde::", "use serde"] {
            assert!(
                !source.contains(forbidden),
                "public XPath syntax contract must not contain `{forbidden}`"
            );
        }
    }

    #[test]
    fn xpath_runtime_scanner_and_parser_have_no_xee_dependency() {
        let manifest = include_str!("../../Cargo.toml");
        let runtime_manifest = manifest
            .split("[dev-dependencies]")
            .next()
            .expect("Cargo manifest runtime sections");
        assert!(
            !runtime_manifest.contains("xee-xpath"),
            "Xee crates must remain outside runtime dependencies"
        );

        for (label, source) in [
            ("scanner", include_str!("xpath/lexer.rs")),
            ("parser", include_str!("xpath/parser.rs")),
            ("syntax AST", include_str!("xpath/syntax.rs")),
        ] {
            assert!(
                !source.contains("xee_"),
                "runtime XPath {label} must not reference Xee"
            );
        }

        let module = include_str!("xpath.rs");
        let lifecycle_entry = module
            .split("pub fn xpath_expression_ast_from_source_bytes")
            .nth(1)
            .and_then(|source| source.split("fn xpath_host_attachment_facts").next())
            .expect("XPath lifecycle production entry");
        assert!(
            !lifecycle_entry.contains("xee_"),
            "XPath lifecycle production entry must not reference Xee"
        );
    }
}
