use cem_ml::{
    import::svg_presentation::RetainedSvgPresentationAttribute, scheduler::AbortSignal,
    svg_replay::SvgReplayPlan,
};
use std::sync::Arc;
#[path = "support/stylesheet_fixture.rs"]
mod fixture;
const DOCUMENT: &str = "https://page.test/docs/page.xhtml";
const NONCE: &str = "0123456789abcdef0123456789abcdef";
fn sources(value: &str) -> Vec<RetainedSvgPresentationAttribute> {
    let (_, inventory, _) = fixture::setup(&format!(
        "<svg xmlns='http://www.w3.org/2000/svg'><path fill='{value}' stroke='{value}'/></svg>"
    ));
    inventory
        .svg_presentations()
        .iter()
        .map(|p| p.source.clone())
        .collect()
}
fn plan(
    sources: &[RetainedSvgPresentationAttribute],
    cancel: AbortSignal,
) -> Result<SvgReplayPlan, String> {
    SvgReplayPlan::new(sources, fixture::BASE, DOCUMENT, NONCE, cancel)
}
#[test]
fn replay_retains_occurrence_source_css_and_fragment_provenance() {
    let sources = sources("url(../paint.svg#p%20q)");
    let mut replay = plan(&sources, AbortSignal::new()).unwrap();
    assert_eq!(replay.routes().len(), 2);
    for (i, route) in replay.routes().iter().enumerate() {
        assert!(Arc::ptr_eq(
            route.consumer.document(),
            sources[i].document()
        ));
        assert!(Arc::ptr_eq(route.consumer.css(), sources[i].css()));
        assert_eq!(route.consumer.owner(), sources[i].owner());
        assert_eq!(route.consumer.attribute(), sources[i].attribute());
        assert_eq!(route.source_url, "https://page.test/paint.svg#p%20q");
        assert_eq!(route.request_url, "https://page.test/paint.svg");
        assert_eq!(route.fragment, "p%20q");
        assert_eq!(
            route.alias_request_url,
            format!("https://page.test/__cem_svg_delivery/{NONCE}/{i}")
        );
        assert_eq!(
            route.alias_url,
            format!("https://page.test/__cem_svg_delivery/{NONCE}/{i}#p%20q")
        );
        assert_eq!(route.replacement, format!("url(\"{}\")", route.alias_url));
        let node = route.consumer.css().node(route.reference).unwrap();
        assert!(node.range.length > 0);
        assert!(!node.source.frames.is_empty());
        assert_eq!(route.consumer.value(), "url(../paint.svg#p%20q)");
    }
    assert_ne!(replay.routes()[0].alias_url, replay.routes()[1].alias_url);
    assert!(replay
        .blockers()
        .contains(&"svg-computed-winning-source-required"));
    assert!(replay
        .blockers()
        .contains(&"svg-resource-consumption-required"));
    replay.validate(&sources).unwrap();
}
#[test]
fn unsupported_values_and_url_authority_fail_closed() {
    for value in [
        "red",
        "url(#p)",
        "url(&quot; #p&quot;)",
        "url(other.svg)",
        "url(other.svg#)",
        "url(other.svg#p) red",
        "url(https://else.test/a.svg#p)",
        "url(data:image/svg+xml,abc#p)",
        "url(https://user@page.test/a.svg#p)",
        "url(/docs/page.xhtml#p)",
        "url(/__cem_svg_delivery/old/0#p)",
    ] {
        assert!(
            plan(&sources(value), AbortSignal::new()).is_err(),
            "{value}"
        );
    }
}
#[test]
fn aliases_require_bounded_context_and_unique_native_occurrences() {
    let sources = sources("url(a.svg#p)");
    for nonce in ["", "UPPERCASE0123456789abcdef01234567", "a/b"] {
        assert!(
            SvgReplayPlan::new(&sources, fixture::BASE, DOCUMENT, nonce, Default::default())
                .is_err()
        );
    }
    assert!(plan(&[], Default::default()).is_err());
    assert!(plan(
        &self::sources(&format!("url(a.svg#{})", "p".repeat(4050))),
        Default::default()
    )
    .is_err());
    assert!(plan(&vec![sources[0].clone(); 257], Default::default()).is_err());
    assert!(plan(
        &[sources[0].clone(), sources[0].clone()],
        Default::default()
    )
    .is_err());
    assert!(SvgReplayPlan::new(
        &sources,
        "file:///tmp/",
        DOCUMENT,
        NONCE,
        Default::default()
    )
    .is_err());
    assert!(SvgReplayPlan::new(
        &sources,
        fixture::BASE,
        "https://user@page.test/a",
        NONCE,
        Default::default()
    )
    .is_err());
}
#[test]
fn cancellation_foreign_owner_and_reordering_revoke_plan() {
    for mode in ["abort", "foreign", "reorder", "close"] {
        let original = sources("url(a.svg#p)");
        let cancel = AbortSignal::new();
        let mut replay = plan(&original, cancel.clone()).unwrap();
        let mut current = original.clone();
        match mode {
            "abort" => cancel.abort(),
            "foreign" => current = sources("url(a.svg#p)"),
            "reorder" => current.reverse(),
            "close" => replay.close(),
            _ => unreachable!(),
        }
        assert!(replay.validate(&current).is_err(), "{mode}");
        assert!(replay.routes().is_empty());
        assert!(replay.validate(&original).is_err());
    }
    let cancel = AbortSignal::new();
    cancel.abort();
    assert!(plan(&sources("url(a.svg#p)"), cancel).is_err());
}
#[test]
fn css_escaped_urls_use_retained_import_and_nonce_separates_plans() {
    let sources = sources("url(\\61.svg#p)");
    let first = plan(&sources, Default::default()).unwrap();
    assert_eq!(
        first.routes()[0].source_url,
        "https://page.test/assets/a.svg#p"
    );
    let second = SvgReplayPlan::new(
        &sources,
        fixture::BASE,
        DOCUMENT,
        "ffffffffffffffffffffffffffffffff",
        Default::default(),
    )
    .unwrap();
    assert_ne!(first.routes()[0].alias_url, second.routes()[0].alias_url);
}

#[test]
fn replacements_roundtrip_opaque_fragments_and_reject_mixed_documents() {
    let original = sources(r#"url(&quot;../paint.svg#p\\q)&quot;)"#);
    let replay = plan(&original, Default::default()).unwrap();
    let route = &replay.routes()[0];
    assert_eq!(route.fragment, r#"p\q)"#);
    let markup = format!(
        "<svg xmlns='http://www.w3.org/2000/svg'><path fill='{}'/></svg>",
        route.replacement
    );
    let (_, inventory, _) = fixture::setup(&markup);
    let refs = &inventory.svg_presentations()[0].resources.references;
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].authored_specifier, route.alias_url);
    let foreign = sources("url(a.svg#p)");
    assert!(plan(
        &[original[0].clone(), foreign[1].clone()],
        Default::default()
    )
    .is_err());
}

#[test]
fn per_element_xml_base_cannot_be_flattened_into_document_base() {
    let (_, inventory, _) = fixture::setup("<svg xmlns='http://www.w3.org/2000/svg' xml:base='elsewhere/'><path fill='url(a.svg#p)'/></svg>");
    let sources: Vec<_> = inventory
        .svg_presentations()
        .iter()
        .map(|p| p.source.clone())
        .collect();
    assert!(plan(&sources, Default::default()).is_err());
}
