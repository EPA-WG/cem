use cem_ml::{
    html_base::HtmlBasePolicyDecision, html_css_import_snapshot::HtmlExecutionCssImports,
    html_stylesheet_selection::HtmlStylesheetSelection, scheduler::AbortSignal,
};
use std::sync::Arc;
#[path = "support/stylesheet_fixture.rs"]
mod fixture;
use fixture::*;

#[test]
fn selection_is_exact_bounded_and_preserves_persistent_and_disabled_roots() {
    let (mut html, doc, cap) = setup("<style>p{color:red}</style><style title='Day'>p{color:blue}</style><link rel='alternate stylesheet' title='Night' href='theme.css'/><link rel='stylesheet' title='Night' href='theme.css' disabled=''/>");
    let sources = snapshot();
    let css = Arc::new(
        html.inspect_css_imports(
            doc,
            &cap,
            sources.clone(),
            Default::default(),
            AbortSignal::new(),
            slots,
            BASE,
        )
        .unwrap(),
    );
    let plan = HtmlStylesheetSelection::new(css.clone(), "Night", 256).unwrap();
    assert_eq!(
        plan.actions()
            .iter()
            .map(|a| (a.link, a.selected))
            .collect::<Vec<_>>(),
        [(false, false), (true, true), (true, false)]
    );
    assert!(HtmlStylesheetSelection::new(css.clone(), "night", 256).is_err());
    assert!(HtmlStylesheetSelection::new(css.clone(), "Day", 256).is_err());
    assert!(HtmlStylesheetSelection::new(css.clone(), "Night", 1).is_err());
    assert!(HtmlStylesheetSelection::new(css.clone(), &"x".repeat(4097), 256).is_err());
    assert!(HtmlStylesheetSelection::new(css, "", 256)
        .unwrap()
        .actions()
        .iter()
        .all(|a| !a.selected));
}

#[test]
fn a_disabled_only_title_cannot_be_activated() {
    let (mut html, doc, cap) =
        setup("<link rel='stylesheet' title='Night' href='theme.css' disabled=''/>");
    let css = Arc::new(
        html.inspect_css_imports(
            doc,
            &cap,
            snapshot(),
            Default::default(),
            AbortSignal::new(),
            slots,
            BASE,
        )
        .unwrap(),
    );
    assert!(HtmlStylesheetSelection::new(css, "Night", 256).is_err());
}

fn recapture(
    html: &mut cem_ml::html_base_host::HtmlBaseExecutionHost,
    css: &HtmlExecutionCssImports,
    cap: &cem_ml::module_resolution::CemModuleUrlResolutionCapability,
    sources: Arc<cem_ml::css_import_snapshot::CssSourceSnapshot>,
    stamp: &str,
) -> Arc<HtmlExecutionCssImports> {
    let resolution = css.document().base().native_resolution();
    let mut context = resolution.context.clone();
    context.policy_stamp = stamp.into();
    html.prepare(resolution.tree.clone(), context, slots, |_| {
        Ok(HtmlBasePolicyDecision::Allow)
    })
    .unwrap();
    let base = html.confirm(BASE.into()).unwrap();
    let doc = Arc::new(
        html.inspect_dependencies(&base, cap, 10000, slots, BASE)
            .unwrap(),
    );
    Arc::new(
        html.inspect_css_imports(
            doc,
            cap,
            sources,
            Default::default(),
            AbortSignal::new(),
            slots,
            BASE,
        )
        .unwrap(),
    )
}

#[test]
fn recapture_requires_same_native_owner_and_source_snapshot_but_a_new_html_epoch() {
    let (mut html, doc, cap) =
        setup("<link rel='alternate stylesheet' title='Night' href='theme.css'/>");
    let sources = snapshot();
    let css = Arc::new(
        html.inspect_css_imports(
            doc,
            &cap,
            sources.clone(),
            Default::default(),
            AbortSignal::new(),
            slots,
            BASE,
        )
        .unwrap(),
    );
    let plan = || HtmlStylesheetSelection::new(css.clone(), "Night", 256).unwrap();
    assert!(plan().validate_recapture(&css, &cap, &sources).is_err());
    let fresh = recapture(&mut html, &css, &cap, sources.clone(), "v1");
    plan().validate_recapture(&fresh, &cap, &sources).unwrap();
    assert!(plan()
        .validate_recapture(&fresh, &cap, &snapshot())
        .is_err());
    let changed = recapture(&mut html, &css, &cap, sources.clone(), "v2");
    assert!(plan().validate_recapture(&changed, &cap, &sources).is_err());
    let (mut other, doc, other_cap) =
        setup("<link rel='alternate stylesheet' title='Night' href='theme.css'/>");
    let foreign = Arc::new(
        other
            .inspect_css_imports(
                doc,
                &other_cap,
                sources.clone(),
                Default::default(),
                AbortSignal::new(),
                slots,
                BASE,
            )
            .unwrap(),
    );
    assert!(plan().validate_recapture(&foreign, &cap, &sources).is_err());
}

#[test]
fn aborted_source_preparation_cannot_be_rebound_after_selection() {
    let (mut html, doc, cap) =
        setup("<link rel='alternate stylesheet' title='Night' href='theme.css'/>");
    let sources = snapshot();
    let abort = AbortSignal::new();
    let css = Arc::new(
        html.inspect_css_imports(
            doc,
            &cap,
            sources.clone(),
            Default::default(),
            abort.clone(),
            slots,
            BASE,
        )
        .unwrap(),
    );
    let plan = HtmlStylesheetSelection::new(css.clone(), "Night", 256).unwrap();
    let fresh = recapture(&mut html, &css, &cap, sources.clone(), "v1");
    abort.abort();
    assert!(plan.validate_recapture(&fresh, &cap, &sources).is_err());
}
