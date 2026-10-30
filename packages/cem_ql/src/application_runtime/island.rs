//! Native island admission with explicit domain and live-state gates.
#[path = "island_domains.rs"]
mod domains;
#[path = "island_payload.rs"]
mod payload;
use crate::{
    eval::{portable::decode_values_with_control, AtomValue, Item, ItemStream},
    render::TemplateData,
};
use cem_ml::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
use std::collections::BTreeMap;

pub const VERSION: &str = "0.2.0";
pub const NAMESPACE: &str = "https://cem.dev/ns/runtime/data-island";
pub const PARTS: &[(&str, &str)] = &[
    ("data", "https://cem.dev/ns/runtime/hydration-data"),
    (
        "attributes",
        "https://cem.dev/ns/runtime/instance-attributes",
    ),
    ("dataset", "https://cem.dev/ns/runtime/instance-dataset"),
    ("payload", "https://cem.dev/ns/runtime/instance-payload"),
    ("slices", "https://cem.dev/ns/runtime/instance-slices"),
    ("resources", "https://cem.dev/ns/runtime/resource-state"),
    ("form-state", "https://cem.dev/ns/runtime/form-state"),
    (
        "validation-state",
        "https://cem.dev/ns/runtime/validation-state",
    ),
    ("event-state", "https://cem.dev/ns/runtime/event-state"),
];
const INVALID: &str = "compiled declaration native island refused";

fn has_source(item: &Item) -> bool {
    item.view()
        .and_then(|view| view.provenance())
        .is_some_and(|source| {
            source.source_key.is_some_and(|key| !key.is_empty())
                && source.source_uri.is_some_and(|uri| {
                    url::Url::parse(&uri).is_ok_and(|uri| !uri.cannot_be_a_base())
                })
        })
        && item
            .source_map()
            .is_some_and(|source| !source.frames.is_empty())
}

fn field(item: &Item, name: &str) -> Option<String> {
    match item.view()?.field(name)?.as_slice() {
        [value] => match value.atom()? {
            AtomValue::String(text) => Some(text),
            _ => None,
        },
        _ => None,
    }
}
fn attrs(item: &Item) -> Result<BTreeMap<String, String>, String> {
    let mut result = BTreeMap::new();
    for attribute in item
        .view()
        .ok_or(INVALID)?
        .field("attributes")
        .ok_or(INVALID)?
    {
        if field(&attribute, "namespace").as_deref() == Some("http://www.w3.org/2000/xmlns/") {
            continue;
        }
        if field(&attribute, "kind").as_deref() != Some("attribute")
            || field(&attribute, "namespace").as_deref() != Some("")
        {
            return Err(INVALID.into());
        }
        let name = field(&attribute, "name").ok_or(INVALID)?;
        let view = attribute.view().ok_or(INVALID)?;
        let values = view
            .field("values")
            .or_else(|| view.field("value"))
            .ok_or(INVALID)?;
        let [value] = values.as_slice() else {
            return Err(INVALID.into());
        };
        let Some(AtomValue::String(value)) = value.atom() else {
            return Err(INVALID.into());
        };
        if result.insert(name, value).is_some() {
            return Err(INVALID.into());
        }
    }
    Ok(result)
}
fn children(item: &Item) -> Result<Vec<Item>, String> {
    item.view()
        .and_then(|view| view.field("children"))
        .ok_or(INVALID.into())
}
fn elements(item: &Item) -> Result<Vec<Item>, String> {
    children(item)?
        .into_iter()
        .filter_map(|child| match field(&child, "kind").as_deref() {
            Some("element") => Some(Ok(child)),
            Some("text" | "whitespace")
                if field(&child, "value").is_some_and(|value| value.trim().is_empty()) =>
            {
                None
            }
            Some("comment") => None,
            _ => Some(Err(INVALID.into())),
        })
        .collect()
}
fn is_element(item: &Item, name: &str, namespace: &str) -> bool {
    field(item, "kind").as_deref() == Some("element")
        && field(item, "name").as_deref() == Some(name)
        && field(item, "namespace").as_deref() == Some(namespace)
}

pub(super) fn bind(
    bytes: &[u8],
    instance_id: &str,
    control: &OperationControl,
) -> Result<TemplateData, String> {
    if !bytes.starts_with(b"CEMV\x03") {
        return Err(INVALID.into());
    }
    let values = decode_values_with_control(
        bytes,
        &super::inputs::LIMITS,
        control,
        ROOT_EXECUTION_SCOPE_ID,
    )
    .map_err(|_| INVALID)?;
    let [root] = values.items.as_slice() else {
        return Err(INVALID.into());
    };
    let root = if field(root, "kind").as_deref() == Some("document") {
        let roots = elements(root)?;
        let [root] = roots.as_slice() else {
            return Err(INVALID.into());
        };
        root.clone()
    } else {
        root.clone()
    };
    if !has_source(&root)
        || !is_element(&root, "context-root", NAMESPACE)
        || attrs(&root)? != BTreeMap::from([("version".into(), VERSION.into())])
    {
        return Err(INVALID.into());
    }
    let parts = elements(&root)?;
    if parts.len() != PARTS.len()
        || parts
            .iter()
            .zip(PARTS)
            .any(|(item, (name, ns))| !is_element(item, name, ns))
    {
        return Err(INVALID.into());
    }
    for (index, part) in parts.iter().enumerate() {
        if index != 3 && !attrs(part)?.is_empty() {
            return Err(INVALID.into());
        }
        if !matches!(index, 0 | 1 | 2 | 3 | 4) && !elements(part)?.is_empty() {
            return Err(INVALID.into());
        }
    }
    let mut hydration = BTreeMap::new();
    for item in elements(&parts[0])? {
        if !is_element(&item, "field", PARTS[0].1) {
            return Err(INVALID.into());
        }
        let attrs = attrs(&item)?;
        if attrs.len() != 2 || attrs.get("kind").map(String::as_str) != Some("string") {
            return Err(INVALID.into());
        }
        let name = attrs.get("name").ok_or(INVALID)?;
        let mut text = String::new();
        for child in children(&item)? {
            if !matches!(
                field(&child, "kind").as_deref(),
                Some("text" | "whitespace" | "cdata")
            ) {
                return Err(INVALID.into());
            }
            text.push_str(&field(&child, "value").ok_or(INVALID)?);
        }
        if hydration.insert(name.clone(), text).is_some() {
            return Err(INVALID.into());
        }
    }
    if hydration
        != BTreeMap::from([
            ("version".into(), VERSION.into()),
            ("instanceId".into(), instance_id.into()),
            ("outputTarget".into(), "light-dom".into()),
        ])
    {
        return Err(INVALID.into());
    }
    if attrs(&parts[3])?
        != BTreeMap::from([
            ("content-type".into(), "text/html".into()),
            ("schema".into(), "https://cem.dev/ns/data/html/1".into()),
        ])
    {
        return Err(INVALID.into());
    }
    let payload = children(&parts[3])?;
    let slots = self::payload::slots(&payload, control)?;
    let mut data = TemplateData::default().with_binding("island", ItemStream::once(root));
    data.bindings.insert(
        "instanceID".into(),
        ItemStream::once(Item::Atomic(AtomValue::String(instance_id.into()))),
    );
    data.bindings.insert(
        "datadom".into(),
        ItemStream::once(Item::Record(BTreeMap::from([
            ("attributes".into(), vec![Item::Record(BTreeMap::new())]),
            ("slices".into(), vec![Item::Record(BTreeMap::new())]),
            ("payload".into(), vec![parts[3].clone()]),
            ("slots".into(), vec![Item::Record(slots)]),
        ]))),
    );
    domains::bind(&mut data, &parts, control)?;
    Ok(data)
}

#[cfg(test)]
#[path = "island_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "island_payload_tests.rs"]
mod payload_tests;
