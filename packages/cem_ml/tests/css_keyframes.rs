use std::sync::Arc;

use cem_ml::{
    css_emission::emit_css_keyframes,
    css_resources::{resolve_css_resources, CssResourcePlan},
    import::import_data,
    module_resolution::{
        CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlResolutionCapability,
        CemResolutionContextHandle, CemScopedModuleUrlResolver,
    },
};

fn plan(css: &str) -> CssResourcePlan {
    let tree = import_data(css, "text/css", "cem", "urn:css:owner").unwrap();
    let handle = CemResolutionContextHandle::new("fixture");
    let resolver = CemScopedModuleUrlResolver::new().with_context(
        handle.clone(),
        CemModuleUrlContext {
            identity: "fixture".into(),
            resolver_identity: "fixture".into(),
            resource_policy_stamp: "policy".into(),
            frames: vec![CemModuleUrlFrame::new(
                "page",
                "https://example.test/page.html",
            )],
        },
    );
    resolve_css_resources(
        tree,
        &CemModuleUrlResolutionCapability::new(Arc::new(resolver), handle),
        "https://example.test/styles/main.css",
    )
    .unwrap()
}
fn first_rule(plan: &CssResourcePlan) -> u32 {
    let root = plan.tree.node(0).unwrap().children[0];
    plan.tree.node(root).unwrap().children[0]
}

#[test]
fn keyframes_emit_scoped_names_ordered_offsets_and_resolved_declarations() {
    let plan =
        plan("@keyframes pulse { from,25% {opacity:0; background:url(icon.svg)} to {opacity:1} }");
    let result = emit_css_keyframes(&plan, first_rule(&plan), "owner-s1").unwrap();
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let rule = result.rule.unwrap();
    assert_eq!(rule.name, "pulse");
    assert_eq!(rule.scoped_name, "pulse-owner-s1");
    assert_eq!(rule.css(), "@keyframes pulse-owner-s1 {from, 25% {opacity:0;background:url(\"https://example.test/styles/icon.svg\");}to {opacity:1;}}");
    assert!(rule
        .frames
        .iter()
        .all(|frame| frame.source.origin().is_some() && frame.range.length > 0));
}

#[test]
fn keyframes_handle_string_names_vendor_prefixes_and_context_isolation() {
    let plan = plan("@-WEBKIT-keyframes \"quoted name\" {0% {opacity:0}}");
    let first = emit_css_keyframes(&plan, first_rule(&plan), "one")
        .unwrap()
        .rule
        .unwrap();
    let second = emit_css_keyframes(&plan, first_rule(&plan), "two")
        .unwrap()
        .rule
        .unwrap();
    assert_eq!(first.name, "quoted name");
    assert_eq!(first.opening, "@-webkit-keyframes quoted\\20 name-one {");
    assert_eq!(second.scoped_name, "quoted name-two");
    assert!(first.css().contains("quoted\\20 name-one"));
    assert!(emit_css_keyframes(&plan, first_rule(&plan), "").is_err());
    let plan = self::plan(".card {color:red}");
    assert!(emit_css_keyframes(&plan, first_rule(&plan), "one").is_err());
}

#[test]
fn keyframes_suppress_invalid_names_offsets_and_unsupported_body_constructs() {
    let plan = plan("@keyframes none {from {opacity:0}}");
    let result = emit_css_keyframes(&plan, first_rule(&plan), "one").unwrap();
    assert!(result.rule.is_none());
    assert_eq!(
        result.diagnostics[0].code,
        "cem.scoped_css.keyframe_name_invalid"
    );
    let plan = self::plan("@keyframes pulse { .card {opacity:0} from {opacity:0 !important; color:red} @media all {to {opacity:0.5}} to {opacity:1} }");
    let result = emit_css_keyframes(&plan, first_rule(&plan), "one").unwrap();
    assert_eq!(
        result.rule.unwrap().css(),
        "@keyframes pulse-one {from {color:red;}to {opacity:1;}}"
    );
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|d| d.code)
            .collect::<Vec<_>>(),
        [
            "cem.scoped_css.keyframe_offset_unsupported",
            "cem.scoped_css.important_unsupported",
            "cem.scoped_css.keyframe_body_unsupported"
        ]
    );
    assert!(result
        .diagnostics
        .iter()
        .all(|d| d.source.origin().is_some()));
    for (css, expected) in [
        ("@keyframes pulse {}", "@keyframes pulse-one {}"),
        (
            "@keyframes pulse {101% {opacity:0}}",
            "@keyframes pulse-one {}",
        ),
        (
            "@keyframes pulse {from {opacity:0 !important}}",
            "@keyframes pulse-one {from {}}",
        ),
    ] {
        let plan = self::plan(css);
        assert_eq!(
            emit_css_keyframes(&plan, first_rule(&plan), "one")
                .unwrap()
                .rule
                .unwrap()
                .css(),
            expected
        );
    }
}
