use cem_ml::{
    css_imports::{CssImportClosure, CssImportLimits, CssImportState},
    import::import_data,
    module_resolution::{
        CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlResolutionCapability,
        CemResolutionContextHandle, CemScopedModuleUrlResolver,
    },
    parser::tree::RetainedCemTree,
    scheduler::AbortSignal,
};
use std::sync::Arc;
fn tree(css: &str) -> Arc<RetainedCemTree> {
    import_data(css, "text/css", "cem", "urn:fixture:css").unwrap()
}
fn closure(css: &str, limits: CssImportLimits, abort: AbortSignal) -> CssImportClosure {
    let mut frame = CemModuleUrlFrame::new("root", "https://example.test/main.css");
    frame.allowed_schemes = Some(["https".into()].into());
    let context = CemResolutionContextHandle::new("test");
    let capability = CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(
            context.clone(),
            CemModuleUrlContext {
                identity: "test".into(),
                resolver_identity: "fixture".into(),
                resource_policy_stamp: "https".into(),
                frames: vec![frame],
            },
        )),
        context,
    );
    CssImportClosure::new(
        tree(css),
        "https://example.test/main.css",
        capability,
        limits,
        abort,
    )
    .unwrap()
}
#[test]
fn css_import_closure_retains_order_conditions_and_final_sheet_bases() {
    let mut c = closure(
        "@import 'a.css' layer(base) screen; @import 'a.css' layer;",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    assert_eq!(c.state(), CssImportState::Pending);
    let a = c.next_import().unwrap().unwrap();
    assert_eq!(a.resolution.resolved_url, "https://example.test/a.css");
    assert!(c.next_import().is_err()); // At most one delivery can be outstanding.
    let shared = tree("@import 'b.css'; a {background:url(icon.svg)}");
    c.complete_import(a.id, shared.clone(), "https://example.test/cdn/a.css")
        .unwrap();
    let b = c.next_import().unwrap().unwrap();
    assert_eq!(b.resolution.resolved_url, "https://example.test/cdn/b.css");
    c.complete_import(b.id, tree("b { color:red }"), &b.resolution.resolved_url)
        .unwrap();
    let again = c.next_import().unwrap().unwrap();
    assert_eq!(again.resolution.resolved_url, a.resolution.resolved_url);
    c.complete_import(again.id, shared.clone(), "https://example.test/cdn/a.css")
        .unwrap();
    let b = c.next_import().unwrap().unwrap();
    c.complete_import(b.id, tree("b {}"), &b.resolution.resolved_url)
        .unwrap();
    assert!(c.next_import().unwrap().is_none());
    assert_eq!(c.state(), CssImportState::Ready);
    let sheets = c.sheets();
    assert_eq!(sheets.len(), 5);
    assert!(Arc::ptr_eq(&sheets[1].resources.tree, &shared));
    assert!(Arc::ptr_eq(&sheets[3].resources.tree, &shared));
    assert_eq!(
        sheets[1].resources.references[1]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://example.test/cdn/icon.svg"
    );
    assert_eq!(
        c.edges().iter().map(|e| e.parent_sheet).collect::<Vec<_>>(),
        [0, 1, 0, 3]
    );
    assert!(
        matches!(&c.edges()[0].conditions, cem_ml::css_resources::CssResourceKind::Import {layer:Some(layer),media:Some(media),..} if layer == "base" && media == "screen")
    );
}
#[test]
fn css_import_closure_rejects_cycles_and_denied_redirects() {
    for url in ["https://example.test/main.css", "http://example.test/a.css"] {
        let mut c = closure(
            "@import 'a.css';",
            CssImportLimits::default(),
            AbortSignal::new(),
        );
        let request = c.next_import().unwrap().unwrap();
        assert!(c.complete_import(request.id, tree("a {}"), url).is_err());
        assert_eq!(c.state(), CssImportState::Failed);
        assert!(c.next_import().is_err());
    }
    let mut c = closure(
        "@import 'main.css#again';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    assert!(c.next_import().is_err());
    assert_eq!(c.state(), CssImportState::Failed);
}
#[test]
fn css_import_closure_bounds_cancellation_and_loader_failures() {
    for limits in [
        CssImportLimits {
            max_sheets: 1,
            ..Default::default()
        },
        CssImportLimits {
            max_depth: 0,
            ..Default::default()
        },
    ] {
        let mut c = closure("@import 'a.css';", limits, AbortSignal::new());
        assert!(c.next_import().is_err());
        assert_eq!(c.state(), CssImportState::Failed);
    }
    let abort = AbortSignal::new();
    let mut c = closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        abort.clone(),
    );
    let r = c.next_import().unwrap().unwrap();
    abort.abort();
    assert_eq!(c.state(), CssImportState::Failed);
    assert_eq!(c.failure().unwrap().code, "cem.css.import_cancelled");
    assert!(c
        .complete_import(r.id, tree("a {}"), &r.resolution.resolved_url)
        .is_err());
    assert_eq!(c.state(), CssImportState::Failed);
    let mut c = closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    let r = c.next_import().unwrap().unwrap();
    c.fail_import(r.id, "loader.mime_mismatch", "expected CSS")
        .unwrap();
    assert_eq!(c.state(), CssImportState::Failed);
    assert_eq!(c.failure().unwrap().code, "loader.mime_mismatch");
}

#[test]
fn css_import_closure_rejects_wrong_deliveries_without_losing_the_pending_request() {
    let mut c = closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    let r = c.next_import().unwrap().unwrap();
    assert!(c
        .complete_import(r.id + 1, tree("a {}"), &r.resolution.resolved_url)
        .is_err());
    assert!(c.fail_import(r.id + 1, "bad", "wrong delivery").is_err());
    assert_eq!(c.state(), CssImportState::Pending);
    assert_eq!(c.sheets().len(), 1);
    c.complete_import(r.id, tree("a {}"), &r.resolution.resolved_url)
        .unwrap();
    assert_eq!(c.state(), CssImportState::Ready);
    assert!(c
        .complete_import(r.id, tree("a {}"), &r.resolution.resolved_url)
        .is_err());
    assert_eq!(c.sheets().len(), 2);
}

#[test]
fn css_import_closure_rejects_non_css_and_bounds_recursive_imports() {
    let mut c = closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    let r = c.next_import().unwrap().unwrap();
    let xml = import_data("<a/>", "application/xml", "cem", "urn:xml").unwrap();
    assert!(c
        .complete_import(r.id, xml, &r.resolution.resolved_url)
        .is_err());
    assert_eq!(c.failure().unwrap().code, "cem.css.import_tree_invalid");
    assert!(c.failure().unwrap().source.origin().is_some());
    assert_eq!(c.sheets().len(), 1);

    let mut wrong_mode = closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    let r = wrong_mode.next_import().unwrap().unwrap();
    let declarations = cem_ml::import::import_data_bytes(
        b"color:red",
        "text/css; mode=declaration-list",
        "cem",
        "urn:css:declarations",
    )
    .unwrap();
    assert_eq!(
        wrong_mode
            .complete_import(r.id, declarations, &r.resolution.resolved_url)
            .unwrap_err()
            .code,
        "cem.css.import_tree_invalid"
    );

    for limits in [
        CssImportLimits {
            max_depth: 1,
            ..Default::default()
        },
        CssImportLimits {
            max_sheets: 2,
            ..Default::default()
        },
    ] {
        let mut c = closure("@import 'a.css';", limits, AbortSignal::new());
        let r = c.next_import().unwrap().unwrap();
        c.complete_import(r.id, tree("@import 'b.css';"), &r.resolution.resolved_url)
            .unwrap();
        assert_eq!(c.next_import().unwrap_err().code, "cem.css.import_limit");
        assert_eq!(c.sheets().len(), 2);
    }
    let mut c = closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    let r = c.next_import().unwrap().unwrap();
    c.complete_import(
        r.id,
        tree("@import 'a.css';"),
        "https://example.test/redirect.css",
    )
    .unwrap();
    assert_eq!(c.next_import().unwrap_err().code, "cem.css.import_cycle");
}
