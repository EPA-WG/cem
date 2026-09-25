//! Pure immutable URL parameter values and their typed CEM-QL boundary.
//!
//! This core does not register query functions or perform host URL resolution.
use std::{collections::BTreeMap, fmt};

use crate::eval::{AtomValue, Item, QueryItemViewKind};

/// A malformed typed input; the evaluator attaches its source range and diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamsTypeError(pub &'static str);

impl fmt::Display for ParamsTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for ParamsTypeError {}

type Result<T> = std::result::Result<T, ParamsTypeError>;

/// An ordered, duplicate-preserving list of decoded names and values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UrlParams(Vec<(String, String)>);

impl UrlParams {
    /// Accept omitted input, form text, one mapping, or a stream of pair arrays.
    /// Empty input is the empty pair stream; arrays are never implicitly flattened.
    pub fn from_initializer(input: Option<&[Item]>) -> Result<Self> {
        let input = input.unwrap_or_default();
        if let [item] = input {
            if let Ok(value) = string(item) {
                return Ok(Self(
                    form_urlencoded::parse(value.strip_prefix('?').unwrap_or(&value).as_bytes())
                        .into_owned()
                        .collect(),
                ));
            }
            if is_record(item) {
                let fields = fields(item)?;
                let pairs = fields
                    .into_iter()
                    .map(|(name, value)| Ok((name, singleton_string(&value)?)))
                    .collect::<Result<Vec<_>>>()?;
                return Ok(Self(pairs));
            }
        }
        let mut pairs = Vec::with_capacity(input.len());
        for item in input {
            let members = match item {
                Item::Array(members) => members.clone(),
                Item::Native(view) if view.kind() == QueryItemViewKind::Array => view
                    .members()
                    .ok_or(ParamsTypeError("array members are unavailable"))?,
                _ => return Err(ParamsTypeError("expected a stream of two-string arrays")),
            };
            let [name, value] = members.as_slice() else {
                return Err(ParamsTypeError(
                    "parameter pair must contain exactly two strings",
                ));
            };
            pairs.push((string(name)?, string(value)?));
        }
        Ok(Self(pairs))
    }

    /// Validate the exact entry-record stream consumed by query operations.
    pub fn from_entries(input: &[Item]) -> Result<Self> {
        let mut pairs = Vec::with_capacity(input.len());
        for item in input {
            let mut fields = fields(item)?;
            if fields.len() != 2 || !fields.contains_key("name") || !fields.contains_key("value") {
                return Err(ParamsTypeError(
                    "entry requires exactly name and value fields",
                ));
            }
            let name = singleton_string(&fields.remove("name").expect("checked field"))?;
            let value = singleton_string(&fields.remove("value").expect("checked field"))?;
            pairs.push((name, value));
        }
        Ok(Self(pairs))
    }

    /// Produce ordinary CEM-QL records, with no serialization boundary.
    pub fn to_entries(&self) -> Vec<Item> {
        self.0
            .iter()
            .map(|(name, value)| {
                Item::Record(BTreeMap::from([
                    (
                        "name".into(),
                        vec![Item::Atomic(AtomValue::String(name.clone()))],
                    ),
                    (
                        "value".into(),
                        vec![Item::Atomic(AtomValue::String(value.clone()))],
                    ),
                ]))
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(name, _)| name.as_str())
    }
    pub fn values(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(_, value)| value.as_str())
    }
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
    pub fn get_all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> {
        self.0
            .iter()
            .filter(move |(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
    pub fn has(&self, name: &str, value: Option<&str>) -> bool {
        self.0
            .iter()
            .any(|(key, found)| key == name && value.is_none_or(|value| value == found))
    }
    pub fn append(&self, name: &str, value: &str) -> Self {
        let mut result = self.clone();
        result.0.push((name.into(), value.into()));
        result
    }
    pub fn delete(&self, name: &str, value: Option<&str>) -> Self {
        Self(
            self.0
                .iter()
                .filter(|(key, found)| !(key == name && value.is_none_or(|value| value == found)))
                .cloned()
                .collect(),
        )
    }
    pub fn set(&self, name: &str, value: &str) -> Self {
        let mut found = false;
        let mut pairs = Vec::with_capacity(self.len());
        for (key, old_value) in &self.0 {
            if key == name {
                if !found {
                    pairs.push((key.clone(), value.into()));
                    found = true;
                }
            } else {
                pairs.push((key.clone(), old_value.clone()));
            }
        }
        if !found {
            pairs.push((name.into(), value.into()));
        }
        Self(pairs)
    }
    pub fn sorted(&self) -> Self {
        let mut result = self.clone();
        // Stable sort: UTF-16 code units differ from Rust's scalar/UTF-8 order.
        result
            .0
            .sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
        result
    }
    pub fn serialize(&self) -> String {
        form_urlencoded::Serializer::new(String::new())
            .extend_pairs(&self.0)
            .finish()
    }
}

fn string(item: &Item) -> Result<String> {
    match item {
        Item::Atomic(AtomValue::String(value)) => Ok(value.clone()),
        Item::Native(view) if view.kind() == QueryItemViewKind::Atomic => match view.atom() {
            Some(AtomValue::String(value)) => Ok(value),
            _ => Err(ParamsTypeError("expected one string atomic value")),
        },
        _ => Err(ParamsTypeError("expected one string atomic value")),
    }
}
fn singleton_string(items: &[Item]) -> Result<String> {
    match items {
        [item] => string(item),
        _ => Err(ParamsTypeError("expected exactly one string item")),
    }
}
fn is_record(item: &Item) -> bool {
    matches!(item, Item::Record(_))
        || matches!(item, Item::Native(view) if view.kind() == QueryItemViewKind::Record)
}
fn fields(item: &Item) -> Result<BTreeMap<String, Vec<Item>>> {
    match item {
        Item::Record(fields) => Ok(fields.clone()),
        Item::Native(view) if view.kind() == QueryItemViewKind::Record => {
            let mut fields = BTreeMap::new();
            for (name, value) in view
                .fields()
                .ok_or(ParamsTypeError("record fields are unavailable"))?
            {
                if fields.insert(name, value).is_some() {
                    return Err(ParamsTypeError("record contains duplicate field names"));
                }
            }
            Ok(fields)
        }
        _ => Err(ParamsTypeError("expected a record")),
    }
}
