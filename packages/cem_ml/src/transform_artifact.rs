use crate::engine::FormatIdentity;
use crate::interpreter::OutputSpan;
use crate::lifecycle::LoadedInputAstStream;
use crate::parser::document::CemDocument;
use crate::projection::{
    CemTreeAstAttribute, CemTreeAstNode, CemTreeAstStream, CemTreeAstWriterTokenMetadata,
    CemTreeAstWriterTokenSourceRange, CemTreeAstWriterTokenStyle, DomJsonProjectionRef,
    EventsJsonProjectionRef, NormalizedEventStream,
};
use crate::schema::registry::{
    content_type_essence, CEM_DOM_JSON_PROJECTION_CONTENT_TYPE, CEM_DOM_PROJECTION_SCHEMA_URI,
    CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE, CEM_EVENTS_PROJECTION_SCHEMA_URI, CSS_CONTENT_TYPE,
    CSS_SCHEMA_URI, HTML_CONTENT_TYPE, HTML_SCHEMA_URI, JSON_CONTENT_TYPE,
    JSON_SCHEMA_CONTENT_TYPE, JSON_SCHEMA_SCHEMA_URI, MARKDOWN_CONTENT_TYPE, MARKDOWN_SCHEMA_URI,
    MATHML_NAMESPACE_URI, MATHML_SCHEMA_URI, RELAX_NG_SCHEMA_URI, SVG_CONTENT_TYPE,
    SVG_NAMESPACE_URI, SVG_SCHEMA_URI, XHTML_CONTENT_TYPE, XHTML_SCHEMA_URI, XML_CONTENT_TYPE,
    XML_SCHEMA_URI, XPATH_RESULT_CONTENT_TYPE, XPATH_SCHEMA_URI, XSLT_NAMESPACE_URI,
    XSLT_SCHEMA_URI, YAML_CONTENT_TYPE, YAML_SCHEMA_URI,
};
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::validation::css::{
    CssDocumentAst, CssDocumentSource, CssEncodingReportAst, CssEventAst, CssFact,
};
use crate::validation::csv::{
    CsvDialectAst, CsvDocumentAst, CsvDocumentParseFact, CsvDocumentSource, CsvEncodingReportAst,
    CsvFieldAst, CsvRecordAst, CsvSourceRange,
};
use crate::validation::generic_data::{
    GenericDataDocumentAst, GenericDataMappingEntryAst, GenericDataSourceRangeAst,
    GenericDataStreamDocumentAst, GenericDataValueAst,
};
use crate::validation::html::{
    HtmlAttributeAst, HtmlDocumentAst, HtmlDocumentSource, HtmlEncodingReportAst, HtmlEventAst,
    HtmlFact,
};
use crate::validation::json::{
    JsonDocumentAst, JsonMemberAst, JsonNumberKind, JsonSourceRange, JsonValueAst,
};
use crate::validation::json_schema::{
    json_schema_source_map, JsonSchemaDialectFact, JsonSchemaDocumentAst, JsonSchemaDocumentSource,
    JsonSchemaParseFact,
};
use crate::validation::markdown::{
    MarkdownDocumentAst, MarkdownDocumentSource, MarkdownEncodingFact, MarkdownEncodingReportAst,
    MarkdownEventAst, MarkdownParseFact, MarkdownSourceRange, MarkdownVariantFact,
};
use crate::validation::mathml::{MathMlDocumentAst, MathMlFact};
use crate::validation::relax_ng::{
    RelaxNgCompactTokenAst, RelaxNgDocumentAst, RelaxNgDocumentSource, RelaxNgFact,
};
use crate::validation::svg::{SvgDocumentAst, SvgDocumentSource, SvgFact};
use crate::validation::xhtml::{XhtmlDocumentAst, XhtmlDocumentSource, XhtmlFact};
use crate::validation::xml::{
    xml_event_markup_tokens, XmlAttributeAst, XmlDocumentAst, XmlDocumentSource,
    XmlEncodingReportAst, XmlEventAst, XmlEventKind, XmlMarkupTokenAst, XmlMarkupTokenKind,
    XmlParseFact, XmlSourceRange,
};
use crate::validation::xpath::XPathResultArtifact;
use crate::validation::xslt::{XsltFact, XsltStylesheetAst};
use crate::validation::yaml::{
    YamlCommentAst, YamlCommentPlacement, YamlDirectiveAst, YamlDocumentAst, YamlDocumentParseFact,
    YamlDocumentSource, YamlEncodingReportAst, YamlNodeAst, YamlNodeKind, YamlPairAst,
    YamlSourceRange, YamlStreamDocumentAst,
};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

pub const CEMT_TREE_REPRESENTATION_ID: &str = "cem.cemt-tree";
pub const CEMT_MATERIALIZED_TREE_REPRESENTATION_ID: &str = "cem.cemt-materialized-tree";
pub const DOM_PROJECTION_REPRESENTATION_ID: &str = "cem.dom-projection";
pub const EVENT_STREAM_REPRESENTATION_ID: &str = "cem.event-stream";
pub const XPATH_RESULT_REPRESENTATION_ID: &str = "cem.xpath-result";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CemtTreeArtifactStage {
    Raw,
    Formatted,
    Colored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemtOverlayProducer {
    pub function_name: String,
    pub formatter_profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemtTreeEnvelopeMetadata {
    pub content_type: String,
    pub schema: String,
    pub category: String,
    pub mode: CemtTreeEnvelopeMode,
    pub canonical: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CemtTreeEnvelopeMode {
    Document,
    Fragment,
}

impl CemtTreeEnvelopeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Fragment => "fragment",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CemtOverlayProvenance {
    SourceMapped(SourceMapStack),
    Generated {
        function_name: String,
        source_map: Option<SourceMapStack>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CemtFormatOperationKind {
    Marker,
    Decision { value: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemtFormatOperation {
    pub name: String,
    pub formatter_role: String,
    pub formatter_profile: Option<String>,
    pub color_role: Option<String>,
    pub kind: CemtFormatOperationKind,
    pub provenance: CemtOverlayProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CemtOwnerPath {
    root: usize,
    steps: Vec<CemtOwnerStep>,
}

impl CemtOwnerPath {
    pub fn root(root: usize) -> Self {
        Self {
            root,
            steps: Vec::new(),
        }
    }

    pub fn child(&self, index: usize) -> Self {
        let mut path = self.clone();
        path.steps.push(CemtOwnerStep::Child(index));
        path
    }

    pub fn attribute(&self, index: usize) -> Self {
        let mut path = self.clone();
        path.steps.push(CemtOwnerStep::Attribute(index));
        path
    }

    pub fn root_index(&self) -> usize {
        self.root
    }

    pub fn steps(&self) -> &[CemtOwnerStep] {
        &self.steps
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CemtOwnerStep {
    Child(usize),
    Attribute(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CemtTreeOwnerRef<'a> {
    Node(&'a CemTreeAstNode),
    Attribute(&'a CemTreeAstAttribute),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CemtFormatLayout {
    Inline,
    InlineEmphasis,
    Block,
}

impl CemtFormatLayout {
    fn as_str(self) -> &'static str {
        match self {
            Self::Inline => "inline",
            Self::InlineEmphasis => "inline-emphasis",
            Self::Block => "block",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CemtFormatFragment {
    Whitespace { value: String },
    Raw { value: String },
}

impl CemtFormatFragment {
    fn kind(&self) -> &'static str {
        match self {
            Self::Whitespace { .. } => "whitespace",
            Self::Raw { .. } => "raw",
        }
    }

    fn value(&self) -> &str {
        match self {
            Self::Whitespace { value } | Self::Raw { value } => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CemtNodeFormatTarget {
    Owner(CemtOwnerPath),
    RootGap {
        before_root: usize,
        ordinal: usize,
    },
    ChildGap {
        parent: CemtOwnerPath,
        before_child: usize,
        ordinal: usize,
    },
}

impl CemtNodeFormatTarget {
    fn belongs_to(&self, path: &CemtOwnerPath) -> bool {
        match self {
            Self::Owner(owner) | Self::ChildGap { parent: owner, .. } => owner == path,
            Self::RootGap { .. } => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CemtNodeFormatOperationKind {
    Layout(CemtFormatLayout),
    BeforeAttributes {
        fragment: CemtFormatFragment,
    },
    BetweenAttributes {
        fragment: CemtFormatFragment,
    },
    BeforeAttribute {
        index: usize,
        fragment: CemtFormatFragment,
    },
    ContentBoundary {
        index: usize,
        fragment: CemtFormatFragment,
    },
    InsertChild {
        fragment: CemtFormatFragment,
    },
    BeforeClose {
        fragment: CemtFormatFragment,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemtNodeFormatOperation {
    pub target: CemtNodeFormatTarget,
    pub producer_function: String,
    pub formatter_role: String,
    pub color_role: Option<String>,
    pub kind: CemtNodeFormatOperationKind,
    pub provenance: CemtOverlayProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemtFormattedTreeOverlay {
    pub envelope: CemtTreeEnvelopeMetadata,
    pub producer: CemtOverlayProducer,
    pub operations: Vec<CemtFormatOperation>,
    pub(crate) retained_node_paths: Vec<CemtOwnerPath>,
    pub node_operations: Vec<CemtNodeFormatOperation>,
}

impl CemtFormattedTreeOverlay {
    pub fn retained_node_paths(&self) -> &[CemtOwnerPath] {
        &self.retained_node_paths
    }

    pub fn retains_node(&self, path: &CemtOwnerPath) -> bool {
        self.retained_node_paths.contains(path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemtColorOverlayProducer {
    pub function_name: String,
    pub color_profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CemtColorOperationKind {
    Marker,
    Decision { value: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemtColorOperation {
    pub name: String,
    pub colorizer_role: String,
    pub color_profile: Option<String>,
    pub color_role: Option<String>,
    pub kind: CemtColorOperationKind,
    pub provenance: CemtOverlayProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemtWriterBoundary {
    pub stage: String,
    pub value: Option<String>,
    pub provenance: CemtOverlayProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CemtColorOutput {
    Terminal,
    Html,
    Markdown,
}

impl CemtColorOutput {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Html => "html",
            Self::Markdown => "md",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemtColorStyle {
    pub color_role: String,
    pub color_profile: String,
    pub output: Option<CemtColorOutput>,
    pub terminal_capability: Option<String>,
    pub html_mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CemtColorTarget {
    Owner(CemtOwnerPath),
    FormatOperation(usize),
    NodeFormatOperation(usize),
}

impl CemtColorTarget {
    fn belongs_to(
        &self,
        path: &CemtOwnerPath,
        format_operations: &[CemtNodeFormatOperation],
    ) -> bool {
        match self {
            Self::Owner(owner) => owner == path,
            Self::NodeFormatOperation(index) => format_operations
                .get(*index)
                .is_some_and(|operation| operation.target.belongs_to(path)),
            Self::FormatOperation(_) => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CemtNodeColorOperationKind {
    Role {
        role: String,
        style: Option<CemtColorStyle>,
    },
    WriterAttribute {
        name: String,
        value: String,
        colorizer_role: String,
        color_profile: String,
        color_role: Option<String>,
        style: Option<CemtColorStyle>,
    },
    Wrapper {
        name: String,
        colorizer_role: String,
        color_profile: String,
        color_role: Option<String>,
        style: Option<CemtColorStyle>,
    },
    WrapperDecision {
        name: String,
        value: String,
        colorizer_role: String,
        color_profile: String,
        color_role: Option<String>,
        style: Option<CemtColorStyle>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemtNodeColorOperation {
    pub target: CemtColorTarget,
    pub producer_function: String,
    pub kind: CemtNodeColorOperationKind,
    pub provenance: CemtOverlayProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemtColoredTreeOverlay {
    pub producer: CemtColorOverlayProducer,
    pub operations: Vec<CemtColorOperation>,
    pub writer_boundaries: Vec<CemtWriterBoundary>,
    pub node_operations: Vec<CemtNodeColorOperation>,
}

#[derive(Debug, Clone)]
pub struct CemtTreeArtifact {
    stage: CemtTreeArtifactStage,
    owner: Arc<CemTreeAstStream>,
    source_map: Option<SourceMapStack>,
    formatted_overlay: Option<CemtFormattedTreeOverlay>,
    colored_overlay: Option<CemtColoredTreeOverlay>,
}

impl CemtTreeArtifact {
    pub fn raw(owner: Arc<CemTreeAstStream>, source_map: Option<SourceMapStack>) -> Self {
        Self {
            stage: CemtTreeArtifactStage::Raw,
            owner,
            source_map,
            formatted_overlay: None,
            colored_overlay: None,
        }
    }

    pub fn formatted(
        owner: Arc<CemTreeAstStream>,
        source_map: Option<SourceMapStack>,
        overlay: CemtFormattedTreeOverlay,
    ) -> Self {
        Self {
            stage: CemtTreeArtifactStage::Formatted,
            owner,
            source_map,
            formatted_overlay: Some(overlay),
            colored_overlay: None,
        }
    }

    pub fn colored(
        owner: Arc<CemTreeAstStream>,
        source_map: Option<SourceMapStack>,
        formatted_overlay: CemtFormattedTreeOverlay,
        colored_overlay: CemtColoredTreeOverlay,
    ) -> Self {
        Self {
            stage: CemtTreeArtifactStage::Colored,
            owner,
            source_map,
            formatted_overlay: Some(formatted_overlay),
            colored_overlay: Some(colored_overlay),
        }
    }

    pub fn stage(&self) -> CemtTreeArtifactStage {
        self.stage
    }

    pub fn owner(&self) -> &Arc<CemTreeAstStream> {
        &self.owner
    }

    pub fn subject(&self) -> CemtTreeSubjectRef<'_> {
        CemtTreeSubjectRef {
            owner: self.owner.as_ref(),
        }
    }

    pub fn formatted_overlay(&self) -> Option<&CemtFormattedTreeOverlay> {
        self.formatted_overlay.as_ref()
    }

    pub fn colored_overlay(&self) -> Option<&CemtColoredTreeOverlay> {
        self.colored_overlay.as_ref()
    }

    pub fn formatted_view(&self) -> Option<CemtFormattedTreeView<'_>> {
        self.formatted_overlay
            .as_ref()
            .map(|overlay| CemtFormattedTreeView {
                subject: self.subject(),
                overlay,
            })
    }

    pub fn colored_view(&self) -> Option<CemtColoredTreeView<'_>> {
        Some(CemtColoredTreeView {
            subject: self.subject(),
            formatted_overlay: self.formatted_overlay.as_ref()?,
            colored_overlay: self.colored_overlay.as_ref()?,
        })
    }

    pub fn evaluator_view(&self) -> CemtEvaluatorValueRef<'_> {
        match self.formatted_overlay.as_ref() {
            Some(overlay) => CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::FormattedTree {
                subject: self.subject(),
                overlay,
            }),
            None => self.subject().evaluator_view(),
        }
    }

    /// Projects the typed tree and its overlays at an explicit public/debug
    /// boundary. Runtime stages retain `CemtTreeArtifact` ownership and must
    /// not consume this JSON projection.
    pub fn to_public_json(&self) -> Result<serde_json::Value, String> {
        let mut value = CemtEvaluatorValue::borrowed(self.evaluator_view()).to_public_json()?;
        if let Some(overlay) = self.colored_overlay.as_ref() {
            let formatted_overlay = self
                .formatted_overlay
                .as_ref()
                .ok_or_else(|| "typed colored CEM-tree has no formatted overlay".to_owned())?;
            project_cemt_colored_overlay_to_public_json(&mut value, formatted_overlay, overlay)?;
        }
        Ok(value)
    }
}

fn project_cemt_colored_overlay_to_public_json(
    tree: &mut serde_json::Value,
    formatted_overlay: &CemtFormattedTreeOverlay,
    overlay: &CemtColoredTreeOverlay,
) -> Result<(), String> {
    let fields = tree
        .as_object_mut()
        .ok_or_else(|| "typed colored CEM-tree public projection must be an object".to_owned())?;
    fields.insert("colored".to_owned(), serde_json::Value::Bool(true));
    if let Some(profile) = overlay.producer.color_profile.as_ref() {
        fields.insert(
            "colorProfile".to_owned(),
            serde_json::Value::String(profile.clone()),
        );
    }
    if let Some(style) = overlay
        .node_operations
        .iter()
        .find_map(cemt_node_color_operation_style)
    {
        if let Some(output) = style.output {
            fields.insert(
                "colorOutput".to_owned(),
                serde_json::Value::String(output.as_str().to_owned()),
            );
        }
        if let Some(capability) = style.terminal_capability.as_ref() {
            fields.insert(
                "terminalCapability".to_owned(),
                serde_json::Value::String(capability.clone()),
            );
        }
        if let Some(mode) = style.html_mode.as_ref() {
            fields.insert(
                "htmlMode".to_owned(),
                serde_json::Value::String(mode.clone()),
            );
        }
    }
    fields.insert(
        "colorNodes".to_owned(),
        serde_json::Value::Array(
            overlay
                .operations
                .iter()
                .map(cemt_public_color_operation)
                .collect::<Result<Vec<_>, _>>()?,
        ),
    );
    if !overlay.writer_boundaries.is_empty() {
        fields.insert(
            "writerBoundaries".to_owned(),
            serde_json::Value::Array(
                overlay
                    .writer_boundaries
                    .iter()
                    .map(cemt_public_writer_boundary)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        );
    }

    let mut owner_paths = Vec::new();
    for operation in &overlay.node_operations {
        if let CemtColorTarget::Owner(path) = &operation.target {
            if !owner_paths.contains(path) {
                owner_paths.push(path.clone());
            }
        }
    }
    owner_paths.sort_by_key(|path| std::cmp::Reverse(path.steps().len()));
    for path in owner_paths {
        let operations = overlay
            .node_operations
            .iter()
            .enumerate()
            .filter(|(_, operation)| operation.target == CemtColorTarget::Owner(path.clone()))
            .collect::<Vec<_>>();
        if !cemt_public_owner_is_retained(formatted_overlay, &path) {
            continue;
        }
        let target =
            cemt_public_owner_value_mut(tree, formatted_overlay, &path).ok_or_else(|| {
                format!(
                    "typed colored CEM-tree public projection could not resolve owner {}",
                    cemt_owner_path_label(&path)
                )
            })?;
        project_cemt_owner_color_operations(target, &operations)?;
    }

    for operation in &overlay.node_operations {
        match operation.target {
            CemtColorTarget::FormatOperation(index) => {
                let Some(target) = tree
                    .get_mut("formatNodes")
                    .and_then(serde_json::Value::as_array_mut)
                    .and_then(|operations| operations.get_mut(index))
                else {
                    continue;
                };
                project_cemt_color_metadata(target, operation)?;
            }
            CemtColorTarget::NodeFormatOperation(index) => {
                let Some(target) =
                    cemt_public_node_format_operation_value_mut(tree, formatted_overlay, index)
                else {
                    continue;
                };
                project_cemt_color_metadata(target, operation)?;
            }
            CemtColorTarget::Owner(_) => {}
        }
    }
    Ok(())
}

fn cemt_public_owner_value_mut<'a>(
    tree: &'a mut serde_json::Value,
    overlay: &CemtFormattedTreeOverlay,
    path: &CemtOwnerPath,
) -> Option<&'a mut serde_json::Value> {
    let root = cemt_public_formatted_node_index(overlay, None, path.root_index());
    let mut current = tree.get_mut("nodes")?.as_array_mut()?.get_mut(root)?;
    let mut owner_path = CemtOwnerPath::root(path.root_index());
    for step in path.steps() {
        current = match step {
            CemtOwnerStep::Child(index) => {
                let child = cemt_public_formatted_node_index(overlay, Some(&owner_path), *index);
                owner_path = owner_path.child(*index);
                current
                    .get_mut("children")?
                    .as_array_mut()?
                    .get_mut(child)?
            }
            CemtOwnerStep::Attribute(index) => current
                .get_mut("attributes")?
                .as_array_mut()?
                .get_mut(*index)?,
        };
    }
    Some(current)
}

fn cemt_public_owner_is_retained(overlay: &CemtFormattedTreeOverlay, path: &CemtOwnerPath) -> bool {
    let mut node_path = path.clone();
    if matches!(node_path.steps.last(), Some(CemtOwnerStep::Attribute(_))) {
        node_path.steps.pop();
    }
    overlay.retains_node(&node_path)
}

fn cemt_public_formatted_node_index(
    overlay: &CemtFormattedTreeOverlay,
    parent: Option<&CemtOwnerPath>,
    original_index: usize,
) -> usize {
    let inserted = (0..=original_index)
        .map(|before_node| {
            overlay
                .node_operations
                .iter()
                .filter(|operation| {
                    cemt_evaluator_gap_operation_matches(operation, parent, before_node)
                })
                .count()
        })
        .sum::<usize>();
    let retained = (0..original_index)
        .filter(|index| {
            let path = match parent {
                Some(parent) => parent.child(*index),
                None => CemtOwnerPath::root(*index),
            };
            overlay.retains_node(&path)
        })
        .count();
    inserted.saturating_add(retained)
}

fn cemt_public_node_format_operation_value_mut<'a>(
    tree: &'a mut serde_json::Value,
    overlay: &CemtFormattedTreeOverlay,
    operation_index: usize,
) -> Option<&'a mut serde_json::Value> {
    let operation = overlay.node_operations.get(operation_index)?;
    match (&operation.target, &operation.kind) {
        (CemtNodeFormatTarget::Owner(path), kind) => {
            if !cemt_public_owner_is_retained(overlay, path) {
                return None;
            }
            let owner = cemt_public_owner_value_mut(tree, overlay, path)?;
            let field = match kind {
                CemtNodeFormatOperationKind::Layout(_) => "formatLayout",
                CemtNodeFormatOperationKind::BeforeAttributes { .. } => "formatBeforeAttributes",
                CemtNodeFormatOperationKind::BetweenAttributes { .. } => "formatBetweenAttributes",
                CemtNodeFormatOperationKind::BeforeAttribute { .. } => "formatBefore",
                CemtNodeFormatOperationKind::ContentBoundary { .. } => "formatContentBoundary",
                CemtNodeFormatOperationKind::BeforeClose { .. } => "formatBeforeClose",
                CemtNodeFormatOperationKind::InsertChild { .. } => return None,
            };
            let value = owner.get_mut(field)?;
            let ordinal = overlay.node_operations[..operation_index]
                .iter()
                .filter(|candidate| {
                    candidate.target == operation.target
                        && std::mem::discriminant(&candidate.kind)
                            == std::mem::discriminant(&operation.kind)
                })
                .count();
            match value {
                serde_json::Value::Array(values) => values.get_mut(ordinal),
                value if ordinal == 0 => Some(value),
                _ => None,
            }
        }
        (
            CemtNodeFormatTarget::RootGap {
                before_root,
                ordinal,
            },
            CemtNodeFormatOperationKind::InsertChild { .. },
        ) => {
            let index = cemt_public_gap_operation_index(overlay, None, *before_root, *ordinal);
            tree.get_mut("nodes")?.as_array_mut()?.get_mut(index)
        }
        (
            CemtNodeFormatTarget::ChildGap {
                parent,
                before_child,
                ordinal,
            },
            CemtNodeFormatOperationKind::InsertChild { .. },
        ) => {
            if !cemt_public_owner_is_retained(overlay, parent) {
                return None;
            }
            let parent_value = cemt_public_owner_value_mut(tree, overlay, parent)?;
            let index =
                cemt_public_gap_operation_index(overlay, Some(parent), *before_child, *ordinal);
            parent_value
                .get_mut("children")?
                .as_array_mut()?
                .get_mut(index)
        }
        _ => None,
    }
}

fn cemt_public_gap_operation_index(
    overlay: &CemtFormattedTreeOverlay,
    parent: Option<&CemtOwnerPath>,
    before_node: usize,
    ordinal: usize,
) -> usize {
    let preceding_insertions = (0..before_node)
        .map(|index| {
            overlay
                .node_operations
                .iter()
                .filter(|operation| cemt_evaluator_gap_operation_matches(operation, parent, index))
                .count()
        })
        .sum::<usize>();
    let retained = (0..before_node)
        .filter(|index| {
            let path = match parent {
                Some(parent) => parent.child(*index),
                None => CemtOwnerPath::root(*index),
            };
            overlay.retains_node(&path)
        })
        .count();
    preceding_insertions
        .saturating_add(retained)
        .saturating_add(ordinal)
}

fn cemt_node_color_operation_style(operation: &CemtNodeColorOperation) -> Option<&CemtColorStyle> {
    match &operation.kind {
        CemtNodeColorOperationKind::Role { style, .. }
        | CemtNodeColorOperationKind::WriterAttribute { style, .. }
        | CemtNodeColorOperationKind::Wrapper { style, .. }
        | CemtNodeColorOperationKind::WrapperDecision { style, .. } => style.as_ref(),
    }
}

fn project_cemt_owner_color_operations(
    target: &mut serde_json::Value,
    operations: &[(usize, &CemtNodeColorOperation)],
) -> Result<(), String> {
    let wrapper_index = operations.iter().find_map(|(index, operation)| {
        matches!(operation.kind, CemtNodeColorOperationKind::Wrapper { .. }).then_some(*index)
    });
    let Some(wrapper_index) = wrapper_index else {
        for (_, operation) in operations {
            project_cemt_color_metadata(target, operation)?;
        }
        return Ok(());
    };

    let mut source = std::mem::take(target);
    let wrapper_name = operations
        .iter()
        .find_map(|(_, operation)| match &operation.kind {
            CemtNodeColorOperationKind::Wrapper { name, .. } => Some(name.clone()),
            _ => None,
        })
        .ok_or_else(|| "typed colored CEM-tree wrapper operation is missing a name".to_owned())?;
    let mut wrapper = serde_json::json!({
        "kind": "element",
        "name": wrapper_name,
        "attributes": [],
        "children": [],
        "colorWrapperNodes": []
    });
    for (index, operation) in operations {
        match &operation.kind {
            CemtNodeColorOperationKind::Wrapper { .. }
            | CemtNodeColorOperationKind::WrapperDecision { .. } => {
                wrapper
                    .get_mut("colorWrapperNodes")
                    .and_then(serde_json::Value::as_array_mut)
                    .expect("typed wrapper projection owns a metadata array")
                    .push(cemt_public_wrapper_operation(operation)?);
            }
            _ if *index < wrapper_index => {
                project_cemt_color_metadata(&mut wrapper, operation)?;
            }
            _ => project_cemt_color_metadata(&mut source, operation)?,
        }
    }
    wrapper
        .get_mut("children")
        .and_then(serde_json::Value::as_array_mut)
        .expect("typed wrapper projection owns a children array")
        .push(source);
    *target = wrapper;
    Ok(())
}

fn project_cemt_color_metadata(
    target: &mut serde_json::Value,
    operation: &CemtNodeColorOperation,
) -> Result<(), String> {
    let fields = target.as_object_mut().ok_or_else(|| {
        "typed colored CEM-tree public projection target must be an object".to_owned()
    })?;
    match &operation.kind {
        CemtNodeColorOperationKind::Role { role, style } => {
            fields.insert(
                "colorRole".to_owned(),
                serde_json::Value::String(role.clone()),
            );
            if let Some(style) = style {
                fields.insert("style".to_owned(), cemt_public_color_style(style));
            }
        }
        CemtNodeColorOperationKind::WriterAttribute {
            name,
            value,
            colorizer_role,
            color_profile,
            color_role,
            style,
        } => {
            let mut attribute = serde_json::json!({
                "kind": "writer-attribute",
                "name": name,
                "value": value,
                "colorizerOwned": true,
                "colorizerRole": colorizer_role,
                "colorProfile": color_profile,
            });
            if let Some(role) = color_role {
                attribute["colorRole"] = serde_json::Value::String(role.clone());
            }
            if let Some(style) = style {
                attribute["style"] = cemt_public_color_style(style);
            }
            cemt_public_insert_provenance(&mut attribute, &operation.provenance)?;
            fields
                .entry("writerAttributeNodes".to_owned())
                .or_insert_with(|| serde_json::Value::Array(Vec::new()))
                .as_array_mut()
                .expect("typed writer attribute projection owns an array")
                .push(attribute);
        }
        CemtNodeColorOperationKind::Wrapper { .. }
        | CemtNodeColorOperationKind::WrapperDecision { .. } => {}
    }
    Ok(())
}

fn cemt_public_color_operation(
    operation: &CemtColorOperation,
) -> Result<serde_json::Value, String> {
    let mut value = serde_json::json!({
        "kind": match operation.kind {
            CemtColorOperationKind::Marker => "color-marker",
            CemtColorOperationKind::Decision { .. } => "color-decision",
        },
        "name": operation.name,
        "colorizerRole": operation.colorizer_role,
    });
    if let Some(profile) = operation.color_profile.as_ref() {
        value["colorProfile"] = serde_json::Value::String(profile.clone());
    }
    if let Some(role) = operation.color_role.as_ref() {
        value["colorRole"] = serde_json::Value::String(role.clone());
    }
    if let CemtColorOperationKind::Decision { value: decision } = &operation.kind {
        value["value"] = serde_json::Value::String(decision.clone());
    }
    cemt_public_insert_provenance(&mut value, &operation.provenance)?;
    Ok(value)
}

fn cemt_public_writer_boundary(
    boundary: &CemtWriterBoundary,
) -> Result<serde_json::Value, String> {
    let mut value = serde_json::json!({
        "kind": "writer-boundary",
        "stage": boundary.stage,
    });
    if let Some(decision) = boundary.value.as_ref() {
        value["value"] = serde_json::Value::String(decision.clone());
    }
    cemt_public_insert_provenance(&mut value, &boundary.provenance)?;
    Ok(value)
}

fn cemt_public_wrapper_operation(
    operation: &CemtNodeColorOperation,
) -> Result<serde_json::Value, String> {
    let mut value = match &operation.kind {
        CemtNodeColorOperationKind::Wrapper {
            name,
            colorizer_role,
            color_profile,
            color_role,
            style,
        } => serde_json::json!({
            "kind": "color-wrapper",
            "name": name,
            "colorizerOwned": true,
            "colorizerRole": colorizer_role,
            "colorProfile": color_profile,
            "colorRole": color_role,
            "style": style.as_ref().map(cemt_public_color_style),
        }),
        CemtNodeColorOperationKind::WrapperDecision {
            name,
            value,
            colorizer_role,
            color_profile,
            color_role,
            style,
        } => serde_json::json!({
            "kind": "color-decision",
            "name": name,
            "value": value,
            "colorizerOwned": true,
            "colorizerRole": colorizer_role,
            "colorProfile": color_profile,
            "colorRole": color_role,
            "style": style.as_ref().map(cemt_public_color_style),
        }),
        _ => {
            return Err(
                "typed colored CEM-tree wrapper projection received a non-wrapper operation"
                    .to_owned(),
            )
        }
    };
    if let Some(fields) = value.as_object_mut() {
        fields.retain(|_, value| !value.is_null());
    }
    cemt_public_insert_provenance(&mut value, &operation.provenance)?;
    Ok(value)
}

fn cemt_public_color_style(style: &CemtColorStyle) -> serde_json::Value {
    let mut value = serde_json::json!({
        "colorRole": style.color_role,
        "colorProfile": style.color_profile,
        "colorOutput": style.output.map(CemtColorOutput::as_str),
        "terminalCapability": style.terminal_capability,
        "htmlMode": style.html_mode,
    });
    if let Some(fields) = value.as_object_mut() {
        fields.retain(|_, value| !value.is_null());
    }
    value
}

fn cemt_public_insert_provenance(
    value: &mut serde_json::Value,
    provenance: &CemtOverlayProvenance,
) -> Result<(), String> {
    let Some(source_map) = cemt_overlay_provenance_source_map(provenance) else {
        return Ok(());
    };
    value["sourceMap"] = serde_json::to_value(source_map).map_err(|error| error.to_string())?;
    Ok(())
}

impl TransformNativeArtifact for CemtTreeArtifact {
    fn representation_id(&self) -> &'static str {
        CEMT_TREE_REPRESENTATION_ID
    }

    fn source_map(&self) -> Option<&SourceMapStack> {
        self.source_map.as_ref()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CemtMaterializedTreeStage {
    Raw,
    Formatted,
    Colored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CemtMaterializedTreeProducerKind {
    Converter,
    Formatter,
    Colorizer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemtMaterializedTreeProducer {
    function_name: String,
    kind: CemtMaterializedTreeProducerKind,
    profile: Option<String>,
}

impl CemtMaterializedTreeProducer {
    pub fn converter(function_name: impl Into<String>) -> Self {
        Self::new(
            function_name,
            CemtMaterializedTreeProducerKind::Converter,
            None,
        )
    }

    pub fn formatter(function_name: impl Into<String>, profile: Option<String>) -> Self {
        Self::new(
            function_name,
            CemtMaterializedTreeProducerKind::Formatter,
            profile,
        )
    }

    pub fn colorizer(function_name: impl Into<String>, profile: Option<String>) -> Self {
        Self::new(
            function_name,
            CemtMaterializedTreeProducerKind::Colorizer,
            profile,
        )
    }

    fn new(
        function_name: impl Into<String>,
        kind: CemtMaterializedTreeProducerKind,
        profile: Option<String>,
    ) -> Self {
        Self {
            function_name: function_name.into(),
            kind,
            profile,
        }
    }

    pub fn function_name(&self) -> &str {
        &self.function_name
    }

    pub fn kind(&self) -> CemtMaterializedTreeProducerKind {
        self.kind
    }

    pub fn profile(&self) -> Option<&str> {
        self.profile.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemtMaterializedTreeIdentity {
    pub content_type: String,
    pub schema: String,
    pub category: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemtMaterializedTreeInputProvenance {
    pub representation_id: String,
    pub source_map: Option<SourceMapStack>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CemtMaterializedTreePipeline {
    Raw {
        producer: CemtMaterializedTreeProducer,
    },
    Formatted {
        formatter: CemtMaterializedTreeProducer,
    },
    Colored {
        formatter: CemtMaterializedTreeProducer,
        colorizer: CemtMaterializedTreeProducer,
    },
}

impl CemtMaterializedTreePipeline {
    pub fn stage(&self) -> CemtMaterializedTreeStage {
        match self {
            Self::Raw { .. } => CemtMaterializedTreeStage::Raw,
            Self::Formatted { .. } => CemtMaterializedTreeStage::Formatted,
            Self::Colored { .. } => CemtMaterializedTreeStage::Colored,
        }
    }

    fn validate(&self) -> Result<(), String> {
        let validate_kind =
            |producer: &CemtMaterializedTreeProducer,
             expected: CemtMaterializedTreeProducerKind| {
                (producer.kind() == expected).then_some(()).ok_or_else(|| {
                    format!(
                        "materialized CEMT tree stage requires a {expected:?} producer, but `{}` is {:?}",
                        producer.function_name(),
                        producer.kind()
                    )
                })
            };
        match self {
            Self::Raw { producer } => {
                validate_kind(producer, CemtMaterializedTreeProducerKind::Converter)
            }
            Self::Formatted { formatter } => {
                validate_kind(formatter, CemtMaterializedTreeProducerKind::Formatter)
            }
            Self::Colored {
                formatter,
                colorizer,
            } => {
                validate_kind(formatter, CemtMaterializedTreeProducerKind::Formatter)?;
                validate_kind(colorizer, CemtMaterializedTreeProducerKind::Colorizer)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemtMaterializedWriterTokenColor {
    pub target: CemtOwnerPath,
    pub color_role: String,
    pub style: CemTreeAstWriterTokenStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemtMaterializedTreeColorOverlay {
    pub producer: CemtMaterializedTreeProducer,
    pub output: CemtColorOutput,
    pub tokens: Vec<CemtMaterializedWriterTokenColor>,
}

impl CemtMaterializedTreeColorOverlay {
    fn validate(
        &self,
        pipeline: &CemtMaterializedTreePipeline,
        owner: &CemTreeAstStream,
    ) -> Result<(), String> {
        let CemtMaterializedTreePipeline::Colored { colorizer, .. } = pipeline else {
            return Err(
                "materialized CEMT color overlay requires the Colored pipeline stage".to_owned(),
            );
        };
        if self.producer.kind() != CemtMaterializedTreeProducerKind::Colorizer {
            return Err(format!(
                "materialized CEMT color overlay requires a Colorizer producer, but `{}` is {:?}",
                self.producer.function_name(),
                self.producer.kind()
            ));
        }
        if &self.producer != colorizer {
            return Err(format!(
                "materialized CEMT color overlay producer `{}` does not match pipeline colorizer `{}`",
                self.producer.function_name(),
                colorizer.function_name()
            ));
        }

        let subject = CemtTreeSubjectRef { owner };
        let mut seen_targets = Vec::with_capacity(self.tokens.len());
        for token in &self.tokens {
            let target = cemt_owner_path_label(&token.target);
            if seen_targets.contains(&token.target) {
                return Err(format!(
                    "materialized CEMT color overlay contains duplicate target {target}"
                ));
            }
            seen_targets.push(token.target.clone());
            if token.color_role.trim().is_empty() {
                return Err(format!(
                    "materialized CEMT color overlay target {target} has an empty color role"
                ));
            }
            if !matches!(
                subject.resolve_owner(&token.target),
                Some(CemtTreeOwnerRef::Node(CemTreeAstNode::WriterToken { .. }))
            ) {
                return Err(format!(
                    "materialized CEMT color overlay target {target} is not a writer-token node"
                ));
            }
            if token
                .style
                .color_role
                .as_deref()
                .is_some_and(|role| role != token.color_role)
            {
                return Err(format!(
                    "materialized CEMT color overlay target {target} style color role does not match `{}`",
                    token.color_role
                ));
            }
            if token
                .style
                .color_profile
                .as_deref()
                .is_some_and(|style| Some(style) != self.producer.profile())
            {
                return Err(format!(
                    "materialized CEMT color overlay target {target} style color profile does not match producer profile"
                ));
            }
            if token
                .style
                .color_output
                .as_deref()
                .is_some_and(|output| output != self.output.as_str())
            {
                return Err(format!(
                    "materialized CEMT color overlay target {target} style color output does not match `{}`",
                    self.output.as_str()
                ));
            }
        }
        Ok(())
    }
}

fn cemt_owner_path_label(path: &CemtOwnerPath) -> String {
    let mut label = format!("root[{}]", path.root_index());
    for step in path.steps() {
        match step {
            CemtOwnerStep::Child(index) => label.push_str(&format!(".children[{index}]")),
            CemtOwnerStep::Attribute(index) => label.push_str(&format!(".attributes[{index}]")),
        }
    }
    label
}

#[derive(Debug, Clone)]
pub struct CemtMaterializedTreeArtifact {
    identity: CemtMaterializedTreeIdentity,
    input_provenance: CemtMaterializedTreeInputProvenance,
    pipeline: CemtMaterializedTreePipeline,
    owner: Arc<CemTreeAstStream>,
    source_map: Option<SourceMapStack>,
    output_spans: Vec<OutputSpan>,
    color_overlay: Option<CemtMaterializedTreeColorOverlay>,
}

impl CemtMaterializedTreeArtifact {
    pub fn new(
        identity: CemtMaterializedTreeIdentity,
        input_provenance: CemtMaterializedTreeInputProvenance,
        pipeline: CemtMaterializedTreePipeline,
        owner: Arc<CemTreeAstStream>,
        source_map: Option<SourceMapStack>,
        output_spans: Vec<OutputSpan>,
    ) -> Result<Self, String> {
        pipeline.validate()?;
        if pipeline.stage() == CemtMaterializedTreeStage::Colored {
            return Err(
                "colored materialized CEMT trees require a typed color overlay; use `new_colored`"
                    .to_owned(),
            );
        }
        Ok(Self {
            identity,
            input_provenance,
            pipeline,
            owner,
            source_map,
            output_spans,
            color_overlay: None,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_colored(
        identity: CemtMaterializedTreeIdentity,
        input_provenance: CemtMaterializedTreeInputProvenance,
        pipeline: CemtMaterializedTreePipeline,
        owner: Arc<CemTreeAstStream>,
        source_map: Option<SourceMapStack>,
        output_spans: Vec<OutputSpan>,
        color_overlay: CemtMaterializedTreeColorOverlay,
    ) -> Result<Self, String> {
        pipeline.validate()?;
        color_overlay.validate(&pipeline, owner.as_ref())?;
        Ok(Self {
            identity,
            input_provenance,
            pipeline,
            owner,
            source_map,
            output_spans,
            color_overlay: Some(color_overlay),
        })
    }

    pub fn stage(&self) -> CemtMaterializedTreeStage {
        self.pipeline.stage()
    }

    pub fn identity(&self) -> &CemtMaterializedTreeIdentity {
        &self.identity
    }

    pub fn input_provenance(&self) -> &CemtMaterializedTreeInputProvenance {
        &self.input_provenance
    }

    pub fn pipeline(&self) -> &CemtMaterializedTreePipeline {
        &self.pipeline
    }

    pub fn owner(&self) -> &Arc<CemTreeAstStream> {
        &self.owner
    }

    pub fn source_map(&self) -> Option<&SourceMapStack> {
        self.source_map.as_ref()
    }

    pub fn output_spans(&self) -> &[OutputSpan] {
        &self.output_spans
    }

    pub fn color_overlay(&self) -> Option<&CemtMaterializedTreeColorOverlay> {
        self.color_overlay.as_ref()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CemtTreeSubjectRef<'a> {
    owner: &'a CemTreeAstStream,
}

impl<'a> CemtTreeSubjectRef<'a> {
    pub fn new(owner: &'a CemTreeAstStream) -> Self {
        Self { owner }
    }

    pub fn stream(self) -> &'a CemTreeAstStream {
        self.owner
    }

    pub fn nodes(self) -> &'a [crate::projection::CemTreeAstNode] {
        self.owner.as_nodes()
    }

    pub fn resolve_owner(self, path: &CemtOwnerPath) -> Option<CemtTreeOwnerRef<'a>> {
        let mut owner = CemtTreeOwnerRef::Node(self.owner.as_nodes().get(path.root)?);
        for step in &path.steps {
            owner = match (owner, step) {
                (CemtTreeOwnerRef::Node(node), CemtOwnerStep::Child(index)) => {
                    CemtTreeOwnerRef::Node(node.children().get(*index)?)
                }
                (CemtTreeOwnerRef::Node(node), CemtOwnerStep::Attribute(index)) => {
                    CemtTreeOwnerRef::Attribute(node.attributes().get(*index)?)
                }
                (CemtTreeOwnerRef::Attribute(_), _) => return None,
            };
        }
        Some(owner)
    }

    pub fn evaluator_view(self) -> CemtEvaluatorValueRef<'a> {
        CemtEvaluatorValueRef::Sequence(CemtEvaluatorSequenceRef::nodes(
            self.owner.as_nodes(),
            None,
        ))
    }
}

impl CemtMaterializedTreeArtifact {
    pub fn evaluator_view(&self) -> CemtEvaluatorValueRef<'_> {
        CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::MaterializedTree { artifact: self })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct JsonDocumentCemtSubjectRef<'a> {
    document: &'a JsonDocumentAst,
}

#[derive(Debug, Clone, Copy)]
pub struct GenericDataJsonDocumentCemtSubjectRef<'a> {
    document: &'a GenericDataDocumentAst,
}

#[derive(Debug, Clone, Copy)]
pub struct CsvDocumentCemtSubjectRef<'a> {
    document: &'a CsvDocumentAst,
}

#[derive(Debug, Clone, Copy)]
pub struct GenericDataCsvDocumentCemtSubjectRef<'a> {
    document: &'a GenericDataDocumentAst,
}

#[derive(Debug, Clone, Copy)]
pub struct YamlDocumentCemtSubjectRef<'a> {
    document: &'a YamlDocumentAst,
}

#[derive(Debug, Clone, Copy)]
pub struct GenericDataYamlDocumentCemtSubjectRef<'a> {
    document: &'a GenericDataDocumentAst,
}

#[derive(Debug, Clone, Copy)]
pub struct MarkdownDocumentCemtSubjectRef<'a> {
    document: &'a MarkdownDocumentAst,
}

#[derive(Debug, Clone, Copy)]
pub struct RelaxNgDocumentCemtSubjectRef<'a> {
    document: &'a RelaxNgDocumentAst,
}

#[derive(Debug, Clone, Copy)]
pub enum XmlFamilyDocumentCemtSubjectRef<'a> {
    Xml(&'a XmlDocumentAst),
    Html(&'a HtmlDocumentAst),
    Css(&'a CssDocumentAst),
    Xhtml(&'a XhtmlDocumentAst),
    Svg(&'a SvgDocumentAst),
    MathMl(&'a MathMlDocumentAst),
    Xslt(&'a XsltStylesheetAst),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmlFamilyMarkupPackage {
    Svg,
    MathMl,
    Xslt,
}

#[derive(Debug, Clone, Copy)]
pub struct JsonSchemaDocumentCemtSubjectRef<'a> {
    document: &'a JsonSchemaDocumentAst,
}

impl<'a> JsonSchemaDocumentCemtSubjectRef<'a> {
    pub fn new(document: &'a JsonSchemaDocumentAst) -> Self {
        Self { document }
    }

    pub fn document(self) -> &'a JsonSchemaDocumentAst {
        self.document
    }

    pub fn json_document(self) -> &'a JsonDocumentAst {
        &self.document.json
    }

    pub fn evaluator_view(self) -> CemtEvaluatorValueRef<'a> {
        CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::JsonSchemaDocument {
            document: self.document,
        })
    }
}

impl<'a> CsvDocumentCemtSubjectRef<'a> {
    pub fn new(document: &'a CsvDocumentAst) -> Self {
        Self { document }
    }

    pub fn document(self) -> &'a CsvDocumentAst {
        self.document
    }

    pub fn evaluator_view(self) -> CemtEvaluatorValueRef<'a> {
        CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::CsvDocument {
            document: self.document,
        })
    }
}

impl<'a> GenericDataCsvDocumentCemtSubjectRef<'a> {
    pub fn new(document: &'a GenericDataDocumentAst) -> Self {
        Self { document }
    }

    pub fn document(self) -> &'a GenericDataDocumentAst {
        self.document
    }

    pub fn evaluator_view(self) -> CemtEvaluatorValueRef<'a> {
        CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::GenericDataCsvDocument {
            document: self.document,
        })
    }
}

impl<'a> YamlDocumentCemtSubjectRef<'a> {
    pub fn new(document: &'a YamlDocumentAst) -> Self {
        Self { document }
    }

    pub fn document(self) -> &'a YamlDocumentAst {
        self.document
    }

    pub fn evaluator_view(self) -> CemtEvaluatorValueRef<'a> {
        CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::YamlDocument {
            document: self.document,
        })
    }
}

impl<'a> GenericDataYamlDocumentCemtSubjectRef<'a> {
    pub fn new(document: &'a GenericDataDocumentAst) -> Self {
        Self { document }
    }

    pub fn document(self) -> &'a GenericDataDocumentAst {
        self.document
    }

    pub fn evaluator_view(self) -> CemtEvaluatorValueRef<'a> {
        CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::GenericDataYamlDocument {
            document: self.document,
        })
    }
}

impl<'a> MarkdownDocumentCemtSubjectRef<'a> {
    pub fn new(document: &'a MarkdownDocumentAst) -> Self {
        Self { document }
    }

    pub fn document(self) -> &'a MarkdownDocumentAst {
        self.document
    }

    pub fn evaluator_view(self) -> CemtEvaluatorValueRef<'a> {
        CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::MarkdownDocument {
            document: self.document,
        })
    }
}

impl<'a> RelaxNgDocumentCemtSubjectRef<'a> {
    pub fn new(document: &'a RelaxNgDocumentAst) -> Self {
        Self { document }
    }

    pub fn document(self) -> &'a RelaxNgDocumentAst {
        self.document
    }

    pub fn evaluator_view(self) -> CemtEvaluatorValueRef<'a> {
        CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::RelaxNgDocument {
            document: self.document,
        })
    }
}

impl<'a> XmlFamilyDocumentCemtSubjectRef<'a> {
    pub fn xml(document: &'a XmlDocumentAst) -> Self {
        Self::Xml(document)
    }

    pub fn html(document: &'a HtmlDocumentAst) -> Self {
        Self::Html(document)
    }

    pub fn css(document: &'a CssDocumentAst) -> Self {
        Self::Css(document)
    }

    pub fn xhtml(document: &'a XhtmlDocumentAst) -> Self {
        Self::Xhtml(document)
    }

    pub fn svg(document: &'a SvgDocumentAst) -> Self {
        Self::Svg(document)
    }

    pub fn mathml(document: &'a MathMlDocumentAst) -> Self {
        Self::MathMl(document)
    }

    pub fn xslt(document: &'a XsltStylesheetAst) -> Self {
        Self::Xslt(document)
    }

    pub fn evaluator_view(self) -> CemtEvaluatorValueRef<'a> {
        CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::XmlFamilyDocument { document: self })
    }
}

impl<'a> GenericDataJsonDocumentCemtSubjectRef<'a> {
    pub fn new(document: &'a GenericDataDocumentAst) -> Self {
        Self { document }
    }

    pub fn document(self) -> &'a GenericDataDocumentAst {
        self.document
    }

    pub fn evaluator_view(self) -> CemtEvaluatorValueRef<'a> {
        CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::GenericDataJsonDocument {
            document: self.document,
        })
    }
}

impl<'a> JsonDocumentCemtSubjectRef<'a> {
    pub fn new(document: &'a JsonDocumentAst) -> Self {
        Self { document }
    }

    pub fn document(self) -> &'a JsonDocumentAst {
        self.document
    }

    pub fn evaluator_view(self) -> CemtEvaluatorValueRef<'a> {
        CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::JsonDocument {
            document: self.document,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CemtEvaluatorValueKind {
    Null,
    Boolean,
    Number,
    String,
    Sequence,
    Record,
    SourceMap,
}

impl CemtEvaluatorValueKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::String => "string",
            Self::Sequence => "sequence",
            Self::Record => "record",
            Self::SourceMap => "source-map",
        }
    }
}

#[derive(Debug, Clone)]
pub enum CemtEvaluatorValueRef<'a> {
    Null,
    Boolean(bool),
    Number(CemtEvaluatorNumber),
    String(&'a str),
    OwnedString(Arc<str>),
    /// A borrowed lossless JSON value owned by a lifecycle AST stream.
    Json(&'a JsonValueAst),
    StringMap(&'a BTreeMap<String, String>),
    Sequence(CemtEvaluatorSequenceRef<'a>),
    Record(CemtEvaluatorRecordRef<'a>),
    SourceMap(&'a SourceMapStack),
    OwnedSourceMap(Arc<SourceMapStack>),
}

/// A borrowed record view implemented by schema/package-owned AST nodes.
///
/// This keeps the evaluator extensible without requiring package crates to
/// serialize their AST into a generic value before executing a CEMT stage.
pub trait CemtEvaluatorRecordView: fmt::Debug + Send + Sync {
    fn field_names(&self) -> &'static [&'static str];

    fn field<'a>(&'a self, name: &str) -> Option<CemtEvaluatorValueRef<'a>>;
}

/// A borrowed sequence view implemented by schema/package-owned AST owners.
pub trait CemtEvaluatorSequenceView: fmt::Debug + Send + Sync {
    fn len(&self) -> usize;

    fn item<'a>(&'a self, index: usize) -> Option<CemtEvaluatorValueRef<'a>>;
}

impl<'a> CemtEvaluatorValueRef<'a> {
    pub fn kind(&self) -> CemtEvaluatorValueKind {
        match self {
            Self::Null => CemtEvaluatorValueKind::Null,
            Self::Boolean(_) => CemtEvaluatorValueKind::Boolean,
            Self::Number(_) => CemtEvaluatorValueKind::Number,
            Self::String(_) | Self::OwnedString(_) => CemtEvaluatorValueKind::String,
            Self::Json(value) => json_evaluator_value_kind(value),
            Self::StringMap(_) => CemtEvaluatorValueKind::Record,
            Self::Sequence(_) => CemtEvaluatorValueKind::Sequence,
            Self::Record(_) => CemtEvaluatorValueKind::Record,
            Self::SourceMap(_) | Self::OwnedSourceMap(_) => CemtEvaluatorValueKind::SourceMap,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) | Self::Json(JsonValueAst::Boolean { value, .. }) => Some(*value),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<CemtEvaluatorNumber> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Json(JsonValueAst::Number { lexeme, .. }) => json_number_evaluator_value(lexeme),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&'a str> {
        match self {
            Self::String(value) => Some(value),
            Self::Json(JsonValueAst::String { value, .. }) => Some(value),
            _ => None,
        }
    }

    pub fn as_sequence(&self) -> Option<&CemtEvaluatorSequenceRef<'a>> {
        match self {
            Self::Sequence(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_record(&self) -> Option<&CemtEvaluatorRecordRef<'a>> {
        match self {
            Self::Record(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_source_map(&self) -> Option<&SourceMapStack> {
        match self {
            Self::SourceMap(value) => Some(value),
            Self::OwnedSourceMap(value) => Some(value),
            _ => None,
        }
    }

    pub fn into_source_map(self) -> Option<SourceMapStack> {
        match self {
            Self::SourceMap(value) => Some(value.clone()),
            Self::OwnedSourceMap(value) => Some((*value).clone()),
            _ => None,
        }
    }

    pub fn field(&self, name: &str) -> Option<Self> {
        match self {
            Self::Record(record) => record.field(name),
            Self::Json(JsonValueAst::Object { members, .. }) => members
                .iter()
                .rev()
                .find(|member| member.name == name)
                .map(|member| Self::Json(&member.value)),
            Self::StringMap(values) => values.get(name).map(|value| Self::String(value)),
            _ => None,
        }
    }

    pub fn item(&self, index: usize) -> Option<Self> {
        match self {
            Self::Json(JsonValueAst::Array { items, .. }) => items.get(index).map(Self::Json),
            _ => self.as_sequence()?.item(index),
        }
    }

    pub fn resolve_path(mut self, path: &str) -> Option<Self> {
        if path.trim().is_empty() {
            return Some(self);
        }
        for segment in path.split('.') {
            let segment = segment.trim();
            if segment.is_empty() {
                return None;
            }
            self = match &self {
                Self::Record(record) => record.field(segment)?,
                Self::StringMap(values) => {
                    CemtEvaluatorValueRef::String(values.get(segment)?.as_str())
                }
                Self::Sequence(sequence) => sequence.item(segment.parse::<usize>().ok()?)?,
                Self::Json(JsonValueAst::Object { members, .. }) => members
                    .iter()
                    .rev()
                    .find(|member| member.name == segment)
                    .map(|member| Self::Json(&member.value))?,
                Self::Json(JsonValueAst::Array { items, .. }) => {
                    Self::Json(items.get(segment.parse::<usize>().ok()?)?)
                }
                _ => return None,
            };
        }
        Some(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CemtEvaluatorNumber {
    representation: CemtEvaluatorNumberRepresentation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CemtEvaluatorNumberRepresentation {
    Integer(i64),
    UnsignedInteger(u64),
    Decimal(f64),
}

impl CemtEvaluatorNumber {
    pub fn integer(value: i64) -> Self {
        Self {
            representation: CemtEvaluatorNumberRepresentation::Integer(value),
        }
    }

    pub fn unsigned_integer(value: u64) -> Self {
        Self {
            representation: CemtEvaluatorNumberRepresentation::UnsignedInteger(value),
        }
    }

    pub fn decimal(value: f64) -> Option<Self> {
        value.is_finite().then_some(Self {
            representation: CemtEvaluatorNumberRepresentation::Decimal(value),
        })
    }

    pub fn as_i64(self) -> Option<i64> {
        match self.representation {
            CemtEvaluatorNumberRepresentation::Integer(value) => Some(value),
            CemtEvaluatorNumberRepresentation::UnsignedInteger(value) => i64::try_from(value).ok(),
            CemtEvaluatorNumberRepresentation::Decimal(_) => None,
        }
    }

    pub fn as_u64(self) -> Option<u64> {
        match self.representation {
            CemtEvaluatorNumberRepresentation::Integer(value) => u64::try_from(value).ok(),
            CemtEvaluatorNumberRepresentation::UnsignedInteger(value) => Some(value),
            CemtEvaluatorNumberRepresentation::Decimal(_) => None,
        }
    }

    pub fn as_f64(self) -> f64 {
        match self.representation {
            CemtEvaluatorNumberRepresentation::Integer(value) => value as f64,
            CemtEvaluatorNumberRepresentation::UnsignedInteger(value) => value as f64,
            CemtEvaluatorNumberRepresentation::Decimal(value) => value,
        }
    }

    pub fn key_string(self) -> String {
        match self.representation {
            CemtEvaluatorNumberRepresentation::Integer(value) => value.to_string(),
            CemtEvaluatorNumberRepresentation::UnsignedInteger(value) => value.to_string(),
            CemtEvaluatorNumberRepresentation::Decimal(value) => {
                ryu::Buffer::new().format_finite(value).to_owned()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum CemtEvaluatorValue<'a> {
    Null,
    Boolean(bool),
    Number(CemtEvaluatorNumber),
    String(Arc<str>),
    /// A borrowed JSON data value backed by the lifecycle-owned lossless AST.
    ///
    /// Unlike a decoded JSON object projection, this retains duplicate object
    /// members, source order, exact string/number lexemes, ranges, and source
    /// maps until an explicit public/export boundary requests a projection.
    Json(&'a JsonValueAst),
    Sequence(Arc<[CemtEvaluatorValue<'a>]>),
    Record(Arc<CemtEvaluatorRecord<'a>>),
    SourceMap(Arc<SourceMapStack>),
    Borrowed(CemtEvaluatorValueRef<'a>),
}

#[derive(Debug, Clone)]
pub struct CemtEvaluatorRecord<'a> {
    native_base: Option<CemtEvaluatorRecordRef<'a>>,
    fields: BTreeMap<String, CemtEvaluatorValue<'a>>,
    removed_fields: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CemtEvaluatorValueAccessError {
    InvalidPath {
        path: String,
    },
    MissingPath {
        path: String,
    },
    UnsupportedOperation {
        operation: &'static str,
        actual: CemtEvaluatorValueKind,
    },
}

impl fmt::Display for CemtEvaluatorValueAccessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath { path } => write!(formatter, "invalid CEMT value path `{path}`"),
            Self::MissingPath { path } => write!(formatter, "CEMT value path `{path}` is missing"),
            Self::UnsupportedOperation { operation, actual } => write!(
                formatter,
                "CEMT {operation} does not support {} values",
                actual.as_str()
            ),
        }
    }
}

impl std::error::Error for CemtEvaluatorValueAccessError {}

impl<'a> CemtEvaluatorValue<'a> {
    pub fn borrowed(value: CemtEvaluatorValueRef<'a>) -> Self {
        Self::from_borrowed_ref(value)
    }

    fn from_borrowed_ref(value: CemtEvaluatorValueRef<'a>) -> Self {
        match value {
            CemtEvaluatorValueRef::OwnedString(value) => Self::String(value),
            CemtEvaluatorValueRef::Json(value) => Self::Json(value),
            value => Self::Borrowed(value),
        }
    }

    pub fn boolean(value: bool) -> Self {
        Self::Boolean(value)
    }

    pub fn integer(value: i64) -> Self {
        Self::Number(CemtEvaluatorNumber::integer(value))
    }

    pub fn unsigned_integer(value: u64) -> Self {
        Self::Number(CemtEvaluatorNumber::unsigned_integer(value))
    }

    pub fn decimal(value: f64) -> Option<Self> {
        CemtEvaluatorNumber::decimal(value).map(Self::Number)
    }

    pub fn string(value: impl Into<Arc<str>>) -> Self {
        Self::String(value.into())
    }

    pub fn json(value: &'a JsonValueAst) -> Self {
        Self::Json(value)
    }

    pub fn json_ast(&self) -> Option<&'a JsonValueAst> {
        match self {
            Self::Json(value) => Some(value),
            _ => None,
        }
    }

    pub fn json_lexeme(&self) -> Option<&'a str> {
        match self {
            Self::Json(JsonValueAst::String { lexeme, .. })
            | Self::Json(JsonValueAst::Number { lexeme, .. }) => Some(lexeme),
            _ => None,
        }
    }

    pub fn json_source_range(&self) -> Option<JsonSourceRange> {
        self.json_ast().map(JsonValueAst::range)
    }

    pub fn json_source_map(&self) -> Option<SourceMapStack> {
        self.json_source_range().map(JsonSourceRange::source_map)
    }

    pub fn sequence(values: impl IntoIterator<Item = Self>) -> Self {
        Self::Sequence(values.into_iter().collect::<Vec<_>>().into())
    }

    pub fn record<K>(fields: impl IntoIterator<Item = (K, Self)>) -> Self
    where
        K: Into<String>,
    {
        Self::Record(Arc::new(CemtEvaluatorRecord {
            native_base: None,
            fields: fields
                .into_iter()
                .map(|(name, value)| (name.into(), value))
                .collect(),
            removed_fields: BTreeSet::new(),
        }))
    }

    pub fn source_map(source_map: SourceMapStack) -> Self {
        Self::SourceMap(Arc::new(source_map))
    }

    pub fn kind(&self) -> CemtEvaluatorValueKind {
        match self {
            Self::Null => CemtEvaluatorValueKind::Null,
            Self::Boolean(_) => CemtEvaluatorValueKind::Boolean,
            Self::Number(_) => CemtEvaluatorValueKind::Number,
            Self::String(_) => CemtEvaluatorValueKind::String,
            Self::Json(value) => json_evaluator_value_kind(value),
            Self::Sequence(_) => CemtEvaluatorValueKind::Sequence,
            Self::Record(_) => CemtEvaluatorValueKind::Record,
            Self::SourceMap(_) => CemtEvaluatorValueKind::SourceMap,
            Self::Borrowed(value) => value.kind(),
        }
    }

    /// Projects a typed evaluator value at an explicit JSON/export boundary.
    ///
    /// Runtime stages must pass `CemtEvaluatorValue` or its owning artifact
    /// directly. This projection exists only for public responses, debug
    /// output, and compatibility ingress/egress adapters.
    pub fn to_public_json(&self) -> Result<serde_json::Value, String> {
        match self {
            Self::Null | Self::Borrowed(CemtEvaluatorValueRef::Null) => Ok(serde_json::Value::Null),
            Self::Boolean(value) => Ok(serde_json::Value::Bool(*value)),
            Self::Number(value) => cemt_evaluator_number_to_json(*value),
            Self::String(value) => Ok(serde_json::Value::String(value.to_string())),
            Self::Json(value) => json_ast_value_to_public_json(value),
            Self::Sequence(values) => values
                .iter()
                .map(Self::to_public_json)
                .collect::<Result<Vec<_>, _>>()
                .map(serde_json::Value::Array),
            Self::Record(_) => self.record_to_public_json(),
            Self::SourceMap(value) => {
                serde_json::to_value(value.as_ref()).map_err(|error| error.to_string())
            }
            Self::Borrowed(value) => match value {
                CemtEvaluatorValueRef::Null => Ok(serde_json::Value::Null),
                CemtEvaluatorValueRef::Boolean(value) => Ok(serde_json::Value::Bool(*value)),
                CemtEvaluatorValueRef::Number(value) => cemt_evaluator_number_to_json(*value),
                CemtEvaluatorValueRef::String(value) => {
                    Ok(serde_json::Value::String((*value).to_owned()))
                }
                CemtEvaluatorValueRef::OwnedString(value) => {
                    Ok(serde_json::Value::String(value.to_string()))
                }
                CemtEvaluatorValueRef::Json(value) => json_ast_value_to_public_json(value),
                CemtEvaluatorValueRef::StringMap(values) => Ok(serde_json::Value::Object(
                    values
                        .iter()
                        .map(|(name, value)| {
                            (name.clone(), serde_json::Value::String(value.clone()))
                        })
                        .collect(),
                )),
                CemtEvaluatorValueRef::Sequence(sequence) => (0..sequence.len())
                    .map(|index| {
                        sequence
                            .item(index)
                            .map(Self::from_borrowed_ref)
                            .ok_or_else(|| {
                                format!("typed evaluator sequence item {index} is unavailable")
                            })?
                            .to_public_json()
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map(serde_json::Value::Array),
                CemtEvaluatorValueRef::Record(_) => self.record_to_public_json(),
                CemtEvaluatorValueRef::SourceMap(value) => {
                    serde_json::to_value(value).map_err(|error| error.to_string())
                }
                CemtEvaluatorValueRef::OwnedSourceMap(value) => {
                    serde_json::to_value(value.as_ref()).map_err(|error| error.to_string())
                }
            },
        }
    }

    fn record_to_public_json(&self) -> Result<serde_json::Value, String> {
        self.record_field_names("public JSON projection")
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|name| {
                let value = self.field(&name).ok_or_else(|| {
                    format!("typed evaluator record field `{name}` is unavailable")
                })?;
                value.to_public_json().map(|value| (name, value))
            })
            .collect::<Result<serde_json::Map<_, _>, _>>()
            .map(serde_json::Value::Object)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            Self::Json(JsonValueAst::Boolean { value, .. }) => Some(*value),
            Self::Borrowed(value) => value.as_bool(),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<CemtEvaluatorNumber> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Json(JsonValueAst::Number { lexeme, .. }) => json_number_evaluator_value(lexeme),
            Self::Borrowed(value) => value.as_number(),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            Self::Json(JsonValueAst::String { value, .. }) => Some(value),
            Self::Borrowed(value) => value.as_str(),
            _ => None,
        }
    }

    pub fn as_source_map(&self) -> Option<&SourceMapStack> {
        match self {
            Self::SourceMap(value) => Some(value),
            Self::Borrowed(CemtEvaluatorValueRef::SourceMap(value)) => Some(value),
            Self::Borrowed(CemtEvaluatorValueRef::OwnedSourceMap(value)) => Some(value),
            _ => None,
        }
    }

    pub fn native_record(&self) -> Option<&CemtEvaluatorRecordRef<'a>> {
        match self {
            Self::Borrowed(CemtEvaluatorValueRef::Record(record)) => Some(record),
            _ => None,
        }
    }

    pub fn owned_record(&self) -> Option<&CemtEvaluatorRecord<'a>> {
        match self {
            Self::Record(record) => Some(record),
            _ => None,
        }
    }

    pub fn field(&self, name: &str) -> Option<Self> {
        match self {
            Self::Record(record) => record.field(name),
            Self::Json(JsonValueAst::Object { members, .. }) => members
                .iter()
                .rev()
                .find(|member| member.name == name)
                .map(|member| Self::Json(&member.value)),
            Self::Borrowed(CemtEvaluatorValueRef::Record(record)) => {
                record.field(name).map(Self::from_borrowed_ref)
            }
            Self::Borrowed(CemtEvaluatorValueRef::StringMap(values)) => values
                .get(name)
                .map(|value| Self::Borrowed(CemtEvaluatorValueRef::String(value))),
            _ => None,
        }
    }

    pub fn item(&self, index: usize) -> Option<Self> {
        match self {
            Self::Sequence(values) => values.get(index).cloned(),
            Self::Json(JsonValueAst::Array { items, .. }) => items.get(index).map(Self::Json),
            Self::Borrowed(CemtEvaluatorValueRef::Sequence(sequence)) => {
                sequence.item(index).map(Self::from_borrowed_ref)
            }
            _ => None,
        }
    }

    pub fn resolve_path(&self, path: &str) -> Option<Self> {
        if path.trim().is_empty() {
            return Some(self.clone());
        }
        let mut cursor = self.clone();
        for segment in path.split('.') {
            let segment = segment.trim();
            if segment.is_empty() {
                return None;
            }
            cursor = match cursor.kind() {
                CemtEvaluatorValueKind::Record => cursor.field(segment)?,
                CemtEvaluatorValueKind::Sequence => cursor.item(segment.parse().ok()?)?,
                _ => return None,
            };
        }
        Some(cursor)
    }

    pub fn length(&self) -> Result<usize, CemtEvaluatorValueAccessError> {
        match self {
            Self::Null | Self::Borrowed(CemtEvaluatorValueRef::Null) => Ok(0),
            Self::String(value) => Ok(value.chars().count()),
            Self::Json(JsonValueAst::String { value, .. }) => Ok(value.chars().count()),
            Self::Json(JsonValueAst::Array { items, .. }) => Ok(items.len()),
            Self::Json(JsonValueAst::Object { members, .. }) => Ok(members.len()),
            Self::Sequence(values) => Ok(values.len()),
            Self::Record(record) => Ok(record.len()),
            Self::Borrowed(CemtEvaluatorValueRef::String(value)) => Ok(value.chars().count()),
            Self::Borrowed(CemtEvaluatorValueRef::OwnedString(value)) => Ok(value.chars().count()),
            Self::Borrowed(CemtEvaluatorValueRef::Sequence(value)) => Ok(value.len()),
            Self::Borrowed(CemtEvaluatorValueRef::Record(value)) => Ok(value
                .field_names()
                .iter()
                .filter(|name| value.field(name).is_some())
                .count()),
            Self::Borrowed(CemtEvaluatorValueRef::StringMap(values)) => Ok(values.len()),
            Self::Json(_) => Err(CemtEvaluatorValueAccessError::UnsupportedOperation {
                operation: "length",
                actual: self.kind(),
            }),
            _ => Err(CemtEvaluatorValueAccessError::UnsupportedOperation {
                operation: "length",
                actual: self.kind(),
            }),
        }
    }

    pub fn get(&self, key: &str) -> Result<Self, CemtEvaluatorValueAccessError> {
        match self.kind() {
            CemtEvaluatorValueKind::Null => Ok(Self::Null),
            CemtEvaluatorValueKind::Record => Ok(self.field(key.trim()).unwrap_or(Self::Null)),
            CemtEvaluatorValueKind::Sequence => Ok(key
                .trim()
                .parse::<usize>()
                .ok()
                .and_then(|index| self.item(index))
                .unwrap_or(Self::Null)),
            CemtEvaluatorValueKind::String => Ok(key
                .trim()
                .parse::<usize>()
                .ok()
                .and_then(|index| self.as_str()?.chars().nth(index))
                .map(|character| Self::string(character.to_string()))
                .unwrap_or(Self::Null)),
            actual => Err(CemtEvaluatorValueAccessError::UnsupportedOperation {
                operation: "get",
                actual,
            }),
        }
    }

    pub fn get_value(&self, key: &Self) -> Result<Self, CemtEvaluatorValueAccessError> {
        match self.kind() {
            CemtEvaluatorValueKind::Null => return Ok(Self::Null),
            CemtEvaluatorValueKind::Record => {
                if let Some(key) = key.as_str() {
                    return self.get(key);
                }
                if let Some(key) = key.as_number() {
                    return self.get(&key.key_string());
                }
            }
            CemtEvaluatorValueKind::Sequence | CemtEvaluatorValueKind::String => {
                if let Some(key) = key.as_str() {
                    return self.get(key);
                }
                if let Some(index) = key.as_number().and_then(|number| number.as_u64()) {
                    let Ok(index) = usize::try_from(index) else {
                        return Ok(Self::Null);
                    };
                    return self.get(&index.to_string());
                }
                if key.as_number().is_some() {
                    return Ok(Self::Null);
                }
            }
            _ => {}
        }
        Err(CemtEvaluatorValueAccessError::UnsupportedOperation {
            operation: "get key",
            actual: key.kind(),
        })
    }

    pub fn append(&self, value: Self) -> Result<Self, CemtEvaluatorValueAccessError> {
        let mut values = self.sequence_values("append")?;
        values.push(value);
        Ok(Self::sequence(values))
    }

    pub fn extend(&self, values: &Self) -> Result<Self, CemtEvaluatorValueAccessError> {
        let mut extended = self.sequence_values("extend")?;
        extended.extend(values.sequence_values("extend")?);
        Ok(Self::sequence(extended))
    }

    pub fn merge(&self, patch: &Self) -> Result<Self, CemtEvaluatorValueAccessError> {
        let mut merged = self.clone();
        for field in patch.record_field_names("merge")? {
            let value =
                patch
                    .field(&field)
                    .ok_or_else(|| CemtEvaluatorValueAccessError::MissingPath {
                        path: field.clone(),
                    })?;
            merged = merged.with_field(field, value)?;
        }
        Ok(merged)
    }

    pub fn set_path(&self, path: &str, value: Self) -> Result<Self, CemtEvaluatorValueAccessError> {
        let segments = path.split('.').map(str::trim).collect::<Vec<_>>();
        if segments.is_empty() || segments.iter().any(|segment| segment.is_empty()) {
            return Err(CemtEvaluatorValueAccessError::InvalidPath {
                path: path.to_owned(),
            });
        }
        self.set_path_segments(&segments, value, path)
    }

    fn set_path_segments(
        &self,
        segments: &[&str],
        value: Self,
        path: &str,
    ) -> Result<Self, CemtEvaluatorValueAccessError> {
        let Some((segment, tail)) = segments.split_first() else {
            return Err(CemtEvaluatorValueAccessError::InvalidPath {
                path: path.to_owned(),
            });
        };
        match self.kind() {
            CemtEvaluatorValueKind::Record => {
                if tail.is_empty() {
                    return self.with_field(*segment, value);
                }
                let child = self.field(segment).ok_or_else(|| {
                    CemtEvaluatorValueAccessError::MissingPath {
                        path: path.to_owned(),
                    }
                })?;
                self.with_field(*segment, child.set_path_segments(tail, value, path)?)
            }
            CemtEvaluatorValueKind::Sequence => {
                let index = segment.parse::<usize>().map_err(|_| {
                    CemtEvaluatorValueAccessError::InvalidPath {
                        path: path.to_owned(),
                    }
                })?;
                if tail.is_empty() {
                    return self.with_item(index, value, path);
                }
                let child =
                    self.item(index)
                        .ok_or_else(|| CemtEvaluatorValueAccessError::MissingPath {
                            path: path.to_owned(),
                        })?;
                self.with_item(index, child.set_path_segments(tail, value, path)?, path)
            }
            actual => Err(CemtEvaluatorValueAccessError::UnsupportedOperation {
                operation: "set",
                actual,
            }),
        }
    }

    pub(crate) fn with_field(
        &self,
        name: impl Into<String>,
        value: Self,
    ) -> Result<Self, CemtEvaluatorValueAccessError> {
        let mut record = match self {
            Self::Record(record) => (**record).clone(),
            Self::Borrowed(CemtEvaluatorValueRef::Record(record)) => CemtEvaluatorRecord {
                native_base: Some(record.clone()),
                fields: BTreeMap::new(),
                removed_fields: BTreeSet::new(),
            },
            Self::Json(JsonValueAst::Object { members, .. }) => CemtEvaluatorRecord {
                native_base: None,
                fields: members
                    .iter()
                    .map(|member| (member.name.clone(), Self::Json(&member.value)))
                    .collect(),
                removed_fields: BTreeSet::new(),
            },
            _ => {
                return Err(CemtEvaluatorValueAccessError::UnsupportedOperation {
                    operation: "set",
                    actual: self.kind(),
                });
            }
        };
        let name = name.into();
        record.removed_fields.remove(&name);
        record.fields.insert(name, value);
        Ok(Self::Record(Arc::new(record)))
    }

    pub(crate) fn without_field(
        &self,
        name: impl Into<String>,
    ) -> Result<Self, CemtEvaluatorValueAccessError> {
        let mut record = match self {
            Self::Record(record) => (**record).clone(),
            Self::Borrowed(CemtEvaluatorValueRef::Record(record)) => CemtEvaluatorRecord {
                native_base: Some(record.clone()),
                fields: BTreeMap::new(),
                removed_fields: BTreeSet::new(),
            },
            Self::Json(JsonValueAst::Object { members, .. }) => CemtEvaluatorRecord {
                native_base: None,
                fields: members
                    .iter()
                    .map(|member| (member.name.clone(), Self::Json(&member.value)))
                    .collect(),
                removed_fields: BTreeSet::new(),
            },
            _ => {
                return Err(CemtEvaluatorValueAccessError::UnsupportedOperation {
                    operation: "remove field",
                    actual: self.kind(),
                });
            }
        };
        let name = name.into();
        record.fields.remove(&name);
        record.removed_fields.insert(name);
        Ok(Self::Record(Arc::new(record)))
    }

    pub(crate) fn with_item(
        &self,
        index: usize,
        value: Self,
        path: &str,
    ) -> Result<Self, CemtEvaluatorValueAccessError> {
        let mut values = self.sequence_values("set")?;
        let item =
            values
                .get_mut(index)
                .ok_or_else(|| CemtEvaluatorValueAccessError::MissingPath {
                    path: path.to_owned(),
                })?;
        *item = value;
        Ok(Self::sequence(values))
    }

    pub(crate) fn sequence_values(
        &self,
        operation: &'static str,
    ) -> Result<Vec<Self>, CemtEvaluatorValueAccessError> {
        match self {
            Self::Sequence(values) => Ok(values.iter().cloned().collect()),
            Self::Json(JsonValueAst::Array { items, .. }) => {
                Ok(items.iter().map(Self::Json).collect())
            }
            Self::Borrowed(CemtEvaluatorValueRef::Sequence(sequence)) => {
                Ok(sequence.iter().map(Self::from_borrowed_ref).collect())
            }
            _ => Err(CemtEvaluatorValueAccessError::UnsupportedOperation {
                operation,
                actual: self.kind(),
            }),
        }
    }

    pub(crate) fn record_field_names(
        &self,
        operation: &'static str,
    ) -> Result<Vec<String>, CemtEvaluatorValueAccessError> {
        match self {
            Self::Record(record) => Ok(record.field_names()),
            Self::Json(JsonValueAst::Object { members, .. }) => {
                Ok(members.iter().map(|member| member.name.clone()).collect())
            }
            Self::Borrowed(CemtEvaluatorValueRef::Record(record)) => Ok(record
                .field_names()
                .iter()
                .filter(|name| record.field(name).is_some())
                .map(|name| (*name).to_owned())
                .collect()),
            Self::Borrowed(CemtEvaluatorValueRef::StringMap(values)) => {
                Ok(values.keys().cloned().collect())
            }
            _ => Err(CemtEvaluatorValueAccessError::UnsupportedOperation {
                operation,
                actual: self.kind(),
            }),
        }
    }
}

fn cemt_evaluator_number_to_json(value: CemtEvaluatorNumber) -> Result<serde_json::Value, String> {
    let number = if let Some(value) = value.as_i64() {
        serde_json::Number::from(value)
    } else if let Some(value) = value.as_u64() {
        serde_json::Number::from(value)
    } else {
        serde_json::Number::from_f64(value.as_f64()).ok_or_else(|| {
            "typed evaluator public projection received a non-finite number".to_owned()
        })?
    };
    Ok(serde_json::Value::Number(number))
}

fn json_evaluator_value_kind(value: &JsonValueAst) -> CemtEvaluatorValueKind {
    match value {
        JsonValueAst::Null { .. } => CemtEvaluatorValueKind::Null,
        JsonValueAst::Boolean { .. } => CemtEvaluatorValueKind::Boolean,
        JsonValueAst::Number { .. } => CemtEvaluatorValueKind::Number,
        JsonValueAst::String { .. } => CemtEvaluatorValueKind::String,
        JsonValueAst::Array { .. } => CemtEvaluatorValueKind::Sequence,
        JsonValueAst::Object { .. } => CemtEvaluatorValueKind::Record,
    }
}

fn json_number_evaluator_value(lexeme: &str) -> Option<CemtEvaluatorNumber> {
    if !lexeme.contains(['.', 'e', 'E']) {
        if let Ok(value) = lexeme.parse::<i64>() {
            return Some(CemtEvaluatorNumber::integer(value));
        }
        if let Ok(value) = lexeme.parse::<u64>() {
            return Some(CemtEvaluatorNumber::unsigned_integer(value));
        }
    }
    lexeme
        .parse::<f64>()
        .ok()
        .and_then(CemtEvaluatorNumber::decimal)
}

fn json_ast_value_to_public_json(value: &JsonValueAst) -> Result<serde_json::Value, String> {
    match value {
        JsonValueAst::Object { members, .. } => {
            let mut object = serde_json::Map::new();
            for member in members {
                object.insert(
                    member.name.clone(),
                    json_ast_value_to_public_json(&member.value)?,
                );
            }
            Ok(serde_json::Value::Object(object))
        }
        JsonValueAst::Array { items, .. } => items
            .iter()
            .map(json_ast_value_to_public_json)
            .collect::<Result<Vec<_>, _>>()
            .map(serde_json::Value::Array),
        JsonValueAst::String { value, .. } => Ok(serde_json::Value::String(value.clone())),
        JsonValueAst::Number { lexeme, .. } => {
            serde_json::from_str::<serde_json::Value>(lexeme).map_err(|error| {
                format!("lossless JSON number `{lexeme}` cannot cross the public JSON boundary: {error}")
            })
        }
        JsonValueAst::Boolean { value, .. } => Ok(serde_json::Value::Bool(*value)),
        JsonValueAst::Null { .. } => Ok(serde_json::Value::Null),
    }
}

impl<'a> CemtEvaluatorRecord<'a> {
    pub fn native_base(&self) -> Option<&CemtEvaluatorRecordRef<'a>> {
        self.native_base.as_ref()
    }

    pub fn field(&self, name: &str) -> Option<CemtEvaluatorValue<'a>> {
        if self.removed_fields.contains(name) {
            return None;
        }
        self.fields.get(name).cloned().or_else(|| {
            self.native_base
                .as_ref()?
                .field(name)
                .map(CemtEvaluatorValue::from_borrowed_ref)
        })
    }

    pub fn field_names(&self) -> Vec<String> {
        let mut names = self
            .native_base
            .as_ref()
            .map(|base| {
                base.field_names()
                    .iter()
                    .filter(|name| base.field(name).is_some())
                    .map(|name| (*name).to_owned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let added_names = self
            .fields
            .keys()
            .filter(|name| !names.contains(name))
            .cloned()
            .collect::<Vec<_>>();
        names.extend(added_names);
        names.retain(|name| !self.removed_fields.contains(name));
        names
    }

    pub fn len(&self) -> usize {
        self.field_names().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, Default)]
pub struct CemtEvaluatorBindings<'a> {
    values: BTreeMap<String, CemtEvaluatorValue<'a>>,
}

impl<'a> CemtEvaluatorBindings<'a> {
    pub fn bind(
        &mut self,
        name: impl Into<String>,
        value: CemtEvaluatorValue<'a>,
    ) -> Option<CemtEvaluatorValue<'a>> {
        self.values.insert(name.into(), value)
    }

    pub fn value(&self, name: &str) -> Option<&CemtEvaluatorValue<'a>> {
        self.values.get(name)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &CemtEvaluatorValue<'a>)> {
        self.values
            .iter()
            .map(|(name, value)| (name.as_str(), value))
    }

    pub fn resolve_path(&self, path: &str) -> Option<CemtEvaluatorValue<'a>> {
        let mut segments = path.split('.');
        let root = segments.next()?.trim();
        if root.is_empty() {
            return None;
        }
        let mut value = self.values.get(root)?.clone();
        for segment in segments {
            let segment = segment.trim();
            if segment.is_empty() {
                return None;
            }
            value = match value.kind() {
                CemtEvaluatorValueKind::Record => value.field(segment)?,
                CemtEvaluatorValueKind::Sequence => value.item(segment.parse().ok()?)?,
                _ => return None,
            };
        }
        Some(value)
    }

    pub fn exists(&self, path: &str) -> bool {
        self.resolve_path(path).is_some()
    }
}

impl<'a, K> FromIterator<(K, CemtEvaluatorValue<'a>)> for CemtEvaluatorBindings<'a>
where
    K: Into<String>,
{
    fn from_iter<T: IntoIterator<Item = (K, CemtEvaluatorValue<'a>)>>(iter: T) -> Self {
        Self {
            values: iter
                .into_iter()
                .map(|(name, value)| (name.into(), value))
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CemtEvaluatorSequenceRef<'a> {
    Package {
        sequence: &'a dyn CemtEvaluatorSequenceView,
    },
    Empty,
    Strings {
        values: &'a [String],
    },
    CsvParseFacts {
        facts: &'a [CsvDocumentParseFact],
    },
    CsvRows {
        rows: &'a [CsvRecordAst],
    },
    CsvFields {
        fields: &'a [CsvFieldAst],
    },
    GenericDataCsvRows {
        document: &'a GenericDataDocumentAst,
    },
    GenericDataCsvHeaderFields {
        document: &'a GenericDataDocumentAst,
    },
    GenericDataCsvMappingFields {
        document: &'a GenericDataDocumentAst,
        entries: &'a [GenericDataMappingEntryAst],
    },
    GenericDataCsvValueFields {
        value: &'a GenericDataValueAst,
    },
    MarkdownEncodingFacts {
        facts: &'a [MarkdownEncodingFact],
    },
    MarkdownVariantFacts {
        facts: &'a [MarkdownVariantFact],
    },
    MarkdownParseFacts {
        facts: &'a [MarkdownParseFact],
    },
    MarkdownEvents {
        events: &'a [MarkdownEventAst],
    },
    RelaxNgCompactTokens {
        document: &'a RelaxNgDocumentAst,
    },
    RelaxNgFacts {
        facts: &'a [RelaxNgFact],
    },
    XmlFamilyFacts {
        document: XmlFamilyDocumentCemtSubjectRef<'a>,
    },
    XmlFamilyEvents {
        document: XmlFamilyDocumentCemtSubjectRef<'a>,
    },
    XmlAttributes {
        attributes: &'a [XmlAttributeAst],
    },
    HtmlAttributes {
        attributes: &'a [HtmlAttributeAst],
    },
    XmlFamilyMarkupTokens {
        event: &'a XmlEventAst,
        content_type: &'a str,
        package: XmlFamilyMarkupPackage,
    },
    YamlParseFacts {
        facts: &'a [YamlDocumentParseFact],
    },
    YamlDirectives {
        directives: &'a [YamlDirectiveAst],
    },
    YamlComments {
        comments: &'a [YamlCommentAst],
    },
    YamlDocuments {
        documents: &'a [YamlStreamDocumentAst],
    },
    YamlNodes {
        nodes: &'a [YamlNodeAst],
    },
    YamlPairs {
        pairs: &'a [YamlPairAst],
    },
    GenericDataYamlDocuments {
        documents: &'a [GenericDataStreamDocumentAst],
    },
    GenericDataYamlValues {
        values: &'a [GenericDataValueAst],
    },
    GenericDataYamlPairs {
        entries: &'a [GenericDataMappingEntryAst],
    },
    JsonMembers {
        members: &'a [JsonMemberAst],
    },
    JsonValues {
        values: &'a [JsonValueAst],
    },
    GenericDataJsonDocuments {
        documents: &'a [GenericDataStreamDocumentAst],
    },
    GenericDataJsonEntries {
        entries: &'a [GenericDataMappingEntryAst],
    },
    GenericDataJsonValues {
        values: &'a [GenericDataValueAst],
    },
    JsonSchemaParseFacts {
        facts: &'a [JsonSchemaParseFact],
    },
    JsonSchemaDialectFacts {
        facts: &'a [JsonSchemaDialectFact],
    },
    Nodes {
        nodes: &'a [CemTreeAstNode],
        parent: Option<CemtOwnerPath>,
    },
    Attributes {
        attributes: &'a [CemTreeAstAttribute],
        parent: CemtOwnerPath,
    },
    FormattedNodes {
        nodes: &'a [CemTreeAstNode],
        parent: Option<CemtOwnerPath>,
        overlay: &'a CemtFormattedTreeOverlay,
    },
    FormattedAttributes {
        attributes: &'a [CemTreeAstAttribute],
        parent: CemtOwnerPath,
        overlay: &'a CemtFormattedTreeOverlay,
    },
    FormatOperations {
        operations: &'a [CemtFormatOperation],
    },
    NodeFormatFragments {
        operations: &'a [CemtNodeFormatOperation],
        owner: CemtOwnerPath,
        kind: CemtEvaluatorNodeFormatSequenceKind,
    },
    OutputSpans {
        output_spans: &'a [OutputSpan],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CemtEvaluatorNodeFormatSequenceKind {
    BeforeAttribute,
    ContentBoundary,
}

impl<'a> CemtEvaluatorSequenceRef<'a> {
    fn nodes(nodes: &'a [CemTreeAstNode], parent: Option<CemtOwnerPath>) -> Self {
        Self::Nodes { nodes, parent }
    }

    fn attributes(attributes: &'a [CemTreeAstAttribute], parent: CemtOwnerPath) -> Self {
        Self::Attributes { attributes, parent }
    }

    fn formatted_nodes(
        nodes: &'a [CemTreeAstNode],
        parent: Option<CemtOwnerPath>,
        overlay: &'a CemtFormattedTreeOverlay,
    ) -> Self {
        Self::FormattedNodes {
            nodes,
            parent,
            overlay,
        }
    }

    fn formatted_attributes(
        attributes: &'a [CemTreeAstAttribute],
        parent: CemtOwnerPath,
        overlay: &'a CemtFormattedTreeOverlay,
    ) -> Self {
        Self::FormattedAttributes {
            attributes,
            parent,
            overlay,
        }
    }

    fn node_format_fragments(
        operations: &'a [CemtNodeFormatOperation],
        owner: CemtOwnerPath,
        kind: CemtEvaluatorNodeFormatSequenceKind,
    ) -> Self {
        Self::NodeFormatFragments {
            operations,
            owner,
            kind,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Package { sequence } => sequence.len(),
            Self::Empty => 0,
            Self::Strings { values } => values.len(),
            Self::CsvParseFacts { facts } => facts.len(),
            Self::CsvRows { rows } => rows.len(),
            Self::CsvFields { fields } => fields.len(),
            Self::GenericDataCsvRows { document } => generic_data_csv_row_count(document),
            Self::GenericDataCsvHeaderFields { document }
            | Self::GenericDataCsvMappingFields { document, .. } => {
                generic_data_csv_header_count(document)
            }
            Self::GenericDataCsvValueFields { value } => match value {
                GenericDataValueAst::Sequence { items, .. } => items.len(),
                _ => 1,
            },
            Self::MarkdownEncodingFacts { facts } => facts.len(),
            Self::MarkdownVariantFacts { facts } => facts.len(),
            Self::MarkdownParseFacts { facts } => facts.len(),
            Self::MarkdownEvents { events } => events.len(),
            Self::RelaxNgCompactTokens { document } => document.compact_tokens.len(),
            Self::RelaxNgFacts { facts } => facts.len(),
            Self::XmlFamilyFacts { document } => xml_family_fact_count(*document),
            Self::XmlFamilyEvents { document } => xml_family_event_count(*document),
            Self::XmlAttributes { attributes } => attributes.len(),
            Self::HtmlAttributes { attributes } => attributes.len(),
            Self::XmlFamilyMarkupTokens { event, .. } => xml_event_markup_tokens(event).len(),
            Self::YamlParseFacts { facts } => facts.len(),
            Self::YamlDirectives { directives } => directives.len(),
            Self::YamlComments { comments } => comments.len(),
            Self::YamlDocuments { documents } => documents.len(),
            Self::YamlNodes { nodes } => nodes.len(),
            Self::YamlPairs { pairs } => pairs.len(),
            Self::GenericDataYamlDocuments { documents } => documents.len(),
            Self::GenericDataYamlValues { values } => values.len(),
            Self::GenericDataYamlPairs { entries } => entries.len(),
            Self::JsonMembers { members } => members.len(),
            Self::JsonValues { values } => values.len(),
            Self::GenericDataJsonDocuments { documents } => documents.len(),
            Self::GenericDataJsonEntries { entries } => entries.len(),
            Self::GenericDataJsonValues { values } => values.len(),
            Self::JsonSchemaParseFacts { facts } => facts.len(),
            Self::JsonSchemaDialectFacts { facts } => facts.len(),
            Self::Nodes { nodes, .. } => nodes.len(),
            Self::Attributes { attributes, .. } => attributes.len(),
            Self::FormattedNodes {
                nodes,
                parent,
                overlay,
            } => cemt_evaluator_formatted_node_sequence_len(nodes, parent.as_ref(), overlay),
            Self::FormattedAttributes { attributes, .. } => attributes.len(),
            Self::FormatOperations { operations } => operations.len(),
            Self::NodeFormatFragments {
                operations,
                owner,
                kind,
            } => operations
                .iter()
                .filter(|operation| {
                    cemt_evaluator_node_format_sequence_matches(operation, owner, *kind)
                })
                .count(),
            Self::OutputSpans { output_spans } => output_spans.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn item(&self, index: usize) -> Option<CemtEvaluatorValueRef<'a>> {
        match self {
            Self::Package { sequence } => (*sequence).item(index),
            Self::Empty => None,
            Self::Strings { values } => values
                .get(index)
                .map(|value| CemtEvaluatorValueRef::String(value)),
            Self::CsvParseFacts { facts } => facts.get(index).map(|fact| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::CsvParseFact { fact })
            }),
            Self::CsvRows { rows } => rows
                .get(index)
                .map(|row| CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::CsvRow { row })),
            Self::CsvFields { fields } => fields.get(index).map(|field| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::CsvField { field })
            }),
            Self::GenericDataCsvRows { document } => {
                generic_data_csv_row_evaluator_value(document, index)
            }
            Self::GenericDataCsvHeaderFields { document } => {
                let entry = generic_data_csv_header_entry(document, index)?;
                Some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::GenericDataCsvHeaderField { entry, index },
                ))
            }
            Self::GenericDataCsvMappingFields { document, entries } => {
                let header_entry = generic_data_csv_header_entry(document, index)?;
                Some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::GenericDataCsvMappingField {
                        header_entry,
                        entries,
                        index,
                    },
                ))
            }
            Self::GenericDataCsvValueFields { value } => {
                let field_value = match value {
                    GenericDataValueAst::Sequence { items, .. } => items.get(index)?,
                    _ if index == 0 => value,
                    _ => return None,
                };
                Some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::GenericDataCsvValueField {
                        value: field_value,
                        index,
                    },
                ))
            }
            Self::MarkdownEncodingFacts { facts } => facts.get(index).map(|fact| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::MarkdownEncodingFact { fact })
            }),
            Self::MarkdownVariantFacts { facts } => facts.get(index).map(|fact| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::MarkdownVariantFact { fact })
            }),
            Self::MarkdownParseFacts { facts } => facts.get(index).map(|fact| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::MarkdownParseFact { fact })
            }),
            Self::MarkdownEvents { events } => events.get(index).map(|event| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::MarkdownEvent { event })
            }),
            Self::RelaxNgCompactTokens { document } => {
                document.compact_tokens.get(index).map(|token| {
                    CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::RelaxNgCompactToken {
                        token,
                        content_type: &document.source.media_type,
                    })
                })
            }
            Self::RelaxNgFacts { facts } => facts.get(index).map(|fact| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::RelaxNgFact { fact })
            }),
            Self::XmlFamilyFacts { document } => (index < xml_family_fact_count(*document))
                .then_some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::XmlFamilyFact {
                        document: *document,
                        index,
                    },
                )),
            Self::XmlFamilyEvents { document } => (index < xml_family_event_count(*document))
                .then_some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::XmlFamilyEvent {
                        document: *document,
                        index,
                    },
                )),
            Self::XmlAttributes { attributes } => attributes.get(index).map(|attribute| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::XmlAttribute { attribute })
            }),
            Self::HtmlAttributes { attributes } => attributes.get(index).map(|attribute| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::HtmlAttribute { attribute })
            }),
            Self::XmlFamilyMarkupTokens {
                event,
                content_type,
                package,
            } => xml_event_markup_tokens(event)
                .get(index)
                .cloned()
                .map(|token| {
                    CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::XmlFamilyMarkupToken {
                        token,
                        content_type,
                        package: *package,
                    })
                }),
            Self::YamlParseFacts { facts } => facts.get(index).map(|fact| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::YamlParseFact { fact })
            }),
            Self::YamlDirectives { directives } => directives.get(index).map(|directive| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::YamlDirective { directive })
            }),
            Self::YamlComments { comments } => comments.get(index).map(|comment| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::YamlComment { comment })
            }),
            Self::YamlDocuments { documents } => documents.get(index).map(|document| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::YamlStreamDocument {
                    document,
                })
            }),
            Self::YamlNodes { nodes } => nodes.get(index).map(|node| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::YamlNode { node })
            }),
            Self::YamlPairs { pairs } => pairs.get(index).map(|pair| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::YamlPair { pair })
            }),
            Self::GenericDataYamlDocuments { documents } => documents.get(index).map(|document| {
                CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::GenericDataYamlStreamDocument { document },
                )
            }),
            Self::GenericDataYamlValues { values } => values.get(index).map(|value| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::GenericDataYamlNode { value })
            }),
            Self::GenericDataYamlPairs { entries } => entries.get(index).map(|entry| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::GenericDataYamlPair { entry })
            }),
            Self::JsonMembers { members } => {
                let member = members.get(index)?;
                Some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::JsonMember { member },
                ))
            }
            Self::JsonValues { values } => {
                let value = values.get(index)?;
                Some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::JsonValue { value },
                ))
            }
            Self::GenericDataJsonDocuments { documents } => {
                generic_data_json_stream_document_value(documents.get(index)?)
            }
            Self::GenericDataJsonEntries { entries } => {
                let entry = entries.get(index)?;
                Some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::GenericDataJsonMember { entry },
                ))
            }
            Self::GenericDataJsonValues { values } => {
                let value = values.get(index)?;
                Some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::GenericDataJsonValue { value },
                ))
            }
            Self::JsonSchemaParseFacts { facts } => {
                let fact = facts.get(index)?;
                Some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::JsonSchemaParseFact { fact },
                ))
            }
            Self::JsonSchemaDialectFacts { facts } => {
                let fact = facts.get(index)?;
                Some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::JsonSchemaDialectFact { fact },
                ))
            }
            Self::Nodes { nodes, parent } => {
                let node = nodes.get(index)?;
                let path = match parent {
                    Some(parent) => parent.child(index),
                    None => CemtOwnerPath::root(index),
                };
                Some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::Node { node, path },
                ))
            }
            Self::Attributes { attributes, parent } => {
                let attribute = attributes.get(index)?;
                Some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::Attribute {
                        attribute,
                        path: parent.attribute(index),
                    },
                ))
            }
            Self::FormattedNodes {
                nodes,
                parent,
                overlay,
            } => {
                cemt_evaluator_formatted_node_sequence_item(nodes, parent.as_ref(), overlay, index)
            }
            Self::FormattedAttributes {
                attributes,
                parent,
                overlay,
            } => {
                let attribute = attributes.get(index)?;
                Some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::FormattedAttribute {
                        attribute,
                        path: parent.attribute(index),
                        overlay,
                    },
                ))
            }
            Self::FormatOperations { operations } => {
                let operation = operations.get(index)?;
                Some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::FormatOperation { operation, index },
                ))
            }
            Self::NodeFormatFragments {
                operations,
                owner,
                kind,
            } => operations
                .iter()
                .enumerate()
                .filter(|(_, operation)| {
                    cemt_evaluator_node_format_sequence_matches(operation, owner, *kind)
                })
                .nth(index)
                .map(|(index, operation)| {
                    CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::NodeFormatOperation {
                        operation,
                        index,
                    })
                }),
            Self::OutputSpans { output_spans } => output_spans.get(index).map(|output_span| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::OutputSpan { output_span })
            }),
        }
    }

    pub fn iter(&self) -> CemtEvaluatorSequenceIter<'a> {
        CemtEvaluatorSequenceIter {
            sequence: self.clone(),
            index: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CemtEvaluatorSequenceIter<'a> {
    sequence: CemtEvaluatorSequenceRef<'a>,
    index: usize,
}

impl<'a> Iterator for CemtEvaluatorSequenceIter<'a> {
    type Item = CemtEvaluatorValueRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.sequence.item(self.index)?;
        self.index = self.index.saturating_add(1);
        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.sequence.len().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for CemtEvaluatorSequenceIter<'_> {}

fn cemt_evaluator_formatted_node_sequence_len(
    nodes: &[CemTreeAstNode],
    parent: Option<&CemtOwnerPath>,
    overlay: &CemtFormattedTreeOverlay,
) -> usize {
    let inserted = (0..=nodes.len())
        .map(|before_node| {
            overlay
                .node_operations
                .iter()
                .filter(|operation| {
                    cemt_evaluator_gap_operation_matches(operation, parent, before_node)
                })
                .count()
        })
        .sum::<usize>();
    let retained = nodes
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            let path = match parent {
                Some(parent) => parent.child(*index),
                None => CemtOwnerPath::root(*index),
            };
            overlay.retains_node(&path)
        })
        .count();
    inserted.saturating_add(retained)
}

fn cemt_evaluator_formatted_node_sequence_item<'a>(
    nodes: &'a [CemTreeAstNode],
    parent: Option<&CemtOwnerPath>,
    overlay: &'a CemtFormattedTreeOverlay,
    requested: usize,
) -> Option<CemtEvaluatorValueRef<'a>> {
    let mut logical_index = 0usize;
    for before_node in 0..=nodes.len() {
        for (operation_index, operation) in overlay.node_operations.iter().enumerate() {
            if !cemt_evaluator_gap_operation_matches(operation, parent, before_node) {
                continue;
            }
            if logical_index == requested {
                return Some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::NodeFormatOperation {
                        operation,
                        index: operation_index,
                    },
                ));
            }
            logical_index = logical_index.saturating_add(1);
        }

        let Some(node) = nodes.get(before_node) else {
            continue;
        };
        let path = match parent {
            Some(parent) => parent.child(before_node),
            None => CemtOwnerPath::root(before_node),
        };
        if !overlay.retains_node(&path) {
            continue;
        }
        if logical_index == requested {
            return Some(CemtEvaluatorValueRef::Record(
                CemtEvaluatorRecordRef::FormattedNode {
                    node,
                    path,
                    overlay,
                },
            ));
        }
        logical_index = logical_index.saturating_add(1);
    }
    None
}

fn cemt_evaluator_gap_operation_matches(
    operation: &CemtNodeFormatOperation,
    parent: Option<&CemtOwnerPath>,
    before_node: usize,
) -> bool {
    matches!(
        &operation.kind,
        CemtNodeFormatOperationKind::InsertChild { .. }
    ) && match (&operation.target, parent) {
        (CemtNodeFormatTarget::RootGap { before_root, .. }, None) => *before_root == before_node,
        (
            CemtNodeFormatTarget::ChildGap {
                parent,
                before_child,
                ..
            },
            Some(expected_parent),
        ) => parent == expected_parent && *before_child == before_node,
        _ => false,
    }
}

fn cemt_evaluator_node_format_sequence_matches(
    operation: &CemtNodeFormatOperation,
    owner: &CemtOwnerPath,
    kind: CemtEvaluatorNodeFormatSequenceKind,
) -> bool {
    matches!(&operation.target, CemtNodeFormatTarget::Owner(path) if path == owner)
        && match kind {
            CemtEvaluatorNodeFormatSequenceKind::BeforeAttribute => matches!(
                &operation.kind,
                CemtNodeFormatOperationKind::BeforeAttribute { .. }
            ),
            CemtEvaluatorNodeFormatSequenceKind::ContentBoundary => matches!(
                &operation.kind,
                CemtNodeFormatOperationKind::ContentBoundary { .. }
            ),
        }
}

#[derive(Debug, Clone)]
pub enum CemtEvaluatorRecordRef<'a> {
    Package {
        record: &'a dyn CemtEvaluatorRecordView,
    },
    CsvDocument {
        document: &'a CsvDocumentAst,
    },
    CsvSource {
        source: &'a CsvDocumentSource,
    },
    CsvEncodingReport {
        report: &'a CsvEncodingReportAst,
    },
    CsvDialect {
        dialect: &'a CsvDialectAst,
    },
    CsvParseFact {
        fact: &'a CsvDocumentParseFact,
    },
    CsvParseFactSourceRange {
        fact: &'a CsvDocumentParseFact,
    },
    CsvRow {
        row: &'a CsvRecordAst,
    },
    CsvField {
        field: &'a CsvFieldAst,
    },
    CsvSourceRange {
        range: CsvSourceRange,
    },
    GenericDataCsvDocument {
        document: &'a GenericDataDocumentAst,
    },
    GenericDataCsvSource {
        document: &'a GenericDataDocumentAst,
    },
    GenericDataCsvEncodingReport,
    GenericDataCsvDialect {
        document: &'a GenericDataDocumentAst,
    },
    GenericDataCsvHeaderRow {
        document: &'a GenericDataDocumentAst,
    },
    GenericDataCsvMappingRow {
        document: &'a GenericDataDocumentAst,
        entries: &'a [GenericDataMappingEntryAst],
        index: usize,
    },
    GenericDataCsvValueRow {
        value: &'a GenericDataValueAst,
        index: usize,
    },
    GenericDataCsvHeaderField {
        entry: &'a GenericDataMappingEntryAst,
        index: usize,
    },
    GenericDataCsvMappingField {
        header_entry: &'a GenericDataMappingEntryAst,
        entries: &'a [GenericDataMappingEntryAst],
        index: usize,
    },
    GenericDataCsvValueField {
        value: &'a GenericDataValueAst,
        index: usize,
    },
    MarkdownDocument {
        document: &'a MarkdownDocumentAst,
    },
    MarkdownSource {
        source: &'a MarkdownDocumentSource,
    },
    MarkdownEncodingReport {
        report: &'a MarkdownEncodingReportAst,
    },
    MarkdownEncodingFact {
        fact: &'a MarkdownEncodingFact,
    },
    MarkdownVariantFact {
        fact: &'a MarkdownVariantFact,
    },
    MarkdownParseFact {
        fact: &'a MarkdownParseFact,
    },
    MarkdownEvent {
        event: &'a MarkdownEventAst,
    },
    MarkdownSourceRange {
        range: MarkdownSourceRange,
    },
    RelaxNgDocument {
        document: &'a RelaxNgDocumentAst,
    },
    RelaxNgSource {
        source: &'a RelaxNgDocumentSource,
    },
    RelaxNgCompactToken {
        token: &'a RelaxNgCompactTokenAst,
        content_type: &'a str,
    },
    RelaxNgFact {
        fact: &'a RelaxNgFact,
    },
    XmlFamilyDocument {
        document: XmlFamilyDocumentCemtSubjectRef<'a>,
    },
    XmlFamilySource {
        document: XmlFamilyDocumentCemtSubjectRef<'a>,
    },
    XmlFamilyEncodingReport {
        document: XmlFamilyDocumentCemtSubjectRef<'a>,
    },
    XmlFamilyFact {
        document: XmlFamilyDocumentCemtSubjectRef<'a>,
        index: usize,
    },
    XmlFamilyEvent {
        document: XmlFamilyDocumentCemtSubjectRef<'a>,
        index: usize,
    },
    XmlAttribute {
        attribute: &'a XmlAttributeAst,
    },
    HtmlAttribute {
        attribute: &'a HtmlAttributeAst,
    },
    XmlFamilyMarkupToken {
        token: XmlMarkupTokenAst,
        content_type: &'a str,
        package: XmlFamilyMarkupPackage,
    },
    XmlFamilySourceRange {
        byte_offset: u64,
        byte_length: u64,
        line: u32,
        column: u32,
    },
    YamlDocument {
        document: &'a YamlDocumentAst,
    },
    YamlSource {
        source: &'a YamlDocumentSource,
    },
    YamlEncodingReport {
        report: &'a YamlEncodingReportAst,
    },
    YamlParseFact {
        fact: &'a YamlDocumentParseFact,
    },
    YamlParseFactSourceRange {
        fact: &'a YamlDocumentParseFact,
    },
    YamlDirective {
        directive: &'a YamlDirectiveAst,
    },
    YamlComment {
        comment: &'a YamlCommentAst,
    },
    YamlStreamDocument {
        document: &'a YamlStreamDocumentAst,
    },
    YamlNode {
        node: &'a YamlNodeAst,
    },
    YamlPair {
        pair: &'a YamlPairAst,
    },
    YamlSourceRange {
        range: YamlSourceRange,
    },
    GenericDataYamlDocument {
        document: &'a GenericDataDocumentAst,
    },
    GenericDataYamlSource {
        document: &'a GenericDataDocumentAst,
    },
    GenericDataYamlEncodingReport,
    GenericDataYamlStreamDocument {
        document: &'a GenericDataStreamDocumentAst,
    },
    GenericDataYamlNode {
        value: &'a GenericDataValueAst,
    },
    GenericDataYamlPair {
        entry: &'a GenericDataMappingEntryAst,
    },
    JsonDocument {
        document: &'a JsonDocumentAst,
    },
    JsonValue {
        value: &'a JsonValueAst,
    },
    JsonMember {
        member: &'a JsonMemberAst,
    },
    JsonSourceRange {
        range: JsonSourceRange,
    },
    JsonSchemaDocument {
        document: &'a JsonSchemaDocumentAst,
    },
    JsonSchemaSource {
        source: &'a JsonSchemaDocumentSource,
    },
    JsonSchemaParseFact {
        fact: &'a JsonSchemaParseFact,
    },
    JsonSchemaParseFactSourceRange {
        fact: &'a JsonSchemaParseFact,
    },
    JsonSchemaDialectFact {
        fact: &'a JsonSchemaDialectFact,
    },
    GenericDataJsonDocument {
        document: &'a GenericDataDocumentAst,
    },
    GenericDataJsonValue {
        value: &'a GenericDataValueAst,
    },
    GenericDataJsonMember {
        entry: &'a GenericDataMappingEntryAst,
    },
    GenericDataJsonDocumentSequenceRoot {
        documents: &'a [GenericDataStreamDocumentAst],
    },
    GenericDataJsonMissingRoot {
        source_range: &'a GenericDataSourceRangeAst,
    },
    GenericDataJsonGeneratedNull,
    GenericDataSourceRange {
        source_range: &'a GenericDataSourceRangeAst,
    },
    GenericDataGeneratedSourceRange,
    WriterTokenStyle {
        style: &'a CemTreeAstWriterTokenStyle,
    },
    WriterTokenMetadata {
        metadata: &'a CemTreeAstWriterTokenMetadata,
    },
    WriterTokenSourceRange {
        range: &'a CemTreeAstWriterTokenSourceRange,
    },
    OutputSpan {
        output_span: &'a OutputSpan,
    },
    OutputRange {
        range: &'a crate::source::ByteRange,
    },
    Node {
        node: &'a CemTreeAstNode,
        path: CemtOwnerPath,
    },
    Attribute {
        attribute: &'a CemTreeAstAttribute,
        path: CemtOwnerPath,
    },
    FormattedTree {
        subject: CemtTreeSubjectRef<'a>,
        overlay: &'a CemtFormattedTreeOverlay,
    },
    MaterializedTree {
        artifact: &'a CemtMaterializedTreeArtifact,
    },
    FormattedNode {
        node: &'a CemTreeAstNode,
        path: CemtOwnerPath,
        overlay: &'a CemtFormattedTreeOverlay,
    },
    FormattedAttribute {
        attribute: &'a CemTreeAstAttribute,
        path: CemtOwnerPath,
        overlay: &'a CemtFormattedTreeOverlay,
    },
    FormatOperation {
        operation: &'a CemtFormatOperation,
        index: usize,
    },
    NodeFormatOperation {
        operation: &'a CemtNodeFormatOperation,
        index: usize,
    },
}

impl<'a> CemtEvaluatorRecordRef<'a> {
    pub fn owner_path(&self) -> Option<&CemtOwnerPath> {
        match self {
            Self::Node { path, .. }
            | Self::Attribute { path, .. }
            | Self::FormattedNode { path, .. }
            | Self::FormattedAttribute { path, .. } => Some(path),
            Self::Package { .. }
            | Self::JsonDocument { .. }
            | Self::CsvDocument { .. }
            | Self::CsvSource { .. }
            | Self::CsvEncodingReport { .. }
            | Self::CsvDialect { .. }
            | Self::CsvParseFact { .. }
            | Self::CsvParseFactSourceRange { .. }
            | Self::CsvRow { .. }
            | Self::CsvField { .. }
            | Self::CsvSourceRange { .. }
            | Self::GenericDataCsvDocument { .. }
            | Self::GenericDataCsvSource { .. }
            | Self::GenericDataCsvEncodingReport
            | Self::GenericDataCsvDialect { .. }
            | Self::GenericDataCsvHeaderRow { .. }
            | Self::GenericDataCsvMappingRow { .. }
            | Self::GenericDataCsvValueRow { .. }
            | Self::GenericDataCsvHeaderField { .. }
            | Self::GenericDataCsvMappingField { .. }
            | Self::GenericDataCsvValueField { .. }
            | Self::MarkdownDocument { .. }
            | Self::MarkdownSource { .. }
            | Self::MarkdownEncodingReport { .. }
            | Self::MarkdownEncodingFact { .. }
            | Self::MarkdownVariantFact { .. }
            | Self::MarkdownParseFact { .. }
            | Self::MarkdownEvent { .. }
            | Self::MarkdownSourceRange { .. }
            | Self::RelaxNgDocument { .. }
            | Self::RelaxNgSource { .. }
            | Self::RelaxNgCompactToken { .. }
            | Self::RelaxNgFact { .. }
            | Self::XmlFamilyDocument { .. }
            | Self::XmlFamilySource { .. }
            | Self::XmlFamilyEncodingReport { .. }
            | Self::XmlFamilyFact { .. }
            | Self::XmlFamilyEvent { .. }
            | Self::XmlAttribute { .. }
            | Self::HtmlAttribute { .. }
            | Self::XmlFamilyMarkupToken { .. }
            | Self::XmlFamilySourceRange { .. }
            | Self::YamlDocument { .. }
            | Self::YamlSource { .. }
            | Self::YamlEncodingReport { .. }
            | Self::YamlParseFact { .. }
            | Self::YamlParseFactSourceRange { .. }
            | Self::YamlDirective { .. }
            | Self::YamlComment { .. }
            | Self::YamlStreamDocument { .. }
            | Self::YamlNode { .. }
            | Self::YamlPair { .. }
            | Self::YamlSourceRange { .. }
            | Self::GenericDataYamlDocument { .. }
            | Self::GenericDataYamlSource { .. }
            | Self::GenericDataYamlEncodingReport
            | Self::GenericDataYamlStreamDocument { .. }
            | Self::GenericDataYamlNode { .. }
            | Self::GenericDataYamlPair { .. }
            | Self::JsonValue { .. }
            | Self::JsonMember { .. }
            | Self::JsonSourceRange { .. }
            | Self::JsonSchemaDocument { .. }
            | Self::JsonSchemaSource { .. }
            | Self::JsonSchemaParseFact { .. }
            | Self::JsonSchemaParseFactSourceRange { .. }
            | Self::JsonSchemaDialectFact { .. }
            | Self::GenericDataJsonDocument { .. }
            | Self::GenericDataJsonValue { .. }
            | Self::GenericDataJsonMember { .. }
            | Self::GenericDataJsonDocumentSequenceRoot { .. }
            | Self::GenericDataJsonMissingRoot { .. }
            | Self::GenericDataJsonGeneratedNull
            | Self::GenericDataSourceRange { .. }
            | Self::GenericDataGeneratedSourceRange
            | Self::WriterTokenStyle { .. }
            | Self::WriterTokenMetadata { .. }
            | Self::WriterTokenSourceRange { .. }
            | Self::OutputSpan { .. }
            | Self::OutputRange { .. }
            | Self::FormattedTree { .. }
            | Self::MaterializedTree { .. }
            | Self::FormatOperation { .. }
            | Self::NodeFormatOperation { .. } => None,
        }
    }

    pub fn owner(&self) -> Option<CemtTreeOwnerRef<'a>> {
        match self {
            Self::Node { node, .. } | Self::FormattedNode { node, .. } => {
                Some(CemtTreeOwnerRef::Node(node))
            }
            Self::Attribute { attribute, .. } | Self::FormattedAttribute { attribute, .. } => {
                Some(CemtTreeOwnerRef::Attribute(attribute))
            }
            Self::Package { .. }
            | Self::JsonDocument { .. }
            | Self::CsvDocument { .. }
            | Self::CsvSource { .. }
            | Self::CsvEncodingReport { .. }
            | Self::CsvDialect { .. }
            | Self::CsvParseFact { .. }
            | Self::CsvParseFactSourceRange { .. }
            | Self::CsvRow { .. }
            | Self::CsvField { .. }
            | Self::CsvSourceRange { .. }
            | Self::GenericDataCsvDocument { .. }
            | Self::GenericDataCsvSource { .. }
            | Self::GenericDataCsvEncodingReport
            | Self::GenericDataCsvDialect { .. }
            | Self::GenericDataCsvHeaderRow { .. }
            | Self::GenericDataCsvMappingRow { .. }
            | Self::GenericDataCsvValueRow { .. }
            | Self::GenericDataCsvHeaderField { .. }
            | Self::GenericDataCsvMappingField { .. }
            | Self::GenericDataCsvValueField { .. }
            | Self::MarkdownDocument { .. }
            | Self::MarkdownSource { .. }
            | Self::MarkdownEncodingReport { .. }
            | Self::MarkdownEncodingFact { .. }
            | Self::MarkdownVariantFact { .. }
            | Self::MarkdownParseFact { .. }
            | Self::MarkdownEvent { .. }
            | Self::MarkdownSourceRange { .. }
            | Self::RelaxNgDocument { .. }
            | Self::RelaxNgSource { .. }
            | Self::RelaxNgCompactToken { .. }
            | Self::RelaxNgFact { .. }
            | Self::XmlFamilyDocument { .. }
            | Self::XmlFamilySource { .. }
            | Self::XmlFamilyEncodingReport { .. }
            | Self::XmlFamilyFact { .. }
            | Self::XmlFamilyEvent { .. }
            | Self::XmlAttribute { .. }
            | Self::HtmlAttribute { .. }
            | Self::XmlFamilyMarkupToken { .. }
            | Self::XmlFamilySourceRange { .. }
            | Self::YamlDocument { .. }
            | Self::YamlSource { .. }
            | Self::YamlEncodingReport { .. }
            | Self::YamlParseFact { .. }
            | Self::YamlParseFactSourceRange { .. }
            | Self::YamlDirective { .. }
            | Self::YamlComment { .. }
            | Self::YamlStreamDocument { .. }
            | Self::YamlNode { .. }
            | Self::YamlPair { .. }
            | Self::YamlSourceRange { .. }
            | Self::GenericDataYamlDocument { .. }
            | Self::GenericDataYamlSource { .. }
            | Self::GenericDataYamlEncodingReport
            | Self::GenericDataYamlStreamDocument { .. }
            | Self::GenericDataYamlNode { .. }
            | Self::GenericDataYamlPair { .. }
            | Self::JsonValue { .. }
            | Self::JsonMember { .. }
            | Self::JsonSourceRange { .. }
            | Self::JsonSchemaDocument { .. }
            | Self::JsonSchemaSource { .. }
            | Self::JsonSchemaParseFact { .. }
            | Self::JsonSchemaParseFactSourceRange { .. }
            | Self::JsonSchemaDialectFact { .. }
            | Self::GenericDataJsonDocument { .. }
            | Self::GenericDataJsonValue { .. }
            | Self::GenericDataJsonMember { .. }
            | Self::GenericDataJsonDocumentSequenceRoot { .. }
            | Self::GenericDataJsonMissingRoot { .. }
            | Self::GenericDataJsonGeneratedNull
            | Self::GenericDataSourceRange { .. }
            | Self::GenericDataGeneratedSourceRange
            | Self::WriterTokenStyle { .. }
            | Self::WriterTokenMetadata { .. }
            | Self::WriterTokenSourceRange { .. }
            | Self::OutputSpan { .. }
            | Self::OutputRange { .. }
            | Self::FormattedTree { .. }
            | Self::MaterializedTree { .. }
            | Self::FormatOperation { .. }
            | Self::NodeFormatOperation { .. } => None,
        }
    }

    pub fn format_operation_index(&self) -> Option<usize> {
        match self {
            Self::FormatOperation { index, .. } => Some(*index),
            _ => None,
        }
    }

    pub fn format_operation(&self) -> Option<&'a CemtFormatOperation> {
        match self {
            Self::FormatOperation { operation, .. } => Some(operation),
            _ => None,
        }
    }

    pub fn node_format_operation_index(&self) -> Option<usize> {
        match self {
            Self::NodeFormatOperation { index, .. } => Some(*index),
            _ => None,
        }
    }

    pub fn node_format_operation(&self) -> Option<&'a CemtNodeFormatOperation> {
        match self {
            Self::NodeFormatOperation { operation, .. } => Some(operation),
            _ => None,
        }
    }

    pub fn field_names(&self) -> &'static [&'static str] {
        match self {
            Self::Package { record } => record.field_names(),
            Self::CsvDocument { document } if document.line_ending.is_some() => &[
                "kind",
                "source",
                "encoding",
                "encodingReport",
                "delimiter",
                "header",
                "dialect",
                "parseFacts",
                "rows",
                "lineEnding",
            ],
            Self::CsvDocument { .. } => &[
                "kind",
                "source",
                "encoding",
                "encodingReport",
                "delimiter",
                "header",
                "dialect",
                "parseFacts",
                "rows",
            ],
            Self::CsvSource { .. } | Self::GenericDataCsvSource { .. } => &[
                "uri",
                "contentType",
                "mediaType",
                "parameters",
                "byteLength",
            ],
            Self::CsvEncodingReport { report } if report.declared_charset.is_some() => {
                if report.invalid_byte_offset.is_some() {
                    &[
                        "declaredCharset",
                        "normalizedCharset",
                        "decoderStatus",
                        "invalidByteOffset",
                    ]
                } else {
                    &["declaredCharset", "normalizedCharset", "decoderStatus"]
                }
            }
            Self::CsvEncodingReport { report } if report.invalid_byte_offset.is_some() => {
                &["normalizedCharset", "decoderStatus", "invalidByteOffset"]
            }
            Self::CsvEncodingReport { .. } | Self::GenericDataCsvEncodingReport => {
                &["normalizedCharset", "decoderStatus"]
            }
            Self::CsvDialect { dialect } if dialect.line_ending.is_some() => {
                &["delimiter", "quote", "escape", "header", "lineEnding"]
            }
            Self::CsvDialect { .. } => &["delimiter", "quote", "escape", "header"],
            Self::CsvParseFact { .. } => &[
                "kind",
                "contract",
                "behavior",
                "diagnosticCode",
                "diagnosticSeverity",
                "recoverable",
                "fatal",
                "parameter",
                "actual",
                "expected",
                "rowIndex",
                "fieldIndex",
                "expectedCount",
                "actualCount",
                "line",
                "column",
                "byteOffset",
                "message",
                "sourceRange",
            ],
            Self::CsvParseFactSourceRange { .. } => &["byteOffset", "line", "column"],
            Self::CsvRow { .. }
            | Self::GenericDataCsvHeaderRow { .. }
            | Self::GenericDataCsvMappingRow { .. }
            | Self::GenericDataCsvValueRow { .. } => &[
                "index",
                "fieldCount",
                "byteOffset",
                "byteLength",
                "sourceRange",
                "sourceMap",
                "recordEndingSourceRange",
                "recordEndingSourceMap",
                "fields",
            ],
            Self::CsvField { .. }
            | Self::GenericDataCsvHeaderField { .. }
            | Self::GenericDataCsvMappingField { .. }
            | Self::GenericDataCsvValueField { .. } => &[
                "index",
                "value",
                "lexeme",
                "quoted",
                "byteOffset",
                "byteLength",
                "sourceRange",
                "sourceMap",
                "delimiterBeforeSourceRange",
                "delimiterBeforeSourceMap",
            ],
            Self::CsvSourceRange { .. } => &[
                "byteOffset",
                "byteLength",
                "line",
                "column",
                "endLine",
                "endColumn",
            ],
            Self::GenericDataCsvDocument { document } if document.line_ending.is_some() => &[
                "kind",
                "source",
                "encoding",
                "encodingReport",
                "delimiter",
                "header",
                "dialect",
                "parseFacts",
                "rows",
                "lineEnding",
            ],
            Self::GenericDataCsvDocument { .. } => &[
                "kind",
                "source",
                "encoding",
                "encodingReport",
                "delimiter",
                "header",
                "dialect",
                "parseFacts",
                "rows",
            ],
            Self::GenericDataCsvDialect { document } if document.line_ending.is_some() => {
                &["delimiter", "quote", "escape", "header", "lineEnding"]
            }
            Self::GenericDataCsvDialect { .. } => &["delimiter", "quote", "escape", "header"],
            Self::MarkdownDocument { document } if document.line_ending.is_some() => &[
                "kind",
                "contentType",
                "schema",
                "source",
                "encodingReport",
                "encodingFacts",
                "variant",
                "variantFacts",
                "parseFacts",
                "events",
                "lineEnding",
            ],
            Self::MarkdownDocument { .. } => &[
                "kind",
                "contentType",
                "schema",
                "source",
                "encodingReport",
                "encodingFacts",
                "variant",
                "variantFacts",
                "parseFacts",
                "events",
            ],
            Self::MarkdownSource { .. } => &[
                "uri",
                "contentType",
                "mediaType",
                "parameters",
                "byteLength",
            ],
            Self::MarkdownEncodingReport { report } if report.declared_charset.is_some() => {
                if report.invalid_byte_offset.is_some() {
                    &[
                        "declaredCharset",
                        "normalizedCharset",
                        "decoderStatus",
                        "invalidByteOffset",
                    ]
                } else {
                    &["declaredCharset", "normalizedCharset", "decoderStatus"]
                }
            }
            Self::MarkdownEncodingReport { report } if report.invalid_byte_offset.is_some() => {
                &["normalizedCharset", "decoderStatus", "invalidByteOffset"]
            }
            Self::MarkdownEncodingReport { .. } => &["normalizedCharset", "decoderStatus"],
            Self::MarkdownEncodingFact { .. } => &[
                "kind",
                "diagnosticCode",
                "diagnosticSeverity",
                "recoverable",
                "fatal",
                "parameter",
                "actual",
                "expected",
                "message",
                "sourceRange",
                "sourceMap",
            ],
            Self::MarkdownVariantFact { .. } => &[
                "kind",
                "variant",
                "diagnosticCode",
                "diagnosticSeverity",
                "recoverable",
                "fatal",
                "message",
            ],
            Self::MarkdownParseFact { .. } => &[
                "kind",
                "diagnosticCode",
                "diagnosticSeverity",
                "recoverable",
                "fatal",
                "eventIndex",
                "eventKind",
                "raw",
                "message",
                "sourceRange",
                "sourceMap",
            ],
            Self::MarkdownEvent { .. } => &[
                "index",
                "kind",
                "tag",
                "text",
                "destination",
                "title",
                "info",
                "level",
                "checked",
                "orderedStart",
                "byteOffset",
                "byteLength",
                "sourceRange",
                "sourceMap",
            ],
            Self::MarkdownSourceRange { .. } => &["byteOffset", "byteLength", "line", "column"],
            Self::RelaxNgDocument { .. } => &[
                "kind",
                "contentType",
                "schema",
                "category",
                "syntaxKind",
                "source",
                "xmlEvents",
                "compactTokens",
                "parseFacts",
                "lineEnding",
            ],
            Self::RelaxNgSource { .. } => &[
                "uri",
                "contentType",
                "mediaType",
                "parameters",
                "byteLength",
            ],
            Self::RelaxNgCompactToken { .. } => &[
                "index",
                "kind",
                "lexeme",
                "depth",
                "role",
                "sourceRange",
                "sourceMap",
            ],
            Self::RelaxNgFact { .. } => &["kind", "syntaxKind", "sourceRange", "message", "value"],
            Self::XmlFamilyDocument { document } => {
                xml_family_document_evaluator_field_names(*document)
            }
            Self::XmlFamilySource { .. } => &[
                "uri",
                "contentType",
                "mediaType",
                "parameters",
                "byteLength",
            ],
            Self::XmlFamilyEncodingReport { document } => {
                xml_family_encoding_report_evaluator_field_names(*document)
            }
            Self::XmlFamilyFact { document, .. } => {
                xml_family_fact_evaluator_field_names(*document)
            }
            Self::XmlFamilyEvent { document, .. } => {
                xml_family_event_evaluator_field_names(*document)
            }
            Self::XmlAttribute { .. } => &[
                "qualifiedName",
                "localName",
                "prefix",
                "namespaceUri",
                "value",
            ],
            Self::HtmlAttribute { .. } => &[
                "lexicalName",
                "localName",
                "value",
                "lexeme",
                "duplicate",
                "sourceRange",
                "sourceMap",
            ],
            Self::XmlFamilyMarkupToken { .. } => {
                &["kind", "text", "role", "sourceRange", "sourceMap"]
            }
            Self::XmlFamilySourceRange { .. } => &["byteOffset", "byteLength", "line", "column"],
            Self::YamlDocument { document } if document.line_ending.is_some() => &[
                "kind",
                "contentType",
                "schema",
                "source",
                "encoding",
                "encodingReport",
                "parseFacts",
                "directives",
                "comments",
                "documents",
                "lineEnding",
            ],
            Self::YamlDocument { .. } => &[
                "kind",
                "contentType",
                "schema",
                "source",
                "encoding",
                "encodingReport",
                "parseFacts",
                "directives",
                "comments",
                "documents",
            ],
            Self::YamlSource { .. } | Self::GenericDataYamlSource { .. } => &[
                "uri",
                "contentType",
                "mediaType",
                "parameters",
                "byteLength",
            ],
            Self::YamlEncodingReport { report } if report.declared_charset.is_some() => {
                if report.invalid_byte_offset.is_some() {
                    &[
                        "declaredCharset",
                        "normalizedCharset",
                        "decoderStatus",
                        "invalidByteOffset",
                    ]
                } else {
                    &["declaredCharset", "normalizedCharset", "decoderStatus"]
                }
            }
            Self::YamlEncodingReport { report } if report.invalid_byte_offset.is_some() => {
                &["normalizedCharset", "decoderStatus", "invalidByteOffset"]
            }
            Self::YamlEncodingReport { .. } | Self::GenericDataYamlEncodingReport => {
                &["normalizedCharset", "decoderStatus"]
            }
            Self::YamlParseFact { .. } => &[
                "kind",
                "contract",
                "behavior",
                "diagnosticCode",
                "diagnosticSeverity",
                "recoverable",
                "fatal",
                "parameter",
                "actual",
                "expected",
                "line",
                "column",
                "byteOffset",
                "byteLength",
                "message",
                "sourceRange",
            ],
            Self::YamlParseFactSourceRange { .. } => {
                &["byteOffset", "byteLength", "line", "column"]
            }
            Self::YamlDirective { .. } => &[
                "index",
                "name",
                "value",
                "byteOffset",
                "sourceRange",
                "sourceMap",
            ],
            Self::YamlComment { .. } => &[
                "index",
                "kind",
                "value",
                "text",
                "indent",
                "placement",
                "byteOffset",
                "sourceRange",
                "sourceMap",
            ],
            Self::YamlStreamDocument { .. } | Self::GenericDataYamlStreamDocument { .. } => {
                &["index", "byteOffset", "sourceRange", "sourceMap", "root"]
            }
            Self::YamlNode { .. } | Self::GenericDataYamlNode { .. } => &[
                "kind",
                "tag",
                "anchor",
                "anchorId",
                "alias",
                "value",
                "style",
                "implicitKind",
                "byteOffset",
                "sourceRange",
                "sourceMap",
                "sequence",
                "mapping",
            ],
            Self::YamlPair { .. } | Self::GenericDataYamlPair { .. } => &["index", "key", "value"],
            Self::YamlSourceRange { .. } => &["byteOffset", "byteLength", "line", "column"],
            Self::GenericDataYamlDocument { document } if document.line_ending.is_some() => &[
                "kind",
                "contentType",
                "schema",
                "source",
                "encoding",
                "encodingReport",
                "parseFacts",
                "directives",
                "comments",
                "documents",
                "lineEnding",
            ],
            Self::GenericDataYamlDocument { .. } => &[
                "kind",
                "contentType",
                "schema",
                "source",
                "encoding",
                "encodingReport",
                "parseFacts",
                "directives",
                "comments",
                "documents",
            ],
            Self::JsonDocument { .. } => &[
                "kind",
                "contentType",
                "schema",
                "encoding",
                "lineEnding",
                "root",
            ],
            Self::JsonValue { value } => match value {
                JsonValueAst::Object { .. } => &["kind", "sourceRange", "sourceMap", "members"],
                JsonValueAst::Array { .. } => &["kind", "sourceRange", "sourceMap", "items"],
                JsonValueAst::String { .. } => {
                    &["kind", "sourceRange", "sourceMap", "value", "lexeme"]
                }
                JsonValueAst::Number { .. } => {
                    &["kind", "sourceRange", "sourceMap", "lexeme", "numberKind"]
                }
                JsonValueAst::Boolean { .. } => &["kind", "sourceRange", "sourceMap", "value"],
                JsonValueAst::Null { .. } => &["kind", "sourceRange", "sourceMap"],
            },
            Self::JsonMember { .. } => &[
                "index",
                "name",
                "nameLexeme",
                "nameSourceRange",
                "nameSourceMap",
                "sourceRange",
                "sourceMap",
                "value",
            ],
            Self::JsonSourceRange { .. } => &["byteOffset", "byteLength", "line", "column"],
            Self::JsonSchemaDocument { .. } => &[
                "kind",
                "contentType",
                "schema",
                "source",
                "json",
                "parseFacts",
                "dialectFacts",
                "dialect",
            ],
            Self::JsonSchemaSource { .. } => &[
                "uri",
                "contentType",
                "mediaType",
                "parameters",
                "byteLength",
            ],
            Self::JsonSchemaParseFact { .. } => &[
                "kind",
                "diagnosticCode",
                "diagnosticSeverity",
                "fatal",
                "memberName",
                "line",
                "column",
                "byteOffset",
                "byteLength",
                "message",
                "sourceRange",
            ],
            Self::JsonSchemaParseFactSourceRange { .. } => {
                &["byteOffset", "byteLength", "line", "column"]
            }
            Self::JsonSchemaDialectFact { fact } if fact.source_range.is_some() => &[
                "kind",
                "dialect",
                "diagnosticCode",
                "diagnosticSeverity",
                "fatal",
                "message",
                "sourceRange",
                "sourceMap",
            ],
            Self::JsonSchemaDialectFact { .. } => &[
                "kind",
                "dialect",
                "diagnosticCode",
                "diagnosticSeverity",
                "fatal",
                "message",
            ],
            Self::GenericDataJsonDocument { .. } => &[
                "kind",
                "contentType",
                "schema",
                "encoding",
                "lineEnding",
                "root",
            ],
            Self::GenericDataJsonValue { value } => match value {
                GenericDataValueAst::Mapping { .. } => {
                    &["kind", "sourceRange", "sourceMap", "members"]
                }
                GenericDataValueAst::Sequence { .. } => {
                    &["kind", "sourceRange", "sourceMap", "items"]
                }
                GenericDataValueAst::String { .. } => {
                    &["kind", "sourceRange", "sourceMap", "value", "lexeme"]
                }
                GenericDataValueAst::Number { .. } => {
                    &["kind", "sourceRange", "sourceMap", "lexeme", "numberKind"]
                }
                GenericDataValueAst::Boolean { .. } => {
                    &["kind", "sourceRange", "sourceMap", "value"]
                }
                GenericDataValueAst::Null { .. } | GenericDataValueAst::Alias { .. } => {
                    &["kind", "sourceRange", "sourceMap"]
                }
            },
            Self::GenericDataJsonMember { .. } => &[
                "index",
                "name",
                "nameLexeme",
                "nameSourceRange",
                "nameSourceMap",
                "sourceRange",
                "sourceMap",
                "value",
            ],
            Self::GenericDataJsonDocumentSequenceRoot { .. } => {
                &["kind", "sourceRange", "sourceMap", "items"]
            }
            Self::GenericDataJsonMissingRoot { .. } | Self::GenericDataJsonGeneratedNull => {
                &["kind", "sourceRange", "sourceMap"]
            }
            Self::GenericDataSourceRange { .. } | Self::GenericDataGeneratedSourceRange => {
                &["byteOffset", "byteLength", "line", "column"]
            }
            Self::WriterTokenStyle { .. } => &[
                "colorRole",
                "colorProfile",
                "colorOutput",
                "htmlMode",
                "class",
                "color",
                "wrapper",
                "terminalCapability",
                "tabular",
            ],
            Self::WriterTokenMetadata { .. } => &[
                "name",
                "formatterProfile",
                "formatterRole",
                "sourceRange",
                "memberIndex",
                "eventIndex",
                "eventKind",
                "eventTag",
                "package",
                "syntaxKind",
                "depth",
                "qualifiedName",
                "lexicalName",
                "localName",
                "namespaceUri",
                "tokenKind",
                "lexeme",
                "index",
                "role",
                "operator",
                "cemQlRole",
                "legacy",
                "diagnostic",
                "replacement",
                "documentSafeBoundary",
                "lexicalSafeBoundary",
                "layoutSensitive",
                "generated",
                "layout",
                "lineEnding",
                "indent",
                "leadingComma",
                "scopeOpeningNewLine",
                "delimiter",
                "rowIndex",
                "fieldIndex",
                "raw",
                "quoted",
                "byteOffset",
                "byteLength",
                "rowSourceRange",
                "rowByteOffset",
                "rowByteLength",
                "fieldCount",
                "tabSize",
                "presentationOnly",
                "strictCsv",
                "dataPreserving",
                "sourcePreserving",
            ],
            Self::WriterTokenSourceRange { .. } => &["byteOffset", "byteLength", "line", "column"],
            Self::OutputSpan { .. } => &["outputRange", "origin"],
            Self::OutputRange { .. } => &["start", "len"],
            Self::Node { node, .. } => match node {
                CemTreeAstNode::Document { .. } => &["kind", "children", "sourceMap"],
                CemTreeAstNode::Element { .. } => {
                    &["kind", "name", "attributes", "children", "sourceMap"]
                }
                CemTreeAstNode::Text { .. } => &["kind", "value", "sourceMap"],
                CemTreeAstNode::Whitespace { .. }
                | CemTreeAstNode::Comment { .. }
                | CemTreeAstNode::Cdata { .. }
                | CemTreeAstNode::RawText { .. } => &["kind", "data", "sourceMap"],
                CemTreeAstNode::ProcessingInstruction { .. } => {
                    &["kind", "name", "target", "data", "sourceMap"]
                }
                CemTreeAstNode::Error { .. } => &["kind", "code", "sourceMap"],
                CemTreeAstNode::WriterToken { .. } => &[
                    "kind",
                    "writerKind",
                    "tokenKind",
                    "text",
                    "role",
                    "style",
                    "value",
                    "outputSpan",
                    "sourceMap",
                ],
            },
            Self::Attribute { .. } => &["kind", "name", "value", "sourceMap"],
            Self::FormattedTree { .. } => &[
                "kind",
                "contentType",
                "schema",
                "category",
                "mode",
                "canonical",
                "formatterProfile",
                "formatNodes",
                "nodes",
            ],
            Self::MaterializedTree { .. } => &[
                "kind",
                "contentType",
                "schema",
                "category",
                "mode",
                "canonical",
                "formatterProfile",
                "nodes",
            ],
            Self::FormattedNode { node, .. } => match node {
                CemTreeAstNode::Document { .. } => &[
                    "kind",
                    "formatLayout",
                    "formatContentBoundary",
                    "children",
                    "formatBeforeClose",
                    "sourceMap",
                ],
                CemTreeAstNode::Element { .. } => &[
                    "kind",
                    "name",
                    "formatLayout",
                    "formatBeforeAttributes",
                    "formatBetweenAttributes",
                    "attributes",
                    "formatContentBoundary",
                    "children",
                    "formatBeforeClose",
                    "sourceMap",
                ],
                _ => match node {
                    CemTreeAstNode::Text { .. } => &["kind", "value", "sourceMap"],
                    CemTreeAstNode::Whitespace { .. }
                    | CemTreeAstNode::Comment { .. }
                    | CemTreeAstNode::Cdata { .. }
                    | CemTreeAstNode::RawText { .. } => &["kind", "data", "sourceMap"],
                    CemTreeAstNode::ProcessingInstruction { .. } => {
                        &["kind", "name", "target", "data", "sourceMap"]
                    }
                    CemTreeAstNode::Error { .. } => &["kind", "code", "sourceMap"],
                    CemTreeAstNode::WriterToken { .. } => &[
                        "kind",
                        "writerKind",
                        "tokenKind",
                        "text",
                        "role",
                        "style",
                        "value",
                        "outputSpan",
                        "sourceMap",
                    ],
                    CemTreeAstNode::Document { .. } | CemTreeAstNode::Element { .. } => {
                        unreachable!()
                    }
                },
            },
            Self::FormattedAttribute { .. } => {
                &["kind", "name", "value", "formatBefore", "sourceMap"]
            }
            Self::FormatOperation { .. } => &[
                "kind",
                "name",
                "formatterRole",
                "formatterProfile",
                "colorRole",
                "value",
                "sourceMap",
            ],
            Self::NodeFormatOperation { operation, .. } => match &operation.kind {
                CemtNodeFormatOperationKind::Layout(_) => &[
                    "kind",
                    "name",
                    "formatterRole",
                    "colorRole",
                    "value",
                    "sourceMap",
                ],
                _ => &[
                    "kind",
                    "value",
                    "formatterOwned",
                    "formatterRole",
                    "colorRole",
                    "sourceMap",
                ],
            },
        }
    }

    pub fn field(&self, name: &str) -> Option<CemtEvaluatorValueRef<'a>> {
        match self {
            Self::Package { record } => (*record).field(name),
            Self::CsvDocument { document } => csv_document_evaluator_field(document, name),
            Self::CsvSource { source } => csv_source_evaluator_field(source, name),
            Self::CsvEncodingReport { report } => csv_encoding_report_evaluator_field(report, name),
            Self::CsvDialect { dialect } => csv_dialect_evaluator_field(dialect, name),
            Self::CsvParseFact { fact } => csv_parse_fact_evaluator_field(fact, name),
            Self::CsvParseFactSourceRange { fact } => {
                csv_parse_fact_source_range_evaluator_field(fact, name)
            }
            Self::CsvRow { row } => csv_row_evaluator_field(row, name),
            Self::CsvField { field } => csv_field_evaluator_field(field, name),
            Self::CsvSourceRange { range } => csv_source_range_evaluator_field(*range, name),
            Self::GenericDataCsvDocument { document } => {
                generic_data_csv_document_evaluator_field(document, name)
            }
            Self::GenericDataCsvSource { document } => {
                generic_data_csv_source_evaluator_field(document, name)
            }
            Self::GenericDataCsvEncodingReport => {
                generic_data_csv_encoding_report_evaluator_field(name)
            }
            Self::GenericDataCsvDialect { document } => {
                generic_data_csv_dialect_evaluator_field(document, name)
            }
            Self::GenericDataCsvHeaderRow { document } => {
                generic_data_csv_header_row_evaluator_field(document, name)
            }
            Self::GenericDataCsvMappingRow {
                document,
                entries,
                index,
            } => generic_data_csv_mapping_row_evaluator_field(document, entries, *index, name),
            Self::GenericDataCsvValueRow { value, index } => {
                generic_data_csv_value_row_evaluator_field(value, *index, name)
            }
            Self::GenericDataCsvHeaderField { entry, index } => {
                generic_data_csv_header_field_evaluator_field(entry, *index, name)
            }
            Self::GenericDataCsvMappingField {
                header_entry,
                entries,
                index,
            } => {
                generic_data_csv_mapping_field_evaluator_field(header_entry, entries, *index, name)
            }
            Self::GenericDataCsvValueField { value, index } => {
                generic_data_csv_value_field_evaluator_field(value, *index, name)
            }
            Self::MarkdownDocument { document } => {
                markdown_document_evaluator_field(document, name)
            }
            Self::MarkdownSource { source } => markdown_source_evaluator_field(source, name),
            Self::MarkdownEncodingReport { report } => {
                markdown_encoding_report_evaluator_field(report, name)
            }
            Self::MarkdownEncodingFact { fact } => {
                markdown_encoding_fact_evaluator_field(fact, name)
            }
            Self::MarkdownVariantFact { fact } => markdown_variant_fact_evaluator_field(fact, name),
            Self::MarkdownParseFact { fact } => markdown_parse_fact_evaluator_field(fact, name),
            Self::MarkdownEvent { event } => markdown_event_evaluator_field(event, name),
            Self::MarkdownSourceRange { range } => {
                markdown_source_range_evaluator_field(*range, name)
            }
            Self::RelaxNgDocument { document } => relax_ng_document_evaluator_field(document, name),
            Self::RelaxNgSource { source } => relax_ng_source_evaluator_field(source, name),
            Self::RelaxNgCompactToken {
                token,
                content_type,
            } => relax_ng_compact_token_evaluator_field(token, content_type, name),
            Self::RelaxNgFact { fact } => relax_ng_fact_evaluator_field(fact, name),
            Self::XmlFamilyDocument { document } => {
                xml_family_document_evaluator_field(*document, name)
            }
            Self::XmlFamilySource { document } => {
                xml_family_source_evaluator_field(*document, name)
            }
            Self::XmlFamilyEncodingReport { document } => {
                xml_family_encoding_report_evaluator_field(*document, name)
            }
            Self::XmlFamilyFact { document, index } => {
                xml_family_fact_evaluator_field(*document, *index, name)
            }
            Self::XmlFamilyEvent { document, index } => {
                xml_family_event_evaluator_field(*document, *index, name)
            }
            Self::XmlAttribute { attribute } => xml_attribute_evaluator_field(attribute, name),
            Self::HtmlAttribute { attribute } => html_attribute_evaluator_field(attribute, name),
            Self::XmlFamilyMarkupToken {
                token,
                content_type,
                package,
            } => xml_family_markup_token_evaluator_field(token, content_type, *package, name),
            Self::XmlFamilySourceRange {
                byte_offset,
                byte_length,
                line,
                column,
            } => xml_family_source_range_evaluator_field(
                *byte_offset,
                *byte_length,
                *line,
                *column,
                name,
            ),
            Self::YamlDocument { document } => yaml_document_evaluator_field(document, name),
            Self::YamlSource { source } => yaml_source_evaluator_field(source, name),
            Self::YamlEncodingReport { report } => {
                yaml_encoding_report_evaluator_field(report, name)
            }
            Self::YamlParseFact { fact } => yaml_parse_fact_evaluator_field(fact, name),
            Self::YamlParseFactSourceRange { fact } => {
                yaml_parse_fact_source_range_evaluator_field(fact, name)
            }
            Self::YamlDirective { directive } => yaml_directive_evaluator_field(directive, name),
            Self::YamlComment { comment } => yaml_comment_evaluator_field(comment, name),
            Self::YamlStreamDocument { document } => {
                yaml_stream_document_evaluator_field(document, name)
            }
            Self::YamlNode { node } => yaml_node_evaluator_field(node, name),
            Self::YamlPair { pair } => yaml_pair_evaluator_field(pair, name),
            Self::YamlSourceRange { range } => yaml_source_range_evaluator_field(*range, name),
            Self::GenericDataYamlDocument { document } => {
                generic_data_yaml_document_evaluator_field(document, name)
            }
            Self::GenericDataYamlSource { document } => {
                generic_data_yaml_source_evaluator_field(document, name)
            }
            Self::GenericDataYamlEncodingReport => {
                generic_data_yaml_encoding_report_evaluator_field(name)
            }
            Self::GenericDataYamlStreamDocument { document } => {
                generic_data_yaml_stream_document_evaluator_field(document, name)
            }
            Self::GenericDataYamlNode { value } => {
                generic_data_yaml_node_evaluator_field(value, name)
            }
            Self::GenericDataYamlPair { entry } => {
                generic_data_yaml_pair_evaluator_field(entry, name)
            }
            Self::JsonDocument { document } => json_document_evaluator_field(document, name),
            Self::JsonValue { value } => json_value_evaluator_field(value, name),
            Self::JsonMember { member } => json_member_evaluator_field(member, name),
            Self::JsonSourceRange { range } => json_source_range_evaluator_field(*range, name),
            Self::JsonSchemaDocument { document } => {
                json_schema_document_evaluator_field(document, name)
            }
            Self::JsonSchemaSource { source } => json_schema_source_evaluator_field(source, name),
            Self::JsonSchemaParseFact { fact } => {
                json_schema_parse_fact_evaluator_field(fact, name)
            }
            Self::JsonSchemaParseFactSourceRange { fact } => {
                json_schema_parse_fact_source_range_evaluator_field(fact, name)
            }
            Self::JsonSchemaDialectFact { fact } => {
                json_schema_dialect_fact_evaluator_field(fact, name)
            }
            Self::GenericDataJsonDocument { document } => {
                generic_data_json_document_evaluator_field(document, name)
            }
            Self::GenericDataJsonValue { value } => {
                generic_data_json_value_evaluator_field(value, name)
            }
            Self::GenericDataJsonMember { entry } => {
                generic_data_json_member_evaluator_field(entry, name)
            }
            Self::GenericDataJsonDocumentSequenceRoot { documents } => {
                generic_data_json_document_sequence_root_evaluator_field(documents, name)
            }
            Self::GenericDataJsonMissingRoot { source_range } => {
                generic_data_json_null_evaluator_field(Some(source_range), name)
            }
            Self::GenericDataJsonGeneratedNull => {
                generic_data_json_null_evaluator_field(None, name)
            }
            Self::GenericDataSourceRange { source_range } => {
                generic_data_source_range_evaluator_field(Some(source_range), name)
            }
            Self::GenericDataGeneratedSourceRange => {
                generic_data_source_range_evaluator_field(None, name)
            }
            Self::WriterTokenStyle { style } => writer_token_style_evaluator_field(style, name),
            Self::WriterTokenMetadata { metadata } => {
                writer_token_metadata_evaluator_field(metadata, name)
            }
            Self::WriterTokenSourceRange { range } => {
                writer_token_source_range_evaluator_field(range, name)
            }
            Self::OutputSpan { output_span } => match name {
                "outputRange" => Some(CemtEvaluatorValueRef::Record(
                    CemtEvaluatorRecordRef::OutputRange {
                        range: &output_span.output_range,
                    },
                )),
                "origin" => Some(CemtEvaluatorValueRef::SourceMap(&output_span.origin)),
                _ => None,
            },
            Self::OutputRange { range } => match name {
                "start" => Some(CemtEvaluatorValueRef::Number(
                    CemtEvaluatorNumber::unsigned_integer(range.start),
                )),
                "len" => Some(CemtEvaluatorValueRef::Number(
                    CemtEvaluatorNumber::unsigned_integer(u64::from(range.len)),
                )),
                _ => None,
            },
            Self::Node { node, path } => cemt_evaluator_node_field(node, path, name),
            Self::Attribute { attribute, .. } => match name {
                "kind" => Some(CemtEvaluatorValueRef::String("attribute")),
                "name" => Some(CemtEvaluatorValueRef::String(&attribute.name)),
                "value" => Some(match attribute.value.as_deref() {
                    Some(value) => CemtEvaluatorValueRef::String(value),
                    None => CemtEvaluatorValueRef::Null,
                }),
                "sourceMap" => Some(CemtEvaluatorValueRef::SourceMap(&attribute.source)),
                _ => None,
            },
            Self::FormattedTree { subject, overlay } => {
                cemt_evaluator_formatted_tree_field(*subject, overlay, name)
            }
            Self::MaterializedTree { artifact } => {
                cemt_evaluator_materialized_tree_field(artifact, name)
            }
            Self::FormattedNode {
                node,
                path,
                overlay,
            } => cemt_evaluator_formatted_node_field(node, path, overlay, name),
            Self::FormattedAttribute {
                attribute,
                path,
                overlay,
            } => cemt_evaluator_formatted_attribute_field(attribute, path, overlay, name),
            Self::FormatOperation { operation, .. } => {
                cemt_evaluator_format_operation_field(operation, name)
            }
            Self::NodeFormatOperation { operation, .. } => {
                cemt_evaluator_node_format_operation_field(operation, name)
            }
        }
    }
}

fn cemt_evaluator_materialized_tree_field<'a>(
    artifact: &'a CemtMaterializedTreeArtifact,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    let formatter = match artifact.pipeline() {
        CemtMaterializedTreePipeline::Formatted { formatter }
        | CemtMaterializedTreePipeline::Colored { formatter, .. } => Some(formatter),
        CemtMaterializedTreePipeline::Raw { .. } => None,
    };
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String("cem-tree")),
        "contentType" => Some(CemtEvaluatorValueRef::String(
            &artifact.identity().content_type,
        )),
        "schema" => Some(CemtEvaluatorValueRef::String(&artifact.identity().schema)),
        "category" => Some(CemtEvaluatorValueRef::String(&artifact.identity().category)),
        "mode" => Some(CemtEvaluatorValueRef::String("document")),
        "canonical" => Some(CemtEvaluatorValueRef::Boolean(
            formatter.and_then(|producer| producer.profile()) == Some("compact"),
        )),
        "formatterProfile" => Some(match formatter.and_then(|producer| producer.profile()) {
            Some(profile) => CemtEvaluatorValueRef::String(profile),
            None => CemtEvaluatorValueRef::Null,
        }),
        "nodes" => Some(
            CemtTreeSubjectRef {
                owner: artifact.owner().as_ref(),
            }
            .evaluator_view(),
        ),
        _ => None,
    }
}

fn csv_document_evaluator_field<'a>(
    document: &'a CsvDocumentAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String("csv-table")),
        "source" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::CsvSource {
                source: &document.source,
            },
        )),
        "encoding" => Some(CemtEvaluatorValueRef::String(&document.encoding)),
        "encodingReport" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::CsvEncodingReport {
                report: &document.encoding_report,
            },
        )),
        "delimiter" => Some(CemtEvaluatorValueRef::String(&document.delimiter)),
        "header" => Some(CemtEvaluatorValueRef::String(&document.header)),
        "dialect" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::CsvDialect {
                dialect: &document.dialect,
            },
        )),
        "parseFacts" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::CsvParseFacts {
                facts: &document.parse_facts,
            },
        )),
        "rows" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::CsvRows {
                rows: &document.rows,
            },
        )),
        "lineEnding" => document
            .line_ending
            .as_deref()
            .map(CemtEvaluatorValueRef::String),
        _ => None,
    }
}

fn csv_source_evaluator_field<'a>(
    source: &'a CsvDocumentSource,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "uri" => Some(CemtEvaluatorValueRef::String(&source.uri)),
        "contentType" => Some(CemtEvaluatorValueRef::String(&source.content_type)),
        "mediaType" => Some(CemtEvaluatorValueRef::String(&source.media_type)),
        "parameters" => Some(CemtEvaluatorValueRef::StringMap(&source.parameters)),
        "byteLength" => Some(usize_evaluator_value(source.byte_length)),
        _ => None,
    }
}

fn csv_encoding_report_evaluator_field<'a>(
    report: &'a CsvEncodingReportAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "declaredCharset" => report
            .declared_charset
            .as_deref()
            .map(CemtEvaluatorValueRef::String),
        "normalizedCharset" => Some(CemtEvaluatorValueRef::String(&report.normalized_charset)),
        "decoderStatus" => Some(CemtEvaluatorValueRef::String(&report.decoder_status)),
        "invalidByteOffset" => report.invalid_byte_offset.map(u64_evaluator_value),
        _ => None,
    }
}

fn csv_dialect_evaluator_field<'a>(
    dialect: &'a CsvDialectAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "delimiter" => Some(CemtEvaluatorValueRef::String(&dialect.delimiter)),
        "quote" => Some(CemtEvaluatorValueRef::String(&dialect.quote)),
        "escape" => Some(CemtEvaluatorValueRef::String(&dialect.escape)),
        "header" => Some(CemtEvaluatorValueRef::String(&dialect.header)),
        "lineEnding" => dialect
            .line_ending
            .as_deref()
            .map(CemtEvaluatorValueRef::String),
        _ => None,
    }
}

fn csv_parse_fact_evaluator_field<'a>(
    fact: &'a CsvDocumentParseFact,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(fact.kind.as_str())),
        "contract" => Some(optional_string_evaluator_value(fact.contract.as_deref())),
        "behavior" => Some(optional_string_evaluator_value(fact.behavior.as_deref())),
        "diagnosticCode" => Some(optional_string_evaluator_value(
            fact.diagnostic_code.as_deref(),
        )),
        "diagnosticSeverity" => Some(optional_string_evaluator_value(
            fact.diagnostic_severity.as_deref(),
        )),
        "recoverable" => Some(CemtEvaluatorValueRef::Boolean(fact.recoverable)),
        "fatal" => Some(CemtEvaluatorValueRef::Boolean(fact.fatal)),
        "parameter" => Some(optional_string_evaluator_value(fact.parameter.as_deref())),
        "actual" => Some(optional_string_evaluator_value(fact.actual.as_deref())),
        "expected" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::Strings {
                values: &fact.expected,
            },
        )),
        "rowIndex" => Some(optional_usize_evaluator_value(fact.row_index)),
        "fieldIndex" => Some(optional_usize_evaluator_value(fact.field_index)),
        "expectedCount" => Some(optional_usize_evaluator_value(fact.expected_count)),
        "actualCount" => Some(optional_usize_evaluator_value(fact.actual_count)),
        "line" => Some(optional_u32_evaluator_value(fact.line)),
        "column" => Some(optional_u32_evaluator_value(fact.column)),
        "byteOffset" => Some(optional_u64_evaluator_value(fact.byte_offset)),
        "message" => Some(CemtEvaluatorValueRef::String(&fact.message)),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::CsvParseFactSourceRange { fact },
        )),
        _ => None,
    }
}

fn csv_parse_fact_source_range_evaluator_field(
    fact: &CsvDocumentParseFact,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'static>> {
    match name {
        "byteOffset" => Some(optional_u64_evaluator_value(fact.byte_offset)),
        "line" => Some(optional_u32_evaluator_value(fact.line)),
        "column" => Some(optional_u32_evaluator_value(fact.column)),
        _ => None,
    }
}

fn csv_row_evaluator_field<'a>(
    row: &'a CsvRecordAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "index" => Some(usize_evaluator_value(row.index)),
        "fieldCount" => Some(usize_evaluator_value(row.fields.len())),
        "byteOffset" => Some(u64_evaluator_value(row.range.start.byte_offset)),
        "byteLength" => Some(u64_evaluator_value(row.range.byte_length())),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::CsvSourceRange { range: row.range },
        )),
        "sourceMap" => Some(CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(
            row.range.source_map(),
        ))),
        "recordEndingSourceRange" => Some(match row.record_ending {
            Some(range) => {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::CsvSourceRange { range })
            }
            None => CemtEvaluatorValueRef::Null,
        }),
        "recordEndingSourceMap" => Some(match row.record_ending {
            Some(range) => CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(range.source_map())),
            None => CemtEvaluatorValueRef::Null,
        }),
        "fields" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::CsvFields {
                fields: &row.fields,
            },
        )),
        _ => None,
    }
}

fn csv_field_evaluator_field<'a>(
    field: &'a CsvFieldAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "index" => Some(usize_evaluator_value(field.index)),
        "value" => Some(CemtEvaluatorValueRef::String(&field.value)),
        "lexeme" => Some(CemtEvaluatorValueRef::String(&field.lexeme)),
        "quoted" => Some(CemtEvaluatorValueRef::Boolean(field.quoted)),
        "byteOffset" => Some(u64_evaluator_value(field.range.start.byte_offset)),
        "byteLength" => Some(u64_evaluator_value(field.range.byte_length())),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::CsvSourceRange { range: field.range },
        )),
        "sourceMap" => Some(CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(
            field.range.source_map(),
        ))),
        "delimiterBeforeSourceRange" => Some(match field.delimiter_before {
            Some(range) => {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::CsvSourceRange { range })
            }
            None => CemtEvaluatorValueRef::Null,
        }),
        "delimiterBeforeSourceMap" => Some(match field.delimiter_before {
            Some(range) => CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(range.source_map())),
            None => CemtEvaluatorValueRef::Null,
        }),
        _ => None,
    }
}

fn csv_source_range_evaluator_field(
    range: CsvSourceRange,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'static>> {
    let value = match name {
        "byteOffset" => range.start.byte_offset,
        "byteLength" => range.byte_length(),
        "line" => u64::from(range.start.line),
        "column" => u64::from(range.start.column),
        "endLine" => u64::from(range.end.line),
        "endColumn" => u64::from(range.end.column),
        _ => return None,
    };
    Some(u64_evaluator_value(value))
}

fn generic_data_csv_document_evaluator_field<'a>(
    document: &'a GenericDataDocumentAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String("csv-table")),
        "source" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::GenericDataCsvSource { document },
        )),
        "encoding" => Some(CemtEvaluatorValueRef::String("utf-8")),
        "encodingReport" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::GenericDataCsvEncodingReport,
        )),
        "delimiter" => Some(CemtEvaluatorValueRef::String(",")),
        "header" => Some(CemtEvaluatorValueRef::String(
            if generic_data_csv_has_header(document) {
                "present"
            } else {
                "absent"
            },
        )),
        "dialect" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::GenericDataCsvDialect { document },
        )),
        "parseFacts" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::Empty,
        )),
        "rows" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::GenericDataCsvRows { document },
        )),
        "lineEnding" => document
            .line_ending
            .as_deref()
            .map(CemtEvaluatorValueRef::String),
        _ => None,
    }
}

fn generic_data_csv_source_evaluator_field<'a>(
    document: &'a GenericDataDocumentAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    let source = &document.source;
    match name {
        "uri" => Some(CemtEvaluatorValueRef::String(&source.uri)),
        "contentType" => Some(CemtEvaluatorValueRef::String(&source.content_type)),
        "mediaType" => Some(CemtEvaluatorValueRef::String(&source.media_type)),
        "parameters" => Some(CemtEvaluatorValueRef::StringMap(&source.parameters)),
        "byteLength" => Some(usize_evaluator_value(source.byte_length)),
        _ => None,
    }
}

fn generic_data_csv_encoding_report_evaluator_field(
    name: &str,
) -> Option<CemtEvaluatorValueRef<'static>> {
    match name {
        "normalizedCharset" => Some(CemtEvaluatorValueRef::String("utf-8")),
        "decoderStatus" => Some(CemtEvaluatorValueRef::String("decoded")),
        _ => None,
    }
}

fn generic_data_csv_dialect_evaluator_field<'a>(
    document: &'a GenericDataDocumentAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "delimiter" => Some(CemtEvaluatorValueRef::String(",")),
        "quote" => Some(CemtEvaluatorValueRef::String("\"")),
        "escape" => Some(CemtEvaluatorValueRef::String("double-quote")),
        "header" => Some(CemtEvaluatorValueRef::String(
            if generic_data_csv_has_header(document) {
                "present"
            } else {
                "absent"
            },
        )),
        "lineEnding" => document
            .line_ending
            .as_deref()
            .map(CemtEvaluatorValueRef::String),
        _ => None,
    }
}

fn generic_data_csv_header_row_evaluator_field<'a>(
    document: &'a GenericDataDocumentAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    let range = generic_data_csv_header_entry(document, 0).map(|entry| entry.key.source_range());
    generic_data_csv_row_common_field(
        0,
        generic_data_csv_header_count(document),
        range,
        name,
        || CemtEvaluatorSequenceRef::GenericDataCsvHeaderFields { document },
    )
}

fn generic_data_csv_mapping_row_evaluator_field<'a>(
    document: &'a GenericDataDocumentAst,
    entries: &'a [GenericDataMappingEntryAst],
    index: usize,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    generic_data_csv_row_common_field(
        index,
        generic_data_csv_header_count(document),
        entries.first().map(|entry| &entry.source_range),
        name,
        || CemtEvaluatorSequenceRef::GenericDataCsvMappingFields { document, entries },
    )
}

fn generic_data_csv_value_row_evaluator_field<'a>(
    value: &'a GenericDataValueAst,
    index: usize,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    let field_count = match value {
        GenericDataValueAst::Sequence { items, .. } => items.len(),
        _ => 1,
    };
    generic_data_csv_row_common_field(index, field_count, Some(value.source_range()), name, || {
        CemtEvaluatorSequenceRef::GenericDataCsvValueFields { value }
    })
}

fn generic_data_csv_row_common_field<'a>(
    index: usize,
    field_count: usize,
    range: Option<&'a GenericDataSourceRangeAst>,
    name: &str,
    fields: impl FnOnce() -> CemtEvaluatorSequenceRef<'a>,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "index" => Some(usize_evaluator_value(index)),
        "fieldCount" => Some(usize_evaluator_value(field_count)),
        "byteOffset" => Some(u64_evaluator_value(
            range.map_or(0, |range| range.byte_offset),
        )),
        "byteLength" => Some(u64_evaluator_value(
            range.map_or(0, |range| range.byte_length),
        )),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(match range {
            Some(source_range) => CemtEvaluatorRecordRef::GenericDataSourceRange { source_range },
            None => CemtEvaluatorRecordRef::GenericDataGeneratedSourceRange,
        })),
        "sourceMap" => Some(match range {
            Some(range) => generic_data_source_map_evaluator_value(range),
            None => CemtEvaluatorValueRef::Null,
        }),
        "recordEndingSourceRange" | "recordEndingSourceMap" => Some(CemtEvaluatorValueRef::Null),
        "fields" => Some(CemtEvaluatorValueRef::Sequence(fields())),
        _ => None,
    }
}

fn generic_data_csv_header_field_evaluator_field<'a>(
    entry: &'a GenericDataMappingEntryAst,
    index: usize,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    generic_data_csv_field_common_field(
        index,
        Some(entry.key.source_range()),
        generic_data_csv_scalar_text(&entry.key),
        name,
    )
}

fn generic_data_csv_mapping_field_evaluator_field<'a>(
    header_entry: &'a GenericDataMappingEntryAst,
    entries: &'a [GenericDataMappingEntryAst],
    index: usize,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    let header = generic_data_csv_scalar_text(&header_entry.key);
    let entry = entries
        .iter()
        .find(|entry| generic_data_csv_scalar_text(&entry.key) == header);
    generic_data_csv_field_common_field(
        index,
        entry.map(|entry| entry.value.source_range()),
        entry
            .map(|entry| generic_data_csv_scalar_text(&entry.value))
            .unwrap_or_default(),
        name,
    )
}

fn generic_data_csv_value_field_evaluator_field<'a>(
    value: &'a GenericDataValueAst,
    index: usize,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    generic_data_csv_field_common_field(
        index,
        Some(value.source_range()),
        generic_data_csv_scalar_text(value),
        name,
    )
}

fn generic_data_csv_field_common_field<'a>(
    index: usize,
    range: Option<&'a GenericDataSourceRangeAst>,
    value: String,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "index" => Some(usize_evaluator_value(index)),
        "value" | "lexeme" => Some(CemtEvaluatorValueRef::OwnedString(Arc::from(value))),
        "quoted" => Some(CemtEvaluatorValueRef::Boolean(false)),
        "byteOffset" => Some(u64_evaluator_value(
            range.map_or(0, |range| range.byte_offset),
        )),
        "byteLength" => Some(u64_evaluator_value(
            range.map_or(0, |range| range.byte_length),
        )),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(match range {
            Some(source_range) => CemtEvaluatorRecordRef::GenericDataSourceRange { source_range },
            None => CemtEvaluatorRecordRef::GenericDataGeneratedSourceRange,
        })),
        "sourceMap" => Some(match range {
            Some(range) => generic_data_source_map_evaluator_value(range),
            None => CemtEvaluatorValueRef::Null,
        }),
        "delimiterBeforeSourceRange" | "delimiterBeforeSourceMap" => {
            Some(CemtEvaluatorValueRef::Null)
        }
        _ => None,
    }
}

fn generic_data_csv_roots(document: &GenericDataDocumentAst) -> Vec<&GenericDataValueAst> {
    document
        .documents
        .iter()
        .filter_map(|document| document.root.as_ref())
        .collect()
}

fn generic_data_csv_mapping_rows(
    document: &GenericDataDocumentAst,
) -> Option<Vec<&[GenericDataMappingEntryAst]>> {
    let roots = generic_data_csv_roots(document);
    if roots.len() == 1 {
        return match roots[0] {
            GenericDataValueAst::Mapping { entries, .. } => Some(vec![entries]),
            GenericDataValueAst::Sequence { items, .. }
                if items
                    .iter()
                    .all(|item| matches!(item, GenericDataValueAst::Mapping { .. })) =>
            {
                Some(
                    items
                        .iter()
                        .filter_map(|item| match item {
                            GenericDataValueAst::Mapping { entries, .. } => {
                                Some(entries.as_slice())
                            }
                            _ => None,
                        })
                        .collect(),
                )
            }
            _ => None,
        };
    }
    roots
        .iter()
        .all(|value| matches!(value, GenericDataValueAst::Mapping { .. }))
        .then(|| {
            roots
                .into_iter()
                .filter_map(|value| match value {
                    GenericDataValueAst::Mapping { entries, .. } => Some(entries.as_slice()),
                    _ => None,
                })
                .collect()
        })
}

fn generic_data_csv_has_header(document: &GenericDataDocumentAst) -> bool {
    generic_data_csv_mapping_rows(document).is_some()
}

fn generic_data_csv_header_entry(
    document: &GenericDataDocumentAst,
    index: usize,
) -> Option<&GenericDataMappingEntryAst> {
    let mut names = BTreeSet::new();
    let mut current = 0usize;
    for entries in generic_data_csv_mapping_rows(document)? {
        for entry in entries {
            if names.insert(generic_data_csv_scalar_text(&entry.key)) {
                if current == index {
                    return Some(entry);
                }
                current = current.saturating_add(1);
            }
        }
    }
    None
}

fn generic_data_csv_header_count(document: &GenericDataDocumentAst) -> usize {
    let mut names = BTreeSet::new();
    for entries in generic_data_csv_mapping_rows(document).unwrap_or_default() {
        for entry in entries {
            names.insert(generic_data_csv_scalar_text(&entry.key));
        }
    }
    names.len()
}

fn generic_data_csv_row_count(document: &GenericDataDocumentAst) -> usize {
    if let Some(rows) = generic_data_csv_mapping_rows(document) {
        return rows.len().saturating_add(1);
    }
    let roots = generic_data_csv_roots(document);
    match roots.as_slice() {
        [GenericDataValueAst::Sequence { items, .. }]
            if items
                .iter()
                .any(|item| matches!(item, GenericDataValueAst::Mapping { .. })) =>
        {
            0
        }
        [GenericDataValueAst::Sequence { items, .. }] => items.len(),
        [_] => 1,
        roots
            if roots
                .iter()
                .any(|value| matches!(value, GenericDataValueAst::Mapping { .. })) =>
        {
            0
        }
        roots => roots.len(),
    }
}

fn generic_data_csv_row_evaluator_value<'a>(
    document: &'a GenericDataDocumentAst,
    index: usize,
) -> Option<CemtEvaluatorValueRef<'a>> {
    if let Some(rows) = generic_data_csv_mapping_rows(document) {
        if index == 0 {
            return Some(CemtEvaluatorValueRef::Record(
                CemtEvaluatorRecordRef::GenericDataCsvHeaderRow { document },
            ));
        }
        let entries = *rows.get(index.saturating_sub(1))?;
        return Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::GenericDataCsvMappingRow {
                document,
                entries,
                index,
            },
        ));
    }
    let roots = generic_data_csv_roots(document);
    let value = match roots.as_slice() {
        [GenericDataValueAst::Sequence { items, .. }] => items.get(index)?,
        [value] if index == 0 => *value,
        roots => *roots.get(index)?,
    };
    Some(CemtEvaluatorValueRef::Record(
        CemtEvaluatorRecordRef::GenericDataCsvValueRow { value, index },
    ))
}

fn generic_data_csv_scalar_text(value: &GenericDataValueAst) -> String {
    match value {
        GenericDataValueAst::String { value, .. } => value.clone(),
        GenericDataValueAst::Number { lexeme, .. } => lexeme.clone(),
        GenericDataValueAst::Boolean { value, .. } => value.to_string(),
        GenericDataValueAst::Null { .. } => String::new(),
        GenericDataValueAst::Alias { alias, .. } => alias.clone().unwrap_or_default(),
        GenericDataValueAst::Mapping { .. } | GenericDataValueAst::Sequence { .. } => String::new(),
    }
}

fn relax_ng_document_evaluator_field<'a>(
    document: &'a RelaxNgDocumentAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String("relax-ng-document")),
        "contentType" => Some(CemtEvaluatorValueRef::String(
            document.syntax_kind.content_type(),
        )),
        "schema" => Some(CemtEvaluatorValueRef::String(RELAX_NG_SCHEMA_URI)),
        "category" => Some(CemtEvaluatorValueRef::String(
            document.syntax_kind.category(),
        )),
        "syntaxKind" => Some(CemtEvaluatorValueRef::String(document.syntax_kind.as_str())),
        "source" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::RelaxNgSource {
                source: &document.source,
            },
        )),
        "xmlEvents" => Some(CemtEvaluatorValueRef::Sequence(
            document
                .xml_document
                .as_ref()
                .map(|xml_document| CemtEvaluatorSequenceRef::XmlFamilyEvents {
                    document: XmlFamilyDocumentCemtSubjectRef::xml(xml_document),
                })
                .unwrap_or(CemtEvaluatorSequenceRef::Empty),
        )),
        "compactTokens" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::RelaxNgCompactTokens { document },
        )),
        "parseFacts" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::RelaxNgFacts {
                facts: &document.facts,
            },
        )),
        "lineEnding" => Some(optional_string_evaluator_value(
            document.line_ending.as_deref(),
        )),
        _ => None,
    }
}

fn relax_ng_source_evaluator_field<'a>(
    source: &'a RelaxNgDocumentSource,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "uri" => Some(CemtEvaluatorValueRef::String(&source.uri)),
        "contentType" => Some(CemtEvaluatorValueRef::String(&source.content_type)),
        "mediaType" => Some(CemtEvaluatorValueRef::String(&source.media_type)),
        "parameters" => Some(CemtEvaluatorValueRef::StringMap(&source.parameters)),
        "byteLength" => Some(usize_evaluator_value(source.byte_length)),
        _ => None,
    }
}

fn relax_ng_compact_token_evaluator_field<'a>(
    token: &'a RelaxNgCompactTokenAst,
    content_type: &str,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "index" => Some(usize_evaluator_value(token.index)),
        "kind" => Some(CemtEvaluatorValueRef::String(token.kind.as_str())),
        "lexeme" => Some(CemtEvaluatorValueRef::String(&token.lexeme)),
        "depth" => Some(usize_evaluator_value(token.depth)),
        "role" => Some(CemtEvaluatorValueRef::String(token.kind.role())),
        "sourceRange" => Some(xml_family_source_range_evaluator_value(
            token.source_range.start.byte_offset,
            token.source_range.byte_length,
            token.source_range.start.line,
            token.source_range.start.column,
        )),
        "sourceMap" => Some(CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(
            xml_family_source_map_from_coordinates(
                token.source_range.start.byte_offset,
                token.source_range.byte_length,
                content_type,
            ),
        ))),
        _ => None,
    }
}

fn relax_ng_fact_evaluator_field<'a>(
    fact: &'a RelaxNgFact,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(fact.kind.as_str())),
        "syntaxKind" => Some(CemtEvaluatorValueRef::String(fact.syntax_kind.as_str())),
        "sourceRange" => Some(
            fact.source_range
                .map(|range| {
                    xml_family_source_range_evaluator_value(
                        range.start.byte_offset,
                        range.byte_length,
                        range.start.line,
                        range.start.column,
                    )
                })
                .unwrap_or(CemtEvaluatorValueRef::Null),
        ),
        "message" => Some(CemtEvaluatorValueRef::String(&fact.message)),
        "value" => Some(optional_string_evaluator_value(fact.value.as_deref())),
        _ => None,
    }
}

fn xml_family_document_evaluator_field_names(
    document: XmlFamilyDocumentCemtSubjectRef<'_>,
) -> &'static [&'static str] {
    match document {
        XmlFamilyDocumentCemtSubjectRef::Xml(_) => &[
            "kind",
            "contentType",
            "schema",
            "source",
            "resourceKind",
            "encodingReport",
            "parseFacts",
            "events",
            "lineEnding",
        ],
        XmlFamilyDocumentCemtSubjectRef::Html(_) => &[
            "kind",
            "contentType",
            "schema",
            "category",
            "source",
            "documentMode",
            "encodingReport",
            "parseFacts",
            "events",
            "lineEnding",
            "recoveryCount",
        ],
        XmlFamilyDocumentCemtSubjectRef::Css(_) => &[
            "kind",
            "contentType",
            "schema",
            "category",
            "source",
            "entryMode",
            "encodingReport",
            "parseFacts",
            "events",
            "lineEnding",
            "recoveryCount",
        ],
        XmlFamilyDocumentCemtSubjectRef::Xhtml(_) | XmlFamilyDocumentCemtSubjectRef::Svg(_) => &[
            "kind",
            "contentType",
            "schema",
            "category",
            "source",
            "resourceKind",
            "encodingReport",
            "parseFacts",
            "events",
            "lineEnding",
        ],
        XmlFamilyDocumentCemtSubjectRef::MathMl(_) => &[
            "kind",
            "contentType",
            "schema",
            "category",
            "mediaProfile",
            "source",
            "resourceKind",
            "encodingReport",
            "parseFacts",
            "events",
            "lineEnding",
        ],
        XmlFamilyDocumentCemtSubjectRef::Xslt(_) => &[
            "kind",
            "contentType",
            "schema",
            "category",
            "version",
            "source",
            "resourceKind",
            "encodingReport",
            "parseFacts",
            "events",
            "lineEnding",
        ],
    }
}

fn xml_family_encoding_report_evaluator_field_names(
    document: XmlFamilyDocumentCemtSubjectRef<'_>,
) -> &'static [&'static str] {
    match document {
        XmlFamilyDocumentCemtSubjectRef::Html(_) => &[
            "mimeCharset",
            "metaCharset",
            "normalizedEncoding",
            "decoderStatus",
        ],
        XmlFamilyDocumentCemtSubjectRef::Css(_) => &[
            "mimeCharset",
            "stylesheetCharset",
            "bom",
            "normalizedEncoding",
            "decoderStatus",
        ],
        _ => &[
            "mimeCharset",
            "declarationEncoding",
            "normalizedEncoding",
            "decoderStatus",
        ],
    }
}

fn xml_family_fact_evaluator_field_names(
    document: XmlFamilyDocumentCemtSubjectRef<'_>,
) -> &'static [&'static str] {
    match document {
        XmlFamilyDocumentCemtSubjectRef::Xml(_) => &[
            "kind",
            "line",
            "column",
            "byteOffset",
            "byteLength",
            "message",
        ],
        _ => &["kind", "sourceRange", "message", "value"],
    }
}

fn xml_family_event_evaluator_field_names(
    document: XmlFamilyDocumentCemtSubjectRef<'_>,
) -> &'static [&'static str] {
    match document {
        XmlFamilyDocumentCemtSubjectRef::Html(_) => &[
            "index",
            "kind",
            "depth",
            "lexicalName",
            "localName",
            "namespace",
            "namespaceUri",
            "attributes",
            "value",
            "lexeme",
            "whitespaceOnly",
            "selfClosing",
            "voidElement",
            "recovered",
            "sourceRange",
            "sourceMap",
        ],
        XmlFamilyDocumentCemtSubjectRef::Css(_) => &[
            "index",
            "depth",
            "kind",
            "tokenKind",
            "value",
            "lexeme",
            "recovered",
            "sourceRange",
            "sourceMap",
        ],
        XmlFamilyDocumentCemtSubjectRef::Svg(_) | XmlFamilyDocumentCemtSubjectRef::MathMl(_) => &[
            "index",
            "kind",
            "depth",
            "qualifiedName",
            "localName",
            "prefix",
            "namespaceUri",
            "attributes",
            "value",
            "lexeme",
            "whitespaceOnly",
            "layoutSensitive",
            "structuralWhitespace",
            "lineBreakBefore",
            "markupTokens",
            "sourceRange",
            "sourceMap",
        ],
        _ => &[
            "index",
            "kind",
            "depth",
            "qualifiedName",
            "localName",
            "prefix",
            "namespaceUri",
            "attributes",
            "value",
            "lexeme",
            "whitespaceOnly",
            "sourceRange",
            "sourceMap",
        ],
    }
}

fn xml_family_document_evaluator_field<'a>(
    document: XmlFamilyDocumentCemtSubjectRef<'a>,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(match document {
            XmlFamilyDocumentCemtSubjectRef::Xml(_) => "xml-document",
            XmlFamilyDocumentCemtSubjectRef::Html(_) => "html-document",
            XmlFamilyDocumentCemtSubjectRef::Css(_) => "css-document",
            XmlFamilyDocumentCemtSubjectRef::Xhtml(_) => "xhtml-document",
            XmlFamilyDocumentCemtSubjectRef::Svg(_) => "svg-document",
            XmlFamilyDocumentCemtSubjectRef::MathMl(_) => "mathml-document",
            XmlFamilyDocumentCemtSubjectRef::Xslt(_) => "xslt-stylesheet",
        })),
        "contentType" => Some(CemtEvaluatorValueRef::String(
            xml_family_document_content_type(document),
        )),
        "schema" => Some(CemtEvaluatorValueRef::String(match document {
            XmlFamilyDocumentCemtSubjectRef::Xml(_) => XML_SCHEMA_URI,
            XmlFamilyDocumentCemtSubjectRef::Html(_) => HTML_SCHEMA_URI,
            XmlFamilyDocumentCemtSubjectRef::Css(_) => CSS_SCHEMA_URI,
            XmlFamilyDocumentCemtSubjectRef::Xhtml(_) => XHTML_SCHEMA_URI,
            XmlFamilyDocumentCemtSubjectRef::Svg(_) => SVG_SCHEMA_URI,
            XmlFamilyDocumentCemtSubjectRef::MathMl(_) => MATHML_SCHEMA_URI,
            XmlFamilyDocumentCemtSubjectRef::Xslt(_) => XSLT_SCHEMA_URI,
        })),
        "category" => match document {
            XmlFamilyDocumentCemtSubjectRef::Xml(_) => None,
            XmlFamilyDocumentCemtSubjectRef::Html(_) => {
                Some(CemtEvaluatorValueRef::String("html-document"))
            }
            XmlFamilyDocumentCemtSubjectRef::Css(_) => {
                Some(CemtEvaluatorValueRef::String("css-document"))
            }
            XmlFamilyDocumentCemtSubjectRef::Xhtml(_) => {
                Some(CemtEvaluatorValueRef::String("xhtml-document"))
            }
            XmlFamilyDocumentCemtSubjectRef::Svg(_) => {
                Some(CemtEvaluatorValueRef::String("svg-document"))
            }
            XmlFamilyDocumentCemtSubjectRef::MathMl(_) => {
                Some(CemtEvaluatorValueRef::String("mathml-document"))
            }
            XmlFamilyDocumentCemtSubjectRef::Xslt(_) => {
                Some(CemtEvaluatorValueRef::String("xslt-stylesheet"))
            }
        },
        "source" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::XmlFamilySource { document },
        )),
        "resourceKind" => xml_family_xml_document(document)
            .map(|xml| CemtEvaluatorValueRef::String(&xml.resource_kind)),
        "documentMode" => match document {
            XmlFamilyDocumentCemtSubjectRef::Html(document) => {
                Some(CemtEvaluatorValueRef::String(document.mode.as_str()))
            }
            _ => None,
        },
        "entryMode" => match document {
            XmlFamilyDocumentCemtSubjectRef::Css(document) => {
                Some(CemtEvaluatorValueRef::String(document.entry_mode.as_str()))
            }
            _ => None,
        },
        "mediaProfile" => match document {
            XmlFamilyDocumentCemtSubjectRef::MathMl(document) => Some(
                CemtEvaluatorValueRef::String(document.media_profile.as_str()),
            ),
            _ => None,
        },
        "version" => match document {
            XmlFamilyDocumentCemtSubjectRef::Xslt(document) => {
                Some(optional_string_evaluator_value(document.version.as_deref()))
            }
            _ => None,
        },
        "encodingReport" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::XmlFamilyEncodingReport { document },
        )),
        "parseFacts" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::XmlFamilyFacts { document },
        )),
        "events" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::XmlFamilyEvents { document },
        )),
        "lineEnding" => Some(optional_string_evaluator_value(xml_family_line_ending(
            document,
        ))),
        "recoveryCount" => match document {
            XmlFamilyDocumentCemtSubjectRef::Html(document) => {
                Some(usize_evaluator_value(document.recovery_count))
            }
            XmlFamilyDocumentCemtSubjectRef::Css(document) => {
                Some(usize_evaluator_value(document.recovery_count))
            }
            _ => None,
        },
        _ => None,
    }
}

fn xml_family_document_content_type<'a>(document: XmlFamilyDocumentCemtSubjectRef<'a>) -> &'a str {
    match document {
        XmlFamilyDocumentCemtSubjectRef::Xml(_) => XML_CONTENT_TYPE,
        XmlFamilyDocumentCemtSubjectRef::Html(document) => &document.source.media_type,
        XmlFamilyDocumentCemtSubjectRef::Css(document) => &document.source.media_type,
        XmlFamilyDocumentCemtSubjectRef::Xhtml(_) => XHTML_CONTENT_TYPE,
        XmlFamilyDocumentCemtSubjectRef::Svg(_) => SVG_CONTENT_TYPE,
        XmlFamilyDocumentCemtSubjectRef::MathMl(document) => {
            &document.xml_document.source.media_type
        }
        XmlFamilyDocumentCemtSubjectRef::Xslt(document) => &document.xml_document.source.media_type,
    }
}

fn xml_family_line_ending(document: XmlFamilyDocumentCemtSubjectRef<'_>) -> Option<&str> {
    match document {
        XmlFamilyDocumentCemtSubjectRef::Xml(document) => document.line_ending.as_deref(),
        XmlFamilyDocumentCemtSubjectRef::Html(document) => document.line_ending.as_deref(),
        XmlFamilyDocumentCemtSubjectRef::Css(document) => document.line_ending.as_deref(),
        XmlFamilyDocumentCemtSubjectRef::Xhtml(document) => document.line_ending.as_deref(),
        XmlFamilyDocumentCemtSubjectRef::Svg(document) => document.line_ending.as_deref(),
        XmlFamilyDocumentCemtSubjectRef::MathMl(document) => document.line_ending.as_deref(),
        XmlFamilyDocumentCemtSubjectRef::Xslt(document) => document.line_ending.as_deref(),
    }
}

fn xml_family_xml_document<'a>(
    document: XmlFamilyDocumentCemtSubjectRef<'a>,
) -> Option<&'a XmlDocumentAst> {
    match document {
        XmlFamilyDocumentCemtSubjectRef::Xml(document) => Some(document),
        XmlFamilyDocumentCemtSubjectRef::Xhtml(document) => Some(&document.xml_document),
        XmlFamilyDocumentCemtSubjectRef::Svg(document) => Some(&document.xml_document),
        XmlFamilyDocumentCemtSubjectRef::MathMl(document) => Some(&document.xml_document),
        XmlFamilyDocumentCemtSubjectRef::Xslt(document) => Some(&document.xml_document),
        XmlFamilyDocumentCemtSubjectRef::Html(_) | XmlFamilyDocumentCemtSubjectRef::Css(_) => None,
    }
}

fn xml_family_source_evaluator_field<'a>(
    document: XmlFamilyDocumentCemtSubjectRef<'a>,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match document {
        XmlFamilyDocumentCemtSubjectRef::Xml(document) => {
            xml_document_source_evaluator_field(&document.source, name)
        }
        XmlFamilyDocumentCemtSubjectRef::Html(document) => {
            html_document_source_evaluator_field(&document.source, name)
        }
        XmlFamilyDocumentCemtSubjectRef::Css(document) => {
            css_document_source_evaluator_field(&document.source, name)
        }
        XmlFamilyDocumentCemtSubjectRef::Xhtml(document) => {
            xhtml_document_source_evaluator_field(&document.source, name)
        }
        XmlFamilyDocumentCemtSubjectRef::Svg(document) => {
            svg_document_source_evaluator_field(&document.source, name)
        }
        XmlFamilyDocumentCemtSubjectRef::MathMl(document) => {
            xml_document_source_evaluator_field(&document.xml_document.source, name)
        }
        XmlFamilyDocumentCemtSubjectRef::Xslt(document) => {
            xml_document_source_evaluator_field(&document.xml_document.source, name)
        }
    }
}

macro_rules! xml_family_source_field_match {
    ($source:expr, $name:expr) => {{
        match $name {
            "uri" => Some(CemtEvaluatorValueRef::String(&$source.uri)),
            "contentType" => Some(CemtEvaluatorValueRef::String(&$source.content_type)),
            "mediaType" => Some(CemtEvaluatorValueRef::String(&$source.media_type)),
            "parameters" => Some(CemtEvaluatorValueRef::StringMap(&$source.parameters)),
            "byteLength" => Some(usize_evaluator_value($source.byte_length)),
            _ => None,
        }
    }};
}

fn xml_document_source_evaluator_field<'a>(
    source: &'a XmlDocumentSource,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    xml_family_source_field_match!(source, name)
}

fn html_document_source_evaluator_field<'a>(
    source: &'a HtmlDocumentSource,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    xml_family_source_field_match!(source, name)
}

fn css_document_source_evaluator_field<'a>(
    source: &'a CssDocumentSource,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    xml_family_source_field_match!(source, name)
}

fn xhtml_document_source_evaluator_field<'a>(
    source: &'a XhtmlDocumentSource,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    xml_family_source_field_match!(source, name)
}

fn svg_document_source_evaluator_field<'a>(
    source: &'a SvgDocumentSource,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    xml_family_source_field_match!(source, name)
}

fn xml_family_encoding_report_evaluator_field<'a>(
    document: XmlFamilyDocumentCemtSubjectRef<'a>,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match document {
        XmlFamilyDocumentCemtSubjectRef::Html(document) => {
            html_encoding_report_evaluator_field(&document.encoding_report, name)
        }
        XmlFamilyDocumentCemtSubjectRef::Css(document) => {
            css_encoding_report_evaluator_field(&document.encoding_report, name)
        }
        _ => xml_encoding_report_evaluator_field(
            &xml_family_xml_document(document)?.encoding_report,
            name,
        ),
    }
}

fn xml_encoding_report_evaluator_field<'a>(
    report: &'a XmlEncodingReportAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "mimeCharset" => Some(optional_string_evaluator_value(
            report.mime_charset.as_deref(),
        )),
        "declarationEncoding" => Some(optional_string_evaluator_value(
            report.declaration_encoding.as_deref(),
        )),
        "normalizedEncoding" => Some(CemtEvaluatorValueRef::String(&report.normalized_encoding)),
        "decoderStatus" => Some(CemtEvaluatorValueRef::String(&report.decoder_status)),
        _ => None,
    }
}

fn html_encoding_report_evaluator_field<'a>(
    report: &'a HtmlEncodingReportAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "mimeCharset" => Some(optional_string_evaluator_value(
            report.mime_charset.as_deref(),
        )),
        "metaCharset" => Some(optional_string_evaluator_value(
            report.meta_charset.as_deref(),
        )),
        "normalizedEncoding" => Some(CemtEvaluatorValueRef::String(&report.normalized_encoding)),
        "decoderStatus" => Some(CemtEvaluatorValueRef::String(&report.decoder_status)),
        _ => None,
    }
}

fn css_encoding_report_evaluator_field<'a>(
    report: &'a CssEncodingReportAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "mimeCharset" => Some(optional_string_evaluator_value(
            report.mime_charset.as_deref(),
        )),
        "stylesheetCharset" => Some(optional_string_evaluator_value(
            report.stylesheet_charset.as_deref(),
        )),
        "bom" => Some(optional_string_evaluator_value(report.bom.as_deref())),
        "normalizedEncoding" => Some(CemtEvaluatorValueRef::String(&report.normalized_encoding)),
        "decoderStatus" => Some(CemtEvaluatorValueRef::String(&report.decoder_status)),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
enum XmlFamilySemanticFactRef<'a> {
    Html(&'a HtmlFact),
    Css(&'a CssFact),
    Xhtml(&'a XhtmlFact),
    Svg(&'a SvgFact),
    MathMl(&'a MathMlFact),
    Xslt(&'a XsltFact),
}

fn xml_family_fact_count(document: XmlFamilyDocumentCemtSubjectRef<'_>) -> usize {
    match document {
        XmlFamilyDocumentCemtSubjectRef::Xml(document) => document.parse_facts.len(),
        XmlFamilyDocumentCemtSubjectRef::Html(document) => document.facts.len(),
        XmlFamilyDocumentCemtSubjectRef::Css(document) => document.facts.len(),
        XmlFamilyDocumentCemtSubjectRef::Xhtml(document) => document.facts.len(),
        XmlFamilyDocumentCemtSubjectRef::Svg(document) => document.facts.len(),
        XmlFamilyDocumentCemtSubjectRef::MathMl(document) => document.facts.len(),
        XmlFamilyDocumentCemtSubjectRef::Xslt(document) => document.facts.len(),
    }
}

fn xml_family_semantic_fact<'a>(
    document: XmlFamilyDocumentCemtSubjectRef<'a>,
    index: usize,
) -> Option<XmlFamilySemanticFactRef<'a>> {
    match document {
        XmlFamilyDocumentCemtSubjectRef::Xml(_) => None,
        XmlFamilyDocumentCemtSubjectRef::Html(document) => document
            .facts
            .get(index)
            .map(XmlFamilySemanticFactRef::Html),
        XmlFamilyDocumentCemtSubjectRef::Css(document) => {
            document.facts.get(index).map(XmlFamilySemanticFactRef::Css)
        }
        XmlFamilyDocumentCemtSubjectRef::Xhtml(document) => document
            .facts
            .get(index)
            .map(XmlFamilySemanticFactRef::Xhtml),
        XmlFamilyDocumentCemtSubjectRef::Svg(document) => {
            document.facts.get(index).map(XmlFamilySemanticFactRef::Svg)
        }
        XmlFamilyDocumentCemtSubjectRef::MathMl(document) => document
            .facts
            .get(index)
            .map(XmlFamilySemanticFactRef::MathMl),
        XmlFamilyDocumentCemtSubjectRef::Xslt(document) => document
            .facts
            .get(index)
            .map(XmlFamilySemanticFactRef::Xslt),
    }
}

fn xml_family_fact_evaluator_field<'a>(
    document: XmlFamilyDocumentCemtSubjectRef<'a>,
    index: usize,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    if let XmlFamilyDocumentCemtSubjectRef::Xml(document) = document {
        return xml_parse_fact_evaluator_field(document.parse_facts.get(index)?, name);
    }
    let fact = xml_family_semantic_fact(document, index)?;
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(match fact {
            XmlFamilySemanticFactRef::Html(fact) => fact.kind.as_str(),
            XmlFamilySemanticFactRef::Css(fact) => fact.kind.as_str(),
            XmlFamilySemanticFactRef::Xhtml(fact) => fact.kind.as_str(),
            XmlFamilySemanticFactRef::Svg(fact) => fact.kind.as_str(),
            XmlFamilySemanticFactRef::MathMl(fact) => fact.kind.as_str(),
            XmlFamilySemanticFactRef::Xslt(fact) => fact.kind.as_str(),
        })),
        "sourceRange" => Some(xml_family_optional_fact_source_range(fact)),
        "message" => Some(CemtEvaluatorValueRef::String(match fact {
            XmlFamilySemanticFactRef::Html(fact) => &fact.message,
            XmlFamilySemanticFactRef::Css(fact) => &fact.message,
            XmlFamilySemanticFactRef::Xhtml(fact) => &fact.message,
            XmlFamilySemanticFactRef::Svg(fact) => &fact.message,
            XmlFamilySemanticFactRef::MathMl(fact) => &fact.message,
            XmlFamilySemanticFactRef::Xslt(fact) => &fact.message,
        })),
        "value" => Some(optional_string_evaluator_value(match fact {
            XmlFamilySemanticFactRef::Html(fact) => fact.value.as_deref(),
            XmlFamilySemanticFactRef::Css(fact) => fact.value.as_deref(),
            XmlFamilySemanticFactRef::Xhtml(fact) => fact.value.as_deref(),
            XmlFamilySemanticFactRef::Svg(fact) => fact.value.as_deref(),
            XmlFamilySemanticFactRef::MathMl(fact) => fact.value.as_deref(),
            XmlFamilySemanticFactRef::Xslt(fact) => fact.value.as_deref(),
        })),
        _ => None,
    }
}

fn xml_parse_fact_evaluator_field<'a>(
    fact: &'a XmlParseFact,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(fact.kind.as_str())),
        "line" => Some(optional_u32_evaluator_value(fact.line)),
        "column" => Some(optional_u32_evaluator_value(fact.column)),
        "byteOffset" => Some(optional_u64_evaluator_value(fact.byte_offset)),
        "byteLength" => Some(optional_u64_evaluator_value(fact.byte_length)),
        "message" => Some(CemtEvaluatorValueRef::String(&fact.message)),
        _ => None,
    }
}

fn xml_family_optional_fact_source_range(
    fact: XmlFamilySemanticFactRef<'_>,
) -> CemtEvaluatorValueRef<'static> {
    let coordinates = match fact {
        XmlFamilySemanticFactRef::Html(fact) => fact.source_range.map(|range| {
            (
                range.start.byte_offset,
                range.byte_length,
                range.start.line,
                range.start.column,
            )
        }),
        XmlFamilySemanticFactRef::Css(fact) => fact.source_range.map(|range| {
            (
                range.start.byte_offset,
                range.byte_length,
                range.start.line,
                range.start.column,
            )
        }),
        XmlFamilySemanticFactRef::Xhtml(fact) => fact.source_range.map(|range| {
            (
                range.start.byte_offset,
                range.byte_length,
                range.start.line,
                range.start.column,
            )
        }),
        XmlFamilySemanticFactRef::Svg(fact) => fact.source_range.map(|range| {
            (
                range.start.byte_offset,
                range.byte_length,
                range.start.line,
                range.start.column,
            )
        }),
        XmlFamilySemanticFactRef::MathMl(fact) => fact.source_range.map(|range| {
            (
                range.start.byte_offset,
                range.byte_length,
                range.start.line,
                range.start.column,
            )
        }),
        XmlFamilySemanticFactRef::Xslt(fact) => fact.source_range.map(|range| {
            (
                range.start.byte_offset,
                range.byte_length,
                range.start.line,
                range.start.column,
            )
        }),
    };
    coordinates
        .map(|(byte_offset, byte_length, line, column)| {
            xml_family_source_range_evaluator_value(byte_offset, byte_length, line, column)
        })
        .unwrap_or(CemtEvaluatorValueRef::Null)
}

fn xml_family_event_count(document: XmlFamilyDocumentCemtSubjectRef<'_>) -> usize {
    match document {
        XmlFamilyDocumentCemtSubjectRef::Html(document) => document.events.len(),
        XmlFamilyDocumentCemtSubjectRef::Css(document) => document.events.len(),
        _ => xml_family_xml_document(document)
            .map(|document| document.events.len())
            .unwrap_or_default(),
    }
}

fn xml_family_event_evaluator_field<'a>(
    document: XmlFamilyDocumentCemtSubjectRef<'a>,
    index: usize,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match document {
        XmlFamilyDocumentCemtSubjectRef::Html(document) => {
            html_event_evaluator_field(document.events.get(index)?, name)
        }
        XmlFamilyDocumentCemtSubjectRef::Css(document) => {
            css_event_evaluator_field(document.events.get(index)?, name)
        }
        _ => xml_backed_event_evaluator_field(document, index, name),
    }
}

fn xml_backed_event_evaluator_field<'a>(
    document: XmlFamilyDocumentCemtSubjectRef<'a>,
    index: usize,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    let xml_document = xml_family_xml_document(document)?;
    let event = xml_document.events.get(index)?;
    let content_type = xml_family_event_content_type(document);
    let markup_package = match document {
        XmlFamilyDocumentCemtSubjectRef::Svg(_) => Some(XmlFamilyMarkupPackage::Svg),
        XmlFamilyDocumentCemtSubjectRef::MathMl(_) => Some(XmlFamilyMarkupPackage::MathMl),
        XmlFamilyDocumentCemtSubjectRef::Xslt(_) => Some(XmlFamilyMarkupPackage::Xslt),
        _ => None,
    };
    match name {
        "index" => Some(usize_evaluator_value(event.index)),
        "kind" => Some(CemtEvaluatorValueRef::String(event.kind.as_str())),
        "depth" => Some(usize_evaluator_value(event.depth)),
        "qualifiedName" => Some(optional_string_evaluator_value(
            event.qualified_name.as_deref(),
        )),
        "localName" => Some(optional_string_evaluator_value(event.local_name.as_deref())),
        "prefix" => Some(optional_string_evaluator_value(event.prefix.as_deref())),
        "namespaceUri" => Some(optional_string_evaluator_value(
            event.namespace_uri.as_deref(),
        )),
        "attributes" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::XmlAttributes {
                attributes: &event.attributes,
            },
        )),
        "value" => Some(optional_string_evaluator_value(event.value.as_deref())),
        "lexeme" => Some(CemtEvaluatorValueRef::String(&event.lexeme)),
        "whitespaceOnly" => Some(CemtEvaluatorValueRef::Boolean(event.whitespace_only)),
        "layoutSensitive" => markup_package.map(|package| {
            CemtEvaluatorValueRef::Boolean(
                xml_family_event_layout_at(&xml_document.events, index, package).0,
            )
        }),
        "structuralWhitespace" => markup_package.map(|package| {
            CemtEvaluatorValueRef::Boolean(
                xml_family_event_layout_at(&xml_document.events, index, package).1,
            )
        }),
        "lineBreakBefore" => markup_package.map(|package| {
            CemtEvaluatorValueRef::Boolean(
                xml_family_event_layout_at(&xml_document.events, index, package).2,
            )
        }),
        "markupTokens" => markup_package.map(|package| {
            CemtEvaluatorValueRef::Sequence(CemtEvaluatorSequenceRef::XmlFamilyMarkupTokens {
                event,
                content_type,
                package,
            })
        }),
        "sourceRange" => Some(xml_family_source_range_evaluator_value(
            event.source_range.start.byte_offset,
            event.source_range.byte_length,
            event.source_range.start.line,
            event.source_range.start.column,
        )),
        "sourceMap" => Some(CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(
            xml_family_source_map(event.source_range, content_type),
        ))),
        _ => None,
    }
}

fn xml_family_event_content_type<'a>(document: XmlFamilyDocumentCemtSubjectRef<'a>) -> &'a str {
    match document {
        XmlFamilyDocumentCemtSubjectRef::Xml(_) => XML_CONTENT_TYPE,
        XmlFamilyDocumentCemtSubjectRef::Xhtml(_) => XHTML_CONTENT_TYPE,
        XmlFamilyDocumentCemtSubjectRef::Svg(_) => SVG_CONTENT_TYPE,
        XmlFamilyDocumentCemtSubjectRef::MathMl(document) => {
            &document.xml_document.source.media_type
        }
        XmlFamilyDocumentCemtSubjectRef::Xslt(document) => &document.xml_document.source.media_type,
        XmlFamilyDocumentCemtSubjectRef::Html(_) => HTML_CONTENT_TYPE,
        XmlFamilyDocumentCemtSubjectRef::Css(_) => CSS_CONTENT_TYPE,
    }
}

fn html_event_evaluator_field<'a>(
    event: &'a HtmlEventAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "index" => Some(usize_evaluator_value(event.index)),
        "kind" => Some(CemtEvaluatorValueRef::String(event.kind.as_str())),
        "depth" => Some(usize_evaluator_value(event.depth)),
        "lexicalName" => Some(optional_string_evaluator_value(
            event.lexical_name.as_deref(),
        )),
        "localName" => Some(optional_string_evaluator_value(event.local_name.as_deref())),
        "namespace" => Some(CemtEvaluatorValueRef::String(event.namespace.as_str())),
        "namespaceUri" => Some(CemtEvaluatorValueRef::String(&event.namespace_uri)),
        "attributes" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::HtmlAttributes {
                attributes: &event.attributes,
            },
        )),
        "value" => Some(optional_string_evaluator_value(event.value.as_deref())),
        "lexeme" => Some(CemtEvaluatorValueRef::String(&event.lexeme)),
        "whitespaceOnly" => Some(CemtEvaluatorValueRef::Boolean(event.whitespace_only)),
        "selfClosing" => Some(CemtEvaluatorValueRef::Boolean(event.self_closing)),
        "voidElement" => Some(CemtEvaluatorValueRef::Boolean(event.void_element)),
        "recovered" => Some(CemtEvaluatorValueRef::Boolean(event.recovered)),
        "sourceRange" => Some(xml_family_source_range_evaluator_value(
            event.source_range.start.byte_offset,
            event.source_range.byte_length,
            event.source_range.start.line,
            event.source_range.start.column,
        )),
        "sourceMap" => Some(CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(
            xml_family_source_map_from_coordinates(
                event.source_range.start.byte_offset,
                event.source_range.byte_length,
                HTML_CONTENT_TYPE,
            ),
        ))),
        _ => None,
    }
}

fn css_event_evaluator_field<'a>(
    event: &'a CssEventAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "index" => Some(usize_evaluator_value(event.index)),
        "depth" => Some(usize_evaluator_value(event.depth)),
        "kind" => Some(CemtEvaluatorValueRef::String(&event.kind)),
        "tokenKind" => Some(CemtEvaluatorValueRef::String(&event.token_kind)),
        "value" => Some(optional_string_evaluator_value(event.value.as_deref())),
        "lexeme" => Some(CemtEvaluatorValueRef::String(&event.lexeme)),
        "recovered" => Some(CemtEvaluatorValueRef::Boolean(event.recovered)),
        "sourceRange" => Some(xml_family_source_range_evaluator_value(
            event.source_range.start.byte_offset,
            event.source_range.byte_length,
            event.source_range.start.line,
            event.source_range.start.column,
        )),
        "sourceMap" => Some(CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(
            xml_family_source_map_from_coordinates(
                event.source_range.start.byte_offset,
                event.source_range.byte_length,
                CSS_CONTENT_TYPE,
            ),
        ))),
        _ => None,
    }
}

fn xml_attribute_evaluator_field<'a>(
    attribute: &'a XmlAttributeAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "qualifiedName" => Some(CemtEvaluatorValueRef::String(&attribute.qualified_name)),
        "localName" => Some(CemtEvaluatorValueRef::String(&attribute.local_name)),
        "prefix" => Some(optional_string_evaluator_value(attribute.prefix.as_deref())),
        "namespaceUri" => Some(optional_string_evaluator_value(
            attribute.namespace_uri.as_deref(),
        )),
        "value" => Some(CemtEvaluatorValueRef::String(&attribute.value)),
        _ => None,
    }
}

fn html_attribute_evaluator_field<'a>(
    attribute: &'a HtmlAttributeAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "lexicalName" => Some(CemtEvaluatorValueRef::String(&attribute.lexical_name)),
        "localName" => Some(CemtEvaluatorValueRef::String(&attribute.local_name)),
        "value" => Some(optional_string_evaluator_value(attribute.value.as_deref())),
        "lexeme" => Some(CemtEvaluatorValueRef::String(&attribute.lexeme)),
        "duplicate" => Some(CemtEvaluatorValueRef::Boolean(attribute.duplicate)),
        "sourceRange" => Some(xml_family_source_range_evaluator_value(
            attribute.source_range.start.byte_offset,
            attribute.source_range.byte_length,
            attribute.source_range.start.line,
            attribute.source_range.start.column,
        )),
        "sourceMap" => Some(CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(
            xml_family_source_map_from_coordinates(
                attribute.source_range.start.byte_offset,
                attribute.source_range.byte_length,
                HTML_CONTENT_TYPE,
            ),
        ))),
        _ => None,
    }
}

fn xml_family_markup_token_evaluator_field<'a>(
    token: &XmlMarkupTokenAst,
    content_type: &str,
    _package: XmlFamilyMarkupPackage,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(token.kind.as_str())),
        "text" => Some(CemtEvaluatorValueRef::OwnedString(Arc::from(
            token.text.as_str(),
        ))),
        "role" => Some(CemtEvaluatorValueRef::String(xml_family_markup_token_role(
            token.kind,
        ))),
        "sourceRange" => Some(xml_family_source_range_evaluator_value(
            token.source_range.start.byte_offset,
            token.source_range.byte_length,
            token.source_range.start.line,
            token.source_range.start.column,
        )),
        "sourceMap" => Some(CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(
            xml_family_source_map(token.source_range, content_type),
        ))),
        _ => None,
    }
}

fn xml_family_markup_token_role(kind: XmlMarkupTokenKind) -> &'static str {
    match kind {
        XmlMarkupTokenKind::Delimiter | XmlMarkupTokenKind::Equals => "syntax.punctuation",
        XmlMarkupTokenKind::ElementName => "syntax.name",
        XmlMarkupTokenKind::AttributeName => "syntax.attribute",
        XmlMarkupTokenKind::AttributeValue => "syntax.string",
        XmlMarkupTokenKind::Whitespace | XmlMarkupTokenKind::Raw => "syntax.raw",
    }
}

fn xml_family_source_range_evaluator_value(
    byte_offset: u64,
    byte_length: u64,
    line: u32,
    column: u32,
) -> CemtEvaluatorValueRef<'static> {
    CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::XmlFamilySourceRange {
        byte_offset,
        byte_length,
        line,
        column,
    })
}

fn xml_family_source_range_evaluator_field(
    byte_offset: u64,
    byte_length: u64,
    line: u32,
    column: u32,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'static>> {
    match name {
        "byteOffset" => Some(u64_evaluator_value(byte_offset)),
        "byteLength" => Some(u64_evaluator_value(byte_length)),
        "line" => Some(u64_evaluator_value(u64::from(line))),
        "column" => Some(u64_evaluator_value(u64::from(column))),
        _ => None,
    }
}

fn xml_family_source_map(range: XmlSourceRange, content_type: &str) -> SourceMapStack {
    xml_family_source_map_from_coordinates(range.start.byte_offset, range.byte_length, content_type)
}

fn xml_family_source_map_from_coordinates(
    byte_offset: u64,
    byte_length: u64,
    content_type: &str,
) -> SourceMapStack {
    SourceMapStack {
        frames: vec![SourceMapFrame {
            source_id: SourceId(1),
            span: FrameSpan::Single(ByteRange::new(
                byte_offset,
                u32::try_from(byte_length).unwrap_or(u32::MAX),
            )),
            transform: TransformKind::ContentTypeTransform {
                content_type: content_type.to_owned(),
            },
        }],
    }
}

fn xml_family_event_layout_at(
    events: &[XmlEventAst],
    requested: usize,
    package: XmlFamilyMarkupPackage,
) -> (bool, bool, bool) {
    let mut sensitive_scopes = vec![None; events.len()];
    let mut stack = Vec::<(usize, bool)>::new();
    let mut ranges = Vec::<(usize, usize, usize)>::new();
    let mut next_scope = 0usize;

    for (index, event) in events.iter().enumerate() {
        match event.kind {
            XmlEventKind::StartElement => {
                let inherited = stack.last().is_some_and(|(_, sensitive)| *sensitive);
                stack.push((
                    index,
                    inherited || xml_family_element_requires_lexical_layout(event, package),
                ));
            }
            XmlEventKind::EmptyElement => {
                if stack.last().is_some_and(|(_, sensitive)| *sensitive)
                    || xml_family_element_requires_lexical_layout(event, package)
                {
                    ranges.push((index, index, next_scope));
                    next_scope = next_scope.saturating_add(1);
                }
            }
            XmlEventKind::EndElement => {
                if let Some((start, true)) = stack.pop() {
                    ranges.push((start, index, next_scope));
                    next_scope = next_scope.saturating_add(1);
                }
            }
            XmlEventKind::Text => {
                if !event.whitespace_only {
                    if let Some((_, sensitive)) = stack.last_mut() {
                        *sensitive = true;
                    } else {
                        ranges.push((index, index, next_scope));
                        next_scope = next_scope.saturating_add(1);
                    }
                }
            }
            XmlEventKind::Cdata | XmlEventKind::EntityReference => {
                if let Some((_, sensitive)) = stack.last_mut() {
                    *sensitive = true;
                } else {
                    ranges.push((index, index, next_scope));
                    next_scope = next_scope.saturating_add(1);
                }
            }
            XmlEventKind::Declaration
            | XmlEventKind::Comment
            | XmlEventKind::ProcessingInstruction
            | XmlEventKind::Doctype => {}
        }
    }

    for (start, end, scope) in ranges {
        for event_scope in &mut sensitive_scopes[start..=end] {
            *event_scope = Some(scope);
        }
    }

    let mut previous_scope = None;
    let mut has_previous = false;
    for (index, event) in events.iter().enumerate() {
        let scope = sensitive_scopes[index];
        let structural_whitespace =
            matches!(event.kind, XmlEventKind::Text) && event.whitespace_only && scope.is_none();
        let line_break_before =
            !structural_whitespace && has_previous && !(scope.is_some() && scope == previous_scope);
        if index == requested {
            return (scope.is_some(), structural_whitespace, line_break_before);
        }
        if !structural_whitespace {
            has_previous = true;
            previous_scope = scope;
        }
    }
    (false, false, false)
}

fn xml_family_element_requires_lexical_layout(
    event: &XmlEventAst,
    package: XmlFamilyMarkupPackage,
) -> bool {
    let local_name = event.local_name.as_deref().unwrap_or_default();
    let name_requires_layout = match package {
        XmlFamilyMarkupPackage::Svg => matches!(
            local_name,
            "text" | "tspan" | "textPath" | "title" | "desc" | "style" | "script" | "foreignObject"
        ),
        XmlFamilyMarkupPackage::MathMl => matches!(
            local_name,
            "mi" | "mn" | "mo" | "mtext" | "ms" | "annotation" | "annotation-xml"
        ),
        XmlFamilyMarkupPackage::Xslt => {
            local_name == "text" && event.namespace_uri.as_deref() == Some(XSLT_NAMESPACE_URI)
        }
    };
    let expected_namespace = match package {
        XmlFamilyMarkupPackage::Svg => SVG_NAMESPACE_URI,
        XmlFamilyMarkupPackage::MathMl => MATHML_NAMESPACE_URI,
        XmlFamilyMarkupPackage::Xslt => XSLT_NAMESPACE_URI,
    };
    name_requires_layout
        || event.namespace_uri.as_deref() != Some(expected_namespace)
        || event.attributes.iter().any(|attribute| {
            attribute.qualified_name == "xml:space" && attribute.value == "preserve"
        })
}

fn markdown_document_evaluator_field<'a>(
    document: &'a MarkdownDocumentAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String("markdown-document")),
        "contentType" => Some(CemtEvaluatorValueRef::String(MARKDOWN_CONTENT_TYPE)),
        "schema" => Some(CemtEvaluatorValueRef::String(MARKDOWN_SCHEMA_URI)),
        "source" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::MarkdownSource {
                source: &document.source,
            },
        )),
        "encodingReport" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::MarkdownEncodingReport {
                report: &document.encoding_report,
            },
        )),
        "encodingFacts" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::MarkdownEncodingFacts {
                facts: &document.encoding_facts,
            },
        )),
        "variant" => Some(CemtEvaluatorValueRef::String(&document.variant)),
        "variantFacts" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::MarkdownVariantFacts {
                facts: &document.variant_facts,
            },
        )),
        "parseFacts" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::MarkdownParseFacts {
                facts: &document.parse_facts,
            },
        )),
        "events" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::MarkdownEvents {
                events: &document.events,
            },
        )),
        "lineEnding" => document
            .line_ending
            .as_deref()
            .map(CemtEvaluatorValueRef::String),
        _ => None,
    }
}

fn markdown_source_evaluator_field<'a>(
    source: &'a MarkdownDocumentSource,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "uri" => Some(CemtEvaluatorValueRef::String(&source.uri)),
        "contentType" => Some(CemtEvaluatorValueRef::String(&source.content_type)),
        "mediaType" => Some(CemtEvaluatorValueRef::String(&source.media_type)),
        "parameters" => Some(CemtEvaluatorValueRef::StringMap(&source.parameters)),
        "byteLength" => Some(usize_evaluator_value(source.byte_length)),
        _ => None,
    }
}

fn markdown_encoding_report_evaluator_field<'a>(
    report: &'a MarkdownEncodingReportAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "declaredCharset" => report
            .declared_charset
            .as_deref()
            .map(CemtEvaluatorValueRef::String),
        "normalizedCharset" => Some(CemtEvaluatorValueRef::String(&report.normalized_charset)),
        "decoderStatus" => Some(CemtEvaluatorValueRef::String(&report.decoder_status)),
        "invalidByteOffset" => report.invalid_byte_offset.map(u64_evaluator_value),
        _ => None,
    }
}

fn markdown_encoding_fact_evaluator_field<'a>(
    fact: &'a MarkdownEncodingFact,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(fact.kind.as_str())),
        "diagnosticCode" => Some(optional_string_evaluator_value(
            fact.diagnostic_code.as_deref(),
        )),
        "diagnosticSeverity" => Some(optional_string_evaluator_value(
            fact.diagnostic_severity.as_deref(),
        )),
        "recoverable" => Some(CemtEvaluatorValueRef::Boolean(fact.recoverable)),
        "fatal" => Some(CemtEvaluatorValueRef::Boolean(fact.fatal)),
        "parameter" => Some(optional_string_evaluator_value(fact.parameter.as_deref())),
        "actual" => Some(optional_string_evaluator_value(fact.actual.as_deref())),
        "expected" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::Strings {
                values: &fact.expected,
            },
        )),
        "message" => Some(CemtEvaluatorValueRef::String(&fact.message)),
        "sourceRange" => Some(markdown_optional_source_range_evaluator_value(
            fact.source_range,
        )),
        "sourceMap" => Some(markdown_optional_source_map_evaluator_value(
            fact.source_range,
        )),
        _ => None,
    }
}

fn markdown_variant_fact_evaluator_field<'a>(
    fact: &'a MarkdownVariantFact,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(fact.kind.as_str())),
        "variant" => Some(optional_string_evaluator_value(fact.variant.as_deref())),
        "diagnosticCode" => Some(optional_string_evaluator_value(
            fact.diagnostic_code.as_deref(),
        )),
        "diagnosticSeverity" => Some(optional_string_evaluator_value(
            fact.diagnostic_severity.as_deref(),
        )),
        "recoverable" => Some(CemtEvaluatorValueRef::Boolean(fact.recoverable)),
        "fatal" => Some(CemtEvaluatorValueRef::Boolean(fact.fatal)),
        "message" => Some(CemtEvaluatorValueRef::String(&fact.message)),
        _ => None,
    }
}

fn markdown_parse_fact_evaluator_field<'a>(
    fact: &'a MarkdownParseFact,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(fact.kind.as_str())),
        "diagnosticCode" => Some(optional_string_evaluator_value(
            fact.diagnostic_code.as_deref(),
        )),
        "diagnosticSeverity" => Some(optional_string_evaluator_value(
            fact.diagnostic_severity.as_deref(),
        )),
        "recoverable" => Some(CemtEvaluatorValueRef::Boolean(fact.recoverable)),
        "fatal" => Some(CemtEvaluatorValueRef::Boolean(fact.fatal)),
        "eventIndex" => Some(optional_usize_evaluator_value(fact.event_index)),
        "eventKind" => Some(optional_string_evaluator_value(fact.event_kind.as_deref())),
        "raw" => Some(optional_string_evaluator_value(fact.raw.as_deref())),
        "message" => Some(CemtEvaluatorValueRef::String(&fact.message)),
        "sourceRange" => Some(markdown_optional_source_range_evaluator_value(
            fact.source_range,
        )),
        "sourceMap" => Some(markdown_optional_source_map_evaluator_value(
            fact.source_range,
        )),
        _ => None,
    }
}

fn markdown_event_evaluator_field<'a>(
    event: &'a MarkdownEventAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "index" => Some(usize_evaluator_value(event.index)),
        "kind" => Some(CemtEvaluatorValueRef::String(&event.kind)),
        "tag" => Some(optional_string_evaluator_value(event.tag.as_deref())),
        "text" => Some(optional_string_evaluator_value(event.text.as_deref())),
        "destination" => Some(optional_string_evaluator_value(
            event.destination.as_deref(),
        )),
        "title" => Some(optional_string_evaluator_value(event.title.as_deref())),
        "info" => Some(optional_string_evaluator_value(event.info.as_deref())),
        "level" => Some(optional_u32_evaluator_value(event.level)),
        "checked" => Some(optional_bool_evaluator_value(event.checked)),
        "orderedStart" => Some(optional_u64_evaluator_value(event.ordered_start)),
        "byteOffset" => Some(u64_evaluator_value(event.source_range.start.byte_offset)),
        "byteLength" => Some(u64_evaluator_value(event.source_range.byte_length)),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::MarkdownSourceRange {
                range: event.source_range,
            },
        )),
        "sourceMap" => Some(CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(
            event.source_range.source_map(),
        ))),
        _ => None,
    }
}

fn markdown_optional_source_range_evaluator_value(
    range: Option<MarkdownSourceRange>,
) -> CemtEvaluatorValueRef<'static> {
    match range {
        Some(range) => {
            CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::MarkdownSourceRange { range })
        }
        None => CemtEvaluatorValueRef::Null,
    }
}

fn markdown_optional_source_map_evaluator_value(
    range: Option<MarkdownSourceRange>,
) -> CemtEvaluatorValueRef<'static> {
    match range {
        Some(range) => CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(range.source_map())),
        None => CemtEvaluatorValueRef::Null,
    }
}

fn markdown_source_range_evaluator_field(
    range: MarkdownSourceRange,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'static>> {
    match name {
        "byteOffset" => Some(u64_evaluator_value(range.start.byte_offset)),
        "byteLength" => Some(u64_evaluator_value(range.byte_length)),
        "line" => Some(u64_evaluator_value(u64::from(range.start.line))),
        "column" => Some(u64_evaluator_value(u64::from(range.start.column))),
        _ => None,
    }
}

fn yaml_document_evaluator_field<'a>(
    document: &'a YamlDocumentAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String("yaml-stream")),
        "contentType" => Some(CemtEvaluatorValueRef::String(YAML_CONTENT_TYPE)),
        "schema" => Some(CemtEvaluatorValueRef::String(YAML_SCHEMA_URI)),
        "source" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::YamlSource {
                source: &document.source,
            },
        )),
        "encoding" => Some(CemtEvaluatorValueRef::String(&document.encoding)),
        "encodingReport" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::YamlEncodingReport {
                report: &document.encoding_report,
            },
        )),
        "parseFacts" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::YamlParseFacts {
                facts: &document.parse_facts,
            },
        )),
        "directives" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::YamlDirectives {
                directives: &document.directives,
            },
        )),
        "comments" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::YamlComments {
                comments: &document.comments,
            },
        )),
        "documents" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::YamlDocuments {
                documents: &document.documents,
            },
        )),
        "lineEnding" => document
            .line_ending
            .as_deref()
            .map(CemtEvaluatorValueRef::String),
        _ => None,
    }
}

fn yaml_source_evaluator_field<'a>(
    source: &'a YamlDocumentSource,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "uri" => Some(CemtEvaluatorValueRef::String(&source.uri)),
        "contentType" => Some(CemtEvaluatorValueRef::String(&source.content_type)),
        "mediaType" => Some(CemtEvaluatorValueRef::String(&source.media_type)),
        "parameters" => Some(CemtEvaluatorValueRef::StringMap(&source.parameters)),
        "byteLength" => Some(usize_evaluator_value(source.byte_length)),
        _ => None,
    }
}

fn yaml_encoding_report_evaluator_field<'a>(
    report: &'a YamlEncodingReportAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "declaredCharset" => report
            .declared_charset
            .as_deref()
            .map(CemtEvaluatorValueRef::String),
        "normalizedCharset" => Some(CemtEvaluatorValueRef::String(&report.normalized_charset)),
        "decoderStatus" => Some(CemtEvaluatorValueRef::String(&report.decoder_status)),
        "invalidByteOffset" => report.invalid_byte_offset.map(u64_evaluator_value),
        _ => None,
    }
}

fn yaml_parse_fact_evaluator_field<'a>(
    fact: &'a YamlDocumentParseFact,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(fact.kind.as_str())),
        "contract" => Some(optional_string_evaluator_value(fact.contract.as_deref())),
        "behavior" => Some(optional_string_evaluator_value(fact.behavior.as_deref())),
        "diagnosticCode" => Some(optional_string_evaluator_value(
            fact.diagnostic_code.as_deref(),
        )),
        "diagnosticSeverity" => Some(optional_string_evaluator_value(
            fact.diagnostic_severity.as_deref(),
        )),
        "recoverable" => Some(CemtEvaluatorValueRef::Boolean(fact.recoverable)),
        "fatal" => Some(CemtEvaluatorValueRef::Boolean(fact.fatal)),
        "parameter" => Some(optional_string_evaluator_value(fact.parameter.as_deref())),
        "actual" => Some(optional_string_evaluator_value(fact.actual.as_deref())),
        "expected" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::Strings {
                values: &fact.expected,
            },
        )),
        "line" => Some(optional_u32_evaluator_value(fact.line)),
        "column" => Some(optional_u32_evaluator_value(fact.column)),
        "byteOffset" => Some(optional_u64_evaluator_value(fact.byte_offset)),
        "byteLength" => Some(optional_u64_evaluator_value(fact.byte_length)),
        "message" => Some(CemtEvaluatorValueRef::String(&fact.message)),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::YamlParseFactSourceRange { fact },
        )),
        _ => None,
    }
}

fn yaml_parse_fact_source_range_evaluator_field(
    fact: &YamlDocumentParseFact,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'static>> {
    match name {
        "byteOffset" => Some(optional_u64_evaluator_value(fact.byte_offset)),
        "byteLength" => Some(optional_u64_evaluator_value(fact.byte_length)),
        "line" => Some(optional_u32_evaluator_value(fact.line)),
        "column" => Some(optional_u32_evaluator_value(fact.column)),
        _ => None,
    }
}

fn yaml_directive_evaluator_field<'a>(
    directive: &'a YamlDirectiveAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "index" => Some(usize_evaluator_value(directive.index)),
        "name" => Some(CemtEvaluatorValueRef::String(&directive.name)),
        "value" => Some(optional_string_evaluator_value(directive.value.as_deref())),
        "byteOffset" => Some(u64_evaluator_value(directive.range.start.byte_offset)),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::YamlSourceRange {
                range: directive.range,
            },
        )),
        "sourceMap" => Some(CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(
            directive.range.source_map(),
        ))),
        _ => None,
    }
}

fn yaml_comment_evaluator_field<'a>(
    comment: &'a YamlCommentAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "index" => Some(usize_evaluator_value(comment.index)),
        "kind" => Some(CemtEvaluatorValueRef::String("comment")),
        "value" => Some(CemtEvaluatorValueRef::String(&comment.value)),
        "text" => Some(CemtEvaluatorValueRef::String(&comment.text)),
        "indent" => Some(CemtEvaluatorValueRef::String(&comment.indent)),
        "placement" => Some(CemtEvaluatorValueRef::String(match comment.placement {
            YamlCommentPlacement::Line => "line",
            YamlCommentPlacement::Inline => "inline",
        })),
        "byteOffset" => Some(u64_evaluator_value(comment.range.start.byte_offset)),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::YamlSourceRange {
                range: comment.range,
            },
        )),
        "sourceMap" => Some(CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(
            comment.range.source_map(),
        ))),
        _ => None,
    }
}

fn yaml_stream_document_evaluator_field<'a>(
    document: &'a YamlStreamDocumentAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "index" => Some(usize_evaluator_value(document.index)),
        "byteOffset" => Some(u64_evaluator_value(document.range.start.byte_offset)),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::YamlSourceRange {
                range: document.range,
            },
        )),
        "sourceMap" => Some(CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(
            document.range.source_map(),
        ))),
        "root" => Some(match document.root.as_ref() {
            Some(node) => CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::YamlNode { node }),
            None => CemtEvaluatorValueRef::Null,
        }),
        _ => None,
    }
}

fn yaml_node_evaluator_field<'a>(
    node: &'a YamlNodeAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(match node.kind {
            YamlNodeKind::Mapping => "mapping",
            YamlNodeKind::Sequence => "sequence",
            YamlNodeKind::Scalar => "scalar",
            YamlNodeKind::Alias => "alias",
        })),
        "tag" => Some(optional_string_evaluator_value(node.tag.as_deref())),
        "anchor" => Some(optional_string_evaluator_value(node.anchor.as_deref())),
        "anchorId" => Some(optional_usize_evaluator_value(node.anchor_id)),
        "alias" => Some(optional_string_evaluator_value(node.alias.as_deref())),
        "value" => Some(optional_string_evaluator_value(node.value.as_deref())),
        "style" => Some(optional_string_evaluator_value(node.style.as_deref())),
        "implicitKind" => Some(optional_string_evaluator_value(
            node.implicit_kind.as_deref(),
        )),
        "byteOffset" => Some(u64_evaluator_value(node.range.start.byte_offset)),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::YamlSourceRange { range: node.range },
        )),
        "sourceMap" => Some(CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(
            node.range.source_map(),
        ))),
        "sequence" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::YamlNodes {
                nodes: &node.sequence,
            },
        )),
        "mapping" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::YamlPairs {
                pairs: &node.mapping,
            },
        )),
        _ => None,
    }
}

fn yaml_pair_evaluator_field<'a>(
    pair: &'a YamlPairAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "index" => Some(usize_evaluator_value(pair.index)),
        "key" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::YamlNode { node: &pair.key },
        )),
        "value" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::YamlNode { node: &pair.value },
        )),
        _ => None,
    }
}

fn yaml_source_range_evaluator_field(
    range: YamlSourceRange,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'static>> {
    let value = match name {
        "byteOffset" => range.start.byte_offset,
        "byteLength" => range.byte_length,
        "line" => u64::from(range.start.line),
        "column" => u64::from(range.start.column),
        _ => return None,
    };
    Some(u64_evaluator_value(value))
}

fn generic_data_yaml_document_evaluator_field<'a>(
    document: &'a GenericDataDocumentAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String("yaml-stream")),
        "contentType" => Some(CemtEvaluatorValueRef::String(YAML_CONTENT_TYPE)),
        "schema" => Some(CemtEvaluatorValueRef::String(YAML_SCHEMA_URI)),
        "source" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::GenericDataYamlSource { document },
        )),
        "encoding" => Some(CemtEvaluatorValueRef::String("utf-8")),
        "encodingReport" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::GenericDataYamlEncodingReport,
        )),
        "parseFacts" | "directives" | "comments" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::Empty,
        )),
        "documents" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::GenericDataYamlDocuments {
                documents: &document.documents,
            },
        )),
        "lineEnding" => document
            .line_ending
            .as_deref()
            .map(CemtEvaluatorValueRef::String),
        _ => None,
    }
}

fn generic_data_yaml_source_evaluator_field<'a>(
    document: &'a GenericDataDocumentAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    let source = &document.source;
    match name {
        "uri" => Some(CemtEvaluatorValueRef::String(&source.uri)),
        "contentType" => Some(CemtEvaluatorValueRef::String(&source.content_type)),
        "mediaType" => Some(CemtEvaluatorValueRef::String(&source.media_type)),
        "parameters" => Some(CemtEvaluatorValueRef::StringMap(&source.parameters)),
        "byteLength" => Some(usize_evaluator_value(source.byte_length)),
        _ => None,
    }
}

fn generic_data_yaml_encoding_report_evaluator_field(
    name: &str,
) -> Option<CemtEvaluatorValueRef<'static>> {
    match name {
        "normalizedCharset" => Some(CemtEvaluatorValueRef::String("utf-8")),
        "decoderStatus" => Some(CemtEvaluatorValueRef::String("decoded")),
        _ => None,
    }
}

fn generic_data_yaml_stream_document_evaluator_field<'a>(
    document: &'a GenericDataStreamDocumentAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "index" => Some(usize_evaluator_value(document.index)),
        "byteOffset" => Some(u64_evaluator_value(document.source_range.byte_offset)),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::GenericDataSourceRange {
                source_range: &document.source_range,
            },
        )),
        "sourceMap" => Some(generic_data_source_map_evaluator_value(
            &document.source_range,
        )),
        "root" => Some(match document.root.as_ref() {
            Some(value) => {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::GenericDataYamlNode { value })
            }
            None => CemtEvaluatorValueRef::Null,
        }),
        _ => None,
    }
}

fn generic_data_yaml_node_evaluator_field<'a>(
    value: &'a GenericDataValueAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    let source_range = value.source_range();
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(match value {
            GenericDataValueAst::Mapping { .. } => "mapping",
            GenericDataValueAst::Sequence { .. } => "sequence",
            GenericDataValueAst::String { .. }
            | GenericDataValueAst::Number { .. }
            | GenericDataValueAst::Boolean { .. }
            | GenericDataValueAst::Null { .. } => "scalar",
            GenericDataValueAst::Alias { .. } => "alias",
        })),
        "tag" | "anchor" | "anchorId" => Some(CemtEvaluatorValueRef::Null),
        "alias" => Some(match value {
            GenericDataValueAst::Alias { alias, .. } => {
                optional_string_evaluator_value(alias.as_deref())
            }
            _ => CemtEvaluatorValueRef::Null,
        }),
        "value" => Some(match value {
            GenericDataValueAst::String { value, .. } => CemtEvaluatorValueRef::String(value),
            GenericDataValueAst::Number { lexeme, .. } => CemtEvaluatorValueRef::String(lexeme),
            GenericDataValueAst::Boolean { value: true, .. } => {
                CemtEvaluatorValueRef::String("true")
            }
            GenericDataValueAst::Boolean { value: false, .. } => {
                CemtEvaluatorValueRef::String("false")
            }
            GenericDataValueAst::Null { .. } => CemtEvaluatorValueRef::String(""),
            GenericDataValueAst::Mapping { .. }
            | GenericDataValueAst::Sequence { .. }
            | GenericDataValueAst::Alias { .. } => CemtEvaluatorValueRef::Null,
        }),
        "style" => Some(match value {
            GenericDataValueAst::String { style, .. } => {
                CemtEvaluatorValueRef::String(style.as_deref().unwrap_or("plain"))
            }
            GenericDataValueAst::Number { .. }
            | GenericDataValueAst::Boolean { .. }
            | GenericDataValueAst::Null { .. } => CemtEvaluatorValueRef::String("plain"),
            GenericDataValueAst::Mapping { .. }
            | GenericDataValueAst::Sequence { .. }
            | GenericDataValueAst::Alias { .. } => CemtEvaluatorValueRef::Null,
        }),
        "implicitKind" => Some(match value {
            GenericDataValueAst::String { .. } => CemtEvaluatorValueRef::String("string"),
            GenericDataValueAst::Number { number_kind, .. } => {
                CemtEvaluatorValueRef::String(number_kind.as_yaml_implicit_kind())
            }
            GenericDataValueAst::Boolean { .. } => CemtEvaluatorValueRef::String("boolean"),
            GenericDataValueAst::Null { .. } => CemtEvaluatorValueRef::String("null"),
            GenericDataValueAst::Mapping { .. }
            | GenericDataValueAst::Sequence { .. }
            | GenericDataValueAst::Alias { .. } => CemtEvaluatorValueRef::Null,
        }),
        "byteOffset" => Some(u64_evaluator_value(source_range.byte_offset)),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::GenericDataSourceRange { source_range },
        )),
        "sourceMap" => Some(generic_data_source_map_evaluator_value(source_range)),
        "sequence" => Some(CemtEvaluatorValueRef::Sequence(match value {
            GenericDataValueAst::Sequence { items, .. } => {
                CemtEvaluatorSequenceRef::GenericDataYamlValues { values: items }
            }
            _ => CemtEvaluatorSequenceRef::Empty,
        })),
        "mapping" => Some(CemtEvaluatorValueRef::Sequence(match value {
            GenericDataValueAst::Mapping { entries, .. } => {
                CemtEvaluatorSequenceRef::GenericDataYamlPairs { entries }
            }
            _ => CemtEvaluatorSequenceRef::Empty,
        })),
        _ => None,
    }
}

fn generic_data_yaml_pair_evaluator_field<'a>(
    entry: &'a GenericDataMappingEntryAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "index" => Some(usize_evaluator_value(entry.index)),
        "key" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::GenericDataYamlNode { value: &entry.key },
        )),
        "value" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::GenericDataYamlNode {
                value: &entry.value,
            },
        )),
        _ => None,
    }
}

fn usize_evaluator_value(value: usize) -> CemtEvaluatorValueRef<'static> {
    u64_evaluator_value(u64::try_from(value).unwrap_or(u64::MAX))
}

fn optional_usize_evaluator_value(value: Option<usize>) -> CemtEvaluatorValueRef<'static> {
    match value {
        Some(value) => usize_evaluator_value(value),
        None => CemtEvaluatorValueRef::Null,
    }
}

fn u64_evaluator_value(value: u64) -> CemtEvaluatorValueRef<'static> {
    CemtEvaluatorValueRef::Number(CemtEvaluatorNumber::unsigned_integer(value))
}

fn json_document_evaluator_field<'a>(
    document: &'a JsonDocumentAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String("json-document")),
        "contentType" => Some(CemtEvaluatorValueRef::String(JSON_CONTENT_TYPE)),
        "schema" => Some(CemtEvaluatorValueRef::String(
            crate::schema::registry::JSON_VALUE_SCHEMA_URI,
        )),
        "encoding" => Some(CemtEvaluatorValueRef::String(&document.encoding)),
        "lineEnding" => Some(match document.line_ending.as_deref() {
            Some(line_ending) => CemtEvaluatorValueRef::String(line_ending),
            None => CemtEvaluatorValueRef::Null,
        }),
        "root" => Some(match document.root.as_ref() {
            Some(value) => {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::JsonValue { value })
            }
            None => CemtEvaluatorValueRef::Null,
        }),
        _ => None,
    }
}

fn json_value_evaluator_field<'a>(
    value: &'a JsonValueAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    let range = value.range();
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(match value {
            JsonValueAst::Object { .. } => "object",
            JsonValueAst::Array { .. } => "array",
            JsonValueAst::String { .. } => "string",
            JsonValueAst::Number { .. } => "number",
            JsonValueAst::Boolean { .. } => "boolean",
            JsonValueAst::Null { .. } => "null",
        })),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::JsonSourceRange { range },
        )),
        "sourceMap" => Some(CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(
            range.source_map(),
        ))),
        "members" => match value {
            JsonValueAst::Object { members, .. } => Some(CemtEvaluatorValueRef::Sequence(
                CemtEvaluatorSequenceRef::JsonMembers { members },
            )),
            _ => None,
        },
        "items" => match value {
            JsonValueAst::Array { items, .. } => Some(CemtEvaluatorValueRef::Sequence(
                CemtEvaluatorSequenceRef::JsonValues { values: items },
            )),
            _ => None,
        },
        "value" => match value {
            JsonValueAst::String { value, .. } => Some(CemtEvaluatorValueRef::String(value)),
            JsonValueAst::Boolean { value, .. } => Some(CemtEvaluatorValueRef::Boolean(*value)),
            _ => None,
        },
        "lexeme" => match value {
            JsonValueAst::String { lexeme, .. } | JsonValueAst::Number { lexeme, .. } => {
                Some(CemtEvaluatorValueRef::String(lexeme))
            }
            _ => None,
        },
        "numberKind" => match value {
            JsonValueAst::Number { number_kind, .. } => {
                Some(CemtEvaluatorValueRef::String(match number_kind {
                    JsonNumberKind::Integer => "integer",
                    JsonNumberKind::Decimal => "decimal",
                    JsonNumberKind::Exponent => "exponent",
                }))
            }
            _ => None,
        },
        _ => None,
    }
}

fn json_member_evaluator_field<'a>(
    member: &'a JsonMemberAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "index" => Some(CemtEvaluatorValueRef::Number(
            CemtEvaluatorNumber::unsigned_integer(member.index as u64),
        )),
        "name" => Some(CemtEvaluatorValueRef::String(&member.name)),
        "nameLexeme" => Some(CemtEvaluatorValueRef::String(&member.name_lexeme)),
        "nameSourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::JsonSourceRange {
                range: member.name_range,
            },
        )),
        "nameSourceMap" => Some(CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(
            member.name_range.source_map(),
        ))),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::JsonSourceRange {
                range: member.range,
            },
        )),
        "sourceMap" => Some(CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(
            member.range.source_map(),
        ))),
        "value" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::JsonValue {
                value: &member.value,
            },
        )),
        _ => None,
    }
}

fn json_source_range_evaluator_field(
    range: JsonSourceRange,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'static>> {
    let value = match name {
        "byteOffset" => range.start.byte_offset,
        "byteLength" => range.byte_length,
        "line" => u64::from(range.start.line),
        "column" => u64::from(range.start.column),
        _ => return None,
    };
    Some(CemtEvaluatorValueRef::Number(
        CemtEvaluatorNumber::unsigned_integer(value),
    ))
}

fn json_schema_document_evaluator_field<'a>(
    document: &'a JsonSchemaDocumentAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String("json-schema-document")),
        "contentType" => Some(CemtEvaluatorValueRef::String(JSON_SCHEMA_CONTENT_TYPE)),
        "schema" => Some(CemtEvaluatorValueRef::String(JSON_SCHEMA_SCHEMA_URI)),
        "source" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::JsonSchemaSource {
                source: &document.source,
            },
        )),
        "json" => Some(JsonDocumentCemtSubjectRef::new(&document.json).evaluator_view()),
        "parseFacts" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::JsonSchemaParseFacts {
                facts: &document.parse_facts,
            },
        )),
        "dialectFacts" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::JsonSchemaDialectFacts {
                facts: &document.dialect_facts,
            },
        )),
        "dialect" => Some(CemtEvaluatorValueRef::String(&document.dialect)),
        _ => None,
    }
}

fn json_schema_source_evaluator_field<'a>(
    source: &'a JsonSchemaDocumentSource,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "uri" => Some(CemtEvaluatorValueRef::String(&source.uri)),
        "contentType" => Some(CemtEvaluatorValueRef::String(&source.content_type)),
        "mediaType" => Some(CemtEvaluatorValueRef::String(&source.media_type)),
        "parameters" => Some(CemtEvaluatorValueRef::StringMap(&source.parameters)),
        "byteLength" => Some(CemtEvaluatorValueRef::Number(
            CemtEvaluatorNumber::unsigned_integer(
                u64::try_from(source.byte_length).unwrap_or(u64::MAX),
            ),
        )),
        _ => None,
    }
}

fn json_schema_parse_fact_evaluator_field<'a>(
    fact: &'a JsonSchemaParseFact,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(fact.kind.as_str())),
        "diagnosticCode" => Some(CemtEvaluatorValueRef::String(&fact.diagnostic_code)),
        "diagnosticSeverity" => Some(CemtEvaluatorValueRef::String(&fact.diagnostic_severity)),
        "fatal" => Some(CemtEvaluatorValueRef::Boolean(fact.fatal)),
        "memberName" => Some(optional_string_evaluator_value(fact.member_name.as_deref())),
        "line" => Some(optional_u32_evaluator_value(fact.line)),
        "column" => Some(optional_u32_evaluator_value(fact.column)),
        "byteOffset" => Some(optional_u64_evaluator_value(fact.byte_offset)),
        "byteLength" => Some(optional_u64_evaluator_value(fact.byte_length)),
        "message" => Some(CemtEvaluatorValueRef::String(&fact.message)),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::JsonSchemaParseFactSourceRange { fact },
        )),
        _ => None,
    }
}

fn json_schema_parse_fact_source_range_evaluator_field(
    fact: &JsonSchemaParseFact,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'static>> {
    match name {
        "byteOffset" => Some(optional_u64_evaluator_value(fact.byte_offset)),
        "byteLength" => Some(optional_u64_evaluator_value(fact.byte_length)),
        "line" => Some(optional_u32_evaluator_value(fact.line)),
        "column" => Some(optional_u32_evaluator_value(fact.column)),
        _ => None,
    }
}

fn json_schema_dialect_fact_evaluator_field<'a>(
    fact: &'a JsonSchemaDialectFact,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(fact.kind.as_str())),
        "dialect" => Some(optional_string_evaluator_value(fact.dialect.as_deref())),
        "diagnosticCode" => Some(optional_string_evaluator_value(
            fact.diagnostic_code.as_deref(),
        )),
        "diagnosticSeverity" => Some(optional_string_evaluator_value(
            fact.diagnostic_severity.as_deref(),
        )),
        "fatal" => Some(CemtEvaluatorValueRef::Boolean(fact.fatal)),
        "message" => Some(CemtEvaluatorValueRef::String(&fact.message)),
        "sourceRange" => fact.source_range.map(|range| {
            CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::JsonSourceRange { range })
        }),
        "sourceMap" => fact.source_range.map(|range| {
            CemtEvaluatorValueRef::OwnedSourceMap(Arc::new(json_schema_source_map(range)))
        }),
        _ => None,
    }
}

fn optional_u32_evaluator_value(value: Option<u32>) -> CemtEvaluatorValueRef<'static> {
    optional_u64_evaluator_value(value.map(u64::from))
}

fn optional_u64_evaluator_value(value: Option<u64>) -> CemtEvaluatorValueRef<'static> {
    match value {
        Some(value) => CemtEvaluatorValueRef::Number(CemtEvaluatorNumber::unsigned_integer(value)),
        None => CemtEvaluatorValueRef::Null,
    }
}

fn generic_data_json_document_evaluator_field<'a>(
    document: &'a GenericDataDocumentAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String("json-document")),
        "contentType" => Some(CemtEvaluatorValueRef::String(JSON_CONTENT_TYPE)),
        "schema" => Some(CemtEvaluatorValueRef::String(
            crate::schema::registry::JSON_VALUE_SCHEMA_URI,
        )),
        "encoding" => Some(CemtEvaluatorValueRef::String("utf-8")),
        "lineEnding" => Some(match document.line_ending.as_deref() {
            Some(line_ending) => CemtEvaluatorValueRef::String(line_ending),
            None => CemtEvaluatorValueRef::Null,
        }),
        "root" => match document.documents.as_slice() {
            [] => Some(CemtEvaluatorValueRef::Record(
                CemtEvaluatorRecordRef::GenericDataJsonGeneratedNull,
            )),
            [stream_document] => generic_data_json_stream_document_value(stream_document),
            documents => Some(CemtEvaluatorValueRef::Record(
                CemtEvaluatorRecordRef::GenericDataJsonDocumentSequenceRoot { documents },
            )),
        },
        _ => None,
    }
}

fn generic_data_json_stream_document_value<'a>(
    document: &'a GenericDataStreamDocumentAst,
) -> Option<CemtEvaluatorValueRef<'a>> {
    Some(match document.root.as_ref() {
        Some(value) => {
            CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::GenericDataJsonValue { value })
        }
        None => CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::GenericDataJsonMissingRoot {
            source_range: &document.source_range,
        }),
    })
}

fn generic_data_json_document_sequence_root_evaluator_field<'a>(
    documents: &'a [GenericDataStreamDocumentAst],
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String("array")),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::GenericDataGeneratedSourceRange,
        )),
        "sourceMap" => Some(CemtEvaluatorValueRef::Null),
        "items" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::GenericDataJsonDocuments { documents },
        )),
        _ => None,
    }
}

fn generic_data_json_value_evaluator_field<'a>(
    value: &'a GenericDataValueAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    let source_range = value.source_range();
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(match value {
            GenericDataValueAst::Mapping { .. } => "object",
            GenericDataValueAst::Sequence { .. } => "array",
            GenericDataValueAst::String { .. } => "string",
            GenericDataValueAst::Number { .. } => "number",
            GenericDataValueAst::Boolean { .. } => "boolean",
            GenericDataValueAst::Null { .. } | GenericDataValueAst::Alias { .. } => "null",
        })),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::GenericDataSourceRange { source_range },
        )),
        "sourceMap" => Some(generic_data_source_map_evaluator_value(source_range)),
        "members" => match value {
            GenericDataValueAst::Mapping { entries, .. } => Some(CemtEvaluatorValueRef::Sequence(
                CemtEvaluatorSequenceRef::GenericDataJsonEntries { entries },
            )),
            _ => None,
        },
        "items" => match value {
            GenericDataValueAst::Sequence { items, .. } => Some(CemtEvaluatorValueRef::Sequence(
                CemtEvaluatorSequenceRef::GenericDataJsonValues { values: items },
            )),
            _ => None,
        },
        "value" => match value {
            GenericDataValueAst::String { value, .. } => Some(CemtEvaluatorValueRef::String(value)),
            GenericDataValueAst::Boolean { value, .. } => {
                Some(CemtEvaluatorValueRef::Boolean(*value))
            }
            _ => None,
        },
        "lexeme" => match value {
            GenericDataValueAst::String { lexeme, .. } => {
                lexeme.as_deref().map(CemtEvaluatorValueRef::String)
            }
            GenericDataValueAst::Number { lexeme, .. } => Some(CemtEvaluatorValueRef::OwnedString(
                Arc::from(normalize_generic_data_json_number_lexeme(lexeme)),
            )),
            _ => None,
        },
        "numberKind" => match value {
            GenericDataValueAst::Number { number_kind, .. } => Some(CemtEvaluatorValueRef::String(
                number_kind.as_json_number_kind(),
            )),
            _ => None,
        },
        _ => None,
    }
}

fn generic_data_json_member_evaluator_field<'a>(
    entry: &'a GenericDataMappingEntryAst,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    let member_name = || generic_data_json_member_name(&entry.key);
    let name_source_range = entry.key.source_range();
    match name {
        "index" => Some(CemtEvaluatorValueRef::Number(
            CemtEvaluatorNumber::unsigned_integer(entry.index as u64),
        )),
        "name" => Some(CemtEvaluatorValueRef::OwnedString(Arc::from(member_name()))),
        "nameLexeme" => Some(CemtEvaluatorValueRef::OwnedString(Arc::from(
            quote_generic_data_json_string(&member_name()),
        ))),
        "nameSourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::GenericDataSourceRange {
                source_range: name_source_range,
            },
        )),
        "nameSourceMap" => Some(generic_data_source_map_evaluator_value(name_source_range)),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::GenericDataSourceRange {
                source_range: &entry.source_range,
            },
        )),
        "sourceMap" => Some(generic_data_source_map_evaluator_value(&entry.source_range)),
        "value" => Some(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::GenericDataJsonValue {
                value: &entry.value,
            },
        )),
        _ => None,
    }
}

fn generic_data_json_null_evaluator_field<'a>(
    source_range: Option<&'a GenericDataSourceRangeAst>,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String("null")),
        "sourceRange" => Some(CemtEvaluatorValueRef::Record(match source_range {
            Some(source_range) => CemtEvaluatorRecordRef::GenericDataSourceRange { source_range },
            None => CemtEvaluatorRecordRef::GenericDataGeneratedSourceRange,
        })),
        "sourceMap" => Some(match source_range {
            Some(source_range) => generic_data_source_map_evaluator_value(source_range),
            None => CemtEvaluatorValueRef::Null,
        }),
        _ => None,
    }
}

fn generic_data_source_range_evaluator_field(
    source_range: Option<&GenericDataSourceRangeAst>,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'static>> {
    let value = match (source_range, name) {
        (Some(source_range), "byteOffset") => source_range.byte_offset,
        (Some(source_range), "byteLength") => source_range.byte_length,
        (Some(source_range), "line") => u64::from(source_range.line),
        (Some(source_range), "column") => u64::from(source_range.column),
        (None, "byteOffset" | "byteLength") => 0,
        (None, "line" | "column") => 1,
        _ => return None,
    };
    Some(CemtEvaluatorValueRef::Number(
        CemtEvaluatorNumber::unsigned_integer(value),
    ))
}

fn generic_data_source_map_evaluator_value(
    source_range: &GenericDataSourceRangeAst,
) -> CemtEvaluatorValueRef<'_> {
    match source_range.source_map.as_ref() {
        Some(source_map) => CemtEvaluatorValueRef::SourceMap(source_map),
        None => CemtEvaluatorValueRef::Null,
    }
}

fn generic_data_json_member_name(value: &GenericDataValueAst) -> String {
    match value {
        GenericDataValueAst::String { value, .. } => value.clone(),
        GenericDataValueAst::Number { lexeme, .. } => {
            normalize_generic_data_json_number_lexeme(lexeme)
        }
        GenericDataValueAst::Boolean { value, .. } => value.to_string(),
        GenericDataValueAst::Null { .. } => "null".to_owned(),
        GenericDataValueAst::Alias { alias, .. } => alias.clone().unwrap_or_default(),
        GenericDataValueAst::Mapping { .. } | GenericDataValueAst::Sequence { .. } => {
            compact_generic_data_json_value(value)
        }
    }
}

fn compact_generic_data_json_value(value: &GenericDataValueAst) -> String {
    match value {
        GenericDataValueAst::Mapping { entries, .. } => {
            let mut output = String::from("{");
            for (index, entry) in entries.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(&quote_generic_data_json_string(
                    &generic_data_json_member_name(&entry.key),
                ));
                output.push(':');
                output.push_str(&compact_generic_data_json_value(&entry.value));
            }
            output.push('}');
            output
        }
        GenericDataValueAst::Sequence { items, .. } => {
            let mut output = String::from("[");
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(&compact_generic_data_json_value(item));
            }
            output.push(']');
            output
        }
        GenericDataValueAst::String { value, .. } => quote_generic_data_json_string(value),
        GenericDataValueAst::Number { lexeme, .. } => {
            normalize_generic_data_json_number_lexeme(lexeme)
        }
        GenericDataValueAst::Boolean { value, .. } => value.to_string(),
        GenericDataValueAst::Null { .. } | GenericDataValueAst::Alias { .. } => "null".to_owned(),
    }
}

fn quote_generic_data_json_string(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(value.len().saturating_add(2));
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character <= '\u{001f}' => {
                let code = character as u8;
                output.push_str("\\u00");
                output.push(HEX[usize::from(code >> 4)] as char);
                output.push(HEX[usize::from(code & 0x0f)] as char);
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

fn normalize_generic_data_json_number_lexeme(lexeme: &str) -> String {
    if is_json_number_lexeme(lexeme) {
        return lexeme.to_owned();
    }
    if let Ok(value) = lexeme.parse::<i64>() {
        return value.to_string();
    }
    if let Ok(value) = lexeme.parse::<f64>() {
        if value.is_finite() {
            return ryu::Buffer::new().format_finite(value).to_owned();
        }
    }
    "0".to_owned()
}

fn is_json_number_lexeme(lexeme: &str) -> bool {
    let bytes = lexeme.as_bytes();
    let mut cursor = 0usize;
    if bytes.get(cursor) == Some(&b'-') {
        cursor += 1;
    }
    match bytes.get(cursor) {
        Some(b'0') => cursor += 1,
        Some(b'1'..=b'9') => {
            cursor += 1;
            while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
                cursor += 1;
            }
        }
        _ => return false,
    }
    if bytes.get(cursor) == Some(&b'.') {
        cursor += 1;
        let fraction_start = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        if cursor == fraction_start {
            return false;
        }
    }
    if matches!(bytes.get(cursor), Some(b'e' | b'E')) {
        cursor += 1;
        if matches!(bytes.get(cursor), Some(b'+' | b'-')) {
            cursor += 1;
        }
        let exponent_start = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        if cursor == exponent_start {
            return false;
        }
    }
    cursor == bytes.len()
}

fn optional_string_evaluator_value(value: Option<&str>) -> CemtEvaluatorValueRef<'_> {
    match value {
        Some(value) => CemtEvaluatorValueRef::String(value),
        None => CemtEvaluatorValueRef::Null,
    }
}

fn writer_token_style_evaluator_field<'a>(
    style: &'a CemTreeAstWriterTokenStyle,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    let value = match name {
        "colorRole" => style.color_role.as_deref(),
        "colorProfile" => style.color_profile.as_deref(),
        "colorOutput" => style.color_output.as_deref(),
        "htmlMode" => style.html_mode.as_deref(),
        "class" => style.class_name.as_deref(),
        "color" => style.color.as_deref(),
        "wrapper" => style.wrapper.as_deref(),
        "terminalCapability" => style.terminal_capability.as_deref(),
        "tabular" => {
            return Some(optional_bool_evaluator_value(style.tabular));
        }
        _ => return None,
    };
    Some(optional_string_evaluator_value(value))
}

fn writer_token_metadata_evaluator_field<'a>(
    metadata: &'a CemTreeAstWriterTokenMetadata,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "name" => Some(optional_string_evaluator_value(metadata.name.as_deref())),
        "formatterProfile" => Some(optional_string_evaluator_value(
            metadata.formatter_profile.as_deref(),
        )),
        "formatterRole" => Some(optional_string_evaluator_value(
            metadata.formatter_role.as_deref(),
        )),
        "sourceRange" => Some(match metadata.source_range.as_ref() {
            Some(range) => {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::WriterTokenSourceRange {
                    range,
                })
            }
            None => CemtEvaluatorValueRef::Null,
        }),
        "memberIndex" => Some(match metadata.member_index {
            Some(index) => {
                CemtEvaluatorValueRef::Number(CemtEvaluatorNumber::unsigned_integer(index))
            }
            None => CemtEvaluatorValueRef::Null,
        }),
        "eventIndex" => Some(optional_u64_evaluator_value(metadata.event_index)),
        "eventKind" => Some(optional_string_evaluator_value(
            metadata.event_kind.as_deref(),
        )),
        "eventTag" => Some(optional_string_evaluator_value(
            metadata.event_tag.as_deref(),
        )),
        "package" => Some(optional_string_evaluator_value(metadata.package.as_deref())),
        "syntaxKind" => Some(optional_string_evaluator_value(
            metadata.syntax_kind.as_deref(),
        )),
        "depth" => Some(optional_u64_evaluator_value(metadata.depth)),
        "qualifiedName" => Some(optional_string_evaluator_value(
            metadata.qualified_name.as_deref(),
        )),
        "lexicalName" => Some(optional_string_evaluator_value(
            metadata.lexical_name.as_deref(),
        )),
        "localName" => Some(optional_string_evaluator_value(
            metadata.local_name.as_deref(),
        )),
        "namespaceUri" => Some(optional_string_evaluator_value(
            metadata.namespace_uri.as_deref(),
        )),
        "tokenKind" => Some(optional_string_evaluator_value(
            metadata.token_kind.as_deref(),
        )),
        "lexeme" => Some(optional_string_evaluator_value(metadata.lexeme.as_deref())),
        "index" => Some(optional_u64_evaluator_value(metadata.index)),
        "role" => Some(optional_string_evaluator_value(metadata.role.as_deref())),
        "operator" => Some(optional_string_evaluator_value(
            metadata.operator.as_deref(),
        )),
        "cemQlRole" => Some(optional_string_evaluator_value(
            metadata.cem_ql_role.as_deref(),
        )),
        "legacy" => Some(optional_string_evaluator_value(metadata.legacy.as_deref())),
        "diagnostic" => Some(optional_string_evaluator_value(
            metadata.diagnostic.as_deref(),
        )),
        "replacement" => Some(optional_string_evaluator_value(
            metadata.replacement.as_deref(),
        )),
        "documentSafeBoundary" => Some(optional_bool_evaluator_value(
            metadata.document_safe_boundary,
        )),
        "lexicalSafeBoundary" => Some(optional_bool_evaluator_value(
            metadata.lexical_safe_boundary,
        )),
        "layoutSensitive" => Some(optional_bool_evaluator_value(metadata.layout_sensitive)),
        "generated" => Some(optional_bool_evaluator_value(metadata.generated)),
        "layout" => Some(optional_string_evaluator_value(metadata.layout.as_deref())),
        "lineEnding" => Some(optional_string_evaluator_value(
            metadata.line_ending.as_deref(),
        )),
        "indent" => Some(optional_string_evaluator_value(metadata.indent.as_deref())),
        "leadingComma" => Some(match metadata.leading_comma {
            Some(value) => CemtEvaluatorValueRef::Boolean(value),
            None => CemtEvaluatorValueRef::Null,
        }),
        "scopeOpeningNewLine" => Some(match metadata.scope_opening_new_line {
            Some(value) => CemtEvaluatorValueRef::Boolean(value),
            None => CemtEvaluatorValueRef::Null,
        }),
        "delimiter" => Some(optional_string_evaluator_value(
            metadata.delimiter.as_deref(),
        )),
        "rowIndex" => Some(optional_u64_evaluator_value(metadata.row_index)),
        "fieldIndex" => Some(optional_u64_evaluator_value(metadata.field_index)),
        "raw" => Some(optional_string_evaluator_value(metadata.raw.as_deref())),
        "quoted" => Some(optional_bool_evaluator_value(metadata.quoted)),
        "byteOffset" => Some(optional_u64_evaluator_value(metadata.byte_offset)),
        "byteLength" => Some(optional_u64_evaluator_value(metadata.byte_length)),
        "rowSourceRange" => Some(match metadata.row_source_range.as_ref() {
            Some(range) => {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::WriterTokenSourceRange {
                    range,
                })
            }
            None => CemtEvaluatorValueRef::Null,
        }),
        "rowByteOffset" => Some(optional_u64_evaluator_value(metadata.row_byte_offset)),
        "rowByteLength" => Some(optional_u64_evaluator_value(metadata.row_byte_length)),
        "fieldCount" => Some(optional_u64_evaluator_value(metadata.field_count)),
        "tabSize" => Some(optional_u64_evaluator_value(metadata.tab_size)),
        "presentationOnly" => Some(optional_bool_evaluator_value(metadata.presentation_only)),
        "strictCsv" => Some(optional_bool_evaluator_value(metadata.strict_csv)),
        "dataPreserving" => Some(optional_bool_evaluator_value(metadata.data_preserving)),
        "sourcePreserving" => Some(optional_bool_evaluator_value(metadata.source_preserving)),
        _ => None,
    }
}

fn optional_bool_evaluator_value(value: Option<bool>) -> CemtEvaluatorValueRef<'static> {
    match value {
        Some(value) => CemtEvaluatorValueRef::Boolean(value),
        None => CemtEvaluatorValueRef::Null,
    }
}

fn writer_token_source_range_evaluator_field(
    range: &CemTreeAstWriterTokenSourceRange,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'static>> {
    let value = match name {
        "byteOffset" => range.byte_offset,
        "byteLength" => range.byte_length,
        "line" => u64::from(range.line),
        "column" => u64::from(range.column),
        _ => return None,
    };
    Some(CemtEvaluatorValueRef::Number(
        CemtEvaluatorNumber::unsigned_integer(value),
    ))
}

fn cemt_evaluator_node_field<'a>(
    node: &'a CemTreeAstNode,
    path: &CemtOwnerPath,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    if name == "kind" {
        return Some(CemtEvaluatorValueRef::String(match node {
            CemTreeAstNode::WriterToken { token_kind, .. } => token_kind,
            _ => node.kind(),
        }));
    }
    if name == "sourceMap" {
        return Some(CemtEvaluatorValueRef::SourceMap(node.source_map()));
    }
    match (node, name) {
        (CemTreeAstNode::Document { children, .. }, "children") => {
            Some(CemtEvaluatorValueRef::Sequence(
                CemtEvaluatorSequenceRef::nodes(children, Some(path.clone())),
            ))
        }
        (CemTreeAstNode::Element { name: value, .. }, "name") => {
            Some(CemtEvaluatorValueRef::String(value))
        }
        (CemTreeAstNode::Element { attributes, .. }, "attributes") => {
            Some(CemtEvaluatorValueRef::Sequence(
                CemtEvaluatorSequenceRef::attributes(attributes, path.clone()),
            ))
        }
        (CemTreeAstNode::Element { children, .. }, "children") => {
            Some(CemtEvaluatorValueRef::Sequence(
                CemtEvaluatorSequenceRef::nodes(children, Some(path.clone())),
            ))
        }
        (CemTreeAstNode::Text { value, .. }, "value") => Some(CemtEvaluatorValueRef::String(value)),
        (
            CemTreeAstNode::Whitespace { data, .. }
            | CemTreeAstNode::Comment { data, .. }
            | CemTreeAstNode::Cdata { data, .. }
            | CemTreeAstNode::RawText { data, .. },
            "data",
        ) => Some(CemtEvaluatorValueRef::String(data)),
        (
            CemTreeAstNode::ProcessingInstruction {
                name: value,
                target,
                data,
                ..
            },
            field,
        ) => match field {
            "name" => Some(CemtEvaluatorValueRef::String(value)),
            "target" => Some(CemtEvaluatorValueRef::String(target)),
            "data" => Some(CemtEvaluatorValueRef::String(data)),
            _ => None,
        },
        (CemTreeAstNode::Error { code, .. }, "code") => Some(CemtEvaluatorValueRef::String(code)),
        (CemTreeAstNode::WriterToken { .. }, "writerKind") => {
            Some(CemtEvaluatorValueRef::String("token"))
        }
        (CemTreeAstNode::WriterToken { token_kind, .. }, "tokenKind") => {
            Some(CemtEvaluatorValueRef::String(token_kind))
        }
        (CemTreeAstNode::WriterToken { text, .. }, "text") => {
            Some(CemtEvaluatorValueRef::String(text))
        }
        (CemTreeAstNode::WriterToken { role, .. }, "role") => {
            Some(CemtEvaluatorValueRef::String(role))
        }
        (CemTreeAstNode::WriterToken { style, .. }, "style") => Some(
            CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::WriterTokenStyle { style }),
        ),
        (CemTreeAstNode::WriterToken { metadata, .. }, "value") => Some(
            CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::WriterTokenMetadata { metadata }),
        ),
        (CemTreeAstNode::WriterToken { output_span, .. }, "outputSpan") => Some(match output_span
            .as_ref()
        {
            Some(output_span) => {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::OutputSpan { output_span })
            }
            None => CemtEvaluatorValueRef::Null,
        }),
        _ => None,
    }
}

fn cemt_evaluator_formatted_tree_field<'a>(
    subject: CemtTreeSubjectRef<'a>,
    overlay: &'a CemtFormattedTreeOverlay,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String("cem-tree")),
        "contentType" => Some(CemtEvaluatorValueRef::String(
            &overlay.envelope.content_type,
        )),
        "schema" => Some(CemtEvaluatorValueRef::String(&overlay.envelope.schema)),
        "category" => Some(CemtEvaluatorValueRef::String(&overlay.envelope.category)),
        "mode" => Some(CemtEvaluatorValueRef::String(
            overlay.envelope.mode.as_str(),
        )),
        "canonical" => Some(CemtEvaluatorValueRef::Boolean(overlay.envelope.canonical)),
        "formatterProfile" => Some(match overlay.producer.formatter_profile.as_deref() {
            Some(profile) => CemtEvaluatorValueRef::String(profile),
            None => CemtEvaluatorValueRef::Null,
        }),
        "formatNodes" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::FormatOperations {
                operations: &overlay.operations,
            },
        )),
        "nodes" => Some(CemtEvaluatorValueRef::Sequence(
            CemtEvaluatorSequenceRef::formatted_nodes(subject.nodes(), None, overlay),
        )),
        _ => None,
    }
}

fn cemt_evaluator_formatted_node_field<'a>(
    node: &'a CemTreeAstNode,
    path: &CemtOwnerPath,
    overlay: &'a CemtFormattedTreeOverlay,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "formatLayout" => cemt_evaluator_owner_operation(overlay, path, |kind| {
            matches!(kind, CemtNodeFormatOperationKind::Layout(_))
        }),
        "formatBeforeAttributes" => cemt_evaluator_owner_operation(overlay, path, |kind| {
            matches!(kind, CemtNodeFormatOperationKind::BeforeAttributes { .. })
        }),
        "formatBetweenAttributes" => cemt_evaluator_owner_operation(overlay, path, |kind| {
            matches!(kind, CemtNodeFormatOperationKind::BetweenAttributes { .. })
        }),
        "attributes" => match node {
            CemTreeAstNode::Element { attributes, .. } => Some(CemtEvaluatorValueRef::Sequence(
                CemtEvaluatorSequenceRef::formatted_attributes(attributes, path.clone(), overlay),
            )),
            _ => None,
        },
        "formatContentBoundary" => {
            let sequence = CemtEvaluatorSequenceRef::node_format_fragments(
                &overlay.node_operations,
                path.clone(),
                CemtEvaluatorNodeFormatSequenceKind::ContentBoundary,
            );
            (!sequence.is_empty()).then_some(CemtEvaluatorValueRef::Sequence(sequence))
        }
        "children" => match node {
            CemTreeAstNode::Document { children, .. }
            | CemTreeAstNode::Element { children, .. } => Some(CemtEvaluatorValueRef::Sequence(
                CemtEvaluatorSequenceRef::formatted_nodes(children, Some(path.clone()), overlay),
            )),
            _ => None,
        },
        "formatBeforeClose" => cemt_evaluator_owner_operation(overlay, path, |kind| {
            matches!(kind, CemtNodeFormatOperationKind::BeforeClose { .. })
        }),
        _ => cemt_evaluator_node_field(node, path, name),
    }
}

fn cemt_evaluator_formatted_attribute_field<'a>(
    attribute: &'a CemTreeAstAttribute,
    path: &CemtOwnerPath,
    overlay: &'a CemtFormattedTreeOverlay,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String("attribute")),
        "name" => Some(CemtEvaluatorValueRef::String(&attribute.name)),
        "value" => Some(match attribute.value.as_deref() {
            Some(value) => CemtEvaluatorValueRef::String(value),
            None => CemtEvaluatorValueRef::Null,
        }),
        "formatBefore" => {
            let sequence = CemtEvaluatorSequenceRef::node_format_fragments(
                &overlay.node_operations,
                path.clone(),
                CemtEvaluatorNodeFormatSequenceKind::BeforeAttribute,
            );
            match sequence.len() {
                0 => None,
                1 => sequence.item(0),
                _ => Some(CemtEvaluatorValueRef::Sequence(sequence)),
            }
        }
        "sourceMap" => Some(CemtEvaluatorValueRef::SourceMap(&attribute.source)),
        _ => None,
    }
}

fn cemt_evaluator_owner_operation<'a>(
    overlay: &'a CemtFormattedTreeOverlay,
    path: &CemtOwnerPath,
    kind_matches: impl Fn(&CemtNodeFormatOperationKind) -> bool,
) -> Option<CemtEvaluatorValueRef<'a>> {
    overlay
        .node_operations
        .iter()
        .enumerate()
        .find(|(_, operation)| {
            matches!(&operation.target, CemtNodeFormatTarget::Owner(owner) if owner == path)
                && kind_matches(&operation.kind)
        })
        .map(|(index, operation)| {
            CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::NodeFormatOperation {
                operation,
                index,
            })
        })
}

fn cemt_evaluator_format_operation_field<'a>(
    operation: &'a CemtFormatOperation,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(match &operation.kind {
            CemtFormatOperationKind::Marker => "format-marker",
            CemtFormatOperationKind::Decision { .. } => "format-decision",
        })),
        "name" => Some(CemtEvaluatorValueRef::String(&operation.name)),
        "formatterRole" => Some(CemtEvaluatorValueRef::String(&operation.formatter_role)),
        "formatterProfile" => operation
            .formatter_profile
            .as_deref()
            .map(CemtEvaluatorValueRef::String),
        "colorRole" => operation
            .color_role
            .as_deref()
            .map(CemtEvaluatorValueRef::String),
        "value" => match &operation.kind {
            CemtFormatOperationKind::Decision { value } => {
                Some(CemtEvaluatorValueRef::String(value))
            }
            CemtFormatOperationKind::Marker => None,
        },
        "sourceMap" => cemt_overlay_provenance_source_map(&operation.provenance)
            .map(CemtEvaluatorValueRef::SourceMap),
        _ => None,
    }
}

fn cemt_evaluator_node_format_operation_field<'a>(
    operation: &'a CemtNodeFormatOperation,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    if let CemtNodeFormatOperationKind::Layout(layout) = &operation.kind {
        return match name {
            "kind" => Some(CemtEvaluatorValueRef::String("format-decision")),
            "name" => Some(CemtEvaluatorValueRef::String("layout")),
            "formatterRole" => Some(CemtEvaluatorValueRef::String(&operation.formatter_role)),
            "colorRole" => operation
                .color_role
                .as_deref()
                .map(CemtEvaluatorValueRef::String),
            "value" => Some(CemtEvaluatorValueRef::String(layout.as_str())),
            "sourceMap" => cemt_overlay_provenance_source_map(&operation.provenance)
                .map(CemtEvaluatorValueRef::SourceMap),
            _ => None,
        };
    }

    let fragment = cemt_evaluator_node_format_fragment(&operation.kind)?;
    match name {
        "kind" => Some(CemtEvaluatorValueRef::String(fragment.kind())),
        "value" => Some(CemtEvaluatorValueRef::String(fragment.value())),
        "formatterOwned" => Some(CemtEvaluatorValueRef::Boolean(true)),
        "formatterRole" => Some(CemtEvaluatorValueRef::String(&operation.formatter_role)),
        "colorRole" => operation
            .color_role
            .as_deref()
            .map(CemtEvaluatorValueRef::String),
        "sourceMap" => cemt_overlay_provenance_source_map(&operation.provenance)
            .map(CemtEvaluatorValueRef::SourceMap),
        _ => None,
    }
}

fn cemt_evaluator_node_format_fragment(
    kind: &CemtNodeFormatOperationKind,
) -> Option<&CemtFormatFragment> {
    match kind {
        CemtNodeFormatOperationKind::BeforeAttributes { fragment }
        | CemtNodeFormatOperationKind::BetweenAttributes { fragment }
        | CemtNodeFormatOperationKind::BeforeAttribute { fragment, .. }
        | CemtNodeFormatOperationKind::ContentBoundary { fragment, .. }
        | CemtNodeFormatOperationKind::InsertChild { fragment }
        | CemtNodeFormatOperationKind::BeforeClose { fragment } => Some(fragment),
        CemtNodeFormatOperationKind::Layout(_) => None,
    }
}

fn cemt_overlay_provenance_source_map(
    provenance: &CemtOverlayProvenance,
) -> Option<&SourceMapStack> {
    match provenance {
        CemtOverlayProvenance::SourceMapped(source_map) => Some(source_map),
        CemtOverlayProvenance::Generated { source_map, .. } => source_map.as_ref(),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CemtFormattedTreeView<'a> {
    subject: CemtTreeSubjectRef<'a>,
    overlay: &'a CemtFormattedTreeOverlay,
}

impl<'a> CemtFormattedTreeView<'a> {
    pub fn resolve_owner(self, path: &CemtOwnerPath) -> Option<CemtFormattedOwnerRef<'a>> {
        Some(CemtFormattedOwnerRef {
            owner: self.subject.resolve_owner(path)?,
            path: path.clone(),
            operations: &self.overlay.node_operations,
        })
    }

    pub fn root_gap_operations(self) -> impl Iterator<Item = &'a CemtNodeFormatOperation> + 'a {
        self.overlay
            .node_operations
            .iter()
            .filter(|operation| matches!(operation.target, CemtNodeFormatTarget::RootGap { .. }))
    }
}

#[derive(Debug, Clone)]
pub struct CemtFormattedOwnerRef<'a> {
    owner: CemtTreeOwnerRef<'a>,
    path: CemtOwnerPath,
    operations: &'a [CemtNodeFormatOperation],
}

impl<'a> CemtFormattedOwnerRef<'a> {
    pub fn owner(&self) -> CemtTreeOwnerRef<'a> {
        self.owner
    }

    pub fn operations(&self) -> impl Iterator<Item = &'a CemtNodeFormatOperation> + '_ {
        self.operations
            .iter()
            .filter(|operation| operation.target.belongs_to(&self.path))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CemtColoredTreeView<'a> {
    subject: CemtTreeSubjectRef<'a>,
    formatted_overlay: &'a CemtFormattedTreeOverlay,
    colored_overlay: &'a CemtColoredTreeOverlay,
}

impl<'a> CemtColoredTreeView<'a> {
    pub fn resolve_owner(self, path: &CemtOwnerPath) -> Option<CemtColoredOwnerRef<'a>> {
        Some(CemtColoredOwnerRef {
            owner: self.subject.resolve_owner(path)?,
            path: path.clone(),
            format_operations: &self.formatted_overlay.node_operations,
            color_operations: &self.colored_overlay.node_operations,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CemtColoredOwnerRef<'a> {
    owner: CemtTreeOwnerRef<'a>,
    path: CemtOwnerPath,
    format_operations: &'a [CemtNodeFormatOperation],
    color_operations: &'a [CemtNodeColorOperation],
}

impl<'a> CemtColoredOwnerRef<'a> {
    pub fn owner(&self) -> CemtTreeOwnerRef<'a> {
        self.owner
    }

    pub fn format_operations(&self) -> impl Iterator<Item = &'a CemtNodeFormatOperation> + '_ {
        self.format_operations
            .iter()
            .filter(|operation| operation.target.belongs_to(&self.path))
    }

    pub fn color_operations(&self) -> impl Iterator<Item = &'a CemtNodeColorOperation> + '_ {
        self.color_operations.iter().filter(|operation| {
            operation
                .target
                .belongs_to(&self.path, self.format_operations)
        })
    }
}

#[derive(Debug, Clone)]
pub struct TransformDataArtifact {
    pub artifact_id: String,
    pub uri: Option<String>,
    pub identity: Option<FormatIdentity>,
    pub body: TransformArtifactBody,
}

impl TransformDataArtifact {
    pub fn new(
        artifact_id: impl Into<String>,
        uri: Option<String>,
        identity: Option<FormatIdentity>,
        body: TransformArtifactBody,
    ) -> Self {
        Self {
            artifact_id: artifact_id.into(),
            uri,
            identity,
            body,
        }
    }

    pub fn encoded(
        artifact_id: impl Into<String>,
        uri: Option<String>,
        identity: FormatIdentity,
        encoding: TransformEncoding,
        bytes: impl Into<Arc<[u8]>>,
    ) -> Result<Self, TransformEncodedArtifactError> {
        let encoded = TransformEncodedArtifact::new(identity.clone(), encoding, bytes)?;
        Ok(Self::new(
            artifact_id,
            uri,
            Some(identity),
            TransformArtifactBody::Encoded(Arc::new(encoded)),
        ))
    }

    pub fn explicit_json(
        artifact_id: impl Into<String>,
        uri: Option<String>,
        identity: FormatIdentity,
        value: &serde_json::Value,
    ) -> Result<Self, TransformEncodedArtifactError> {
        let bytes = serde_json::to_vec(value).map_err(|error| {
            TransformEncodedArtifactError::JsonEncodingFailed {
                message: error.to_string(),
            }
        })?;
        Self::encoded(artifact_id, uri, identity, TransformEncoding::Json, bytes)
    }

    pub fn encoded_text(&self) -> Result<&str, TransformArtifactAccessError> {
        let TransformArtifactBody::Encoded(encoded) = &self.body else {
            return Err(TransformArtifactAccessError::UnsupportedRepresentation {
                expected: "encoded UTF-8 text",
                actual: self.body.representation_id(),
            });
        };
        if encoded.encoding != TransformEncoding::Text {
            return Err(TransformArtifactAccessError::UnsupportedRepresentation {
                expected: "encoded UTF-8 text",
                actual: self.body.representation_id(),
            });
        }
        std::str::from_utf8(encoded.bytes.as_ref()).map_err(|error| {
            TransformArtifactAccessError::InvalidUtf8 {
                message: error.to_string(),
            }
        })
    }

    /// Borrows the package-owned AST view used by native CEMT evaluation.
    ///
    /// This is deliberately a view over the artifact owner: it never encodes,
    /// serializes, reparses, or constructs a generic intermediary value.
    pub fn cemt_evaluator_view(&self) -> Option<CemtEvaluatorValueRef<'_>> {
        match &self.body {
            TransformArtifactBody::Lifecycle(stream) => match stream.as_ref() {
                LoadedInputAstStream::JsonDocument(document) => {
                    document.root.as_ref().map(CemtEvaluatorValueRef::Json)
                }
                _ => None,
            },
            TransformArtifactBody::CemTree(stream) => {
                Some(CemtTreeSubjectRef::new(stream.as_ref()).evaluator_view())
            }
            TransformArtifactBody::MaterializedCemtTree(tree) => Some(tree.evaluator_view()),
            TransformArtifactBody::Collection(collection) => Some(CemtEvaluatorValueRef::Record(
                CemtEvaluatorRecordRef::Package {
                    record: collection.as_ref(),
                },
            )),
            TransformArtifactBody::Extension(native) => native
                .as_any()
                .downcast_ref::<CemtTreeArtifact>()
                .map(CemtTreeArtifact::evaluator_view),
            TransformArtifactBody::CemDocument(_)
            | TransformArtifactBody::GenericData(_)
            | TransformArtifactBody::DomProjection(_)
            | TransformArtifactBody::EventStream(_)
            | TransformArtifactBody::XPathResult(_)
            | TransformArtifactBody::Encoded(_) => None,
        }
    }
}

#[derive(Clone)]
pub enum TransformArtifactBody {
    Lifecycle(Arc<LoadedInputAstStream>),
    CemDocument(Arc<CemDocument>),
    GenericData(Arc<GenericDataDocumentAst>),
    CemTree(Arc<CemTreeAstStream>),
    MaterializedCemtTree(Arc<CemtMaterializedTreeArtifact>),
    DomProjection(Arc<CemDocument>),
    EventStream(Arc<NormalizedEventStream>),
    XPathResult(Arc<XPathResultArtifact>),
    Collection(Arc<TransformArtifactCollection>),
    Extension(Arc<dyn TransformNativeArtifact>),
    Encoded(Arc<TransformEncodedArtifact>),
}

impl TransformArtifactBody {
    pub fn representation_id(&self) -> &'static str {
        match self {
            Self::Lifecycle(_) => "cem.lifecycle-ast",
            Self::CemDocument(_) => "cem.document-ast",
            Self::GenericData(_) => "cem.generic-data-ast",
            Self::CemTree(_) => "cem.tree-ast",
            Self::MaterializedCemtTree(_) => CEMT_MATERIALIZED_TREE_REPRESENTATION_ID,
            Self::DomProjection(_) => DOM_PROJECTION_REPRESENTATION_ID,
            Self::EventStream(_) => EVENT_STREAM_REPRESENTATION_ID,
            Self::XPathResult(_) => XPATH_RESULT_REPRESENTATION_ID,
            Self::Collection(_) => "cem.transform-collection",
            Self::Extension(artifact) => artifact.representation_id(),
            Self::Encoded(_) => "cem.encoded",
        }
    }
}

impl fmt::Debug for TransformArtifactBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransformArtifactBody")
            .field("representation_id", &self.representation_id())
            .finish_non_exhaustive()
    }
}

pub trait TransformNativeArtifact: Any + Send + Sync {
    fn representation_id(&self) -> &'static str;
    fn source_map(&self) -> Option<&SourceMapStack>;
    fn as_any(&self) -> &dyn Any;
}

pub trait TransformArtifactExporter: Send + Sync {
    fn id(&self) -> &'static str;
    fn representation_id(&self) -> &'static str;
    fn export(
        &self,
        request: TransformArtifactExportRequest<'_>,
    ) -> Result<Arc<TransformEncodedArtifact>, String>;
}

#[derive(Debug, Clone, Copy)]
pub struct TransformArtifactExportRequest<'a> {
    pub body: &'a TransformArtifactBody,
    pub target: &'a FormatIdentity,
    pub source_map: Option<&'a SourceMapStack>,
    pub output_spans: &'a [OutputSpan],
}

#[derive(Clone, Default)]
pub struct TransformArtifactExporterRegistry {
    exporters: BTreeMap<&'static str, Arc<dyn TransformArtifactExporter>>,
}

impl TransformArtifactExporterRegistry {
    pub fn with_builtin_exporters() -> Self {
        let mut registry = Self::default();
        registry.register(DomProjectionJsonExporter);
        registry.register(EventStreamJsonExporter);
        registry.register(XPathResultJsonExporter);
        registry
    }

    pub fn register(&mut self, exporter: impl TransformArtifactExporter + 'static) {
        self.exporters
            .insert(exporter.representation_id(), Arc::new(exporter));
    }

    pub fn export(
        &self,
        body: &TransformArtifactBody,
        target: &FormatIdentity,
    ) -> Result<Arc<TransformEncodedArtifact>, String> {
        self.export_with_metadata(TransformArtifactExportRequest {
            body,
            target,
            source_map: None,
            output_spans: &[],
        })
    }

    pub fn export_with_metadata(
        &self,
        request: TransformArtifactExportRequest<'_>,
    ) -> Result<Arc<TransformEncodedArtifact>, String> {
        let representation_id = request.body.representation_id();
        let exporter = self.exporters.get(representation_id).ok_or_else(|| {
            format!(
                "no transform artifact exporter is registered for native representation `{representation_id}`"
            )
        })?;
        exporter.export(request).map_err(|message| {
            format!(
                "transform artifact exporter `{}` failed for `{representation_id}`: {message}",
                exporter.id()
            )
        })
    }
}

fn projection_json_target_matches(
    target: &FormatIdentity,
    vendor_content_type: &str,
    schema_uri: &str,
) -> bool {
    let schema_matches = target.schema.as_deref() == Some(schema_uri);
    match target.content_type.as_deref().map(content_type_essence) {
        Some(content_type) if content_type == vendor_content_type => {
            target.schema.is_none() || schema_matches
        }
        Some(content_type) if content_type == JSON_CONTENT_TYPE => schema_matches,
        _ => false,
    }
}

fn projection_json_target_error(
    exporter_id: &str,
    target: &FormatIdentity,
    vendor_content_type: &str,
    schema_uri: &str,
) -> String {
    format!(
        "{exporter_id} target must use `{vendor_content_type}`, or `application/json` with schema `{schema_uri}`; got content type `{}` and schema `{}`",
        target.content_type.as_deref().unwrap_or("none"),
        target.schema.as_deref().unwrap_or("none")
    )
}

#[derive(Debug, Clone, Default)]
struct DomProjectionJsonExporter;

impl TransformArtifactExporter for DomProjectionJsonExporter {
    fn id(&self) -> &'static str {
        "cem.dom-projection-json"
    }

    fn representation_id(&self) -> &'static str {
        DOM_PROJECTION_REPRESENTATION_ID
    }

    fn export(
        &self,
        request: TransformArtifactExportRequest<'_>,
    ) -> Result<Arc<TransformEncodedArtifact>, String> {
        if !projection_json_target_matches(
            request.target,
            CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
            CEM_DOM_PROJECTION_SCHEMA_URI,
        ) {
            return Err(projection_json_target_error(
                self.id(),
                request.target,
                CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ));
        }
        let TransformArtifactBody::DomProjection(document) = request.body else {
            return Err(format!(
                "expected native `{DOM_PROJECTION_REPRESENTATION_ID}` body, got `{}`",
                request.body.representation_id()
            ));
        };
        let bytes = serde_json::to_vec(&DomJsonProjectionRef::new(document.as_ref()))
            .map_err(|error| format!("DOM projection JSON encoding failed: {error}"))?;
        TransformEncodedArtifact::new(request.target.clone(), TransformEncoding::Json, bytes)
            .map(Arc::new)
            .map_err(|error| error.to_string())
    }
}

#[derive(Debug, Clone, Default)]
struct EventStreamJsonExporter;

impl TransformArtifactExporter for EventStreamJsonExporter {
    fn id(&self) -> &'static str {
        "cem.event-stream-json"
    }

    fn representation_id(&self) -> &'static str {
        EVENT_STREAM_REPRESENTATION_ID
    }

    fn export(
        &self,
        request: TransformArtifactExportRequest<'_>,
    ) -> Result<Arc<TransformEncodedArtifact>, String> {
        if !projection_json_target_matches(
            request.target,
            CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
            CEM_EVENTS_PROJECTION_SCHEMA_URI,
        ) {
            return Err(projection_json_target_error(
                self.id(),
                request.target,
                CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
                CEM_EVENTS_PROJECTION_SCHEMA_URI,
            ));
        }
        let TransformArtifactBody::EventStream(stream) = request.body else {
            return Err(format!(
                "expected native `{EVENT_STREAM_REPRESENTATION_ID}` body, got `{}`",
                request.body.representation_id()
            ));
        };
        let bytes = serde_json::to_vec(&EventsJsonProjectionRef::new(stream.as_ref()))
            .map_err(|error| format!("event stream JSON encoding failed: {error}"))?;
        TransformEncodedArtifact::new(request.target.clone(), TransformEncoding::Json, bytes)
            .map(Arc::new)
            .map_err(|error| error.to_string())
    }
}

#[derive(Debug, Clone, Default)]
struct XPathResultJsonExporter;

impl TransformArtifactExporter for XPathResultJsonExporter {
    fn id(&self) -> &'static str {
        "cem.xpath-result-json"
    }

    fn representation_id(&self) -> &'static str {
        XPATH_RESULT_REPRESENTATION_ID
    }

    fn export(
        &self,
        request: TransformArtifactExportRequest<'_>,
    ) -> Result<Arc<TransformEncodedArtifact>, String> {
        if !projection_json_target_matches(
            request.target,
            XPATH_RESULT_CONTENT_TYPE,
            XPATH_SCHEMA_URI,
        ) {
            return Err(projection_json_target_error(
                self.id(),
                request.target,
                XPATH_RESULT_CONTENT_TYPE,
                XPATH_SCHEMA_URI,
            ));
        }
        let TransformArtifactBody::XPathResult(result) = request.body else {
            return Err(format!(
                "expected native `{XPATH_RESULT_REPRESENTATION_ID}` body, got `{}`",
                request.body.representation_id()
            ));
        };
        let bytes = serde_json::to_vec(result.as_ref())
            .map_err(|error| format!("XPath result JSON encoding failed: {error}"))?;
        TransformEncodedArtifact::new(request.target.clone(), TransformEncoding::Json, bytes)
            .map(Arc::new)
            .map_err(|error| error.to_string())
    }
}

impl fmt::Debug for TransformArtifactExporterRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TransformArtifactExporterRegistry")
            .field(
                "representations",
                &self.exporters.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformArtifactCollectionMode {
    Collect,
    GroupBy,
    MatchBy,
    Zip,
}

impl TransformArtifactCollectionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Collect => "collect",
            Self::GroupBy => "group-by",
            Self::MatchBy => "match-by",
            Self::Zip => "zip",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransformArtifactCollection {
    pub mode: TransformArtifactCollectionMode,
    pub bindings: BTreeMap<String, String>,
    pub items: Vec<TransformArtifactCollectionItem>,
}

#[derive(Debug, Clone)]
pub struct TransformArtifactCollectionItem {
    pub input_name: String,
    pub destination: Option<String>,
    pub target: Option<FormatIdentity>,
    pub bindings: BTreeMap<String, String>,
    pub artifact: Arc<TransformDataArtifact>,
    pub source_map: Option<SourceMapStack>,
    pub output_spans: Vec<OutputSpan>,
}

impl CemtEvaluatorRecordView for TransformArtifactCollection {
    fn field_names(&self) -> &'static [&'static str] {
        &["kind", "mode", "count", "bindings", "items"]
    }

    fn field<'a>(&'a self, name: &str) -> Option<CemtEvaluatorValueRef<'a>> {
        match name {
            "kind" => Some(CemtEvaluatorValueRef::String("collection")),
            "mode" => Some(CemtEvaluatorValueRef::String(self.mode.as_str())),
            "count" => Some(CemtEvaluatorValueRef::Number(
                CemtEvaluatorNumber::unsigned_integer(
                    u64::try_from(self.items.len()).unwrap_or(u64::MAX),
                ),
            )),
            "bindings" => Some(CemtEvaluatorValueRef::StringMap(&self.bindings)),
            "items" => Some(CemtEvaluatorValueRef::Sequence(
                CemtEvaluatorSequenceRef::Package { sequence: self },
            )),
            _ => None,
        }
    }
}

impl CemtEvaluatorSequenceView for TransformArtifactCollection {
    fn len(&self) -> usize {
        self.items.len()
    }

    fn item<'a>(&'a self, index: usize) -> Option<CemtEvaluatorValueRef<'a>> {
        self.items.get(index).map(|item| {
            CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::Package { record: item })
        })
    }
}

impl CemtEvaluatorRecordView for TransformArtifactCollectionItem {
    fn field_names(&self) -> &'static [&'static str] {
        &[
            "inputName",
            "artifactId",
            "uri",
            "destination",
            "target",
            "bindings",
            "artifact",
            "sourceMap",
            "outputSpans",
        ]
    }

    fn field<'a>(&'a self, name: &str) -> Option<CemtEvaluatorValueRef<'a>> {
        match name {
            "inputName" => Some(CemtEvaluatorValueRef::String(&self.input_name)),
            "artifactId" => Some(CemtEvaluatorValueRef::String(&self.artifact.artifact_id)),
            "uri" => Some(match self.artifact.uri.as_deref() {
                Some(uri) => CemtEvaluatorValueRef::String(uri),
                None => CemtEvaluatorValueRef::Null,
            }),
            "destination" => Some(match self.destination.as_deref() {
                Some(destination) => CemtEvaluatorValueRef::String(destination),
                None => CemtEvaluatorValueRef::Null,
            }),
            "target" => Some(match self.target.as_ref() {
                Some(target) => CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::Package {
                    record: target,
                }),
                None => CemtEvaluatorValueRef::Null,
            }),
            "bindings" => Some(CemtEvaluatorValueRef::StringMap(&self.bindings)),
            "artifact" => self.artifact.cemt_evaluator_view(),
            "sourceMap" => Some(match self.source_map.as_ref() {
                Some(source_map) => CemtEvaluatorValueRef::SourceMap(source_map),
                None => CemtEvaluatorValueRef::Null,
            }),
            "outputSpans" => Some(CemtEvaluatorValueRef::Sequence(
                CemtEvaluatorSequenceRef::OutputSpans {
                    output_spans: &self.output_spans,
                },
            )),
            _ => None,
        }
    }
}

impl CemtEvaluatorRecordView for FormatIdentity {
    fn field_names(&self) -> &'static [&'static str] {
        &[
            "contentType",
            "schema",
            "defaultNamespace",
            "namespaces",
            "baseUri",
        ]
    }

    fn field<'a>(&'a self, name: &str) -> Option<CemtEvaluatorValueRef<'a>> {
        match name {
            "contentType" => Some(match self.content_type.as_deref() {
                Some(content_type) => CemtEvaluatorValueRef::String(content_type),
                None => CemtEvaluatorValueRef::Null,
            }),
            "schema" => Some(match self.schema.as_deref() {
                Some(schema) => CemtEvaluatorValueRef::String(schema),
                None => CemtEvaluatorValueRef::Null,
            }),
            "defaultNamespace" => Some(match self.default_namespace.as_deref() {
                Some(namespace) => CemtEvaluatorValueRef::String(namespace),
                None => CemtEvaluatorValueRef::Null,
            }),
            "namespaces" => Some(CemtEvaluatorValueRef::StringMap(&self.namespaces)),
            "baseUri" => Some(match self.base_uri.as_deref() {
                Some(base_uri) => CemtEvaluatorValueRef::String(base_uri),
                None => CemtEvaluatorValueRef::Null,
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformEncoding {
    Text,
    Json,
    Binary,
}

#[derive(Debug, Clone)]
pub struct TransformEncodedArtifact {
    pub identity: FormatIdentity,
    pub encoding: TransformEncoding,
    pub bytes: Arc<[u8]>,
}

impl TransformEncodedArtifact {
    pub fn new(
        identity: FormatIdentity,
        encoding: TransformEncoding,
        bytes: impl Into<Arc<[u8]>>,
    ) -> Result<Self, TransformEncodedArtifactError> {
        if encoding == TransformEncoding::Json && !identity_has_json_content_type(&identity) {
            return Err(TransformEncodedArtifactError::JsonIdentityRequired {
                content_type: identity.content_type.clone(),
            });
        }
        if encoding == TransformEncoding::Text && identity_has_json_content_type(&identity) {
            return Err(TransformEncodedArtifactError::JsonEncodingRequired {
                content_type: identity.content_type.clone(),
            });
        }
        Ok(Self {
            identity,
            encoding,
            bytes: bytes.into(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransformEncodedArtifactError {
    JsonIdentityRequired { content_type: Option<String> },
    JsonEncodingRequired { content_type: Option<String> },
    JsonEncodingFailed { message: String },
}

impl fmt::Display for TransformEncodedArtifactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JsonIdentityRequired { content_type } => write!(
                f,
                "JSON encoding requires an explicit JSON or +json content type, got {}",
                content_type.as_deref().unwrap_or("no content type")
            ),
            Self::JsonEncodingRequired { content_type } => write!(
                f,
                "explicit JSON or +json content type {} requires JSON encoding",
                content_type.as_deref().unwrap_or("no content type")
            ),
            Self::JsonEncodingFailed { message } => {
                write!(f, "JSON transform artifact encoding failed: {message}")
            }
        }
    }
}

impl std::error::Error for TransformEncodedArtifactError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransformArtifactAccessError {
    UnsupportedRepresentation {
        expected: &'static str,
        actual: &'static str,
    },
    InvalidUtf8 {
        message: String,
    },
    InvalidJson {
        message: String,
    },
}

impl fmt::Display for TransformArtifactAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedRepresentation { expected, actual } => {
                write!(f, "transform artifact representation `{actual}` is unsupported; expected {expected}")
            }
            Self::InvalidUtf8 { message } => {
                write!(
                    f,
                    "encoded transform artifact is not valid UTF-8: {message}"
                )
            }
            Self::InvalidJson { message } => {
                write!(f, "encoded JSON transform artifact is invalid: {message}")
            }
        }
    }
}

impl std::error::Error for TransformArtifactAccessError {}

fn identity_has_json_content_type(identity: &FormatIdentity) -> bool {
    identity
        .content_type
        .as_deref()
        .is_some_and(|content_type| {
            let essence = content_type_essence(content_type);
            essence == JSON_CONTENT_TYPE || essence.ends_with("+json")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json_document_owner(source: &str) -> Arc<crate::validation::json::JsonDocumentAst> {
        let (document, diagnostics) = crate::validation::json::json_document_ast_from_source_bytes(
            crate::validation::json::JsonSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "memory:materialized-json",
                content_type: Some("application/json"),
            },
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.severity.is_hard_violation()),
            "JSON fixture diagnostics: {diagnostics:?}"
        );
        Arc::new(document.expect("lossless JSON document AST"))
    }

    #[test]
    fn json_data_evaluator_borrows_lossless_values_without_collapsing_members() {
        let owner =
            json_document_owner(r#"{"n":1.00e+2,"escaped":"a\nb","items":[{"id":1}],"n":2}"#);
        let root = owner.root.as_ref().expect("JSON root");
        let value = CemtEvaluatorValue::json(root);

        assert!(std::ptr::eq(value.json_ast().expect("borrowed root"), root));
        assert_eq!(value.kind(), CemtEvaluatorValueKind::Record);
        assert_eq!(value.length().expect("ordered member length"), 4);
        assert_eq!(
            value
                .record_field_names("JSON member names")
                .expect("member names"),
            vec!["n", "escaped", "items", "n"]
        );

        let last_n = value.field("n").expect("last declaration wins lookup");
        assert_eq!(last_n.json_lexeme(), Some("2"));
        assert_eq!(
            last_n.as_number().and_then(|number| number.as_i64()),
            Some(2)
        );
        assert!(last_n.json_source_range().is_some());
        assert!(last_n.json_source_map().is_some());

        let escaped = value.field("escaped").expect("escaped string");
        assert_eq!(escaped.as_str(), Some("a\nb"));
        assert_eq!(escaped.json_lexeme(), Some(r#""a\nb""#));

        let item = value
            .resolve_path("items.0.id")
            .expect("nested array/object path");
        assert_eq!(item.json_lexeme(), Some("1"));
        assert!(matches!(item.json_ast(), Some(JsonValueAst::Number { .. })));
    }

    #[test]
    fn markdown_document_evaluator_view_borrows_events_facts_ranges_and_maps() {
        use crate::validation::markdown::{
            markdown_document_ast_from_source_bytes, MarkdownSourceValidationRequest,
        };

        let source = "# Release\r\n\r\n3. [x] **Ready** with [docs](https://example.test \"Guide\")\r\n\r\n> quoted\r\n\r\n```rust\r\nlet ready = true;\r\n```\r\n\r\n| name | state |\r\n| --- | --- |\r\n| CEM | ready |\r\n\r\n<div>embedded</div>\r\n";
        let (document, diagnostics) =
            markdown_document_ast_from_source_bytes(MarkdownSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "memory:borrowed.md",
                content_type: Some("text/markdown; charset=utf-8; variant=GFM"),
            });
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code == "cem.markdown.embedded_html_rejected"),
            "Markdown fixture diagnostics: {diagnostics:?}"
        );
        let owner = Arc::new(document.expect("Markdown document AST"));
        let subject = MarkdownDocumentCemtSubjectRef::new(owner.as_ref());
        assert!(std::ptr::eq(subject.document(), owner.as_ref()));

        let document = CemtEvaluatorValue::borrowed(subject.evaluator_view());
        assert_eq!(
            document
                .resolve_path("source.parameters.variant")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("GFM")
        );
        assert_eq!(
            document
                .field("lineEnding")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("crlf")
        );
        let variant_facts = document
            .field("variantFacts")
            .expect("variant facts")
            .sequence_values("variant facts")
            .expect("variant fact sequence");
        assert_eq!(variant_facts.len(), 1);
        assert_eq!(
            variant_facts[0]
                .field("kind")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("known-variant")
        );
        let parse_facts = document
            .field("parseFacts")
            .expect("parse facts")
            .sequence_values("parse facts")
            .expect("parse fact sequence");
        assert!(!parse_facts.is_empty());
        assert!(parse_facts[0]
            .field("sourceMap")
            .and_then(|value| value.as_source_map().cloned())
            .is_some());

        let events = document
            .field("events")
            .expect("Markdown events")
            .sequence_values("Markdown events")
            .expect("Markdown event sequence");
        for tag in [
            "heading",
            "ordered-list",
            "strong",
            "link",
            "blockquote",
            "code-block",
            "table",
        ] {
            assert!(
                events.iter().any(|event| {
                    event
                        .field("tag")
                        .and_then(|value| value.as_str().map(str::to_owned))
                        .as_deref()
                        == Some(tag)
                }),
                "missing borrowed Markdown event tag `{tag}`"
            );
        }
        assert!(events.iter().any(|event| {
            event.field("checked").and_then(|value| value.as_bool()) == Some(true)
        }));
        assert!(events.iter().any(|event| {
            event
                .field("orderedStart")
                .and_then(|value| value.as_number())
                .and_then(CemtEvaluatorNumber::as_u64)
                == Some(3)
        }));
        let link = events
            .iter()
            .find(|event| {
                event
                    .field("tag")
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .as_deref()
                    == Some("link")
            })
            .expect("link event");
        assert_eq!(
            link.field("destination")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("https://example.test")
        );
        assert_eq!(
            link.field("title")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("Guide")
        );
        assert!(link
            .field("sourceMap")
            .and_then(|value| value.as_source_map().cloned())
            .is_some());
    }

    #[test]
    fn csv_document_evaluator_view_borrows_rows_fields_lexemes_and_maps() {
        use crate::validation::csv::{
            csv_document_ast_from_source_bytes, CsvSourceValidationRequest,
        };

        let source = "same,same\r\n\"a,b\",\"line\nbreak\"\r\n";
        let (document, diagnostics) =
            csv_document_ast_from_source_bytes(CsvSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "memory:borrowed.csv",
                content_type: Some("text/csv; charset=utf-8; header=present"),
            });
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.severity.is_hard_violation()),
            "CSV fixture diagnostics: {diagnostics:?}"
        );
        let owner = Arc::new(document.expect("CSV document AST"));
        let subject = CsvDocumentCemtSubjectRef::new(owner.as_ref());
        assert!(std::ptr::eq(subject.document(), owner.as_ref()));

        let document = CemtEvaluatorValue::borrowed(subject.evaluator_view());
        assert_eq!(
            document
                .resolve_path("source.parameters.header")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("present")
        );
        assert_eq!(
            document
                .field("lineEnding")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("crlf")
        );
        let rows = document
            .field("rows")
            .expect("borrowed CSV rows")
            .sequence_values("borrowed CSV rows")
            .expect("CSV row sequence");
        assert_eq!(rows.len(), 2);
        let fields = rows[1]
            .field("fields")
            .expect("borrowed CSV fields")
            .sequence_values("borrowed CSV fields")
            .expect("CSV field sequence");
        assert_eq!(fields.len(), 2);
        assert_eq!(
            fields[0]
                .field("value")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("a,b")
        );
        assert_eq!(
            fields[0]
                .field("lexeme")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("\"a,b\"")
        );
        assert_eq!(
            fields[1]
                .field("value")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("line\nbreak")
        );
        assert!(fields[1]
            .field("sourceMap")
            .and_then(|value| value.as_source_map().cloned())
            .is_some());
    }

    #[test]
    fn generic_data_csv_evaluator_view_borrows_table_shape_without_csv_dto() {
        use crate::validation::generic_data::{GenericDataNumberKind, GenericDataSourceAst};

        let range = |byte_offset, byte_length| GenericDataSourceRangeAst {
            byte_offset,
            byte_length,
            line: 1,
            column: u32::try_from(byte_offset + 1).expect("test column"),
            source_map: None,
        };
        let string = |value: &str, offset| GenericDataValueAst::String {
            source_range: range(offset, value.len() as u64),
            value: value.to_owned(),
            lexeme: None,
            style: None,
        };
        let mapping = |index: usize, entries| GenericDataStreamDocumentAst {
            index,
            source_range: range(index as u64 * 20, 20),
            root: Some(GenericDataValueAst::Mapping {
                source_range: range(index as u64 * 20, 20),
                entries,
            }),
        };
        let owner = Arc::new(GenericDataDocumentAst {
            source: GenericDataSourceAst {
                uri: "memory:generic.csv-view".to_owned(),
                content_type: "application/yaml".to_owned(),
                media_type: "application/yaml".to_owned(),
                parameters: BTreeMap::new(),
                byte_length: 40,
            },
            documents: vec![
                mapping(
                    0,
                    vec![
                        GenericDataMappingEntryAst {
                            index: 0,
                            key: string("name", 0),
                            value: string("Ada", 5),
                            source_range: range(0, 8),
                        },
                        GenericDataMappingEntryAst {
                            index: 1,
                            key: string("score", 9),
                            value: GenericDataValueAst::Number {
                                source_range: range(15, 10),
                                lexeme: "1.2300e+4".to_owned(),
                                number_kind: GenericDataNumberKind::Exponent,
                            },
                            source_range: range(9, 16),
                        },
                    ],
                ),
                mapping(
                    1,
                    vec![GenericDataMappingEntryAst {
                        index: 0,
                        key: string("name", 26),
                        value: string("Lin", 31),
                        source_range: range(26, 8),
                    }],
                ),
            ],
            line_ending: Some("lf".to_owned()),
        });
        let subject = GenericDataCsvDocumentCemtSubjectRef::new(owner.as_ref());
        assert!(std::ptr::eq(subject.document(), owner.as_ref()));
        let document = CemtEvaluatorValue::borrowed(subject.evaluator_view());
        assert_eq!(
            document
                .field("header")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("present")
        );
        let rows = document
            .field("rows")
            .expect("generic CSV rows")
            .sequence_values("generic CSV rows")
            .expect("generic CSV row sequence");
        assert_eq!(rows.len(), 3);
        let header = rows[0]
            .field("fields")
            .expect("generic CSV header")
            .sequence_values("generic CSV header")
            .expect("generic CSV header fields");
        assert_eq!(header.len(), 2);
        assert_eq!(
            header[1]
                .field("value")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("score")
        );
        let first = rows[1]
            .field("fields")
            .expect("first generic CSV row")
            .sequence_values("first generic CSV row")
            .expect("first row fields");
        assert_eq!(
            first[1]
                .field("value")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("1.2300e+4")
        );
        let second = rows[2]
            .field("fields")
            .expect("second generic CSV row")
            .sequence_values("second generic CSV row")
            .expect("second row fields");
        assert_eq!(
            second[1]
                .field("value")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("")
        );
        assert_eq!(
            second[1].field("sourceMap").map(|value| value.kind()),
            Some(CemtEvaluatorValueKind::Null)
        );
    }

    #[test]
    fn yaml_document_evaluator_view_borrows_syntax_owner_without_value_projection() {
        use crate::validation::yaml::{
            yaml_document_ast_from_source_bytes, YamlSourceValidationRequest,
        };

        let source =
            "%YAML 1.2\r\n# header\r\n---\r\nroot: &base\r\n  quoted: \"Ada\"\r\nalias: *base\r\n";
        let (document, diagnostics) =
            yaml_document_ast_from_source_bytes(YamlSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "memory:borrowed.yaml",
                content_type: Some("application/yaml; charset=utf-8"),
            });
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.severity.is_hard_violation()),
            "YAML fixture diagnostics: {diagnostics:?}"
        );
        let owner = Arc::new(document.expect("YAML document AST"));
        let subject = YamlDocumentCemtSubjectRef::new(owner.as_ref());
        assert!(std::ptr::eq(subject.document(), owner.as_ref()));

        let document = CemtEvaluatorValue::borrowed(subject.evaluator_view());
        assert_eq!(
            document
                .field("lineEnding")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("crlf")
        );
        let directives = document
            .field("directives")
            .expect("borrowed YAML directives")
            .sequence_values("borrowed YAML directives")
            .expect("YAML directive sequence");
        assert_eq!(directives.len(), 1);
        assert_eq!(
            directives[0]
                .field("value")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("1.2")
        );
        let comments = document
            .field("comments")
            .expect("borrowed YAML comments")
            .sequence_values("borrowed YAML comments")
            .expect("YAML comment sequence");
        assert_eq!(comments.len(), 1);
        assert!(comments[0]
            .field("sourceMap")
            .and_then(|value| value.as_source_map().cloned())
            .is_some());

        let documents = document
            .field("documents")
            .expect("borrowed YAML documents")
            .sequence_values("borrowed YAML documents")
            .expect("YAML document sequence");
        let mapping = documents[0]
            .field("root")
            .and_then(|root| root.field("mapping"))
            .expect("root mapping")
            .sequence_values("root mapping")
            .expect("root mapping entries");
        assert_eq!(mapping.len(), 2);
        let anchored = mapping[0].field("value").expect("anchored mapping value");
        assert_eq!(
            anchored
                .field("anchor")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("base")
        );
        let alias = mapping[1].field("value").expect("alias value");
        assert_eq!(
            alias
                .field("alias")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("base")
        );
    }

    #[test]
    fn generic_data_yaml_evaluator_view_preserves_documents_duplicates_and_exact_numbers() {
        use crate::validation::generic_data::{GenericDataNumberKind, GenericDataSourceAst};

        let range = |byte_offset, byte_length| GenericDataSourceRangeAst {
            byte_offset,
            byte_length,
            line: 1,
            column: u32::try_from(byte_offset + 1).expect("test column"),
            source_map: None,
        };
        let string = |value: &str, offset| GenericDataValueAst::String {
            source_range: range(offset, value.len() as u64),
            value: value.to_owned(),
            lexeme: None,
            style: Some("single-quoted".to_owned()),
        };
        let owner = Arc::new(GenericDataDocumentAst {
            source: GenericDataSourceAst {
                uri: "memory:generic.yaml-view".to_owned(),
                content_type: "application/json".to_owned(),
                media_type: "application/json".to_owned(),
                parameters: BTreeMap::new(),
                byte_length: 32,
            },
            documents: vec![
                GenericDataStreamDocumentAst {
                    index: 0,
                    source_range: range(0, 32),
                    root: Some(GenericDataValueAst::Mapping {
                        source_range: range(0, 32),
                        entries: vec![
                            GenericDataMappingEntryAst {
                                index: 0,
                                key: string("same", 0),
                                value: string("first", 6),
                                source_range: range(0, 11),
                            },
                            GenericDataMappingEntryAst {
                                index: 1,
                                key: string("same", 12),
                                value: GenericDataValueAst::Number {
                                    source_range: range(18, 10),
                                    lexeme: "1.2300e+4".to_owned(),
                                    number_kind: GenericDataNumberKind::Exponent,
                                },
                                source_range: range(12, 16),
                            },
                        ],
                    }),
                },
                GenericDataStreamDocumentAst {
                    index: 1,
                    source_range: range(32, 0),
                    root: None,
                },
            ],
            line_ending: Some("lf".to_owned()),
        });
        let subject = GenericDataYamlDocumentCemtSubjectRef::new(owner.as_ref());
        assert!(std::ptr::eq(subject.document(), owner.as_ref()));

        let document = CemtEvaluatorValue::borrowed(subject.evaluator_view());
        let documents = document
            .field("documents")
            .expect("generic YAML documents")
            .sequence_values("generic YAML documents")
            .expect("generic YAML document sequence");
        assert_eq!(documents.len(), 2);
        let mapping = documents[0]
            .field("root")
            .and_then(|root| root.field("mapping"))
            .expect("generic YAML mapping")
            .sequence_values("generic YAML mapping")
            .expect("generic YAML mapping entries");
        assert_eq!(mapping.len(), 2);
        assert_eq!(
            mapping[0]
                .field("key")
                .and_then(|key| key.field("value"))
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("same")
        );
        assert_eq!(
            mapping[1]
                .field("value")
                .and_then(|value| value.field("value"))
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("1.2300e+4")
        );
        assert_eq!(
            documents[1].field("root").map(|value| value.kind()),
            Some(CemtEvaluatorValueKind::Null)
        );
    }

    #[test]
    fn json_document_evaluator_view_borrows_lossless_owner_without_value_projection() {
        let owner = json_document_owner(r#"{"same": 1, "same": 1.2300e+4}"#);
        let subject = JsonDocumentCemtSubjectRef::new(owner.as_ref());

        assert!(std::ptr::eq(subject.document(), owner.as_ref()));
        let document = subject.evaluator_view();
        let root = document.field("root").expect("JSON document root");
        let members = root
            .field("members")
            .and_then(|members| members.as_sequence().cloned())
            .expect("ordered JSON members");
        assert_eq!(members.len(), 2);

        let first = members.item(0).expect("first duplicate member");
        let second = members.item(1).expect("second duplicate member");
        assert_eq!(
            first.field("name").and_then(|value| value.as_str()),
            Some("same")
        );
        assert_eq!(
            second.field("name").and_then(|value| value.as_str()),
            Some("same")
        );
        assert_eq!(
            first
                .field("index")
                .and_then(|value| value.as_number())
                .and_then(CemtEvaluatorNumber::as_u64),
            Some(0)
        );
        assert_eq!(
            second
                .field("index")
                .and_then(|value| value.as_number())
                .and_then(CemtEvaluatorNumber::as_u64),
            Some(1)
        );
        assert_eq!(
            second
                .field("value")
                .and_then(|value| value.field("lexeme"))
                .and_then(|value| value.as_str()),
            Some("1.2300e+4")
        );

        let name_source_map = second
            .field("nameSourceMap")
            .and_then(CemtEvaluatorValueRef::into_source_map)
            .expect("member name source map");
        assert_eq!(name_source_map.frames.len(), 1);
        assert_eq!(
            second
                .field("sourceRange")
                .and_then(|value| value.field("byteOffset"))
                .and_then(|value| value.as_number())
                .and_then(CemtEvaluatorNumber::as_u64),
            Some(12)
        );
    }

    #[test]
    fn json_schema_document_evaluator_view_borrows_outer_and_nested_owners() {
        use crate::schema::registry::JSON_SCHEMA_CONTENT_TYPE;
        use crate::validation::json::{
            json_document_ast_from_source_bytes, JsonParseFactKind, JsonSourceValidationRequest,
            JsonValueAst,
        };
        use crate::validation::json_schema::{
            JsonSchemaDialectFact, JsonSchemaDialectFactKind, JsonSchemaDocumentAst,
            JsonSchemaDocumentSource, JsonSchemaParseFact,
        };

        let source = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","same":1.2300e+4,"same":true,"properties":{"flag":true}}"#;
        let (json, diagnostics) =
            json_document_ast_from_source_bytes(JsonSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "memory:schema-owner",
                content_type: Some(JSON_SCHEMA_CONTENT_TYPE),
            });
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.severity.is_hard_violation()),
            "JSON Schema fixture diagnostics: {diagnostics:?}"
        );
        let json = json.expect("lossless nested JSON AST");
        let dialect_range = match json.root.as_ref() {
            Some(JsonValueAst::Object { members, .. }) => members[0].value.range(),
            root => panic!("expected JSON Schema object root, received {root:?}"),
        };
        let owner = Arc::new(JsonSchemaDocumentAst {
            source: JsonSchemaDocumentSource {
                uri: "memory:schema-owner".to_owned(),
                content_type: "application/schema+json; charset=utf-8; profile=contract-test"
                    .to_owned(),
                media_type: JSON_SCHEMA_CONTENT_TYPE.to_owned(),
                parameters: BTreeMap::from([
                    ("charset".to_owned(), "utf-8".to_owned()),
                    ("profile".to_owned(), "contract-test".to_owned()),
                ]),
                byte_length: source.len(),
            },
            json,
            parse_facts: vec![JsonSchemaParseFact {
                kind: JsonParseFactKind::DuplicateMemberName,
                diagnostic_code: "cem.json_schema.parse_error".to_owned(),
                diagnostic_severity: "warning".to_owned(),
                fatal: false,
                member_name: Some("same".to_owned()),
                line: Some(1),
                column: Some(75),
                byte_offset: Some(74),
                byte_length: Some(6),
                message: "duplicate JSON object member name `same`".to_owned(),
            }],
            dialect_facts: vec![JsonSchemaDialectFact {
                kind: JsonSchemaDialectFactKind::SupportedDialect,
                dialect: Some("https://json-schema.org/draft/2020-12/schema".to_owned()),
                diagnostic_code: None,
                diagnostic_severity: None,
                fatal: false,
                source_range: Some(dialect_range),
                message: "JSON Schema dialect is supported".to_owned(),
            }],
            dialect: "https://json-schema.org/draft/2020-12/schema".to_owned(),
        });
        let subject = JsonSchemaDocumentCemtSubjectRef::new(owner.as_ref());

        assert!(std::ptr::eq(subject.document(), owner.as_ref()));
        assert!(std::ptr::eq(subject.json_document(), &owner.json));
        assert_eq!(
            subject
                .evaluator_view()
                .resolve_path("source.parameters.charset")
                .and_then(|value| value.as_str()),
            Some("utf-8")
        );

        let document = CemtEvaluatorValue::borrowed(subject.evaluator_view());
        assert_eq!(
            document
                .field("contentType")
                .and_then(|value| { value.as_str().map(str::to_owned) }),
            Some(JSON_SCHEMA_CONTENT_TYPE.to_owned())
        );
        assert_eq!(
            document
                .field("dialect")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("https://json-schema.org/draft/2020-12/schema")
        );

        let source = document.field("source").expect("JSON Schema source");
        assert_eq!(
            source
                .field("uri")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("memory:schema-owner")
        );
        let parameters = source.field("parameters").expect("source parameters");
        assert_eq!(
            parameters
                .field("charset")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("utf-8")
        );
        assert_eq!(
            parameters
                .field("profile")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("contract-test")
        );

        let parse_facts = document
            .field("parseFacts")
            .expect("JSON Schema parse facts")
            .sequence_values("JSON Schema parse facts")
            .expect("parse fact sequence");
        assert_eq!(parse_facts.len(), 1);
        assert_eq!(
            parse_facts[0]
                .field("kind")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("duplicate-member-name")
        );
        assert_eq!(
            parse_facts[0]
                .field("memberName")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("same")
        );
        assert_eq!(
            parse_facts[0]
                .field("sourceRange")
                .and_then(|range| range.field("byteLength"))
                .and_then(|value| value.as_number())
                .and_then(CemtEvaluatorNumber::as_u64),
            Some(6)
        );

        let dialect_facts = document
            .field("dialectFacts")
            .expect("JSON Schema dialect facts")
            .sequence_values("JSON Schema dialect facts")
            .expect("dialect fact sequence");
        assert_eq!(dialect_facts.len(), 1);
        assert_eq!(
            dialect_facts[0]
                .field("kind")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("supported-dialect")
        );
        assert!(dialect_facts[0]
            .field("sourceMap")
            .and_then(|value| value.as_source_map().cloned())
            .is_some());

        let members = document
            .field("json")
            .and_then(|json| json.field("root"))
            .and_then(|root| root.field("members"))
            .expect("lossless nested JSON members")
            .sequence_values("JSON Schema nested JSON members")
            .expect("nested member sequence");
        assert_eq!(members.len(), 4);
        assert_eq!(
            members[1]
                .field("value")
                .and_then(|value| value.field("lexeme"))
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("1.2300e+4")
        );
        assert_eq!(
            members[2]
                .field("value")
                .and_then(|value| value.field("value"))
                .and_then(|value| value.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn generic_data_json_evaluator_view_borrows_owner_and_preserves_json_contract() {
        use crate::source::{ByteRange, SourceId};
        use crate::source_map::{FrameSpan, SourceMapFrame, TransformKind};
        use crate::validation::generic_data::{
            GenericDataMappingEntryAst, GenericDataNumberKind, GenericDataSourceAst,
            GenericDataSourceRangeAst, GenericDataStreamDocumentAst, GenericDataValueAst,
        };

        let range = |byte_offset: u64, byte_length: u64| GenericDataSourceRangeAst {
            byte_offset,
            byte_length,
            line: 1,
            column: u32::try_from(byte_offset + 1).expect("test column"),
            source_map: Some(SourceMapStack {
                frames: vec![SourceMapFrame {
                    source_id: SourceId(41),
                    span: FrameSpan::Single(ByteRange::new(
                        byte_offset,
                        u32::try_from(byte_length).expect("test range length"),
                    )),
                    transform: TransformKind::ContentTypeTransform {
                        content_type: "application/json".to_owned(),
                    },
                }],
            }),
        };
        let string =
            |value: &str, source_range: GenericDataSourceRangeAst| GenericDataValueAst::String {
                source_range,
                value: value.to_owned(),
                lexeme: None,
                style: None,
            };
        let entries = vec![
            GenericDataMappingEntryAst {
                index: 0,
                key: string("same", range(1, 4)),
                value: string("first", range(7, 5)),
                source_range: range(1, 11),
            },
            GenericDataMappingEntryAst {
                index: 1,
                key: string("same", range(14, 4)),
                value: GenericDataValueAst::Number {
                    source_range: range(20, 2),
                    lexeme: "01".to_owned(),
                    number_kind: GenericDataNumberKind::Integer,
                },
                source_range: range(14, 8),
            },
        ];
        let owner = Arc::new(GenericDataDocumentAst {
            source: GenericDataSourceAst {
                uri: "memory:generic-data.json".to_owned(),
                content_type: "application/yaml".to_owned(),
                media_type: "application/yaml".to_owned(),
                parameters: BTreeMap::new(),
                byte_length: 22,
            },
            documents: vec![GenericDataStreamDocumentAst {
                index: 0,
                source_range: range(0, 22),
                root: Some(GenericDataValueAst::Mapping {
                    source_range: range(0, 22),
                    entries,
                }),
            }],
            line_ending: Some("lf".to_owned()),
        });
        let subject = GenericDataJsonDocumentCemtSubjectRef::new(owner.as_ref());

        assert!(std::ptr::eq(subject.document(), owner.as_ref()));
        let document = CemtEvaluatorValue::borrowed(subject.evaluator_view());
        let members = document
            .field("root")
            .and_then(|root| root.field("members"))
            .expect("ordered generic-data JSON members");
        let members = members
            .sequence_values("generic-data JSON evaluator test")
            .expect("generic-data JSON member sequence");
        assert_eq!(members.len(), 2);

        let first = &members[0];
        let second = &members[1];
        assert_eq!(
            first
                .field("name")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("same")
        );
        assert_eq!(
            second
                .field("name")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("same")
        );
        assert_eq!(
            second
                .field("nameLexeme")
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("\"same\"")
        );
        assert_eq!(
            second
                .field("value")
                .and_then(|value| value.field("lexeme"))
                .and_then(|value| value.as_str().map(str::to_owned))
                .as_deref(),
            Some("1")
        );
        let name_source_map = second
            .field("nameSourceMap")
            .and_then(|value| value.as_source_map().cloned())
            .expect("generic-data member name source map");
        assert_eq!(name_source_map.frames.len(), 1);
        assert_eq!(
            second
                .field("sourceRange")
                .and_then(|value| value.field("byteOffset"))
                .and_then(|value| value.as_number())
                .and_then(CemtEvaluatorNumber::as_u64),
            Some(14)
        );
    }

    #[test]
    fn json_document_evaluator_view_has_no_serializer_or_dto_projection() {
        let source = include_str!("transform_artifact.rs");
        let view = source
            .split_once("pub struct JsonDocumentCemtSubjectRef")
            .and_then(|(_, suffix)| suffix.split_once("pub enum CemtEvaluatorValueKind"))
            .map(|(view, _)| view)
            .expect("JSON document evaluator-view source");

        for forbidden in [
            "serde_json",
            "to_cemt_subject",
            "into_cemt_subject",
            "to_json_value",
        ] {
            assert!(
                !view.contains(forbidden),
                "JSON evaluator view must not cross `{forbidden}`"
            );
        }
    }

    #[test]
    fn writer_token_evaluator_view_borrows_typed_ast_fields_and_output_span() {
        let source_map = SourceMapStack {
            frames: vec![crate::source_map::SourceMapFrame {
                source_id: crate::source::SourceId(23),
                span: crate::source_map::FrameSpan::Single(crate::source::ByteRange::new(8, 4)),
                transform: crate::source_map::TransformKind::TemplateTransform {
                    function: "json.format-document".to_owned(),
                },
            }],
        };
        let owner = Arc::new(CemTreeAstStream::new(vec![CemTreeAstNode::WriterToken {
            token_kind: "json.number".to_owned(),
            text: "1.00".to_owned(),
            role: "syntax.number".to_owned(),
            style: Box::new(crate::projection::CemTreeAstWriterTokenStyle {
                color_role: Some("syntax.number".to_owned()),
                ..crate::projection::CemTreeAstWriterTokenStyle::default()
            }),
            metadata: Box::new(crate::projection::CemTreeAstWriterTokenMetadata {
                formatter_profile: Some("compact".to_owned()),
                source_range: Some(crate::projection::CemTreeAstWriterTokenSourceRange {
                    byte_offset: 8,
                    byte_length: 4,
                    line: 1,
                    column: 9,
                }),
                ..crate::projection::CemTreeAstWriterTokenMetadata::default()
            }),
            output_span: Some(OutputSpan {
                output_range: crate::source::ByteRange::new(0, 4),
                origin: source_map.clone(),
            }),
            source: source_map.clone(),
        }]));

        let token = CemtTreeSubjectRef {
            owner: owner.as_ref(),
        }
        .evaluator_view()
        .item(0)
        .expect("writer token evaluator record");
        assert_eq!(
            token.field("kind").and_then(|value| value.as_str()),
            Some("json.number")
        );
        assert_eq!(
            token.field("writerKind").and_then(|value| value.as_str()),
            Some("token")
        );
        assert_eq!(
            token
                .clone()
                .resolve_path("style.colorRole")
                .and_then(|value| value.as_str()),
            Some("syntax.number")
        );
        assert_eq!(
            token
                .clone()
                .resolve_path("value.sourceRange.byteOffset")
                .and_then(|value| value.as_number())
                .and_then(CemtEvaluatorNumber::as_u64),
            Some(8)
        );
        assert_eq!(
            token
                .clone()
                .resolve_path("outputSpan.outputRange.len")
                .and_then(|value| value.as_number())
                .and_then(CemtEvaluatorNumber::as_u64),
            Some(4)
        );
        assert_eq!(
            token
                .resolve_path("outputSpan.origin")
                .and_then(CemtEvaluatorValueRef::into_source_map),
            Some(source_map)
        );
    }

    fn materialized_tree_identity() -> CemtMaterializedTreeIdentity {
        CemtMaterializedTreeIdentity {
            content_type: "application/json".to_owned(),
            schema: "https://cem.dev/ns/data/json/1".to_owned(),
            category: "json-document".to_owned(),
        }
    }

    #[test]
    fn materialized_cemt_tree_artifact_retains_ast_stream_and_provenance() {
        let owner = Arc::new(CemTreeAstStream::new(vec![CemTreeAstNode::Text {
            value: "ready".to_owned(),
            source: SourceMapStack::default(),
        }]));
        let input_source_map = SourceMapStack {
            frames: vec![crate::source_map::SourceMapFrame {
                source_id: crate::source::SourceId(17),
                span: crate::source_map::FrameSpan::Single(crate::source::ByteRange::new(2, 5)),
                transform: crate::source_map::TransformKind::ContentTypeTransform {
                    content_type: "application/json".to_owned(),
                },
            }],
        };
        let output_source_map = SourceMapStack {
            frames: vec![crate::source_map::SourceMapFrame {
                source_id: crate::source::SourceId(17),
                span: crate::source_map::FrameSpan::Single(crate::source::ByteRange::new(2, 5)),
                transform: crate::source_map::TransformKind::TemplateTransform {
                    function: "json.format-document".to_owned(),
                },
            }],
        };
        let output_spans = vec![OutputSpan {
            output_range: crate::source::ByteRange::new(0, 5),
            origin: output_source_map.clone(),
        }];
        let artifact = Arc::new(
            CemtMaterializedTreeArtifact::new(
                materialized_tree_identity(),
                CemtMaterializedTreeInputProvenance {
                    representation_id: "cem.json-document-ast".to_owned(),
                    source_map: Some(input_source_map.clone()),
                },
                CemtMaterializedTreePipeline::Formatted {
                    formatter: CemtMaterializedTreeProducer::formatter(
                        "json.format-document",
                        Some("compact".to_owned()),
                    ),
                },
                owner.clone(),
                Some(output_source_map.clone()),
                output_spans.clone(),
            )
            .expect("typed formatter producer"),
        );

        assert_eq!(artifact.stage(), CemtMaterializedTreeStage::Formatted);
        assert!(Arc::ptr_eq(artifact.owner(), &owner));
        assert_eq!(artifact.identity(), &materialized_tree_identity());
        assert_eq!(
            artifact.input_provenance(),
            &CemtMaterializedTreeInputProvenance {
                representation_id: "cem.json-document-ast".to_owned(),
                source_map: Some(input_source_map),
            }
        );
        assert_eq!(artifact.source_map(), Some(&output_source_map));
        assert_eq!(artifact.output_spans(), output_spans.as_slice());

        let body = TransformArtifactBody::MaterializedCemtTree(artifact.clone());
        let cloned = body.clone();
        let TransformArtifactBody::MaterializedCemtTree(cloned_artifact) = cloned else {
            panic!("expected first-class materialized CEMT tree body");
        };
        assert!(Arc::ptr_eq(&artifact, &cloned_artifact));
        assert_eq!(
            body.representation_id(),
            CEMT_MATERIALIZED_TREE_REPRESENTATION_ID
        );
    }

    #[test]
    fn materialized_cemt_tree_artifact_rejects_stage_producer_mismatch() {
        let result = CemtMaterializedTreeArtifact::new(
            materialized_tree_identity(),
            CemtMaterializedTreeInputProvenance {
                representation_id: "cem.json-document-ast".to_owned(),
                source_map: None,
            },
            CemtMaterializedTreePipeline::Formatted {
                formatter: CemtMaterializedTreeProducer::converter("json.to-cem-tree"),
            },
            Arc::new(CemTreeAstStream::empty()),
            None,
            Vec::new(),
        );

        assert_eq!(
            result.expect_err("converter cannot claim the formatted stage"),
            "materialized CEMT tree stage requires a Formatter producer, but `json.to-cem-tree` is Converter"
        );
    }

    #[test]
    fn colored_materialized_tree_retains_formatted_owner_with_typed_token_overlay() {
        let owner = Arc::new(CemTreeAstStream::new(vec![CemTreeAstNode::WriterToken {
            token_kind: "json.string".to_owned(),
            text: "\"ready\"".to_owned(),
            role: "syntax.string".to_owned(),
            style: Box::new(crate::projection::CemTreeAstWriterTokenStyle {
                color_role: Some("syntax.string".to_owned()),
                ..crate::projection::CemTreeAstWriterTokenStyle::default()
            }),
            metadata: Box::new(crate::projection::CemTreeAstWriterTokenMetadata {
                formatter_profile: Some("compact".to_owned()),
                ..crate::projection::CemTreeAstWriterTokenMetadata::default()
            }),
            output_span: None,
            source: SourceMapStack::default(),
        }]));
        let overlay = CemtMaterializedTreeColorOverlay {
            producer: CemtMaterializedTreeProducer::colorizer(
                "json.color-document",
                Some("html".to_owned()),
            ),
            output: CemtColorOutput::Html,
            tokens: vec![CemtMaterializedWriterTokenColor {
                target: CemtOwnerPath::root(0),
                color_role: "syntax.string".to_owned(),
                style: crate::projection::CemTreeAstWriterTokenStyle {
                    color_role: Some("syntax.string".to_owned()),
                    color_profile: Some("html".to_owned()),
                    color_output: Some("html".to_owned()),
                    html_mode: Some("classes".to_owned()),
                    class_name: Some("cem-color cem-color-syntax-string".to_owned()),
                    color: Some("var(--cem-color-syntax-string)".to_owned()),
                    ..crate::projection::CemTreeAstWriterTokenStyle::default()
                },
            }],
        };
        let artifact = CemtMaterializedTreeArtifact::new_colored(
            materialized_tree_identity(),
            CemtMaterializedTreeInputProvenance {
                representation_id: CEMT_MATERIALIZED_TREE_REPRESENTATION_ID.to_owned(),
                source_map: None,
            },
            CemtMaterializedTreePipeline::Colored {
                formatter: CemtMaterializedTreeProducer::formatter(
                    "json.format-document",
                    Some("compact".to_owned()),
                ),
                colorizer: CemtMaterializedTreeProducer::colorizer(
                    "json.color-document",
                    Some("html".to_owned()),
                ),
            },
            owner.clone(),
            None,
            Vec::new(),
            overlay.clone(),
        )
        .expect("typed materialized color overlay");

        assert_eq!(artifact.stage(), CemtMaterializedTreeStage::Colored);
        assert!(Arc::ptr_eq(artifact.owner(), &owner));
        assert_eq!(artifact.color_overlay(), Some(&overlay));
        assert_eq!(
            artifact.color_overlay().unwrap().tokens[0].target,
            CemtOwnerPath::root(0)
        );
    }

    #[test]
    fn materialized_color_overlay_rejects_non_writer_token_targets() {
        let result = CemtMaterializedTreeArtifact::new_colored(
            materialized_tree_identity(),
            CemtMaterializedTreeInputProvenance {
                representation_id: CEMT_MATERIALIZED_TREE_REPRESENTATION_ID.to_owned(),
                source_map: None,
            },
            CemtMaterializedTreePipeline::Colored {
                formatter: CemtMaterializedTreeProducer::formatter(
                    "json.format-document",
                    Some("compact".to_owned()),
                ),
                colorizer: CemtMaterializedTreeProducer::colorizer(
                    "json.color-document",
                    Some("html".to_owned()),
                ),
            },
            Arc::new(CemTreeAstStream::new(vec![CemTreeAstNode::Text {
                value: "not a writer token".to_owned(),
                source: SourceMapStack::default(),
            }])),
            None,
            Vec::new(),
            CemtMaterializedTreeColorOverlay {
                producer: CemtMaterializedTreeProducer::colorizer(
                    "json.color-document",
                    Some("html".to_owned()),
                ),
                output: CemtColorOutput::Html,
                tokens: vec![CemtMaterializedWriterTokenColor {
                    target: CemtOwnerPath::root(0),
                    color_role: "syntax.string".to_owned(),
                    style: crate::projection::CemTreeAstWriterTokenStyle::default(),
                }],
            },
        );

        assert_eq!(
            result.expect_err("color overlay target must be a writer token"),
            "materialized CEMT color overlay target root[0] is not a writer-token node"
        );
    }

    #[test]
    fn materialized_cemt_tree_contract_has_no_serialized_value_boundary() {
        let source = include_str!("transform_artifact.rs");
        let contract = source
            .split_once("pub enum CemtMaterializedTreeStage")
            .and_then(|(_, suffix)| suffix.split_once("pub struct CemtTreeSubjectRef"))
            .map(|(contract, _)| contract)
            .expect("materialized CEMT tree contract source");

        for forbidden in [
            "serde_json",
            "Value",
            "to_cemt_subject",
            "into_cemt_subject",
            "try_from_cemt_subject",
        ] {
            assert!(
                !contract.contains(forbidden),
                "materialized CEMT tree contract must not cross `{forbidden}`"
            );
        }
    }

    #[test]
    fn raw_cemt_tree_artifact_retains_owner_identity_and_lazy_nodes() {
        let owner = Arc::new(CemTreeAstStream::new(vec![
            crate::projection::CemTreeAstNode::Text {
                value: "ready".to_owned(),
                source: SourceMapStack::default(),
            },
        ]));
        let source_map = SourceMapStack {
            frames: vec![crate::source_map::SourceMapFrame {
                source_id: crate::source::SourceId(7),
                span: crate::source_map::FrameSpan::Single(crate::source::ByteRange::new(4, 5)),
                transform: crate::source_map::TransformKind::TemplateTransform {
                    function: "cem.format-tree".to_owned(),
                },
            }],
        };
        let artifact = CemtTreeArtifact::raw(owner.clone(), Some(source_map.clone()));

        assert_eq!(artifact.stage(), CemtTreeArtifactStage::Raw);
        assert!(Arc::ptr_eq(artifact.owner(), &owner));
        assert!(std::ptr::eq(artifact.subject().nodes(), owner.as_nodes()));
        assert_eq!(artifact.source_map(), Some(&source_map));
        assert_eq!(artifact.representation_id(), CEMT_TREE_REPRESENTATION_ID);
    }

    #[test]
    fn raw_cemt_evaluator_view_reads_nested_records_without_value_materialization() {
        let text_source = SourceMapStack {
            frames: vec![crate::source_map::SourceMapFrame {
                source_id: crate::source::SourceId(11),
                span: crate::source_map::FrameSpan::Single(crate::source::ByteRange::new(8, 5)),
                transform: crate::source_map::TransformKind::CemAstBuilder,
            }],
        };
        let owner = Arc::new(CemTreeAstStream::new(vec![CemTreeAstNode::Element {
            name: "article".to_owned(),
            attributes: vec![CemTreeAstAttribute {
                name: "id".to_owned(),
                value: Some("intro".to_owned()),
                source: SourceMapStack::default(),
            }],
            children: vec![CemTreeAstNode::Text {
                value: "ready".to_owned(),
                source: text_source.clone(),
            }],
            source: SourceMapStack::default(),
        }]));
        let artifact = CemtTreeArtifact::raw(owner.clone(), None);

        let subject = artifact.subject().evaluator_view();
        assert_eq!(subject.kind(), CemtEvaluatorValueKind::Sequence);
        assert_eq!(
            subject.as_sequence().map(CemtEvaluatorSequenceRef::len),
            Some(1)
        );

        let root = subject.item(0).expect("root record view");
        let root_record = root.as_record().expect("root is a record");
        assert_eq!(root_record.owner_path(), Some(&CemtOwnerPath::root(0)));
        assert_eq!(
            root.field("kind").and_then(|value| value.as_str()),
            Some("element")
        );
        assert_eq!(
            root.field("name").and_then(|value| value.as_str()),
            Some("article")
        );

        let attribute = root
            .field("attributes")
            .and_then(|value| value.item(0))
            .expect("attribute record view");
        assert_eq!(
            attribute
                .as_record()
                .and_then(CemtEvaluatorRecordRef::owner_path),
            Some(&CemtOwnerPath::root(0).attribute(0))
        );
        assert_eq!(
            attribute.field("value").and_then(|value| value.as_str()),
            Some("intro")
        );

        let child = root
            .field("children")
            .and_then(|value| value.item(0))
            .expect("child record view");
        assert_eq!(
            child
                .as_record()
                .and_then(CemtEvaluatorRecordRef::owner_path),
            Some(&CemtOwnerPath::root(0).child(0))
        );
        assert_eq!(
            child.field("value").and_then(|value| value.as_str()),
            Some("ready")
        );
        assert_eq!(
            child
                .field("sourceMap")
                .and_then(CemtEvaluatorValueRef::into_source_map),
            Some(text_source.clone())
        );
        assert_eq!(
            subject
                .clone()
                .resolve_path("0.children.0.value")
                .and_then(|value| value.as_str()),
            Some("ready")
        );
        assert_eq!(
            subject.clone().resolve_path("").map(|value| value.kind()),
            Some(CemtEvaluatorValueKind::Sequence)
        );
        assert_eq!(
            subject
                .as_sequence()
                .expect("root sequence")
                .iter()
                .map(|value| value.field("kind").and_then(|kind| kind.as_str()))
                .collect::<Vec<_>>(),
            vec![Some("element")]
        );
        assert!(Arc::ptr_eq(artifact.owner(), &owner));
    }

    #[test]
    fn owned_cemt_evaluator_values_keep_native_records_borrowed() {
        let source_map = SourceMapStack {
            frames: vec![crate::source_map::SourceMapFrame {
                source_id: crate::source::SourceId(14),
                span: crate::source_map::FrameSpan::Single(crate::source::ByteRange::new(2, 3)),
                transform: crate::source_map::TransformKind::CemAstBuilder,
            }],
        };
        let owner = Arc::new(CemTreeAstStream::new(vec![CemTreeAstNode::Text {
            value: "native".to_owned(),
            source: source_map.clone(),
        }]));
        let artifact = CemtTreeArtifact::raw(owner.clone(), None);
        let native = artifact
            .evaluator_view()
            .item(0)
            .expect("native record view");
        let borrowed = CemtEvaluatorValue::borrowed(native);

        assert!(matches!(borrowed, CemtEvaluatorValue::Borrowed(_)));
        assert!(matches!(
            borrowed.native_record().and_then(CemtEvaluatorRecordRef::owner),
            Some(CemtTreeOwnerRef::Node(node)) if std::ptr::eq(node, &owner.as_nodes()[0])
        ));
        assert_eq!(
            borrowed
                .resolve_path("sourceMap")
                .and_then(|value| value.as_source_map().cloned()),
            Some(source_map.clone())
        );

        let values = CemtEvaluatorValue::sequence([
            CemtEvaluatorValue::Null,
            CemtEvaluatorValue::boolean(true),
            CemtEvaluatorValue::string("generated"),
            CemtEvaluatorValue::record([("sourceMap", CemtEvaluatorValue::source_map(source_map))]),
        ]);
        assert_eq!(values.kind(), CemtEvaluatorValueKind::Sequence);
        assert!(matches!(values.item(0), Some(CemtEvaluatorValue::Null)));
        assert_eq!(values.item(1).and_then(|value| value.as_bool()), Some(true));
        assert_eq!(
            values
                .item(2)
                .and_then(|value| value.as_str().map(str::to_owned)),
            Some("generated".to_owned())
        );
        assert_eq!(
            values
                .item(3)
                .and_then(|value| value.field("sourceMap"))
                .map(|value| value.kind()),
            Some(CemtEvaluatorValueKind::SourceMap)
        );
        let integer = CemtEvaluatorValue::unsigned_integer(3);
        assert_eq!(integer.kind(), CemtEvaluatorValueKind::Number);
        assert_eq!(
            integer.as_number().and_then(|value| value.as_u64()),
            Some(3)
        );
        assert!(CemtEvaluatorValue::decimal(f64::NAN).is_none());
    }

    #[test]
    fn typed_cemt_evaluator_bindings_preserve_missing_null_and_sequence_order() {
        let bindings = CemtEvaluatorBindings::from_iter([
            (
                "input",
                CemtEvaluatorValue::record([
                    ("present", CemtEvaluatorValue::Null),
                    (
                        "items",
                        CemtEvaluatorValue::sequence([
                            CemtEvaluatorValue::string("first"),
                            CemtEvaluatorValue::string("second"),
                        ]),
                    ),
                ]),
            ),
            ("label", CemtEvaluatorValue::string("card")),
        ]);

        assert!(bindings.exists("input.present"));
        assert!(!bindings.exists("input.missing"));
        assert!(matches!(
            bindings.resolve_path("input.present"),
            Some(CemtEvaluatorValue::Null)
        ));
        assert_eq!(
            bindings
                .resolve_path("input.items.1")
                .and_then(|value| value.as_str().map(str::to_owned)),
            Some("second".to_owned())
        );
        assert_eq!(
            bindings
                .resolve_path("input.items")
                .expect("items")
                .length(),
            Ok(2)
        );
        assert_eq!(
            bindings.resolve_path("label").expect("label").length(),
            Ok(4)
        );
        assert!(matches!(
            bindings
                .resolve_path("input")
                .expect("input")
                .get("missing"),
            Ok(CemtEvaluatorValue::Null)
        ));
        assert_eq!(
            bindings
                .resolve_path("input.items")
                .expect("items")
                .get("0")
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned)),
            Some("first".to_owned())
        );

        let items = bindings.resolve_path("input.items").expect("items");
        assert_eq!(
            items
                .get_value(&CemtEvaluatorValue::unsigned_integer(1))
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned)),
            Some("second".to_owned())
        );
        assert!(matches!(
            items.get_value(&CemtEvaluatorValue::decimal(1.0).expect("finite decimal")),
            Ok(CemtEvaluatorValue::Null)
        ));
    }

    #[test]
    fn typed_cemt_evaluator_updates_layer_over_native_records() {
        let owner = Arc::new(CemTreeAstStream::new(vec![CemTreeAstNode::Element {
            name: "article".to_owned(),
            attributes: Vec::new(),
            children: vec![CemTreeAstNode::Text {
                value: "before".to_owned(),
                source: SourceMapStack::default(),
            }],
            source: SourceMapStack::default(),
        }]));
        let artifact = CemtTreeArtifact::raw(owner.clone(), None);
        let native = CemtEvaluatorValue::borrowed(
            artifact
                .evaluator_view()
                .item(0)
                .expect("native root record"),
        );

        let updated = native
            .set_path("name", CemtEvaluatorValue::string("card"))
            .expect("set root field")
            .set_path("children.0.value", CemtEvaluatorValue::string("after"))
            .expect("set nested field");

        assert_eq!(
            native
                .resolve_path("name")
                .and_then(|value| value.as_str().map(str::to_owned)),
            Some("article".to_owned())
        );
        assert_eq!(
            updated
                .resolve_path("name")
                .and_then(|value| value.as_str().map(str::to_owned)),
            Some("card".to_owned())
        );
        assert_eq!(
            updated
                .resolve_path("children.0.value")
                .and_then(|value| value.as_str().map(str::to_owned)),
            Some("after".to_owned())
        );
        assert!(matches!(
            updated
                .owned_record()
                .and_then(CemtEvaluatorRecord::native_base)
                .and_then(CemtEvaluatorRecordRef::owner),
            Some(CemtTreeOwnerRef::Node(node)) if std::ptr::eq(node, &owner.as_nodes()[0])
        ));
        let updated_child = updated
            .resolve_path("children.0")
            .expect("updated child record");
        assert!(matches!(
            updated_child
                .owned_record()
                .and_then(CemtEvaluatorRecord::native_base)
                .and_then(CemtEvaluatorRecordRef::owner),
            Some(CemtTreeOwnerRef::Node(node)) if std::ptr::eq(node, &owner.as_nodes()[0].children()[0])
        ));

        let appended = updated
            .resolve_path("children")
            .expect("children")
            .append(CemtEvaluatorValue::string("generated"))
            .expect("append generated child");
        assert_eq!(appended.length(), Ok(2));
        assert_eq!(
            appended
                .item(1)
                .and_then(|value| value.as_str().map(str::to_owned)),
            Some("generated".to_owned())
        );
    }

    #[test]
    fn formatted_cemt_evaluator_view_lazily_merges_owner_and_overlay_records() {
        let operation_source = SourceMapStack {
            frames: vec![crate::source_map::SourceMapFrame {
                source_id: crate::source::SourceId(12),
                span: crate::source_map::FrameSpan::Single(crate::source::ByteRange::new(3, 4)),
                transform: crate::source_map::TransformKind::TemplateTransform {
                    function: "cem.format-tree".to_owned(),
                },
            }],
        };
        let owner = Arc::new(CemTreeAstStream::new(vec![CemTreeAstNode::Element {
            name: "card".to_owned(),
            attributes: vec![CemTreeAstAttribute {
                name: "tone".to_owned(),
                value: Some("info".to_owned()),
                source: SourceMapStack::default(),
            }],
            children: vec![CemTreeAstNode::Text {
                value: "Ready".to_owned(),
                source: SourceMapStack::default(),
            }],
            source: SourceMapStack::default(),
        }]));
        let root_path = CemtOwnerPath::root(0);
        let attribute_path = root_path.attribute(0);
        let child_path = root_path.child(0);
        let fragment = |target, formatter_role: &str, kind| CemtNodeFormatOperation {
            target,
            producer_function: "cem.format-tree".to_owned(),
            formatter_role: formatter_role.to_owned(),
            color_role: Some("syntax.raw".to_owned()),
            kind,
            provenance: CemtOverlayProvenance::Generated {
                function_name: "cem.format-tree".to_owned(),
                source_map: None,
            },
        };
        let overlay = CemtFormattedTreeOverlay {
            envelope: CemtTreeEnvelopeMetadata {
                content_type: "application/cem".to_owned(),
                schema: "https://cem.dev/ns/cem-ml/1".to_owned(),
                category: "cem-tree".to_owned(),
                mode: CemtTreeEnvelopeMode::Document,
                canonical: false,
            },
            producer: CemtOverlayProducer {
                function_name: "cem.format-tree".to_owned(),
                formatter_profile: Some("tabular".to_owned()),
            },
            operations: vec![
                CemtFormatOperation {
                    name: "cem.format-tree".to_owned(),
                    formatter_role: "formatter.boundary".to_owned(),
                    formatter_profile: Some("tabular".to_owned()),
                    color_role: Some("source.gutter".to_owned()),
                    kind: CemtFormatOperationKind::Marker,
                    provenance: CemtOverlayProvenance::SourceMapped(operation_source.clone()),
                },
                CemtFormatOperation {
                    name: "line-ending".to_owned(),
                    formatter_role: "formatter.line-ending".to_owned(),
                    formatter_profile: Some("tabular".to_owned()),
                    color_role: None,
                    kind: CemtFormatOperationKind::Decision {
                        value: "lf".to_owned(),
                    },
                    provenance: CemtOverlayProvenance::Generated {
                        function_name: "cem.format-tree".to_owned(),
                        source_map: None,
                    },
                },
            ],
            retained_node_paths: vec![root_path.clone(), child_path.clone()],
            node_operations: vec![
                fragment(
                    CemtNodeFormatTarget::Owner(root_path.clone()),
                    "formatter.layout",
                    CemtNodeFormatOperationKind::Layout(CemtFormatLayout::Block),
                ),
                fragment(
                    CemtNodeFormatTarget::Owner(root_path.clone()),
                    "formatter.attribute-prefix",
                    CemtNodeFormatOperationKind::BeforeAttributes {
                        fragment: CemtFormatFragment::Whitespace {
                            value: " ".to_owned(),
                        },
                    },
                ),
                fragment(
                    CemtNodeFormatTarget::Owner(attribute_path.clone()),
                    "formatter.attribute-indent",
                    CemtNodeFormatOperationKind::BeforeAttribute {
                        index: 0,
                        fragment: CemtFormatFragment::Whitespace {
                            value: "\t".to_owned(),
                        },
                    },
                ),
                fragment(
                    CemtNodeFormatTarget::Owner(root_path.clone()),
                    "formatter.content-boundary",
                    CemtNodeFormatOperationKind::ContentBoundary {
                        index: 0,
                        fragment: CemtFormatFragment::Raw {
                            value: "|".to_owned(),
                        },
                    },
                ),
                fragment(
                    CemtNodeFormatTarget::ChildGap {
                        parent: root_path.clone(),
                        before_child: 0,
                        ordinal: 0,
                    },
                    "formatter.indent",
                    CemtNodeFormatOperationKind::InsertChild {
                        fragment: CemtFormatFragment::Whitespace {
                            value: "    ".to_owned(),
                        },
                    },
                ),
                fragment(
                    CemtNodeFormatTarget::Owner(root_path.clone()),
                    "formatter.close-indent",
                    CemtNodeFormatOperationKind::BeforeClose {
                        fragment: CemtFormatFragment::Whitespace {
                            value: "\n".to_owned(),
                        },
                    },
                ),
            ],
        };
        let artifact = CemtTreeArtifact::formatted(owner.clone(), None, overlay);
        let view = artifact.evaluator_view();

        assert_eq!(
            view.clone()
                .resolve_path("contentType")
                .and_then(|value| value.as_str()),
            Some("application/cem")
        );
        assert_eq!(
            view.clone()
                .resolve_path("formatterProfile")
                .and_then(|value| value.as_str()),
            Some("tabular")
        );
        assert_eq!(
            view.clone()
                .resolve_path("canonical")
                .and_then(|value| value.as_bool()),
            Some(false)
        );

        let format_marker = view
            .clone()
            .resolve_path("formatNodes.0")
            .expect("format marker view");
        let format_marker_record = format_marker.as_record().expect("format marker record");
        assert_eq!(format_marker_record.format_operation_index(), Some(0));
        assert!(std::ptr::eq(
            format_marker_record
                .format_operation()
                .expect("typed format operation"),
            &artifact.formatted_overlay().expect("overlay").operations[0]
        ));
        assert_eq!(
            format_marker.field("kind").and_then(|value| value.as_str()),
            Some("format-marker")
        );
        assert_eq!(
            format_marker
                .field("sourceMap")
                .and_then(CemtEvaluatorValueRef::into_source_map),
            Some(operation_source.clone())
        );
        assert_eq!(
            view.clone()
                .resolve_path("formatNodes.1.value")
                .and_then(|value| value.as_str()),
            Some("lf")
        );

        let root = view
            .clone()
            .resolve_path("nodes.0")
            .expect("formatted root view");
        assert_eq!(
            root.as_record()
                .and_then(CemtEvaluatorRecordRef::owner_path),
            Some(&root_path)
        );
        assert!(matches!(
            root.as_record().and_then(CemtEvaluatorRecordRef::owner),
            Some(CemtTreeOwnerRef::Node(node)) if std::ptr::eq(node, &owner.as_nodes()[0])
        ));
        let layout = root.field("formatLayout").expect("format layout view");
        assert_eq!(
            layout.field("value").and_then(|value| value.as_str()),
            Some("block")
        );
        assert_eq!(
            layout
                .as_record()
                .and_then(CemtEvaluatorRecordRef::node_format_operation_index),
            Some(0)
        );
        assert_eq!(
            root.field("formatBeforeAttributes")
                .and_then(|value| value.field("value"))
                .and_then(|value| value.as_str()),
            Some(" ")
        );

        let attribute = root
            .field("attributes")
            .and_then(|value| value.item(0))
            .expect("formatted attribute view");
        assert_eq!(
            attribute
                .as_record()
                .and_then(CemtEvaluatorRecordRef::owner_path),
            Some(&attribute_path)
        );
        assert_eq!(
            attribute
                .field("formatBefore")
                .and_then(|value| value.field("value"))
                .and_then(|value| value.as_str()),
            Some("\t")
        );
        assert_eq!(
            root.field("formatContentBoundary")
                .and_then(|value| value.item(0))
                .and_then(|value| value.field("value"))
                .and_then(|value| value.as_str()),
            Some("|")
        );

        let generated_child = root
            .field("children")
            .and_then(|value| value.item(0))
            .expect("generated child fragment");
        let generated_record = generated_child
            .as_record()
            .expect("generated fragment record");
        assert_eq!(generated_record.node_format_operation_index(), Some(4));
        assert!(std::ptr::eq(
            generated_record
                .node_format_operation()
                .expect("typed node format operation"),
            &artifact
                .formatted_overlay()
                .expect("overlay")
                .node_operations[4]
        ));
        assert_eq!(
            generated_child
                .field("formatterOwned")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        let source_child = root
            .field("children")
            .and_then(|value| value.item(1))
            .expect("source child view");
        assert_eq!(
            source_child
                .as_record()
                .and_then(CemtEvaluatorRecordRef::owner_path),
            Some(&child_path)
        );
        assert_eq!(
            root.field("formatBeforeClose")
                .and_then(|value| value.field("value"))
                .and_then(|value| value.as_str()),
            Some("\n")
        );
        assert!(Arc::ptr_eq(artifact.owner(), &owner));
    }

    #[test]
    fn cemt_evaluator_view_contract_does_not_use_serialized_json_storage() {
        let source = include_str!("transform_artifact.rs");
        let view_contract = source
            .split_once("pub enum CemtEvaluatorValueKind")
            .expect("evaluator view contract")
            .1
            .split_once("pub fn to_public_json")
            .expect("evaluator view contract boundary")
            .0;

        assert!(!view_contract.contains("serde_json"));

        let public_projection = source
            .split_once("pub fn to_public_json")
            .expect("explicit public projection")
            .1
            .split_once("pub fn as_bool")
            .expect("explicit public projection boundary")
            .0;
        assert!(public_projection.contains("serde_json"));
    }

    #[test]
    fn transform_collection_evaluator_view_has_no_serializer_or_dto_boundary() {
        let source = include_str!("transform_artifact.rs");
        let collection_view = source
            .split_once("impl CemtEvaluatorRecordView for TransformArtifactCollection")
            .expect("collection evaluator view")
            .1
            .split_once("pub enum TransformEncoding")
            .expect("collection evaluator view boundary")
            .0;

        for required in [
            "CemtEvaluatorSequenceRef::Package",
            "CemtEvaluatorRecordRef::Package",
            "cemt_evaluator_view",
            "CemtEvaluatorSequenceRef::OutputSpans",
        ] {
            assert!(
                collection_view.contains(required),
                "collection evaluator view must retain `{required}`"
            );
        }
        for forbidden in [
            "serde_json",
            "to_public_json",
            "to_json",
            "serialize",
            "deserialize",
        ] {
            assert!(
                !collection_view.contains(forbidden),
                "collection evaluator view must not cross `{forbidden}`"
            );
        }
    }

    #[test]
    fn json_encoded_artifact_requires_explicit_json_identity() {
        let error = TransformEncodedArtifact::new(
            FormatIdentity {
                content_type: Some("application/xml".to_owned()),
                ..FormatIdentity::default()
            },
            TransformEncoding::Json,
            Vec::<u8>::new(),
        )
        .expect_err("non-JSON identity must be rejected");
        assert!(matches!(
            error,
            TransformEncodedArtifactError::JsonIdentityRequired { .. }
        ));

        assert!(TransformEncodedArtifact::new(
            FormatIdentity {
                content_type: Some("application/vnd.cem.dom+json".to_owned()),
                ..FormatIdentity::default()
            },
            TransformEncoding::Json,
            Vec::<u8>::new(),
        )
        .is_ok());

        let error = TransformEncodedArtifact::new(
            FormatIdentity {
                content_type: Some(JSON_CONTENT_TYPE.to_owned()),
                ..FormatIdentity::default()
            },
            TransformEncoding::Text,
            b"{}".to_vec(),
        )
        .expect_err("JSON identity must not be labeled as generic text");
        assert!(matches!(
            error,
            TransformEncodedArtifactError::JsonEncodingRequired { .. }
        ));
    }

    fn projection_test_document() -> Arc<CemDocument> {
        use crate::events::cem::CemEventNormalizer;
        use crate::parser::builder::CemAstBuilder;
        use crate::source::BytesSource;
        use crate::tokenizer::cem::CemTokenizer;

        let source = BytesSource::new(SourceId(41), br#"{main @id=root |hello}"#.to_vec());
        Arc::new(
            CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(source))).build(),
        )
    }

    fn projection_json_target(content_type: &str, schema: Option<&str>) -> FormatIdentity {
        FormatIdentity {
            content_type: Some(content_type.to_owned()),
            schema: schema.map(str::to_owned),
            ..FormatIdentity::default()
        }
    }

    fn test_xpath_result_artifact() -> Arc<XPathResultArtifact> {
        use crate::schema::registry::{XPATH_RESULT_CONTENT_TYPE, XPATH_SCHEMA_URI};
        use crate::validation::xpath::{
            XPathAtomicValue, XPathEvaluatorIdentity, XPathInvocationHost, XPathResultItem,
            XPathResultSequence, XPathStaticContext, XPATH_GRAMMAR_VERSION,
        };

        Arc::new(XPathResultArtifact {
            content_type: XPATH_RESULT_CONTENT_TYPE.to_owned(),
            schema_uri: XPATH_SCHEMA_URI.to_owned(),
            xpath_version: "3.1".to_owned(),
            grammar_version: XPATH_GRAMMAR_VERSION.to_owned(),
            invocation_host: XPathInvocationHost::StandaloneTransform,
            evaluator: XPathEvaluatorIdentity {
                evaluator_id: "test.xpath".to_owned(),
                evaluator_version: "1.0.0".to_owned(),
            },
            expression_uri: "memory:projection-export.xpath".to_owned(),
            static_context: XPathStaticContext::default(),
            resolver_policy_stamp: "resolver-policy/1;test".to_owned(),
            safety_policy_stamp: "xpath-safety/1;pure".to_owned(),
            expected_result: None,
            sequence: XPathResultSequence {
                sequence_type: "xs:string".to_owned(),
                items: vec![XPathResultItem::Atomic {
                    value: XPathAtomicValue {
                        type_name: "xs:string".to_owned(),
                        lexical_value: "native".to_owned(),
                        namespace_uri: None,
                        local_name: None,
                    },
                    source_map: SourceMapStack::default(),
                }],
            },
            source_map: SourceMapStack::default(),
        })
    }

    #[test]
    fn builtin_projection_exporters_encode_borrowed_native_owners_only_for_registered_json_targets()
    {
        use crate::engine::InputFormat;
        use crate::projection::{events_json_as, NormalizedEventStream};
        use crate::schema::registry::{
            CEM_DOM_JSON_PROJECTION_CONTENT_TYPE, CEM_DOM_PROJECTION_SCHEMA_URI,
            CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE, CEM_EVENTS_PROJECTION_SCHEMA_URI,
            XPATH_RESULT_CONTENT_TYPE, XPATH_SCHEMA_URI,
        };

        let document = projection_test_document();
        let event_stream = Arc::new(NormalizedEventStream::from_source(
            br#"{main @id=root |hello}"#,
            InputFormat::Cem,
        ));
        let xpath_result = test_xpath_result_artifact();
        let context = crate::engine::EngineContext::default();
        let registry = &context.transform_artifact_exporter_registry;

        let cases = [
            (
                TransformArtifactBody::DomProjection(Arc::clone(&document)),
                CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            (
                TransformArtifactBody::EventStream(Arc::clone(&event_stream)),
                CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
                CEM_EVENTS_PROJECTION_SCHEMA_URI,
            ),
            (
                TransformArtifactBody::XPathResult(Arc::clone(&xpath_result)),
                XPATH_RESULT_CONTENT_TYPE,
                XPATH_SCHEMA_URI,
            ),
        ];

        for (body, vendor_content_type, schema) in cases {
            for target in [
                projection_json_target(vendor_content_type, Some(schema)),
                projection_json_target(JSON_CONTENT_TYPE, Some(schema)),
            ] {
                let encoded = registry
                    .export(&body, &target)
                    .unwrap_or_else(|error| panic!("{}: {error}", body.representation_id()));
                assert_eq!(encoded.encoding, TransformEncoding::Json);
                assert_eq!(encoded.identity, target);
                let value: serde_json::Value =
                    serde_json::from_slice(encoded.bytes.as_ref()).expect("exported JSON");
                assert!(!value.is_null(), "{}", body.representation_id());
            }

            for target in [
                projection_json_target(JSON_CONTENT_TYPE, None),
                projection_json_target("application/xml", Some(schema)),
                projection_json_target(vendor_content_type, Some("https://cem.dev/ns/wrong/1")),
            ] {
                let error = registry
                    .export(&body, &target)
                    .expect_err("implicit, non-JSON, and mismatched targets must be rejected");
                assert!(error.contains("target"), "{error}");
            }
        }

        let dom_encoded = registry
            .export(
                &TransformArtifactBody::DomProjection(Arc::clone(&document)),
                &projection_json_target(
                    CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
                    Some(CEM_DOM_PROJECTION_SCHEMA_URI),
                ),
            )
            .expect("DOM projection export");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(dom_encoded.bytes.as_ref())
                .expect("DOM export JSON"),
            crate::projection::dom_json(document.as_ref())
        );

        let events_encoded = registry
            .export(
                &TransformArtifactBody::EventStream(Arc::clone(&event_stream)),
                &projection_json_target(
                    CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
                    Some(CEM_EVENTS_PROJECTION_SCHEMA_URI),
                ),
            )
            .expect("event stream export");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(events_encoded.bytes.as_ref())
                .expect("events export JSON"),
            events_json_as(br#"{main @id=root |hello}"#, InputFormat::Cem)
        );

        let xpath_encoded = registry
            .export(
                &TransformArtifactBody::XPathResult(Arc::clone(&xpath_result)),
                &projection_json_target(XPATH_RESULT_CONTENT_TYPE, Some(XPATH_SCHEMA_URI)),
            )
            .expect("XPath result export");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(xpath_encoded.bytes.as_ref())
                .expect("XPath export JSON"),
            serde_json::to_value(xpath_result.as_ref()).expect("XPath parity JSON")
        );

        let TransformArtifactBody::DomProjection(routed_document) =
            TransformArtifactBody::DomProjection(Arc::clone(&document))
        else {
            unreachable!()
        };
        assert!(Arc::ptr_eq(&routed_document, &document));
        let TransformArtifactBody::EventStream(routed_events) =
            TransformArtifactBody::EventStream(Arc::clone(&event_stream))
        else {
            unreachable!()
        };
        assert!(Arc::ptr_eq(&routed_events, &event_stream));
        let TransformArtifactBody::XPathResult(routed_xpath) =
            TransformArtifactBody::XPathResult(Arc::clone(&xpath_result))
        else {
            unreachable!()
        };
        assert!(Arc::ptr_eq(&routed_xpath, &xpath_result));
    }

    #[derive(Clone)]
    struct ExportMetadataProbe {
        seen: Arc<std::sync::Mutex<Option<(usize, usize, usize)>>>,
    }

    impl TransformArtifactExporter for ExportMetadataProbe {
        fn id(&self) -> &'static str {
            "test.export-metadata-probe"
        }

        fn representation_id(&self) -> &'static str {
            DOM_PROJECTION_REPRESENTATION_ID
        }

        fn export(
            &self,
            request: TransformArtifactExportRequest<'_>,
        ) -> Result<Arc<TransformEncodedArtifact>, String> {
            *self.seen.lock().expect("probe lock") = Some((
                request.body as *const TransformArtifactBody as usize,
                request
                    .source_map
                    .map(|source_map| source_map as *const SourceMapStack as usize)
                    .unwrap_or_default(),
                request.output_spans.as_ptr() as usize,
            ));
            TransformEncodedArtifact::new(
                request.target.clone(),
                TransformEncoding::Json,
                b"{}".to_vec(),
            )
            .map(Arc::new)
            .map_err(|error| error.to_string())
        }
    }

    #[test]
    fn exporter_registry_borrows_body_source_map_and_output_spans_without_reconstruction() {
        let body = TransformArtifactBody::DomProjection(projection_test_document());
        let source_map = SourceMapStack::default();
        let output_spans = vec![OutputSpan {
            output_range: ByteRange::new(7, 3),
            origin: SourceMapStack::default(),
        }];
        let seen = Arc::new(std::sync::Mutex::new(None));
        let mut registry = TransformArtifactExporterRegistry::default();
        registry.register(ExportMetadataProbe {
            seen: Arc::clone(&seen),
        });
        let target = projection_json_target(
            crate::schema::registry::CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
            Some(crate::schema::registry::CEM_DOM_PROJECTION_SCHEMA_URI),
        );

        registry
            .export_with_metadata(TransformArtifactExportRequest {
                body: &body,
                target: &target,
                source_map: Some(&source_map),
                output_spans: &output_spans,
            })
            .expect("probe export");

        assert_eq!(
            *seen.lock().expect("probe lock"),
            Some((
                &body as *const TransformArtifactBody as usize,
                &source_map as *const SourceMapStack as usize,
                output_spans.as_ptr() as usize,
            ))
        );
    }

    #[test]
    fn unregistered_projection_exporter_fails_without_generic_fallback() {
        let body = TransformArtifactBody::DomProjection(projection_test_document());
        let target = projection_json_target(
            crate::schema::registry::CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
            Some(crate::schema::registry::CEM_DOM_PROJECTION_SCHEMA_URI),
        );
        let error = TransformArtifactExporterRegistry::default()
            .export(&body, &target)
            .expect_err("missing registration must fail");
        assert!(error.contains(DOM_PROJECTION_REPRESENTATION_ID));
        assert!(!error.contains("fallback"));
    }

    #[test]
    fn native_projection_exporters_have_no_generic_value_or_compatibility_projection_bridge() {
        let source = include_str!("transform_artifact.rs");
        let exporters = source
            .split_once("struct DomProjectionJsonExporter;")
            .expect("native projection exporters")
            .1
            .split_once("impl fmt::Debug for TransformArtifactExporterRegistry")
            .expect("native projection exporter boundary")
            .0;
        for required in [
            "DomJsonProjectionRef::new(document.as_ref())",
            "EventsJsonProjectionRef::new(stream.as_ref())",
            "serde_json::to_vec(result.as_ref())",
            "projection_json_target_matches",
        ] {
            assert!(
                exporters.contains(required),
                "native exporter must retain `{required}`"
            );
        }
        for forbidden in [
            "serde_json::Value",
            "serde_json::to_value",
            "serde_json::from_value",
            "dom_json(",
            "events_json",
            "to_public_json",
        ] {
            assert!(
                !exporters.contains(forbidden),
                "native exporter must not cross `{forbidden}`"
            );
        }

        let real_source = include_str!("real.rs");
        let export_route = real_source
            .split_once("fn transform_artifact_export_primary")
            .expect("transform export route")
            .1
            .split_once("fn transform_graph_target_is_css")
            .expect("transform export route boundary")
            .0;
        for required in ["export_with_metadata", "source_map", "output_spans"] {
            assert!(
                export_route.contains(required),
                "final exporter route must borrow `{required}`"
            );
        }
        for forbidden in ["to_value", "from_value", "dom_json", "events_json"] {
            assert!(
                !export_route.contains(forbidden),
                "final exporter route must not reconstruct `{forbidden}`"
            );
        }
    }
}
