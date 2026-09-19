//! Format-neutral CEM reader views. External syntax belongs to cem_ml::import.
use super::{AtomValue, Item, QueryItemView, QueryItemViewKind};
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

pub(super) fn source_metadata(item: &Item, line: bool) -> Result<Option<AtomValue>, ()> {
    let node = if let Some(view) = item
        .view()
        .and_then(|v| v.downcast_ref::<crate::xpath::functions::XPathQueryItem>())
    {
        view.xpath_item().native_node().cloned().ok_or(())?
    } else {
        xpath_node(item).ok_or(())?.map_err(|_| ())?
    };
    Ok(if line {
        node.source_line_number()
            .map(|line| AtomValue::Integer(line.into()))
    } else {
        node.source_key().map(AtomValue::String)
    })
}

/// A retained CEM document, shared by imports, lifecycle adapters and loaders.
/// The native owner travels with every selected node; no document is serialized.
pub fn imported_cem_tree(tree: Arc<RetainedCemTree>) -> Item {
    Item::native(CemAstView {
        owner: Arc::new(Imported {
            identity: format!("cem:{:p}", Arc::as_ptr(&tree)),
            tree: Some(tree),
            source: String::new(),
            error: String::new(),
        }),
        node: Some(0),
    })
}

#[derive(Debug)]
struct Imported {
    tree: Option<Arc<RetainedCemTree>>,
    source: String,
    identity: String,
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
    Item::native(CemAstView {
        owner: Arc::new(Imported {
            tree,
            source: source.into(),
            identity: format!("{:016x}", hash.finish()),
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
            return Some(string(self.identity()));
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
