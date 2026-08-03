use crate::engine::FormatIdentity;
use crate::interpreter::OutputSpan;
use crate::lifecycle::LoadedInputAstStream;
use crate::parser::document::CemDocument;
use crate::projection::{CemTreeAstAttribute, CemTreeAstNode, CemTreeAstStream};
use crate::schema::registry::{content_type_essence, JSON_CONTENT_TYPE};
use crate::source_map::SourceMapStack;
use crate::validation::generic_data::GenericDataDocumentAst;
use crate::validation::xpath::XPathResultArtifact;
use std::any::Any;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

pub const CEMT_TREE_REPRESENTATION_ID: &str = "cem.cemt-tree";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CemtColorOutput {
    Terminal,
    Html,
    Markdown,
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

#[derive(Debug, Clone, Copy)]
pub struct CemtTreeSubjectRef<'a> {
    owner: &'a CemTreeAstStream,
}

impl<'a> CemtTreeSubjectRef<'a> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CemtEvaluatorValueKind {
    Null,
    Boolean,
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
    String(&'a str),
    Sequence(CemtEvaluatorSequenceRef<'a>),
    Record(CemtEvaluatorRecordRef<'a>),
    SourceMap(&'a SourceMapStack),
}

impl<'a> CemtEvaluatorValueRef<'a> {
    pub fn kind(&self) -> CemtEvaluatorValueKind {
        match self {
            Self::Null => CemtEvaluatorValueKind::Null,
            Self::Boolean(_) => CemtEvaluatorValueKind::Boolean,
            Self::String(_) => CemtEvaluatorValueKind::String,
            Self::Sequence(_) => CemtEvaluatorValueKind::Sequence,
            Self::Record(_) => CemtEvaluatorValueKind::Record,
            Self::SourceMap(_) => CemtEvaluatorValueKind::SourceMap,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&'a str> {
        match self {
            Self::String(value) => Some(value),
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

    pub fn as_source_map(&self) -> Option<&'a SourceMapStack> {
        match self {
            Self::SourceMap(value) => Some(value),
            _ => None,
        }
    }

    pub fn field(&self, name: &str) -> Option<Self> {
        self.as_record()?.field(name)
    }

    pub fn item(&self, index: usize) -> Option<Self> {
        self.as_sequence()?.item(index)
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
                Self::Sequence(sequence) => sequence.item(segment.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        Some(self)
    }
}

#[derive(Debug, Clone)]
pub enum CemtEvaluatorSequenceRef<'a> {
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
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn item(&self, index: usize) -> Option<CemtEvaluatorValueRef<'a>> {
        match self {
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
            Self::FormattedTree { .. }
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
            Self::FormattedTree { .. }
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

fn cemt_evaluator_node_field<'a>(
    node: &'a CemTreeAstNode,
    path: &CemtOwnerPath,
    name: &str,
) -> Option<CemtEvaluatorValueRef<'a>> {
    if name == "kind" {
        return Some(CemtEvaluatorValueRef::String(node.kind()));
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

    pub fn explicit_json_bytes(&self) -> Result<&[u8], TransformArtifactAccessError> {
        let TransformArtifactBody::Encoded(encoded) = &self.body else {
            return Err(TransformArtifactAccessError::UnsupportedRepresentation {
                expected: "explicit encoded JSON",
                actual: self.body.representation_id(),
            });
        };
        if encoded.encoding != TransformEncoding::Json
            || !identity_has_json_content_type(&encoded.identity)
        {
            return Err(TransformArtifactAccessError::UnsupportedRepresentation {
                expected: "explicit encoded JSON",
                actual: self.body.representation_id(),
            });
        }
        Ok(encoded.bytes.as_ref())
    }

    pub fn explicit_json_value(&self) -> Result<serde_json::Value, TransformArtifactAccessError> {
        serde_json::from_slice(self.explicit_json_bytes()?).map_err(|error| {
            TransformArtifactAccessError::InvalidJson {
                message: error.to_string(),
            }
        })
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
}

#[derive(Clone)]
pub enum TransformArtifactBody {
    Lifecycle(Arc<LoadedInputAstStream>),
    CemDocument(Arc<CemDocument>),
    GenericData(Arc<GenericDataDocumentAst>),
    CemTree(Arc<CemTreeAstStream>),
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
            Self::XPathResult(_) => "cem.xpath-result",
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
        body: &TransformArtifactBody,
        target: &FormatIdentity,
    ) -> Result<Arc<TransformEncodedArtifact>, String>;
}

#[derive(Clone, Default)]
pub struct TransformArtifactExporterRegistry {
    exporters: BTreeMap<&'static str, Arc<dyn TransformArtifactExporter>>,
}

impl TransformArtifactExporterRegistry {
    pub fn register(&mut self, exporter: impl TransformArtifactExporter + 'static) {
        self.exporters
            .insert(exporter.representation_id(), Arc::new(exporter));
    }

    pub fn export(
        &self,
        body: &TransformArtifactBody,
        target: &FormatIdentity,
    ) -> Result<Arc<TransformEncodedArtifact>, String> {
        let representation_id = body.representation_id();
        let exporter = self.exporters.get(representation_id).ok_or_else(|| {
            format!(
                "no transform artifact exporter is registered for native representation `{representation_id}`"
            )
        })?;
        exporter.export(body, target).map_err(|message| {
            format!(
                "transform artifact exporter `{}` failed for `{representation_id}`: {message}",
                exporter.id()
            )
        })
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
                .and_then(|value| value.as_source_map()),
            Some(&text_source)
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
                .and_then(|value| value.as_source_map()),
            Some(&operation_source)
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
    fn cemt_evaluator_view_contract_does_not_use_json_value_storage() {
        let source = include_str!("transform_artifact.rs");
        let view_contract = source
            .split_once("pub enum CemtEvaluatorValueKind")
            .expect("evaluator view contract")
            .1
            .split_once("pub struct CemtFormattedTreeView")
            .expect("evaluator view contract boundary")
            .0;

        assert!(!view_contract.contains("serde_json"));
        assert!(!view_contract.contains("Value::"));
        assert!(!view_contract.contains("Json"));
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
}
