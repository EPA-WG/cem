//! Opaque document handles; only bytes enter and control metadata leaves WASM.
use crate::{import::documents::CemDocuments, parser::tree::RetainedCemTree};
use std::{cell::RefCell, sync::Arc};
use wasm_bindgen::prelude::*;

thread_local! { static DOCUMENTS: RefCell<CemDocuments> = RefCell::default(); }

pub fn retained_cem_document(id: u32) -> Option<Arc<RetainedCemTree>> {
    DOCUMENTS.with(|documents| documents.borrow().get(id))
}

#[wasm_bindgen(js_name = "retainCemDocument")]
pub fn retain_cem_document(bytes: &[u8], content_type: &str, uri: &str) -> Result<u32, JsValue> {
    DOCUMENTS
        .with(|documents| documents.borrow_mut().retain(bytes, content_type, uri))
        .map_err(|message| JsValue::from_str(&message))
}

#[wasm_bindgen(js_name = "disposeCemDocument")]
pub fn dispose_cem_document(id: u32) -> bool {
    DOCUMENTS.with(|documents| documents.borrow_mut().dispose(id))
}
