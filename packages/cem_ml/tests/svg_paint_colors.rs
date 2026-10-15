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
    assert!(v.color_winners.is_empty());
    let tree = &p.content().paint().inventory().document().tree;
    assert_eq!(
        tree.node(v.paint_winner.document_source()).unwrap().value,
        "#1a2b3c"
    );
    assert!(!tree
        .node(v.paint_winner.document_source())
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
        e.colors()[0]
            .color_winners
            .iter()
            .map(|w| w.document_source())
            .collect::<Vec<_>>(),
        e.colors()[1]
            .color_winners
            .iter()
            .map(|w| w.document_source())
            .collect::<Vec<_>>()
    );
    assert!(e
        .blockers()
        .contains(&"svg-computed-winning-source-required"));
}
#[test]
fn currentcolor_on_color_inherits_and_records_every_consulted_declaration() {
    let e = evaluate("<svg xmlns='http://www.w3.org/2000/svg' color='#0f0'><linearGradient id='a' color='currentColor'><stop color='currentColor' stop-color='currentColor'/></linearGradient><path fill='url(#a)'/></svg>").unwrap();
    assert_eq!(e.colors()[0].rgba, [0, 255, 0, 255]);
    assert_eq!(e.colors()[0].color_winners.len(), 3);
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
        "<style>g stop{stop-color:blue}</style><linearGradient id='a'><stop stop-color='red'/></linearGradient>",
        "<linearGradient id='a'><stop style='filter:blur(1px)' stop-color='red'/></linearGradient>",
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
    assert_eq!(value.color_winners.len(), 1);
}

#[test]
fn inline_winner_retains_css_declaration_and_host_attribute() {
    let e = evaluate("<svg xmlns='http://www.w3.org/2000/svg'><linearGradient id='a'><stop stop-color='red' style='stop-color:blue'/></linearGradient><path fill='url(#a)'/></svg>").unwrap();
    let c = &e.colors()[0];
    assert_eq!(c.rgba, [0, 0, 255, 255]);
    assert_eq!(c.property, "stop-color");
    let winner = &c.paint_winner;
    let css = winner.css().unwrap();
    let node = css.node(winner.css_declaration().unwrap()).unwrap();
    assert!(!node.source.frames.is_empty());
    assert!(node.range.length > 0);
    assert_eq!(winner.order, 0);
    assert!(!winner.important);
    assert_eq!(
        e.plan()
            .content()
            .paint()
            .inventory()
            .document()
            .tree
            .node(winner.document_source())
            .unwrap()
            .value,
        "stop-color:blue"
    );
}
#[test]
fn inline_duplicates_respect_importance_before_source_order() {
    for (style, expected, important, order) in [
        ("stop-color:red;stop-color:blue", [0, 0, 255, 255], false, 1),
        (
            "stop-color:blue!important;stop-color:red",
            [0, 0, 255, 255],
            true,
            0,
        ),
        (
            "stop-color:red!important;stop-color:blue!IMPORTANT",
            [0, 0, 255, 255],
            true,
            1,
        ),
        (
            "/*before*/ STOP-COLOR:blue ! /*gap*/ important;",
            [0, 0, 255, 255],
            true,
            0,
        ),
    ] {
        let e=evaluate(&format!("<svg xmlns='http://www.w3.org/2000/svg'><linearGradient id='a'><stop stop-color='green' style='{style}'/></linearGradient><path fill='url(#a)'/></svg>")).unwrap();
        let c = &e.colors()[0];
        assert_eq!(c.rgba, expected);
        assert_eq!(c.paint_winner.important, important);
        assert_eq!(c.paint_winner.order, order);
    }
}
#[test]
fn inline_only_paint_and_currentcolor_inheritance_keep_both_winners() {
    let e=evaluate("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a' color='red' style='color:blue!important'><rect style='fill:currentColor'/></pattern><path fill='url(#a)'/></svg>").unwrap();
    let c = &e.colors()[0];
    assert_eq!(c.rgba, [0, 0, 255, 255]);
    assert!(c.paint_winner.css().is_some());
    assert!(c.color_winners[0].important);
    assert_eq!(c.color_winners.len(), 1);
}
#[test]
fn child_color_beats_inherited_important_declaration() {
    let e=evaluate("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a' style='color:red!important'><rect color='green' style='color:blue;fill:currentColor'/></pattern><path fill='url(#a)'/></svg>").unwrap();
    assert_eq!(e.colors()[0].rgba, [0, 0, 255, 255]);
    assert!(!e.colors()[0].color_winners[0].important);
}
#[test]
fn unsupported_or_invalid_inline_declarations_do_not_fall_back_silently() {
    for style in [
        "fill:var(--paint)",
        "color:red;all:unset",
        "fill:red;fill:orange",
        "fill:red;fill:",
        "fill:red;@media screen{fill:blue}",
        "fill:red;--paint:blue",
    ] {
        assert!(evaluate(&format!("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a'><rect fill='blue' style='{style}'/></pattern><path fill='url(#a)'/></svg>")).is_err(),"{style}");
    }
}
#[test]
fn empty_inline_style_keeps_presentation_provenance() {
    let e=evaluate("<svg xmlns='http://www.w3.org/2000/svg'><pattern id='a'><rect fill='blue' style='/*empty*/'/></pattern><path fill='url(#a)'/></svg>").unwrap();
    assert_eq!(e.colors()[0].rgba, [0, 0, 255, 255]);
    assert!(e.colors()[0].paint_winner.css().is_none());
}
