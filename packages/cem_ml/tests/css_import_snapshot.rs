use cem_ml::{
    css_import_snapshot::*,
    css_imports::{CssImportLimits, CssImportResponsePolicy},
    css_resources::CssResourceKind,
    import::import_data,
    module_resolution::*,
    parser::tree::RetainedCemTree,
    scheduler::AbortSignal,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

fn tree(css: &str) -> Arc<RetainedCemTree> {
    import_data(css, "text/css", "cem", "urn:retained:inline-css").unwrap()
}
fn capability(mapping: Option<CemModuleUrlMapping>) -> CemModuleUrlResolutionCapability {
    let mut frame = CemModuleUrlFrame::new("test", "https://example.test/root.css");
    frame.allowed_schemes = Some(["https".into()].into());
    if let Some(mapping) = mapping {
        frame.specifiers.resources.insert("a.css".into(), mapping);
    }
    let handle = CemResolutionContextHandle::new("test");
    CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(
            handle.clone(),
            CemModuleUrlContext {
                identity: "test".into(),
                resolver_identity: "resolver".into(),
                resource_policy_stamp: "https-only".into(),
                frames: vec![frame],
            },
        )),
        handle,
    )
}
fn source(requested: &str, final_url: &str, css: &str) -> CssSnapshotSource {
    CssSnapshotSource {
        requested_url: format!("https://example.test/{requested}"),
        final_url: format!("https://example.test/{final_url}"),
        revision: "revision-1".into(),
        bytes: css.as_bytes().to_vec(),
        content_type: "text/css".into(),
    }
}
fn snapshot() -> Arc<CssSourceSnapshot> {
    CssSourceSnapshot::new(
        vec![
            source(
                "a.css",
                "cdn/a.css",
                "@import 'b.css'; a {background:url(icon.svg)}",
            ),
            source("cdn/b.css", "cdn/b.css", "b {color:red}"),
        ],
        Default::default(),
    )
    .unwrap()
}
fn capture(
    root: Arc<RetainedCemTree>,
    cap: &CemModuleUrlResolutionCapability,
    snapshot: Arc<CssSourceSnapshot>,
) -> Result<SnapshotCssImportClosure, cem_ml::css_imports::CssImportFailure> {
    capture_css_import_snapshot(
        root,
        "https://example.test/root.css",
        cap,
        snapshot,
        Default::default(),
        AbortSignal::new(),
    )
}

#[test]
fn snapshot_capture_retains_redirects_conditions_occurrences_and_acquisition_owners() {
    let source = "/* λ */\n@import 'a.css' layer(base) supports(display:grid) screen; @import 'a.css#same-response' layer;";
    let root = tree(source);
    let cap = capability(None);
    let snap = snapshot();
    let result = capture(root.clone(), &cap, snap.clone()).unwrap();
    assert_eq!(result.closure().sheets().len(), 5);
    assert_eq!(result.acquisitions().len(), 4);
    assert!(Arc::ptr_eq(
        &result.closure().sheets()[0].resources.tree,
        &root
    ));
    assert_eq!(
        result
            .closure()
            .edges()
            .iter()
            .map(|e| e.parent_sheet)
            .collect::<Vec<_>>(),
        [0, 1, 0, 3]
    );
    assert!(
        matches!(&result.closure().edges()[0].conditions, CssResourceKind::Import {layer: Some(l), supports: Some(s), media: Some(m)} if l == "base" && s == "display:grid" && m == "screen")
    );
    let r = &result.closure().sheets()[0].resources.references[0];
    assert_eq!(r.range.line, 2);
    assert!(r.source.origin().is_some());
    let resolved = result.closure().sheets()[1].resources.references[1]
        .resolution
        .as_ref()
        .unwrap();
    assert_eq!(resolved.resolved_url, "https://example.test/cdn/icon.svg");
    assert_eq!(resolved.resource_policy_stamp, "https-only");
    assert_eq!(result.acquisitions()[0].sheet, 1);
    assert_eq!(result.acquisitions()[0].source.revision, "revision-1");
    assert!(std::ptr::eq(
        result.acquisitions()[0].source.as_ref(),
        snap.source("https://example.test/a.css").unwrap().as_ref()
    ));
    assert!(result
        .validate_current(&root, "https://example.test/root.css", &cap, &snap)
        .is_ok());
    assert!(result
        .validate_current(&root, "https://example.test/root.css", &cap, &snapshot())
        .is_err());
    assert!(result
        .validate_current(&root, "https://example.test/other.css", &cap, &snap)
        .is_err());
    assert!(result
        .validate_current(
            &root,
            "https://example.test/root.css",
            &capability(None),
            &snap
        )
        .is_err());
    assert!(result
        .validate_current(&tree(source), "https://example.test/root.css", &cap, &snap)
        .is_err());
    assert!(result
        .blockers()
        .iter()
        .any(|b| b == "css-runtime-dependency-closure-required"));
}

#[test]
fn immutable_snapshot_rejects_conflicts_metadata_and_bounds() {
    let a = source("a.css", "cdn/a.css", "a {}");
    assert!(CssSourceSnapshot::new(vec![a.clone(), a.clone()], Default::default()).is_err());
    let mut alias = a.clone();
    alias.requested_url = "https://example.test/alias.css".into();
    assert!(CssSourceSnapshot::new(vec![a.clone(), alias.clone()], Default::default()).is_ok());
    alias.revision = "changed".into();
    assert!(CssSourceSnapshot::new(vec![a.clone(), alias], Default::default()).is_err());
    for field in ["relative", "revision", "fragment", "content-type"] {
        let mut bad = a.clone();
        match field {
            "relative" => bad.final_url = "relative.css".into(),
            "revision" => bad.revision.clear(),
            "fragment" => bad.requested_url.push_str("#part"),
            _ => bad.content_type.clear(),
        }
        assert!(CssSourceSnapshot::new(vec![bad], Default::default()).is_err());
    }
    assert!(CssSourceSnapshot::new(
        vec![a.clone()],
        CssSnapshotLimits {
            max_sources: 0,
            ..Default::default()
        }
    )
    .is_err());
    assert!(CssSourceSnapshot::new(
        vec![a],
        CssSnapshotLimits {
            max_bytes: 2,
            ..Default::default()
        }
    )
    .is_err());
}

#[test]
fn snapshot_capture_never_falls_back_and_keeps_native_failure_locations() {
    let cap = capability(None);
    for missing in ["missing.css", "file:///definitely/not/acquired.css"] {
        let err = capture(
            tree(&format!("/* λ */\n@import '{missing}';")),
            &cap,
            snapshot(),
        )
        .err()
        .unwrap();
        assert!(matches!(
            err.code.as_str(),
            "cem.css.snapshot_missing" | "cem.css.import_resolution_failed"
        ));
        assert_eq!(err.location.as_ref().unwrap().range.line, 2);
        assert!(err.source.origin().is_some());
    }
    let denied = capture(
        tree("@import 'a.css';"),
        &capability(Some(CemModuleUrlMapping::blocked())),
        snapshot(),
    )
    .err()
    .unwrap();
    assert_eq!(denied.code, "cem.css.import_resolution_failed");
    for (kind, expected) in [
        ("mime", "cem.css.import_content_type"),
        ("integrity", "cem.css.import_integrity"),
        ("redirect", "cem.css.import_redirect_denied"),
        ("cycle", "cem.css.import_cycle"),
        ("placement", "cem.css.import_placement_invalid"),
    ] {
        let mut a = source("a.css", "a.css", "a {}");
        let mut cap = capability(None);
        match kind {
            "mime" => a.content_type = "text/html".into(),
            "integrity" => {
                cap = capability(Some(
                    CemModuleUrlMapping::target("./a.css")
                        .with_integrity("sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="),
                ))
            }
            "redirect" => a.final_url = "http://example.test/a.css".into(),
            "cycle" => a.bytes = b"@import 'root.css';".to_vec(),
            _ => a.bytes = b"a {} @import 'b.css';".to_vec(),
        }
        let snap = CssSourceSnapshot::new(vec![a], Default::default()).unwrap();
        let err = capture(tree("@import 'a.css';"), &cap, snap).err().unwrap();
        assert_eq!(err.code, expected, "{kind}: {err}");
        assert!(err.location.is_some());
    }
}

#[test]
fn snapshot_capture_bounds_occurrences_bytes_root_work_and_cancellation() {
    let root = tree("@import 'a.css'; @import 'a.css';");
    let cap = capability(None);
    for limits in [
        CssSnapshotCaptureLimits {
            imports: CssImportLimits {
                max_sheets: 2,
                ..Default::default()
            },
            ..Default::default()
        },
        CssSnapshotCaptureLimits {
            imports: CssImportLimits {
                max_depth: 0,
                ..Default::default()
            },
            ..Default::default()
        },
        CssSnapshotCaptureLimits {
            responses: CssImportResponsePolicy {
                max_response_bytes: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        CssSnapshotCaptureLimits {
            responses: CssImportResponsePolicy {
                max_total_bytes: 50,
                ..Default::default()
            },
            ..Default::default()
        },
        CssSnapshotCaptureLimits {
            max_root_nodes: 1,
            ..Default::default()
        },
    ] {
        assert!(capture_css_import_snapshot(
            root.clone(),
            "https://example.test/root.css",
            &cap,
            snapshot(),
            limits,
            AbortSignal::new()
        )
        .is_err());
    }
    let abort = AbortSignal::new();
    let snap = snapshot();
    let result = capture_css_import_snapshot(
        root.clone(),
        "https://example.test/root.css",
        &cap,
        snap.clone(),
        Default::default(),
        abort.clone(),
    )
    .unwrap();
    abort.abort();
    assert!(result
        .validate_current(&root, "https://example.test/root.css", &cap, &snap)
        .is_err());
    assert!(capture_css_import_snapshot(
        root,
        "https://example.test/root.css",
        &cap,
        snap,
        Default::default(),
        abort
    )
    .is_err());
}

struct MutableResolver {
    delegate: CemModuleUrlResolutionCapability,
    revision: Arc<AtomicUsize>,
    calls: Arc<AtomicUsize>,
    drift: bool,
    cacheable: bool,
}
impl CemModuleUrlResolver for MutableResolver {
    fn context_cache_identity(&self, _: &CemResolutionContextHandle) -> Option<String> {
        self.cacheable
            .then(|| self.revision.load(Ordering::SeqCst).to_string())
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
fn incomplete_fingerprints_bounds_and_drift_cannot_publish_or_restore_authority() {
    for mode in ["uncacheable", "drift", "bounds", "later"] {
        let calls = Arc::new(AtomicUsize::new(0));
        let revision = Arc::new(AtomicUsize::new(1));
        let cap = CemModuleUrlResolutionCapability::new(
            Arc::new(MutableResolver {
                delegate: capability(None),
                revision: revision.clone(),
                calls: calls.clone(),
                drift: mode == "drift",
                cacheable: mode != "uncacheable",
            }),
            CemResolutionContextHandle::new("test"),
        );
        let root = tree("@import 'a.css';");
        let snap = snapshot();
        let limits = CssSnapshotCaptureLimits {
            max_root_nodes: if mode == "bounds" { 0 } else { 10_000 },
            ..Default::default()
        };
        let result = capture_css_import_snapshot(
            root.clone(),
            "https://example.test/root.css",
            &cap,
            snap.clone(),
            limits,
            AbortSignal::new(),
        );
        if mode == "later" {
            let result = result.unwrap();
            revision.store(2, Ordering::SeqCst);
            assert!(result
                .validate_current(&root, "https://example.test/root.css", &cap, &snap)
                .is_err());
            revision.store(1, Ordering::SeqCst);
            assert!(result
                .validate_current(&root, "https://example.test/root.css", &cap, &snap)
                .is_err());
        } else {
            assert!(result.is_err());
        }
        if matches!(mode, "bounds" | "uncacheable") {
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        }
    }
}

#[test]
fn complete_import_coverage_keeps_leaf_resource_denial_and_empty_snapshot_ownership() {
    let root = tree("a {background:url(a.css)}");
    let cap = capability(Some(CemModuleUrlMapping::blocked()));
    let snap = CssSourceSnapshot::new(vec![], Default::default()).unwrap();
    let result = capture(root.clone(), &cap, snap.clone()).unwrap();
    assert!(result.acquisitions().is_empty());
    assert!(result.closure().sheets()[0].resources.references[0]
        .resolution
        .is_err());
    assert!(result
        .blockers()
        .iter()
        .any(|b| b == "css-resource-resolution-incomplete"));
    assert!(result
        .validate_current(&root, "https://example.test/root.css", &cap, &snap)
        .is_ok());
    let replacement = CssSourceSnapshot::new(vec![], Default::default()).unwrap();
    assert!(result
        .validate_current(&root, "https://example.test/root.css", &cap, &replacement)
        .is_err());
}
