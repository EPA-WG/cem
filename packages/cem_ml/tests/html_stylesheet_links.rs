use cem_ml::{css_import_snapshot::*, scheduler::AbortSignal};
use std::sync::Arc;
#[path = "support/stylesheet_fixture.rs"]
mod fixture;
use fixture::*;
#[test]
fn external_occurrences_preserve_sources_conditions_and_final_bases() {
    let (mut host,document,cap)=setup("<link rel='alternate STYLESHEET' title='Night' media='screen' href='theme.css'/><link rel='stylesheet' disabled='' href='theme.css'/><link rel='preload' href='ignored.css'/>");
    assert_eq!(document.stylesheet_links().len(), 2);
    let snapshot = snapshot();
    let result = host
        .inspect_css_imports(
            document.clone(),
            &cap,
            snapshot.clone(),
            Default::default(),
            AbortSignal::new(),
            slots,
            BASE,
        )
        .unwrap();
    assert_eq!(result.external().len(), 2);
    assert!(result.closures().is_empty());
    let first = &result.external()[0];
    assert!(first.link.alternate);
    assert_eq!(first.link.title, "Night");
    assert_eq!(first.link.media, "screen");
    assert!(!first.link.source.frames.is_empty());
    assert!(result.external()[1].link.disabled);
    assert!(Arc::ptr_eq(
        &first.source,
        snapshot.source(&format!("{BASE}theme.css")).unwrap()
    ));
    assert_eq!(
        first.closure.closure().sheets()[0].url,
        "https://cdn.test/main.css"
    );
    assert_eq!(
        first.closure.closure().sheets()[1].resources.references[0]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://cdn.test/image.svg"
    );
    assert!(result
        .blockers()
        .iter()
        .any(|b| b == "stylesheet-execution-history-required"));
    assert!(!result
        .blockers()
        .iter()
        .any(|b| b == "external-stylesheet-closure-required"));
    host.validate_css_imports(&result, &cap, &snapshot, slots, BASE)
        .unwrap();
    assert!(host
        .validate_css_imports(&result, &cap, &fixture::snapshot(), slots, BASE)
        .is_err());
    host.invalidate().unwrap();
    assert!(host
        .validate_css_imports(&result, &cap, &snapshot, slots, BASE)
        .is_err());
}
#[test]
fn namespaces_and_rel_tokens_do_not_mint_link_roots() {
    let (_,document,_)=setup("<link xmlns='urn:foreign' rel='stylesheet' href='bad.css'/><link rel='stylesheetx' href='bad.css'/>");
    assert!(document.stylesheet_links().is_empty());
}
#[test]
fn unsupported_or_missing_responses_and_integrity_fail_closed() {
    for link in [
        "<link rel='stylesheet' href='missing.css'/>",
        "<link rel='stylesheet' href='denied.css'/>",
        "<link rel='stylesheet' href='theme.css' integrity='sha256-AAAA'/>",
        "<link rel='stylesheet' href='theme.css' type='text/plain'/>",
        "<link rel='alternate stylesheet' href='theme.css'/>",
        "<link rel='stylesheet'/>",
    ] {
        let (mut host, document, cap) = setup(link);
        assert!(
            host.inspect_css_imports(
                document,
                &cap,
                snapshot(),
                Default::default(),
                AbortSignal::new(),
                slots,
                BASE
            )
            .is_err(),
            "{link}"
        );
    }
}
#[test]
fn external_root_responses_and_imports_share_limits_with_inline_roots() {
    let (mut host,document,cap)=setup("<style>p{color:red}</style><link rel='stylesheet' href='theme.css'/><link rel='stylesheet' href='theme.css'/>");
    for mode in ["sheets", "bytes", "nodes"] {
        let mut limits = CssSnapshotCaptureLimits::default();
        if mode == "sheets" {
            limits.imports.max_sheets = 4;
        }
        if mode == "bytes" {
            limits.responses.max_total_bytes = 60;
        }
        if mode == "nodes" {
            limits.max_root_nodes = 1;
        }
        assert!(
            host.inspect_css_imports(
                document.clone(),
                &cap,
                snapshot(),
                limits,
                AbortSignal::new(),
                slots,
                BASE
            )
            .is_err(),
            "{mode}"
        );
    }
    let result = host
        .inspect_css_imports(
            document,
            &cap,
            snapshot(),
            Default::default(),
            AbortSignal::new(),
            slots,
            BASE,
        )
        .unwrap();
    assert_eq!(result.sheet_count(), 5);
    assert_eq!(
        result.received_bytes(),
        2 * (snapshot()
            .source(&format!("{BASE}theme.css"))
            .unwrap()
            .bytes
            .len()
            + snapshot()
                .source("https://cdn.test/child.css")
                .unwrap()
                .bytes
                .len())
    );
}
