//! Opaque compiled-source and consumer leases. JS sees comparison predicates,
//! never reconstructed native trees or a transferable installation grant.
use super::ARTIFACTS;
use crate::stylesheet_installation::*;
use js_sys::{Array, Function};
use wasm_bindgen::prelude::*;
#[wasm_bindgen(js_name = "CompiledStylesheetSession")]
pub struct WasmCompiledStylesheetSession {
    set: CompiledStylesheetSet,
    host: CompiledStylesheetHost,
    evidence: Option<CompiledStylesheetEvidence>,
}
#[wasm_bindgen(js_class = "CompiledStylesheetSession")]
impl WasmCompiledStylesheetSession {
    #[wasm_bindgen(constructor)]
    pub fn new(artifact: u32, consumer: &str, indices: &str) -> Result<Self, JsValue> {
        if indices.len() > 4096 || consumer.len() > 4096 {
            return Err(error("bounded compiled stylesheet control required"));
        }
        let indices: Vec<usize> =
            serde_json::from_str(indices).map_err(|e| error(e.to_string()))?;
        let set = ARTIFACTS
            .with(|cell| {
                let artifacts = cell.borrow();
                artifacts
                    .get(artifact.checked_sub(1).ok_or("unknown template artifact")? as usize)
                    .and_then(Option::as_ref)
                    .ok_or("unknown template artifact")?
                    .stylesheet_installations(consumer, &indices)
            })
            .map_err(error)?;
        Ok(Self {
            set,
            host: Default::default(),
            evidence: None,
        })
    }
    pub fn len(&self) -> usize {
        self.set.len()
    }
    pub fn css(&self, index: usize) -> Result<String, JsValue> {
        self.set.css(index).map_err(error)
    }
    pub fn invalidate(&mut self) -> Result<(), JsValue> {
        self.evidence = None;
        self.host.invalidate().map_err(error)
    }
    pub fn capture(&mut self, revision: &Function, observe: &Function) -> Result<String, JsValue> {
        self.invalidate()?;
        self.evidence = Some(
            self.host
                .capture(
                    &self.set,
                    &mut Observer { revision, observe },
                    256,
                    4 * 1024 * 1024,
                )
                .map_err(error)?,
        );
        self.summary()
    }
    pub fn read(&mut self, revision: &Function, observe: &Function) -> Result<String, JsValue> {
        let evidence = self
            .evidence
            .as_ref()
            .ok_or_else(|| error("fresh compiled stylesheet capture required"))?;
        if let Err(e) = self
            .host
            .validate(evidence, &self.set, &mut Observer { revision, observe })
        {
            self.invalidate()?;
            return Err(error(e));
        }
        self.summary()
    }
}
impl WasmCompiledStylesheetSession {
    fn summary(&self) -> Result<String, JsValue> {
        let e = self
            .evidence
            .as_ref()
            .ok_or_else(|| error("compiled stylesheet evidence absent"))?;
        Ok(
            serde_json::json!({"occurrences":e.observations().len(),"blockers":e.blockers()})
                .to_string(),
        )
    }
}
struct Observer<'a> {
    revision: &'a Function,
    observe: &'a Function,
}
impl CompiledStylesheetObserver for Observer<'_> {
    fn environment_revision(&mut self) -> Result<String, String> {
        self.revision
            .call0(&JsValue::NULL)
            .map_err(|_| "compiled stylesheet revision callback failed")?
            .as_string()
            .ok_or_else(|| "synchronous compiled stylesheet revision required".into())
    }
    fn observe(
        &mut self,
        slot: CompiledStylesheetSlot<'_>,
    ) -> Result<CompiledStylesheetObservation, String> {
        let value = self
            .observe
            .call2(
                &JsValue::NULL,
                &JsValue::from(slot.index as u32),
                &JsValue::from(slot.css),
            )
            .map_err(|_| "compiled stylesheet CSSOM predicate failed")?;
        if !Array::is_array(&value) {
            return Err("synchronous compiled observation tuple required".into());
        }
        let a = Array::from(&value);
        if a.length() != 3 {
            return Err("compiled observation tuple length".into());
        }
        let id = a
            .get(0)
            .as_f64()
            .ok_or("compiled installation identity required")?;
        if id < 1.0 || id > u32::MAX as f64 || !id.is_finite() || id.fract() != 0.0 {
            return Err("compiled installation identity out of range".into());
        }
        Ok(CompiledStylesheetObservation {
            installation: id as u32,
            selected: a.get(1).as_bool().ok_or("compiled selection required")?,
            source_matches: a
                .get(2)
                .as_bool()
                .ok_or("compiled source comparison required")?,
        })
    }
}
fn error(message: impl AsRef<str>) -> JsValue {
    JsValue::from_str(message.as_ref())
}
