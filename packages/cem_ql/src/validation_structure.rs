//! Native query access to consumed validation placements, never expanded arenas.
//! The explicit consumer has already selected targets and enforced scope grants.
use crate::eval::{
    AtomValue, Item, QueryContextScope, QueryItemView, QueryItemViewKind, QueryNodeAccessError,
    QueryNodeIterator, QueryNodeTextIterator,
};
use cem_ml::{
    parser::{document::CemDocument, AstNodeId, CemAstNode, ExpandedName},
    schema::{
        declaration_references::SchemaDeclarationNode,
        input_references::{InputNodeView, RetainedValidationStructure, StructuralValidationNode},
    },
    source_map::SourceMapStack,
};
use std::{any::Any, sync::Arc};

#[derive(Debug)]
pub struct RetainedValidationQueryTree {
    // Retain handles and placement edges only. Original CEM arenas stay shared.
    source: Arc<CemDocument>,
    nodes: Vec<StructuralValidationNode>,
    roots: Vec<usize>,
    parents: Vec<Option<usize>>,
    attribute_values: Vec<
        std::collections::HashMap<
            AstNodeId,
            Arc<crate::attribute_values::NativeAttributeQueryTree>,
        >,
    >,
}
impl RetainedValidationQueryTree {
    /// Only complete forests can be queried: unavailable children are not empty.
    /// Validate graph invariants before any axis or field access is exposed.
    pub fn new(structure: RetainedValidationStructure<'_>) -> Result<Arc<Self>, String> {
        if !structure.complete || structure.nodes.iter().any(|node| !node.children_complete) {
            return Err("Retained validation structure is incomplete".into());
        }
        let mut parents = vec![None; structure.nodes.len()];
        let mut roots = vec![false; structure.nodes.len()];
        for &root in structure.roots {
            let Some(root_slot) = roots.get_mut(root) else {
                return Err("Invalid validation root index".into());
            };
            if *root_slot {
                return Err("Repeated validation root placement".into());
            }
            *root_slot = true;
        }
        for (index, node) in structure.nodes.iter().enumerate() {
            validate_input_view(&node.source, node.input_view.as_ref())?;
            match node.source.node() {
                CemAstNode::Element { attributes, .. } => {
                    if attributes.iter().any(|id| {
                        !matches!(
                            node.source.document().get(*id),
                            Some(CemAstNode::Attribute { .. })
                        )
                    }) {
                        return Err("Invalid original attribute handle".into());
                    }
                }
                CemAstNode::Text { .. }
                | CemAstNode::Whitespace { .. }
                | CemAstNode::Cdata { .. }
                | CemAstNode::RawText { .. }
                | CemAstNode::Comment { .. }
                | CemAstNode::ProcessingInstruction { .. }
                    if node.children.is_empty() => {}
                _ => return Err("Expected an original structural node placement".into()),
            }
            for &child in &node.children {
                let Some(parent) = parents.get_mut(child) else {
                    return Err("Invalid validation child index".into());
                };
                if roots[child] || parent.is_some() {
                    return Err("Validation placement has multiple owning edges".into());
                }
                *parent = Some(index);
            }
        }
        let mut visited = vec![false; structure.nodes.len()];
        let mut pending = structure.roots.to_vec();
        while let Some(index) = pending.pop() {
            if visited[index] {
                return Err("Cyclic validation placement graph".into());
            }
            visited[index] = true;
            pending.extend(structure.nodes[index].children.iter().copied());
        }
        if visited.iter().any(|visited| !visited) {
            return Err("Unreachable validation placement".into());
        }
        let attribute_values = structure
            .nodes
            .iter()
            .map(|node| {
                node.attribute_values
                    .iter()
                    .map(|value| {
                        if !value.complete {
                            return Err("Consumed attribute value is incomplete".to_owned());
                        }
                        Ok((
                            value.attribute.node_id(),
                            crate::attribute_values::NativeAttributeQueryTree::new(
                                value.access.clone(),
                            )?,
                        ))
                    })
                    .collect::<Result<std::collections::HashMap<_, _>, String>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Arc::new(Self {
            source: structure.source.clone(),
            nodes: structure.nodes.to_vec(),
            roots: structure.roots.to_vec(),
            parents,
            attribute_values,
        }))
    }
    pub fn source(&self) -> &Arc<CemDocument> {
        &self.source
    }
    pub fn roots(self: &Arc<Self>) -> Vec<Item> {
        self.roots
            .iter()
            .map(|&index| self.item(index, None))
            .collect()
    }
    pub fn node(self: &Arc<Self>, placement: usize) -> Option<Item> {
        self.nodes.get(placement)?;
        Some(self.item(placement, None))
    }
    fn item(self: &Arc<Self>, placement: usize, attribute: Option<AstNodeId>) -> Item {
        Item::native(ValidationPlacementNode {
            tree: self.clone(),
            placement,
            attribute,
        })
    }
}

/// Placement identity is local to this snapshot. Original identity/lexical
/// context remains available separately; neither is an authored ID convention.
#[derive(Debug, Clone)]
pub struct ValidationPlacementNode {
    tree: Arc<RetainedValidationQueryTree>,
    placement: usize,
    attribute: Option<AstNodeId>,
}
impl ValidationPlacementNode {
    fn authored(&self) -> Option<Item> {
        authored_input_node(
            &self.source_node(),
            self.tree.nodes[self.placement].input_view.as_ref(),
        )
    }
    fn expanded_name(&self) -> Option<&ExpandedName> {
        let source = &self.tree.nodes[self.placement].source;
        input_expanded_name(
            self.node(),
            self.attribute.unwrap_or(source.node_id()),
            self.tree.nodes[self.placement].input_view.as_ref(),
        )
    }
    /// Snapshot owner used by consumers to reject selections from another stage.
    pub fn owner(&self) -> &Arc<RetainedValidationQueryTree> {
        &self.tree
    }
    pub fn placement(&self) -> usize {
        self.placement
    }
    pub fn source_node(&self) -> SchemaDeclarationNode {
        let source = &self.tree.nodes[self.placement].source;
        self.attribute
            .map(|id| SchemaDeclarationNode::new(source.document().clone(), id).unwrap())
            .unwrap_or_else(|| source.clone())
    }
    pub fn declaring_schema(&self) -> Option<&SchemaDeclarationNode> {
        self.tree.nodes[self.placement].declaring_schema.as_ref()
    }
    fn node(&self) -> &CemAstNode {
        let source = &self.tree.nodes[self.placement].source;
        self.attribute
            .map(|id| source.document().get(id).unwrap())
            .unwrap_or_else(|| source.node())
    }
    fn child_items(&self) -> Vec<Item> {
        if self.attribute.is_some() {
            return vec![];
        }
        self.tree.nodes[self.placement]
            .children
            .iter()
            .map(|&index| self.tree.item(index, None))
            .collect()
    }
    fn attribute_items(&self) -> Vec<Item> {
        match self.node() {
            CemAstNode::Element { attributes, .. } => attributes
                .iter()
                .map(|&id| self.tree.item(self.placement, Some(id)))
                .collect(),
            _ => vec![],
        }
    }
}
fn string(value: &str) -> Vec<Item> {
    vec![Item::Atomic(AtomValue::String(value.into()))]
}
impl QueryItemView for ValidationPlacementNode {
    fn provenance(&self) -> Option<cem_ml::value::artifact::CemValueProvenance> {
        self.authored()?.view()?.provenance()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.ql.validation-placement"
    }
    fn identity(&self) -> String {
        format!(
            "validation:{:p}:{}:{:?}",
            Arc::as_ptr(&self.tree),
            self.placement,
            self.attribute
        )
    }
    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Node
    }
    fn source_map(&self) -> Option<SourceMapStack> {
        Some(source_map(self.node()).clone())
    }
    fn parent(&self, _: QueryContextScope) -> Result<Option<Item>, QueryNodeAccessError> {
        let parent = if self.attribute.is_some() {
            Some(self.placement)
        } else {
            self.tree.parents[self.placement]
        };
        Ok(parent.map(|index| self.tree.item(index, None)))
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
        if self.attribute.is_some() {
            return Ok(Box::new(std::iter::once(Ok(
                text(self.node()).unwrap_or("")
            ))));
        }
        let mut pending = vec![self.placement];
        Ok(Box::new(std::iter::from_fn(move || {
            let index = pending.pop()?;
            let current = &self.tree.nodes[index];
            pending.extend(current.children.iter().rev().copied());
            // Empty fragments preserve visits for the query runtime's work budget.
            Some(Ok(text(current.source.node()).unwrap_or("")))
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
        if matches!(name, "value" | "values") {
            if let Some(attribute) = self.attribute {
                if let Some(values) = self.tree.attribute_values[self.placement].get(&attribute) {
                    return Some(values.roots());
                }
            }
        }
        match (self.node(), name) {
            (CemAstNode::Element { .. }, "kind") => Some(string("element")),
            (CemAstNode::Attribute { .. }, "kind") => Some(string("attribute")),
            (CemAstNode::Text { .. }, "kind") => Some(string("text")),
            (CemAstNode::Whitespace { .. }, "kind") => Some(string("whitespace")),
            (CemAstNode::Cdata { .. }, "kind") => Some(string("cdata")),
            (CemAstNode::RawText { .. }, "kind") => Some(string("raw-text")),
            (CemAstNode::Comment { .. }, "kind") => Some(string("comment")),
            (CemAstNode::ProcessingInstruction { .. }, "kind") => {
                Some(string("processing-instruction"))
            }
            (CemAstNode::Element { .. } | CemAstNode::Attribute { .. }, "name" | "element") => {
                Some(string(&self.expanded_name()?.local_name))
            }
            (CemAstNode::Element { .. } | CemAstNode::Attribute { .. }, "namespace") => {
                Some(string(&self.expanded_name()?.namespace_uri))
            }
            (CemAstNode::Attribute { value, .. }, "value" | "values") => {
                Some(value.as_deref().map(string).unwrap_or_default())
            }
            (
                CemAstNode::Text { data, .. }
                | CemAstNode::Whitespace { data, .. }
                | CemAstNode::Cdata { data, .. }
                | CemAstNode::RawText { data, .. }
                | CemAstNode::Comment { data, .. }
                | CemAstNode::ProcessingInstruction { data, .. },
                "value" | "data",
            ) => Some(string(data)),
            (CemAstNode::ProcessingInstruction { target, .. }, "name" | "target") => {
                Some(string(target))
            }
            _ => None,
        }
    }
}
pub(crate) fn authored_input_node(
    source: &SchemaDeclarationNode,
    view: Option<&InputNodeView>,
) -> Option<Item> {
    let tree = view?.source_tree.as_ref()?;
    crate::eval::RetainedCemNode::new(tree.clone(), source.node_id()).map(|node| node.query_item())
}
pub(crate) fn input_expanded_name<'a>(
    node: &'a CemAstNode,
    id: AstNodeId,
    view: Option<&'a InputNodeView>,
) -> Option<&'a ExpandedName> {
    if let Some(view) = view {
        return view.names.get(&id);
    }
    match node {
        CemAstNode::Element { expanded_name, .. } | CemAstNode::Attribute { expanded_name, .. } => {
            Some(expanded_name)
        }
        _ => None,
    }
}
pub(crate) fn validate_input_view(
    source: &SchemaDeclarationNode,
    view: Option<&InputNodeView>,
) -> Result<(), String> {
    let Some(view) = view else { return Ok(()) };
    if view
        .source_tree
        .as_ref()
        .is_some_and(|tree| !Arc::ptr_eq(tree.ast_owner(), source.document()))
    {
        return Err("Input view belongs to another original source owner".into());
    }
    let mut ids = vec![source.node_id()];
    if let CemAstNode::Element { attributes, .. } = source.node() {
        ids.extend(attributes);
    }
    for id in ids {
        if let Some(
            CemAstNode::Element { expanded_name, .. } | CemAstNode::Attribute { expanded_name, .. },
        ) = source.document().get(id)
        {
            let Some(name) = view.names.get(&id) else {
                return Err("Input view name is incomplete".into());
            };
            if name.local_name != expanded_name.local_name {
                return Err("Input view changed an original local name".into());
            }
        }
    }
    Ok(())
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
fn source_map(node: &CemAstNode) -> &SourceMapStack {
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
