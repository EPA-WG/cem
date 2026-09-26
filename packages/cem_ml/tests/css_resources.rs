use cem_ml::{
    css_resources::{resolve_css_resources, CssResourceKind},
    import::import_data,
    module_resolution::{
        CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlMapping,
        CemModuleUrlResolutionCapability, CemModuleUrlResolutionErrorReason,
        CemResolutionContextHandle, CemScopedModuleUrlResolver,
    },
};
use std::sync::Arc;

fn capability(inner_target: &str, blocked: bool) -> CemModuleUrlResolutionCapability {
    let mut outer = CemModuleUrlFrame::new("page", "https://example.test/page.html");
    outer.specifiers.resources.insert(
        "theme".into(),
        if blocked {
            CemModuleUrlMapping::blocked()
        } else {
            CemModuleUrlMapping::target("./outer.css")
        },
    );
    outer
        .specifiers
        .resources
        .insert("icon".into(), CemModuleUrlMapping::target("./icon.svg"));
    let mut inner = CemModuleUrlFrame::new("component", "https://example.test/card/card.cem");
    inner.specifiers.resources.insert(
        "theme".into(),
        CemModuleUrlMapping::target(inner_target)
            .with_content_type("text/css")
            .with_integrity("sha256-fixture"),
    );
    let handle = CemResolutionContextHandle::new(inner_target);
    let resolver = CemScopedModuleUrlResolver::new().with_context(
        handle.clone(),
        CemModuleUrlContext {
            identity: inner_target.into(),
            resolver_identity: "fixture".into(),
            resource_policy_stamp: "policy".into(),
            frames: vec![outer, inner],
        },
    );
    CemModuleUrlResolutionCapability::new(Arc::new(resolver), handle)
}

#[test]
fn retained_css_resources_preserve_order_conditions_metadata_and_source() {
    let source = "/* url(fake) */\n@import 'theme' layer(base) supports(display:grid) screen;\n:scope { background: url(icon); mask: URL(\"./a\\20 b.svg\"); content: 'url(fake)'; --asset: url(./custom.svg); }";
    let tree = import_data(source, "text/css", "cem", "urn:css:lexical-owner").unwrap();
    let plan = resolve_css_resources(
        tree.clone(),
        &capability("./inner.css", false),
        "https://example.test/assets/imported.css",
    )
    .unwrap();
    assert!(Arc::ptr_eq(&tree, &plan.tree));
    assert_eq!(plan.references.len(), 4);
    let first = &plan.references[0];
    assert_eq!(
        first.kind,
        CssResourceKind::Import {
            layer: Some("base".into()),
            supports: Some("display:grid".into()),
            media: Some("screen".into())
        }
    );
    let resolved = first.resolution.as_ref().unwrap();
    assert_eq!(resolved.resolved_url, "https://example.test/card/inner.css");
    assert_eq!(resolved.content_type_hint.as_deref(), Some("text/css"));
    assert_eq!(resolved.integrity.as_deref(), Some("sha256-fixture"));
    assert_eq!(first.range.line, 2);
    assert_eq!(first.range.offset as usize, source.find("@import").unwrap());
    assert!(first.source.origin().is_some());
    assert_eq!(
        plan.references[1].resolution.as_ref().unwrap().resolved_url,
        "https://example.test/icon.svg"
    );
    assert_eq!(plan.references[2].authored_specifier, "./a b.svg");
    assert_eq!(
        plan.references[2].resolution.as_ref().unwrap().resolved_url,
        "https://example.test/assets/a%20b.svg"
    );
    assert_eq!(
        plan.references[3].resolution.as_ref().unwrap().resolved_url,
        "https://example.test/assets/custom.svg"
    );
}

#[test]
fn retained_css_resources_keep_failures_and_contexts_independent() {
    let tree = import_data(
        "@import 'theme'; a { mask:url(missing); background:url(./ok.svg) }",
        "text/css",
        "cem",
        "urn:css:owner",
    )
    .unwrap();
    let first = resolve_css_resources(
        tree.clone(),
        &capability("./one.css", true),
        "https://example.test/css/one.css",
    )
    .unwrap();
    assert_eq!(
        first.references[0].resolution.as_ref().unwrap_err().reason,
        CemModuleUrlResolutionErrorReason::Blocked
    );
    assert_eq!(
        first.references[1].resolution.as_ref().unwrap_err().reason,
        CemModuleUrlResolutionErrorReason::Unresolved
    );
    assert_eq!(
        first.references[2]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://example.test/css/ok.svg"
    );
    let second = resolve_css_resources(
        tree,
        &capability("./two.css", false),
        "https://example.test/other/two.css",
    )
    .unwrap();
    assert_eq!(
        second.references[0]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://example.test/card/two.css"
    );
    assert_eq!(
        second.references[2]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://example.test/other/ok.svg"
    );
    let xml = import_data(
        "<a/>",
        "application/xml",
        "cem",
        "https://example.test/a.xml",
    )
    .unwrap();
    assert!(resolve_css_resources(
        xml,
        &capability("./one.css", false),
        "https://example.test/"
    )
    .is_err());
}

#[test]
fn retained_css_resources_cover_nested_urls_import_variants_and_empty_trees() {
    let source = "@import url(\"./first.css\") layer; @import url(./second.css); @media screen { a { mask: url( './mask.svg' /* gap */); --image: { image: url(./custom.svg) }; } }";
    let tree = cem_ml::import::import_data_bytes(
        source.as_bytes(),
        "text/css; mode=scoped-style-block",
        "cem",
        "urn:css:owner",
    )
    .unwrap();
    let plan = resolve_css_resources(
        tree,
        &capability("./theme.css", false),
        "https://example.test/css/main.css",
    )
    .unwrap();
    assert_eq!(
        plan.references
            .iter()
            .map(|r| r.authored_specifier.as_str())
            .collect::<Vec<_>>(),
        ["./first.css", "./second.css", "./mask.svg", "./custom.svg"]
    );
    assert_eq!(
        plan.references[0].kind,
        CssResourceKind::Import {
            layer: Some(String::new()),
            supports: None,
            media: None
        }
    );
    assert_eq!(
        plan.references[1].kind,
        CssResourceKind::Import {
            layer: None,
            supports: None,
            media: None
        }
    );
    for reference in &plan.references {
        assert!(reference.resolution.is_ok());
    }
    let empty = import_data("", "text/css", "cem", "urn:css:empty").unwrap();
    assert!(resolve_css_resources(
        empty.clone(),
        &capability("./theme.css", false),
        "https://example.test/main.css"
    )
    .unwrap()
    .references
    .is_empty());
    assert!(
        resolve_css_resources(empty, &capability("./theme.css", false), "relative.css").is_err()
    );
}

#[test]
fn retained_css_resources_do_not_ignore_recovered_bad_strings_as_trivia() {
    let tree = import_data(
        "a { mask: url(\"./ok.svg\" \"broken\n) }",
        "text/css",
        "cem",
        "urn:css:recovery",
    )
    .unwrap();
    assert!(resolve_css_resources(
        tree,
        &capability("./theme.css", false),
        "https://example.test/main.css"
    )
    .is_err());
}
