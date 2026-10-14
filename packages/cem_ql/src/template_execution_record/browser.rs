//! Bounded source-ingress and staged delivery contract for synchronous browser
//! observation. All semantic trees and preparation evidence stay native.
use super::*;
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
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn output_tree(&self) -> Result<Arc<RetainedCemTree>, String> {
        self.output
            .as_ref()
            .map(|o| o.tree().clone())
            .ok_or("composed native output required".into())
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
            let evidence = html.confirm(base.into())?;
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
            s.output = Some(installations.compose_output_with_hosts(
                &vec![head; installations.len()],
                &s.host_paths,
                MAX_NODES,
                MAX_BYTES,
            )?);
            s.stage = Stage::Composed;
            s.html()
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
            let evidence = html.confirm(base.into())?;
            let document = Arc::new(html.inspect_dependencies(
                &evidence,
                &s.cap,
                MAX_NODES,
                &mut matches,
                base,
            )?);
            let handle = s.worker.prepare(
                s.output.as_ref().unwrap().clone(),
                html,
                document,
                s.cancel.clone(),
                observer,
                &mut matches,
                base,
                Default::default(),
            )?;
            let blockers = s.worker.read(&handle, observer, matches, base)?;
            s.handle = Some(handle);
            s.stage = Stage::Captured;
            Ok(blockers)
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
            s.worker
                .read(s.handle.as_ref().unwrap(), observer, matches, base)
        })
    }
}
impl Drop for RecordedOutputSession {
    fn drop(&mut self) {
        self.close();
    }
}
