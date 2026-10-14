use cem_ml::{browser_stylesheet_sources::browser_stylesheet_sources, scheduler::AbortSignal};
use std::sync::Arc;
#[path = "support/stylesheet_fixture.rs"]
mod fixture;
use fixture::*;

#[test]
fn probes_retain_inline_attributes_external_sources_and_import_order() {
    let (mut html, document, cap) = setup("<style>p{color:red}</style><link rel='stylesheet' href='theme.css'/><span style='color:blue' title='Helpful label'/>");
    let snapshot = snapshot();
    let css = Arc::new(
        html.inspect_css_imports(
            document,
            &cap,
            snapshot,
            Default::default(),
            AbortSignal::new(),
            slots,
            BASE,
        )
        .unwrap(),
    );
    let probes = browser_stylesheet_sources(&css, 10, 10000).unwrap();
    assert_eq!(probes.len(), 4);
    assert_eq!(probes[0].text, "p{color:red}");
    assert!(!probes[0].attribute);
    assert_eq!(probes[1].text, "color:blue");
    assert!(probes[1].attribute);
    assert_eq!(probes[2].url, "https://cdn.test/main.css");
    assert!(probes[2].text.starts_with("@import"));
    assert_eq!(probes[3].parent_sheet, Some(0));
    assert_eq!(probes[3].import_index, Some(0));
    assert_eq!(probes[3].text, "p{background:url(./image.svg)}");
    assert!(browser_stylesheet_sources(&css, 3, 10000).is_err());
    assert!(browser_stylesheet_sources(&css, 10, 1).is_err());
}

#[test]
fn browser_profile_retains_titled_selection_for_explicit_host_control() {
    for tag in [
        "<style title='preferred'>p{color:red}</style>",
        "<link rel='alternate stylesheet' title='alternate' href='theme.css'/>",
    ] {
        let (mut html, document, cap) = setup(tag);
        let css = html
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
        assert!(!browser_stylesheet_sources(&css, 10, 10000)
            .unwrap()
            .is_empty());
        assert!(!css.execution_roots()[0].title.is_empty());
    }
}

#[test]
fn repeated_imports_keep_distinct_occurrences_and_source_order() {
    use cem_ml::css_import_snapshot::*;
    let (mut html, document, cap) =
        setup("<style>@import 'same.css' screen; @import 'same.css' print;</style>");
    let snapshot = CssSourceSnapshot::new(
        vec![CssSnapshotSource {
            requested_url: format!("{BASE}same.css"),
            final_url: format!("{BASE}same.css"),
            revision: "v1".into(),
            content_type: "text/css".into(),
            bytes: b"p{color:red}".to_vec(),
        }],
        Default::default(),
    )
    .unwrap();
    let css = html
        .inspect_css_imports(
            document,
            &cap,
            snapshot,
            Default::default(),
            AbortSignal::new(),
            slots,
            BASE,
        )
        .unwrap();
    let probes = browser_stylesheet_sources(&css, 3, 10000).unwrap();
    assert_eq!(
        probes.iter().map(|p| p.import_index).collect::<Vec<_>>(),
        [None, Some(0), Some(1)]
    );
    assert_ne!(probes[1].sheet, probes[2].sheet);
    assert_eq!(probes[1].text, probes[2].text);
}

#[test]
fn legacy_encoding_cannot_be_reinterpreted_as_utf8_observation_text() {
    use cem_ml::css_import_snapshot::*;
    let (mut html, document, cap) = setup("<link rel='stylesheet' href='legacy.css'/>");
    let snapshot = CssSourceSnapshot::new(
        vec![CssSnapshotSource {
            requested_url: format!("{BASE}legacy.css"),
            final_url: format!("{BASE}legacy.css"),
            revision: "v1".into(),
            content_type: "text/css; charset=windows-1252".into(),
            bytes: b"p{color:red}".to_vec(),
        }],
        Default::default(),
    )
    .unwrap();
    let css = html
        .inspect_css_imports(
            document,
            &cap,
            snapshot,
            Default::default(),
            AbortSignal::new(),
            slots,
            BASE,
        )
        .unwrap();
    assert!(browser_stylesheet_sources(&css, 3, 10000)
        .unwrap_err()
        .contains("UTF-8"));
}
