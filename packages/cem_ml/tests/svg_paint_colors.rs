use cem_ml::{
    svg_paint_clone_profile::{SvgPaintCloneProfile, CHROMIUM_CLONE_PROFILE_VERSION},
    svg_paint_colors::BoundedPaintColors,
    svg_paint_content::SvgPaintContentPlan,
    svg_paint_dependencies::SvgPaintPlan,
    svg_paint_style::SvgPaintStylePlan,
};
use std::sync::Arc;
#[path = "support/stylesheet_fixture.rs"]
mod fixture;
fn plan(body: &str) -> Arc<SvgPaintStylePlan> {
    let url = "https://page.test/docs/page.xhtml";
    let (_, inv, _) = fixture::setup_with_base(body, url, url);
    Arc::new(
        SvgPaintStylePlan::new(
            Arc::new(
                SvgPaintContentPlan::new(
                    Arc::new(SvgPaintPlan::from_inventory(inv, 100_000).unwrap()),
                    100_000,
                )
                .unwrap(),
            ),
            100_000,
        )
        .unwrap(),
    )
}
fn evaluate(body: &str) -> Result<BoundedPaintColors, String> {
    BoundedPaintColors::evaluate(
        plan(body),
        SvgPaintCloneProfile::for_browser("chromium", CHROMIUM_CLONE_PROFILE_VERSION).unwrap(),
        100_000,
    )
}
#[test]
fn literal_color_retains_its_exact_declaration_and_native_tree() {
    let p = plan("<svg xmlns='http://www.w3.org/2000/svg'><linearGradient id='a'><stop stop-color='#1a2b3c'/></linearGradient><path fill='url(#a)'/></svg>");
    let e = BoundedPaintColors::evaluate(
        p.clone(),
        SvgPaintCloneProfile::for_browser("chromium", CHROMIUM_CLONE_PROFILE_VERSION).unwrap(),
        100_000,
    )
    .unwrap();
    assert!(Arc::ptr_eq(e.plan(), &p));
    let v = &e.colors()[0];
    assert_eq!(v.rgba, [26, 43, 60, 255]);
    assert!(v.color_declarations.is_empty());
    let tree = &p.content().paint().inventory().document().tree;
    assert_eq!(tree.node(v.paint_declaration).unwrap().value, "#1a2b3c");
    assert!(!tree
        .node(v.paint_declaration)
        .unwrap()
        .source
        .frames
        .is_empty());
}
#[test]
fn cloned_currentcolor_uses_source_ancestry_without_losing_host_identity() {
    let e = evaluate("<svg xmlns='http://www.w3.org/2000/svg'><linearGradient id='a' href='#b' color='blue'/><linearGradient id='b' color='red'><stop stop-color='currentColor'/></linearGradient><path fill='url(#a)'/></svg>").unwrap();
    assert_eq!(e.colors().len(), 2);
    assert!(e.colors().iter().all(|v| v.rgba == [255, 0, 0, 255]));
    assert_ne!(e.colors()[0].host, e.colors()[1].host);
    assert_eq!(e.colors()[0].source, e.colors()[1].source);
    assert_eq!(
        e.colors()[0].color_declarations,
        e.colors()[1].color_declarations
    );
    assert!(e
        .blockers()
        .contains(&"svg-computed-winning-source-required"));
}
#[test]
fn currentcolor_on_color_inherits_and_records_every_consulted_declaration() {
    let e = evaluate("<svg xmlns='http://www.w3.org/2000/svg' color='#0f0'><linearGradient id='a' color='currentColor'><stop color='currentColor' stop-color='currentColor'/></linearGradient><path fill='url(#a)'/></svg>").unwrap();
    assert_eq!(e.colors()[0].rgba, [0, 255, 0, 255]);
    assert_eq!(e.colors()[0].color_declarations.len(), 3);
    assert_eq!(e.colors()[0].inheritance_nodes.len(), 3);
}
#[test]
fn pattern_descriptive_host_is_suppressed_but_its_source_is_retained() {
    let e = evaluate("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a' href='#b'><title>Host</title></pattern><pattern id='b' color='blue'><rect fill='currentColor'/></pattern><path fill='url(#a)'/></svg>").unwrap();
    assert_eq!(e.suppressed_hosts().len(), 1);
    assert_eq!(e.colors().len(), 1);
    assert_eq!(e.colors()[0].rgba, [0, 0, 255, 255]);
    assert_eq!(e.plan().content().servers().len(), 2);
}
#[test]
fn unsupported_cascade_values_and_shapes_fail_without_partial_evaluation() {
    for body in [
        "<style>stop{stop-color:blue}</style><linearGradient id='a'><stop stop-color='red'/></linearGradient>",
        "<linearGradient id='a'><stop style='stop-color:blue' stop-color='red'/></linearGradient>",
        "<linearGradient id='a'><stop stop-color='var(--paint)'/></linearGradient>",
        "<linearGradient id='a'><stop stop-color='currentColor'/></linearGradient>",
        "<linearGradient id='a'><stop stop-color='red'/><stop stop-color='blue'/></linearGradient>",
        "<linearGradient id='a' href='#missing'/>",
        "<pattern id='a'><g><rect fill='red'/></g></pattern>",
        "<linearGradient id='a' href='#b'/><linearGradient id='b' href='#c'/><linearGradient id='c'><stop stop-color='red'/></linearGradient>",
        "<linearGradient id='a'><stop stop-color='red'><animate attributeName='stop-color'/></stop></linearGradient>",
    ] {
        assert!(evaluate(&format!("<svg xmlns='http://www.w3.org/2000/svg'>{body}<path fill='url(#a)'/></svg>")).is_err(),"{body}");
    }
}
#[test]
fn work_exhaustion_does_not_publish_a_partial_result() {
    let p = plan("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a'><rect fill='blue'/></pattern><path fill='url(#a)'/></svg>");
    let profile =
        SvgPaintCloneProfile::for_browser("chromium", CHROMIUM_CLONE_PROFILE_VERSION).unwrap();
    assert!(BoundedPaintColors::evaluate(p.clone(), profile, 1).is_err());
    assert_eq!(
        BoundedPaintColors::evaluate(p, profile, 100_000)
            .unwrap()
            .colors()[0]
            .rgba,
        [0, 0, 255, 255]
    );
}
#[test]
fn evaluation_retains_original_inventory_validation_requirement() {
    let (mut host, inv, cap) = fixture::setup("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a'><rect fill='red'/></pattern><path fill='url(#a)'/></svg>");
    let p = Arc::new(
        SvgPaintStylePlan::new(
            Arc::new(
                SvgPaintContentPlan::new(
                    Arc::new(SvgPaintPlan::from_inventory(inv, 100_000).unwrap()),
                    100_000,
                )
                .unwrap(),
            ),
            100_000,
        )
        .unwrap(),
    );
    let e = BoundedPaintColors::evaluate(
        p,
        SvgPaintCloneProfile::for_browser("chromium", CHROMIUM_CLONE_PROFILE_VERSION).unwrap(),
        100_000,
    )
    .unwrap();
    host.validate_dependencies(
        e.plan().content().paint().inventory(),
        &cap,
        fixture::slots,
        fixture::BASE,
    )
    .unwrap();
    host.invalidate().unwrap();
    assert!(host
        .validate_dependencies(
            e.plan().content().paint().inventory(),
            &cap,
            fixture::slots,
            fixture::BASE
        )
        .is_err());
}

#[test]
fn retained_css_tokens_allow_case_and_comments_but_reject_trailing_values() {
    for (value, expected) in [
        ("ReD /* comment */", [255, 0, 0, 255]),
        ("#ABC", [170, 187, 204, 255]),
        ("transparent", [0, 0, 0, 0]),
    ] {
        let e = evaluate(&format!("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a'><rect fill='{value}'/></pattern><path fill='url(#a)'/></svg>")).unwrap();
        assert_eq!(e.colors()[0].rgba, expected);
    }
    for value in [
        "red blue",
        "#abcd",
        "#12",
        "rgb(1,2,3)",
        "inherit",
        "initial",
        "context-fill",
    ] {
        assert!(evaluate(&format!("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a'><rect fill='{value}'/></pattern><path fill='url(#a)'/></svg>")).is_err());
    }
}
#[test]
fn local_color_override_wins_within_the_presentation_only_profile() {
    let e = evaluate("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a' color='red'><rect color='blue' fill='currentColor'/></pattern><path color='green' fill='url(#a)'/></svg>").unwrap();
    let value = &e.colors()[0];
    assert_eq!(value.rgba, [0, 0, 255, 255]);
    assert_eq!(value.inheritance_nodes, vec![value.source]);
    assert_eq!(value.color_declarations.len(), 1);
}
