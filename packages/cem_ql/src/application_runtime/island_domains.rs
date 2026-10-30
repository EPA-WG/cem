//! Explicit data-island value vocabulary over retained CEM nodes. These are
//! semantic control views, never JSON-shaped documents or inferred CEM nodes.
use super::*;
use crate::eval::{QueryItemView, QueryItemViewKind};
use cem_ml::{
    schema::document_model::{convert_attribute_value, AttributeModel, AttributeValueContract},
    source_map::SourceMapStack,
    value::artifact::CemValueProvenance,
};
use std::{any::Any, collections::BTreeSet};

#[derive(Debug)]
enum Value {
    Scalar(AtomValue, &'static str),
    Record(BTreeMap<String, Vec<Item>>),
    Array(Vec<Item>),
    Attribute(String, Item),
}
#[derive(Debug)]
struct DomainView {
    source: Item,
    value: Value,
}
impl QueryItemView for DomainView {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.runtime.island-domain.v1"
    }
    fn identity(&self) -> String {
        format!(
            "{:?}:{}",
            self.kind(),
            self.source.view().unwrap().identity()
        )
    }
    fn kind(&self) -> QueryItemViewKind {
        match self.value {
            Value::Scalar(..) => QueryItemViewKind::Atomic,
            Value::Record(..) => QueryItemViewKind::Record,
            Value::Array(..) => QueryItemViewKind::Array,
            Value::Attribute(..) => QueryItemViewKind::Node,
        }
    }
    fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
        match &self.value {
            Value::Record(fields) => {
                Some(fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            }
            _ => None,
        }
    }
    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let text = |value: &str| vec![Item::Atomic(AtomValue::String(value.into()))];
        match &self.value {
            Value::Record(fields) => fields.get(name).cloned(),
            Value::Scalar(_, datatype) if name == "datatype" => Some(text(datatype)),
            Value::Attribute(key, value) => match name {
                "kind" => Some(text("attribute")),
                "name" => Some(text(key)),
                "values" => Some(vec![value.clone()]),
                _ => None,
            },
            _ => None,
        }
    }
    fn members(&self) -> Option<Vec<Item>> {
        match &self.value {
            Value::Array(values) => Some(values.clone()),
            _ => None,
        }
    }
    fn atom(&self) -> Option<AtomValue> {
        match &self.value {
            Value::Scalar(value, _) => Some(value.clone()),
            _ => None,
        }
    }
    fn source_map(&self) -> Option<SourceMapStack> {
        self.source.source_map()
    }
    fn provenance(&self) -> Option<CemValueProvenance> {
        self.source.view()?.provenance()
    }
}
fn native(source: &Item, value: Value) -> Result<Item, String> {
    if !has_source(source) {
        return Err(INVALID.into());
    }
    Ok(Item::native(DomainView {
        source: source.clone(),
        value,
    }))
}
fn literal_source(item: &Item, name: &str) -> Result<Item, String> {
    item.view()
        .and_then(|v| v.field("attributes"))
        .ok_or(INVALID)?
        .into_iter()
        .find(|a| {
            field(a, "namespace").as_deref() == Some("")
                && field(a, "name").as_deref() == Some(name)
        })
        .ok_or(INVALID.into())
}
fn names(item: &Item, expected: &[&str]) -> Result<BTreeMap<String, String>, String> {
    let values = attrs(item)?;
    if values.len() != expected.len() || expected.iter().any(|name| !values.contains_key(*name)) {
        return Err(INVALID.into());
    }
    Ok(values)
}
fn text(item: &Item) -> Result<String, String> {
    let mut text = String::new();
    for child in children(item)? {
        if !matches!(
            field(&child, "kind").as_deref(),
            Some("text" | "whitespace" | "cdata")
        ) || !has_source(&child)
        {
            return Err(INVALID.into());
        }
        text.push_str(&field(&child, "value").ok_or(INVALID)?);
    }
    Ok(text)
}
fn slice(item: &Item, depth: usize, control: &OperationControl) -> Result<(String, Item), String> {
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|_| INVALID)?;
    if depth > super::super::inputs::LIMITS.max_depth || !is_element(item, "slice", PARTS[4].1) {
        return Err(INVALID.into());
    }
    let attrs = names(item, &["name", "kind"])?;
    let name = attrs["name"].clone();
    if name.is_empty() || name.len() > 256 {
        return Err(INVALID.into());
    }
    let value = match attrs["kind"].as_str() {
        "record" | "array" => {
            let mut fields = BTreeMap::new();
            let mut members = Vec::new();
            for child in elements(item)? {
                let (name, value) = slice(&child, depth + 1, control)?;
                if attrs["kind"] == "array" && name != members.len().to_string() {
                    return Err(INVALID.into());
                }
                if fields.insert(name, vec![value.clone()]).is_some() {
                    return Err(INVALID.into());
                }
                members.push(value);
            }
            if attrs["kind"] == "array" {
                Value::Array(members)
            } else {
                Value::Record(fields)
            }
        }
        "string" => Value::Scalar(AtomValue::String(text(item)?), "string"),
        "boolean" => Value::Scalar(
            AtomValue::Boolean(match text(item)?.as_str() {
                "true" => true,
                "false" => false,
                _ => return Err(INVALID.into()),
            }),
            "boolean",
        ),
        "number" => {
            let contract = AttributeValueContract {
                model: AttributeModel {
                    value_type: Some("number".into()),
                    ..Default::default()
                },
                ..Default::default()
            };
            let value = convert_attribute_value(
                &text(item)?,
                &contract,
                &item.source_map().unwrap_or_default(),
            )
            .map_err(|_| INVALID)?;
            Value::Scalar(AtomValue::Decimal(value.lexical), "decimal")
        }
        "null" if text(item)?.is_empty() => Value::Scalar(AtomValue::Null, "null"),
        _ => return Err(INVALID.into()),
    };
    Ok((name, native(item, value)?))
}
fn dataset_key(name: &str) -> String {
    let mut chars = name.chars().peekable();
    let mut result = String::new();
    while let Some(c) = chars.next() {
        if c == '-' && chars.peek().is_some_and(char::is_ascii_lowercase) {
            result.push(chars.next().unwrap().to_ascii_uppercase());
        } else {
            result.push(c);
        }
    }
    result
}
fn attribute_name(name: &str) -> bool {
    name.len() <= 256
        && name == name.to_ascii_lowercase()
        && name != "dataset"
        && (super::super::inputs::name_allowed(name)
            || name.strip_prefix("data-").is_some_and(|suffix| {
                suffix.chars().all(|c| {
                    !c.is_control()
                        && !c.is_whitespace()
                        && !matches!(c, '"' | '\'' | '<' | '>' | '/' | '=')
                })
            }))
}
pub(super) fn bind(
    data: &mut TemplateData,
    parts: &[Item],
    control: &OperationControl,
) -> Result<(), String> {
    let mut aliases = BTreeSet::new();
    let mut projected = BTreeMap::new();
    for entry in elements(&parts[1])? {
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID)?;
        if !is_element(&entry, "attribute", PARTS[1].1) || !children(&entry)?.is_empty() {
            return Err(INVALID.into());
        }
        let attrs = names(&entry, &["name", "value"])?;
        let name = &attrs["name"];
        let raw = &attrs["value"];
        if !attribute_name(name)
            || !aliases.insert(name.clone())
            || aliases.len() > 64
            || (name == "type" && matches!(raw.to_ascii_lowercase().as_str(), "password" | "file"))
            || (name == "autocomplete"
                && raw.split_ascii_whitespace().any(|token| {
                    matches!(
                        token.to_ascii_lowercase().as_str(),
                        "one-time-code" | "current-password" | "new-password"
                    )
                }))
        {
            return Err(INVALID.into());
        }
        if !has_source(&entry) {
            return Err(INVALID.into());
        }
        let source = literal_source(&entry, "value")?;
        let value = native(
            &source,
            Value::Scalar(AtomValue::String(raw.clone()), "string"),
        )?;
        if let Some(suffix) = name.strip_prefix("data-") {
            projected.insert(dataset_key(suffix), vec![value.clone()]);
        }
        data.bind_native_attribute(native(&entry, Value::Attribute(name.clone(), value))?)
            .map_err(|_| INVALID)?;
    }
    let mut asserted = BTreeMap::new();
    for entry in elements(&parts[2])? {
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID)?;
        if !is_element(&entry, "entry", PARTS[2].1)
            || !has_source(&entry)
            || !children(&entry)?.is_empty()
        {
            return Err(INVALID.into());
        }
        let attrs = names(&entry, &["key", "value"])?;
        if asserted
            .insert(attrs["key"].clone(), attrs["value"].clone())
            .is_some()
        {
            return Err(INVALID.into());
        }
    }
    if asserted.len() != projected.len()
        || projected.iter().any(|(key, values)| {
            values[0].atom() != asserted.get(key).map(|s| AtomValue::String(s.clone()))
        })
    {
        return Err(INVALID.into());
    }
    let dataset = native(&parts[2], Value::Record(projected))?;
    let [Item::Record(fields)] = data
        .bindings
        .get_mut("datadom")
        .ok_or(INVALID)?
        .items
        .as_mut_slice()
    else {
        return Err(INVALID.into());
    };
    fields.insert("dataset".into(), vec![dataset]);
    for entry in elements(&parts[4])? {
        let (name, value) = slice(&entry, 0, control)?;
        if !super::super::inputs::name_allowed(&name)
            || name == "dataset"
            || !aliases.insert(name.clone())
            || aliases.len() > 64
        {
            return Err(INVALID.into());
        }
        data.bind_native_slice(&name, ItemStream::once(value))
            .map_err(|_| INVALID)?;
    }
    Ok(())
}
