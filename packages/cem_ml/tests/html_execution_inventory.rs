use cem_ml::{
    html_base::*, html_base_evidence::*, html_base_host::*, module_resolution::*,
    parser::tree::RetainedCemTree,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[path = "support/html_execution_inventory_fixture.rs"]
mod browser_fixture;

#[test]
fn shared_browser_reference_and_inline_css_candidates() {
    let result = browser_fixture::verify(&[browser_fixture::InventoryObservation {
        document_url: "https://example.test/docs/inventory.html".into(),
        base_uri: "https://example.test/assets/".into(),
        engine: "chromium".into(),
        version: CHROMIUM_BASE_PROFILE_VERSION.into(),
        image_url: "https://example.test/assets/images/a.svg?x=1&y=2".into(),
        navigation_url: "https://example.test/assets/next.html".into(),
        stylesheet_url: "https://example.test/assets/sheet.svg".into(),
        inline_url: "https://example.test/assets/inline.svg".into(),
        srcset_urls: vec![
            "data:image/svg+xml,%3Csvg%20xmlns='http://www.w3.org/2000/svg'%3E%3C/svg%3E".into(),
            "https://example.test/assets/images/a,b.svg".into(),
            "https://example.test/assets/caf%C3%A9.svg?x=1&y=2".into(),
            "https://example.test/assets/width.svg".into(),
            "https://example.test/assets/tail.svg".into(),
            "https://example.test/assets/%C2%A0space.svg".into(),
            "https://example.test/assets/zero.svg".into(),
        ],
    }]);
    assert_eq!(result.len(), 1);
}

const SOURCE: &str = r#"<html xmlns="http://www.w3.org/1999/xhtml"><head><style>p { background: url(./sheet.svg) } @import "./theme.css";</style><base href="../assets/"/></head><body><img src="images/a.svg?x=1&amp;y=2"/><a href="next.html">Next</a><p style="background: url(icon)">Sample</p></body></html>"#;
fn tree(source: &str) -> Arc<RetainedCemTree> {
    cem_ml::import::import_data_bytes(
        source.as_bytes(),
        "application/xml",
        "cem",
        "studio://source/original.xhtml",
    )
    .unwrap()
}
fn slots(slot: HtmlBaseDomSlot<'_>) -> Result<u32, String> {
    Ok(slot.id + 1)
}
fn host(
    source: &str,
    href: &str,
    profile: HtmlBaseCompatibilityProfile,
) -> (HtmlBaseExecutionHost, Arc<CapturedHtmlBaseEvidence>) {
    let mut host = HtmlBaseExecutionHost::new(
        profile,
        HtmlBaseBrowser {
            engine: "chromium".into(),
            version: CHROMIUM_BASE_PROFILE_VERSION.into(),
        },
        HtmlBaseLimits {
            nodes: 100,
            url_bytes: 4096,
        },
    )
    .unwrap();
    host.prepare(
        tree(source),
        HtmlBaseContext {
            fallback: HtmlFallbackBase::DocumentUrl("https://example.test/docs/page.html".into()),
            encoding: HtmlBaseEncoding::Utf8,
            policy_stamp: "base:allow:1".into(),
        },
        slots,
        |_| Ok(HtmlBasePolicyDecision::Allow),
    )
    .unwrap();
    let evidence = host.confirm(href.into()).unwrap();
    (host, evidence)
}
fn capability(block: bool) -> CemModuleUrlResolutionCapability {
    let handle = CemResolutionContextHandle::new("execution");
    let mut frame = CemModuleUrlFrame::new("execution", "https://maps.test/map.json");
    frame.specifiers.resources.insert(
        "icon".into(),
        if block {
            CemModuleUrlMapping::blocked()
        } else {
            CemModuleUrlMapping::target("./mapped.svg")
        },
    );
    CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(
            handle.clone(),
            CemModuleUrlContext {
                identity: "execution:1".into(),
                resolver_identity: "resolver:1".into(),
                resource_policy_stamp: "resources:1".into(),
                frames: vec![frame],
            },
        )),
        handle,
    )
}

#[test]
fn confirmed_base_resolves_document_and_css_with_native_source_owners() {
    let (mut host, evidence) = host(
        SOURCE,
        "https://example.test/assets/",
        HtmlBaseCompatibilityProfile::StandardAgreement,
    );
    let cap = capability(false);
    let inventory = host
        .inspect_dependencies(&evidence, &cap, 100, slots, evidence.effective_url())
        .unwrap();
    assert!(Arc::ptr_eq(
        &inventory.document().tree,
        &evidence.native_resolution().tree
    ));
    assert_eq!(inventory.references().len(), 2); // base href is context, not a resource
    assert_eq!(
        inventory.references()[0].resolved_uri.as_ref().unwrap(),
        "https://example.test/assets/images/a.svg?x=1&y=2"
    );
    assert_eq!(
        inventory.references()[1].resolved_uri.as_ref().unwrap(),
        "https://example.test/assets/next.html"
    );
    for reference in inventory.references() {
        assert_eq!(
            reference.source_uri.as_deref(),
            Some("studio://source/original.xhtml")
        );
        assert!(reference.source_key.is_some());
        assert!(!reference.reference.source.frames.is_empty());
        assert!(inventory
            .document()
            .tree
            .node(reference.reference.attribute)
            .is_some());
    }
    let sheets = inventory.stylesheets();
    assert_eq!(sheets.len(), 2);
    let css = sheets[0].resources.as_ref().unwrap();
    assert_eq!(
        css.references[0].resolution.as_ref().unwrap().resolved_url,
        "https://example.test/assets/sheet.svg"
    );
    assert_eq!(
        css.references[1].resolution.as_ref().unwrap().resolved_url,
        "https://example.test/assets/theme.css"
    );
    assert!(css.tree.node(css.references[0].node_id).is_some());
    assert!(!sheets[0].host_source.frames.is_empty());
    assert_eq!(
        sheets[1].resources.as_ref().unwrap().references[0]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://maps.test/mapped.svg"
    );
    assert!(inventory.document().body_analysis_required);
    assert!(inventory
        .blockers()
        .iter()
        .any(|b| b == "runtime-dependency-closure-required"));
    assert!(!inventory
        .blockers()
        .iter()
        .any(|b| b == "document-base-context-required"));
    host.validate_dependencies(&inventory, &cap, slots, evidence.effective_url())
        .unwrap();
}

#[test]
fn stale_foreign_recaptured_and_changed_dom_inventories_fail() {
    let cap = capability(false);
    let (mut first, evidence) = host(
        SOURCE,
        "https://example.test/assets/",
        HtmlBaseCompatibilityProfile::StandardAgreement,
    );
    let inventory = first
        .inspect_dependencies(&evidence, &cap, 100, slots, evidence.effective_url())
        .unwrap();
    let (mut second, fresh) = host(
        SOURCE,
        evidence.effective_url(),
        HtmlBaseCompatibilityProfile::StandardAgreement,
    );
    assert!(second
        .inspect_dependencies(&evidence, &cap, 100, slots, evidence.effective_url())
        .is_err());
    assert!(second
        .validate_dependencies(&inventory, &cap, slots, evidence.effective_url())
        .is_err());
    first.invalidate().unwrap();
    assert!(first
        .inspect_dependencies(&evidence, &cap, 100, slots, evidence.effective_url())
        .is_err());
    assert!(first
        .validate_dependencies(&inventory, &cap, slots, evidence.effective_url())
        .is_err());
    first
        .prepare(
            tree(SOURCE),
            evidence.native_resolution().context.clone(),
            slots,
            |_| Ok(HtmlBasePolicyDecision::Allow),
        )
        .unwrap();
    let recaptured = first.confirm(evidence.effective_url().into()).unwrap();
    assert!(!Arc::ptr_eq(
        &recaptured.native_resolution().tree,
        &inventory.document().tree
    ));
    assert!(first
        .validate_dependencies(&inventory, &cap, slots, evidence.effective_url())
        .is_err());
    assert!(first
        .inspect_dependencies(&recaptured, &cap, 100, slots, recaptured.effective_url())
        .is_ok());
    let current = second
        .inspect_dependencies(&fresh, &cap, 100, slots, fresh.effective_url())
        .unwrap();
    assert!(second
        .validate_dependencies(
            &current,
            &cap,
            |slot| Ok(slot.id + 100),
            fresh.effective_url()
        )
        .is_err());
}

struct ObservedResolver {
    delegate: CemModuleUrlResolutionCapability,
    calls: Arc<AtomicUsize>,
    revision: Arc<AtomicUsize>,
    drift: bool,
    cacheable: bool,
}
impl CemModuleUrlResolver for ObservedResolver {
    fn context_cache_identity(&self, _: &CemResolutionContextHandle) -> Option<String> {
        self.cacheable
            .then(|| format!("revision:{}", self.revision.load(Ordering::SeqCst)))
    }
    fn resolve_module_url(
        &self,
        request: &CemModuleUrlResolutionRequest,
    ) -> Result<CemModuleUrlResolution, CemModuleUrlResolutionError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.drift {
            self.revision.fetch_add(1, Ordering::SeqCst);
        }
        self.delegate.resolve_with_referrer(
            request.purpose,
            &request.authored_specifier,
            request.referrer.clone().unwrap(),
            request.source_map.clone(),
        )
    }
}
#[test]
fn bounds_and_staleness_precede_resolution_and_resolver_drift_cannot_publish() {
    for mode in ["bounds", "stale", "drift", "uncacheable"] {
        let calls = Arc::new(AtomicUsize::new(0));
        let cap = CemModuleUrlResolutionCapability::new(
            Arc::new(ObservedResolver {
                delegate: capability(false),
                calls: calls.clone(),
                revision: Arc::new(AtomicUsize::new(1)),
                drift: mode == "drift",
                cacheable: mode != "uncacheable",
            }),
            CemResolutionContextHandle::new("execution"),
        );
        let (mut host, evidence) = host(
            SOURCE,
            "https://example.test/assets/",
            HtmlBaseCompatibilityProfile::StandardAgreement,
        );
        if mode == "stale" {
            host.invalidate().unwrap();
        }
        assert!(host
            .inspect_dependencies(
                &evidence,
                &cap,
                if mode == "bounds" { 2 } else { 100 },
                slots,
                evidence.effective_url()
            )
            .is_err());
        assert_eq!(calls.load(Ordering::SeqCst) > 0, mode == "drift");
    }
}

#[test]
fn resolver_replacement_denial_and_compatibility_remain_explicit() {
    let (mut host, evidence) = host(
        SOURCE,
        "https://example.test/assets/",
        HtmlBaseCompatibilityProfile::StandardAgreement,
    );
    let cap = capability(true);
    let inventory = host
        .inspect_dependencies(&evidence, &cap, 100, slots, evidence.effective_url())
        .unwrap();
    let denied = inventory.stylesheets()[1]
        .resources
        .as_ref()
        .unwrap()
        .references[0]
        .resolution
        .as_ref()
        .unwrap_err();
    assert_eq!(denied.reason, CemModuleUrlResolutionErrorReason::Blocked);
    assert_eq!(denied.resource_policy_stamp, "resources:1");
    assert!(inventory
        .blockers()
        .iter()
        .any(|b| b == "inline-css-resolution-incomplete"));
    assert!(host
        .validate_dependencies(
            &inventory,
            &capability(true),
            slots,
            evidence.effective_url()
        )
        .is_err());
    let source = SOURCE.replace("../assets/", "http://[");
    let (mut chromium, evidence) = self::host(
        &source,
        "about:blank",
        HtmlBaseCompatibilityProfile::Chromium148,
    );
    let inventory = chromium
        .inspect_dependencies(&evidence, &cap, 100, slots, "about:blank")
        .unwrap();
    assert_eq!(
        inventory.base().compatibility(),
        HtmlBaseCompatibility::ChromiumInvalidHref
    );
    assert!(inventory.references()[0].resolved_uri.is_err());
    assert_eq!(
        inventory.base().native_resolution().effective_url,
        "https://example.test/docs/page.html"
    );
}

#[test]
fn imported_xml_bases_and_unicode_whitespace_do_not_override_execution_urls() {
    let source = SOURCE
        .replace("<body>", "<body xml:base=\"https://wrong.test/\">")
        .replace("images/a.svg?x=1&amp;y=2", "\u{a0}image.svg");
    let (mut host, evidence) = host(
        &source,
        "https://example.test/assets/",
        HtmlBaseCompatibilityProfile::StandardAgreement,
    );
    let inventory = host
        .inspect_dependencies(
            &evidence,
            &capability(false),
            100,
            slots,
            evidence.effective_url(),
        )
        .unwrap();
    assert_eq!(
        inventory.references()[0].resolved_uri.as_ref().unwrap(),
        "https://example.test/assets/%C2%A0image.svg"
    );
    assert_eq!(
        inventory.references()[1].resolved_uri.as_ref().unwrap(),
        "https://example.test/assets/next.html"
    );
    assert_eq!(
        inventory.stylesheets()[0]
            .resources
            .as_ref()
            .unwrap()
            .references[0]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://example.test/assets/sheet.svg"
    );
}

#[test]
fn srcset_candidates_keep_decoded_sources_and_confirmed_execution_base() {
    let source = SOURCE.replace(
        "<body>",
        "<body xml:base=\"https://wrong.test/\"><img srcset=\"café.svg?x=1&amp;y=2 1x, icon 2x\"/>",
    );
    let (mut host, evidence) = host(
        &source,
        "https://example.test/assets/",
        HtmlBaseCompatibilityProfile::StandardAgreement,
    );
    let cap = capability(false);
    let inventory = host
        .inspect_dependencies(&evidence, &cap, 500, slots, evidence.effective_url())
        .unwrap();
    assert_eq!(inventory.srcsets().len(), 1);
    let srcset = &inventory.srcsets()[0];
    assert!(Arc::ptr_eq(
        srcset.source.tree(),
        &inventory.document().tree
    ));
    assert_eq!(
        srcset.source.source_uri(),
        Some("studio://source/original.xhtml")
    );
    assert_eq!(
        srcset
            .resolved_urls
            .iter()
            .map(|u| u.as_ref().unwrap().as_str())
            .collect::<Vec<_>>(),
        [
            "https://example.test/assets/caf%C3%A9.svg?x=1&y=2",
            "https://example.test/assets/icon"
        ]
    );
    assert!(!inventory.blockers().iter().any(|b| matches!(
        b.as_str(),
        "srcset-analysis-required" | "srcset-base-and-policy-required"
    )));
    assert!(inventory
        .blockers()
        .iter()
        .any(|b| b == "srcset-selection-and-load-history-required"));
    assert!(host
        .validate_dependencies(&inventory, &cap, slots, evidence.effective_url())
        .is_ok());
    assert!(host
        .validate_dependencies(
            &inventory,
            &cap,
            |s| Ok(s.id + 100),
            evidence.effective_url()
        )
        .is_err());
}

#[test]
fn incomplete_srcset_scan_precedes_css_callbacks_and_invalid_base_stays_unresolved() {
    for (value, budget) in [
        ("a.svg 1x, b.svg 2x", 20),
        ("a.svg 999999999999999999w", 500),
    ] {
        let source = SOURCE.replace("<body>", &format!("<body><img srcset=\"{value}\"/>"));
        let (mut host, evidence) = host(
            &source,
            "https://example.test/assets/",
            HtmlBaseCompatibilityProfile::StandardAgreement,
        );
        let calls = Arc::new(AtomicUsize::new(0));
        let cap = CemModuleUrlResolutionCapability::new(
            Arc::new(ObservedResolver {
                delegate: capability(false),
                calls: calls.clone(),
                revision: Arc::new(AtomicUsize::new(1)),
                drift: false,
                cacheable: true,
            }),
            CemResolutionContextHandle::new("execution"),
        );
        assert!(host
            .inspect_dependencies(&evidence, &cap, budget, slots, evidence.effective_url())
            .is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
    let source = SOURCE
        .replace("../assets/", "http://[")
        .replace("<body>", "<body><img srcset=\"a.svg 1x\"/>");
    let (mut host, evidence) = host(
        &source,
        "about:blank",
        HtmlBaseCompatibilityProfile::Chromium148,
    );
    let inventory = host
        .inspect_dependencies(
            &evidence,
            &capability(false),
            500,
            slots,
            evidence.effective_url(),
        )
        .unwrap();
    assert!(inventory.srcsets()[0].resolved_urls[0].is_err());
    assert!(inventory
        .blockers()
        .iter()
        .any(|b| b == "srcset-resolution-incomplete"));
}

#[test]
fn policy_drift_after_publication_and_dom_drift_during_resolution_are_rejected() {
    let calls = Arc::new(AtomicUsize::new(0));
    let revision = Arc::new(AtomicUsize::new(1));
    let cap = CemModuleUrlResolutionCapability::new(
        Arc::new(ObservedResolver {
            delegate: capability(false),
            calls: calls.clone(),
            revision: revision.clone(),
            drift: false,
            cacheable: true,
        }),
        CemResolutionContextHandle::new("execution"),
    );
    let (mut host, evidence) = host(
        SOURCE,
        "https://example.test/assets/",
        HtmlBaseCompatibilityProfile::StandardAgreement,
    );
    let inventory = host
        .inspect_dependencies(&evidence, &cap, 100, slots, evidence.effective_url())
        .unwrap();
    revision.fetch_add(1, Ordering::SeqCst);
    assert!(host
        .validate_dependencies(&inventory, &cap, slots, evidence.effective_url())
        .is_err());
    calls.store(0, Ordering::SeqCst);
    assert!(host
        .inspect_dependencies(
            &evidence,
            &cap,
            100,
            |slot| Ok(slot.id
                + if calls.load(Ordering::SeqCst) == 0 {
                    1
                } else {
                    100
                }),
            evidence.effective_url()
        )
        .is_err());
    assert!(host.current().is_none());
}
