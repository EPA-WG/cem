//! WASM-callable observer surface (AC-O-1 / AC-C-1).
//!
//! Exposes the named event channels — `onParseEvent`, `onValidate`,
//! `onTransform` — to JS callers through `wasm-bindgen`. Each
//! registration replaces the previously-installed callback for that
//! channel; the matching `off*` function clears it. Callbacks receive
//! the event as a JSON string (the canonical wire form documented in
//! `cem-ml-stack-design-impl.md` §3.12.1 and modelled by
//! `packages/cem_ml/schema/observability/report-event.schema.json`).
//!
//! ```js
//! import init, { onParseEvent, offParseEvent } from "@epa-wg/cem-ml/wasm";
//!
//! await init();
//! onParseEvent((json) => {
//!   const event = JSON.parse(json);
//!   console.log(event.channel, event.sequence, event.byteOffset);
//! });
//! // later:
//! offParseEvent();
//! ```
//!
//! [`JsObserver`] is the `EngineObserver` adapter that dispatches each
//! event through whichever JS callback is currently registered.
//! Embedders running the pipeline from Rust-side WASM code pass it as
//! the observer to `observe_pipeline`.

use std::cell::RefCell;

use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::observability::{EngineObserver, ReportEvent};

thread_local! {
    static PARSE_OBSERVER: RefCell<Option<Function>> = const { RefCell::new(None) };
    static VALIDATE_OBSERVER: RefCell<Option<Function>> = const { RefCell::new(None) };
    static TRANSFORM_OBSERVER: RefCell<Option<Function>> = const { RefCell::new(None) };
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
    PARSE_OBSERVER.with(|cell| invoke(cell.borrow().as_ref(), event));
}

fn dispatch_validate(event: &ReportEvent) {
    VALIDATE_OBSERVER.with(|cell| invoke(cell.borrow().as_ref(), event));
}

fn dispatch_transform(event: &ReportEvent) {
    TRANSFORM_OBSERVER.with(|cell| invoke(cell.borrow().as_ref(), event));
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
