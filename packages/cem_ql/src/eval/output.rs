//! Query access to shared constructed CEM values. A reference occurrence has
//! an output parent; its explicit targets retain their original source parents.
use super::*;
use crate::render::{RenderPlanAttribute, RenderPlanNode};

#[derive(Debug, Clone)]
pub struct OutputView {
    owner: Arc<Vec<RenderPlanNode>>,
    path: Vec<usize>,
    attribute: Option<usize>,
}

pub fn output_nodes(nodes: Vec<RenderPlanNode>) -> ItemStream {
    shared_output_nodes(Arc::new(nodes))
}

pub fn shared_output_nodes(owner: Arc<Vec<RenderPlanNode>>) -> ItemStream {
    ItemStream::from_items(
        (0..owner.len())
            .map(|index| {
                Item::native(OutputView {
                    owner: owner.clone(),
                    path: vec![index],
                    attribute: None,
                })
            })
            .collect(),
    )
}

pub fn output_attribute(attribute: RenderPlanAttribute) -> Item {
    let source_map = attribute.source_map.clone();
    Item::native(OutputView {
        owner: Arc::new(vec![RenderPlanNode::Element {
            tag: "value".into(),
            namespace: None,
            qualified_name: None,
            attributes: vec![attribute],
            children: Vec::new(),
            source_map,
        }]),
        path: vec![0],
        attribute: Some(0),
    })
}

impl OutputView {
    pub fn node(&self) -> &RenderPlanNode {
        let mut node = &self.owner[self.path[0]];
        for &index in &self.path[1..] {
            let RenderPlanNode::Element { children, .. } = node else {
                unreachable!("validated output path")
            };
            node = &children[index];
        }
        node
    }
    fn attribute(&self) -> Option<&RenderPlanAttribute> {
        let RenderPlanNode::Element { attributes, .. } = self.node() else {
            return None;
        };
        attributes.get(self.attribute?)
    }
    fn child(&self, index: usize) -> Item {
        let mut path = self.path.clone();
        path.push(index);
        Item::native(Self {
            owner: self.owner.clone(),
            path,
            attribute: None,
        })
    }
}

impl QueryItemView for OutputView {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.constructed-value"
    }
    fn identity(&self) -> String {
        format!(
            "cem:output:{:p}:{:?}:{:?}",
            Arc::as_ptr(&self.owner),
            self.path,
            self.attribute
        )
    }
    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Node
    }
    fn parent(&self, _: QueryContextScope) -> Result<Option<Item>, QueryNodeAccessError> {
        let mut parent = self.clone();
        if parent.attribute.take().is_none() {
            parent.path.pop();
        }
        Ok((!parent.path.is_empty()).then(|| Item::native(parent)))
    }
    fn children(
        &self,
        _: QueryContextScope,
    ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
        let count = match self.node() {
            RenderPlanNode::Element { children, .. } if self.attribute.is_none() => children.len(),
            _ => 0,
        };
        Ok(Box::new((0..count).map(|i| Ok(self.child(i)))))
    }
    fn source_map(&self) -> Option<SourceMapStack> {
        if let Some(attribute) = self.attribute() {
            return Some(attribute.source_map.clone());
        }
        Some(match self.node() {
            RenderPlanNode::Element { source_map, .. }
            | RenderPlanNode::Reference { source_map, .. }
            | RenderPlanNode::Text { source_map, .. }
            | RenderPlanNode::Cdata { source_map, .. }
            | RenderPlanNode::Comment { source_map, .. }
            | RenderPlanNode::ProcessingInstruction { source_map, .. } => source_map.clone(),
        })
    }
    fn value_contract(&self) -> Option<cem_ml::schema::document_model::AttributeValueContract> {
        self.attribute()?.contract.as_deref().cloned()
    }
    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let text = |s: &str| Some(vec![Item::Atomic(AtomValue::String(s.into()))]);
        if name == "id" {
            return text(&self.identity());
        }
        if let Some(attribute) = self.attribute() {
            return match name {
                "kind" => text("attribute"),
                "name" => text(&attribute.name),
                "namespace" => text(attribute.namespace.as_deref().unwrap_or("")),
                "value" => text(&attribute.value),
                "values" => Some(attribute.value_stream.items.clone()),
                "datatype" => attribute
                    .contract
                    .as_ref()
                    .and_then(|c| c.model.value_type.as_deref())
                    .and_then(text),
                _ => None,
            };
        }
        match (name, self.node()) {
            ("kind", node) => text(match node {
                RenderPlanNode::Element { .. } => "element",
                RenderPlanNode::Reference { .. } => "reference",
                RenderPlanNode::Text { .. } => "text",
                RenderPlanNode::Cdata { .. } => "cdata",
                RenderPlanNode::Comment { .. } => "comment",
                RenderPlanNode::ProcessingInstruction { .. } => "processing-instruction",
            }),
            ("name", RenderPlanNode::Element { tag, .. }) => text(tag),
            ("namespace", RenderPlanNode::Element { namespace, .. }) => {
                text(namespace.as_deref().unwrap_or(""))
            }
            ("children", RenderPlanNode::Element { children, .. }) => {
                Some((0..children.len()).map(|i| self.child(i)).collect())
            }
            ("attributes", RenderPlanNode::Element { attributes, .. }) => Some(
                (0..attributes.len())
                    .map(|i| {
                        Item::native(Self {
                            attribute: Some(i),
                            ..self.clone()
                        })
                    })
                    .collect(),
            ),
            ("targets", RenderPlanNode::Reference { reference, .. }) => {
                Some(reference.values().to_vec())
            }
            (
                "value",
                RenderPlanNode::Text { text: value, .. }
                | RenderPlanNode::Cdata { text: value, .. }
                | RenderPlanNode::Comment { text: value, .. },
            ) => text(value),
            _ => None,
        }
    }
    fn text_fragments(
        &self,
        scope: QueryContextScope,
    ) -> Result<QueryNodeTextIterator<'_>, QueryNodeAccessError> {
        if let Some(attribute) = self.attribute() {
            return Ok(item_text(&attribute.value_stream.items, scope));
        }
        Ok(node_text(self.node(), scope, true))
    }
}

fn item_text(items: &[Item], scope: QueryContextScope) -> QueryNodeTextIterator<'_> {
    Box::new(
        items
            .iter()
            .flat_map(move |item| -> QueryNodeTextIterator<'_> {
                if let Some(values) = values::reference_values(item) {
                    return item_text(values, scope);
                }
                if let Some(view) = item.view() {
                    return view
                        .text_fragments(scope)
                        .unwrap_or_else(|error| Box::new(std::iter::once(Err(error))));
                }
                match item {
                    Item::Atomic(
                        AtomValue::String(s) | AtomValue::Decimal(s) | AtomValue::AnyUri(s),
                    ) => Box::new(std::iter::once(Ok(s.as_str()))),
                    _ => Box::new(std::iter::once(Err(QueryNodeAccessError::Unsupported))),
                }
            }),
    )
}

fn node_text(
    node: &RenderPlanNode,
    scope: QueryContextScope,
    root: bool,
) -> QueryNodeTextIterator<'_> {
    match node {
        RenderPlanNode::Element { children, .. } => Box::new(
            std::iter::once(Ok("")).chain(
                children
                    .iter()
                    .flat_map(move |child| node_text(child, scope, false)),
            ),
        ),
        RenderPlanNode::Reference { reference, .. } => item_text(reference.values(), scope),
        RenderPlanNode::Text { text, .. } | RenderPlanNode::Cdata { text, .. } => {
            Box::new(std::iter::once(Ok(text.as_str())))
        }
        RenderPlanNode::Comment { text, .. } if root => {
            Box::new(std::iter::once(Ok(text.as_str())))
        }
        RenderPlanNode::ProcessingInstruction { data, .. } if root => {
            Box::new(std::iter::once(Ok(data.as_str())))
        }
        _ => Box::new(std::iter::once(Ok(""))),
    }
}
