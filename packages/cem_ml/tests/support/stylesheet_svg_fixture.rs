//! Explicit JSON browser observation protocol, never an AST handoff.
use cem_ml::{
    css_import_snapshot::*, html_base::*, html_base_evidence::*, html_base_host::*,
    module_resolution::*, scheduler::AbortSignal, stylesheet_execution::*,
};
use serde::Deserialize;
use std::sync::Arc;
#[derive(Deserialize)]
pub struct BrowserEvidence {
    pub version: String,
    pub base: String,
    pub observations: Vec<Observation>,
}
#[derive(Deserialize)]
pub struct Observation {
    pub root: usize,
    pub sheet: usize,
    pub source: Option<String>,
    pub url: Option<String>,
    pub installation: Option<u64>,
    pub selected: bool,
    pub conditions_match: bool,
}
pub fn verify(evidence: BrowserEvidence) -> serde_json::Value {
    let tree = cem_ml::import::import_data_bytes(
        include_bytes!("../fixtures/stylesheet-svg.xhtml"),
        "application/xml",
        "cem",
        "urn:stylesheet-svg",
    )
    .unwrap();
    let mut html = HtmlBaseExecutionHost::new(
        HtmlBaseCompatibilityProfile::StandardAgreement,
        HtmlBaseBrowser {
            engine: "chromium".into(),
            version: evidence.version.clone(),
        },
        HtmlBaseLimits {
            nodes: 1000,
            url_bytes: 4096,
        },
    )
    .unwrap();
    let slots = |slot: HtmlBaseDomSlot<'_>| Ok(slot.id + 1);
    html.prepare(
        tree,
        HtmlBaseContext {
            fallback: HtmlFallbackBase::DocumentUrl("https://page.test/docs/styles.xhtml".into()),
            encoding: HtmlBaseEncoding::Utf8,
            policy_stamp: "v1".into(),
        },
        slots,
        |_| Ok(HtmlBasePolicyDecision::Allow),
    )
    .unwrap();
    let base = html.confirm(evidence.base.clone()).unwrap();
    let handle = CemResolutionContextHandle::new("browser");
    let cap = CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(
            handle.clone(),
            CemModuleUrlContext {
                identity: "browser".into(),
                resolver_identity: "native".into(),
                resource_policy_stamp: "v1".into(),
                frames: vec![CemModuleUrlFrame::new("browser", evidence.base.clone())],
            },
        )),
        handle,
    );
    let document = Arc::new(
        html.inspect_dependencies(&base, &cap, 10000, slots, &evidence.base)
            .unwrap(),
    );
    let urls: Vec<_> = document
        .svg_presentations()
        .iter()
        .flat_map(|p| &p.resources.references)
        .filter(|r| !r.authored_specifier.starts_with('#'))
        .map(|r| r.resolution.as_ref().unwrap().resolved_url.clone())
        .collect();
    let local_fragments: Vec<_> = document
        .svg_presentations()
        .iter()
        .flat_map(|p| &p.resources.references)
        .filter(|r| r.authored_specifier.starts_with('#'))
        .map(|r| r.authored_specifier.clone())
        .collect();
    let snapshot = CssSourceSnapshot::new(
        vec![
            CssSnapshotSource {
                requested_url: "https://page.test/assets/theme.css".into(),
                final_url: "https://page.test/assets/theme.css".into(),
                revision: "main:1".into(),
                content_type: "text/css".into(),
                bytes: b"@import './child.css' screen; p{background-color:rgb(4,5,6)}".to_vec(),
            },
            CssSnapshotSource {
                requested_url: "https://page.test/assets/child.css".into(),
                final_url: "https://page.test/assets/child.css".into(),
                revision: "child:1".into(),
                content_type: "text/css".into(),
                bytes: b"p{border-top:1px solid rgb(7,8,9)}".to_vec(),
            },
        ],
        Default::default(),
    )
    .unwrap();
    let css = Arc::new(
        html.inspect_css_imports(
            document,
            &cap,
            snapshot.clone(),
            Default::default(),
            AbortSignal::new(),
            slots,
            &evidence.base,
        )
        .unwrap(),
    );
    struct Observer<'a> {
        data: &'a [Observation],
        revision: u64,
        snapshot: &'a CssSourceSnapshot,
    }
    impl StylesheetExecutionObserver for Observer<'_> {
        fn environment_revision(&mut self) -> Result<String, String> {
            Ok(self.revision.to_string())
        }
        fn observe(
            &mut self,
            slot: StylesheetExecutionSlot<'_>,
        ) -> Result<StylesheetObservation, String> {
            let actual = self
                .data
                .iter()
                .find(|o| o.root == slot.root && o.sheet == slot.sheet)
                .ok_or("missing browser occurrence")?;
            if let Some(source) = &actual.source {
                if slot.root != 0 && actual.url.as_deref() != Some(slot.url) {
                    return Err(format!(
                        "browser stylesheet URL {:?} differs from {} at root {} sheet {}",
                        actual.url, slot.url, slot.root, slot.sheet
                    ));
                }
                let expected = if slot.root == 0 {
                    b"p{color:rgb(1,2,3)}".as_slice()
                } else if slot.sheet == 0 {
                    &self
                        .snapshot
                        .source("https://page.test/assets/theme.css")
                        .unwrap()
                        .bytes
                } else {
                    &self
                        .snapshot
                        .source("https://page.test/assets/child.css")
                        .unwrap()
                        .bytes
                };
                if source.as_bytes() != expected {
                    return Err("browser loaded different CSS bytes".into());
                }
            }
            Ok(StylesheetObservation {
                selected: actual.selected,
                conditions_match: actual.conditions_match,
                loaded_source: actual.source.as_ref().map(|_| slot.source_key.into()),
                installation: actual.installation,
            })
        }
    }
    let mut observer = Observer {
        data: &evidence.observations,
        revision: 1,
        snapshot: &snapshot,
    };
    let mut host = StylesheetExecutionHost::default();
    let record = css
        .record_execution(
            &mut html,
            &cap,
            &snapshot,
            &mut host,
            &mut observer,
            32,
            slots,
            &evidence.base,
        )
        .unwrap();
    record
        .validate(
            &mut html,
            &cap,
            &snapshot,
            &mut host,
            &mut observer,
            slots,
            &evidence.base,
        )
        .unwrap();
    observer.revision = 2;
    assert!(record
        .validate(
            &mut html,
            &cap,
            &snapshot,
            &mut host,
            &mut observer,
            slots,
            &evidence.base
        )
        .is_err());
    observer.revision = 1;
    assert!(record
        .validate(
            &mut html,
            &cap,
            &snapshot,
            &mut host,
            &mut observer,
            slots,
            &evidence.base
        )
        .is_err());
    serde_json::json!({"observations":record.execution().observations().len(),"svg_urls":urls,"local_fragments":local_fragments,"stale_rejections":2})
}
