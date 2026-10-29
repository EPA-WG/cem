//! Isolated compiled-component prototype. No source compile or dynamic selection ABI.
//! Retained declaration/style ownership is explicit; handles stay in one heap.
use crate::{
    eval::{imported_cem_tree, ItemStream},
    render::{
        render_compiled_template_with_control, render_plan_to_html_with_control, TemplateData,
    },
    template_artifact::{CompiledTemplateArtifact, PinnedTemplateArtifactContext},
};
use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    scheduler::{AbortSignal, ScopePolicy},
};
use std::sync::Arc;

pub mod owners;

#[cfg(test)]
mod tests;
mod generated {
    // CLI-generated Rust uses the public crate path, also when included here.
    #[allow(unused_imports)]
    use crate as cem_ql;
    include!(concat!(env!("OUT_DIR"), "/application_registry.rs"));
}
pub const MAX_INPUT_BYTES: usize = 1024 * 1024;

/// A deliberately small prototype boundary: one pinned template and one native
/// JSON document. It does not promise component ownership, CSS installation,
/// shared providers or checkpoint handoff. Production defaults use the full ABI.
pub fn render(
    bytes: &[u8],
    pins_json: &str,
    json: &[u8],
    source_uri: &str,
    binding: &str,
) -> Result<String, String> {
    if bytes.len() > MAX_INPUT_BYTES
        || json.len() > MAX_INPUT_BYTES
        || pins_json.len() > 16 * 1024
        || source_uri.len() > 4096
        || binding.len() > 256
    {
        return Err("compiled application input limit exceeded".into());
    }
    if !(bytes.starts_with(b"CEMTPLA2\n") || bytes.starts_with(b"CEMTPLA3\n")) {
        return Err("compiled application requires compiled CSS v2/v3 artifacts".into());
    }
    let uri = url::Url::parse(source_uri).map_err(|_| "JSON input needs an absolute source URI")?;
    if uri.cannot_be_a_base() {
        return Err("JSON input needs a hierarchical source URI".into());
    }
    let _native = cem_ml::runtime_admission::admit(false).map_err(str::to_owned)?;
    #[cfg(target_arch = "wasm32")]
    let _memory = cem_ml::runtime_memory::admit_input(bytes.len() + pins_json.len() + json.len())
        .map_err(|error| error.to_string())?;
    let pins: PinnedTemplateArtifactContext =
        serde_json::from_str(pins_json).map_err(|_| "invalid deployment pins")?;
    if binding.is_empty() || pins.host_bindings.as_slice() != [binding] {
        return Err("prototype requires exactly one pinned native JSON binding".into());
    }
    let artifact = CompiledTemplateArtifact::from_bytes(bytes.to_vec())
        .map_err(|error| error.code.to_owned())?
        .reload_pinned(&pins)
        .map_err(|error| error.code.to_owned())?;
    render_loaded(&artifact, json, source_uri, binding)
}

fn render_loaded(
    artifact: &crate::render::TemplateArtifact,
    json: &[u8],
    source_uri: &str,
    binding: &str,
) -> Result<String, String> {
    if json.len() > MAX_INPUT_BYTES || source_uri.len() > 4096 || binding.len() > 256 {
        return Err("compiled application input limit exceeded".into());
    }
    let uri = url::Url::parse(source_uri).map_err(|_| "JSON input needs an absolute source URI")?;
    if uri.cannot_be_a_base() {
        return Err("JSON input needs a hierarchical source URI".into());
    }
    let tree = cem_ml::import::import_json_bytes(json, source_uri)
        .map_err(|_| "native JSON import failed")?;
    let mut data =
        TemplateData::default().with_binding(binding, ItemStream::once(imported_cem_tree(tree)));
    data.selected_registry = Some(Arc::new(generated::selected_registry()));
    let control = OperationControl::with_root_policy(
        AbortSignal::new(),
        ScopePolicy::host_root().with_timeout_ms(Some(5000)),
    )
    .map_err(|error| error.to_string())?;
    let plan =
        render_compiled_template_with_control(artifact, &data, &control, ROOT_EXECUTION_SCOPE_ID);
    if plan
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation())
    {
        // Do not copy expressions, input content or credential values to errors.
        return Err("compiled application render failed".into());
    }
    let output = render_plan_to_html_with_control(&plan, &control, ROOT_EXECUTION_SCOPE_ID)
        .map_err(|error| error.code().to_owned())?;
    if output.rendered.len() > MAX_INPUT_BYTES {
        return Err("compiled application output limit exceeded".into());
    }
    Ok(output.rendered)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = "renderCompiledApplication")]
pub fn render_wasm(
    bytes: &[u8],
    pins_json: &str,
    json: &[u8],
    source_uri: &str,
    binding: &str,
) -> Result<String, wasm_bindgen::JsValue> {
    render(bytes, pins_json, json, source_uri, binding)
        .map_err(|error| wasm_bindgen::JsValue::from_str(&error))
}
