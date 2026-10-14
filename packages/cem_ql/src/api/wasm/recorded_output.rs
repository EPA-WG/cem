//! Source ingress, HTML delivery and scalar live predicates. No AST/evidence DTO.
use crate::template_execution_record::browser::RecordedOutputSession;
use cem_ml::{
    api::wasm::{match_html_base_slot, BrowserStylesheetObserver},
    browser_stylesheet_sources::BrowserStylesheetSource,
    html_base::HtmlBasePolicyDecision,
};
use js_sys::Function;
use wasm_bindgen::prelude::*;
#[wasm_bindgen(js_name = "RecordedOutputSession")]
pub struct WasmRecordedOutputSession {
    inner: RecordedOutputSession,
    sources: Vec<BrowserStylesheetSource>,
}
#[wasm_bindgen(js_class = "RecordedOutputSession")]
impl WasmRecordedOutputSession {
    #[wasm_bindgen(constructor)]
    pub fn new(request: &str, slot: u32, generation: u32) -> Result<Self, JsValue> {
        RecordedOutputSession::from_json(request, slot, generation)
            .map(|inner| Self {
                inner,
                sources: Vec::new(),
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
    pub fn capture(
        &mut self,
        base: &str,
        matches: &Function,
        revision: &Function,
        observe: &Function,
    ) -> Result<String, JsValue> {
        self.inner
            .capture(
                base,
                |s| match_html_base_slot(matches, s),
                &mut BrowserStylesheetObserver {
                    sources: &self.sources,
                    revision,
                    observe,
                },
            )
            .map(summary)
            .map_err(error)
    }
    pub fn read(
        &mut self,
        base: &str,
        matches: &Function,
        revision: &Function,
        observe: &Function,
    ) -> Result<String, JsValue> {
        self.inner
            .read(
                base,
                |s| match_html_base_slot(matches, s),
                &mut BrowserStylesheetObserver {
                    sources: &self.sources,
                    revision,
                    observe,
                },
            )
            .map(summary)
            .map_err(error)
    }
}
fn summary(blockers: Vec<String>) -> String {
    serde_json::json!({"blockers":blockers}).to_string()
}
fn error(e: impl AsRef<str>) -> JsValue {
    JsValue::from_str(e.as_ref())
}
