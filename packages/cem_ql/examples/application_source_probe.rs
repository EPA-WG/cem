//! Passive test-only reader. No production installation or owner provisioning.
#![cfg(target_arch = "wasm32")]
use cem_ml::operation_control::OperationControl;
use cem_ql::application_source_transport::{self, ApplicationSourcePin};
use wasm_bindgen::prelude::*;
#[path = "../fixtures/application-handoff/runtime.rs"]
mod installed;
use cem_ml::{
    committed_checkpoint::MAX_WORKSPACE_CHECKPOINT_BYTES,
    runtime_admission::generation::NativeGeneration,
};
use cem_ql::application_source_endpoint::{
    wasm, ApplicationBootstrap, ApplicationBootstrapOwner, ApplicationBrowserBehavior, ApplicationSourceEndpoint,
    ApplicationTypedReceiverProducer, PreparedApplication, ReceiverCheckpointSeal,
    ReceiverRecoveryPin, ReceiverWriteJournal,
};
use std::sync::{Arc, Mutex};

struct MutableBootstrap {
    inputs: Arc<Mutex<Arc<installed::Inputs>>>,
    redirected_styles: Option<bool>,
    browser: Option<ApplicationBrowserBehavior>,
}

impl ApplicationBootstrap for MutableBootstrap {
    fn prepare(
        &self,
        sources: Arc<application_source_transport::VerifiedApplicationSources>,
        generation: &Arc<NativeGeneration>,
        control: OperationControl,
    ) -> Result<PreparedApplication, String> {
        let inputs = self.inputs.lock().map_err(|_| "fixture inputs")?.clone();
        let staged = installed::Bootstrap { inputs }.prepare(sources, generation, control)?;
        let staged = match self.redirected_styles {
            Some(redirected) => staged.with_stylesheet_policy(
                installed::style_capability(redirected),
                Default::default(),
            ),
            None => Ok(staged),
        }?;
        match self.browser {
            Some(behavior @ ApplicationBrowserBehavior::NavigationLandmark) => staged.with_browser_behavior("cem-count", behavior),
            Some(behavior @ ApplicationBrowserBehavior::FormControl) => staged.with_browser_behavior("cem-receiver-frame", behavior),
            None => Ok(staged),
        }
    }
}

struct Deployment {
    code: Arc<ApplicationBootstrapOwner>,
    endpoint: Arc<ApplicationSourceEndpoint>,
    inputs: Arc<Mutex<Arc<installed::Inputs>>>,
    captured: Option<(JsValue, ReceiverCheckpointSeal)>,
    journal: Option<(JsValue, ReceiverWriteJournal)>,
    typed_producer: Option<ApplicationTypedReceiverProducer>,
    browser: Option<ApplicationBrowserBehavior>,
}
thread_local! {
    static DEPLOYMENT: cem_ml::runtime_inventory::NativeRegistry<Option<Deployment>> =
        const { cem_ml::runtime_inventory::NativeRegistry::new(None) };
}

fn fixture_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

fn original_inputs() -> Result<Arc<installed::Inputs>, JsValue> {
    DEPLOYMENT.with(|slot| {
        let slot = slot.borrow();
        let state = slot
            .as_ref()
            .ok_or_else(|| fixture_error("missing fixture"))?;
        let inputs = state.inputs.lock().map_err(fixture_error)?.clone();
        Ok(inputs)
    })
}
// Fixture-only provisioning. Library ABI installation remains Rust-only.
#[wasm_bindgen(js_name=probeApplicationProvision)]
pub fn provision(pins: &str, input: &[u8]) -> Result<(), JsValue> {
    provision_fixture(pins, input, None, None)
}

#[wasm_bindgen(js_name = probeApplicationProvisionStyled)]
pub fn provision_styled(pins: &str, input: &[u8], redirected: bool) -> Result<(), JsValue> {
    provision_fixture(pins, input, Some(redirected), None)
}

#[wasm_bindgen(js_name = probeApplicationProvisionNavigation)]
pub fn provision_navigation(pins: &str, input: &[u8]) -> Result<(), JsValue> {
    provision_fixture(pins, input, None, Some(ApplicationBrowserBehavior::NavigationLandmark))
}

#[wasm_bindgen(js_name = probeApplicationProvisionProvider)]
pub fn provision_provider(pins: &str, input: &[u8]) -> Result<(), JsValue> {
    provision_fixture(pins, input, None, Some(ApplicationBrowserBehavior::FormControl))
}

fn provision_fixture(
    pins: &str,
    input: &[u8],
    redirected_styles: Option<bool>,
    browser: Option<ApplicationBrowserBehavior>,
) -> Result<(), JsValue> {
    let error = |e: String| JsValue::from_str(&e);
    if pins.len() > 16384 {
        return Err(error("fixture pins".into()));
    }
    let pin: ApplicationSourcePin = serde_json::from_str(pins).map_err(|e| error(e.to_string()))?;
    let g = cem_ml::runtime_admission::linked_generation().map_err(|e| error(e.into()))?;
    let control = OperationControl::default();
    let inputs = Arc::new(Mutex::new(
        installed::Inputs::restore(input, &control).map_err(error)?,
    ));
    let code = ApplicationBootstrapOwner::new(
        &g,
        control,
        pin,
        MutableBootstrap {
            inputs: inputs.clone(),
            redirected_styles,
            browser,
        },
    )
    .map_err(error)?;
    let endpoint = ApplicationSourceEndpoint::new(&code).map_err(error)?;
    wasm::install(&endpoint).map_err(error)?;
    DEPLOYMENT.with(|s| {
        *s.borrow_mut() = Some(Deployment {
            code,
            endpoint,
            inputs,
            captured: None,
            journal: None,
            typed_producer: None,
            browser,
        })
    });
    Ok(())
}
#[wasm_bindgen(js_name=probeApplicationUnprovision)]
pub fn unprovision() -> Result<(), JsValue> {
    DEPLOYMENT.with(|s| {
        let mut s = s.borrow_mut();
        let state = s
            .as_ref()
            .ok_or_else(|| JsValue::from_str("missing fixture"))?;
        wasm::uninstall(&state.endpoint).map_err(|e| JsValue::from_str(&e))?;
        s.take();
        Ok(())
    })
}
#[wasm_bindgen(js_name=probeApplicationValidate)]
pub fn validate_owned(owner: &JsValue) -> Result<String, JsValue> {
    let inputs = original_inputs()?;
    wasm::with_session(owner, |s| installed::validate(s, &inputs))
        .map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen(js_name=probeApplicationRetire)]
pub fn retire() {
    DEPLOYMENT.with(|s| {
        if let Some(state) = s.borrow().as_ref() {
            state.code.retire()
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
    let root = original_inputs()?.root();
    let provider = DEPLOYMENT.with(|slot| slot.borrow().as_ref().is_some_and(|d| d.browser == Some(ApplicationBrowserBehavior::FormControl)));
    register_fixture_receiver(owner, root, provider)
}

fn register_fixture_receiver(owner: &JsValue, root: cem_ql::eval::RetainedCemNode, provider: bool) -> Result<JsValue, JsValue> {
    if provider {
        wasm::register_browser_receiver_input(owner, "cem-receiver-frame", root, "source-route")
    } else {
        wasm::register_receiver_input(owner, "cem-receiver-frame", root)
    }.map_err(fixture_error)
}

#[wasm_bindgen(js_name = probeApplicationTypedReceiverInput)]
pub fn typed_receiver_input(owner: &JsValue) -> Result<JsValue, JsValue> {
    let root = original_inputs()?.root();
    let producer = wasm::with_session(owner, |session| {
        session.register_receiver_typed_producer(
            "cem-receiver-frame",
            &root,
            "choice",
            vec![cem_ql::eval::Item::Atomic(cem_ql::eval::AtomValue::String(
                "typed".into(),
            ))],
            OperationControl::default(),
        )
    })
    .map_err(fixture_error)?;
    let input = wasm::register_receiver_input_with_typed_producers(
        owner,
        "cem-receiver-frame",
        root,
        &[&producer],
    )
    .map_err(fixture_error)?;
    DEPLOYMENT.with(|slot| {
        slot.borrow_mut()
            .as_mut()
            .expect("fixture provisioned")
            .typed_producer = Some(producer);
    });
    Ok(input)
}

#[wasm_bindgen(js_name = probeApplicationDropTypedProducer)]
pub fn drop_typed_producer() {
    DEPLOYMENT.with(|slot| {
        if let Some(deployment) = slot.borrow_mut().as_mut() {
            deployment.typed_producer.take();
        }
    });
}

#[wasm_bindgen(js_name = probeApplicationEditReceiver)]
pub fn edit_receiver(
    owner: &JsValue,
    input: &JsValue,
    bytes: &[u8],
    pins: &str,
    edited: &[u8],
) -> Result<JsValue, JsValue> {
    let fresh =
        installed::Inputs::restore(edited, &OperationControl::default()).map_err(fixture_error)?;
    DEPLOYMENT.with(|slot| {
        let mut slot = slot.borrow_mut();
        let state = slot
            .as_mut()
            .ok_or_else(|| fixture_error("missing fixture"))?;
        let previous = std::mem::replace(
            &mut *state.inputs.lock().map_err(fixture_error)?,
            fresh.clone(),
        );
        if let Err(error) = wasm::with_session(owner, |session| session.replace(bytes, pins)) {
            *state.inputs.lock().map_err(fixture_error)? = previous;
            return Err(fixture_error(error));
        }
        wasm::release_receiver(owner, input)?;
        register_fixture_receiver(owner, fresh.root(), state.browser == Some(ApplicationBrowserBehavior::FormControl))
    })
}

#[wasm_bindgen(js_name = probeApplicationCaptureReceiver)]
pub fn capture_receiver(owner: &JsValue, input: &JsValue) -> Result<js_sys::Array, JsValue> {
    let inputs = original_inputs()?;
    let (bytes, seal) = wasm::capture_receiver_checkpoint(
        owner,
        input,
        &[&inputs.view, &inputs.cart],
        MAX_WORKSPACE_CHECKPOINT_BYTES,
        &OperationControl::default(),
        |_| true,
    )
    .map_err(fixture_error)?;
    let revision = seal.revision();
    DEPLOYMENT.with(|slot| {
        slot.borrow_mut()
            .as_mut()
            .expect("fixture provisioned")
            .captured = Some((owner.clone(), seal));
    });
    Ok(js_sys::Array::of2(
        &js_sys::Uint8Array::from(bytes.as_slice()),
        &JsValue::from_str(&revision),
    ))
}

#[wasm_bindgen(js_name = probeApplicationConfirmReceiver)]
pub fn confirm_receiver(
    owner: &JsValue,
    bytes: &[u8],
    journal: &[u8],
    runtime_id: &str,
) -> Result<String, JsValue> {
    let seal = DEPLOYMENT
        .with(|slot| {
            let mut slot = slot.borrow_mut();
            let state = slot.as_mut().ok_or("missing fixture")?;
            if state
                .captured
                .as_ref()
                .is_none_or(|(original, _)| original != owner)
            {
                return Err("missing receiver seal");
            }
            Ok(state.captured.take().expect("matching seal").1)
        })
        .map_err(fixture_error)?;
    let pin =
        wasm::retire_receiver_checkpoint(owner, seal, bytes, journal, runtime_id, "source-route")
            .map_err(fixture_error)?;
    serde_json::to_string(&pin).map_err(fixture_error)
}

#[wasm_bindgen(js_name = probeApplicationRestoreReceiver)]
pub fn restore_receiver(
    bytes: &[u8],
    pins: &str,
    capsule: &[u8],
    pin: &str,
    journal: &[u8],
    runtime_id: &str,
) -> Result<js_sys::Array, JsValue> {
    if pin.len() > 16384 {
        return Err(fixture_error("fixture pin limit"));
    }
    let pin: ReceiverRecoveryPin = serde_json::from_str(pin).map_err(fixture_error)?;
    let generation = cem_ml::runtime_admission::linked_generation().map_err(fixture_error)?;
    let (checkpoint, writes) = pin
        .verify(
            capsule,
            journal,
            runtime_id,
            &generation,
            &OperationControl::default(),
        )
        .map_err(fixture_error)?;
    let fresh = installed::Inputs::from_receiver_checkpoint(&checkpoint).map_err(fixture_error)?;
    let inputs = DEPLOYMENT
        .with(|slot| slot.borrow().as_ref().map(|state| state.inputs.clone()))
        .ok_or_else(|| fixture_error("missing fixture"))?;
    let previous = std::mem::replace(&mut *inputs.lock().map_err(fixture_error)?, fresh);
    let result = (|| {
        let owner = wasm::open(bytes, pins)?;
        match wasm::restore_receiver_checkpoint(
            &owner,
            checkpoint,
            MAX_WORKSPACE_CHECKPOINT_BYTES - journal.len(),
        ) {
            Ok(input) => Ok((owner, input)),
            Err(error) => {
                wasm::release(&owner)?;
                Err(fixture_error(error))
            }
        }
    })();
    match result {
        Ok((owner, input)) => {
            DEPLOYMENT.with(|slot| {
                slot.borrow_mut()
                    .as_mut()
                    .expect("fixture provisioned")
                    .journal = Some((owner.clone(), writes))
            });
            Ok(js_sys::Array::of2(&owner, &input))
        }
        Err(error) => {
            *inputs.lock().map_err(fixture_error)? = previous;
            Err(error)
        }
    }
}

#[wasm_bindgen(js_name = probeApplicationClaimWrite)]
pub fn claim_write(owner: &JsValue, sequence: u32, id: &str) -> Result<bool, JsValue> {
    wasm::with_session(owner, |_| Ok(())).map_err(fixture_error)?;
    DEPLOYMENT.with(|slot| {
        let mut slot = slot.borrow_mut();
        let (_, journal) = slot
            .as_mut()
            .and_then(|state| state.journal.as_mut())
            .filter(|(original, _)| original == owner)
            .ok_or_else(|| fixture_error("missing receiver journal"))?;
        Ok(journal.claim_write(sequence.into(), id))
    })
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
