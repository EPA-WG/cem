//! Passive test-only reader. No production installation or owner provisioning.
#![cfg(target_arch = "wasm32")]
use cem_ml::operation_control::OperationControl;
use cem_ql::application_source_transport::{self, ApplicationSourcePin};
use wasm_bindgen::prelude::*;
#[path = "../fixtures/application-handoff/runtime.rs"]
mod installed;
use cem_ql::application_source_endpoint::{
    wasm, ApplicationBootstrapOwner, ApplicationSourceEndpoint,
};
use std::sync::Arc;
thread_local! {static DEPLOYMENT:cem_ml::runtime_inventory::NativeRegistry<Option<(Arc<ApplicationBootstrapOwner>,Arc<ApplicationSourceEndpoint>,Arc<installed::Inputs>)>>=const{cem_ml::runtime_inventory::NativeRegistry::new(None)};}
// Fixture-only provisioning. Library ABI installation remains Rust-only.
#[wasm_bindgen(js_name=probeApplicationProvision)]
pub fn provision(pins: &str, input: &[u8]) -> Result<(), JsValue> {
    let error = |e: String| JsValue::from_str(&e);
    if pins.len() > 16384 {
        return Err(error("fixture pins".into()));
    }
    let pin: ApplicationSourcePin = serde_json::from_str(pins).map_err(|e| error(e.to_string()))?;
    let g = cem_ml::runtime_admission::linked_generation().map_err(|e| error(e.into()))?;
    let control = OperationControl::default();
    let inputs = installed::Inputs::restore(input, &control).map_err(error)?;
    let code = ApplicationBootstrapOwner::new(
        &g,
        control,
        pin,
        installed::Bootstrap {
            inputs: inputs.clone(),
        },
    )
    .map_err(error)?;
    let endpoint = ApplicationSourceEndpoint::new(&code).map_err(error)?;
    wasm::install(&endpoint).map_err(error)?;
    DEPLOYMENT.with(|s| *s.borrow_mut() = Some((code, endpoint, inputs)));
    Ok(())
}
#[wasm_bindgen(js_name=probeApplicationUnprovision)]
pub fn unprovision() -> Result<(), JsValue> {
    DEPLOYMENT.with(|s| {
        let mut s = s.borrow_mut();
        let (_, e, _) = s
            .as_ref()
            .ok_or_else(|| JsValue::from_str("missing fixture"))?;
        wasm::uninstall(e).map_err(|e| JsValue::from_str(&e))?;
        s.take();
        Ok(())
    })
}
#[wasm_bindgen(js_name=probeApplicationValidate)]
pub fn validate_owned(owner: &JsValue) -> Result<String, JsValue> {
    let inputs = DEPLOYMENT
        .with(|s| s.borrow().as_ref().map(|s| s.2.clone()))
        .ok_or_else(|| JsValue::from_str("missing fixture"))?;
    wasm::with_session(owner, |s| installed::validate(s, &inputs))
        .map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen(js_name=probeApplicationRetire)]
pub fn retire() {
    DEPLOYMENT.with(|s| {
        if let Some((code, _, _)) = s.borrow().as_ref() {
            code.retire()
        }
    });
}
#[wasm_bindgen(js_name=probeApplicationReentrant)]
pub fn reentrant(owner: &JsValue) -> Result<bool, JsValue> {
    wasm::with_session(owner, |_| {
        Ok(wasm::capture(owner).is_err() && wasm::release(owner).is_err())
    })
    .map_err(|e| JsValue::from_str(&e))
}

// Fixture embedding alone selects an original, already registered native root.
#[wasm_bindgen(js_name=probeApplicationReceiverInput)]
pub fn receiver_input(owner: &JsValue) -> Result<JsValue, JsValue> {
    let root = DEPLOYMENT
        .with(|s| s.borrow().as_ref().map(|s| s.2.root()))
        .ok_or_else(|| JsValue::from_str("missing fixture"))?;
    wasm::register_receiver_input(owner, "cem-receiver-frame", root)
        .map_err(|e| JsValue::from_str(&e))
}

// Native projection assertion only: never expose the retained graph as JS AST.
#[wasm_bindgen(js_name=probeApplicationProjectionValid)]
pub fn projection_valid(bytes: &[u8]) -> bool {
    cem_ml::value::artifact::CemValueGraph::decode(bytes, &Default::default()).is_ok_and(|graph| {
        graph.records.iter().any(|n| {
            n.name == "strong"
                && n.namespace == "http://www.w3.org/1999/xhtml"
                && n.provenance.as_ref().and_then(|p| p.source_uri.as_deref())
                    == Some("app://runtime/island.xhtml")
        })
    })
}

#[wasm_bindgen(js_name = probeApplicationSources)]
pub fn verify(bytes: &[u8], pins: &str, cancelled: bool) -> Result<String, JsValue> {
    let error = |e: String| JsValue::from_str(&e);
    if pins.len() > 16 * 1024 {
        return Err(error("fixture pin limit".into()));
    }
    let pin: ApplicationSourcePin = serde_json::from_str(pins).map_err(|e| error(e.to_string()))?;
    let control = OperationControl::default();
    if cancelled {
        control
            .cancel_root(Some("fixture cancellation".into()), None)
            .map_err(|e| error(format!("{e:?}")))?;
    }
    let verified = application_source_transport::verify(bytes, &pin, &control).map_err(error)?;
    let (base, mixed) = verified.receivers().map_err(error)?;
    Ok(format!(
        "{}:{}:{}:{}",
        verified.application().component_count(),
        verified.claims().len(),
        base.len(),
        mixed.len()
    ))
}
