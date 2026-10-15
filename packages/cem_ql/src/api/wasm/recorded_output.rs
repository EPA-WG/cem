//! Source ingress, HTML delivery and scalar live predicates. No AST/evidence DTO.
use crate::template_execution_record::browser::RecordedOutputSession;
use cem_ml::html_image_execution::{ImageObservation, ImageResponse};
use cem_ml::svg_computed_execution::{SvgComputedObserver, SvgComputedSlot};
use cem_ml::svg_paint_clone_profile::{CloneProbe, CloneProbeObservation};
use cem_ml::svg_resource_acquisition::{
    SvgDeliveryCapture, SvgDeliveryResponse, SvgResourceResponse,
};
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
    svg_snapshots: Option<Vec<SvgResourceResponse>>,
    svg_bytes: usize,
    svg_fragments: bool,
    svg_templates: bool,
    svg_content: bool,
    svg_nested: Option<Vec<SvgResourceResponse>>,
    svg_nested_bytes: usize,
    svg_delivery: Option<SvgDeliveryCapture>,
    capture_started: bool,
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
                svg_snapshots: None,
                svg_bytes: 0,
                svg_fragments: false,
                svg_templates: false,
                svg_content: false,
                svg_nested: None,
                svg_nested_bytes: 0,
                svg_delivery: None,
                capture_started: false,
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
        self.svg_snapshots = None;
        self.svg_nested = None;
        self.svg_delivery = None;
        self.capture_started = true;
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
    #[wasm_bindgen(js_name = "enableImageReplay")]
    pub fn enable_image_replay(&mut self, nonce: &str) -> Result<(), JsValue> {
        self.inner.enable_image_replay(nonce).map_err(error)
    }
    #[wasm_bindgen(js_name = "imageReplayRoutes")]
    pub fn image_replay_routes(&mut self, route: &Function) -> Result<(), JsValue> {
        for (alias, source) in self.inner.image_replay_routes().map_err(error)? {
            if !matches!(route.call2(&JsValue::NULL, &alias.into(), &source.into()), Ok(v) if v.is_undefined())
            {
                self.inner.close();
                return Err(error("synchronous image replay route ingress required"));
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
    #[wasm_bindgen(js_name = "beginSvgDelivery")]
    pub fn begin_svg_delivery(&mut self) -> Result<(), JsValue> {
        if self.capture_started
            || self.svg_delivery.is_some()
            || self.svg_snapshots.is_none()
            || self.inner.html().is_err()
        {
            self.close();
            return Err(error("fresh SVG snapshots and delivery required"));
        }
        self.svg_delivery = Some(SvgDeliveryCapture::new(self.inner.cancellation()));
        Ok(())
    }
    #[wasm_bindgen(js_name = "acceptSvgDelivery")]
    pub fn accept_svg_delivery(
        &mut self,
        url: String,
        content_type: String,
        bytes: Vec<u8>,
    ) -> Result<(), JsValue> {
        let result = if self.capture_started {
            Err("SVG delivery already sealed".into())
        } else {
            self.svg_delivery
                .as_mut()
                .ok_or_else(|| "SVG delivery not open".to_owned())
                .and_then(|ledger| {
                    ledger.accept(SvgDeliveryResponse {
                        requested_url: url.clone(),
                        final_url: url,
                        content_type,
                        bytes,
                    })
                })
        };
        if result.is_err() {
            self.close();
        }
        result.map_err(error)
    }
    /// Raw source ingress only; not a browser delivery receipt.
    #[wasm_bindgen(js_name = "beginSvgSnapshots")]
    pub fn begin_svg_snapshots(&mut self) -> Result<(), JsValue> {
        if self.capture_started || self.svg_snapshots.is_some() || self.inner.html().is_err() {
            self.close();
            return Err(error("fresh SVG snapshot ingress required"));
        }
        self.svg_snapshots = Some(Vec::new());
        Ok(())
    }
    /// Opt in after opening snapshot ingress, before capture.
    #[wasm_bindgen(js_name = "enableSvgFragmentValidation")]
    pub fn enable_svg_fragment_validation(&mut self) -> Result<(), JsValue> {
        if self.capture_started
            || self.svg_snapshots.is_none()
            || self.svg_fragments
            || self.inner.html().is_err()
        {
            self.close();
            return Err(error("fresh SVG fragment validation opt-in required"));
        }
        self.svg_fragments = true;
        Ok(())
    }
    /// Includes fragment validation; must be enabled before capture.
    #[wasm_bindgen(js_name = "enableSvgPaintTemplateValidation")]
    pub fn enable_svg_paint_template_validation(&mut self) -> Result<(), JsValue> {
        if self.capture_started
            || self.svg_snapshots.is_none()
            || self.svg_templates
            || self.inner.html().is_err()
        {
            self.close();
            return Err(error("fresh SVG template validation opt-in required"));
        }
        self.svg_templates = true;
        Ok(())
    }
    #[wasm_bindgen(js_name = "enableSvgPaintContent")]
    pub fn enable_svg_paint_content(&mut self) -> Result<(), JsValue> {
        if self.capture_started
            || self.svg_snapshots.is_none()
            || self.svg_content
            || self.inner.html().is_err()
        {
            self.close();
            return Err(error("fresh acquired paint content opt-in required"));
        }
        self.svg_content = true;
        Ok(())
    }
    #[wasm_bindgen(js_name = "beginSvgNestedSnapshots")]
    pub fn begin_svg_nested_snapshots(&mut self) -> Result<(), JsValue> {
        if self.capture_started
            || !self.svg_content
            || self.svg_nested.is_some()
            || self.inner.html().is_err()
        {
            self.close();
            return Err(error(
                "fresh nested snapshot ingress requires content opt-in",
            ));
        }
        self.svg_nested = Some(Vec::new());
        Ok(())
    }
    #[wasm_bindgen(js_name = "addSvgNestedSnapshot")]
    pub fn add_svg_nested_snapshot(
        &mut self,
        requested: String,
        final_url: String,
        content_type: String,
        revision: String,
        bytes: Vec<u8>,
    ) -> Result<(), JsValue> {
        self.svg_nested_bytes = self
            .svg_nested_bytes
            .saturating_add(bytes.len())
            .saturating_add(requested.len())
            .saturating_add(final_url.len())
            .saturating_add(content_type.len())
            .saturating_add(revision.len());
        if self.capture_started
            || self.inner.html().is_err()
            || self.svg_nested.as_ref().is_none_or(|v| v.len() >= 256)
            || self.svg_nested_bytes > 4 * 1024 * 1024
        {
            self.close();
            return Err(error("nested snapshot ingress stage or bounds exceeded"));
        }
        self.svg_nested.as_mut().unwrap().push(SvgResourceResponse {
            requested_url: requested,
            final_url,
            content_type,
            revision,
            bytes,
        });
        Ok(())
    }
    #[wasm_bindgen(js_name = "addSvgSnapshot")]
    pub fn add_svg_snapshot(
        &mut self,
        requested: String,
        final_url: String,
        content_type: String,
        revision: String,
        bytes: Vec<u8>,
    ) -> Result<(), JsValue> {
        self.svg_bytes = self
            .svg_bytes
            .saturating_add(bytes.len())
            .saturating_add(requested.len())
            .saturating_add(final_url.len())
            .saturating_add(content_type.len())
            .saturating_add(revision.len());
        if self.capture_started
            || self.inner.html().is_err()
            || self.svg_snapshots.as_ref().is_none_or(|v| v.len() >= 256)
            || self.svg_bytes > 4 * 1024 * 1024
        {
            self.close();
            return Err(error("SVG snapshot ingress stage or bounds exceeded"));
        }
        self.svg_snapshots
            .as_mut()
            .unwrap()
            .push(SvgResourceResponse {
                requested_url: requested,
                final_url,
                content_type,
                revision,
                bytes,
            });
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
    #[wasm_bindgen(js_name = "confirmCloneProfile")]
    pub fn confirm_clone_profile(&mut self, name: &str, observe: &Function) -> Result<(), JsValue> {
        self.inner
            .confirm_clone_profile(name, |case| {
                let index = CloneProbe::ALL
                    .iter()
                    .position(|c| *c == case)
                    .ok_or("unknown clone probe")?;
                let raw = observe
                    .call1(&JsValue::NULL, &JsValue::from_f64(index as f64))
                    .map_err(|_| "clone probe callback failed")?;
                if !Array::is_array(&raw) {
                    return Err("clone probe requires scalar array".into());
                }
                let array = Array::from(&raw);
                if array.length() != 12 {
                    return Err("clone probe requires twelve RGBA channels".into());
                }
                let mut bytes = [0u8; 12];
                for (i, byte) in bytes.iter_mut().enumerate() {
                    let value = array
                        .get(i as u32)
                        .as_f64()
                        .ok_or("clone probe channel must be numeric")?;
                    if !value.is_finite() || value.fract() != 0.0 || !(0.0..=255.0).contains(&value)
                    {
                        return Err("clone probe channel out of range".into());
                    }
                    *byte = value as u8;
                }
                Ok(CloneProbeObservation {
                    case,
                    source_color: bytes[0..4].try_into().unwrap(),
                    host_color: bytes[4..8].try_into().unwrap(),
                    pixel: bytes[8..12].try_into().unwrap(),
                })
            })
            .map_err(error)
    }
    #[wasm_bindgen(js_name = "enableSvgReplayPlan")]
    pub fn enable_svg_replay_plan(&mut self, nonce: &str) -> Result<(), JsValue> {
        self.inner.enable_svg_replay_plan(nonce).map_err(error)
    }
    #[wasm_bindgen(js_name = "enableBoundedPaintColors")]
    pub fn enable_bounded_paint_colors(&mut self) -> Result<(), JsValue> {
        self.inner.enable_paint_colors().map_err(error)
    }
    pub fn capture(
        &mut self,
        base: &str,
        matches: &Function,
        revision: &Function,
        observe: &Function,
        images: &Function,
        svg: &Function,
    ) -> Result<String, JsValue> {
        self.capture_started = true;
        self.inner
            .record_svg(&mut SvgObserver {
                revision,
                observe: svg,
            })
            .map_err(error)?;
        if let Some(responses) = self.svg_snapshots.take() {
            self.inner
                .record_svg_resources(
                    responses,
                    self.inner.cancellation(),
                    &mut SvgObserver {
                        revision,
                        observe: svg,
                    },
                )
                .map_err(error)?;
        }
        if self.svg_content {
            self.inner
                .prepare_svg_paint_content(&mut SvgObserver {
                    revision,
                    observe: svg,
                })
                .map_err(error)?;
        } else if self.svg_templates {
            self.inner
                .validate_svg_paint_templates(&mut SvgObserver {
                    revision,
                    observe: svg,
                })
                .map_err(error)?;
        } else if self.svg_fragments {
            self.inner
                .validate_svg_fragments(&mut SvgObserver {
                    revision,
                    observe: svg,
                })
                .map_err(error)?;
        }
        if let Some(responses) = self.svg_nested.take() {
            self.inner
                .bind_svg_nested_snapshots(
                    responses,
                    self.inner.cancellation(),
                    &mut SvgObserver {
                        revision,
                        observe: svg,
                    },
                )
                .map_err(error)?;
        }
        if let Some(ledger) = self.svg_delivery.take() {
            self.inner
                .bind_svg_delivery(
                    ledger,
                    &mut SvgObserver {
                        revision,
                        observe: svg,
                    },
                )
                .map_err(error)?;
        }
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
        self.inner
            .validate_svg(&mut SvgObserver {
                revision,
                observe: svg,
            })
            .map_err(error)?;
        self.verify_paint_colors(svg)?;
        Ok(summary(
            result,
            self.inner.image_acquisitions(),
            self.inner.image_deliveries(),
            self.inner.image_replay_bound(),
            self.inner.svg_computed_slots(),
            self.inner.svg_replay_candidates(),
            self.inner.svg_resource_bindings(),
            self.inner.svg_fragment_targets(),
            self.inner.svg_paint_template_counts(),
            self.inner.svg_acquired_content_counts(),
            self.inner.svg_acquired_style_counts(),
            self.inner.svg_delivered_bindings(),
            self.inner.svg_paint_counts(),
            self.inner.svg_paint_content_counts(),
            self.inner.svg_paint_style_counts(),
            self.inner.clone_profile_confirmed(),
            self.inner.paint_color_counts(),
        ))
    }
    pub fn read(
        &mut self,
        base: &str,
        matches: &Function,
        revision: &Function,
        observe: &Function,
        images: &Function,
        svg: &Function,
    ) -> Result<String, JsValue> {
        self.verify_paint_colors(svg)?;
        self.inner
            .validate_svg(&mut SvgObserver {
                revision,
                observe: svg,
            })
            .map_err(error)?;
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
        self.inner
            .validate_svg(&mut SvgObserver {
                revision,
                observe: svg,
            })
            .map_err(error)?;
        self.verify_paint_colors(svg)?;
        Ok(summary(
            result,
            self.inner.image_acquisitions(),
            self.inner.image_deliveries(),
            self.inner.image_replay_bound(),
            self.inner.svg_computed_slots(),
            self.inner.svg_replay_candidates(),
            self.inner.svg_resource_bindings(),
            self.inner.svg_fragment_targets(),
            self.inner.svg_paint_template_counts(),
            self.inner.svg_acquired_content_counts(),
            self.inner.svg_acquired_style_counts(),
            self.inner.svg_delivered_bindings(),
            self.inner.svg_paint_counts(),
            self.inner.svg_paint_content_counts(),
            self.inner.svg_paint_style_counts(),
            self.inner.clone_profile_confirmed(),
            self.inner.paint_color_counts(),
        ))
    }
}
fn summary(
    mut blockers: Vec<String>,
    images: usize,
    deliveries: usize,
    replay_bound: bool,
    svg: usize,
    svg_replay: usize,
    svg_bindings: usize,
    svg_fragments: usize,
    svg_templates: (usize, usize),
    svg_content: (usize, usize, usize, usize),
    svg_styles: (usize, usize, usize),
    svg_delivered: usize,
    paint: (usize, usize),
    content: (usize, usize),
    style: (usize, usize),
    clone_profile: bool,
    colors: (usize, usize),
) -> String {
    if images > 0 && !replay_bound {
        blockers.push("image-browser-response-identity-required".into());
    }
    if svg > 0 {
        blockers.extend(
            [
                "svg-computed-winning-source-required",
                "svg-computed-resource-acquisition-required",
                "general-runtime-resource-coverage-required",
            ]
            .map(str::to_owned),
        );
    }
    serde_json::json!({"blockers":blockers, "imageAcquisitions": images, "imageDeliveries": deliveries, "svgComputedSlots": svg, "svgReplayCandidates": svg_replay, "svgResourceBindings": svg_bindings, "svgFragmentTargets": svg_fragments, "svgPaintTemplateBindings": svg_templates.0, "svgPaintTemplateLinks": svg_templates.1, "svgAcquiredContentBindings": svg_content.0, "svgAcquiredContentNodes": svg_content.1, "svgAcquiredDependencies": svg_content.2, "svgNestedBodies": svg_content.3, "svgAcquiredColors": svg_styles.0, "svgAcquiredInheritedBindings": svg_styles.1, "svgAcquiredCompatibilityDifferences": svg_styles.2, "svgDeliveredBindings": svg_delivered, "svgPaintRoots": paint.0, "svgPaintLinks": paint.1, "svgPaintServers": content.0, "svgPaintContentNodes": content.1, "svgPaintStyleHosts": style.0, "svgPaintStyleInstances": style.1, "svgCloneProfileConfirmed": clone_profile, "svgPaintColors": colors.0, "svgPaintSuppressedHosts": colors.1}).to_string()
}
fn error(e: impl AsRef<str>) -> JsValue {
    JsValue::from_str(e.as_ref())
}

impl WasmRecordedOutputSession {
    fn verify_paint_colors(&mut self, svg: &Function) -> Result<(), JsValue> {
        self.inner
            .validate_paint_colors(|owner, property| {
                svg.call2(&JsValue::NULL, &owner.into(), &property.into())
                    .map_err(|_| "paint color observation failed")?
                    .as_string()
                    .ok_or_else(|| "synchronous paint color string required".into())
            })
            .map_err(error)
    }
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

struct SvgObserver<'a> {
    revision: &'a Function,
    observe: &'a Function,
}
impl SvgComputedObserver for SvgObserver<'_> {
    fn environment_revision(&mut self) -> Result<String, String> {
        self.revision
            .call0(&JsValue::NULL)
            .map_err(|_| "SVG environment callback failed")?
            .as_string()
            .ok_or_else(|| "synchronous SVG environment string required".into())
    }
    fn observe(&mut self, slot: SvgComputedSlot<'_>) -> Result<String, String> {
        self.observe
            .call2(
                &JsValue::NULL,
                &slot.source.owner().into(),
                &slot.property.into(),
            )
            .map_err(|_| "SVG computed callback failed")?
            .as_string()
            .ok_or_else(|| "synchronous SVG computed string required".into())
    }
}
