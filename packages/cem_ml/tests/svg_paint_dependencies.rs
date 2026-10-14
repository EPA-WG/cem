use cem_ml::svg_paint_dependencies::{PaintTarget, SvgPaintPlan};
#[path = "support/stylesheet_fixture.rs"]
mod fixture;
fn plan(body: &str) -> SvgPaintPlan {
    let (_, inventory, _) = fixture::setup(body);
    SvgPaintPlan::from_inventory(inventory, 100_000).unwrap()
}
#[test]
fn local_css_fragment_keeps_source_fallback_and_target_despite_html_base() {
    let p = plan("<svg xmlns='http://www.w3.org/2000/svg'><defs><linearGradient id='paint'><stop stop-color='red'/></linearGradient></defs><path fill='url(#paint) blue'/></svg>");
    assert_eq!(p.roots().len(), 1);
    assert!(matches!(p.roots()[0].target, PaintTarget::Local(_)));
    assert_eq!(p.roots()[0].source.value(), "url(#paint) blue");
    assert!(std::sync::Arc::ptr_eq(
        p.roots()[0].source.document(),
        &p.inventory().document().tree
    ));
    assert!(p
        .blockers()
        .contains(&"svg-paint-rendering-evidence-required"));
}
#[test]
fn duplicates_missing_wrong_kind_and_encoded_fragments_are_explicit() {
    let p = plan("<svg xmlns='http://www.w3.org/2000/svg'><linearGradient id='dup'/><radialGradient id='dup'/><path id='shape'/><path fill='url(#dup)' stroke='url(#missing)'/><path fill='url(#shape)' stroke='url(#%70aint)'/></svg>");
    assert!(matches!(p.roots()[0].target, PaintTarget::Ambiguous));
    assert!(matches!(p.roots()[1].target, PaintTarget::Missing));
    assert!(matches!(p.roots()[2].target, PaintTarget::WrongKind(_)));
    assert!(matches!(
        p.roots()[3].target,
        PaintTarget::UnsupportedFragment
    ));
}
#[test]
fn template_links_require_explicit_base_agreement_and_href_wins_over_xlink() {
    let p = plan("<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'><linearGradient id='a' href='#b' xlink:href='external.svg#ignored'/><radialGradient id='b'/><path fill='url(#a)'/></svg>");
    let links = &p.roots()[0].links;
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].authored_uri, "#b");
    assert!(matches!(links[0].target, PaintTarget::BaseContextRequired));
}
#[test]
fn nonfragment_urls_are_retained_without_fetch_or_local_alias_guessing() {
    let p = plan("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a' href='other.svg#b'/><path fill='url(#a)' stroke='url(other.svg#p) red'/></svg>");
    assert!(matches!(
        p.roots()[0].links[0].target,
        PaintTarget::UntraversedUrl
    ));
    assert_eq!(p.roots()[0].links[0].authored_uri, "other.svg#b");
    assert!(matches!(p.roots()[1].target, PaintTarget::UntraversedUrl));
}
#[test]
fn only_fill_and_stroke_are_paint_roots_and_budget_failure_is_atomic() {
    let (_, inventory, _) = fixture::setup(
        "<svg xmlns='http://www.w3.org/2000/svg'><path fill='none' filter='url(#f)'/></svg>",
    );
    assert!(SvgPaintPlan::from_inventory(inventory.clone(), 1).is_err());
    assert!(SvgPaintPlan::from_inventory(inventory, 100_000)
        .unwrap()
        .roots()
        .is_empty());
}

fn same_base(body: &str) -> SvgPaintPlan {
    let url = "https://page.test/docs/page.xhtml";
    let (_, inventory, _) = fixture::setup_with_base(body, url, url);
    SvgPaintPlan::from_inventory(inventory, 100_000).unwrap()
}
#[test]
fn gradient_template_chains_allow_both_gradient_types_and_terminate_cycles() {
    let p = same_base("<svg xmlns='http://www.w3.org/2000/svg'><linearGradient id='a' href='#b'/><radialGradient id='b' href='#a'/><path fill='url(#a)' stroke='url(#a)'/></svg>");
    for root in p.roots() {
        assert_eq!(root.links.len(), 2);
        assert!(matches!(root.links[0].target, PaintTarget::Local(_)));
        assert!(matches!(root.links[1].target, PaintTarget::Cycle(_)));
    }
}
#[test]
fn patterns_only_template_patterns_and_legacy_href_retains_attribute_owner() {
    let p = same_base("<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'><pattern id='a' xlink:href='#b'/><pattern id='b' href='#g'/><linearGradient id='g'/><path fill='url(#a)'/></svg>");
    let links = &p.roots()[0].links;
    assert_eq!(links.len(), 2);
    assert!(matches!(links[0].target, PaintTarget::Local(_)));
    assert!(matches!(links[1].target, PaintTarget::WrongKind(_)));
    let attr = p
        .inventory()
        .document()
        .tree
        .node(links[0].attribute)
        .unwrap();
    assert_eq!(
        attr.name.as_ref().unwrap().namespace_uri,
        "http://www.w3.org/1999/xlink"
    );
}
#[test]
fn id_lookup_is_document_wide_and_does_not_hide_foreign_collisions() {
    let p = plan("<div id='paint'/><svg xmlns='http://www.w3.org/2000/svg'><linearGradient id='paint'/><path fill='url(#paint)'/></svg>");
    assert!(matches!(p.roots()[0].target, PaintTarget::Ambiguous));
}

#[test]
fn depth_and_root_limits_fail_without_publishing_partial_plans() {
    let mut body = String::from("<svg xmlns='http://www.w3.org/2000/svg'>");
    for i in 0..66 {
        body.push_str(&format!("<linearGradient id='g{i}' href='#g{}'/>", i + 1));
    }
    body.push_str("<path fill='url(#g0)'/></svg>");
    let url = "https://page.test/docs/page.xhtml";
    let (_, inventory, _) = fixture::setup_with_base(&body, url, url);
    assert!(SvgPaintPlan::from_inventory(inventory, 100_000)
        .err()
        .unwrap()
        .contains("depth"));
    let body = format!(
        "<svg xmlns='http://www.w3.org/2000/svg'><pattern id='p'/>{}</svg>",
        "<path fill='url(#p)'/>".repeat(257)
    );
    let (_, inventory, _) = fixture::setup(&body);
    assert!(SvgPaintPlan::from_inventory(inventory, 100_000)
        .err()
        .unwrap()
        .contains("root budget"));
}
#[test]
fn empty_modern_href_does_not_fall_back_to_legacy_link() {
    let p = same_base("<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'><pattern id='a' href='' xlink:href='#a'/><path fill='url(#a)'/></svg>");
    assert_eq!(p.roots()[0].links[0].authored_uri, "");
    assert!(matches!(
        p.roots()[0].links[0].target,
        PaintTarget::UntraversedUrl
    ));
}
#[test]
fn original_inventory_lifetime_remains_required_and_pattern_content_is_not_closed() {
    let (mut host, inventory, cap) = fixture::setup("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='p'><image href='image.png'/></pattern><path fill='url(#p)'/></svg>");
    let p = SvgPaintPlan::from_inventory(inventory, 100_000).unwrap();
    assert!(matches!(p.roots()[0].target, PaintTarget::Local(_)));
    assert!(p
        .blockers()
        .contains(&"svg-paint-content-dependencies-required"));
    host.validate_dependencies(p.inventory(), &cap, fixture::slots, fixture::BASE)
        .unwrap();
    host.invalidate().unwrap();
    assert!(host
        .validate_dependencies(p.inventory(), &cap, fixture::slots, fixture::BASE)
        .is_err());
}
