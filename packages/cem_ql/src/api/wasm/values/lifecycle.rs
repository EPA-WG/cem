//! Opaque execution/snapshot handles. JSON is transport control metadata only.
use super::*;
use crate::api::reference_lifecycle::{
    query_snapshot::ReferenceQuerySnapshot, resources::ReferenceResourceExecution,
};
use std::sync::Arc;
thread_local! {
    static EXECUTIONS: RefCell<BTreeMap<u32, ReferenceResourceExecution>> = const { RefCell::new(BTreeMap::new()) };
    static SNAPSHOTS: RefCell<BTreeMap<u32, Arc<ReferenceQuerySnapshot>>> = const { RefCell::new(BTreeMap::new()) };
}
#[wasm_bindgen(js_name = "startReferenceResourceExecution")]
pub fn start(session_id: u32) -> Result<u32, JsValue> {
    let execution = VALIDATIONS.with(|sessions| {
        sessions
            .borrow()
            .get(&session_id)
            .ok_or_else(|| validation_error("Unknown validation session"))?
            .start_resources(Default::default())
            .map_err(validation_error)
    })?;
    let id = next_id().map_err(validation_error)?;
    EXECUTIONS.with(|runs| runs.borrow_mut().insert(id, execution));
    Ok(id)
}
#[wasm_bindgen(js_name = "advanceReferenceResourceExecution")]
pub fn advance(id: u32) -> Result<String, JsValue> {
    EXECUTIONS.with(|runs| {
        let progress = runs
            .borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .advance()
            .map_err(validation_error)?;
        serde_json::to_string(&progress).map_err(validation_error)
    })
}
#[wasm_bindgen(js_name = "completeReferenceResource")]
pub fn complete(
    id: u32,
    request_id: u32,
    bytes: &[u8],
    content_type: &str,
    final_uri: &str,
) -> Result<String, JsValue> {
    if bytes.len() > cem_ml::ast::reload::ReloadLimits::default().max_bytes {
        return Err(validation_error("Resource byte limit exceeded"));
    }
    let source = EXECUTIONS.with(|runs| {
        runs.borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .complete(
                request_id.into(),
                Ok(cem_ml::resolver::ResolvedRead {
                    uri: final_uri.into(),
                    bytes: bytes.into(),
                    content_type: (!content_type.is_empty()).then(|| content_type.into()),
                }),
            )
            .map_err(validation_error)
    })?;
    let Some(source) = source else {
        return Ok("null".into());
    };
    Ok(json!({"sourceIndex":source.index, "sourceId":retain_source(source.source)?}).to_string())
}
#[wasm_bindgen(js_name = "failReferenceResource")]
pub fn fail_resource(id: u32, request_id: u32, reason: &str) -> Result<(), JsValue> {
    EXECUTIONS.with(|runs| {
        runs.borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .complete(
                request_id.into(),
                Err(cem_ml::diagnostics::Diagnostic {
                    code: "cem.reference.resource_unavailable".into(),
                    severity: cem_ml::diagnostics::Severity::Error,
                    message: reason.into(),
                    ..Default::default()
                }),
            )
            .map(|_| ())
            .map_err(validation_error)
    })
}
#[wasm_bindgen(js_name = "setReferenceResourceContext")]
pub fn context(
    id: u32,
    source_index: u32,
    ready: bool,
    bindings_json: &str,
) -> Result<(), JsValue> {
    let context = validation_context(ready, bindings_json)?;
    EXECUTIONS.with(|runs| {
        runs.borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .set_loaded_context(source_index as usize, context)
            .map_err(validation_error)
    })
}
#[wasm_bindgen(js_name = "allowReferenceResourceCrossing")]
pub fn crossing(id: u32, from: u32, to: u32) -> Result<(), JsValue> {
    EXECUTIONS.with(|runs| {
        runs.borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .allow_crossing(from as usize, to as usize)
            .map_err(validation_error)
    })
}
#[wasm_bindgen(js_name = "cancelReferenceResourceExecution")]
pub fn cancel(id: u32) -> Result<(), JsValue> {
    EXECUTIONS.with(|runs| {
        runs.borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .cancel();
        Ok(())
    })
}
#[wasm_bindgen(js_name = "disposeReferenceResourceExecution")]
pub fn dispose_execution(id: u32) -> bool {
    EXECUTIONS.with(|runs| runs.borrow_mut().remove(&id).is_some())
}
#[wasm_bindgen(js_name = "prepareReferenceQuerySnapshot")]
pub fn snapshot(session_id: u32) -> Result<u32, JsValue> {
    let snapshot = VALIDATIONS.with(|sessions| {
        sessions
            .borrow()
            .get(&session_id)
            .ok_or_else(|| validation_error("Unknown validation session"))?
            .query_snapshot()
            .map_err(validation_error)
    })?;
    let id = next_id().map_err(validation_error)?;
    SNAPSHOTS.with(|snapshots| snapshots.borrow_mut().insert(id, Arc::new(snapshot)));
    Ok(id)
}
fn retained_snapshot(id: u32) -> Result<Arc<ReferenceQuerySnapshot>, JsValue> {
    SNAPSHOTS
        .with(|snapshots| snapshots.borrow().get(&id).cloned())
        .ok_or_else(|| validation_error("Unknown query snapshot"))
}
#[wasm_bindgen(js_name = "inspectReferenceQuerySnapshot")]
pub fn inspect_snapshot(id: u32) -> Result<String, JsValue> {
    serde_json::to_string(retained_snapshot(id)?.report()).map_err(validation_error)
}
#[wasm_bindgen(js_name = "queryReferenceQuerySnapshot")]
pub fn query_snapshot(id: u32, expression: &str, query_uri: &str) -> Result<u32, JsValue> {
    let snapshot = retained_snapshot(id)?;
    let context = crate::api::StandaloneExpressionContext {
        source_uri: Some(query_uri.into()),
        ..Default::default()
    };
    let evaluated = snapshot
        .evaluate(expression, &context)
        .map_err(|e| validation_error(e.message))?;
    if let Some(error) = &evaluated.result.error {
        return Err(validation_error(format!("{error:?}")));
    }
    let id = next_id().map_err(validation_error)?;
    // Native result nodes retain their completed-name view independently.
    INPUTS.with(|inputs| {
        inputs.borrow_mut().insert(
            id,
            RetainedInput {
                values: evaluated.result,
                source: Some(snapshot.source().clone()),
            },
        )
    });
    Ok(id)
}
#[wasm_bindgen(js_name = "disposeReferenceQuerySnapshot")]
pub fn dispose_snapshot(id: u32) -> bool {
    SNAPSHOTS.with(|snapshots| snapshots.borrow_mut().remove(&id).is_some())
}
