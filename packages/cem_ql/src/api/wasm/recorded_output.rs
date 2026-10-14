//! Source ingress, HTML delivery and scalar live predicates. No AST/evidence DTO.
use crate::template_execution_record::browser::RecordedOutputSession;
use cem_ml::html_image_execution::{ImageObservation, ImageResponse};
use cem_ml::{
    api::wasm::{match_html_base_slot, BrowserStylesheetObserver},
    browser_stylesheet_sources::BrowserStylesheetSource,
    html_base::HtmlBasePolicyDecision,
};
use js_sys::{Array, Function};
use wasm_bindgen::prelude::*;
#[wasm_bindgen(js_name = "RecordedOutputSession")]
pub struct WasmRecordedOutputSession {
    inner: RecordedOutputSession,
    sources: Vec<BrowserStylesheetSource>,
    images: Vec<ImageResponse>,
    image_bytes: usize,
}
#[wasm_bindgen(js_class = "RecordedOutputSession")]
impl WasmRecordedOutputSession {
    #[wasm_bindgen(constructor)]
    pub fn new(request: &str, slot: u32, generation: u32) -> Result<Self, JsValue> {
        RecordedOutputSession::from_json(request, slot, generation)
            .map(|inner| Self {
                inner,
                sources: Vec::new(),
                images: Vec::new(),
                image_bytes: 0,
            })
            .map_err(error)
    }
    pub fn url(&self) -> String {
        self.inner.url().into()
    }
    pub fn html(&self) -> Result<String, JsValue> {
        self.inner.html().map_err(error)
    }
    pub fn close(&mut self) {
        self.inner.close();
    }
    #[wasm_bindgen(js_name = "prepareBase")]
    pub fn prepare_base(&mut self, matches: &Function, decide: &Function) -> Result<u32, JsValue> {
        self.inner
            .prepare_base(
                |s| match_html_base_slot(matches, s),
                |r| match decide
                    .call3(
                        &JsValue::NULL,
                        &r.candidate_url.as_str().into(),
                        &r.fallback_url.as_str().into(),
                        &r.policy_stamp.into(),
                    )
                    .map_err(|_| "base policy callback failed")?
                    .as_bool()
                {
                    Some(true) => Ok(HtmlBasePolicyDecision::Allow),
                    Some(false) => Ok(HtmlBasePolicyDecision::Deny),
                    None => Err("synchronous base policy decision required".into()),
                },
            )
            .map(|n| n.unwrap_or(0))
            .map_err(error)
    }
    pub fn compose(&mut self, base: &str, matches: &Function) -> Result<String, JsValue> {
        self.inner
            .compose(base, |s| match_html_base_slot(matches, s))
            .map_err(error)
    }
    #[wasm_bindgen(js_name = "prepareStylesheets")]
    pub fn prepare_stylesheets(
        &mut self,
        base: &str,
        matches: &Function,
        title: Option<String>,
        command: &Function,
    ) -> Result<(), JsValue> {
        let (sources, actions) = self
            .inner
            .prepare_stylesheets(base, |s| match_html_base_slot(matches, s), title.as_deref())
            .map_err(error)?;
        self.sources = sources;
        for action in actions {
            let result = command.call3(
                &JsValue::NULL,
                &action.owner.into(),
                &action.link.into(),
                &action.selected.into(),
            );
            if !matches!(result, Ok(ref value) if value.is_undefined()) {
                self.inner.close();
                return Err(error("synchronous stylesheet selection command required"));
            }
        }
        Ok(())
    }
    #[wasm_bindgen(js_name = "beginImageDelivery")]
    pub fn begin_image_delivery(&mut self) -> Result<(), JsValue> {
        self.inner.begin_image_delivery().map_err(error)
    }
    #[wasm_bindgen(js_name = "acceptImageDelivery")]
    pub fn accept_image_delivery(
        &mut self,
        url: String,
        content_type: String,
        bytes: Vec<u8>,
    ) -> Result<(), JsValue> {
        self.inner
            .accept_image_delivery(ImageResponse {
                final_url: url.clone(),
                requested_url: url,
                content_type,
                bytes,
                revision: String::new(),
            })
            .map_err(error)
    }
    #[wasm_bindgen(js_name = "recordDeliveredImages")]
    pub fn record_delivered_images(
        &mut self,
        environment: &str,
        observe: &Function,
    ) -> Result<(), JsValue> {
        self.inner
            .record_delivered_images(environment, |id| image_observation(observe, id))
            .map_err(error)
    }
    #[wasm_bindgen(js_name = "selectImages")]
    pub fn select_images(&mut self, observe: &Function, request: &Function) -> Result<(), JsValue> {
        let requests = self
            .inner
            .select_images(|id| image_observation(observe, id))
            .map_err(error)?;
        for (id, url) in requests {
            let result = request.call2(&JsValue::NULL, &id.into(), &url.into());
            if !matches!(result, Ok(ref value) if value.is_undefined()) {
                self.inner.close();
                return Err(error("synchronous image request callback required"));
            }
        }
        Ok(())
    }
    #[wasm_bindgen(js_name = "addImageResponse")]
    pub fn add_image_response(
        &mut self,
        requested: String,
        final_url: String,
        content_type: String,
        revision: String,
        bytes: Vec<u8>,
    ) -> Result<(), JsValue> {
        self.image_bytes = self.image_bytes.saturating_add(bytes.len());
        if self.image_bytes > 4 * 1024 * 1024 || self.images.len() >= 256 {
            self.inner.close();
            return Err(error("image acquisition bounds exceeded"));
        }
        self.images.push(ImageResponse {
            requested_url: requested,
            final_url,
            content_type,
            revision,
            bytes,
        });
        Ok(())
    }
    #[wasm_bindgen(js_name = "recordImages")]
    pub fn record_images(&mut self, environment: &str, observe: &Function) -> Result<(), JsValue> {
        self.inner
            .record_images(std::mem::take(&mut self.images), environment, |id| {
                image_observation(observe, id)
            })
            .map_err(error)
    }
    pub fn capture(
        &mut self,
        base: &str,
        matches: &Function,
        revision: &Function,
        observe: &Function,
        images: &Function,
    ) -> Result<String, JsValue> {
        self.verify_images(revision, images)?;
        let result = self
            .inner
            .capture(
                base,
                |s| match_html_base_slot(matches, s),
                &mut BrowserStylesheetObserver {
                    sources: &self.sources,
                    revision,
                    observe,
                },
            )
            .map_err(error)?;
        self.verify_images(revision, images)?;
        Ok(summary(
            result,
            self.inner.image_acquisitions(),
            self.inner.image_deliveries(),
        ))
    }
    pub fn read(
        &mut self,
        base: &str,
        matches: &Function,
        revision: &Function,
        observe: &Function,
        images: &Function,
    ) -> Result<String, JsValue> {
        self.verify_images(revision, images)?;
        let result = self
            .inner
            .read(
                base,
                |s| match_html_base_slot(matches, s),
                &mut BrowserStylesheetObserver {
                    sources: &self.sources,
                    revision,
                    observe,
                },
            )
            .map_err(error)?;
        self.verify_images(revision, images)?;
        Ok(summary(
            result,
            self.inner.image_acquisitions(),
            self.inner.image_deliveries(),
        ))
    }
}
fn summary(mut blockers: Vec<String>, images: usize, deliveries: usize) -> String {
    if images > 0 {
        blockers.push("image-browser-response-identity-required".into());
    }
    serde_json::json!({"blockers":blockers, "imageAcquisitions": images, "imageDeliveries": deliveries}).to_string()
}
fn error(e: impl AsRef<str>) -> JsValue {
    JsValue::from_str(e.as_ref())
}

impl WasmRecordedOutputSession {
    fn verify_images(&mut self, revision: &Function, images: &Function) -> Result<(), JsValue> {
        let result = (|| {
            let env = revision
                .call0(&JsValue::NULL)
                .map_err(|_| "image environment callback failed")?
                .as_string()
                .ok_or("image environment string required")?;
            self.inner
                .validate_images(&env, |id| image_observation(images, id))
        })();
        if result.is_err() {
            self.inner.close();
        }
        result.map_err(error)
    }
}
fn image_observation(observe: &Function, id: u32) -> Result<ImageObservation, String> {
    let result = observe
        .call1(&JsValue::NULL, &id.into())
        .map_err(|_| "image observation callback failed")?;
    if !Array::is_array(&result) {
        return Err("synchronous image observation tuple required".into());
    }
    let values = Array::from(&result);
    if values.length() != 4 {
        return Err("image observation tuple length".into());
    }
    let number = |i| -> Result<u32, String> {
        let n = values.get(i).as_f64().ok_or("image dimension required")?;
        if !n.is_finite() || n < 1.0 || n > u32::MAX as f64 || n.fract() != 0.0 {
            return Err("bounded image dimensions required".into());
        }
        Ok(n as u32)
    };
    Ok(ImageObservation {
        url: values.get(0).as_string().ok_or("image URL required")?,
        width: number(1)?,
        height: number(2)?,
        complete: values.get(3).as_bool().ok_or("image completion required")?,
    })
}
