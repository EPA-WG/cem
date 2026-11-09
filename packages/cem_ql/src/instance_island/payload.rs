//! Review the rendered payload closure without rewriting its retained source.
use super::*;
const HTML: &str = cem_ml::schema::registry::HTML_NAMESPACE_URI;
fn check(work: &mut usize, control: &OperationControl) -> Result<(), String> {
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|_| INVALID)?;
    *work = work.saturating_add(1);
    if *work > super::LIMITS.max_values {
        return Err(INVALID.into());
    }
    Ok(())
}
fn targets(item: &Item) -> Result<Vec<Item>, String> {
    item.view()
        .and_then(|view| view.field("targets"))
        .ok_or(INVALID.into())
}
pub(super) fn slots(
    payload: &[Item],
    control: &OperationControl,
) -> Result<BTreeMap<String, Vec<Item>>, String> {
    // The shared CEMV decoder has already checked the complete graph's edges,
    // DAG, depth and expanded work. Review every rendered target, including
    // targets below nested HTML, before publishing any slot projection.
    let mut pending = payload.to_vec();
    let mut work = 0;
    while let Some(item) = pending.pop() {
        check(&mut work, control)?;
        if !has_source(&item) {
            return Err(INVALID.into());
        }
        match field(&item, "kind").as_deref() {
            Some("reference") => pending.extend(targets(&item)?),
            Some("element") => {
                let namespace = field(&item, "namespace").ok_or(INVALID)?;
                if namespace != HTML {
                    validate_foreign(&item, &namespace, &field(&item, "name").ok_or(INVALID)?)?;
                    pending.extend(children(&item)?);
                    continue;
                }
                let attributes = attrs(&item)?;
                let tag = field(&item, "name").ok_or(INVALID)?;
                if tag != tag.to_ascii_lowercase()
                    || attributes
                        .keys()
                        .any(|name| *name != name.to_ascii_lowercase())
                {
                    return Err(INVALID.into());
                }
                if (matches!(tag.as_str(), "input" | "textarea")
                    && attributes.get("type").is_some_and(|v| {
                        matches!(v.to_ascii_lowercase().as_str(), "password" | "file")
                    }))
                    || attributes.get("autocomplete").is_some_and(|v| {
                        v.split_ascii_whitespace().any(|part| {
                            matches!(
                                part.to_ascii_lowercase().as_str(),
                                "one-time-code" | "current-password" | "new-password"
                            )
                        })
                    })
                {
                    return Err(INVALID.into());
                }
                pending.extend(children(&item)?);
            }
            Some("text" | "whitespace" | "cdata" | "raw-text" | "comment") => {}
            _ => return Err(INVALID.into()),
        }
    }
    index_slots(payload, control)
}

pub(super) fn validate_foreign(item: &Item, namespace: &str, name: &str) -> Result<(), String> {
    let attributes = expanded_attrs(item)?;
    let pairs = attributes
        .iter()
        .map(|((namespace, name), value)| (namespace.as_str(), name.as_str(), value.as_str()))
        .collect::<Vec<_>>();
    cem_ml::validation::html::validate_passive_foreign_projection(
        namespace,
        name,
        &pairs,
        children(item)?.iter().any(|item| {
            !matches!(
                field(item, "kind").as_deref(),
                Some("text" | "whitespace" | "cdata" | "raw-text" | "comment")
            ) || field(item, "value").is_some_and(|text| text.contains('<'))
        }),
    )
    .map_err(|_| INVALID.into())
}

pub(crate) fn index_slots(
    payload: &[Item],
    control: &OperationControl,
) -> Result<BTreeMap<String, Vec<Item>>, String> {
    // Only direct rendered roots determine slots. Preserve repetitions and
    // encounter order; nested references stay inside their source elements.
    // Original reference occurrences remain visible through datadom.payload.
    let mut pending = payload.iter().rev().cloned().collect::<Vec<_>>();
    let mut slots = BTreeMap::<String, Vec<Item>>::new();
    let mut work = 0;
    while let Some(item) = pending.pop() {
        check(&mut work, control)?;
        if field(&item, "kind").as_deref() == Some("reference") {
            pending.extend(targets(&item)?.into_iter().rev());
            continue;
        }
        let name = if field(&item, "kind").as_deref() == Some("element") {
            slot_name(&item)?
        } else {
            String::new()
        };
        slots.entry(name).or_default().push(item);
    }
    Ok(slots)
}

fn slot_name(item: &Item) -> Result<String, String> {
    for attribute in item
        .view()
        .and_then(|v| v.field("attributes"))
        .ok_or(INVALID)?
    {
        if field(&attribute, "name").as_deref() != Some("slot")
            || field(&attribute, "namespace").as_deref() != Some("")
        {
            continue;
        }
        let view = attribute.view().ok_or(INVALID)?;
        let values = view
            .field("values")
            .or_else(|| view.field("value"))
            .ok_or(INVALID)?;
        if let [value] = values.as_slice() {
            if let Some(AtomValue::String(value)) = value.atom() {
                return Ok(value);
            }
        }
        return Err("Native slot name requires one string value".into());
    }
    Ok(String::new())
}

/// Source templates keep their original native HTML owner. A CEMV graph with
/// matching strings/provenance is not a replacement source admission grant.
pub(crate) fn source_payload(
    payload: &[Item],
    control: &OperationControl,
    admitted: Option<&std::sync::Arc<cem_ml::import::element_tree::RetainedCemPayloadSource>>,
) -> Result<std::sync::Arc<cem_ml::import::element_tree::RetainedCemPayloadSource>, String> {
    let admitted = admitted.ok_or(INVALID)?;
    let mut pending = payload.iter().rev().cloned().collect::<Vec<_>>();
    let mut roots = vec![];
    let mut work = 0;
    while let Some(item) = pending.pop() {
        check(&mut work, control)?;
        if field(&item, "kind").as_deref() == Some("reference") {
            pending.extend(targets(&item)?.into_iter().rev());
        } else {
            roots.push(item);
        }
    }
    let [template] = roots.as_slice() else {
        return Err(INVALID.into());
    };
    if !is_element(template, "template", HTML) {
        return Err(INVALID.into());
    }
    let attributes = attrs(template)?;
    if attributes.get("type").map(String::as_str) != Some("text/cem-ml")
        || attributes.iter().any(|(name, value)| {
            name != "type"
                && (name != "data-cem-schema"
                    || value != cem_ml::schema::registry::CEM_ML_SCHEMA_URI)
        })
    {
        return Err(INVALID.into());
    }
    let native = crate::eval::retained_cem_node(template).ok_or(INVALID)?;
    if !admitted.matches_canonical_source(native.owner(), native.node_id())
        || !admitted.matches_operation(control)
    {
        return Err(INVALID.into());
    }
    Ok(admitted.clone())
}
