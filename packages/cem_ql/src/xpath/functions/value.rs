//! Explicit native XPath values, not generic record/JSON projections.
use crate::eval::{
    AtomValue, Item, QueryContextScope, QueryItemView, QueryItemViewKind, QueryNodeAccessError,
    QueryNodeIterator, QueryNodeTextIterator,
};
use cem_ml::{
    source_map::SourceMapStack,
    validation::xpath::{XPathNativeNode, XPathResultItem},
};
use std::{any::Any, sync::Arc};

#[derive(Debug, Clone)]
pub struct XPathQueryItem {
    value: Arc<XPathResultItem>,
}

impl XPathQueryItem {
    pub fn from_node(node: XPathNativeNode) -> Item {
        Self::wrap(XPathResultItem::from_native_node(node))
    }

    pub fn xpath_item(&self) -> &XPathResultItem {
        &self.value
    }

    pub(crate) fn wrap(value: XPathResultItem) -> Item {
        Item::native(Self {
            value: Arc::new(value),
        })
    }
}

impl QueryItemView for XPathQueryItem {
    fn provenance(&self) -> Option<cem_ml::value::artifact::CemValueProvenance> {
        let node = self.xpath_item().native_node()?;
        Some(cem_ml::value::artifact::CemValueProvenance { source_uri: node.owner().document_uri().map(str::to_owned), source_key: node.source_key(), line_number: node.source_line_number() })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.xpath.native-item/1"
    }
    fn identity(&self) -> String {
        if let Some(node) = self.value.native_node() {
            node.identity()
        } else {
            format!("{:p}", Arc::as_ptr(&self.value))
        }
    }
    fn kind(&self) -> QueryItemViewKind {
        if self.value.native_node().is_some() {
            QueryItemViewKind::Node
        } else {
            QueryItemViewKind::Atomic
        }
    }
    fn parent(&self, _scope: QueryContextScope) -> Result<Option<Item>, QueryNodeAccessError> {
        let node = self
            .value
            .native_node()
            .ok_or(QueryNodeAccessError::Unsupported)?;
        Ok(node.parent_node().map(Self::from_node))
    }
    fn children(
        &self,
        _scope: QueryContextScope,
    ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
        let node = self
            .value
            .native_node()
            .ok_or(QueryNodeAccessError::Unsupported)?;
        Ok(Box::new(
            node.child_nodes_iter().map(|node| Ok(Self::from_node(node))),
        ))
    }
    fn text_fragments(
        &self,
        _scope: QueryContextScope,
    ) -> Result<QueryNodeTextIterator<'_>, QueryNodeAccessError> {
        let node = self.value.native_node().ok_or(QueryNodeAccessError::Unsupported)?;
        Ok(Box::new(node.text_fragments().map(Ok)))
    }
    fn atom(&self) -> Option<AtomValue> {
        if let Some(node) = self.value.native_node() {
            return Some(AtomValue::String(node.string_value()));
        }
        let XPathResultItem::Atomic { value, .. } = self.value.as_ref() else {
            return None;
        };
        let text = &value.lexical_value;
        Some(match value.type_name.as_str() {
            "xs:string" | "xs:untypedAtomic" => AtomValue::String(text.clone()),
            "xs:anyURI" => AtomValue::AnyUri(text.clone()),
            "xs:boolean" => AtomValue::Boolean(matches!(text.as_str(), "true" | "1")),
            "xs:integer" => text
                .parse::<i64>()
                .map(AtomValue::Integer)
                .unwrap_or_else(|_| AtomValue::Decimal(text.clone())),
            "xs:decimal" => AtomValue::Decimal(text.clone()),
            "xs:float" | "xs:double" => AtomValue::Double(match text.as_str() {
                "INF" => f64::INFINITY,
                "-INF" => f64::NEG_INFINITY,
                "NaN" => f64::NAN,
                _ => text.parse().ok()?,
            }),
            _ => return None,
        })
    }
    fn source_map(&self) -> Option<SourceMapStack> {
        Some(match self.value.as_ref() {
            XPathResultItem::Node { source_map, .. }
            | XPathResultItem::Atomic { source_map, .. }
            | XPathResultItem::Map { source_map, .. }
            | XPathResultItem::Array { source_map, .. }
            | XPathResultItem::Function { source_map, .. } => source_map.clone(),
        })
    }
    // No fields/members view: never flatten XDM arrays or infer CEM records.
}
