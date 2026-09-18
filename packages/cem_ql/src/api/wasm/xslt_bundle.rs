//! Explicit XSLT bundle control plane. JSON carries scalar parameters and
//! retained-document handles, never documents or executable capabilities.
use super::*;
use crate::xslt::{parse_hash, XsltBundleHost, BUNDLE_CONTENT_TYPE, BUNDLE_VERSION};

thread_local! {
    static BUNDLES: RefCell<XsltBundleHost> = RefCell::new(XsltBundleHost::default());
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
