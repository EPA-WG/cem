//! Synchronous scalar CSSOM predicates over retained HTML/snapshot owners.
use super::html_base_host::{match_slot, WasmHtmlBaseExecutionHost};
use crate::{
    browser_stylesheet_sources::{browser_stylesheet_sources, BrowserStylesheetSource},
    css_import_snapshot::*,
    html_css_import_snapshot::HtmlExecutionCssImports,
    html_stylesheet_execution::HtmlStylesheetExecution,
    html_stylesheet_selection::HtmlStylesheetSelection,
    module_resolution::*,
    scheduler::AbortSignal,
    stylesheet_execution::*,
};
use js_sys::{Array, Function};
use serde::Deserialize;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
const SHEETS: usize = 256;
const BYTES: usize = 4 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Response {
    requested_url: String,
    final_url: String,
    revision: String,
    content_type: String,
    text: String,
}
#[wasm_bindgen(js_name = "HtmlStylesheetSession")]
pub struct WasmHtmlStylesheetSession {
    css: Arc<HtmlExecutionCssImports>,
    snapshot: Arc<CssSourceSnapshot>,
    cap: CemModuleUrlResolutionCapability,
    sources: Vec<BrowserStylesheetSource>,
    host: StylesheetExecutionHost,
    evidence: Option<HtmlStylesheetExecution>,
    selection: Option<HtmlStylesheetSelection>,
}
#[wasm_bindgen(js_class = "HtmlStylesheetSession")]
impl WasmHtmlStylesheetSession {
    #[wasm_bindgen(constructor)]
    pub fn new(
        html: &mut WasmHtmlBaseExecutionHost,
        responses: &str,
        matches: &Function,
        base: &str,
    ) -> Result<Self, JsValue> {
        if responses.len() > 8 * BYTES {
            return Err(error("bounded CSS response control required"));
        }
        let responses: Vec<Response> =
            serde_json::from_str(responses).map_err(|e| error(e.to_string()))?;
        let snapshot = CssSourceSnapshot::new(
            responses
                .into_iter()
                .map(|r| CssSnapshotSource {
                    requested_url: r.requested_url,
                    final_url: r.final_url,
                    revision: r.revision,
                    content_type: r.content_type,
                    bytes: r.text.into_bytes(),
                })
                .collect(),
            CssSnapshotLimits {
                max_sources: SHEETS,
                max_bytes: BYTES,
            },
        )
        .map_err(error)?;
        let handle = CemResolutionContextHandle::new("browser-stylesheet");
        let mut frame = CemModuleUrlFrame::new("browser-stylesheet", base);
        frame.allowed_schemes = Some(["http".into(), "https".into()].into());
        let cap = CemModuleUrlResolutionCapability::new(
            Arc::new(CemScopedModuleUrlResolver::new().with_context(
                handle.clone(),
                CemModuleUrlContext {
                    identity: "browser-stylesheet".into(),
                    resolver_identity: "browser-unmapped".into(),
                    resource_policy_stamp: "http-stylesheet-observation:1".into(),
                    frames: vec![frame],
                },
            )),
            handle,
        );
        let css = prepare_css(html, &cap, snapshot.clone(), matches, base)?;
        let sources = browser_stylesheet_sources(&css, SHEETS, BYTES).map_err(error)?;
        Ok(Self {
            css,
            snapshot,
            cap,
            sources,
            host: Default::default(),
            evidence: None,
            selection: None,
        })
    }
    /// Scalar native commands only. Activation proof stays private to the DOM host.
    #[wasm_bindgen(js_name = "beginSelection")]
    pub fn begin_selection(
        &mut self,
        html: &mut WasmHtmlBaseExecutionHost,
        matches: &Function,
        base: &str,
        title: &str,
        command: &Function,
    ) -> Result<(), JsValue> {
        self.invalidate()?;
        self.selection = None;
        html.0
            .validate_css_imports(
                &self.css,
                &self.cap,
                &self.snapshot,
                |s| match_slot(matches, s),
                base,
            )
            .map_err(error)?;
        let plan = HtmlStylesheetSelection::new(self.css.clone(), title, SHEETS).map_err(error)?;
        for action in plan.actions() {
            let result = command
                .call3(
                    &JsValue::NULL,
                    &action.owner.into(),
                    &action.link.into(),
                    &action.selected.into(),
                )
                .map_err(|_| error("stylesheet selection command failed"))?;
            if !result.is_undefined() {
                return Err(error("synchronous void selection command required"));
            }
        }
        self.selection = Some(plan);
        Ok(())
    }
    #[wasm_bindgen(js_name = "recaptureSelection")]
    pub fn recapture_selection(
        &mut self,
        html: &mut WasmHtmlBaseExecutionHost,
        matches: &Function,
        base: &str,
    ) -> Result<(), JsValue> {
        self.invalidate()?;
        let plan = self
            .selection
            .take()
            .ok_or_else(|| error("pending native selection required"))?;
        let css = prepare_css(html, &self.cap, self.snapshot.clone(), matches, base)?;
        plan.validate_recapture(&css, &self.cap, &self.snapshot)
            .map_err(error)?;
        self.sources = browser_stylesheet_sources(&css, SHEETS, BYTES).map_err(error)?;
        self.css = css;
        Ok(())
    }
    pub fn invalidate(&mut self) -> Result<(), JsValue> {
        self.evidence = None;
        self.host.invalidate().map_err(error)
    }
    pub fn capture(
        &mut self,
        html: &mut WasmHtmlBaseExecutionHost,
        matches: &Function,
        base: &str,
        revision: &Function,
        observe: &Function,
    ) -> Result<String, JsValue> {
        self.invalidate()?;
        let mut observer = Observer {
            sources: &self.sources,
            revision,
            observe,
        };
        self.evidence = Some(
            self.css
                .record_execution(
                    &mut html.0,
                    &self.cap,
                    &self.snapshot,
                    &mut self.host,
                    &mut observer,
                    SHEETS,
                    |s| match_slot(matches, s),
                    base,
                )
                .map_err(error)?,
        );
        self.summary()
    }
    pub fn read(
        &mut self,
        html: &mut WasmHtmlBaseExecutionHost,
        matches: &Function,
        base: &str,
        revision: &Function,
        observe: &Function,
    ) -> Result<String, JsValue> {
        let mut observer = Observer {
            sources: &self.sources,
            revision,
            observe,
        };
        let evidence = self
            .evidence
            .as_ref()
            .ok_or_else(|| error("fresh stylesheet capture required"))?;
        if let Err(e) = evidence.validate(
            &mut html.0,
            &self.cap,
            &self.snapshot,
            &mut self.host,
            &mut observer,
            |s| match_slot(matches, s),
            base,
        ) {
            self.invalidate()?;
            return Err(error(e));
        }
        self.summary()
    }
}
impl WasmHtmlStylesheetSession {
    fn summary(&self) -> Result<String, JsValue> {
        let e = self
            .evidence
            .as_ref()
            .ok_or_else(|| error("stylesheet evidence absent"))?;
        // Explicit diagnostic/control projection, never an execution grant.
        Ok(serde_json::json!({"occurrences": e.execution().observations().len(), "blockers": e.blockers()}).to_string())
    }
}
pub struct Observer<'a> {
    pub sources: &'a [BrowserStylesheetSource],
    pub revision: &'a Function,
    pub observe: &'a Function,
}
impl StylesheetExecutionObserver for Observer<'_> {
    fn environment_revision(&mut self) -> Result<String, String> {
        self.revision
            .call0(&JsValue::NULL)
            .map_err(|_| "stylesheet environment callback failed")?
            .as_string()
            .ok_or_else(|| "synchronous stylesheet revision required".into())
    }
    fn observe(
        &mut self,
        slot: StylesheetExecutionSlot<'_>,
    ) -> Result<StylesheetObservation, String> {
        let source = self
            .sources
            .iter()
            .find(|s| s.root == slot.root && s.sheet == slot.sheet)
            .ok_or("stylesheet source predicate missing")?;
        let args = Array::new();
        for value in [
            JsValue::from(source.root as u32),
            JsValue::from(source.sheet as u32),
            source.owner.into(),
            source.attribute.into(),
            source
                .parent_sheet
                .map(|n| JsValue::from(n as u32))
                .unwrap_or(JsValue::NULL),
            source
                .import_index
                .map(|n| JsValue::from(n as u32))
                .unwrap_or(JsValue::NULL),
            source.url.as_str().into(),
            source.text.as_str().into(),
            slot.media.into(),
            slot.title.into(),
            slot.disabled.into(),
        ] {
            args.push(&value);
        }
        let value = self
            .observe
            .apply(&JsValue::NULL, &args)
            .map_err(|_| "stylesheet CSSOM predicate failed")?;
        if !Array::is_array(&value) {
            return Err("synchronous stylesheet observation tuple required".into());
        }
        let a = Array::from(&value);
        if a.length() != 4 {
            return Err("stylesheet observation tuple length".into());
        }
        let selected = a.get(0).as_bool().ok_or("stylesheet selection required")?;
        let conditions_match = a.get(1).as_bool().ok_or("stylesheet conditions required")?;
        let verified = a
            .get(2)
            .as_bool()
            .ok_or("stylesheet source comparison required")?;
        let id = a.get(3);
        let installation = if id.is_null() {
            None
        } else {
            let n = id
                .as_f64()
                .ok_or("stylesheet installation identity required")?;
            if !n.is_finite() || n < 1.0 || n > u32::MAX as f64 || n.fract() != 0.0 {
                return Err("stylesheet installation identity out of bounds".into());
            }
            Some(n as u64)
        };
        Ok(StylesheetObservation {
            selected,
            conditions_match,
            loaded_source: verified.then(|| slot.source_key.into()),
            installation,
        })
    }
}
fn error(e: impl AsRef<str>) -> JsValue {
    JsValue::from_str(e.as_ref())
}

fn prepare_css(
    html: &mut WasmHtmlBaseExecutionHost,
    cap: &CemModuleUrlResolutionCapability,
    snapshot: Arc<CssSourceSnapshot>,
    matches: &Function,
    base: &str,
) -> Result<Arc<HtmlExecutionCssImports>, JsValue> {
    let base_evidence = html
        .0
        .current()
        .cloned()
        .ok_or_else(|| error("confirmed HTML execution required"))?;
    let document = Arc::new(
        html.0
            .inspect_dependencies(&base_evidence, cap, 65536, |s| match_slot(matches, s), base)
            .map_err(error)?,
    );
    let css = Arc::new(
        html.0
            .inspect_css_imports(
                document,
                cap,
                snapshot.clone(),
                CssSnapshotCaptureLimits {
                    imports: crate::css_imports::CssImportLimits {
                        max_sheets: SHEETS,
                        max_depth: 16,
                    },
                    responses: crate::css_imports::CssImportResponsePolicy {
                        max_response_bytes: BYTES,
                        max_total_bytes: BYTES,
                    },
                    max_root_nodes: 65536,
                },
                AbortSignal::new(),
                |s| match_slot(matches, s),
                base,
            )
            .map_err(error)?,
    );
    Ok(css)
}
