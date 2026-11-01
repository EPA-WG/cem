//! Passive test-only reader. No production installation or owner provisioning.
#![cfg(target_arch = "wasm32")]
use cem_ml::operation_control::OperationControl;
use cem_ql::application_source_transport::{self, ApplicationSourcePin};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = probeApplicationSources)]
pub fn verify(bytes: &[u8], pins: &str, cancelled: bool) -> Result<String, JsValue> {
    let error = |e: String| JsValue::from_str(&e);
    if pins.len() > 16 * 1024 {
        return Err(error("fixture pin limit".into()));
    }
    let pin: ApplicationSourcePin = serde_json::from_str(pins).map_err(|e| error(e.to_string()))?;
    let control = OperationControl::default();
    if cancelled {
        control
            .cancel_root(Some("fixture cancellation".into()), None)
            .map_err(|e| error(format!("{e:?}")))?;
    }
    let verified = application_source_transport::verify(bytes, &pin, &control).map_err(error)?;
    let (base, mixed) = verified.receivers().map_err(error)?;
    Ok(format!(
        "{}:{}:{}:{}",
        verified.application().component_count(),
        verified.claims().len(),
        base.len(),
        mixed.len()
    ))
}
