//! WASM-callable observer surface (AC-O-1 / AC-C-1).
//!
//! Exposes the named event channels — `onParseEvent`, `onValidate`,
//! `onTransform` — to JS callers through `wasm-bindgen`. Each
//! registration replaces the previously-installed callback for that
//! channel; the matching `off*` function clears it. Callbacks receive
//! the event as a JSON string (the canonical wire form documented in
//! `cem-ml-stack-design-impl.md` §3.12.1 and modelled by
//! `packages/cem_ml/schema/observability/report-event.schema.json`).
//! Resolver callbacks follow the same registration pattern through
//! `onResolveRead` and `onResolveWrite`; [`JsResourceResolver`] adapts
//! those callbacks to the shared `ResourceResolver` trait for Rust-side
//! WASM entrypoints.
//!
//! ```js
//! import init, { onParseEvent, offParseEvent } from "@epa-wg/cem-ml/wasm";
//!
//! await init();
//! onParseEvent((json) => {
//!   const event = JSON.parse(json);
//!   console.log(event.channel, event.sequence, event.byteOffset);
//! });
//! onResolveRead((json) => {
//!   const request = JSON.parse(json);
//!   return { uri: request.uri, bytes: new TextEncoder().encode("<cem />") };
//! });
//! // later:
//! offParseEvent();
//! offResolveRead();
//! ```
//!
//! The same adapter also exposes the bounded legacy custom-element
//! HTML+XSLT lowering path as JSON so JS hosts can call the CEM-owned
//! converter before routing the canonical CEM-ML through the render engine.
//!
//! [`JsObserver`] is the `EngineObserver` adapter that dispatches each
//! event through whichever JS callback is currently registered.
//! Embedders running the pipeline from Rust-side WASM code pass it as
//! the observer to `observe_pipeline`.

use std::cell::RefCell;

use js_sys::{Array, ArrayBuffer, Function, Reflect, Uint8Array};
use wasm_bindgen::{prelude::*, JsCast};

use crate::observability::{EngineObserver, ReportEvent};
use crate::resolver::{
    ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedRead, ResolvedWrite,
    ResolverDiagnostic, ResolverRegistry, ResourceResolver,
};

thread_local! {
    static PARSE_OBSERVER: RefCell<Option<Function>> = const { RefCell::new(None) };
    static VALIDATE_OBSERVER: RefCell<Option<Function>> = const { RefCell::new(None) };
    static TRANSFORM_OBSERVER: RefCell<Option<Function>> = const { RefCell::new(None) };
    static RESOLVER_READ: RefCell<Option<Function>> = const { RefCell::new(None) };
    static RESOLVER_WRITE: RefCell<Option<Function>> = const { RefCell::new(None) };
}

#[wasm_bindgen(js_name = "onParseEvent")]
pub fn on_parse_event(callback: Function) {
    PARSE_OBSERVER.with(|cell| *cell.borrow_mut() = Some(callback));
}

#[wasm_bindgen(js_name = "offParseEvent")]
pub fn off_parse_event() {
    PARSE_OBSERVER.with(|cell| *cell.borrow_mut() = None);
}

#[wasm_bindgen(js_name = "onValidate")]
pub fn on_validate(callback: Function) {
    VALIDATE_OBSERVER.with(|cell| *cell.borrow_mut() = Some(callback));
}

#[wasm_bindgen(js_name = "offValidate")]
pub fn off_validate() {
    VALIDATE_OBSERVER.with(|cell| *cell.borrow_mut() = None);
}

#[wasm_bindgen(js_name = "onTransform")]
pub fn on_transform(callback: Function) {
    TRANSFORM_OBSERVER.with(|cell| *cell.borrow_mut() = Some(callback));
}

#[wasm_bindgen(js_name = "offTransform")]
pub fn off_transform() {
    TRANSFORM_OBSERVER.with(|cell| *cell.borrow_mut() = None);
}

#[wasm_bindgen(js_name = "onResolveRead")]
pub fn on_resolve_read(callback: Function) {
    RESOLVER_READ.with(|cell| *cell.borrow_mut() = Some(callback));
}

#[wasm_bindgen(js_name = "offResolveRead")]
pub fn off_resolve_read() {
    RESOLVER_READ.with(|cell| *cell.borrow_mut() = None);
}

#[wasm_bindgen(js_name = "onResolveWrite")]
pub fn on_resolve_write(callback: Function) {
    RESOLVER_WRITE.with(|cell| *cell.borrow_mut() = Some(callback));
}

#[wasm_bindgen(js_name = "offResolveWrite")]
pub fn off_resolve_write() {
    RESOLVER_WRITE.with(|cell| *cell.borrow_mut() = None);
}

#[wasm_bindgen(js_name = "convertLegacyCustomElementTemplate")]
pub fn convert_legacy_custom_element_template(source: &str) -> String {
    let result = crate::legacy_custom_element::convert_template_source(source);
    serde_json::to_string(&result).unwrap_or_else(|error| {
        serde_json::json!({
            "source": "",
            "diagnostics": [{
                "code": "legacy_xslt.wasm.serialize_failed",
                "message": format!("legacy conversion result could not be serialized: {error}")
            }]
        })
        .to_string()
    })
}

#[wasm_bindgen(js_name = "normalizeRunConfig")]
pub fn normalize_run_config(json: &str) -> String {
    let request = crate::run_config::RunConfigParseRequest {
        bytes: json.as_bytes().to_vec(),
        identity: crate::engine::FormatIdentity {
            content_type: Some("application/json".to_owned()),
            ..crate::engine::FormatIdentity::default()
        },
        base_uri: None,
    };
    match crate::run_config::parse_run_config(request) {
        Ok(response) => serde_json::to_string(&response).unwrap_or_else(wasm_serialize_error),
        Err(error) => serde_json::json!({
            "error": {
                "code": error.code,
                "message": error.to_string()
            }
        })
        .to_string(),
    }
}

#[wasm_bindgen(js_name = "parseInputSpecRecord")]
pub fn parse_input_spec_record(record: &str) -> String {
    match crate::run_config::parse_input_spec_record(record) {
        Ok(spec) => serde_json::to_string(&spec).unwrap_or_else(wasm_serialize_error),
        Err(error) => serde_json::json!({
            "error": {
                "code": "cem.run_config.invalid_input_spec",
                "message": error.to_string()
            }
        })
        .to_string(),
    }
}

#[wasm_bindgen(js_name = "parseOutputSpecRecord")]
pub fn parse_output_spec_record(record: &str) -> String {
    match crate::run_config::parse_output_spec_record(record) {
        Ok(spec) => serde_json::to_string(&spec).unwrap_or_else(wasm_serialize_error),
        Err(error) => serde_json::json!({
            "error": {
                "code": "cem.run_config.invalid_output_spec",
                "message": error.to_string()
            }
        })
        .to_string(),
    }
}

fn wasm_serialize_error(error: serde_json::Error) -> String {
    serde_json::json!({
        "error": {
            "code": "cem.run_config.serialize_failed",
            "message": error.to_string()
        }
    })
    .to_string()
}

/// `ResourceResolver` adapter that forwards read/write requests to
/// JavaScript callbacks registered with `onResolveRead` and
/// `onResolveWrite`.
///
/// Rust-side WASM entrypoints can install this resolver into an
/// `EngineContext` using [`resolver_registry_for_schemes`]. The JS read
/// callback receives a request JSON string and returns either a string
/// body or an object with `uri`, `bytes`, and optional `contentType`.
/// `bytes` may be a string, `Uint8Array`, `ArrayBuffer`, or numeric array.
/// The JS write callback receives request JSON plus a `Uint8Array` payload
/// and returns nothing, a URI string, or an object with an optional `uri`.
#[derive(Debug, Clone)]
pub struct JsResourceResolver;

impl ResourceResolver for JsResourceResolver {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        let Some(callback) = RESOLVER_READ.with(|cell| cell.borrow().clone()) else {
            return Err(unsupported(request));
        };
        let request_json = request_json(request)?;
        let response = callback
            .call1(&JsValue::NULL, &JsValue::from_str(&request_json))
            .map_err(|error| js_io(request, error_message(&error)))?;
        if response.is_null() || response.is_undefined() {
            return Err(unsupported(request));
        }
        read_response(request, response)
    }

    fn write(
        &self,
        request: &ResolveRequest,
        bytes: &[u8],
    ) -> Result<ResolvedWrite, ResolverDiagnostic> {
        let Some(callback) = RESOLVER_WRITE.with(|cell| cell.borrow().clone()) else {
            return Err(unsupported(request));
        };
        let request_json = request_json(request)?;
        let payload = Uint8Array::from(bytes);
        let response = callback
            .call2(
                &JsValue::NULL,
                &JsValue::from_str(&request_json),
                payload.as_ref(),
            )
            .map_err(|error| js_io(request, error_message(&error)))?;
        write_response(request, response)
    }
}

pub fn resolver_registry_for_schemes(schemes: &[&str]) -> ResolverRegistry {
    let mut registry = ResolverRegistry::new();
    for scheme in schemes {
        for purpose in RESOLVE_PURPOSES {
            registry.register(*scheme, purpose, ResolveDirection::Read, JsResourceResolver);
            registry.register(
                *scheme,
                purpose,
                ResolveDirection::Write,
                JsResourceResolver,
            );
        }
    }
    registry
}

pub fn context_with_resolver_schemes(schemes: &[&str]) -> crate::engine::EngineContext {
    crate::engine::EngineContext {
        resolver_registry: resolver_registry_for_schemes(schemes),
        ..crate::engine::EngineContext::default()
    }
}

const RESOLVE_PURPOSES: [ResolvePurpose; 6] = [
    ResolvePurpose::Config,
    ResolvePurpose::Input,
    ResolvePurpose::ModuleMap,
    ResolvePurpose::Output,
    ResolvePurpose::Report,
    ResolvePurpose::ObserveEvents,
];

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmResolveRequest<'a> {
    uri: &'a str,
    base_uri: Option<&'a str>,
    purpose: &'static str,
    direction: &'static str,
    content_type_hint: Option<&'a str>,
}

fn request_json(request: &ResolveRequest) -> Result<String, ResolverDiagnostic> {
    serde_json::to_string(&WasmResolveRequest {
        uri: &request.uri,
        base_uri: request.base_uri.as_deref(),
        purpose: request.purpose.as_str(),
        direction: request.direction.as_str(),
        content_type_hint: request.content_type_hint.as_deref(),
    })
    .map_err(|error| js_io(request, error.to_string()))
}

fn read_response(
    request: &ResolveRequest,
    response: JsValue,
) -> Result<ResolvedRead, ResolverDiagnostic> {
    if let Some(text) = response.as_string() {
        return Ok(ResolvedRead {
            uri: request.uri.clone(),
            bytes: text.into_bytes(),
            content_type: None,
        });
    }
    if let Some(message) = response_error(&response) {
        return Err(js_io(request, message));
    }

    let uri =
        optional_string_property(request, &response, "uri")?.unwrap_or_else(|| request.uri.clone());
    let bytes_value = property(request, &response, "bytes")?;
    if bytes_value.is_null() || bytes_value.is_undefined() {
        return Err(js_io(
            request,
            "resolver read response must include bytes".to_owned(),
        ));
    }
    let bytes = bytes_from_js(request, &bytes_value)?;
    let content_type = optional_string_property(request, &response, "contentType")?;
    Ok(ResolvedRead {
        uri,
        bytes,
        content_type,
    })
}

fn write_response(
    request: &ResolveRequest,
    response: JsValue,
) -> Result<ResolvedWrite, ResolverDiagnostic> {
    if response.is_null() || response.is_undefined() {
        return Ok(ResolvedWrite {
            uri: request.uri.clone(),
        });
    }
    if let Some(uri) = response.as_string() {
        return Ok(ResolvedWrite { uri });
    }
    if let Some(message) = response_error(&response) {
        return Err(js_io(request, message));
    }
    Ok(ResolvedWrite {
        uri: optional_string_property(request, &response, "uri")?
            .unwrap_or_else(|| request.uri.clone()),
    })
}

fn bytes_from_js(request: &ResolveRequest, value: &JsValue) -> Result<Vec<u8>, ResolverDiagnostic> {
    if let Some(text) = value.as_string() {
        return Ok(text.into_bytes());
    }
    if value.is_instance_of::<Uint8Array>()
        || value.is_instance_of::<ArrayBuffer>()
        || Array::is_array(value)
    {
        let array = Uint8Array::new(value);
        let mut bytes = vec![0; array.length() as usize];
        array.copy_to(&mut bytes);
        return Ok(bytes);
    }
    Err(js_io(
        request,
        "resolver read response bytes must be a string, Uint8Array, ArrayBuffer, or number array"
            .to_owned(),
    ))
}

fn property(
    request: &ResolveRequest,
    value: &JsValue,
    name: &str,
) -> Result<JsValue, ResolverDiagnostic> {
    Reflect::get(value, &JsValue::from_str(name)).map_err(|error| {
        js_io(
            request,
            format!(
                "resolver response property `{name}` could not be read: {}",
                error_message(&error)
            ),
        )
    })
}

fn optional_string_property(
    request: &ResolveRequest,
    value: &JsValue,
    name: &str,
) -> Result<Option<String>, ResolverDiagnostic> {
    let property = property(request, value, name)?;
    if property.is_null() || property.is_undefined() {
        return Ok(None);
    }
    property.as_string().map(Some).ok_or_else(|| {
        js_io(
            request,
            format!("resolver response property `{name}` must be a string"),
        )
    })
}

fn response_error(value: &JsValue) -> Option<String> {
    let error = Reflect::get(value, &JsValue::from_str("error")).ok()?;
    if error.is_null() || error.is_undefined() {
        return None;
    }
    Some(error_message(&error))
}

fn unsupported(request: &ResolveRequest) -> ResolverDiagnostic {
    ResolverDiagnostic::UnsupportedResolver {
        uri: request.uri.clone(),
        purpose: request.purpose,
        direction: request.direction,
    }
}

fn js_io(request: &ResolveRequest, message: String) -> ResolverDiagnostic {
    ResolverDiagnostic::Io {
        uri: request.uri.clone(),
        message,
    }
}

fn error_message(error: &JsValue) -> String {
    if let Some(message) = error.as_string() {
        return message;
    }
    js_sys::JSON::stringify(error)
        .ok()
        .and_then(|value| value.as_string())
        .unwrap_or_else(|| "JavaScript resolver callback failed".to_owned())
}

/// `EngineObserver` adapter that forwards every event to whichever
/// JS callback is currently registered through `onParseEvent` /
/// `onValidate` / `onTransform`. Embedders pass `&JsObserver` to
/// `observe_pipeline` so JS code sees the events on the registered
/// channels.
pub struct JsObserver;

impl EngineObserver for JsObserver {
    fn on_parse_event(&self, event: &ReportEvent) {
        dispatch_parse(event);
    }
    fn on_validate(&self, event: &ReportEvent) {
        dispatch_validate(event);
    }
    fn on_transform(&self, event: &ReportEvent) {
        dispatch_transform(event);
    }
}

fn dispatch_parse(event: &ReportEvent) {
    let callback = PARSE_OBSERVER.with(|cell| cell.borrow().clone());
    invoke(callback.as_ref(), event);
}

fn dispatch_validate(event: &ReportEvent) {
    let callback = VALIDATE_OBSERVER.with(|cell| cell.borrow().clone());
    invoke(callback.as_ref(), event);
}

fn dispatch_transform(event: &ReportEvent) {
    let callback = TRANSFORM_OBSERVER.with(|cell| cell.borrow().clone());
    invoke(callback.as_ref(), event);
}

fn invoke(callback: Option<&Function>, event: &ReportEvent) {
    let Some(callback) = callback else { return };
    let json = match serde_json::to_string(event) {
        Ok(s) => s,
        Err(_) => return,
    };
    // Ignore the JS-side return value and any thrown error — observer
    // callbacks are fire-and-forget per AC-O-1; engine work must not
    // abort because a JS observer threw.
    let _ = callback.call1(&JsValue::NULL, &JsValue::from_str(&json));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_input_spec_record_preserves_namespace_scope() {
        let json = parse_input_spec_record(
            "uri=src/a.data,defaultNs=http://www.w3.org/1999/xhtml,namespaces=svg:http://www.w3.org/2000/svg",
        );
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["uri"], "src/a.data");
        assert_eq!(
            value["rootScope"]["defaultNamespace"],
            "http://www.w3.org/1999/xhtml"
        );
        assert_eq!(
            value["rootScope"]["namespaces"]["svg"],
            "http://www.w3.org/2000/svg"
        );
    }

    #[test]
    fn parse_output_spec_record_preserves_namespace_target_scope() {
        let json = parse_output_spec_record(
            "dest=dist/a.out,defaultNs=https://cem.dev/ns/core/1,namespaces=html:http://www.w3.org/1999/xhtml",
        );
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["destination"], "dist/a.out");
        assert_eq!(
            value["rootScope"]["defaultNamespace"],
            "https://cem.dev/ns/core/1"
        );
        assert_eq!(
            value["rootScope"]["namespaces"]["html"],
            "http://www.w3.org/1999/xhtml"
        );
    }

    #[test]
    fn normalize_run_config_preserves_input_and_output_namespace_scopes() {
        let json = normalize_run_config(
            r#"{
                "inputs": [{
                    "uri": "src/a.data",
                    "rootScope": {
                        "defaultNamespace": "http://www.w3.org/1999/xhtml"
                    }
                }],
                "outputs": [{
                    "destination": "dist/a.out",
                    "rootScope": {
                        "namespaces": {
                            "svg": "http://www.w3.org/2000/svg"
                        }
                    }
                }]
            }"#,
        );
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(
            value["config"]["inputs"][0]["rootScope"]["defaultNamespace"],
            "http://www.w3.org/1999/xhtml"
        );
        assert_eq!(
            value["config"]["outputs"][0]["rootScope"]["namespaces"]["svg"],
            "http://www.w3.org/2000/svg"
        );
        assert_eq!(value["diagnostics"], serde_json::json!([]));
    }

    #[test]
    fn normalize_run_config_preserves_uri_shaped_paths_at_api_boundary() {
        let json = normalize_run_config(
            r#"{
                "inputs": [{
                    "uri": "https://example.test/src/a.cem",
                    "rootScope": {
                        "moduleMap": "cem+vfs://workspace/cem.modules.json"
                    }
                }],
                "outputs": [{
                    "inputRef": "https://example.test/src/a.cem",
                    "destination": "file://example.test/dist/a.json",
                    "rootScope": {
                        "moduleMap": "https://example.test/out.modules.json"
                    }
                }]
            }"#,
        );
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(
            value["config"]["inputs"][0]["uri"],
            "https://example.test/src/a.cem"
        );
        assert_eq!(
            value["config"]["inputs"][0]["rootScope"]["moduleMap"],
            "cem+vfs://workspace/cem.modules.json"
        );
        assert_eq!(
            value["config"]["outputs"][0]["destination"],
            "file://example.test/dist/a.json"
        );
        assert_eq!(
            value["config"]["outputs"][0]["rootScope"]["moduleMap"],
            "https://example.test/out.modules.json"
        );
        assert_eq!(value["diagnostics"], serde_json::json!([]));
    }
}
