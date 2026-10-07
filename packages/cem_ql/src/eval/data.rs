//! Format-neutral CEM reader views. External syntax belongs to cem_ml::import.
use super::{
    AtomValue, Item, QueryContextScope, QueryItemView, QueryItemViewKind, QueryNodeAccessError,
    QueryNodeIterator, QueryNodeTextIterator,
};
use cem_ml::{
    import::import_data,
    parser::{tree::RetainedCemTree, AstNodeId, CemAstNode},
    source_map::{FrameSpan, SourceMapStack},
    validation::xpath::XPathNativeNode,
};
use std::{
    any::Any,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
};
mod xpath_view;
pub use xpath_view::DataReaderCache;

mod string_import;
pub(super) use string_import::parse;

pub(super) fn source_metadata(item: &Item, name: &str) -> Result<Option<AtomValue>, ()> {
    if let Some(provenance) = item.view().and_then(|v| v.provenance()) {
        match name {
            "node_key" => return Ok(provenance.source_key.map(AtomValue::String)),
            "line_number" => return Ok(provenance.line_number.map(|n| AtomValue::Integer(n.into()))),
            _ => {},
        }
    }

    let node = if let Some(view) = item
        .view()
        .and_then(|v| v.downcast_ref::<crate::xpath::functions::XPathQueryItem>())
    {
        view.xpath_item().native_node().cloned().ok_or(())?
    } else if let Some(view) = item
        .view()
        .and_then(|v| v.downcast_ref::<crate::namespace_names::NamespaceQueryNode>())
    {
        // Source metadata can use the retained original handle without replacing
        // completed-view navigation or names with an authored XPath projection.
        let source = view.source_node();
        XPathNativeNode::cem_node(source.owner().clone(), source.node_id()).map_err(|_| ())?
    } else {
        xpath_node(item).ok_or(())?.map_err(|_| ())?
    };
    Ok(match name {
        "line_number" => node.source_line_number().map(|line| AtomValue::Integer(line.into())),
        "node_key" => node.source_key().map(AtomValue::String),
        "base_uri" => node.base_uri().map(|uri| AtomValue::AnyUri(uri.into())),
        "document_uri" if node.result_node_kind() == cem_ml::validation::xpath::XPathResultNodeKind::Document => node.document_uri().map(|uri| AtomValue::AnyUri(uri.into())),
        _ => None,
    })
}

/// A retained CEM document, shared by imports, lifecycle adapters and loaders.
/// The native owner travels with every selected node; no document is serialized.
pub fn imported_cem_tree(tree: Arc<RetainedCemTree>) -> Item {
    Item::native(CemAstView {
        owner: Arc::new(Imported {
            // Multiple structural projections of one original arena retain source
            // identity; independently imported arenas remain distinct owners.
            identity: format!("cem:{:p}", Arc::as_ptr(tree.ast_owner())),
            selection_key: None,
            tree: Some(tree),
            source: String::new(),
            error: String::new(),
        }),
        node: Some(0),
    })
}

/// A typed original-source node and its retained storage owner.
#[derive(Debug, Clone)]
pub struct RetainedCemNode {
    owner: Arc<RetainedCemTree>,
    node_id: AstNodeId,
}
impl RetainedCemNode {
    pub fn new(owner: Arc<RetainedCemTree>, node_id: AstNodeId) -> Option<Self> {
        owner.ast().get(node_id)?;
        Some(Self { owner, node_id })
    }
    pub fn owner(&self) -> &Arc<RetainedCemTree> {
        &self.owner
    }
    pub fn node_id(&self) -> AstNodeId {
        self.node_id
    }
    pub fn node(&self) -> &CemAstNode {
        self.owner.ast().get(self.node_id).unwrap()
    }
    pub fn query_item(&self) -> Item {
        let root = imported_cem_tree(self.owner.clone());
        root.view()
            .unwrap()
            .downcast_ref::<CemAstView>()
            .unwrap()
            .item(self.node_id)
    }
}

/// Extract an imported original-source handle. This does not coerce XPath or
/// arbitrary host projections into source arenas or grant target access.
pub fn retained_cem_node(item: &Item) -> Option<RetainedCemNode> {
    if let Some(view) = item.view()?.downcast_ref::<crate::namespace_names::NamespaceQueryNode>() {
        return Some(view.source_node());
    }
    let view = item.view()?.downcast_ref::<CemAstView>()?;
    RetainedCemNode::new(view.owner.tree.clone()?, view.node?)
}

#[derive(Debug)]
struct Imported {
    tree: Option<Arc<RetainedCemTree>>,
    source: String,
    identity: String,
    // Compatibility key for data:read selection across unchanged rerenders.
    // It is not the identity of a retained document owner.
    selection_key: Option<String>,
    error: String,
}

pub(super) fn read(source: &str, format: &str, projection: &str) -> Item {
    let mut hash = DefaultHasher::new();
    source.hash(&mut hash);
    format.hash(&mut hash);
    if projection != "cem" {
        projection.hash(&mut hash);
    }
    let (tree, error) = match import_data(source, format, projection, "data:read/source") {
        Ok(tree) => (Some(tree), String::new()),
        Err(error) => (None, error),
    };
    if projection == "xpath" {
        return xpath_view::report(tree, error);
    }
    // Equal source bytes do not identify an independently imported document.
    // Match the host-supplied retained-tree view's owner identity instead.
    let identity = tree
        .as_ref()
        .map(|tree| format!("cem:{:p}", Arc::as_ptr(tree)))
        .unwrap_or_else(|| format!("read-error:{:016x}", hash.finish()));
    Item::native(CemAstView {
        owner: Arc::new(Imported {
            tree,
            source: source.into(),
            identity,
            selection_key: Some(format!("{:016x}", hash.finish())),
            error,
        }),
        node: None,
    })
}

pub(crate) fn xpath_node(item: &Item) -> Option<Result<XPathNativeNode, String>> {
    let view = item.view()?.downcast_ref::<CemAstView>()?;
    let id = view.node?;
    let tree = view.owner.tree.as_ref()?.clone();
    Some(XPathNativeNode::cem_node(tree, id).map_err(|error| error.to_string()))
}

pub(crate) fn source_node(item: &Item) -> Option<(Arc<RetainedCemTree>, AstNodeId)> {
    let node = retained_cem_node(item)?;
    Some((node.owner().clone(), node.node_id()))
}

/// Whole-document source owner for inert structural inspection. In particular,
/// XPath's semantic text coalescing must not replace the original source arena.
pub(super) fn retained_document(item: &Item) -> Option<Arc<RetainedCemTree>> {
    let view = item.view()?;
    if let Some(view) = view.downcast_ref::<CemAstView>() {
        return (view.node == Some(0)).then(|| view.owner.tree.clone()).flatten();
    }
    let node = view
        .downcast_ref::<crate::xpath::functions::XPathQueryItem>()?
        .xpath_item()
        .native_node()?;
    (node.result_node_kind() == cem_ml::validation::xpath::XPathResultNodeKind::Document)
        .then(|| node.owner().clone())
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
        | CemAstNode::Reference { source, .. }
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
    fn provenance(&self) -> Option<cem_ml::value::artifact::CemValueProvenance> {
        let tree = self.owner.tree.as_ref()?;
        Some(cem_ml::value::artifact::CemValueProvenance { source_uri: Some(tree.source_uri().into()), source_key: tree.source_key(self.node?), line_number: tree.source_line_number(self.node?) })
    }

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
    fn parent(&self, _scope: QueryContextScope) -> Result<Option<Item>, QueryNodeAccessError> {
        // Import grants this view a complete retained document. Restricted
        // host projections must implement their own scope-aware node views.
        let id = self.node.ok_or(QueryNodeAccessError::Unsupported)?;
        let tree = self
            .owner
            .tree
            .as_ref()
            .ok_or(QueryNodeAccessError::Unsupported)?;
        Ok(tree.source_parent(id).map(|id| self.item(id)))
    }
    fn children(
        &self,
        _scope: QueryContextScope,
    ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
        let id = self.node.ok_or(QueryNodeAccessError::Unsupported)?;
        let tree = self
            .owner
            .tree
            .as_ref()
            .ok_or(QueryNodeAccessError::Unsupported)?;
        let node = tree.ast().get(id).ok_or(QueryNodeAccessError::Unsupported)?;
        Ok(Box::new(children(node).iter().map(|id| Ok(self.item(*id)))))
    }
    fn attributes(
        &self,
        _scope: QueryContextScope,
    ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
        let id = self.node.ok_or(QueryNodeAccessError::Unsupported)?;
        let tree = self
            .owner
            .tree
            .as_ref()
            .ok_or(QueryNodeAccessError::Unsupported)?;
        let node = tree
            .ast()
            .get(id)
            .ok_or(QueryNodeAccessError::Unsupported)?;
        let attributes = match node {
            CemAstNode::Element { attributes, .. } => attributes.as_slice(),
            _ => &[],
        };
        Ok(Box::new(attributes.iter().map(|&id| Ok(self.item(id)))))
    }
    fn text_fragments(
        &self,
        _scope: QueryContextScope,
    ) -> Result<QueryNodeTextIterator<'_>, QueryNodeAccessError> {
        let fragments = self.owner.tree.as_ref()
            .and_then(|tree| tree.source_text_fragments(self.node?))
            .ok_or(QueryNodeAccessError::Unsupported)?;
        Ok(Box::new(fragments.map(Ok)))
    }
    fn source_map(&self) -> Option<SourceMapStack> {
        let tree = self.owner.tree.as_ref()?;
        let id = self.node?;
        if id == 0 { Some(tree.node(0)?.source.clone()) }
        else { Some(node_source(tree.ast().get(id)?).clone()) }
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
        let node = self.owner.tree.as_ref()?.ast().get(id)?;
        if name == "id" {
            return Some(string(
                self.owner
                    .selection_key
                    .as_ref()
                    .map(|key| format!("{key}:{:?}", self.node))
                    .unwrap_or_else(|| self.identity()),
            ));
        }
        if name == "line" {
            if self.owner.source.is_empty() {
                let tree = self.owner.tree.as_ref()?;
                return Some(string(
                    tree.node(tree.canonical_id(id)?)?.range.line.to_string(),
                ));
            }
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
                pending.extend(
                    children(self.owner.tree.as_ref()?.ast().get(id)?)
                        .iter()
                        .rev()
                        .copied(),
                );
            }
            return Some(out);
        }
        Some(match (node, name) {
            (CemAstNode::Reference { .. }, "kind") => string("reference"),
            (CemAstNode::Reference { expression, .. }, "expression") => string(expression),
            (CemAstNode::Reference { targets, .. }, "targets_available") =>
                vec![Item::Atomic(AtomValue::Boolean(targets.is_some()))],
            (CemAstNode::Reference { context, .. }, "context") => vec![self.item(*context)],
            (CemAstNode::Reference { targets, .. }, "targets") => {
                targets.as_ref()?.iter().map(|id| self.item(*id)).collect()
            }
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
            (CemAstNode::Attribute { value, .. }, "value" | "values") => {
                value.as_ref().map(string).unwrap_or_default()
            }
            (CemAstNode::Attribute { value_nodes, .. }, "valueNodes") => {
                value_nodes.iter().map(|id| self.item(*id)).collect()
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
