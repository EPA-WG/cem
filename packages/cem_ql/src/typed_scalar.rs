//! Shared immutable view for scalar values whose declared datatype is richer
//! than the query engine's atom. Only checked preparation/conversion paths construct it.
use crate::eval::{AtomValue, Item, QueryItemView, QueryItemViewKind};
use cem_ml::{
    schema::{datatype_validation::ScalarRepresentation, document_model::TypedAttributeValue},
    source_map::SourceMapStack,
};

#[derive(Debug, Clone)]
struct TypedValue {
    value: TypedAttributeValue,
    source: Option<SourceMapStack>,
}
impl QueryItemView for TypedValue {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.typed-atomic"
    }
    fn identity(&self) -> String {
        format!("{}:{}", self.value.datatype, self.value.lexical)
    }
    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Atomic
    }
    fn atom(&self) -> Option<AtomValue> {
        Some(match self.value.datatype.as_str() {
            "integer" => AtomValue::from_integer_lexical(&self.value.lexical),
            _ => AtomValue::String(self.value.lexical.clone()),
        })
    }
    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let value = match name {
            "datatype" => &self.value.datatype,
            "value" => &self.value.lexical,
            _ => return None,
        };
        Some(vec![Item::Atomic(AtomValue::String(value.clone()))])
    }
    fn source_map(&self) -> Option<SourceMapStack> {
        self.source.clone()
    }
}
/// The caller has checked the primitive lexical value. Construct its native
/// representation without invoking normalization or a registered capability.
pub(crate) fn from_typed_value(value: TypedAttributeValue, source: Option<SourceMapStack>) -> Item {
    match value.datatype.as_str() {
        "integer" => match value.lexical.parse::<i64>() {
            Ok(value) => Item::Atomic(AtomValue::Integer(value)),
            Err(_) => Item::native(TypedValue { value, source }),
        },
        "number" | "decimal" => Item::Atomic(AtomValue::Decimal(value.lexical)),
        "boolean" => Item::Atomic(AtomValue::Boolean(value.lexical == "true")),
        "string" => Item::Atomic(AtomValue::String(value.lexical)),
        _ => Item::native(TypedValue { value, source }),
    }
}
/// Type identity is obtained from our immutable view, never user-facing fields
/// or a representation-id string that an unrelated native view can imitate.
pub(crate) fn representation(value: &Item) -> Option<ScalarRepresentation> {
    let typed = value.view()?.downcast_ref::<TypedValue>()?;
    (typed.value.datatype == "integer").then_some(ScalarRepresentation::Integer)
}
pub(crate) fn integer_lexical(value: &Item) -> Option<&str> {
    let typed = value.view()?.downcast_ref::<TypedValue>()?;
    (typed.value.datatype == "integer").then_some(typed.value.lexical.as_str())
}

pub(crate) fn compare_integers(left: &Item, right: &Item) -> Option<Option<std::cmp::Ordering>> {
    let left_wide = integer_lexical(left);
    let right_wide = integer_lexical(right);
    if left_wide.is_none() && right_wide.is_none() {
        return None;
    }
    let lexical = |wide: Option<&str>, item: &Item| -> Option<String> {
        // Owned text keeps the lifetime independent of the caller's temporary atom.
        wide.map(|text| text.to_owned())
            .or_else(|| match item.atom() {
                Some(AtomValue::Integer(value)) => Some(value.to_string()),
                _ => None,
            })
    };
    Some(
        match (lexical(left_wide, left), lexical(right_wide, right)) {
            (Some(left), Some(right)) => {
                cem_ml::schema::document_model::shipped_datatypes::compare_integer_lexical(
                    &left, &right,
                )
            }
            _ => None,
        },
    )
}
