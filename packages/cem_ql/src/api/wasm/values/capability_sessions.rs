//! Live native sessions are WASM-local. Only explicit presentation exports use CEMV.
use super::*;
use crate::api::native_capability_session::NativeCapabilitySession;
thread_local! {
    static SESSIONS: RefCell<BTreeMap<u32, NativeCapabilitySession>> = const { RefCell::new(BTreeMap::new()) };
}
fn session_error(message: impl ToString) -> JsValue {
    let message = message.to_string();
    let code = if message.starts_with("cem.capability.source_incomplete:") {
        "cem.capability.source_incomplete"
    } else if message.starts_with("cem.capability.source_denied:") {
        "cem.capability.source_denied"
    } else {
        "cem.capability.session"
    };
    transport_error(code, "CapabilitySessionFailure", message, None)
}
#[wasm_bindgen(js_name = "prepareNativeCapabilitySession")]
pub fn prepare(
    data_json: &str,
    sources_json: &str,
    select: &str,
    resolve: bool,
    bindings_json: &str,
    limits_json: &str,
) -> Result<u32, JsValue> {
    let limits = limits(limits_json).map_err(session_error)?;
    if data_json.len() > limits.max_bytes {
        return Err(session_error("Session control input byte limit exceeded"));
    }
    let mut data = parse_template_data(data_json).map_err(session_error)?;
    bind(&mut data, bindings_json).map_err(session_error)?;
    let session =
        element_references::prepare_native_session(data, sources_json, select, resolve, limits)
            .map_err(session_error)?;
    let id = next_id().map_err(session_error)?;
    SESSIONS.with(|sessions| sessions.borrow_mut().insert(id, session));
    Ok(id)
}
#[wasm_bindgen(js_name = "nativeCapabilitySessionLength")]
pub fn length(id: u32) -> Result<u32, JsValue> {
    SESSIONS.with(|sessions| {
        let sessions = sessions.borrow();
        let session = sessions
            .get(&id)
            .ok_or_else(|| session_error("Unknown native capability session"))?;
        u32::try_from(session.len()).map_err(session_error)
    })
}
#[wasm_bindgen(js_name = "exportNativeCapabilityView")]
pub fn export(id: u32, expression: &str, index: Option<u32>) -> Result<String, JsValue> {
    SESSIONS.with(|sessions| {
        let sessions = sessions.borrow();
        let session = sessions
            .get(&id)
            .ok_or_else(|| session_error("Unknown native capability session"))?;
        let values = session
            .evaluate(expression, index.map(|i| i as usize))
            .map_err(session_error)?;
        clear_output();
        if values.items.is_empty() {
            return Ok(json!({"length": 0, "artifactId": null, "contentHash": null}).to_string());
        }
        let bytes =
            crate::eval::portable::export_values(&values, session.limits()).map_err(|error| {
                transport_error(
                    error.diagnostic_code(),
                    &format!("{:?}", error.kind),
                    error.message,
                    error.source.as_ref(),
                )
            })?;
        let hash = cem_ml::content_cache::ContentHash::from_blake3(&bytes).header_value();
        let artifact_id = next_id().map_err(session_error)?;
        OUTPUT.with(|output| *output.borrow_mut() = Some((artifact_id, bytes)));
        Ok(
            json!({"length": values.items.len(), "artifactId": artifact_id, "contentHash": hash})
                .to_string(),
        )
    })
}
#[wasm_bindgen(js_name = "renderNativeCapabilityTemplate")]
pub fn render(id: u32, artifact_id: u32, index: Option<u32>) -> Result<String, JsValue> {
    SESSIONS.with(|sessions| {
        let sessions = sessions.borrow();
        let session = sessions
            .get(&id)
            .ok_or_else(|| session_error("Unknown native capability session"))?;
        ARTIFACTS.with(|artifacts| {
            let artifacts = artifacts.borrow();
            let artifact = artifact_id
                .checked_sub(1)
                .and_then(|i| artifacts.get(i as usize))
                .and_then(Option::as_ref)
                .ok_or_else(|| session_error("Unknown native label template"))?;
            let plan = session
                .render(artifact.artifact(), index.map(|i| i as usize))
                .map_err(session_error)?;
            Ok(plan_json_with_limits(&plan, session.limits()).to_string())
        })
    })
}
#[wasm_bindgen(js_name = "disposeNativeCapabilitySession")]
pub fn dispose(id: u32) -> bool {
    SESSIONS.with(|sessions| sessions.borrow_mut().remove(&id).is_some())
}
