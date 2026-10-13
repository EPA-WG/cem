use cem_ml::{
    html_base::*, html_base_evidence::*, html_base_host::*, parser::tree::RetainedCemTree,
};
use std::{cell::Cell, sync::Arc};

fn tree(href: &str) -> Arc<RetainedCemTree> {
    cem_ml::import::import_data_bytes(
        format!(r#"<html xmlns="http://www.w3.org/1999/xhtml"><head><base href="{href}"/></head><body><a href="next">Next</a></body></html>"#).as_bytes(),
        "application/xml", "cem", "studio://source/host.xhtml",
    ).unwrap()
}
fn host(nodes: usize) -> HtmlBaseExecutionHost {
    HtmlBaseExecutionHost::new(
        HtmlBaseCompatibilityProfile::StandardAgreement,
        HtmlBaseBrowser {
            engine: "fixture".into(),
            version: "1".into(),
        },
        HtmlBaseLimits {
            nodes,
            url_bytes: 4096,
        },
    )
    .unwrap()
}
fn context() -> HtmlBaseContext {
    HtmlBaseContext {
        fallback: HtmlFallbackBase::DocumentUrl("https://example.test/a/page".into()),
        encoding: HtmlBaseEncoding::Utf8,
        policy_stamp: "allow:1".into(),
    }
}
fn slots(slot: HtmlBaseDomSlot<'_>) -> Result<u32, String> {
    Ok(slot.id + 1)
}
fn allow(_: &HtmlBasePolicyRequest<'_>) -> Result<HtmlBasePolicyDecision, String> {
    Ok(HtmlBasePolicyDecision::Allow)
}

#[test]
fn shared_browser_snapshot_retains_native_selection_and_fallback() {
    let source = include_str!("fixtures/html-base-host.xhtml").trim();
    let tree = cem_ml::import::import_data_bytes(
        source.as_bytes(),
        "application/xml",
        "cem",
        "studio://source/host.xhtml",
    )
    .unwrap();
    let mut host = host(100);
    host.prepare(tree, context(), slots, allow).unwrap();
    let evidence = host
        .confirm("https://example.test/a/child/".into())
        .unwrap();
    assert_eq!(
        evidence
            .native_resolution()
            .selected
            .as_ref()
            .unwrap()
            .authored_href,
        "child/"
    );
    host.validate(&evidence, slots, evidence.effective_url())
        .unwrap();
}

#[test]
fn host_owns_epochs_slots_source_and_read_sets() {
    let tree = tree("child/");
    let mut host = host(100);
    let handle = host
        .prepare(tree.clone(), context(), slots, allow)
        .unwrap()
        .unwrap();
    let first = host
        .confirm("https://example.test/a/child/".into())
        .unwrap();
    assert_eq!(
        handle,
        first
            .native_resolution()
            .selected
            .as_ref()
            .unwrap()
            .attribute
            + 1
    );
    assert!(Arc::ptr_eq(&tree, &first.native_resolution().tree));
    assert!(first
        .native_resolution()
        .selected
        .as_ref()
        .unwrap()
        .source_key
        .is_some());
    host.validate(&first, slots, first.effective_url()).unwrap();
    host.invalidate().unwrap();
    assert!(host.validate(&first, slots, first.effective_url()).is_err());
    host.prepare(tree, context(), slots, allow).unwrap();
    let second = host.confirm(first.effective_url().into()).unwrap();
    assert_eq!(second.observation().binding.epoch, 2);
    assert_eq!(second.observation().binding.revision, "3");
    assert!(host.validate(&first, slots, first.effective_url()).is_err());
    host.validate(&second, slots, second.effective_url())
        .unwrap();
}

#[test]
fn correspondence_failures_and_bounds_precede_policy_and_revoke() {
    for mode in 0..4 {
        let mut host = host(if mode == 3 { 2 } else { 100 });
        let calls = Cell::new(0);
        let result = host.prepare(
            tree("child/"),
            context(),
            |slot| match mode {
                0 => Err("DOM topology mismatch".into()),
                1 => Ok(0),
                2 => Ok(1), // aliased DOM handles cannot represent distinct native slots
                _ => slots(slot),
            },
            |_| {
                calls.set(calls.get() + 1);
                Ok(HtmlBasePolicyDecision::Allow)
            },
        );
        assert!(result.is_err());
        assert_eq!(calls.get(), 0);
        assert!(host
            .prepare(tree("child/"), context(), slots, allow)
            .is_err());
    }
}

#[test]
fn changed_dom_identity_or_value_revokes_even_with_the_same_base_url() {
    for identity_change in [true, false] {
        let mut host = host(100);
        host.prepare(tree("child/"), context(), slots, allow)
            .unwrap();
        let evidence = host
            .confirm("https://example.test/a/child/".into())
            .unwrap();
        assert!(host
            .validate(
                &evidence,
                |slot| {
                    if identity_change {
                        Ok(slot.id + 100)
                    } else {
                        Err("changed body attribute".into())
                    }
                },
                evidence.effective_url()
            )
            .is_err());
        assert!(host
            .prepare(tree("child/"), context(), slots, allow)
            .is_err());
    }
}

#[test]
fn foreign_pending_failed_and_revoked_sessions_never_admit() {
    let mut one = host(100);
    let mut two = host(100);
    one.prepare(tree("child/"), context(), slots, allow)
        .unwrap();
    let evidence = one.confirm("https://example.test/a/child/".into()).unwrap();
    two.prepare(tree("child/"), context(), slots, allow)
        .unwrap();
    let foreign = two.confirm(evidence.effective_url().into()).unwrap();
    assert_ne!(
        evidence.observation().binding.document_id,
        foreign.observation().binding.document_id
    );
    assert!(one
        .validate(&foreign, slots, foreign.effective_url())
        .is_err());
    assert!(two.confirm(foreign.effective_url().into()).is_err());
    let mut pending = host(100);
    pending
        .prepare(tree("child/"), context(), slots, allow)
        .unwrap();
    assert!(pending
        .prepare(tree("child/"), context(), slots, allow)
        .is_err());
    assert!(pending.confirm(evidence.effective_url().into()).is_err());
    let mut failed = host(100);
    failed
        .prepare(tree("child/"), context(), slots, allow)
        .unwrap();
    assert!(failed.confirm("https://wrong.test/".into()).is_err());
    assert!(failed
        .prepare(tree("child/"), context(), slots, allow)
        .is_err());
    one.revoke();
    assert!(one
        .validate(&evidence, slots, evidence.effective_url())
        .is_err());
}
