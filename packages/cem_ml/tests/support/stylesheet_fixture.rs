use cem_ml::{
    css_import_snapshot::*, html_base::*, html_base_evidence::*, html_base_host::*,
    html_execution_inventory::HtmlExecutionInventory, module_resolution::*,
};
use std::sync::Arc;
pub const BASE: &str = "https://page.test/assets/";
pub fn slots(slot: HtmlBaseDomSlot<'_>) -> Result<u32, String> {
    Ok(slot.id + 1)
}
pub fn setup(
    body: &str,
) -> (
    HtmlBaseExecutionHost,
    Arc<HtmlExecutionInventory>,
    CemModuleUrlResolutionCapability,
) {
    setup_with_base(body, "../assets/", BASE)
}
#[allow(dead_code)]
pub fn setup_with_base(
    body: &str,
    href: &str,
    confirmed_base: &str,
) -> (
    HtmlBaseExecutionHost,
    Arc<HtmlExecutionInventory>,
    CemModuleUrlResolutionCapability,
) {
    let source=format!("<html xmlns='http://www.w3.org/1999/xhtml'><head><base href='{href}'/>{body}</head><body/></html>");
    let tree = cem_ml::import::import_data_bytes(
        source.as_bytes(),
        "application/xml",
        "cem",
        "urn:fixture",
    )
    .unwrap();
    let mut host = HtmlBaseExecutionHost::new(
        HtmlBaseCompatibilityProfile::StandardAgreement,
        HtmlBaseBrowser {
            engine: "chromium".into(),
            version: CHROMIUM_BASE_PROFILE_VERSION.into(),
        },
        HtmlBaseLimits {
            nodes: 1000,
            url_bytes: 4096,
        },
    )
    .unwrap();
    host.prepare(
        tree,
        HtmlBaseContext {
            fallback: HtmlFallbackBase::DocumentUrl("https://page.test/docs/page.xhtml".into()),
            encoding: HtmlBaseEncoding::Utf8,
            policy_stamp: "v1".into(),
        },
        slots,
        |_| Ok(HtmlBasePolicyDecision::Allow),
    )
    .unwrap();
    let base = host.confirm(confirmed_base.into()).unwrap();
    let handle = CemResolutionContextHandle::new("fixture");
    let mut frame = CemModuleUrlFrame::new("fixture", "https://maps.test/");
    frame.allowed_schemes = Some(["https".into()].into());
    frame
        .specifiers
        .resources
        .insert("icon".into(), CemModuleUrlMapping::target("mapped.svg"));
    frame.specifiers.resources.insert(
        "https://page.test/assets/denied.css".into(),
        CemModuleUrlMapping::blocked(),
    );
    let cap = CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(
            handle.clone(),
            CemModuleUrlContext {
                identity: "fixture".into(),
                resolver_identity: "native".into(),
                resource_policy_stamp: "v1".into(),
                frames: vec![frame],
            },
        )),
        handle,
    );
    let inventory = Arc::new(
        host.inspect_dependencies(&base, &cap, 10_000, slots, confirmed_base)
            .unwrap(),
    );
    (host, inventory, cap)
}
#[allow(dead_code)]
pub fn snapshot() -> Arc<CssSourceSnapshot> {
    CssSourceSnapshot::new(
        vec![
            CssSnapshotSource {
                requested_url: format!("{BASE}theme.css"),
                final_url: "https://cdn.test/main.css".into(),
                revision: "root:1".into(),
                content_type: "text/css".into(),
                bytes: b"@import './child.css' screen; p{background:url(icon)}".to_vec(),
            },
            CssSnapshotSource {
                requested_url: "https://cdn.test/child.css".into(),
                final_url: "https://cdn.test/child.css".into(),
                revision: "child:1".into(),
                content_type: "text/css".into(),
                bytes: b"p{background:url(./image.svg)}".to_vec(),
            },
        ],
        Default::default(),
    )
    .unwrap()
}
