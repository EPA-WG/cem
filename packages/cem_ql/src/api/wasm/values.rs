//! Portable value artifacts cross the worker boundary; handles stay WASM-local.
use super::*;
use crate::eval::{
    output::output_attribute,
    portable::{decode_values, encode_values_with_control},
    ItemStream,
};
use cem_ml::value::artifact::CemValueArtifactLimits;
use std::collections::BTreeMap;
use crate::api::reference_transport::RetainedReferenceSource;

struct RetainedInput {
    values: ItemStream,
    // Query results retain executable source capture until their handle is disposed.
    source: Option<RetainedReferenceSource>,
}

thread_local! {
    static INPUTS: RefCell<BTreeMap<u32, RetainedInput>> = const { RefCell::new(BTreeMap::new()) };
    static SOURCES: RefCell<BTreeMap<u32, RetainedReferenceSource>> = const { RefCell::new(BTreeMap::new()) };
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
    INPUTS.with(|inputs| inputs.borrow_mut().insert(id, RetainedInput { values, source: None }));
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
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    attribute: Option<String>,
}

pub(super) fn bind(data: &mut TemplateData, bindings_json: &str) -> Result<(), String> {
    if bindings_json.len() > 128 * 1024 {
        return Err("Native value binding metadata limit exceeded".into());
    }
    let bindings: Vec<Binding> = serde_json::from_str(bindings_json).map_err(|e| e.to_string())?;
    let mut names = std::collections::BTreeSet::new();
    for binding in bindings {
        if !names.insert((binding.target.as_deref().unwrap_or("attribute").to_owned(), binding.name.clone())) {
            return Err("Duplicate native attribute binding".into());
        }
        let value = INPUTS
            .with(|inputs| {
                inputs
                    .borrow()
                    .get(&binding.artifact_id)
                    .and_then(|input| input.values.items.get(binding.index))
                    .cloned()
            })
            .ok_or("Unknown native value artifact or root index")?;
        match binding.target.as_deref().unwrap_or("attribute") {
            "slice" => {
                let values = if let Some(name) = binding.attribute {
                    crate::api::native_values::attribute_values(&value, &name)?
                } else { ItemStream::once(value) };
                data.bind_native_slice(&binding.name, values)?;
            }
            "attribute" => {
                crate::api::native_values::attribute_values(&value, &binding.name)?;
                data.bind_native_attribute(value)?;
            }
            _ => return Err("Unknown native value binding target".into()),
        }
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
    control: &cem_ml::operation_control::OperationControl,
    scope: cem_ml::operation_control::ExecutionScopeId,
) -> Result<Option<(u32, String)>, String> {
    OUTPUT.with(|output| output.borrow_mut().take());
    if values.items.is_empty() {
        return Ok(None);
    }
    let bytes = encode_values_with_control(values, limits, crate::eval::QueryContextScope(0), control, scope).map_err(|e| e.to_string())?;
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

pub(super) fn clear_output() { OUTPUT.with(|output| output.borrow_mut().take()); }

#[wasm_bindgen(js_name = "importCemDocumentValue")]
pub fn import_document(bytes: &[u8], content_type: &str, uri: &str, limits_json: &str) -> Result<Vec<u8>, JsValue> {
    let limits = limits(limits_json).map_err(|e| JsValue::from_str(&e))?;
    crate::api::native_values::import_document(bytes, content_type, uri, &limits).map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen(js_name = "exportCemJsonValue")]
pub fn export_json(bytes: &[u8], index: usize, attribute: &str, limits_json: &str) -> Result<Option<String>, JsValue> {
    let limits = limits(limits_json).map_err(|e| JsValue::from_str(&e))?;
    crate::api::native_values::export_json(bytes, index, (!attribute.is_empty()).then_some(attribute), &limits).map_err(|e| JsValue::from_str(&e))
}

// New transport failures are JSON records in the thrown string. Existing
// compatibility exports above keep their historical string errors.
fn transport_error(
    code: &str,
    kind: &str,
    message: impl ToString,
    source: Option<&cem_ml::source_map::SourceMapStack>,
) -> JsValue {
    JsValue::from_str(
        &json!({ "code": code, "kind": kind, "message": message.to_string(), "sourceMap": source })
            .to_string(),
    )
}
fn reload_limits(json: &str) -> Result<cem_ml::ast::reload::ReloadLimits, JsValue> {
    if json.is_empty() {
        return Ok(Default::default());
    }
    serde_json::from_str(json)
        .map_err(|e| transport_error("cem.reference.reload_limits", "InvalidLimits", e, None))
}
fn retained_source(id: u32) -> Result<RetainedReferenceSource, JsValue> {
    SOURCES
        .with(|sources| sources.borrow().get(&id).cloned())
        .ok_or_else(|| {
            transport_error(
                "cem.reference.source_handle",
                "UnknownHandle",
                "Unknown reference source handle",
                None,
            )
        })
}
fn retain_source(source: RetainedReferenceSource) -> Result<u32, JsValue> {
    let id =
        next_id().map_err(|e| transport_error("cem.reference.source_handle", "Limit", e, None))?;
    SOURCES.with(|sources| sources.borrow_mut().insert(id, source));
    Ok(id)
}
#[wasm_bindgen(js_name = "parseReferenceSource")]
pub fn parse_reference_source(
    bytes: &[u8],
    content_type: &str,
    uri: &str,
    limits_json: &str,
) -> Result<u32, JsValue> {
    let source =
        RetainedReferenceSource::parse(bytes, content_type, uri, reload_limits(limits_json)?)
            .map_err(|e| transport_error("cem.reference.reload", e.kind_name(), &e, None))?;
    retain_source(source)
}
#[wasm_bindgen(js_name = "importReferenceReloadBundle")]
pub fn import_reference_bundle(
    bytes: &[u8],
    primary_source_id: u32,
    limits_json: &str,
) -> Result<u32, JsValue> {
    let source =
        RetainedReferenceSource::reload(bytes, primary_source_id, reload_limits(limits_json)?)
            .map_err(|e| transport_error("cem.reference.reload", e.kind_name(), &e, None))?;
    retain_source(source)
}
#[wasm_bindgen(js_name = "exportReferenceReloadBundle")]
pub fn export_reference_bundle(id: u32, limits_json: &str) -> Result<Vec<u8>, JsValue> {
    retained_source(id)?
        .export_bundle(reload_limits(limits_json)?)
        .map_err(|e| transport_error("cem.reference.reload_export", e.kind_name(), &e, None))
}
#[wasm_bindgen(js_name = "inspectReferenceSource")]
pub fn inspect_reference_source(id: u32) -> Result<String, JsValue> {
    let source = retained_source(id)?;
    Ok(json!({ "sourceUri": source.ingress().source().source_uri(), "primarySourceId": source.ingress().primary_source().0,
        "lexicalReady": source.require_lexical().is_ok(), "pendingDependency": source.require_lexical().err().map(|e| format!("{e:?}")) }).to_string())
}
#[wasm_bindgen(js_name = "disposeReferenceSource")]
pub fn dispose_reference_source(id: u32) -> bool {
    SOURCES.with(|sources| sources.borrow_mut().remove(&id).is_some())
}

/// Inert native query ingress. No runtime grant or reference evaluator is inferred.
#[wasm_bindgen(js_name = "queryReferenceSource")]
pub fn query_reference_source(id: u32, expression: &str, query_uri: &str) -> Result<u32, JsValue> {
    let source = retained_source(id)?;
    let context = crate::api::StandaloneExpressionContext {
        source_uri: Some(query_uri.into()),
        ..Default::default()
    };
    let evaluated = source.evaluate(expression, &context).map_err(|e|
        JsValue::from_str(&json!({ "code": e.code, "kind": "QueryPreparation", "message": e.message, "diagnostics": e.diagnostics }).to_string()))?;
    if let Some(error) = &evaluated.result.error {
        return Err(transport_error(
            "cem.query.execution",
            "ExecutionFailure",
            format!("{error:?}"),
            None,
        ));
    }
    let result_id = next_id().map_err(|e| transport_error("cem.value.handle", "Limit", e, None))?;
    INPUTS.with(|inputs| {
        inputs.borrow_mut().insert(
            result_id,
            RetainedInput {
                values: evaluated.result,
                source: Some(source),
            },
        )
    });
    Ok(result_id)
}
#[wasm_bindgen(js_name = "exportNativeValueArtifact")]
pub fn export_native_value(id: u32, limits_json: &str) -> Result<Vec<u8>, JsValue> {
    let limits = limits(limits_json)
        .map_err(|e| transport_error("cem.value.limits", "InvalidLimits", e, None))?;
    INPUTS.with(|inputs| {
        let inputs = inputs.borrow();
        let input = inputs.get(&id).ok_or_else(|| {
            transport_error(
                "cem.value.handle",
                "UnknownHandle",
                "Unknown native result handle",
                None,
            )
        })?;
        crate::eval::portable::export_values(&input.values, &limits).map_err(|error| {
            let source_uri = input.source.as_ref().and_then(|retained| {
                let source_id = error.source.as_ref()?.origin()?.source_id;
                retained.ingress().reloaded().sources.iter().find(|source| source.source_id == source_id).map(|source| source.uri.as_str())
            });
            JsValue::from_str(&json!({ "code": error.diagnostic_code(), "kind": format!("{:?}", error.kind), "message": error.message, "sourceMap": error.source, "sourceUri": source_uri }).to_string())
        })
    })
}
