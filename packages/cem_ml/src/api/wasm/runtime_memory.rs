//! Small trusted setup/telemetry boundary, available in each linked WASM module.
use crate::runtime_memory::{runtime_budget, runtime_sample, MemoryBudgetConfig};
use wasm_bindgen::prelude::*;
#[wasm_bindgen(js_name = "runtimeMemoryStatus")]
pub fn status() -> String {
    serde_json::to_string(&runtime_budget().status(runtime_sample())).expect("memory status")
}
#[wasm_bindgen(js_name = "configureRuntimeMemoryBudget")]
pub fn configure(config_json: &str) -> Result<String, JsValue> {
    let _native = match crate::runtime_admission::admit(false) { Ok(permit) => permit, Err(error) => return Err(JsValue::from_str(error)) };
    if config_json.len() > 512 {
        return Err(JsValue::from_str("cem.runtime.memory.config"));
    }
    let config: MemoryBudgetConfig = serde_json::from_str(config_json)
        .map_err(|_| JsValue::from_str("cem.runtime.memory.config"))?;
    runtime_budget()
        .configure(config)
        .map_err(|error| JsValue::from_str(error.0))?;
    Ok(status())
}

/// Export bounded owned text while ordinary input admission is stopped. No
/// semantic parsing/rendering or operation-control bypass occurs on this path.
#[wasm_bindgen(js_name = "sealCommittedCheckpoint")]
pub fn seal_checkpoint(candidate: &str, receipt: &str) -> Result<String, JsValue> {
    let _native = match crate::runtime_admission::admit(true) { Ok(permit) => permit, Err(error) => return Err(JsValue::from_str(error)) };
    use crate::committed_checkpoint::{seal, MAX_CHECKPOINT_BYTES, MAX_WORKSPACE_CHECKPOINT_BYTES};
    if candidate.len() > MAX_WORKSPACE_CHECKPOINT_BYTES || receipt.len() > MAX_CHECKPOINT_BYTES {
        return Err(JsValue::from_str("cem.checkpoint.limit"));
    }
    let workspace = ((candidate.len() + receipt.len()) as u64) * 8 + 65536;
    let _permit = runtime_budget()
        .admit_recovery(runtime_sample(), workspace)
        .map_err(|error| JsValue::from_str(error.0))?;
    seal(candidate, receipt).map_err(JsValue::from_str)
}

/// Bounded browser control serialization admission in each linked runtime.
#[wasm_bindgen(js_name = "validateCheckpointControls")]
pub fn validate_checkpoint_controls(input: &str) -> Result<String, JsValue> {
    let _native = match crate::runtime_admission::admit(true) { Ok(permit) => permit, Err(error) => return Err(JsValue::from_str(error)) };
    use crate::checkpoint_control_state::{validate, MAX_BYTES};
    if input.len() > MAX_BYTES {
        return Err(JsValue::from_str("cem.checkpoint.controls.limit"));
    }
    let _permit = runtime_budget()
        .admit_recovery(runtime_sample(), (input.len() as u64) * 8 + 65536)
        .map_err(|error| JsValue::from_str(error.0))?;
    validate(input).map_err(JsValue::from_str)
}

/// Native admission for capability/resource checkpoint control metadata and bytes.
#[wasm_bindgen(js_name = "validateCheckpointRuntime")]
pub fn validate_checkpoint_runtime(input: &str) -> Result<String, JsValue> {
    let _native = match crate::runtime_admission::admit(true) { Ok(permit) => permit, Err(error) => return Err(JsValue::from_str(error)) };
    use crate::checkpoint_runtime_state::{validate, MAX_BYTES};
    if input.len() > MAX_BYTES {
        return Err(JsValue::from_str("cem.checkpoint.runtime.limit"));
    }
    let _permit = runtime_budget()
        .admit_recovery(runtime_sample(), (input.len() as u64) * 8 + 65536)
        .map_err(|error| JsValue::from_str(error.0))?;
    validate(input).map_err(JsValue::from_str)
}
