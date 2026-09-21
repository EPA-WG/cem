//! Host value I/O. External parsing and JSON export remain owned by CEM-ML.
use crate::eval::{imported_cem_tree, portable::encode_values, ItemStream};
use cem_ml::value::artifact::{CemValueArtifactLimits, CemValueGraph};

pub fn import_document(
    bytes: &[u8],
    content_type: &str,
    uri: &str,
    limits: &CemValueArtifactLimits,
) -> Result<Vec<u8>, String> {
    if bytes.len() > limits.max_bytes {
        return Err("Native document byte limit exceeded".into());
    }
    let tree = cem_ml::import::import_data_bytes(bytes, content_type, "cem", uri)?;
    encode_values(&ItemStream::once(imported_cem_tree(tree)), limits)
}

/// A native event attribute transports its value sequence. Attribute metadata
/// must be checked before that wrapper is removed; arbitrary roots stay values.
pub fn export_json(
    bytes: &[u8],
    index: usize,
    attribute: Option<&str>,
    limits: &CemValueArtifactLimits,
) -> Result<Option<String>, String> {
    let mut graph = CemValueGraph::decode(bytes, limits)?;
    let root = *graph
        .roots
        .get(index)
        .ok_or("Unknown native CEM root index")?;
    graph.roots = if let Some(name) = attribute {
        let record = &graph.records[root as usize];
        if record.kind != "attribute" || record.name != name || !record.has_native_content() {
            return Err("Native event attribute does not match its binding".into());
        }
        record.values.clone()
    } else {
        vec![root]
    };
    // References preserve identity in transport, but emptiness/null retain their
    // storage meaning after explicit dereferencing. A document containing the
    // generic-data null element remains the JSON value `null`.
    let mut pending: Vec<_> = graph.roots.iter().rev().map(|id| (*id, 0)).collect();
    let mut roots = Vec::new();
    let mut visits = 0;
    while let Some((id, depth)) = pending.pop() {
        visits += 1;
        if visits > limits.max_values || depth > limits.max_depth {
            return Err("Native value reference limit exceeded".into());
        }
        let record = &graph.records[id as usize];
        if record.kind == "reference" {
            pending.extend(record.targets.iter().rev().map(|id| (*id, depth + 1)));
        } else {
            roots.push(id);
        }
    }
    if roots.is_empty()
        || matches!(roots.as_slice(), [id]
        if graph.records[*id as usize].kind == "atomic" && graph.records[*id as usize].datatype == "null")
    {
        return Ok(None);
    }
    cem_ml::value::json::write_json(&graph, limits).map(Some)
}

/// Validate the native wrapper before exposing its authoritative value sequence.
pub fn attribute_values(value: &crate::eval::Item, name: &str) -> Result<ItemStream, String> {
    use crate::eval::{AtomValue, Item};
    let view = value.view().ok_or("A native event attribute is required")?;
    let field = |key| {
        view.field(key)
            .and_then(|items| items.first().and_then(Item::atom))
    };
    if field("kind") != Some(AtomValue::String("attribute".into()))
        || field("name") != Some(AtomValue::String(name.into()))
    {
        return Err("Native event attribute does not match its binding".into());
    }
    Ok(ItemStream::from_items(view.field("values").unwrap_or_else(
        || view.field("value").unwrap_or_default(),
    )))
}
