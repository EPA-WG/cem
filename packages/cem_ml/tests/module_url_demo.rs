use cem_ml::module_resolution::{
    CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlMapping, CemModuleUrlResolutionPurpose,
    CemModuleUrlResolutionRequest, CemModuleUrlResolver, CemResolutionContextHandle,
    CemScopedModuleUrlResolver,
};
use cem_ml::source_map::SourceMapStack;

#[test]
fn prefix_mapped_demo_image_and_fragment_keep_their_resource_bases() {
    let base = "https://example.test/demo/module-url.html";
    let mut frame = CemModuleUrlFrame::new("demo-page", base);
    frame.specifiers.imports.insert(
        "lib-root/".into(),
        CemModuleUrlMapping::target("./lib-dir/"),
    );
    let handle = CemResolutionContextHandle::new("demo");
    let resolver = CemScopedModuleUrlResolver::new().with_context(
        handle.clone(),
        CemModuleUrlContext {
            identity: "demo-context".into(),
            resolver_identity: "demo-resolver".into(),
            resource_policy_stamp: "demo-policy".into(),
            frames: vec![frame],
        },
    );
    for (specifier, expected) in [
        (
            "lib-root/embed-lib.html#embed-relative-hash",
            "https://example.test/demo/lib-dir/embed-lib.html#embed-relative-hash",
        ),
        (
            "lib-root/Smiley.svg",
            "https://example.test/demo/lib-dir/Smiley.svg",
        ),
    ] {
        let result = resolver
            .resolve_module_url(&CemModuleUrlResolutionRequest {
                purpose: CemModuleUrlResolutionPurpose::TemplateSlice,
                authored_specifier: specifier.into(),
                current_context: handle.clone(),
                referrer: None,
                source_map: SourceMapStack::default(),
            })
            .expect("both resources resolve through the authored prefix map");
        assert_eq!(result.resolved_url, expected);
        assert_eq!(result.matched_frame_id.as_deref(), Some("demo-page"));
        assert_eq!(result.matched_key.as_deref(), Some("lib-root/"));
    }
}
