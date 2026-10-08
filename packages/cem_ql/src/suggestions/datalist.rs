//! Scalar native presentation over original source nodes. No selection proof,
//! query filtering, DOM IDs, editor claims or browser behavior is created here.
use super::*;

pub const DATALIST_NAMESPACE: &str = "https://cem.dev/ns/runtime/suggestions/datalist/v1";
pub const DATALIST_CONSUMER_IDENTITY: &str = "cem-native-datalist-v1";

/// Explicit control boundary: this profile has no query or label controls.
#[derive(Debug, Default)]
pub struct NativeDatalistConfig {}
impl NativeDatalistConfig {
    pub fn parse(json: &str) -> Result<Self, SuggestionsError> {
        if json.len() > 1024 {
            return Err(configuration("Native datalist controls exceed their bound"));
        }
        let controls: BTreeMap<String, serde::de::IgnoredAny> =
            serde_json::from_str(json).map_err(|e| configuration(e.to_string()))?;
        if !controls.is_empty() {
            return Err(configuration(
                "Native datalist accepts no query, filter, label or selection controls",
            ));
        }
        Ok(Self {})
    }
}
pub(crate) fn configuration(message: impl Into<String>) -> SuggestionsError {
    error("cem.suggestions.datalist_configuration", message, None)
}

#[derive(Debug)]
struct Projection {
    plan: Arc<SuggestionsPlan>,
    rows: Vec<usize>,
    warnings: Vec<SuggestionsError>,
}
#[derive(Debug, Clone)]
pub struct NativeDatalistView(Arc<Projection>);
impl NativeDatalistView {
    pub(crate) fn prepare(plan: Arc<SuggestionsPlan>) -> Result<Self, SuggestionsError> {
        if let Some(group) = plan.groups.first() {
            return Err(error(
                "cem.suggestions.datalist_group",
                "Native datalist requires an explicitly ungrouped source set",
                Some(&group.source),
            ));
        }
        let mut warnings = plan.warnings.clone();
        let mut rows = vec![];
        for (index, row) in plan.rows.iter().enumerate() {
            if row.value.is_empty() {
                warnings.push(error(
                    "cem.suggestions.datalist_empty_value",
                    "Empty source values are omitted from native datalist suggestions",
                    Some(&row.source),
                ));
            } else if !row.disabled && !row.hidden {
                rows.push(index);
            }
        }
        Ok(Self(Arc::new(Projection {
            plan,
            rows,
            warnings,
        })))
    }
    pub fn len(&self) -> usize {
        self.0.rows.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.rows.is_empty()
    }
    pub fn warnings(&self) -> &[SuggestionsError] {
        &self.0.warnings
    }
    pub fn root(&self) -> Item {
        self.node(None, None)
    }
    fn node(&self, row: Option<usize>, attribute: Option<usize>) -> Item {
        Item::native(DatalistNode {
            view: self.clone(),
            row,
            attribute,
        })
    }
}
#[derive(Debug, Clone)]
struct DatalistNode {
    view: NativeDatalistView,
    row: Option<usize>,
    attribute: Option<usize>,
}
impl DatalistNode {
    fn source(&self) -> Option<&Item> {
        self.row.map(|i| &self.view.0.plan.rows[i].source)
    }
    fn attribute_value(&self, index: usize) -> Option<(&str, &str)> {
        let row = &self.view.0.plan.rows[self.row?];
        match index {
            0 => Some(("value", &row.value)),
            1 => Some(("label", &row.label)),
            _ => None,
        }
    }
    fn children_vec(&self) -> Vec<Item> {
        if self.row.is_some() {
            return vec![];
        }
        self.view
            .0
            .rows
            .iter()
            .map(|i| self.view.node(Some(*i), None))
            .collect()
    }
    fn attributes_vec(&self) -> Vec<Item> {
        if self.row.is_none() || self.attribute.is_some() {
            return vec![];
        }
        (0..2).map(|i| self.view.node(self.row, Some(i))).collect()
    }
}
impl QueryItemView for DatalistNode {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        DATALIST_CONSUMER_IDENTITY
    }
    fn identity(&self) -> String {
        format!(
            "cem:datalist:{:p}:{:?}:{:?}",
            Arc::as_ptr(&self.view.0),
            self.row,
            self.attribute
        )
    }
    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Node
    }
    fn source_map(&self) -> Option<SourceMapStack> {
        self.source().and_then(Item::source_map)
    }
    fn parent(&self, _: QueryContextScope) -> Result<Option<Item>, QueryNodeAccessError> {
        Ok(if self.attribute.is_some() {
            Some(self.view.node(self.row, None))
        } else {
            self.row.map(|_| self.view.root())
        })
    }
    fn children(
        &self,
        _: QueryContextScope,
    ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
        Ok(Box::new(self.children_vec().into_iter().map(Ok)))
    }
    fn attributes(
        &self,
        _: QueryContextScope,
    ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
        Ok(Box::new(self.attributes_vec().into_iter().map(Ok)))
    }
    fn text_fragments(
        &self,
        _: QueryContextScope,
    ) -> Result<QueryNodeTextIterator<'_>, QueryNodeAccessError> {
        Ok(Box::new(std::iter::empty()))
    }
    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let text = |s: &str| vec![Item::Atomic(AtomValue::String(s.into()))];
        if name == "id" {
            return Some(text(&self.identity()));
        }
        if let Some(index) = self.attribute {
            let (key, value) = self.attribute_value(index)?;
            return match name {
                "kind" => Some(text("attribute")),
                "name" => Some(text(key)),
                "namespace" => Some(text("")),
                "value" | "values" => Some(text(value)),
                "parent" => Some(vec![self.view.node(self.row, None)]),
                _ => None,
            };
        }
        match name {
            "kind" => Some(text("element")),
            "name" => Some(text(if self.row.is_some() {
                "option"
            } else {
                "datalist"
            })),
            "namespace" => Some(text(DATALIST_NAMESPACE)),
            "source" => Some(self.source().cloned().into_iter().collect()),
            "children" => Some(self.children_vec()),
            "attributes" => Some(self.attributes_vec()),
            "parent" => Some(self.row.map(|_| self.view.root()).into_iter().collect()),
            _ => None,
        }
    }
}
