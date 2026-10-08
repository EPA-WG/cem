//! Live native sessions are WASM-local. Only explicit presentation exports use CEMV.
use super::*;
use crate::api::native_capability_session::NativeCapabilitySession;
thread_local! {
    static SESSIONS: RefCell<BTreeMap<u32, NativeCapabilitySession>> = const { RefCell::new(BTreeMap::new()) };
}
pub(super) fn bind_publication(data: &mut TemplateData, id: u32, key: &str) -> Result<(), String> {
    SESSIONS.with(|sessions| {
        let sessions = sessions.borrow();
        let session = sessions
            .get(&id)
            .ok_or("Unknown native publication owner")?;
        let view = session
            .suggestions_publication(key)
            .map_err(|e| e.message)?;
        data.bind_reserved_native_slice("suggestions", ItemStream::once(view.root()))
    })
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
fn suggestions_error(error: crate::suggestions::SuggestionsError) -> JsValue {
    transport_error(
        error.code,
        "SuggestionsConsumerFailure",
        error.message,
        Some(&error.source),
    )
}
fn suggestions_config(
    session: &NativeCapabilitySession,
    json: &str,
) -> Result<crate::suggestions::SuggestionsConfig, JsValue> {
    if json.len() > session.limits().max_bytes {
        return Err(session_error("Suggestions control byte limit exceeded"));
    }
    serde_json::from_str(json)
        .map_err(|e| transport_error("cem.suggestions.configuration", "InvalidConfig", e, None))
}
#[wasm_bindgen(js_name = "prepareNativeSuggestions")]
pub fn prepare_suggestions(id: u32) -> Result<String, JsValue> {
    SESSIONS.with(|sessions| {
        let sessions = sessions.borrow();
        let session = sessions.get(&id).ok_or_else(|| session_error("Unknown native capability session"))?;
        let plan = session.suggestions().map_err(suggestions_error)?;
        let mut diagnostics = Vec::new();
        let mut diagnostic_bytes = 0usize;
        for d in plan.warnings() {
            let diagnostic = json!({"code":d.code,"message":d.message,"severity":"warning","sourceMap":d.source});
            diagnostic_bytes = diagnostic_bytes.checked_add(diagnostic.to_string().len())
                .filter(|bytes| *bytes <= session.limits().max_bytes)
                .ok_or_else(|| session_error("Suggestions control byte limit exceeded"))?;
            diagnostics.push(diagnostic);
        }
        let control = json!({"rows":plan.len(),"groups":plan.group_count(),"identity":crate::suggestions::CONSUMER_IDENTITY,"diagnostics":diagnostics}).to_string();
        if control.len() > session.limits().max_bytes {
            return Err(session_error("Suggestions control byte limit exceeded"));
        }
        Ok(control)
    })
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
        export_view(session, values)
    })
}
fn export_view(session: &NativeCapabilitySession, values: ItemStream) -> Result<String, JsValue> {
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
}
#[wasm_bindgen(js_name = "exportNativeSuggestionsView")]
pub fn export_suggestions(
    id: u32,
    config_json: &str,
    expression: &str,
    index: Option<u32>,
) -> Result<String, JsValue> {
    SESSIONS.with(|sessions| {
        let sessions = sessions.borrow();
        let session = sessions
            .get(&id)
            .ok_or_else(|| session_error("Unknown native capability session"))?;
        let config = suggestions_config(session, config_json)?;
        let values = session
            .evaluate_suggestions(&config, expression, index.map(|i| i as usize))
            .map_err(suggestions_error)?;
        export_view(session, values)
    })
}
#[wasm_bindgen(js_name = "renderNativeSuggestionTemplate")]
pub fn render_suggestion(
    id: u32,
    artifact_id: u32,
    config_json: &str,
    index: Option<u32>,
    group: bool,
) -> Result<String, JsValue> {
    SESSIONS.with(|sessions| {
        let sessions = sessions.borrow();
        let session = sessions
            .get(&id)
            .ok_or_else(|| session_error("Unknown native capability session"))?;
        let config = suggestions_config(session, config_json)?;
        ARTIFACTS.with(|artifacts| {
            let artifacts = artifacts.borrow();
            let artifact = artifact_id
                .checked_sub(1)
                .and_then(|i| artifacts.get(i as usize))
                .and_then(Option::as_ref)
                .ok_or_else(|| session_error("Unknown native label template"))?;
            let plan = if group {
                session.render_suggestion_group(
                    &config,
                    artifact.artifact(),
                    index.ok_or_else(|| session_error("A group label requires a group handle"))?
                        as usize,
                )
            } else {
                session.render_suggestion(&config, artifact.artifact(), index.map(|i| i as usize))
            }
            .map_err(suggestions_error)?;
            Ok(plan_json_with_limits(&plan, session.limits()).to_string())
        })
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
#[wasm_bindgen(js_name = "publishNativeSuggestions")]
pub fn publish_suggestions(id: u32, key: &str, config_json: &str) -> Result<(), JsValue> {
    SESSIONS.with(|sessions| {
        let sessions = sessions.borrow();
        let session = sessions
            .get(&id)
            .ok_or_else(|| session_error("Unknown native capability session"))?;
        let config = suggestions_config(session, config_json)?;
        session
            .publish_suggestions(key, &config)
            .map_err(suggestions_error)?;
        Ok(())
    })
}
#[wasm_bindgen(js_name = "releaseNativeSuggestions")]
pub fn release_suggestions(id: u32, key: &str) -> bool {
    SESSIONS.with(|sessions| {
        sessions
            .borrow()
            .get(&id)
            .is_some_and(|session| session.release_suggestions(key))
    })
}
#[wasm_bindgen(js_name = "renderNativeSuggestionsFrame")]
pub fn render_suggestions_frame(
    id: u32,
    key: &str,
    artifact_id: u32,
    data_json: &str,
) -> Result<String, JsValue> {
    SESSIONS.with(|sessions| {
        let sessions = sessions.borrow();
        let session = sessions
            .get(&id)
            .ok_or_else(|| session_error("Unknown native capability session"))?;
        if data_json.len() > session.limits().max_bytes {
            return Err(session_error("Consumer frame control byte limit exceeded"));
        }
        let data = parse_template_data(data_json).map_err(session_error)?;
        ARTIFACTS.with(|artifacts| {
            let artifacts = artifacts.borrow();
            let artifact = artifact_id
                .checked_sub(1)
                .and_then(|i| artifacts.get(i as usize))
                .and_then(Option::as_ref)
                .ok_or_else(|| session_error("Unknown native consumer template"))?;
            let plan = session
                .render_suggestions_frame(key, artifact.artifact(), data)
                .map_err(suggestions_error)?;
            Ok(plan_json_with_limits(&plan, session.limits()).to_string())
        })
    })
}
#[wasm_bindgen(js_name = "nativeSuggestionRowControls")]
pub fn row_controls(id: u32, key: &str) -> Result<String, JsValue> {
    SESSIONS.with(|sessions| {
        let sessions = sessions.borrow();
        let session = sessions
            .get(&id)
            .ok_or_else(|| session_error("Unknown native capability session"))?;
        let controls = session
            .suggestion_row_controls(key)
            .map_err(suggestions_error)?;
        serde_json::to_string(&controls).map_err(session_error)
    })
}
#[wasm_bindgen(js_name = "disposeNativeCapabilitySession")]
pub fn dispose(id: u32) -> bool {
    SESSIONS.with(|sessions| sessions.borrow_mut().remove(&id).is_some())
}
