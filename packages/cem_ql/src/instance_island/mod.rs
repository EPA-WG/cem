//! Native island admission with explicit domain and live-state gates.
mod domains;
mod payload;
use crate::{
    eval::{AtomValue, Item, ItemStream},
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

/// Shared native island input ceilings, unchanged from the compiled receiver.
pub(crate) const LIMITS: cem_ml::value::artifact::CemValueArtifactLimits =
    cem_ml::value::artifact::CemValueArtifactLimits {
        max_bytes: 1024 * 1024,
        max_values: 16_384,
        max_depth: 64,
    };
pub(crate) const BOUNDS: &str = "native island bounds exceeded";
pub(crate) fn dataset_key(name: &str) -> String {
    let mut chars = name.chars().peekable();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        if c == '-' && chars.peek().is_some_and(char::is_ascii_lowercase) {
            out.push(chars.next().unwrap().to_ascii_uppercase())
        } else {
            out.push(c)
        }
    }
    out
}
pub(crate) fn name_allowed(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        && !matches!(
            name,
            "datadom"
                | "attributes"
                | "slices"
                | "dataset"
                | "slots"
                | "island"
                | "instanceID"
                | "suggestions"
                | "input"
                | "position"
        )
}

#[cfg(test)]
fn test_control() -> OperationControl {
    OperationControl::new(cem_ml::scheduler::AbortSignal::new())
}

const INVALID: &str = "compiled declaration native island refused";

pub(crate) fn has_source(item: &Item) -> bool {
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

/// Reuse the existing passive payload/source/sensitive-input checks for output roots.
pub(crate) fn admit_payload_roots(
    items: &[Item],
    control: &OperationControl,
) -> Result<(), String> {
    payload::slots(items, control).map(|_| ())
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

#[cfg(any(
    test,
    feature = "component-runtime",
    feature = "application-source-runtime"
))]
pub(crate) fn bind(
    bytes: &[u8],
    instance_id: &str,
    control: &OperationControl,
) -> Result<TemplateData, String> {
    if !bytes.starts_with(b"CEMV\x03") {
        return Err(INVALID.into());
    }
    let values = crate::eval::portable::decode_values_with_control(
        bytes,
        &LIMITS,
        control,
        ROOT_EXECUTION_SCOPE_ID,
    )
    .map_err(|_| INVALID)?;
    let [root] = values.items.as_slice() else {
        return Err(INVALID.into());
    };
    bind_root(root, instance_id, control)
}

/// Direct retained native ingress; never encodes/reloads a document.
pub(crate) fn bind_retained_root(
    root: &Item,
    instance_id: &str,
    control: &OperationControl,
) -> Result<TemplateData, String> {
    bounded_root(root, control)?;
    bind_root(root, instance_id, control)
}

fn bind_root(
    root: &Item,
    instance_id: &str,
    control: &OperationControl,
) -> Result<TemplateData, String> {
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|_| INVALID)?;
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
        if !matches!(index, 0..=4) && !elements(part)?.is_empty() {
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
    let mut payload_attrs = attrs(&parts[3])?;
    if let Some(marker) = payload_attrs.remove("data-cem-explicit-payload") {
        if marker != "true" {
            return Err(INVALID.into());
        }
    }
    if payload_attrs
        != BTreeMap::from([
            (
                "content-type".into(),
                cem_ml::schema::registry::HTML_CONTENT_TYPE.into(),
            ),
            (
                "schema".into(),
                cem_ml::schema::registry::HTML_SCHEMA_URI.into(),
            ),
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
mod tests;

#[cfg(test)]
mod payload_tests;

// Inspect the owning native value closure, excluding original-source parents.
// Reference targets retain their source arenas and count toward expanded work.
fn bounded_root(root: &Item, control: &OperationControl) -> Result<(), String> {
    let scopes = control.scope_tree();
    let policy = scopes
        .scope(ROOT_EXECUTION_SCOPE_ID)
        .ok_or(INVALID)?
        .effective_policy;
    let byte_limit = LIMITS
        .max_bytes
        .min(policy.memory_bytes.try_into().unwrap_or(usize::MAX));
    let depth_limit = LIMITS.max_depth.min(policy.stack_depth as usize);
    let mut pending = vec![(root.clone(), 0usize)];
    let mut work = 0usize;
    let mut bytes = 0usize;
    while let Some((item, depth)) = pending.pop() {
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID)?;
        work = work.saturating_add(1);
        if work > LIMITS.max_values || depth > depth_limit {
            return Err(BOUNDS.into());
        }
        if let Some(atom) = item.atom() {
            bytes = bytes.saturating_add(crate::render::item_to_string(&Item::Atomic(atom)).len());
        } else {
            let view = item.view().ok_or(INVALID)?;
            for name in ["name", "namespace", "value"] {
                if let Some(values) = view.field(name) {
                    for value in values {
                        if let Some(atom) = value.atom() {
                            bytes = bytes.saturating_add(
                                crate::render::item_to_string(&Item::Atomic(atom)).len(),
                            );
                        }
                    }
                }
            }
            for name in ["children", "attributes", "values", "targets"] {
                if let Some(values) = view.field(name) {
                    if pending
                        .len()
                        .saturating_add(values.len())
                        .saturating_add(work)
                        > LIMITS.max_values
                    {
                        return Err(BOUNDS.into());
                    }
                    pending.extend(values.into_iter().map(|value| (value, depth + 1)));
                }
            }
        }
        if let Some(map) = item.source_map() {
            bytes = bytes.saturating_add(
                map.frames
                    .len()
                    .saturating_mul(std::mem::size_of::<cem_ml::source_map::SourceMapFrame>()),
            );
            for frame in map.frames {
                if let cem_ml::source_map::TransformKind::SourceBoundary { uri } = frame.transform {
                    bytes = bytes.saturating_add(uri.len());
                }
            }
        }
        if let Some(provenance) = item.view().and_then(|v| v.provenance()) {
            bytes = bytes
                .saturating_add(provenance.source_uri.as_ref().map_or(0, String::len))
                .saturating_add(provenance.source_key.as_ref().map_or(0, String::len));
        }
        bytes = bytes.saturating_add(std::mem::size_of::<Item>());
        if bytes > byte_limit {
            return Err(BOUNDS.into());
        }
    }
    Ok(())
}
