//! No JS constructor/provisioning or numeric owner handles. The original native
//! embedding installs a weak authority; source tokens remain in this exact heap.
use super::*;
use cem_ml::{runtime_admission, runtime_inventory::NativeRegistry};
use wasm_bindgen::prelude::*;
thread_local! {
    static ENDPOINT: NativeRegistry<Option<Weak<QuerySourceEndpoint>>> = const { NativeRegistry::new(None) };
    static SESSIONS: NativeRegistry<Vec<(JsValue, QuerySourceSession)>> = const { NativeRegistry::new(Vec::new()) };
}
fn error(_: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(INVALID)
}
/// Trusted Rust bootstrap only. Bytes and JS metadata cannot install an owner.
pub fn install(endpoint: &Arc<QuerySourceEndpoint>) -> Result<(), String> {
    let _entry = runtime_admission::admit(false).map_err(|_| INVALID)?;
    let actual = runtime_admission::linked_generation().map_err(|_| INVALID)?;
    if !endpoint
        .generation
        .upgrade()
        .is_some_and(|g| Arc::ptr_eq(&g, &actual))
    {
        return Err(INVALID.into());
    }
    endpoint.check()?;
    ENDPOINT.with(|slot| {
        if slot.borrow().is_some() || SESSIONS.with(|s| !s.borrow().is_empty()) {
            return Err(INVALID.into());
        }
        *slot.borrow_mut() = Some(Arc::downgrade(endpoint));
        Ok(())
    })
}
/// Cleanup behind closed admission still requires the actual original owner.
pub fn uninstall(endpoint: &Arc<QuerySourceEndpoint>) -> Result<(), String> {
    let _entry = runtime_admission::admit(true).map_err(|_| INVALID)?;
    ENDPOINT.with(|slot| {
        if !slot
            .borrow()
            .as_ref()
            .is_some_and(|s| Weak::ptr_eq(s, &Arc::downgrade(endpoint)))
        {
            return Err(INVALID.into());
        }
        endpoint.retire();
        *slot.borrow_mut() = None;
        Ok(())
    })
}
/// An embedding can provide typed input to its own native session without
/// exporting JS data objects, raw session pointers or a new evaluation ABI.
pub fn with_session<T>(
    owner: &JsValue,
    use_session: impl FnOnce(&QuerySourceSession) -> Result<T, String>,
) -> Result<T, String> {
    SESSIONS.with(|sessions| {
        let sessions = sessions.borrow();
        let session = sessions
            .iter()
            .find(|(key, _)| key == owner)
            .ok_or(INVALID)?;
        use_session(&session.1)
    })
}
#[wasm_bindgen(js_name = querySourceOpenV1)]
pub fn open(bytes: &[u8], pins: &str) -> Result<JsValue, JsValue> {
    let _entry = runtime_admission::admit(false).map_err(error)?;
    if bytes.len() > handoff::MAX_BYTES || pins.len() > MAX_PINS_BYTES {
        return Err(error(INVALID));
    }
    let _memory = cem_ml::runtime_memory::admit_input(bytes.len().saturating_add(pins.len()))
        .map_err(error)?;
    let endpoint = ENDPOINT
        .with(|s| s.borrow().as_ref().and_then(Weak::upgrade))
        .ok_or_else(|| error(INVALID))?;
    SESSIONS.with(|s| {
        let mut sessions = s.borrow_mut();
        if sessions.len() >= MAX_SESSIONS {
            return Err(error(INVALID));
        }
        let session = endpoint.open(bytes, pins).map_err(error)?;
        let owner: JsValue = js_sys::Object::freeze(&js_sys::Object::new()).into();
        sessions.push((owner.clone(), session));
        Ok(owner)
    })
}
#[wasm_bindgen(js_name = querySourceCaptureV1)]
pub fn capture(owner: &JsValue) -> Result<Vec<u8>, JsValue> {
    let _entry = runtime_admission::admit(false).map_err(error)?;
    with_session(owner, QuerySourceSession::capture).map_err(error)
}
#[wasm_bindgen(js_name = querySourceReleaseV1)]
pub fn release(owner: &JsValue) -> Result<(), JsValue> {
    let _entry = runtime_admission::admit(true).map_err(error)?;
    SESSIONS.with(|s| {
        let mut sessions = s.borrow_mut();
        let index = sessions
            .iter()
            .position(|(key, _)| key == owner)
            .ok_or_else(|| error(INVALID))?;
        sessions.swap_remove(index);
        Ok(())
    })
}
