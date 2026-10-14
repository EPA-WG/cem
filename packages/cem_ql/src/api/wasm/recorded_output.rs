//! Source ingress, HTML delivery and scalar live predicates. No AST/evidence DTO.
use crate::template_execution_record::browser::RecordedOutputSession;
use cem_ml::{
    api::wasm::match_html_base_slot, html_base::HtmlBasePolicyDecision,
    parser::tree::RetainedCemTree, stylesheet_execution::*,
};
use js_sys::{Array, Function};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
#[wasm_bindgen(js_name = "RecordedOutputSession")]
pub struct WasmRecordedOutputSession {
    inner: RecordedOutputSession,
}
#[wasm_bindgen(js_class = "RecordedOutputSession")]
impl WasmRecordedOutputSession {
    #[wasm_bindgen(constructor)]
    pub fn new(request: &str, slot: u32, generation: u32) -> Result<Self, JsValue> {
        RecordedOutputSession::from_json(request, slot, generation)
            .map(|inner| Self { inner })
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
    pub fn capture(
        &mut self,
        base: &str,
        matches: &Function,
        revision: &Function,
        observe: &Function,
    ) -> Result<String, JsValue> {
        let tree = self.inner.output_tree().map_err(error)?;
        self.inner
            .capture(
                base,
                |s| match_html_base_slot(matches, s),
                &mut Observer {
                    tree,
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
        let tree = self.inner.output_tree().map_err(error)?;
        self.inner
            .read(
                base,
                |s| match_html_base_slot(matches, s),
                &mut Observer {
                    tree,
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
struct Observer<'a> {
    tree: Arc<RetainedCemTree>,
    revision: &'a Function,
    observe: &'a Function,
}
impl StylesheetExecutionObserver for Observer<'_> {
    fn environment_revision(&mut self) -> Result<String, String> {
        self.revision
            .call0(&JsValue::NULL)
            .map_err(|_| "recorded revision callback failed")?
            .as_string()
            .ok_or("synchronous recorded revision required".into())
    }
    fn observe(
        &mut self,
        slot: StylesheetExecutionSlot<'_>,
    ) -> Result<StylesheetObservation, String> {
        let StylesheetOwner::Document { owner, source_node } = slot.owner else {
            return Err("recorded browser supports document style roots only".into());
        };
        let node = self
            .tree
            .node(*owner)
            .ok_or("recorded style owner missing")?;
        if owner != source_node
            || slot.parent_sheet.is_some()
            || !slot.media.is_empty()
            || !slot.title.is_empty()
            || slot.alternate
            || !node.name.as_ref().is_some_and(|n| {
                n.local_name == "style" && n.namespace_uri == "http://www.w3.org/1999/xhtml"
            })
        {
            return Err(
                "recorded browser requires untitled flattened inline stylesheet roots".into(),
            );
        }
        let text = self
            .tree
            .text_fragments(*owner)
            .ok_or("native style text missing")?
            .collect::<String>();
        let result = self
            .observe
            .call2(&JsValue::NULL, &(*owner).into(), &text.into())
            .map_err(|_| "recorded CSS predicate failed")?;
        if !Array::is_array(&result) {
            return Err("synchronous recorded stylesheet tuple required".into());
        }
        let a = Array::from(&result);
        if a.length() != 3 {
            return Err("recorded stylesheet tuple length".into());
        }
        let id = a
            .get(0)
            .as_f64()
            .ok_or("recorded stylesheet identity required")?;
        if !id.is_finite() || id < 1.0 || id > u32::MAX as f64 || id.fract() != 0.0 {
            return Err("recorded stylesheet identity out of bounds".into());
        }
        Ok(StylesheetObservation {
            installation: Some(id as u64),
            selected: a.get(1).as_bool().ok_or("selection required")?,
            conditions_match: true,
            loaded_source: a
                .get(2)
                .as_bool()
                .ok_or("source predicate required")?
                .then(|| slot.source_key.into()),
        })
    }
}
