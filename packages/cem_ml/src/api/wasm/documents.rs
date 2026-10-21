//! Opaque document handles; only bytes enter and control metadata leaves WASM.
use crate::{import::documents::CemDocuments, parser::tree::RetainedCemTree};
use std::sync::Arc;
use crate::runtime_inventory::NativeRegistry as RefCell;
use wasm_bindgen::prelude::*;

thread_local! { static DOCUMENTS: RefCell<CemDocuments> = RefCell::default(); }

pub fn retained_cem_document(id: u32) -> Option<Arc<RetainedCemTree>> {
    DOCUMENTS.with(|documents| documents.borrow().get(id))
}

#[wasm_bindgen(js_name = "retainCemDocument")]
pub fn retain_cem_document(bytes: &[u8], content_type: &str, uri: &str) -> Result<u32, JsValue> {
    let _native = match crate::runtime_admission::admit(false) { Ok(permit) => permit, Err(error) => return Err(JsValue::from_str(error)) };
    let _memory = crate::runtime_memory::admit_input(bytes.len()).map_err(|error|JsValue::from_str(error.0))?;
    DOCUMENTS
        .with(|documents| documents.borrow_mut().retain(bytes, content_type, uri))
        .map_err(|message| JsValue::from_str(&message))
}

#[wasm_bindgen(js_name = "disposeCemDocument")]
pub fn dispose_cem_document(id: u32) -> bool {
    let _native = match crate::runtime_admission::admit(true) { Ok(permit) => permit, Err(_error) => return false };
    DOCUMENTS.with(|documents| documents.borrow_mut().dispose(id))
}

pub(super) fn inventory_count() -> u64 { DOCUMENTS.with(|s| s.borrow().inventory_count() as u64) }
