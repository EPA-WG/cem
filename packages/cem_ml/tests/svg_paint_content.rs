use cem_ml::{svg_paint_content::SvgPaintContentPlan, svg_paint_dependencies::SvgPaintPlan};
use std::sync::Arc;
#[path = "support/stylesheet_fixture.rs"]
mod fixture;
fn paint(body: &str) -> Arc<SvgPaintPlan> {
    let url = "https://page.test/docs/page.xhtml";
    let (_, inventory, _) = fixture::setup_with_base(body, url, url);
    Arc::new(SvgPaintPlan::from_inventory(inventory, 100_000).unwrap())
}
#[test]
fn stops_retain_source_ancestry_not_the_paint_consumers_ancestry() {
    let p = paint("<svg xmlns='http://www.w3.org/2000/svg'><defs color='red'><linearGradient id='a'><stop stop-color='currentColor'/></linearGradient></defs><g color='blue'><path fill='url(#a)'/></g></svg>");
    let content = SvgPaintContentPlan::new(p.clone(), 100_000).unwrap();
    assert!(Arc::ptr_eq(content.paint(), &p));
    let server = &content.servers()[0];
    assert_eq!(server.direct_stops.len(), 1);
    let stop = server
        .nodes
        .iter()
        .find(|n| n.node == server.direct_stops[0])
        .unwrap();
    let tree = &p.inventory().document().tree;
    assert_eq!(stop.source_ancestors[0], server.owner);
    assert_eq!(
        tree.node(stop.source_ancestors[1])
            .unwrap()
            .name
            .as_ref()
            .unwrap()
            .local_name,
        "defs"
    );
    assert!(!stop.source_ancestors.contains(&p.roots()[0].source.owner()));
    let consumer_parent = tree
        .node(p.roots()[0].source.owner())
        .unwrap()
        .parent
        .unwrap();
    assert!(!stop.source_ancestors.contains(&consumer_parent));
    assert!(!tree.node(stop.node).unwrap().source.frames.is_empty());
    assert!(content
        .blockers()
        .contains(&"svg-paint-clone-style-provenance-required"));
}
#[test]
fn own_and_template_content_remain_separate_without_selecting_a_rendered_source() {
    let p = paint("<svg xmlns='http://www.w3.org/2000/svg'><linearGradient id='a' href='#b'><stop stop-color='red'/></linearGradient><radialGradient id='b' href='#a'><stop stop-color='blue'/></radialGradient><path fill='url(#a)' stroke='url(#a)'/></svg>");
    let c = SvgPaintContentPlan::new(p, 100_000).unwrap();
    assert_eq!(c.servers().len(), 2);
    assert!(c.servers().iter().all(|s| s.direct_stops.len() == 1));
    assert!(c
        .blockers()
        .contains(&"svg-paint-content-evaluation-required"));
}
#[test]
fn pattern_descendants_and_direct_uri_owners_remain_native() {
    let p = paint("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='p'><title>Example</title><g style='color:red'><image href='asset.png'/><path fill='url(#missing)'/></g></pattern><path fill='url(#p)'/></svg>");
    let c = SvgPaintContentPlan::new(p.clone(), 100_000).unwrap();
    let s = &c.servers()[0];
    assert!(s.direct_stops.is_empty());
    let tree = &p.inventory().document().tree;
    assert!(s
        .nodes
        .iter()
        .any(|n| tree.node(n.node).unwrap().value == "Example"));
    assert_eq!(s.reference_attributes.len(), 1);
    assert_eq!(
        tree.node(s.reference_attributes[0]).unwrap().value,
        "asset.png"
    );
    assert!(c
        .blockers()
        .contains(&"svg-computed-resource-acquisition-required"));
}
#[test]
fn unknown_targets_produce_no_invented_content() {
    let c = SvgPaintContentPlan::new(
        paint("<svg xmlns='http://www.w3.org/2000/svg'><path fill='url(#missing)'/></svg>"),
        100_000,
    )
    .unwrap();
    assert!(c.servers().is_empty());
}
#[test]
fn only_direct_svg_stop_children_are_gradient_stops() {
    let c = SvgPaintContentPlan::new(paint("<svg xmlns='http://www.w3.org/2000/svg'><linearGradient id='a'><g><stop/></g><stop/><stop xmlns='urn:other'/></linearGradient><path fill='url(#a)'/></svg>"), 100_000).unwrap();
    assert_eq!(c.servers()[0].direct_stops.len(), 1);
    assert!(c.servers()[0].nodes.len() >= 5);
}
#[test]
fn content_budget_exhaustion_is_atomic() {
    let p = paint("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='p'><g><path/></g></pattern><path fill='url(#p)'/></svg>");
    assert!(SvgPaintContentPlan::new(p.clone(), 1).is_err());
    assert_eq!(
        SvgPaintContentPlan::new(p, 100_000)
            .unwrap()
            .servers()
            .len(),
        1
    );
}

#[test]
fn deferred_template_links_keep_only_authored_local_content() {
    let p = paint("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a' href='external.svg#p'><title>Local</title></pattern><path fill='url(#a)'/></svg>");
    let c = SvgPaintContentPlan::new(p.clone(), 100_000).unwrap();
    assert_eq!(c.servers().len(), 1);
    assert_eq!(c.servers()[0].reference_attributes.len(), 1);
    let attribute = c.servers()[0].reference_attributes[0];
    assert_eq!(
        p.inventory().document().tree.node(attribute).unwrap().value,
        "external.svg#p"
    );
    assert!(c
        .blockers()
        .contains(&"svg-paint-content-evaluation-required"));
}
#[test]
fn content_preserves_original_inventory_validation_requirement() {
    let (mut host, inventory, cap) = fixture::setup("<svg xmlns='http://www.w3.org/2000/svg'><linearGradient id='a'><stop/></linearGradient><path fill='url(#a)'/></svg>");
    let c = SvgPaintContentPlan::new(
        Arc::new(SvgPaintPlan::from_inventory(inventory, 100_000).unwrap()),
        100_000,
    )
    .unwrap();
    host.validate_dependencies(c.paint().inventory(), &cap, fixture::slots, fixture::BASE)
        .unwrap();
    host.invalidate().unwrap();
    assert!(host
        .validate_dependencies(c.paint().inventory(), &cap, fixture::slots, fixture::BASE)
        .is_err());
}
