//! Explicit XSLT bundle control plane. JSON carries scalar parameters and
//! retained-document handles, never documents or executable capabilities.
use super::*;
use crate::xslt::{
    compiler::compile_xslt_bundle, parse_hash, XsltBundleHost, BUNDLE_CONTENT_TYPE, BUNDLE_VERSION,
};

thread_local! {
    static BUNDLES: RefCell<XsltBundleHost> = RefCell::new(XsltBundleHost::default());
    static COMPONENTS: RefCell<ComponentHost> = RefCell::new(ComponentHost::default());
}

#[derive(Default)]
struct ComponentHost {
    next_id: u32,
    bytes: usize,
    entries: std::collections::BTreeMap<u32, std::sync::Arc<crate::xslt::component::XsltComponent>>,
}

#[wasm_bindgen(js_name = "xsltStylesheetImports")]
pub fn stylesheet_imports(source: &str, source_uri: &str) -> Result<String, JsValue> {
    crate::xslt::compiler::stylesheet_imports(source, source_uri)
        .map(|imports| json!(imports).to_string())
        .map_err(|diagnostics| {
            JsValue::from_str(&json!({ "diagnostics": diagnostics_json(&diagnostics) }).to_string())
        })
}

/// Retain a typed bundle and compiled scalar selectors as one lifecycle unit.
#[wasm_bindgen(js_name = "retainXsltComponent")]
pub fn retain_xslt_component(
    source: &str,
    source_uri: &str,
    options_json: &str,
    host_bindings_json: &str,
) -> Result<String, JsValue> {
    if options_json.len() > crate::xslt::MAX_BUNDLE_BYTES || host_bindings_json.len() > 128 * 1024 {
        return Err(JsValue::from_str("XSLT component options exceed limits"));
    }
    let options =
        serde_json::from_str(options_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let bindings = parse_host_bindings(host_bindings_json).map_err(|e| JsValue::from_str(&e))?;
    COMPONENTS.with(|host| {
        let mut host = host.borrow_mut();
        if host.entries.len() >= 16 {
            return Err(JsValue::from_str("XSLT component handle limit exceeded"));
        }
        let component =
            crate::xslt::component::XsltComponent::compile(source, source_uri, &options, &bindings)
                .map_err(|diagnostics| {
                    JsValue::from_str(
                        &json!({"diagnostics": diagnostics_json(&diagnostics)}).to_string(),
                    )
                })?;
        if component.retained_bytes > (32usize * 1024 * 1024).saturating_sub(host.bytes) {
            return Err(JsValue::from_str("XSLT component byte limit exceeded"));
        }
        let id = host
            .next_id
            .checked_add(1)
            .ok_or_else(|| JsValue::from_str("XSLT component handle limit exceeded"))?;
        let result =
            json!({"artifactId": id, "stylesheets": stylesheets_json(component.bundle.template()),
            "diagnostics": diagnostics_json(component.diagnostics())})
            .to_string();
        host.bytes += component.retained_bytes;
        host.next_id = id;
        host.entries.insert(id, std::sync::Arc::new(component));
        Ok(result)
    })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResourceBinding {
    slice: String,
    document_id: u32,
}

#[wasm_bindgen(js_name = "renderXsltComponent")]
pub fn render_xslt_component(
    id: u32,
    data_json: &str,
    documents_json: &str,
    initial_document_id: Option<u32>,
) -> String {
    let Some(component) = COMPONENTS.with(|host| host.borrow().entries.get(&id).cloned()) else {
        return error_json(
            "cem.xslt.unknown_component",
            "XSLT component is not retained",
        );
    };
    if data_json.len() > 128 * 1024 || documents_json.len() > 128 * 1024 {
        return error_json(
            "cem.xslt.binding_limit",
            "XSLT control bindings exceed byte limit",
        );
    }
    let mut data = match parse_template_data(data_json) {
        Ok(data) => data,
        Err(error) => return error_json("cem.xslt.invalid_binding", error),
    };
    // JSON controls cannot supply an initial native document/focus.
    data.bindings.remove("document");
    if let Some(id) = initial_document_id {
        let Some(tree) = cem_ml::api::wasm::retained_cem_document(id) else {
            return error_json(
                "cem.xslt.unknown_document",
                "initial CEM document is not retained",
            );
        };
        data.bindings.insert(
            "document".into(),
            crate::eval::ItemStream::once(crate::eval::imported_cem_tree(tree)),
        );
    }
    let documents: Vec<ResourceBinding> = match serde_json::from_str(documents_json) {
        Ok(documents) => documents,
        Err(error) => return error_json("cem.xslt.invalid_binding", error.to_string()),
    };
    let mut seen = std::collections::BTreeSet::new();
    for binding in documents {
        if !seen.insert(binding.slice.clone()) {
            return error_json(
                "cem.xslt.invalid_binding",
                "duplicate resource slice binding",
            );
        }
        let Some(tree) = cem_ml::api::wasm::retained_cem_document(binding.document_id) else {
            return error_json("cem.xslt.unknown_document", "CEM document is not retained");
        };
        if let Err(error) = data.bind_cem_document(&binding.slice, tree) {
            return error_json("cem.xslt.invalid_binding", error);
        }
    }
    plan_json(&component.render(&data)).to_string()
}

#[wasm_bindgen(js_name = "disposeXsltComponent")]
pub fn dispose_xslt_component(id: u32) -> bool {
    COMPONENTS.with(|host| {
        let mut host = host.borrow_mut();
        if let Some(component) = host.entries.remove(&id) {
            host.bytes -= component.retained_bytes;
            true
        } else {
            false
        }
    })
}

/// Compile stylesheet authoring source into portable native members. Errors
/// carry explicit diagnostic metadata; executable ASTs never cross as JSON.
#[wasm_bindgen(js_name = "compileXsltBundle")]
pub fn compile_xslt_bundle_bytes(source: &str, source_uri: &str) -> Result<Vec<u8>, JsValue> {
    compile_xslt_bundle(source, source_uri)
        .map(|compiled| compiled.bytes)
        .map_err(|diagnostics| {
            JsValue::from_str(
                &json!({
                    "diagnostics": diagnostics_json(&diagnostics),
                })
                .to_string(),
            )
        })
}

/// Source-loaded hosts obtain trusted hashes from the native compiler and use
/// the same bounded retention/disposal lifecycle as precompiled bundles.
#[wasm_bindgen(js_name = "retainXsltStylesheet")]
pub fn retain_xslt_stylesheet(source: &str, source_uri: &str) -> Result<String, JsValue> {
    let bytes = compile_xslt_bundle_bytes(source, source_uri)?;
    import_xslt_bundle(
        &bytes,
        &cem_ml::content_cache::ContentHash::from_blake3(&bytes).header_value(),
        &cem_ml::content_cache::ContentHash::from_blake3(source.as_bytes()).header_value(),
    )
}

#[wasm_bindgen(js_name = "importXsltBundle")]
pub fn import_xslt_bundle(
    bytes: &[u8],
    expected_content_hash: &str,
    expected_root_source_hash: &str,
) -> Result<String, JsValue> {
    let hash = parse_hash(expected_content_hash).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let source =
        parse_hash(expected_root_source_hash).map_err(|e| JsValue::from_str(&e.to_string()))?;
    BUNDLES.with(|host| {
        let mut host = host.borrow_mut();
        let id = host.import(bytes, &hash, &source).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let bundle = host.get(id).expect("just retained bundle");
        Ok(json!({
            "bundleId": id, "contentType": BUNDLE_CONTENT_TYPE, "formatVersion": BUNDLE_VERSION,
            "contentHash": hash.header_value(), "rootSourceHash": source.header_value(),
            "sourceClosure": bundle.stylesheets(), "generatedSource": bundle.generated_source(),
            "hostBindings": bundle.host_bindings(), "stylesheets": stylesheets_json(bundle.template()),
            "moduleMap": module_map_json(bundle.template()), "diagnostics": diagnostics_json(&bundle.template().diagnostics),
        }).to_string())
    })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DocumentBinding {
    name: String,
    document_id: u32,
}

#[wasm_bindgen(js_name = "renderXsltBundle")]
pub fn render_xslt_bundle(
    bundle_id: u32,
    scalar_bindings_json: &str,
    document_bindings_json: &str,
) -> String {
    let Some(bundle) = BUNDLES.with(|host| host.borrow().get(bundle_id)) else {
        return error_json("cem.xslt.unknown_bundle", "XSLT bundle is not retained");
    };
    if scalar_bindings_json.len() > 128 * 1024 || document_bindings_json.len() > 128 * 1024 {
        return error_json(
            "cem.xslt.binding_limit",
            "XSLT control bindings exceed byte limit",
        );
    }
    let scalars: serde_json::Map<String, Value> = match serde_json::from_str(scalar_bindings_json) {
        Ok(value) => value,
        Err(error) => return error_json("cem.xslt.invalid_binding", error.to_string()),
    };
    let documents: Vec<DocumentBinding> = match serde_json::from_str(document_bindings_json) {
        Ok(value) => value,
        Err(error) => return error_json("cem.xslt.invalid_binding", error.to_string()),
    };
    let mut data = TemplateData::default();
    for (name, value) in scalars {
        if !bundle.host_bindings().contains(&name) || value.is_array() || value.is_object() {
            return error_json(
                "cem.xslt.invalid_binding",
                "only declared scalar host parameters are accepted",
            );
        }
        let value = match json_value_to_stream(value) {
            Ok(value) => value,
            Err(error) => return error_json("cem.xslt.invalid_binding", error),
        };
        data.bindings.insert(name, value);
    }
    for binding in documents {
        if !bundle.host_bindings().contains(&binding.name)
            || data.bindings.contains_key(&binding.name)
        {
            return error_json(
                "cem.xslt.invalid_binding",
                "unknown or duplicate document host parameter",
            );
        }
        let Some(tree) = cem_ml::api::wasm::retained_cem_document(binding.document_id) else {
            return error_json("cem.xslt.unknown_document", "CEM document is not retained");
        };
        data.bindings.insert(
            binding.name,
            crate::eval::ItemStream::once(crate::eval::imported_cem_tree(tree)),
        );
    }
    plan_json(&bundle.render(&data)).to_string()
}

#[wasm_bindgen(js_name = "disposeXsltBundle")]
pub fn dispose_xslt_bundle(bundle_id: u32) -> bool {
    BUNDLES.with(|host| host.borrow_mut().dispose(bundle_id))
}
