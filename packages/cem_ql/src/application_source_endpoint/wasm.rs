//! No JS constructor/provisioning or numeric owner handles. The original native
//! embedding installs a weak authority; source tokens remain in this exact heap.
use super::*;
use cem_ml::{runtime_admission, runtime_inventory::NativeRegistry};
use wasm_bindgen::prelude::*;
struct Session {
    token: JsValue,
    session: ApplicationSourceSession,
    inputs: Vec<(JsValue, ApplicationReceiverInput)>,
}
thread_local! {
    static ENDPOINT: NativeRegistry<Option<Weak<ApplicationSourceEndpoint>>> = const { NativeRegistry::new(None) };
    static SESSIONS: NativeRegistry<Vec<Session>> = const { NativeRegistry::new(Vec::new()) };
}
fn error(_: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(INVALID)
}
/// Original registered native source only. There is no JS token constructor.
pub fn register_receiver_input(
    owner: &JsValue,
    tag: &str,
    root: crate::eval::RetainedCemNode,
) -> Result<JsValue, String> {
    let _entry = runtime_admission::admit(false).map_err(|_| INVALID)?;
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.try_borrow_mut().map_err(|_| INVALID)?;
        let entry = sessions
            .iter_mut()
            .find(|entry| &entry.token == owner)
            .ok_or(INVALID)?;
        if entry.inputs.len() >= 64 {
            return Err(INVALID.into());
        }
        let input = entry.session.register_receiver_input(tag, root)?;
        let token: JsValue = js_sys::Object::freeze(&js_sys::Object::new()).into();
        entry.inputs.push((token.clone(), input));
        Ok(token)
    })
}

/// Rust-only capture on the original opaque session/input pair.
#[cfg(feature = "application-source-runtime")]
pub fn capture_receiver_checkpoint(
    owner: &JsValue,
    input: &JsValue,
    sources: &[&transport::SourceInput],
    available_bytes: usize,
    control: &OperationControl,
    may_capture: impl Fn(&transport::SourceInput) -> bool,
) -> Result<(Vec<u8>, ReceiverCheckpointSeal), String> {
    SESSIONS.with(|sessions| {
        let sessions = sessions.try_borrow_mut().map_err(|_| INVALID)?;
        let entry = sessions.iter().find(|entry| &entry.token == owner).ok_or(INVALID)?;
        let (_, input) = entry.inputs.iter().find(|(key, _)| key == input).ok_or(INVALID)?;
        entry.session.capture_receiver_checkpoint(input, sources, available_bytes, control, may_capture)
    })
}

/// Confirm and release the exact original session in one native call. The
/// retained pin belongs to the trusted physical recovery transaction.
#[cfg(feature = "application-source-runtime")]
pub fn retire_receiver_checkpoint(
    owner: &JsValue,
    seal: ReceiverCheckpointSeal,
    bytes: &[u8],
    journal: &[u8],
    runtime_id: &str,
    instance_id: &str,
) -> Result<ReceiverRecoveryPin, String> {
    let _entry = runtime_admission::admit(false).map_err(|_| INVALID)?;
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.try_borrow_mut().map_err(|_| INVALID)?;
        let index = sessions.iter().position(|entry| &entry.token == owner).ok_or(INVALID)?;
        let pin = sessions[index].session.confirm_receiver_retirement(seal, bytes, journal, runtime_id, instance_id)?;
        sessions.swap_remove(index);
        Ok(pin)
    })
}

/// The trusted embedding must prepare the fresh session with the decoded
/// owners before this function can enroll a new opaque receiver token.
#[cfg(feature = "application-source-runtime")]
pub fn restore_receiver_checkpoint(
    owner: &JsValue,
    checkpoint: VerifiedReceiverCheckpoint,
    available_bytes: usize,
) -> Result<JsValue, String> {
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.try_borrow_mut().map_err(|_| INVALID)?;
        let entry = sessions.iter_mut().find(|entry| &entry.token == owner).ok_or(INVALID)?;
        if !entry.inputs.is_empty() {
            return Err(INVALID.into());
        }
        let input = entry.session.restore_receiver_checkpoint(checkpoint, available_bytes)?;
        let token: JsValue = js_sys::Object::freeze(&js_sys::Object::new()).into();
        entry.inputs.push((token.clone(), input));
        Ok(token)
    })
}
#[wasm_bindgen(js_name=applicationSourceRenderReceiverV1)]
pub fn render_receiver(
    owner: &JsValue,
    input: &JsValue,
    instance: &str,
) -> Result<String, JsValue> {
    let _entry = runtime_admission::admit(false).map_err(error)?;
    let _memory = cem_ml::runtime_memory::admit_input(instance.len()).map_err(error)?;
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.try_borrow_mut().map_err(error)?;
        let entry = sessions
            .iter_mut()
            .find(|entry| &entry.token == owner)
            .ok_or_else(|| error(INVALID))?;
        let input = &entry
            .inputs
            .iter()
            .find(|(key, _)| key == input)
            .ok_or_else(|| error(INVALID))?
            .1;
        entry
            .session
            .render_receiver_native(input, instance)
            .map_err(error)
    })
}
#[wasm_bindgen(js_name=applicationSourceProjectReceiverV1)]
pub fn project_receiver(
    owner: &JsValue,
    input: &JsValue,
    instance: &str,
) -> Result<Vec<u8>, JsValue> {
    let _entry = runtime_admission::admit(false).map_err(error)?;
    let _memory = cem_ml::runtime_memory::admit_input(instance.len()).map_err(error)?;
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.try_borrow_mut().map_err(error)?;
        let entry = sessions
            .iter_mut()
            .find(|entry| &entry.token == owner)
            .ok_or_else(|| error(INVALID))?;
        let input = &entry
            .inputs
            .iter()
            .find(|(key, _)| key == input)
            .ok_or_else(|| error(INVALID))?
            .1;
        entry
            .session
            .project_receiver_native(input, instance)
            .map_err(error)
    })
}
#[wasm_bindgen(js_name=applicationSourceReleaseReceiverInputV1)]
pub fn release_receiver(owner: &JsValue, input: &JsValue) -> Result<(), JsValue> {
    let _entry = runtime_admission::admit(true).map_err(error)?;
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.try_borrow_mut().map_err(error)?;
        let entry = sessions
            .iter_mut()
            .find(|entry| &entry.token == owner)
            .ok_or_else(|| error(INVALID))?;
        let index = entry
            .inputs
            .iter()
            .position(|(key, _)| key == input)
            .ok_or_else(|| error(INVALID))?;
        entry.inputs.swap_remove(index);
        Ok(())
    })
}
/// Trusted Rust bootstrap only. Bytes and JS metadata cannot install an owner.
pub fn install(endpoint: &Arc<ApplicationSourceEndpoint>) -> Result<(), String> {
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
pub fn uninstall(endpoint: &Arc<ApplicationSourceEndpoint>) -> Result<(), String> {
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
    use_session: impl FnOnce(&mut ApplicationSourceSession) -> Result<T, String>,
) -> Result<T, String> {
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.try_borrow_mut().map_err(|_| INVALID)?;
        let session = sessions
            .iter_mut()
            .find(|entry| &entry.token == owner)
            .ok_or(INVALID)?;
        session.session.check()?;
        let result = use_session(&mut session.session);
        session.session.check()?;
        result
    })
}
#[wasm_bindgen(js_name = applicationSourceOpenV1)]
pub fn open(bytes: &[u8], pins: &str) -> Result<JsValue, JsValue> {
    let _entry = runtime_admission::admit(false).map_err(error)?;
    if bytes.len() > transport::MAX_BYTES || pins.len() > 16384 {
        return Err(error(INVALID));
    }
    let _memory = cem_ml::runtime_memory::admit_input(bytes.len().saturating_add(pins.len()))
        .map_err(error)?;
    let endpoint = ENDPOINT
        .with(|s| s.borrow().as_ref().and_then(Weak::upgrade))
        .ok_or_else(|| error(INVALID))?;
    SESSIONS.with(|s| {
        let mut sessions = s.try_borrow_mut().map_err(error)?;
        if sessions.len() >= MAX_SESSIONS {
            return Err(error(INVALID));
        }
        let session = endpoint.open(bytes, pins).map_err(error)?;
        let owner: JsValue = js_sys::Object::freeze(&js_sys::Object::new()).into();
        sessions.push(Session {
            token: owner.clone(),
            session,
            inputs: vec![],
        });
        Ok(owner)
    })
}
#[wasm_bindgen(js_name = applicationSourceCaptureV1)]
pub fn capture(owner: &JsValue) -> Result<Vec<u8>, JsValue> {
    let _entry = runtime_admission::admit(false).map_err(error)?;
    with_session(owner, |s| s.capture()).map_err(error)
}
#[wasm_bindgen(js_name = applicationSourceReleaseV1)]
pub fn release(owner: &JsValue) -> Result<(), JsValue> {
    let _entry = runtime_admission::admit(true).map_err(error)?;
    SESSIONS.with(|s| {
        let mut sessions = s.try_borrow_mut().map_err(error)?;
        let index = sessions
            .iter()
            .position(|entry| &entry.token == owner)
            .ok_or_else(|| error(INVALID))?;
        sessions.swap_remove(index);
        Ok(())
    })
}
#[wasm_bindgen(js_name=applicationSourceRenderIslandV1)]
pub fn render_island(
    owner: &JsValue,
    tag: &str,
    bytes: &[u8],
    instance: &str,
) -> Result<String, JsValue> {
    let _entry = runtime_admission::admit(false).map_err(error)?;
    let _memory = cem_ml::runtime_memory::admit_input(bytes.len()).map_err(error)?;
    with_session(owner, |s| s.render_island_native(tag, bytes, instance)).map_err(error)
}
#[wasm_bindgen(js_name=applicationSourceProjectIslandV1)]
pub fn project_island(
    owner: &JsValue,
    tag: &str,
    bytes: &[u8],
    instance: &str,
) -> Result<Vec<u8>, JsValue> {
    let _entry = runtime_admission::admit(false).map_err(error)?;
    let _memory = cem_ml::runtime_memory::admit_input(bytes.len()).map_err(error)?;
    with_session(owner, |s| s.project_island_native(tag, bytes, instance)).map_err(error)
}
#[wasm_bindgen(js_name=applicationSourceRenderInputsV1)]
pub fn render_inputs(
    owner: &JsValue,
    tag: &str,
    bytes: &[u8],
    routing: &str,
    instance: &str,
) -> Result<String, JsValue> {
    let _entry = runtime_admission::admit(false).map_err(error)?;
    let _memory = cem_ml::runtime_memory::admit_input(bytes.len().saturating_add(routing.len()))
        .map_err(error)?;
    with_session(owner, |s| {
        s.render_inputs_native(tag, bytes, routing, instance)
    })
    .map_err(error)
}
