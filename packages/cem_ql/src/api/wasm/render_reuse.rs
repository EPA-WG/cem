//! One bounded output store per WASM runtime, never per host/element inventory.
use super::*;
use crate::render_cache::{
    retained::{RetainedJsonRenderCache, RetainedJsonRenderContext},
    CheckedPureRender,
};
thread_local! {
    static RENDERS: RefCell<RetainedJsonRenderCache> = RefCell::new(RetainedJsonRenderCache::default());
}
pub(super) fn render(
    artifact_id: u32,
    artifact: &RetainedTemplate,
    json: &str,
    mut context: RetainedJsonRenderContext,
) -> Result<CheckedPureRender, cem_ml::content_cache::CacheError> {
    context.owner = format!("{artifact_id}/{}", context.owner);
    RENDERS.with(|cell| {
        cell.borrow_mut()
            .render(artifact.artifact(), json, &context, artifact.data_readers())
    })
}
pub(super) fn telemetry(hit: bool) -> Value {
    RENDERS.with(|cell| {
        let cache = cell.borrow(); let stats = cache.stats();
        json!({"hit": hit, "evaluations": cache.evaluations(), "admissions": cache.admissions(), "runtimeGeneration":stats.runtime_generation})
    })
}
pub(super) fn reset() {
    RENDERS.with(|cell| {
        cell.borrow_mut()
            .reset()
            .expect("native render generation available")
    });
}
#[wasm_bindgen(js_name = "retainedRenderCacheStats")]
pub fn stats() -> String {
    RENDERS.with(|cell| { let cache = cell.borrow(); let stats = cache.stats();
        json!({"evaluations":cache.evaluations(), "admissions":cache.admissions(), "runtimeGeneration":stats.runtime_generation,
            "residentEntries":stats.resident_entries,"retainedBytes":stats.retained_bytes,"liveLeases":stats.live_leases}).to_string()
    })
}
#[wasm_bindgen(js_name = "resetRetainedRenderCache")]
pub fn reset_export() -> Result<(), JsValue> {
    let _native = match cem_ml::runtime_admission::admit(true) { Ok(permit) => permit, Err(error) => return Err(JsValue::from_str(error)) };
    reset();
    Ok(())
}

pub(super) fn resource_inventory_entries() -> Vec<cem_ml::runtime_inventory::RegistryInventory> {
    use cem_ml::runtime_inventory::RegistryInventory as Entry;
    RENDERS.with(|s| { let stats = s.borrow().stats(); vec![Entry::new("render-cache", stats.live_leases as u64, stats.resident_entries as u64)] })
}
