//! Portable value artifacts cross the worker boundary; handles stay WASM-local.
use super::*;
use crate::eval::{
    output::output_attribute,
    portable::{decode_values, encode_values},
    ItemStream,
};
use cem_ml::value::artifact::CemValueArtifactLimits;
use std::collections::BTreeMap;

thread_local! {
    static INPUTS: RefCell<BTreeMap<u32, ItemStream>> = const { RefCell::new(BTreeMap::new()) };
    // Only the immediately preceding render can be taken. Ignored results never
    // accumulate retained output artifacts.
    static OUTPUT: RefCell<Option<(u32, Vec<u8>)>> = const { RefCell::new(None) };
    static NEXT: RefCell<u32> = const { RefCell::new(0) };
}
fn next_id() -> Result<u32, String> {
    NEXT.with(|next| {
        let mut next = next.borrow_mut();
        *next = next
            .checked_add(1)
            .ok_or("Native value handle space exhausted")?;
        Ok(*next)
    })
}

#[wasm_bindgen(js_name = "importNativeValueArtifact")]
pub fn import(bytes: &[u8], limits_json: &str) -> Result<u32, JsValue> {
    let limits: CemValueArtifactLimits =
        serde_json::from_str(limits_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let values = decode_values(bytes, &limits).map_err(|e| JsValue::from_str(&e))?;
    let id = next_id().map_err(|e| JsValue::from_str(&e))?;
    INPUTS.with(|inputs| inputs.borrow_mut().insert(id, values));
    Ok(id)
}

#[wasm_bindgen(js_name = "disposeNativeValueArtifact")]
pub fn dispose(id: u32) -> bool {
    INPUTS.with(|inputs| inputs.borrow_mut().remove(&id).is_some())
}

#[wasm_bindgen(js_name = "takeRenderValueArtifact")]
pub fn take(id: u32) -> Result<Vec<u8>, JsValue> {
    OUTPUT.with(|output| {
        let mut output = output.borrow_mut();
        if output.as_ref().is_some_and(|(retained, _)| *retained == id) {
            Ok(output.take().expect("checked output").1)
        } else {
            Err(JsValue::from_str(
                "Native render value artifact is no longer retained",
            ))
        }
    })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Binding {
    name: String,
    artifact_id: u32,
    index: usize,
}

pub(super) fn bind(data: &mut TemplateData, bindings_json: &str) -> Result<(), String> {
    if bindings_json.len() > 128 * 1024 {
        return Err("Native value binding metadata limit exceeded".into());
    }
    let bindings: Vec<Binding> = serde_json::from_str(bindings_json).map_err(|e| e.to_string())?;
    let mut names = std::collections::BTreeSet::new();
    for binding in bindings {
        if !names.insert(binding.name.clone()) {
            return Err("Duplicate native attribute binding".into());
        }
        let value = INPUTS
            .with(|inputs| {
                inputs
                    .borrow()
                    .get(&binding.artifact_id)
                    .and_then(|values| values.items.get(binding.index))
                    .cloned()
            })
            .ok_or("Unknown native value artifact or root index")?;
        let name = value
            .view()
            .and_then(|v| v.field("name"))
            .and_then(|items| items.first().and_then(crate::eval::Item::atom));
        if name != Some(crate::eval::AtomValue::String(binding.name)) {
            return Err("Native attribute name does not match its binding".into());
        }
        data.bind_native_attribute(value)?;
    }
    Ok(())
}

pub(super) fn attribute(
    attribute: &crate::render::RenderPlanAttribute,
    values: &mut ItemStream,
) -> Option<usize> {
    if attribute.contract.is_none()
        && matches!(attribute.value_stream.items.as_slice(),
        [crate::eval::Item::Atomic(crate::eval::AtomValue::String(value))] if value == &attribute.value)
    {
        return None;
    }
    let index = values.items.len();
    values.items.push(output_attribute(attribute.clone()));
    Some(index)
}

pub(super) fn publish(
    values: &ItemStream,
    limits: &CemValueArtifactLimits,
) -> Result<Option<(u32, String)>, String> {
    OUTPUT.with(|output| output.borrow_mut().take());
    if values.items.is_empty() {
        return Ok(None);
    }
    let bytes = encode_values(values, limits)?;
    let hash = cem_ml::content_cache::ContentHash::from_blake3(&bytes).header_value();
    let id = next_id()?;
    OUTPUT.with(|output| *output.borrow_mut() = Some((id, bytes)));
    Ok(Some((id, hash)))
}

pub(super) fn limits(input: &str) -> Result<CemValueArtifactLimits, String> {
    if input.is_empty() {
        return Ok(CemValueArtifactLimits::default());
    }
    serde_json::from_str(input).map_err(|e| e.to_string())
}
