//! Explicit companion control plane, separate from ordinary JSON data bindings.
use super::*;
use crate::xpath::functions::{
    companion::{parse_hash, CemtXPathCompanions, COMPANION_CONTENT_TYPE, COMPANION_VERSION},
    CemtXPathFunctions,
};

thread_local! {
    static COMPANIONS: RefCell<CemtXPathCompanions> = RefCell::new(CemtXPathCompanions::default());
}

/// Source-loaded hosts get their hashes directly from the compiler. Program
/// bytes stay binary/native and do not pass through the JSON metadata response.
#[wasm_bindgen(js_name = "retainCemtXPathFunctions")]
pub fn retain_cemt_xpath_functions(source: &str, source_uri: &str) -> Result<String, JsValue> {
    let bytes = compile_cemt_xpath_functions(source, source_uri)?;
    import_cemt_xpath_functions(
        &bytes,
        &cem_ml::content_cache::ContentHash::from_blake3(&bytes).header_value(),
        &cem_ml::content_cache::ContentHash::from_blake3(source.as_bytes()).header_value(),
    )
}

#[wasm_bindgen(js_name = "compileCemtXPathFunctions")]
pub fn compile_cemt_xpath_functions(source: &str, source_uri: &str) -> Result<Vec<u8>, JsValue> {
    let library = CemtXPathFunctions::compile(source, source_uri).map_err(|diagnostics| {
        JsValue::from_str(
            &diagnostics
                .iter()
                .map(|d| format!("{}: {}", d.code, d.message))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    })?;
    library
        .to_companion_bytes()
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = "importCemtXPathFunctions")]
pub fn import_cemt_xpath_functions(
    bytes: &[u8],
    expected_content_hash: &str,
    expected_source_hash: &str,
) -> Result<String, JsValue> {
    let content_hash =
        parse_hash(expected_content_hash).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let source_hash =
        parse_hash(expected_source_hash).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let id = COMPANIONS
        .with(|host| host.borrow_mut().import(bytes, &content_hash, &source_hash))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(
        json!({ "companionId": id, "contentType": COMPANION_CONTENT_TYPE,
        "formatVersion": COMPANION_VERSION, "contentHash": content_hash.header_value(),
        "sourceHash": source_hash.header_value() })
        .to_string(),
    )
}

#[wasm_bindgen(js_name = "disposeCemtXPathFunctions")]
pub fn dispose_cemt_xpath_functions(companion_id: u32) -> bool {
    COMPANIONS.with(|host| host.borrow_mut().dispose(companion_id))
}

/// The companion is a separate, explicit capability argument, never a field in
/// JSON data. No template/default registry is mutated by this call.
#[wasm_bindgen(js_name = "renderTemplateWithXPathFunctions")]
pub fn render_template_with_xpath_functions(
    artifact_id: u32,
    companion_id: u32,
    data_json: &str,
) -> String {
    render_template_with_cem_documents(artifact_id, companion_id, data_json, "[]")
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DocumentBinding {
    slice: String,
    document_id: u32,
}

/// Document handles and function capabilities are explicit control arguments.
#[wasm_bindgen(js_name = "renderTemplateWithCemDocuments")]
pub fn render_template_with_cem_documents(
    artifact_id: u32,
    companion_id: u32,
    data_json: &str,
    bindings_json: &str,
) -> String {
    let mut data = match parse_template_data(data_json) {
        Ok(data) => data,
        Err(message) => return error_json("cem.ql.wasm.invalid_data", message),
    };
    if companion_id != 0 {
        let Some(functions) = COMPANIONS.with(|host| host.borrow().functions(companion_id)) else {
            return error_json(
                "cem.ql.wasm.unknown_xpath_companion",
                "XPath function companion is not registered",
            );
        };
        data.native_functions = functions;
    }
    let bindings: Vec<DocumentBinding> = match serde_json::from_str(bindings_json) {
        Ok(bindings) => bindings,
        Err(error) => return error_json("cem.ql.wasm.invalid_document_binding", error.to_string()),
    };
    let mut seen = std::collections::BTreeSet::new();
    for binding in bindings {
        if !seen.insert(binding.slice.clone()) {
            return error_json(
                "cem.ql.wasm.invalid_document_binding",
                "duplicate resource slice binding",
            );
        }
        let Some(tree) = cem_ml::api::wasm::retained_cem_document(binding.document_id) else {
            return error_json(
                "cem.ql.wasm.unknown_document",
                "CEM document is not retained",
            );
        };
        if let Err(message) = data.bind_cem_document(&binding.slice, tree) {
            return error_json("cem.ql.wasm.invalid_document_binding", message);
        }
    }
    ARTIFACTS.with(|cell| {
        let artifacts = cell.borrow();
        let Some(Some(artifact)) = artifact_id
            .checked_sub(1)
            .and_then(|index| artifacts.get(index as usize))
        else {
            return error_json(
                "cem.ql.wasm.unknown_artifact",
                "template artifact is not registered",
            );
        };
        data.data_readers = artifact.data_readers.clone();
        plan_json(&render_compiled_template(&artifact.artifact, &data)).to_string()
    })
}
