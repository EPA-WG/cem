//! Bounded source-ingress and staged delivery contract for synchronous browser
//! observation. All semantic trees and preparation evidence stay native.
use super::*;
use cem_ml::html_image_execution::{
    ImageDeliveryCapture, ImageObservation, ImagePlan, ImageRecord, ImageReplayPlan, ImageResponse,
};
use cem_ml::import::svg_presentation::RetainedSvgPresentationAttribute;
use cem_ml::svg_computed_execution::{SvgComputedEvidence, SvgComputedHost, SvgComputedObserver};
use cem_ml::svg_paint_clone_profile::{CloneProbe, CloneProbeObservation, SvgPaintCloneProfile};
use cem_ml::svg_paint_colors::BoundedPaintColors;
use cem_ml::svg_paint_content::SvgPaintContentPlan;
use cem_ml::svg_paint_dependencies::{PaintTarget, SvgPaintPlan};
use cem_ml::svg_paint_style::SvgPaintStylePlan;
use cem_ml::{
    css_emission::CssManagedScope,
    css_import_snapshot::{CssSnapshotSource, CssSourceSnapshot},
    html_base::*,
    html_base_evidence::*,
    module_resolution::*,
    scheduler::AbortSignal,
    stylesheet_execution::StylesheetExecutionObserver,
    template_snapshot::{preflight_template_snapshot, TemplateSnapshotSource},
    worker_control::{WorkerAddress, WorkerGeneration, WorkerSlotId},
};
use serde::Deserialize;
const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_NODES: usize = 10_000;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Source {
    uri: String,
    revision: String,
    source: String,
    content_type: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CssSource {
    requested_url: String,
    final_url: String,
    revision: String,
    source: String,
    content_type: String,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
enum Scope {
    Private { tag: String },
    Shared { name: String },
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Choice {
    body_uri: String,
    index: usize,
    scope: Scope,
    #[serde(default)]
    host_paths: Vec<Vec<usize>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Browser {
    engine: String,
    version: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    root_uri: String,
    output_uri: String,
    resolution_base: String,
    policy_stamp: String,
    browser: Browser,
    sources: Vec<Source>,
    stylesheets: Vec<CssSource>,
    choices: Vec<Choice>,
    #[serde(default)]
    bindings: BTreeMap<String, String>,
}
#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Original,
    OriginalBase,
    Composed,
    ComposedBase,
    Captured,
    Closed,
}

pub struct RecordedOutputSession {
    stage: Stage,
    image_plan: Option<Arc<ImagePlan>>,
    svg_sources: Option<Vec<RetainedSvgPresentationAttribute>>,
    svg_host: SvgComputedHost,
    svg_evidence: Option<SvgComputedEvidence>,
    paint: Option<Arc<SvgPaintPlan>>,
    paint_content: Option<Arc<SvgPaintContentPlan>>,
    paint_style: Option<Arc<SvgPaintStylePlan>>,
    paint_colors: Option<BoundedPaintColors>,
    paint_colors_enabled: bool,
    clone_profile: Option<SvgPaintCloneProfile>,
    image_delivery: Option<ImageDeliveryCapture>,
    delivered_images: usize,
    replay_nonce: Option<String>,
    replay: Option<Arc<ImageReplayPlan>>,
    image_record: Option<ImageRecord>,
    pending_images: Option<Vec<ImageObservation>>,
    snapshot: Arc<TemplateSourceSnapshot>,
    engine: Arc<EngineContext>,
    cap: CemModuleUrlResolutionCapability,
    execution: Arc<SnapshotTemplateExecution>,
    css: Arc<CssSourceSnapshot>,
    choices: Vec<BodyStylesheetInstallation>,
    host_paths: Vec<Vec<Vec<usize>>>,
    output: Option<Arc<RecordedTemplateOutput>>,
    html: Option<HtmlBaseExecutionHost>,
    worker: RecordedPreparationWorker,
    handle: Option<RecordedPreparationHandle>,
    cancel: AbortSignal,
    output_uri: String,
    policy_stamp: String,
    browser: Browser,
}
impl RecordedOutputSession {
    /// Explicit source-acquisition envelope: textual sources enter the shared
    /// importer once. This accepts neither ASTs nor evidence/handle DTOs.
    pub fn from_json(json: &str, slot: u32, generation: u32) -> Result<Self, String> {
        if json.len() > MAX_BYTES {
            return Err("recorded source request exceeds byte bound".into());
        }
        let r: Request = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if r.sources.len() > 256
            || r.stylesheets.len() > 256
            || r.choices.len() > 256
            || r.bindings.len() > 256
            || r.policy_stamp.is_empty()
            || r.policy_stamp.len() > 4096
            || r.browser.engine.is_empty()
            || r.browser.version.is_empty()
        {
            return Err("bounded complete recorded source request required".into());
        }
        for uri in [&r.output_uri, &r.root_uri, &r.resolution_base] {
            let url = url::Url::parse(uri)
                .map_err(|_| "absolute HTTP(S) recorded source context required")?;
            if !matches!(url.scheme(), "http" | "https")
                || url.fragment().is_some()
                || uri.len() > 4096
            {
                return Err("fragment-free HTTP(S) recorded source context required".into());
            }
        }
        let worker = RecordedPreparationWorker::new(
            WorkerAddress::new(
                WorkerSlotId::from_raw(slot.into()),
                WorkerGeneration::from_raw(generation.into()),
            ),
            1,
        )?;
        let snapshot = TemplateSourceSnapshot::new(
            r.sources
                .into_iter()
                .map(|s| TemplateSnapshotSource {
                    uri: s.uri,
                    revision: s.revision,
                    bytes: s.source.into_bytes(),
                    content_type: s.content_type,
                })
                .collect(),
            Default::default(),
        )?;
        let css = CssSourceSnapshot::new(
            r.stylesheets
                .into_iter()
                .map(|s| CssSnapshotSource {
                    requested_url: s.requested_url,
                    final_url: s.final_url,
                    revision: s.revision,
                    bytes: s.source.into_bytes(),
                    content_type: s.content_type,
                })
                .collect(),
            Default::default(),
        )?;
        let engine = Arc::new(EngineContext::default());
        let preflight = Arc::new(preflight_template_snapshot(
            snapshot.clone(),
            engine.clone(),
            &r.root_uri,
            256,
        )?);
        let context = CemResolutionContextHandle::new("recorded-browser");
        let cap = CemModuleUrlResolutionCapability::new(
            Arc::new(CemScopedModuleUrlResolver::new().with_context(
                context.clone(),
                CemModuleUrlContext {
                    identity: "recorded-browser".into(),
                    resolver_identity: "immutable-snapshot".into(),
                    resource_policy_stamp: r.policy_stamp.clone(),
                    frames: vec![CemModuleUrlFrame::new(
                        "recorded-browser",
                        &r.resolution_base,
                    )],
                },
            )),
            context,
        );
        let execution = execute_snapshot_xhtml_scoped(
            preflight,
            &cap,
            &r.bindings,
            &r.output_uri,
            Default::default(),
        )?;
        if execution.tree().ast().nodes.len() > MAX_NODES
            || execution.output().rendered.len() > MAX_BYTES
        {
            return Err("recorded original output exceeds observation bounds".into());
        }
        let host_paths = r.choices.iter().map(|c| c.host_paths.clone()).collect();
        let choices = r
            .choices
            .into_iter()
            .map(|c| BodyStylesheetInstallation {
                body_uri: c.body_uri,
                index: c.index,
                scope: match c.scope {
                    Scope::Private { tag } => CssManagedScope::Private { tag, context: None },
                    Scope::Shared { name } => CssManagedScope::Shared {
                        name,
                        context: None,
                    },
                },
            })
            .collect();
        let cancel = engine.operation_control.abort_signal().clone();
        Ok(Self {
            stage: Stage::Original,
            image_plan: None,
            svg_sources: None,
            svg_host: SvgComputedHost::default(),
            svg_evidence: None,
            paint: None,
            paint_content: None,
            paint_style: None,
            paint_colors: None,
            paint_colors_enabled: false,
            clone_profile: None,
            image_delivery: None,
            delivered_images: 0,
            replay_nonce: None,
            replay: None,
            image_record: None,
            pending_images: None,
            snapshot,
            engine,
            cap,
            execution,
            css,
            choices,
            host_paths,
            output: None,
            html: None,
            worker,
            handle: None,
            cancel,
            output_uri: r.output_uri,
            policy_stamp: r.policy_stamp,
            browser: r.browser,
        })
    }
    pub fn url(&self) -> &str {
        &self.output_uri
    }
    pub fn cancellation(&self) -> AbortSignal {
        self.cancel.clone()
    }
    pub fn html(&self) -> Result<String, String> {
        if self.stage == Stage::Closed || self.cancel.is_aborted() {
            return Err("recorded session closed".into());
        }
        let text = if let Some(output) = &self.output {
            output.html_output().rendered.clone()
        } else {
            self.execution.html_output()?.rendered
        };
        if text.len() > MAX_BYTES {
            return Err("recorded HTML delivery exceeds byte bound".into());
        }
        Ok(text)
    }
    pub fn close(&mut self) {
        self.stage = Stage::Closed;
        self.cancel.abort();
        self.engine.operation_control.abort_signal().abort();
        self.worker.close();
        self.html = None;
        self.handle = None;
        self.output = None;
        self.image_plan = None;
        self.svg_sources = None;
        self.svg_evidence = None;
        self.paint = None;
        self.paint_content = None;
        self.paint_style = None;
        self.paint_colors = None;
        self.paint_colors_enabled = false;
        self.clone_profile = None;
        let _ = self.svg_host.invalidate();
        self.image_delivery = None;
        self.delivered_images = 0;
        self.replay_nonce = None;
        self.replay = None;
        self.image_record = None;
        self.pending_images = None;
    }
    fn run<T>(&mut self, work: impl FnOnce(&mut Self) -> Result<T, String>) -> Result<T, String> {
        let result = if self.stage == Stage::Closed || self.cancel.is_aborted() {
            Err("recorded session closed".into())
        } else {
            work(self).and_then(|value| {
                if self.cancel.is_aborted() {
                    Err("recorded session cancelled during observation".into())
                } else {
                    Ok(value)
                }
            })
        };
        if result.is_err() {
            self.close();
        }
        result
    }
    pub fn prepare_base(
        &mut self,
        matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        decide: impl FnOnce(&HtmlBasePolicyRequest<'_>) -> Result<HtmlBasePolicyDecision, String>,
    ) -> Result<Option<u32>, String> {
        self.run(|s| {
            let (tree, next) = match s.stage {
                Stage::Original => (s.execution.tree().clone(), Stage::OriginalBase),
                Stage::Composed => (
                    s.output.as_ref().unwrap().tree().clone(),
                    Stage::ComposedBase,
                ),
                _ => return Err("recorded base preparation out of order".into()),
            };
            let mut html = HtmlBaseExecutionHost::new(
                HtmlBaseCompatibilityProfile::StandardAgreement,
                HtmlBaseBrowser {
                    engine: s.browser.engine.clone(),
                    version: s.browser.version.clone(),
                },
                HtmlBaseLimits {
                    nodes: MAX_NODES,
                    url_bytes: 4096,
                },
            )?;
            let selected = html.prepare(
                tree,
                HtmlBaseContext {
                    fallback: HtmlFallbackBase::DocumentUrl(s.output_uri.clone()),
                    encoding: HtmlBaseEncoding::Utf8,
                    policy_stamp: s.policy_stamp.clone(),
                },
                matches,
                decide,
            )?;
            s.html = Some(html);
            s.stage = next;
            Ok(selected)
        })
    }
    pub fn compose(
        &mut self,
        base: &str,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
    ) -> Result<String, String> {
        self.run(|s| {
            if s.stage != Stage::OriginalBase {
                return Err("recorded composition out of order".into());
            }
            let mut html = s.html.take().ok_or("original HTML host missing")?;
            let evidence = match html.current() {
                Some(e) => e.clone(),
                None => html.confirm(base.into())?,
            };
            let document = Arc::new(html.inspect_dependencies(
                &evidence,
                &s.cap,
                MAX_NODES,
                &mut matches,
                base,
            )?);
            let prepared = Arc::new(s.execution.prepare_dependencies(
                &mut html,
                document,
                &s.snapshot,
                &s.engine,
                &s.cap,
                &mut matches,
                base,
            )?);
            let css = Arc::new(prepared.prepare_css_imports(
                &mut html,
                &s.execution,
                &s.snapshot,
                &s.engine,
                &s.cap,
                s.css.clone(),
                Default::default(),
                matches,
                base,
            )?);
            let installations = css.compile_installations(&s.choices, MAX_BYTES)?;
            let head = (0..s.execution.tree().ast().nodes.len() as u32)
                .find(|id| {
                    s.execution.tree().node(*id).is_some_and(|n| {
                        n.name.as_ref().is_some_and(|n| {
                            n.namespace_uri == "http://www.w3.org/1999/xhtml"
                                && n.local_name == "head"
                        })
                    })
                })
                .ok_or("native output head required")?;
            s.output = Some(if let Some(nonce) = &s.replay_nonce {
                let replay = Arc::new(ImageReplayPlan::new(
                    s.execution.tree().clone(),
                    base,
                    &s.output_uri,
                    nonce,
                )?);
                let output = installations.compose_output_with_image_replay(
                    &vec![head; installations.len()],
                    &s.host_paths,
                    &replay,
                    MAX_NODES,
                    MAX_BYTES,
                )?;
                s.replay = Some(replay);
                output
            } else {
                installations.compose_output_with_hosts(
                    &vec![head; installations.len()],
                    &s.host_paths,
                    MAX_NODES,
                    MAX_BYTES,
                )?
            });
            s.stage = Stage::Composed;
            s.html()
        })
    }
    /// Native scalar source predicates and bounded selection commands, before
    /// any preparation evidence exists. A later capture revalidates the full DOM.
    pub fn prepare_stylesheets(
        &mut self,
        base: &str,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        title: Option<&str>,
    ) -> Result<
        (
            Vec<cem_ml::browser_stylesheet_sources::BrowserStylesheetSource>,
            Vec<cem_ml::html_stylesheet_selection::TitledStylesheetAction>,
        ),
        String,
    > {
        self.run(|s| {
            if s.stage != Stage::ComposedBase {
                return Err("composed base required for stylesheet preparation".into());
            }
            let html = s.html.as_mut().ok_or("composed HTML host missing")?;
            if html.current().is_some() {
                return Err("stylesheet preparation already started".into());
            }
            let evidence = match html.current() {
                Some(e) => e.clone(),
                None => html.confirm(base.into())?,
            };
            let document = Arc::new(html.inspect_dependencies(
                &evidence,
                &s.cap,
                MAX_NODES,
                &mut matches,
                base,
            )?);
            s.svg_sources = Some(
                document
                    .svg_presentations()
                    .iter()
                    .map(|p| p.source.clone())
                    .collect(),
            );
            s.image_plan = Some(ImagePlan::from_tree(
                document.document().tree.clone(),
                base,
                MAX_NODES,
                256,
            )?);
            let css = Arc::new(html.inspect_css_imports(
                document,
                &s.cap,
                s.css.clone(),
                Default::default(),
                s.cancel.clone(),
                matches,
                base,
            )?);
            let sources = cem_ml::browser_stylesheet_sources::browser_stylesheet_sources(
                &css, 256, MAX_BYTES,
            )?;
            let actions = if let Some(title) = title {
                cem_ml::html_stylesheet_selection::HtmlStylesheetSelection::new(css, title, 256)?
                    .actions()
                    .iter()
                    .map(
                        |a| cem_ml::html_stylesheet_selection::TitledStylesheetAction {
                            owner: a.owner,
                            link: a.link,
                            selected: a.selected,
                        },
                    )
                    .collect()
            } else {
                Vec::new()
            };
            Ok((sources, actions))
        })
    }
    pub fn enable_image_replay(&mut self, nonce: &str) -> Result<(), String> {
        self.run(|s| {
            if s.stage != Stage::Original
                || s.image_delivery.is_none()
                || s.replay_nonce.is_some()
                || nonce.len() != 32
                || !nonce
                    .bytes()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
            {
                return Err("fresh pre-delivery image replay nonce required".into());
            }
            s.replay_nonce = Some(nonce.into());
            Ok(())
        })
    }
    pub fn image_replay_routes(&self) -> Result<Vec<(String, String)>, String> {
        if self.stage != Stage::Composed || self.cancel.is_aborted() {
            return Err("composed replay routes required".into());
        }
        Ok(self
            .replay
            .as_ref()
            .ok_or("native replay required")?
            .routes()
            .iter()
            .map(|r| (r.url.clone(), r.source.clone()))
            .collect())
    }
    pub fn image_replay_bound(&self) -> bool {
        self.replay.is_some() && self.image_record.is_some() && self.delivered_images > 0
    }
    pub fn begin_image_delivery(&mut self) -> Result<(), String> {
        self.run(|s| {
            if s.stage != Stage::Original || s.image_delivery.is_some() {
                return Err("image transport must begin before HTML delivery".into());
            }
            s.image_delivery = Some(ImageDeliveryCapture::new(s.cancel.clone()));
            Ok(())
        })
    }
    pub fn accept_image_delivery(&mut self, response: ImageResponse) -> Result<(), String> {
        self.run(|s| {
            if s.stage == Stage::Captured || s.stage == Stage::Closed || s.image_record.is_some() {
                return Err("image delivery after publication".into());
            }
            if url::Url::parse(&response.requested_url)
                .map_err(|_| "image URL required")?
                .origin()
                != url::Url::parse(&s.output_uri)
                    .map_err(|_| "output URL required")?
                    .origin()
            {
                return Err("same-origin image delivery required".into());
            }
            s.image_delivery
                .as_mut()
                .ok_or("image transport not begun")?
                .accept(response)
        })
    }
    pub fn record_delivered_images(
        &mut self,
        environment: &str,
        mut observe: impl FnMut(u32) -> Result<ImageObservation, String>,
    ) -> Result<(), String> {
        self.run(|s| {
            if s.stage != Stage::ComposedBase {
                return Err("image delivery join out of order".into());
            }
            let selected = s
                .pending_images
                .as_ref()
                .ok_or("image selection required")?
                .iter()
                .map(|o| o.url.clone())
                .collect::<Vec<_>>();
            let responses = s
                .image_delivery
                .as_mut()
                .ok_or("image transport not begun")?
                .seal(&selected)?;
            let count = responses.len();
            s.record_images(responses, environment, &mut observe)?;
            s.delivered_images = count;
            Ok(())
        })
    }
    pub fn image_deliveries(&self) -> usize {
        self.delivered_images
    }
    pub fn image_acquisitions(&self) -> usize {
        self.image_record
            .as_ref()
            .map_or(0, |r| r.responses().len())
    }
    pub fn select_images(
        &mut self,
        mut observe: impl FnMut(u32) -> Result<ImageObservation, String>,
    ) -> Result<Vec<(u32, String)>, String> {
        self.run(|s| {
            if s.stage != Stage::ComposedBase
                || s.pending_images.is_some()
                || s.image_record.is_some()
            {
                return Err("fresh composed image preparation required".into());
            }
            let plan = s.image_plan.as_ref().ok_or("native image plan required")?;
            let observations = plan.observe(&mut observe)?;
            if observations != plan.observe(observe)? {
                return Err("image selection changed".into());
            }
            let requests = plan
                .targets()
                .iter()
                .zip(&observations)
                .map(|(t, o)| (t.owner, o.url.clone()))
                .collect();
            s.pending_images = Some(observations);
            Ok(requests)
        })
    }
    pub fn record_images(
        &mut self,
        responses: Vec<ImageResponse>,
        environment: &str,
        mut observe: impl FnMut(u32) -> Result<ImageObservation, String>,
    ) -> Result<(), String> {
        self.run(|s| {
            if s.stage != Stage::ComposedBase {
                return Err("image acquisition out of order".into());
            }
            let plan = s.image_plan.as_ref().ok_or("image plan required")?;
            let pending = s.pending_images.take().ok_or("image selection required")?;
            if pending != plan.observe(&mut observe)? {
                return Err("image changed during acquisition".into());
            }
            s.image_record = Some(ImageRecord::capture(
                plan.clone(),
                responses,
                environment,
                observe,
                s.cancel.clone(),
                MAX_BYTES,
            )?);
            Ok(())
        })
    }
    pub fn record_svg(&mut self, observer: &mut impl SvgComputedObserver) -> Result<(), String> {
        self.run(|s| {
            if s.stage != Stage::ComposedBase || s.svg_evidence.is_some() {
                return Err("fresh composed SVG preparation required".into());
            }
            let sources = s
                .svg_sources
                .as_ref()
                .ok_or("SVG source preparation required")?;
            s.svg_evidence = Some(s.svg_host.capture(sources, observer)?);
            Ok(())
        })
    }
    pub fn validate_svg(&mut self, observer: &mut impl SvgComputedObserver) -> Result<(), String> {
        self.run(|s| {
            let evidence = s
                .svg_evidence
                .as_ref()
                .ok_or("SVG computed capture required")?;
            let sources = s
                .svg_sources
                .as_ref()
                .ok_or("SVG source preparation required")?;
            s.svg_host.validate(evidence, sources, observer)
        })
    }
    /// Diagnostic counts only; the plan and its inventory never leave the session.
    pub fn svg_paint_counts(&self) -> (usize, usize) {
        if self.stage != Stage::Captured || self.cancel.is_aborted() {
            return (0, 0);
        }
        self.paint.as_ref().map_or((0, 0), |p| {
            (
                p.roots().len(),
                p.roots().iter().map(|r| r.links.len()).sum(),
            )
        })
    }
    /// Bounded diagnostics; retained content and source ancestry stay native.
    pub fn svg_paint_content_counts(&self) -> (usize, usize) {
        if self.stage != Stage::Captured || self.cancel.is_aborted() {
            return (0, 0);
        }
        self.paint_content.as_ref().map_or((0, 0), |p| {
            (
                p.servers().len(),
                p.servers().iter().map(|s| s.nodes.len()).sum(),
            )
        })
    }
    /// Prospective contexts only; no browser clone or computed-style evidence.
    pub fn svg_paint_style_counts(&self) -> (usize, usize) {
        if self.stage != Stage::Captured || self.cancel.is_aborted() {
            return (0, 0);
        }
        self.paint_style.as_ref().map_or((0, 0), |p| {
            (
                p.hosts().len(),
                p.hosts().iter().map(|h| h.instances.len()).sum(),
            )
        })
    }
    /// Compatibility configuration/report validation, never an execution seal.
    pub fn confirm_clone_profile(
        &mut self,
        name: &str,
        mut observe: impl FnMut(CloneProbe) -> Result<CloneProbeObservation, String>,
    ) -> Result<(), String> {
        self.run(|s| {
            if s.stage != Stage::ComposedBase {
                return Err("clone profile requires composed base preparation".into());
            }
            if s.svg_sources.is_none() || s.clone_profile.is_some() {
                return Err("clone profile requires prepared sources and one confirmation".into());
            }
            if name != "chromium-148-solid-currentcolor-v1" {
                return Err("unknown SVG clone compatibility profile".into());
            }
            let profile = SvgPaintCloneProfile::for_browser(&s.browser.engine, &s.browser.version)?;
            let mut observations = Vec::new();
            for case in CloneProbe::ALL {
                let observation = observe(case)?;
                if observation.case != case {
                    return Err("clone probe identity mismatch".into());
                }
                observations.push(observation);
            }
            profile.verify_probes(&observations)?;
            s.clone_profile = Some(profile);
            Ok(())
        })
    }
    pub fn clone_profile_confirmed(&self) -> bool {
        self.stage == Stage::Captured && !self.cancel.is_aborted() && self.clone_profile.is_some()
    }
    pub fn enable_paint_colors(&mut self) -> Result<(), String> {
        self.run(|s| {
            if s.stage != Stage::ComposedBase || s.clone_profile.is_none() || s.paint_colors_enabled
            {
                return Err(
                    "paint colors require confirmed profile and one pre-capture opt-in".into(),
                );
            }
            s.paint_colors_enabled = true;
            Ok(())
        })
    }
    pub fn paint_color_counts(&self) -> (usize, usize) {
        if self.stage != Stage::Captured || self.cancel.is_aborted() {
            return (0, 0);
        }
        self.paint_colors
            .as_ref()
            .map_or((0, 0), |p| (p.colors().len(), p.suppressed_hosts().len()))
    }
    /// Compare source CSSOM values only, never claim to observe hidden clones.
    pub fn validate_paint_colors(
        &mut self,
        mut observe: impl FnMut(cem_ml::parser::AstNodeId, &str) -> Result<String, String>,
    ) -> Result<(), String> {
        self.run(|s| {
            if s.stage != Stage::Captured {
                return Err("paint color validation requires capture".into());
            }
            if let Some(colors) = &s.paint_colors {
                for color in colors.colors() {
                    let property = color.property;
                    let [r, g, b, a] = color.rgba;
                    let expected = if a == 255 {
                        format!("rgb({r}, {g}, {b})")
                    } else {
                        format!("rgba({r}, {g}, {b}, 0)")
                    };
                    if observe(color.source, property)? != expected {
                        return Err(
                            "paint color source CSSOM disagrees with native derivation".into()
                        );
                    }
                }
            }
            Ok(())
        })
    }
    fn paint_blockers(&self, mut blockers: Vec<String>) -> Vec<String> {
        if let Some(profile) = &self.clone_profile {
            blockers.extend(profile.blockers().iter().map(|b| (*b).to_owned()));
            blockers.sort();
            blockers.dedup();
        }
        if let Some(colors) = &self.paint_colors {
            blockers.extend(colors.blockers().iter().map(|b| (*b).to_owned()));
            blockers.sort();
            blockers.dedup();
        }
        let Some(plan) = &self.paint else {
            return blockers;
        };
        if plan.roots().is_empty() {
            return blockers;
        }
        if let Some(content) = &self.paint_content {
            if !content.servers().is_empty() {
                blockers.extend(content.blockers().iter().map(|b| (*b).to_owned()));
            }
        }
        if let Some(style) = &self.paint_style {
            if !style.hosts().is_empty() {
                blockers.extend(style.blockers().iter().map(|b| (*b).to_owned()));
            }
        }
        blockers.extend(plan.blockers().iter().map(|b| (*b).to_owned()));
        for target in plan
            .roots()
            .iter()
            .flat_map(|r| std::iter::once(&r.target).chain(r.links.iter().map(|l| &l.target)))
        {
            let blocker = match target {
                PaintTarget::Local(_) => continue,
                PaintTarget::Missing => "svg-paint-target-missing",
                PaintTarget::Ambiguous => "svg-paint-target-ambiguous",
                PaintTarget::WrongKind(_) => "svg-paint-target-kind-unsupported",
                PaintTarget::Cycle(_) => "svg-paint-template-cycle",
                PaintTarget::UnsupportedFragment => "svg-paint-fragment-unsupported",
                PaintTarget::UntraversedUrl => "svg-paint-url-untraversed",
                PaintTarget::BaseContextRequired => "svg-paint-base-context-required",
            };
            blockers.push(blocker.into());
        }
        blockers.sort();
        blockers.dedup();
        blockers
    }
    pub fn svg_computed_slots(&self) -> usize {
        self.svg_evidence.as_ref().map_or(0, |e| e.values().len())
    }
    pub fn validate_images(
        &mut self,
        environment: &str,
        observe: impl FnMut(u32) -> Result<ImageObservation, String>,
    ) -> Result<(), String> {
        self.run(|s| {
            let plan = s.image_plan.as_ref().ok_or("image plan required")?;
            if plan.targets().is_empty() {
                return Ok(());
            }
            s.image_record
                .as_mut()
                .ok_or("image selection and acquisition required")?
                .validate(plan, environment, observe)
        })
    }
    pub fn capture(
        &mut self,
        base: &str,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        observer: &mut impl StylesheetExecutionObserver,
    ) -> Result<Vec<String>, String> {
        self.run(|s| {
            if s.stage != Stage::ComposedBase {
                return Err("recorded capture out of order".into());
            }
            let mut html = s.html.take().ok_or("composed HTML host missing")?;
            let evidence = match html.current() {
                Some(e) => e.clone(),
                None => html.confirm(base.into())?,
            };
            let document = Arc::new(html.inspect_dependencies(
                &evidence,
                &s.cap,
                MAX_NODES,
                &mut matches,
                base,
            )?);
            let paint = Arc::new(SvgPaintPlan::from_inventory(document.clone(), 1_000_000)?);
            let content = Arc::new(SvgPaintContentPlan::new(paint.clone(), 1_000_000)?);
            let style = Arc::new(SvgPaintStylePlan::new(content.clone(), 1_000_000)?);
            let colors = if s.paint_colors_enabled {
                Some(BoundedPaintColors::evaluate(
                    style.clone(),
                    s.clone_profile.ok_or("paint color profile missing")?,
                    1_000_000,
                )?)
            } else {
                None
            };
            let mut paint_observer = PaintStylesheetObserver::new(colors.as_ref(), observer);
            let handle = s.worker.prepare(
                s.output.as_ref().unwrap().clone(),
                html,
                document,
                s.cancel.clone(),
                &mut paint_observer,
                &mut matches,
                base,
                Default::default(),
            )?;
            let blockers = s.worker.read(&handle, &mut paint_observer, matches, base)?;
            paint_observer.complete()?;
            s.handle = Some(handle);
            s.paint = Some(paint);
            s.paint_content = Some(content);
            s.paint_style = Some(style);
            s.paint_colors = colors;
            s.stage = Stage::Captured;
            Ok(s.paint_blockers(blockers))
        })
    }
    pub fn read(
        &mut self,
        base: &str,
        matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        observer: &mut impl StylesheetExecutionObserver,
    ) -> Result<Vec<String>, String> {
        self.run(|s| {
            if s.stage != Stage::Captured {
                return Err("recorded read requires capture".into());
            }
            let mut paint_observer =
                PaintStylesheetObserver::new(s.paint_colors.as_ref(), observer);
            let blockers = s.worker.read(
                s.handle.as_ref().unwrap(),
                &mut paint_observer,
                matches,
                base,
            )?;
            paint_observer.complete()?;
            Ok(s.paint_blockers(blockers))
        })
    }
}
impl Drop for RecordedOutputSession {
    fn drop(&mut self) {
        self.close();
    }
}

/// Join author-rule derivations to the existing sealed whole-output stylesheet
/// observations. These checks run inside capture/read, before any publication.
struct PaintStylesheetObserver<'a, O> {
    inner: &'a mut O,
    required: Vec<(
        cem_ml::parser::AstNodeId,
        Arc<cem_ml::parser::tree::RetainedCemTree>,
        bool,
    )>,
}
impl<'a, O: StylesheetExecutionObserver> PaintStylesheetObserver<'a, O> {
    fn new(colors: Option<&BoundedPaintColors>, inner: &'a mut O) -> Self {
        let required = colors
            .into_iter()
            .flat_map(|c| c.plan().content().paint().inventory().stylesheets())
            .filter_map(|sheet| {
                // The evaluator has already checked successful imports, exact
                // SVG owners and absence of selection/conditional metadata.
                (sheet.owner == sheet.source_node).then(|| {
                    (
                        sheet.owner,
                        sheet.resources.as_ref().unwrap().tree.clone(),
                        false,
                    )
                })
            })
            .collect();
        Self { inner, required }
    }
    fn complete(&self) -> Result<(), String> {
        if self.required.iter().any(|(_, _, seen)| !seen) {
            return Err("paint stylesheet installation observation missing".into());
        }
        Ok(())
    }
}
impl<O: StylesheetExecutionObserver> StylesheetExecutionObserver
    for PaintStylesheetObserver<'_, O>
{
    fn environment_revision(&mut self) -> Result<String, String> {
        self.inner.environment_revision()
    }
    fn observe(
        &mut self,
        slot: cem_ml::stylesheet_execution::StylesheetExecutionSlot<'_>,
    ) -> Result<cem_ml::stylesheet_execution::StylesheetObservation, String> {
        use cem_ml::stylesheet_execution::StylesheetOwner;
        let required = match slot.owner {
            StylesheetOwner::Document { owner, source_node }
                if owner == source_node && slot.parent_sheet.is_none() =>
            {
                self.required.iter_mut().find(|(id, _, _)| id == owner)
            }
            _ => None,
        };
        if let Some((_, tree, seen)) = required {
            if !Arc::ptr_eq(tree, slot.tree)
                || !slot.media.is_empty()
                || !slot.title.is_empty()
                || slot.disabled
                || slot.alternate
            {
                return Err("paint stylesheet installation owner mismatch".into());
            }
            let key = slot.source_key.to_owned();
            let observation = self.inner.observe(slot)?;
            if !observation.selected
                || !observation.conditions_match
                || observation.installation.is_none_or(|id| id == 0)
                || observation.loaded_source.as_deref() != Some(key.as_str())
            {
                return Err("paint stylesheet requires enabled source installation".into());
            }
            *seen = true;
            Ok(observation)
        } else {
            self.inner.observe(slot)
        }
    }
}
