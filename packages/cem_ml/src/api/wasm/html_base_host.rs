//! Synchronous live-DOM predicate/control boundary. No parsed AST crosses WASM.
use super::documents::retained_cem_document;
use crate::{
    html_base::*, html_base_evidence::*, html_base_host::*, parser::tree::CemTreeNodeKind,
};
use js_sys::{Array, Function};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "HtmlBaseExecutionHost")]
pub struct WasmHtmlBaseExecutionHost(pub(super) HtmlBaseExecutionHost);

#[wasm_bindgen(js_class = "HtmlBaseExecutionHost")]
impl WasmHtmlBaseExecutionHost {
    #[wasm_bindgen(constructor)]
    pub fn new(
        profile: &str,
        engine: &str,
        version: &str,
        nodes: u32,
        url_bytes: u32,
    ) -> Result<Self, JsValue> {
        let profile = match profile {
            "standard-agreement" => HtmlBaseCompatibilityProfile::StandardAgreement,
            "chromium-148" => HtmlBaseCompatibilityProfile::Chromium148,
            _ => {
                return Err(JsValue::from_str(
                    "explicit HTML base compatibility profile required",
                ))
            }
        };
        HtmlBaseExecutionHost::new(
            profile,
            HtmlBaseBrowser {
                engine: engine.into(),
                version: version.into(),
            },
            HtmlBaseLimits {
                nodes: nodes as usize,
                url_bytes: url_bytes as usize,
            },
        )
        .map(Self)
        .map_err(error)
    }

    pub fn invalidate(&mut self) -> Result<(), JsValue> {
        self.0.invalidate().map_err(error)
    }
    pub fn revoke(&mut self) {
        self.0.revoke();
    }

    /// Only ordinary UTF-8 HTML documents are in this initial host profile.
    pub fn prepare(
        &mut self,
        document: u32,
        fallback: &str,
        policy: &str,
        matches: &Function,
        decide: &Function,
    ) -> Result<u32, JsValue> {
        let Some(tree) = retained_cem_document(document) else {
            self.0.revoke();
            return Err(error("retained native HTML execution document required"));
        };
        self.0
            .prepare(
                tree,
                HtmlBaseContext {
                    fallback: HtmlFallbackBase::DocumentUrl(fallback.into()),
                    encoding: HtmlBaseEncoding::Utf8,
                    policy_stamp: policy.into(),
                },
                |slot| match_slot(matches, slot),
                |request| {
                    let result = decide
                        .call3(
                            &JsValue::NULL,
                            &request.candidate_url.as_str().into(),
                            &request.fallback_url.as_str().into(),
                            &request.policy_stamp.into(),
                        )
                        .map_err(|_| "HTML base host policy callback failed")?;
                    match result.as_bool() {
                        Some(true) => Ok(HtmlBasePolicyDecision::Allow),
                        Some(false) => Ok(HtmlBasePolicyDecision::Deny),
                        None => {
                            Err("HTML base host policy must return a synchronous boolean".into())
                        }
                    }
                },
            )
            .map(|handle| handle.unwrap_or(0))
            .map_err(error)
    }
    pub fn confirm(&mut self, base_uri: &str) -> Result<String, JsValue> {
        self.0
            .confirm(base_uri.into())
            .map(|e| e.effective_url().to_owned())
            .map_err(error)
    }
    pub fn validate(&mut self, matches: &Function, base_uri: &str) -> Result<String, JsValue> {
        let evidence = self
            .0
            .current()
            .cloned()
            .ok_or_else(|| error("HTML base host has no current evidence"))?;
        self.0
            .validate(&evidence, |slot| match_slot(matches, slot), base_uri)
            .map_err(error)?;
        Ok(evidence.effective_url().into())
    }
}

fn error(message: impl AsRef<str>) -> JsValue {
    JsValue::from_str(message.as_ref())
}

pub fn match_slot(matches: &Function, slot: HtmlBaseDomSlot<'_>) -> Result<u32, String> {
    let node = slot.node;
    let kind = match node.kind {
        CemTreeNodeKind::Document => 9,
        CemTreeNodeKind::Element => 1,
        CemTreeNodeKind::Attribute => 2,
        CemTreeNodeKind::Text => 3,
        CemTreeNodeKind::Comment => 8,
        _ => return Err("unsupported HTML execution DOM slot kind".into()),
    };
    let args = Array::new();
    args.push(&slot.id.into());
    args.push(&node.parent.map(JsValue::from).unwrap_or(JsValue::NULL));
    args.push(&(slot.index as u32).into());
    args.push(&kind.into());
    args.push(
        &node
            .name
            .as_ref()
            .map(|n| n.namespace_uri.as_str())
            .unwrap_or("")
            .into(),
    );
    args.push(
        &node
            .name
            .as_ref()
            .map(|n| n.local_name.as_str())
            .unwrap_or("")
            .into(),
    );
    args.push(&node.value.as_str().into());
    args.push(&(node.children.len() as u32).into());
    args.push(&(node.attributes.len() as u32).into());
    let result = matches
        .apply(&JsValue::NULL, &args)
        .map_err(|_| "HTML base host DOM predicate failed")?;
    let handle = result
        .as_f64()
        .ok_or("HTML base host DOM predicate must return a numeric identity")?;
    if !handle.is_finite() || handle < 1.0 || handle > u32::MAX as f64 || handle.fract() != 0.0 {
        return Err("HTML base host DOM identity out of range".into());
    }
    Ok(handle as u32)
}
