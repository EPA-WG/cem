use cem_ml::{
    svg_paint_content::SvgPaintContentPlan,
    svg_paint_dependencies::{PaintTarget, SvgPaintPlan},
    svg_paint_style::{PaintContentSelection, PaintInstanceParent, SvgPaintStylePlan},
};
use std::sync::Arc;
#[path = "support/stylesheet_fixture.rs"]
mod fixture;
fn content(body: &str) -> Arc<SvgPaintContentPlan> {
    let url = "https://page.test/docs/page.xhtml";
    let (_, inventory, _) = fixture::setup_with_base(body, url, url);
    Arc::new(
        SvgPaintContentPlan::new(
            Arc::new(SvgPaintPlan::from_inventory(inventory, 100_000).unwrap()),
            100_000,
        )
        .unwrap(),
    )
}
fn id(c: &SvgPaintContentPlan, name: &str) -> cem_ml::parser::AstNodeId {
    let tree = &c.paint().inventory().document().tree;
    c.servers()
        .iter()
        .find(|s| {
            tree.node(s.owner).unwrap().attributes.iter().any(|a| {
                let a = tree.node(*a).unwrap();
                a.name.as_ref().is_some_and(|n| n.local_name == "id") && a.value == name
            })
        })
        .unwrap()
        .owner
}
#[test]
fn shared_source_has_distinct_host_inheritance_and_retained_source_styles() {
    let c = content("<svg xmlns='http://www.w3.org/2000/svg'><g color='red'><linearGradient id='a' href='#t'/></g><g color='blue'><linearGradient id='b' href='#t'/></g><defs color='green'><linearGradient id='t'><stop style='stop-color:var(--paint,currentColor)'/></linearGradient></defs><path fill='url(#a)' stroke='url(#b)'/></svg>");
    let p = SvgPaintStylePlan::new(c.clone(), 100_000).unwrap();
    assert!(Arc::ptr_eq(p.content(), &c));
    let a = p.hosts().iter().find(|h| h.host == id(&c, "a")).unwrap();
    let b = p.hosts().iter().find(|h| h.host == id(&c, "b")).unwrap();
    assert_eq!(a.selection, PaintContentSelection::Template(id(&c, "t")));
    assert_eq!(a.instances[0].source, b.instances[0].source);
    assert_eq!(a.instances[0].parent, PaintInstanceParent::Host);
    assert_ne!(a.host_ancestors, b.host_ancestors);
    let source = c.servers().iter().find(|s| s.owner == id(&c, "t")).unwrap();
    assert_eq!(source.nodes[1].source_ancestors[0], source.owner);
    assert!(!source.nodes[1].source_ancestors.contains(&a.host));
    let tree = &c.paint().inventory().document().tree;
    assert!(!tree
        .node(a.instances[0].source)
        .unwrap()
        .source
        .frames
        .is_empty());
    assert!(p
        .blockers()
        .contains(&"svg-paint-clone-style-evaluation-required"));
}
#[test]
fn own_content_stops_selection_but_descriptive_only_hosts_follow_the_chain() {
    let c = content("<svg xmlns='http://www.w3.org/2000/svg'><linearGradient id='a' href='#b'><title>Host</title></linearGradient><linearGradient id='b' href='#c'><desc>Middle</desc></linearGradient><radialGradient id='c'><stop/></radialGradient><path fill='url(#a)'/></svg>");
    let p = SvgPaintStylePlan::new(c.clone(), 100_000).unwrap();
    let a = p.hosts().iter().find(|h| h.host == id(&c, "a")).unwrap();
    assert_eq!(a.selection, PaintContentSelection::Template(id(&c, "c")));
    assert_eq!(a.template_attributes.len(), 2);
    assert_eq!(a.instances.len(), 1);
    let own = p.hosts().iter().find(|h| h.host == id(&c, "c")).unwrap();
    assert_eq!(own.selection, PaintContentSelection::Own);
    assert!(own.instances.is_empty());
}
#[test]
fn pattern_instances_keep_internal_parents_and_descriptive_source_content() {
    let c = content("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a' href='#b'/><pattern id='b'><title>Source</title><g color='red'><path/></g></pattern><path fill='url(#a)'/></svg>");
    let p = SvgPaintStylePlan::new(c.clone(), 100_000).unwrap();
    let a = p.hosts().iter().find(|h| h.host == id(&c, "a")).unwrap();
    assert_eq!(a.instances.len(), 4);
    assert_eq!(a.instances[0].parent, PaintInstanceParent::Host);
    assert_eq!(
        a.instances[1].parent,
        PaintInstanceParent::SourceInstance(a.instances[0].source)
    );
    assert_eq!(
        a.instances[3].parent,
        PaintInstanceParent::SourceInstance(a.instances[2].source)
    );
}
#[test]
fn cycle_edges_are_reconsidered_for_each_host_before_declaring_a_cycle() {
    let c = content("<svg xmlns='http://www.w3.org/2000/svg'><linearGradient id='a' href='#b'><stop/></linearGradient><linearGradient id='b' href='#a'/><path fill='url(#a)'/></svg>");
    let p = SvgPaintStylePlan::new(c.clone(), 100_000).unwrap();
    assert_eq!(
        p.hosts()
            .iter()
            .find(|h| h.host == id(&c, "b"))
            .unwrap()
            .selection,
        PaintContentSelection::Template(id(&c, "a"))
    );
    let c = content("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a' href='#b'/><pattern id='b' href='#a'/><path fill='url(#a)'/></svg>");
    let p = SvgPaintStylePlan::new(c, 100_000).unwrap();
    assert!(p.hosts().iter().all(|h| matches!(
        h.selection,
        PaintContentSelection::Unresolved(PaintTarget::Cycle(_))
    ) && h.instances.is_empty()));
}
#[test]
fn unresolved_links_and_empty_content_do_not_invent_instances() {
    for (href, expected) in [
        ("#missing", PaintTarget::Missing),
        ("other.svg#p", PaintTarget::UntraversedUrl),
    ] {
        let c = content(&format!("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a' href='{href}'/><path fill='url(#a)'/></svg>"));
        let p = SvgPaintStylePlan::new(c, 100_000).unwrap();
        assert_eq!(
            p.hosts()[0].selection,
            PaintContentSelection::Unresolved(expected)
        );
        assert!(p.hosts()[0].instances.is_empty());
    }
    let p = SvgPaintStylePlan::new(content("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a'><title>Only</title>text</pattern><path fill='url(#a)'/></svg>"), 100_000).unwrap();
    assert_eq!(p.hosts()[0].selection, PaintContentSelection::Empty);
}
#[test]
fn foreign_descriptive_names_are_not_svg_descriptive_elements() {
    let p = SvgPaintStylePlan::new(content("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a' href='#missing'><title xmlns='urn:foreign'/></pattern><path fill='url(#a)'/></svg>"), 100_000).unwrap();
    assert_eq!(p.hosts()[0].selection, PaintContentSelection::Own);
}
#[test]
fn work_exhaustion_is_atomic_and_plan_retains_original_validation() {
    let (mut host, inventory, cap) = fixture::setup("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a'><path/></pattern><path fill='url(#a)'/></svg>");
    let c = Arc::new(
        SvgPaintContentPlan::new(
            Arc::new(SvgPaintPlan::from_inventory(inventory, 100_000).unwrap()),
            100_000,
        )
        .unwrap(),
    );
    assert!(SvgPaintStylePlan::new(c.clone(), 0).is_err());
    let p = SvgPaintStylePlan::new(c, 100_000).unwrap();
    host.validate_dependencies(
        p.content().paint().inventory(),
        &cap,
        fixture::slots,
        fixture::BASE,
    )
    .unwrap();
    host.invalidate().unwrap();
    assert!(host
        .validate_dependencies(
            p.content().paint().inventory(),
            &cap,
            fixture::slots,
            fixture::BASE
        )
        .is_err());
}

#[test]
fn shared_content_expansion_has_an_aggregate_instance_bound() {
    let mut body = String::from("<svg xmlns='http://www.w3.org/2000/svg'><linearGradient id='t'>");
    for _ in 0..33 {
        body.push_str("<stop/>");
    }
    body.push_str("</linearGradient>");
    for n in 0..128 {
        body.push_str(&format!(
            "<linearGradient id='h{n}' href='#t'/><path fill='url(#h{n})'/>"
        ));
    }
    body.push_str("</svg>");
    let c = content(&body);
    assert_eq!(c.servers().len(), 129);
    assert!(SvgPaintStylePlan::new(c, 1_000_000)
        .err()
        .unwrap()
        .contains("instance bound"));
}

#[test]
fn own_content_keeps_source_context_even_with_a_template() {
    let c = content("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a' href='#b'><path/></pattern><pattern id='b'><g/></pattern><path fill='url(#a)'/></svg>");
    let p = SvgPaintStylePlan::new(c.clone(), 100_000).unwrap();
    let a = p.hosts().iter().find(|h| h.host == id(&c, "a")).unwrap();
    assert_eq!(a.selection, PaintContentSelection::Own);
    assert!(a.instances.is_empty() && a.template_attributes.is_empty());
    // Traversal still retains the template: attribute inheritance is a separate concern.
    assert_eq!(c.servers().len(), 2);
}
