//! Per-execution expanded names over an unchanged retained source tree.
//! Construction consumes an already-ready completion; it grants no crossings
//! and performs no reference evaluation. Authored access is explicit via source.
use crate::eval::{
    AtomValue, Item, QueryContextScope, QueryItemView, QueryItemViewKind, QueryNodeAccessError,
    QueryNodeIterator, QueryNodeTextIterator, RetainedCemNode,
};
use cem_ml::{
    parser::{tree::RetainedCemTree, AstNodeId},
    schema::namespace_references::NamespaceNameCompletion,
    source_map::SourceMapStack,
};
use std::{any::Any, sync::Arc};

#[derive(Debug)]
pub struct NamespaceQueryTree {
    source: Arc<RetainedCemTree>,
    completion: Arc<NamespaceNameCompletion>,
}
impl NamespaceQueryTree {
    pub fn new(
        source: Arc<RetainedCemTree>,
        completion: Arc<NamespaceNameCompletion>,
    ) -> Result<Arc<Self>, String> {
        if !Arc::ptr_eq(source.ast_owner(), completion.captured().document()) {
            return Err("Namespace completion belongs to another source owner".into());
        }
        Ok(Arc::new(Self { source, completion }))
    }
    pub fn completion(&self) -> &Arc<NamespaceNameCompletion> {
        &self.completion
    }
    pub fn roots(self: &Arc<Self>) -> Vec<Item> {
        self.completion
            .roots()
            .iter()
            .map(|id| self.node(*id).unwrap())
            .collect()
    }
    pub fn node(self: &Arc<Self>, id: AstNodeId) -> Option<Item> {
        self.completion.contains(id).then(|| {
            Item::native(NamespaceQueryNode {
                tree: self.clone(),
                id,
            })
        })
    }
}
#[derive(Debug, Clone)]
pub struct NamespaceQueryNode {
    tree: Arc<NamespaceQueryTree>,
    id: AstNodeId,
}
impl NamespaceQueryNode {
    pub fn source_node(&self) -> RetainedCemNode {
        RetainedCemNode::new(self.tree.source.clone(), self.id).unwrap()
    }
    fn authored(&self) -> Item {
        self.source_node().query_item()
    }
    fn project(&self, item: Item) -> Option<Item> {
        if let Some(node) = crate::eval::retained_cem_node(&item) {
            if !Arc::ptr_eq(node.owner().ast_owner(), self.tree.source.ast_owner()) {
                return None;
            }
            return self.tree.node(node.node_id());
        }
        Some(item)
    }
    fn axis(&self, name: &str) -> Vec<Item> {
        self.authored()
            .view()
            .unwrap()
            .field(name)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| self.project(item))
            .collect()
    }
}
impl QueryItemView for NamespaceQueryNode {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.ql.namespace-names"
    }
    fn identity(&self) -> String {
        format!("namespace-names:{:p}:{}", Arc::as_ptr(&self.tree), self.id)
    }
    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Node
    }
    fn source_map(&self) -> Option<SourceMapStack> {
        self.authored().source_map()
    }
    fn provenance(&self) -> Option<cem_ml::value::artifact::CemValueProvenance> {
        self.authored().view()?.provenance()
    }
    fn parent(&self, _: QueryContextScope) -> Result<Option<Item>, QueryNodeAccessError> {
        // The selected roots have no completed-view ancestry outside the forest.
        Ok(self
            .tree
            .source
            .source_parent(self.id)
            .and_then(|id| self.tree.node(id)))
    }
    fn children(
        &self,
        _: QueryContextScope,
    ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
        Ok(Box::new(self.axis("children").into_iter().map(Ok)))
    }
    fn attributes(
        &self,
        _: QueryContextScope,
    ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
        Ok(Box::new(self.axis("attributes").into_iter().map(Ok)))
    }
    fn text_fragments(
        &self,
        _: QueryContextScope,
    ) -> Result<QueryNodeTextIterator<'_>, QueryNodeAccessError> {
        // Every source owning edge below this node is in the completed forest.
        let fragments = self
            .tree
            .source
            .source_text_fragments(self.id)
            .ok_or(QueryNodeAccessError::Unsupported)?;
        Ok(Box::new(fragments.map(Ok)))
    }
    fn field(&self, name: &str) -> Option<Vec<Item>> {
        match name {
            "source" => return Some(vec![self.authored()]),
            "id" => return Some(vec![Item::Atomic(AtomValue::String(self.identity()))]),
            "parent" => {
                return Some(
                    self.parent(QueryContextScope(0))
                        .ok()?
                        .into_iter()
                        .collect(),
                )
            }
            _ => {}
        }
        if let Some(expanded) = self.tree.completion.expanded_name(self.id) {
            let value = match name {
                "name" => Some(&expanded.local_name),
                "namespace" => Some(&expanded.namespace_uri),
                _ => None,
            };
            if let Some(value) = value {
                return Some(vec![Item::Atomic(AtomValue::String(value.clone()))]);
            }
        }
        // Unavailable reference graph edges do not borrow the authored view's
        // unresolved names. Owning axes are fully covered by construction.
        self.authored()
            .view()?
            .field(name)?
            .into_iter()
            .map(|item| self.project(item))
            .collect()
    }
}
