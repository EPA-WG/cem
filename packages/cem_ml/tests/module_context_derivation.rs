use cem_ml::module_resolution::*;
use std::sync::Arc;
fn capability() -> CemModuleUrlResolutionCapability {
    let handle = CemResolutionContextHandle::new("host");
    let mut outer = CemModuleUrlFrame::new("outer", "https://host.test/root/");
    outer
        .specifiers
        .resources
        .insert("asset".into(), CemModuleUrlMapping::target("./host.svg"));
    outer
        .specifiers
        .resources
        .insert("denied".into(), CemModuleUrlMapping::blocked());
    outer.allowed_schemes = Some(["https".into()].into());
    CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(
            handle.clone(),
            CemModuleUrlContext {
                identity: "host".into(),
                resolver_identity: "native".into(),
                resource_policy_stamp: "policy".into(),
                frames: vec![outer],
            },
        )),
        handle,
    )
}
#[test]
fn derived_native_context_preserves_parent_rules_and_separate_ownership() {
    let parent = capability();
    let parent_stamp = parent.cache_identity();
    let mut frame = CemModuleUrlFrame::new("child", "https://module.test/child.cemt");
    frame
        .specifiers
        .resources
        .insert("asset".into(), CemModuleUrlMapping::target("./child.svg"));
    frame
        .specifiers
        .resources
        .insert("denied".into(), CemModuleUrlMapping::target("./child.svg"));
    let child = parent.derive_child_context(frame.clone()).unwrap();
    let equal = parent.derive_child_context(frame).unwrap();
    assert!(!parent.same_owner(&child));
    assert!(!child.same_owner(&equal));
    assert_eq!(child.cache_identity(), equal.cache_identity());
    assert_eq!(parent.cache_identity(), parent_stamp);
    let resolve = |purpose, value| child.resolve(purpose, value, Default::default());
    assert_eq!(
        resolve(CemModuleUrlResolutionPurpose::CemQl, "asset")
            .unwrap()
            .resolved_url,
        "https://host.test/root/host.svg"
    );
    assert_eq!(
        resolve(CemModuleUrlResolutionPurpose::Css, "asset")
            .unwrap()
            .resolved_url,
        "https://module.test/child.svg"
    );
    assert_eq!(
        resolve(CemModuleUrlResolutionPurpose::CemQl, "./local.svg")
            .unwrap()
            .resolved_url,
        "https://module.test/local.svg"
    );
    assert_eq!(
        resolve(CemModuleUrlResolutionPurpose::Css, "denied")
            .unwrap_err()
            .reason,
        CemModuleUrlResolutionErrorReason::Blocked
    );
    assert_eq!(
        resolve(CemModuleUrlResolutionPurpose::CemQl, "file:///no")
            .unwrap_err()
            .reason,
        CemModuleUrlResolutionErrorReason::PolicyDenied
    );
}
#[test]
fn derived_native_context_rejects_invalid_or_duplicate_frames() {
    let parent = capability();
    assert!(parent
        .derive_child_context(CemModuleUrlFrame::new("outer", "https://module.test/"))
        .is_err());
    assert!(parent
        .derive_child_context(CemModuleUrlFrame::new("child", "relative/path"))
        .is_err());
    assert!(parent
        .derive_child_context(CemModuleUrlFrame::new("", "https://module.test/"))
        .is_err());
}
