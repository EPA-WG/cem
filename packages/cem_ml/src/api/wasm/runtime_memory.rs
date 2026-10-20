//! Small trusted setup/telemetry boundary, available in each linked WASM module.
use crate::runtime_memory::{runtime_budget, runtime_sample, MemoryBudgetConfig};
use wasm_bindgen::prelude::*;
#[wasm_bindgen(js_name = "runtimeMemoryStatus")]
pub fn status() -> String {
    serde_json::to_string(&runtime_budget().status(runtime_sample())).expect("memory status")
}
#[wasm_bindgen(js_name = "configureRuntimeMemoryBudget")]
pub fn configure(config_json: &str) -> Result<String, JsValue> {
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
