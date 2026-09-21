//! One node view over imported typed CEM trees, independent of source syntax.
use super::*;
use crate::parser::tree::{CemTreeNode, CemTreeNodeKind, RetainedCemTree};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathNativeNodeHandle {
    CemNode {
        node_id: u32,
    },
    XmlDocument,
    XmlEvent {
        event_index: usize,
    },
    XmlAttribute {
        event_index: usize,
        attribute_index: usize,
    },
}

#[derive(Debug, Clone)]
pub struct XPathNativeNode {
    tree: Arc<RetainedCemTree>,
    id: u32,
    // Legacy source handles are compatibility metadata, never navigation input.
    legacy_handles: Option<Arc<Vec<XPathNativeNodeHandle>>>,
    resolution_context: Option<CemResolutionContextHandle>,
}
impl PartialEq for XPathNativeNode {
    fn eq(&self, other: &Self) -> bool {
        self.document_identity() == other.document_identity() && self.id == other.id
    }
}
impl Eq for XPathNativeNode {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XPathNativeNodeError {
    OwnerIsNotXml,
    InvalidCemTree(String),
    CemNodeMissing {
        node_id: u32,
    },
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
    XmlAttributeIsNamespace {
        event_index: usize,
        attribute_index: usize,
    },
    XmlEntityReferenceUnsupported {
        event_index: usize,
    },
}

impl std::fmt::Display for XPathNativeNodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCemTree(message) => write!(formatter, "{message}"),
            Self::CemNodeMissing { node_id } => write!(formatter, "CEM node `{node_id}` is not a semantic node"),
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
            Self::XmlAttributeIsNamespace {
                event_index,
                attribute_index,
            } => write!(
                formatter,
                "XML attribute `{attribute_index}` on event `{event_index}` declares a namespace, not an XPath attribute"
            ),
            Self::XmlEntityReferenceUnsupported { event_index } => write!(
                formatter,
                "XML entity reference at event `{event_index}` has no supported XPath text value"
            ),
        }
    }
}

impl std::error::Error for XPathNativeNodeError {}

impl XPathNativeNode {
    pub fn cem_document(tree: Arc<RetainedCemTree>) -> Self {
        Self {
            tree,
            id: 0,
            legacy_handles: None,
            resolution_context: None,
        }
    }
    pub fn cem_node(tree: Arc<RetainedCemTree>, id: u32) -> Result<Self, XPathNativeNodeError> {
        let id = tree
            .canonical_id(id)
            .ok_or(XPathNativeNodeError::CemNodeMissing { node_id: id })?;
        Ok(Self {
            tree,
            id,
            legacy_handles: None,
            resolution_context: None,
        })
    }
    pub fn xml_document(owner: Arc<LoadedInputAstStream>) -> Result<Self, XPathNativeNodeError> {
        Self::import_xml(owner, XPathNativeNodeHandle::XmlDocument)
    }
    pub fn xml_event(
        owner: Arc<LoadedInputAstStream>,
        event_index: usize,
    ) -> Result<Self, XPathNativeNodeError> {
        Self::import_xml(owner, XPathNativeNodeHandle::XmlEvent { event_index })
    }
    pub fn xml_attribute(
        owner: Arc<LoadedInputAstStream>,
        event_index: usize,
        attribute_index: usize,
    ) -> Result<Self, XPathNativeNodeError> {
        Self::import_xml(
            owner,
            XPathNativeNodeHandle::XmlAttribute {
                event_index,
                attribute_index,
            },
        )
    }
    pub(super) fn import_xml(
        owner: Arc<LoadedInputAstStream>,
        selection: XPathNativeNodeHandle,
    ) -> Result<Self, XPathNativeNodeError> {
        let imported = crate::import::retain_xml(owner).map_err(|error| {
            if error == "owner-is-not-xml" {
                XPathNativeNodeError::OwnerIsNotXml
            } else {
                XPathNativeNodeError::InvalidCemTree(error)
            }
        })?;
        let raw_id = match selection {
            XPathNativeNodeHandle::XmlDocument => 0,
            XPathNativeNodeHandle::XmlEvent { event_index } => imported
                .event_nodes
                .get(event_index)
                .ok_or(XPathNativeNodeError::XmlEventMissing { event_index })?
                .ok_or(XPathNativeNodeError::XmlEventIsNotNode { event_index })?,
            XPathNativeNodeHandle::XmlAttribute {
                event_index,
                attribute_index,
            } => *imported
                .attribute_nodes
                .get(event_index)
                .ok_or(XPathNativeNodeError::XmlEventMissing { event_index })?
                .get(attribute_index)
                .ok_or(XPathNativeNodeError::XmlAttributeMissing {
                    event_index,
                    attribute_index,
                })?,
            _ => unreachable!(),
        };
        let id = imported.tree.canonical_id(raw_id).ok_or(match selection {
            XPathNativeNodeHandle::XmlAttribute {
                event_index,
                attribute_index,
            } => XPathNativeNodeError::XmlAttributeIsNamespace {
                event_index,
                attribute_index,
            },
            XPathNativeNodeHandle::XmlEvent { event_index } => {
                XPathNativeNodeError::XmlEventIsNotNode { event_index }
            }
            _ => XPathNativeNodeError::CemNodeMissing { node_id: raw_id },
        })?;
        let mut handles: Vec<_> = (0..imported.tree.ast().nodes.len())
            .map(|id| XPathNativeNodeHandle::CemNode { node_id: id as u32 })
            .collect();
        handles[0] = XPathNativeNodeHandle::XmlDocument;
        for (event_index, raw) in imported.event_nodes.iter().enumerate() {
            if let Some(id) = raw.and_then(|id| imported.tree.canonical_id(id)) {
                if matches!(handles[id as usize], XPathNativeNodeHandle::CemNode { .. }) {
                    handles[id as usize] = XPathNativeNodeHandle::XmlEvent { event_index };
                }
            }
            for (attribute_index, &id) in imported.attribute_nodes[event_index].iter().enumerate() {
                handles[id as usize] = XPathNativeNodeHandle::XmlAttribute {
                    event_index,
                    attribute_index,
                };
            }
        }
        Ok(Self {
            tree: imported.tree,
            id,
            legacy_handles: Some(Arc::new(handles)),
            resolution_context: None,
        })
    }
    pub fn owner(&self) -> &Arc<RetainedCemTree> {
        &self.tree
    }
    pub fn base_uri(&self) -> Option<&str> {
        self.tree.node_base_uri(self.id)
    }
    pub fn source_owner(&self) -> Option<Arc<LoadedInputAstStream>> {
        self.tree
            .native_owner()
            .and_then(|owner| Arc::downcast::<LoadedInputAstStream>(owner.clone()).ok())
    }
    pub fn handle(&self) -> XPathNativeNodeHandle {
        self.legacy_handles.as_ref().map_or(
            XPathNativeNodeHandle::CemNode { node_id: self.id },
            |handles| handles[self.id as usize],
        )
    }
    pub fn document_identity(&self) -> (bool, usize) {
        if self.legacy_handles.is_some() {
            if let Some(owner) = self.tree.native_owner() {
                return (true, Arc::as_ptr(owner) as *const () as usize);
            }
        }
        (false, Arc::as_ptr(&self.tree) as usize)
    }
    pub fn identity(&self) -> String {
        format!("{:?}:{}", self.document_identity(), self.id)
    }
    pub fn with_resolution_context(mut self, context: CemResolutionContextHandle) -> Self {
        self.resolution_context = Some(context);
        self
    }
    pub fn resolution_context(&self) -> Option<&CemResolutionContextHandle> {
        self.resolution_context.as_ref()
    }
    pub(super) fn data(&self) -> &CemTreeNode {
        self.tree.node(self.id).expect("validated semantic node")
    }
    pub fn source_map(&self) -> SourceMapStack {
        self.data().source.clone()
    }
    pub fn source_key(&self) -> Option<String> {
        self.tree.source_key(self.id)
    }
    pub fn source_line_number(&self) -> Option<u32> {
        self.tree.source_line_number(self.id)
    }
    pub(super) fn at(&self, id: u32) -> Self {
        Self { id, ..self.clone() }
    }
    pub(super) fn document_root(&self) -> Self {
        self.at(0)
    }
    pub fn child_nodes(&self) -> Vec<Self> {
        self.child_nodes_iter().collect()
    }
    /// Iterate the semantic children without allocating a second node list.
    pub fn child_nodes_iter(&self) -> impl Iterator<Item = Self> + '_ {
        self.data().children.iter().map(|&id| self.at(id))
    }
    pub fn attribute_nodes(&self) -> Vec<Self> {
        self.data()
            .attributes
            .iter()
            .map(|&id| self.at(id))
            .collect()
    }
    pub fn parent_node(&self) -> Option<Self> {
        self.data().parent.map(|id| self.at(id))
    }
    pub(super) fn document_order_key(&self) -> (usize, usize, usize) {
        (self.data().order, 0, 0)
    }
    pub fn string_value(&self) -> String {
        if !matches!(
            self.data().kind,
            CemTreeNodeKind::Document | CemTreeNodeKind::Element
        ) {
            return self.data().value.clone();
        }
        self.descendant_nodes()
            .iter()
            .filter(|node| node.data().kind == CemTreeNodeKind::Text)
            .map(|node| node.data().value.as_str())
            .collect()
    }
    /// Borrow string-value fragments so consumers can bound extraction before
    /// allocation while retaining this node's semantic projection.
    pub fn text_fragments(&self) -> crate::parser::tree::CemTreeTextFragments<'_> {
        self.tree.text_fragments(self.id).expect("validated semantic node")
    }
    pub fn result_node_kind(&self) -> XPathResultNodeKind {
        match self.data().kind {
            CemTreeNodeKind::Document => XPathResultNodeKind::Document,
            CemTreeNodeKind::Element => XPathResultNodeKind::Element,
            CemTreeNodeKind::Attribute => XPathResultNodeKind::Attribute,
            CemTreeNodeKind::Text => XPathResultNodeKind::Text,
            CemTreeNodeKind::Comment => XPathResultNodeKind::Comment,
            CemTreeNodeKind::ProcessingInstruction => XPathResultNodeKind::ProcessingInstruction,
        }
    }
    pub(super) fn source_range(&self) -> XPathSourceRange {
        let r = self.data().range;
        XPathSourceRange::new(r.line, r.column, r.offset, r.length)
    }
    pub(super) fn node_id(&self) -> String {
        match self.handle() {
            XPathNativeNodeHandle::CemNode { node_id } => format!("cem:node:{node_id}"),
            XPathNativeNodeHandle::XmlDocument => "xml:document".into(),
            XPathNativeNodeHandle::XmlEvent { event_index } => format!("xml:event:{event_index}"),
            XPathNativeNodeHandle::XmlAttribute {
                event_index,
                attribute_index,
            } => format!("xml:event:{event_index}:attribute:{attribute_index}"),
        }
    }
    pub(super) fn expanded_name(&self) -> Option<String> {
        self.data().name.as_ref().map(|name| {
            if name.namespace_uri.is_empty() {
                name.local_name.clone()
            } else {
                format!("{{{}}}{}", name.namespace_uri, name.local_name)
            }
        })
    }
    pub fn local_name(&self) -> &str {
        self.data().name.as_ref().map_or("", |n| &n.local_name)
    }
    pub fn namespace_uri(&self) -> &str {
        self.data().name.as_ref().map_or("", |n| &n.namespace_uri)
    }
    pub fn semantic_value(&self) -> &str {
        &self.data().value
    }
    pub(super) fn descendant_nodes(&self) -> Vec<Self> {
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

    pub(super) fn ancestor_nodes(&self) -> Vec<Self> {
        let mut ancestors = Vec::new();
        let mut current = self.parent_node();
        while let Some(node) = current {
            current = node.parent_node();
            ancestors.push(node);
        }
        ancestors
    }

    pub(super) fn following_sibling_nodes(&self) -> Vec<Self> {
        if matches!(
            self.result_node_kind(),
            XPathResultNodeKind::Document | XPathResultNodeKind::Attribute
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

    pub(super) fn preceding_sibling_nodes(&self) -> Vec<Self> {
        if matches!(
            self.result_node_kind(),
            XPathResultNodeKind::Document | XPathResultNodeKind::Attribute
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

    pub(super) fn is_ancestor_of(&self, other: &Self) -> bool {
        let mut current = other.parent_node();
        while let Some(node) = current {
            if node == *self {
                return true;
            }
            current = node.parent_node();
        }
        false
    }

    pub(super) fn following_nodes(&self) -> Vec<Self> {
        let context_order = self.document_order_key();
        self.document_root()
            .descendant_nodes()
            .into_iter()
            .filter(|candidate| {
                candidate.document_order_key() > context_order && !self.is_ancestor_of(candidate)
            })
            .collect()
    }

    pub(super) fn preceding_nodes(&self) -> Vec<Self> {
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

    pub(super) fn matches_node_test(&self, node_test: &XPathNodeTest) -> bool {
        match node_test {
            XPathNodeTest::Name(name_test) => {
                if !matches!(
                    self.result_node_kind(),
                    XPathResultNodeKind::Element | XPathResultNodeKind::Attribute
                ) {
                    return false;
                }
                let Some(name) = &self.data().name else {
                    return false;
                };
                let local_name = name.local_name.as_str();
                let node_namespace_uri =
                    (!name.namespace_uri.is_empty()).then_some(name.namespace_uri.as_str());
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
            XPathNodeTest::Kind {
                kind,
                processing_instruction_target,
                ..
            } => match kind {
                XPathKindTest::Document => self.result_node_kind() == XPathResultNodeKind::Document,
                XPathKindTest::Element | XPathKindTest::SchemaElement => {
                    self.result_node_kind() == XPathResultNodeKind::Element
                }
                XPathKindTest::Attribute | XPathKindTest::SchemaAttribute => {
                    self.result_node_kind() == XPathResultNodeKind::Attribute
                }
                XPathKindTest::ProcessingInstruction => {
                    self.result_node_kind() == XPathResultNodeKind::ProcessingInstruction
                        && processing_instruction_target
                            .as_ref()
                            .is_none_or(|target| self.expanded_name().as_ref() == Some(target))
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
