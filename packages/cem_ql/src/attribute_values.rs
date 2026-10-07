//! Query access to authorized original attribute targets. Source ownership and
//! authored structure stay intact; navigation uses only the captured boundary.
use crate::eval::{
    AtomValue, Item, QueryContextScope, QueryItemView, QueryItemViewKind, QueryNodeAccessError,
    QueryNodeIterator, QueryNodeTextIterator,
};
use cem_ml::{
    parser::{AstNodeId, CemAstNode},
    schema::{
        attribute_references::NativeAttributeTargetAccess,
        declaration_references::SchemaDeclarationNode,
    },
    source_map::SourceMapStack,
};
use std::{any::Any, collections::HashMap, sync::Arc};
#[derive(Debug)]
pub struct NativeAttributeQueryTree {
    access: Arc<NativeAttributeTargetAccess>,
    original: HashMap<(usize, AstNodeId), usize>,
    owning: HashMap<(usize, AstNodeId), usize>,
}
impl NativeAttributeQueryTree {
    pub fn new(access: Arc<NativeAttributeTargetAccess>) -> Result<Arc<Self>, String> {
        if !access.is_complete() {
            return Err("Attribute target access is incomplete".into());
        }
        let mut original = HashMap::new();
        let mut owning = HashMap::new();
        let mut pending = access.roots().to_vec();
        let mut visited = std::collections::HashSet::new();
        while let Some(index) = pending.pop() {
            if !visited.insert(index) {
                return Err("Invalid attribute access forest".into());
            }
            let node = access.node(index).ok_or("Invalid attribute access node")?;
            crate::validation_structure::validate_input_view(node, access.input_view(index))?;
            original
                .entry((Arc::as_ptr(node.document()) as usize, node.node_id()))
                .or_insert(index);
            for child in access
                .children(index)
                .ok_or("Invalid attribute access children")?
            {
                if access.parent(*child) != Some(index) {
                    return Err("Invalid attribute access parent".into());
                }
                let child_node = access.node(*child).ok_or("Invalid original child handle")?;
                if !Arc::ptr_eq(node.document(), child_node.document())
                    || owning
                        .insert((index, child_node.node_id()), *child)
                        .is_some()
                {
                    return Err("Invalid original owning edge".into());
                }
                pending.push(*child);
            }
        }
        Ok(Arc::new(Self {
            access,
            original,
            owning,
        }))
    }
    pub fn roots(self: &Arc<Self>) -> Vec<Item> {
        self.access
            .roots()
            .iter()
            .map(|index| self.item(*index))
            .collect()
    }
    fn item(self: &Arc<Self>, index: usize) -> Item {
        Item::native(NativeAttributeQueryNode {
            tree: self.clone(),
            index,
        })
    }
}
#[derive(Debug, Clone)]
pub struct NativeAttributeQueryNode {
    tree: Arc<NativeAttributeQueryTree>,
    index: usize,
}
impl NativeAttributeQueryNode {
    fn authored(&self) -> Option<Item> {
        crate::validation_structure::authored_input_node(
            self.source_node(),
            self.tree.access.input_view(self.index),
        )
    }
    fn expanded_name(&self) -> Option<&cem_ml::parser::ExpandedName> {
        crate::validation_structure::input_expanded_name(
            self.node(),
            self.source_node().node_id(),
            self.tree.access.input_view(self.index),
        )
    }
    pub fn source_node(&self) -> &SchemaDeclarationNode {
        self.tree.access.node(self.index).unwrap()
    }
    fn node(&self) -> &CemAstNode {
        self.source_node().node()
    }
    fn owning_items(&self, ids: &[AstNodeId]) -> Vec<Item> {
        ids.iter()
            .filter_map(|id| self.tree.owning.get(&(self.index, *id)))
            .map(|index| self.tree.item(*index))
            .collect()
    }
    fn nonowning_items(&self, ids: &[AstNodeId]) -> Option<Vec<Item>> {
        ids.iter()
            .map(|id| {
                self.tree
                    .original
                    .get(&(Arc::as_ptr(self.source_node().document()) as usize, *id))
                    .map(|index| self.tree.item(*index))
            })
            .collect()
    }
    fn child_items(&self) -> Vec<Item> {
        match self.node() {
            CemAstNode::Document { root_children, .. } => self.owning_items(root_children),
            CemAstNode::Element { children, .. } => self.owning_items(children),
            _ => vec![],
        }
    }
    fn attribute_items(&self) -> Vec<Item> {
        match self.node() {
            CemAstNode::Element { attributes, .. } => self.owning_items(attributes),
            _ => vec![],
        }
    }
}
fn scalar(value: &str) -> Vec<Item> {
    vec![Item::Atomic(AtomValue::String(value.into()))]
}
impl QueryItemView for NativeAttributeQueryNode {
    fn provenance(&self) -> Option<cem_ml::value::artifact::CemValueProvenance> {
        self.authored()?.view()?.provenance()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.ql.attribute-target"
    }
    fn identity(&self) -> String {
        self.source_node().identity()
    }
    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Node
    }
    fn source_map(&self) -> Option<SourceMapStack> {
        Some(source(self.node()).clone())
    }
    fn parent(&self, _: QueryContextScope) -> Result<Option<Item>, QueryNodeAccessError> {
        Ok(self
            .tree
            .access
            .parent(self.index)
            .map(|index| self.tree.item(index)))
    }
    fn children(
        &self,
        _: QueryContextScope,
    ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
        Ok(Box::new(self.child_items().into_iter().map(Ok)))
    }
    fn attributes(
        &self,
        _: QueryContextScope,
    ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
        Ok(Box::new(self.attribute_items().into_iter().map(Ok)))
    }
    fn text_fragments(
        &self,
        _: QueryContextScope,
    ) -> Result<QueryNodeTextIterator<'_>, QueryNodeAccessError> {
        let mut pending = vec![self.index];
        Ok(Box::new(std::iter::from_fn(move || {
            let index = pending.pop()?;
            let current = self.tree.access.node(index).unwrap();
            if matches!(
                current.node(),
                CemAstNode::Document { .. } | CemAstNode::Element { .. }
            ) {
                pending.extend(
                    self.tree
                        .access
                        .children(index)
                        .unwrap()
                        .iter()
                        .rev()
                        .filter(|child| {
                            !matches!(
                                self.tree.access.node(**child).unwrap().node(),
                                CemAstNode::Attribute { .. }
                            )
                        })
                        .copied(),
                );
            }
            Some(Ok(text(current.node()).unwrap_or("")))
        })))
    }
    fn field(&self, name: &str) -> Option<Vec<Item>> {
        match name {
            "source" => return self.authored().map(|node| vec![node]),
            "children" => return Some(self.child_items()),
            "attributes" => return Some(self.attribute_items()),
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
        match (self.node(), name) {
            (node, "kind") => Some(scalar(match node {
                CemAstNode::Document { .. } => "document",
                CemAstNode::Element { .. } => "element",
                CemAstNode::Attribute { .. } => "attribute",
                CemAstNode::Reference { .. } => "reference",
                CemAstNode::Text { .. } => "text",
                CemAstNode::Whitespace { .. } => "whitespace",
                CemAstNode::Comment { .. } => "comment",
                CemAstNode::Cdata { .. } => "cdata",
                CemAstNode::RawText { .. } => "raw-text",
                CemAstNode::ProcessingInstruction { .. } => "processing-instruction",
                CemAstNode::Error { .. } => "error",
            })),
            (CemAstNode::Element { .. } | CemAstNode::Attribute { .. }, "name" | "element") => {
                Some(scalar(&self.expanded_name()?.local_name))
            }
            (CemAstNode::Element { .. } | CemAstNode::Attribute { .. }, "namespace") => {
                Some(scalar(&self.expanded_name()?.namespace_uri))
            }
            (CemAstNode::Attribute { value_nodes, .. }, "valueNodes") => {
                Some(self.owning_items(value_nodes))
            }
            (CemAstNode::Attribute { value_nodes, .. }, "value" | "values")
                if !value_nodes.is_empty() =>
            {
                Some(self.owning_items(value_nodes))
            }
            (CemAstNode::Attribute { value, .. }, "value" | "values") => {
                Some(value.as_deref().map(scalar).unwrap_or_default())
            }
            (CemAstNode::Reference { expression, .. }, "expression") => Some(scalar(expression)),
            (CemAstNode::Reference { context, .. }, "context") => self.nonowning_items(&[*context]),
            (CemAstNode::Reference { targets, .. }, "targets") => {
                self.nonowning_items(targets.as_deref()?)
            }
            (
                CemAstNode::Text { data, .. }
                | CemAstNode::Whitespace { data, .. }
                | CemAstNode::Cdata { data, .. }
                | CemAstNode::RawText { data, .. }
                | CemAstNode::Comment { data, .. }
                | CemAstNode::ProcessingInstruction { data, .. },
                "data" | "value",
            ) => Some(scalar(data)),
            (CemAstNode::ProcessingInstruction { target, .. }, "name" | "target") => {
                Some(scalar(target))
            }
            (CemAstNode::Error { code, .. }, "code") => Some(scalar(code)),
            _ => None,
        }
    }
}
fn text(node: &CemAstNode) -> Option<&str> {
    match node {
        CemAstNode::Attribute { value, .. } => value.as_deref(),
        CemAstNode::Text { data, .. }
        | CemAstNode::Whitespace { data, .. }
        | CemAstNode::Cdata { data, .. }
        | CemAstNode::RawText { data, .. } => Some(data),
        _ => None,
    }
}
fn source(node: &CemAstNode) -> &SourceMapStack {
    match node {
        CemAstNode::Document { source, .. }
        | CemAstNode::Element { source, .. }
        | CemAstNode::Reference { source, .. }
        | CemAstNode::Attribute { source, .. }
        | CemAstNode::Text { source, .. }
        | CemAstNode::Whitespace { source, .. }
        | CemAstNode::Comment { source, .. }
        | CemAstNode::Cdata { source, .. }
        | CemAstNode::RawText { source, .. }
        | CemAstNode::ProcessingInstruction { source, .. }
        | CemAstNode::Error { source, .. } => source,
    }
}
