use cem_ml::{
    svg_paint_clone_profile::{SvgPaintCloneProfile, CHROMIUM_CLONE_PROFILE_VERSION},
    svg_paint_colors::{BoundedPaintColors, PaintDeclarationOrigin},
    svg_paint_content::SvgPaintContentPlan,
    svg_paint_dependencies::SvgPaintPlan,
    svg_paint_style::SvgPaintStylePlan,
};
use std::sync::Arc;
#[allow(dead_code)]
#[path = "support/stylesheet_fixture.rs"]
mod fixture;
fn evaluate(css: &str, stop: &str) -> Result<BoundedPaintColors, String> {
    let css = css.replace('&', "&amp;").replace('<', "&lt;");
    evaluate_body(
        &format!("<style>{css}</style><linearGradient id='a'>{stop}</linearGradient>"),
        1_000_000,
    )
}
fn evaluate_body(body: &str, budget: usize) -> Result<BoundedPaintColors, String> {
    let url = "https://page.test/docs/page.xhtml";
    let (_, inv, _) = fixture::setup_with_base(
        &format!("<svg xmlns='http://www.w3.org/2000/svg'>{body}<path fill='url(#a)'/></svg>"),
        url,
        url,
    );
    let plan = Arc::new(SvgPaintStylePlan::new(
        Arc::new(SvgPaintContentPlan::new(
            Arc::new(SvgPaintPlan::from_inventory(inv, 1_000_000)?),
            1_000_000,
        )?),
        1_000_000,
    )?);
    BoundedPaintColors::evaluate(
        plan,
        SvgPaintCloneProfile::for_browser("chromium", CHROMIUM_CLONE_PROFILE_VERSION).unwrap(),
        budget,
    )
}
#[test]
fn compound_lists_keep_matched_specificity_and_exact_native_origins() {
    let e = evaluate(
        "#absent, stop {stop-color:red} .hot, stop.hot {stop-color:blue}",
        "<stop class='warm hot'/>",
    )
    .unwrap();
    let w = &e.colors()[0].paint_winner;
    assert_eq!(e.colors()[0].rgba, [0, 0, 255, 255]);
    let PaintDeclarationOrigin::Rule {
        owner,
        css,
        rule,
        declaration,
        selectors,
        specificity,
        sheet_order,
        rule_order,
    } = &w.origin
    else {
        panic!("rule origin required")
    };
    assert_eq!(*specificity, (0, 1, 1));
    assert_eq!(selectors.len(), 2);
    assert_eq!((*sheet_order, *rule_order, w.order), (0, 1, 0));
    let inv = e.plan().content().paint().inventory();
    assert_eq!(
        inv.document()
            .tree
            .node(*owner)
            .unwrap()
            .name
            .as_ref()
            .unwrap()
            .local_name,
        "style"
    );
    assert!(Arc::ptr_eq(
        css,
        &inv.stylesheets()[0].resources.as_ref().unwrap().tree
    ));
    for id in [*rule, *declaration]
        .into_iter()
        .chain(selectors.iter().copied())
    {
        let n = css.node(id).unwrap();
        assert!(n.range.length > 0);
        assert!(!n.source.frames.is_empty());
    }
}
#[test]
fn author_priority_orders_importance_inline_specificity_and_source_order() {
    for (css, attrs, expected) in [
        ("*{stop-color:blue}", "stop-color='red'", [0, 0, 255, 255]),
        (
            "#s{stop-color:red}",
            "id='s' style='stop-color:blue'",
            [0, 0, 255, 255],
        ),
        (
            "stop{stop-color:blue!important}",
            "style='stop-color:red'",
            [0, 0, 255, 255],
        ),
        (
            "#s{stop-color:red!important}",
            "id='s' style='stop-color:blue!important'",
            [0, 0, 255, 255],
        ),
        (
            "#s{stop-color:blue} stop{stop-color:red}",
            "id='s'",
            [0, 0, 255, 255],
        ),
        ("stop{stop-color:red;stop-color:blue}", "", [0, 0, 255, 255]),
        (
            "stop{stop-color:blue!important;stop-color:red}",
            "",
            [0, 0, 255, 255],
        ),
        (
            ".hot.hot{stop-color:blue}.hot{stop-color:red}",
            "class='hot'",
            [0, 0, 255, 255],
        ),
        (
            "#missing,stop{stop-color:red}.hot{stop-color:blue}",
            "class='hot'",
            [0, 0, 255, 255],
        ),
    ] {
        assert_eq!(
            evaluate(css, &format!("<stop {attrs}/>")).unwrap().colors()[0].rgba,
            expected,
            "{css} {attrs}"
        );
    }
}
#[test]
fn namespace_case_escapes_and_ascii_class_membership() {
    for (css, attrs, expected) in [
        (
            "STOP{stop-color:red}stop{stop-color:blue}",
            "",
            [0, 0, 255, 255],
        ),
        (
            "|stop{stop-color:red}*|stop{stop-color:blue}",
            "",
            [0, 0, 255, 255],
        ),
        (
            ".Hot{stop-color:red}.hot{stop-color:blue}",
            "class='hot'",
            [0, 0, 255, 255],
        ),
        (
            "#S{stop-color:red}#s{stop-color:blue}",
            "id='s'",
            [0, 0, 255, 255],
        ),
        (
            r".h\6ft{stop-color:blue}",
            "class='warm&#9;hot'",
            [0, 0, 255, 255],
        ),
        (
            ".hot{stop-color:red}stop{stop-color:blue}",
            "class='hot warm'",
            [0, 0, 255, 255],
        ),
    ] {
        assert_eq!(
            evaluate(css, &format!("<stop {attrs}/>")).unwrap().colors()[0].rgba,
            expected,
            "{css}"
        );
    }
}
#[test]
fn document_sheet_order_and_same_color_winners_remain_distinct() {
    let e = evaluate_body("<style>stop{stop-color:red}</style><linearGradient id='a'><stop/></linearGradient><style>stop{stop-color:red}</style>", 1_000_000).unwrap();
    let PaintDeclarationOrigin::Rule {
        owner, sheet_order, ..
    } = &e.colors()[0].paint_winner.origin
    else {
        panic!()
    };
    assert_eq!(*sheet_order, 1);
    assert_eq!(
        *owner,
        e.plan().content().paint().inventory().stylesheets()[1].owner
    );
}
#[test]
fn rule_currentcolor_uses_source_ancestry_and_local_winners() {
    let e = evaluate_body("<style>#a{color:red!important} stop{color:blue;stop-color:currentColor}</style><linearGradient id='a'><stop/></linearGradient>", 1_000_000).unwrap();
    assert_eq!(e.colors()[0].rgba, [0, 0, 255, 255]);
    assert_eq!(e.colors()[0].color_winners.len(), 1);
    assert!(!e.colors()[0].color_winners[0].important);
    let e = evaluate_body("<style>#b{color:blue}stop{stop-color:currentColor}</style><linearGradient id='a' href='#b' color='red'/><linearGradient id='b'><stop/></linearGradient>",1_000_000).unwrap();
    assert!(e.colors().iter().all(|c| c.rgba == [0, 0, 255, 255]));
    assert_eq!(e.colors()[0].source, e.colors()[1].source);
}
#[test]
fn unsupported_even_nonmatching_rules_and_losing_values_fail_atomically() {
    for css in [
        "g stop{stop-color:blue}",
        "stop:hover{stop-color:blue}",
        "[id]{stop-color:blue}",
        "stop::before{color:red}",
        "stop,.x:hover{stop-color:red}",
        "@media all{stop{stop-color:blue}}",
        "@layer x{stop{stop-color:blue}}",
        "@namespace x url(http://www.w3.org/2000/svg);x|stop{stop-color:blue}",
        "stop{&.hot{stop-color:blue}}",
        ".missing{opacity:0}",
        ".missing{stop-color:orange}",
        "stop{stop-color:orange;stop-color:blue}",
    ] {
        assert!(evaluate(css, "<stop stop-color='red'/>").is_err(), "{css}");
    }
    for attrs in [
        "media='all'",
        "title='colors'",
        "type='application/unknown'",
        "xmlns='http://www.w3.org/1999/xhtml'",
    ] {
        assert!(evaluate_body(&format!("<style {attrs}>stop{{stop-color:blue}}</style><linearGradient id='a'><stop stop-color='red'/></linearGradient>"),1_000_000).is_err(), "{attrs}");
    }
}
#[test]
fn declaration_and_matching_work_bounds_fail_without_partial_plan() {
    assert!(evaluate_body(
        "<style>stop{stop-color:blue}</style><linearGradient id='a'><stop/></linearGradient>",
        1
    )
    .is_err());
    let compound = ".x".repeat(65);
    assert!(evaluate(
        &format!("{compound}{{stop-color:blue}}"),
        "<stop class='x'/>"
    )
    .is_err());
}

#[test]
fn stylesheet_rule_branch_and_declaration_caps_are_explicit() {
    // Spread aggregate bounds over small sheets: the shared CSS importer has
    // its own per-document token cap, which must not hide evaluator limits.
    for (kind, limit, message) in [
        (0, 1024, "rule bound"),
        (1, 4096, "selector bound"),
        (2, 4096, "declaration bound"),
    ] {
        for count in [limit, limit + 1] {
            let mut sheets = String::new();
            let mut remaining = count;
            while remaining > 0 {
                let n = remaining.min(100);
                let css = match kind {
                    0 => "stop{stop-color:blue}".repeat(n),
                    1 => format!("{}{{stop-color:blue}}", vec!["stop"; n].join(",")),
                    _ => format!("stop{{{}}}", "stop-color:blue;".repeat(n)),
                };
                sheets.push_str(&format!("<style>{css}</style>"));
                remaining -= n;
            }
            let result = evaluate_body(
                &format!("{sheets}<linearGradient id='a'><stop/></linearGradient>"),
                1_000_000,
            );
            if count == limit {
                assert!(result.is_ok(), "{message}: {:?}", result.err());
            } else {
                let error = result.err().expect("bound must reject");
                assert!(error.contains(message), "{message}: {error}");
            }
        }
    }
    let body = format!(
        "{}<linearGradient id='a'><stop stop-color='red'/></linearGradient>",
        "<style/>".repeat(257)
    );
    assert!(evaluate_body(&body, 1_000_000)
        .err()
        .unwrap()
        .contains("stylesheet bound"));
}
#[test]
fn stylesheet_plan_keeps_original_inventory_lifetime() {
    let (mut host, inv, cap) = fixture::setup("<svg xmlns='http://www.w3.org/2000/svg'><style>stop{stop-color:blue}</style><linearGradient id='a'><stop/></linearGradient><path fill='url(#a)'/></svg>");
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
        1_000_000,
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
