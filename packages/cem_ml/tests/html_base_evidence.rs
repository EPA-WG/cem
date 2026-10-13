use cem_ml::{html_base::*, html_base_evidence::*, parser::tree::RetainedCemTree};
use std::{cell::Cell, sync::Arc};

#[path = "support/html_base_freeze_trace.rs"]
mod trace;

#[test]
fn html_base_evidence_freeze_transition_fixture() {
    assert_eq!(trace::verify_trace(&trace::fixture_observations()).len(), 8);
}

fn tree(href: Option<&str>) -> Arc<RetainedCemTree> {
    let base = href
        .map(|h| format!(r#"<base href="{h}"/>"#))
        .unwrap_or_default();
    cem_ml::import::import_data_bytes(
        format!(r#"<html xmlns="http://www.w3.org/1999/xhtml"><head>{base}</head></html>"#)
            .as_bytes(),
        "application/xml",
        "cem",
        "studio://source/snapshot.xhtml",
    )
    .unwrap()
}
fn request(
    tree: Arc<RetainedCemTree>,
    profile: HtmlBaseCompatibilityProfile,
) -> HtmlBaseFreezeRequest {
    HtmlBaseFreezeRequest {
        tree,
        context: HtmlBaseContext {
            fallback: HtmlFallbackBase::DocumentUrl("https://example.test/a/page.html#old".into()),
            encoding: HtmlBaseEncoding::Utf8,
            policy_stamp: "policy:allow:1".into(),
        },
        binding: HtmlBaseEpoch {
            document_id: "document:1".into(),
            revision: "revision:1".into(),
            epoch: 1,
            kind: HtmlBaseEpochKind::HrefFreeze,
        },
        profile,
        limits: HtmlBaseLimits {
            nodes: 100,
            url_bytes: 4096,
        },
    }
}
fn observation(prepared: &PreparedHtmlBaseFreeze, uri: &str) -> HtmlBaseObservation {
    HtmlBaseObservation {
        binding: prepared.binding().clone(),
        browser: HtmlBaseBrowser {
            engine: "chromium".into(),
            version: CHROMIUM_BASE_PROFILE_VERSION.into(),
        },
        selected_attribute: prepared
            .native_resolution()
            .selected
            .as_ref()
            .map(|s| s.attribute),
        policy_stamp: prepared.native_resolution().context.policy_stamp.clone(),
        base_uri: uri.into(),
    }
}
fn prepare(href: Option<&str>) -> PreparedHtmlBaseFreeze {
    prepare_html_base_freeze(
        request(tree(href), HtmlBaseCompatibilityProfile::StandardAgreement),
        |_| Ok(HtmlBasePolicyDecision::Allow),
    )
    .unwrap()
}

#[test]
fn html_base_evidence_captures_policy_once_and_retains_original_native_owner() {
    let retained = tree(Some("child/"));
    let calls = Cell::new(0);
    let prepared = prepare_html_base_freeze(
        request(
            retained.clone(),
            HtmlBaseCompatibilityProfile::StandardAgreement,
        ),
        |query| {
            calls.set(calls.get() + 1);
            assert_eq!(
                query.fallback_url.as_str(),
                "https://example.test/a/page.html#old"
            );
            assert_eq!(query.policy_stamp, "policy:allow:1");
            Ok(HtmlBasePolicyDecision::Allow)
        },
    )
    .unwrap();
    let observed = observation(&prepared, "https://example.test/a/child/");
    let evidence = prepared.confirm(observed.clone()).unwrap();
    evidence
        .validate_current(&retained, &observed, "policy:allow:1")
        .unwrap();
    assert_eq!(calls.get(), 1);
    assert!(Arc::ptr_eq(&evidence.native_resolution().tree, &retained));
    assert!(evidence
        .native_resolution()
        .selected
        .as_ref()
        .unwrap()
        .source_key
        .is_some());
    assert_eq!(
        evidence.native_resolution().fallback_url,
        "https://example.test/a/page.html#old"
    );
    assert_eq!(
        evidence.policy_decision(),
        Some(HtmlBasePolicyDecision::Allow)
    );
    assert!(evidence.body_analysis_required());
    assert!(!evidence.blockers().is_empty());
}

#[test]
fn html_base_evidence_chromium_differences_require_explicit_profile_and_exact_observation() {
    for (href, standard, browser, compatibility) in [
        (
            "http://[",
            "https://example.test/a/page.html#old",
            "about:blank",
            HtmlBaseCompatibility::ChromiumInvalidHref,
        ),
        (
            "",
            "https://example.test/a/page.html",
            "https://example.test/a/page.html#old",
            HtmlBaseCompatibility::ChromiumEmptyHref,
        ),
    ] {
        let prepared = prepare(Some(href));
        assert!(prepared.confirm(observation(&prepared, browser)).is_err());
        let prepared = prepare_html_base_freeze(
            request(tree(Some(href)), HtmlBaseCompatibilityProfile::Chromium148),
            |_| Ok(HtmlBasePolicyDecision::Allow),
        )
        .unwrap();
        assert!(prepared
            .confirm(observation(&prepared, "https://unexpected.test/"))
            .is_err());
        let evidence = prepared.confirm(observation(&prepared, browser)).unwrap();
        assert_eq!(evidence.native_resolution().effective_url, standard);
        assert_eq!(evidence.effective_url(), browser);
        assert_eq!(evidence.compatibility(), compatibility);
    }
}

#[test]
fn html_base_evidence_rejects_stale_read_sets_including_same_value_new_epoch() {
    let prepared = prepare(Some("child/"));
    let observed = observation(&prepared, "https://example.test/a/child/");
    let evidence = prepared.confirm(observed.clone()).unwrap();
    let retained = &evidence.native_resolution().tree;
    for drift in 0..7 {
        let mut current = observed.clone();
        match drift {
            0 => current.binding.document_id = "document:2".into(),
            1 => current.binding.revision = "revision:2".into(),
            2 => current.binding.epoch += 1,
            3 => current.selected_attribute = None,
            4 => current.browser.version = "new-browser".into(),
            5 => current.base_uri = "https://elsewhere.test/".into(),
            _ => current.binding.kind = HtmlBaseEpochKind::FallbackSnapshot,
        }
        assert!(evidence
            .validate_current(retained, &current, "policy:allow:1")
            .is_err());
    }
    assert!(evidence
        .validate_current(retained, &observed, "policy:deny:2")
        .is_err());
    assert!(evidence
        .validate_current(&tree(Some("child/")), &observed, "policy:allow:1")
        .is_err());
}

#[test]
fn html_base_evidence_confirmation_binds_browser_identity_epoch_and_selected_slot() {
    let prepared = prepare_html_base_freeze(
        request(tree(Some("")), HtmlBaseCompatibilityProfile::Chromium148),
        |_| Ok(HtmlBasePolicyDecision::Allow),
    )
    .unwrap();
    let observed = observation(&prepared, "https://example.test/a/page.html#old");
    for drift in 0..7 {
        let mut current = observed.clone();
        match drift {
            0 => current.browser.engine = "other-engine".into(),
            1 => current.browser.version = "149.0.0.0".into(),
            2 => current.binding.epoch += 1,
            3 => current.selected_attribute = Some(0),
            4 => current.base_uri = "relative/".into(),
            5 => current.policy_stamp = "different-policy".into(),
            _ => current.browser.version.clear(),
        }
        assert!(prepared.confirm(current).is_err());
    }
}

#[test]
fn html_base_evidence_captures_denial_and_requires_real_freeze_or_fallback_kind() {
    let mut input = request(
        tree(Some("https://cdn.test/")),
        HtmlBaseCompatibilityProfile::StandardAgreement,
    );
    input.context.policy_stamp = "policy:deny:2".into();
    let prepared = prepare_html_base_freeze(input, |_| Ok(HtmlBasePolicyDecision::Deny)).unwrap();
    let evidence = prepared
        .confirm(observation(
            &prepared,
            "https://example.test/a/page.html#old",
        ))
        .unwrap();
    assert_eq!(
        evidence.policy_decision(),
        Some(HtmlBasePolicyDecision::Deny)
    );
    assert_eq!(
        evidence.native_resolution().context.policy_stamp,
        "policy:deny:2"
    );
    assert!(prepare_html_base_freeze(
        request(tree(None), HtmlBaseCompatibilityProfile::StandardAgreement),
        |_| panic!("no base")
    )
    .is_err());
    let mut input = request(tree(None), HtmlBaseCompatibilityProfile::StandardAgreement);
    input.binding.kind = HtmlBaseEpochKind::FallbackSnapshot;
    let prepared = prepare_html_base_freeze(input, |_| panic!("no base")).unwrap();
    let evidence = prepared
        .confirm(observation(
            &prepared,
            "https://example.test/a/page.html#old",
        ))
        .unwrap();
    assert_eq!(evidence.policy_decision(), None);
}

#[test]
fn html_base_evidence_missing_context_policy_and_bounded_work_fail_closed() {
    for drift in 0..6 {
        let mut input = request(
            tree(Some("child/")),
            HtmlBaseCompatibilityProfile::StandardAgreement,
        );
        match drift {
            0 => input.binding.document_id.clear(),
            1 => input.binding.revision.clear(),
            2 => input.binding.epoch = 0,
            3 => input.limits.nodes = 1,
            4 => input.context.policy_stamp.clear(),
            _ => input.binding.kind = HtmlBaseEpochKind::FallbackSnapshot,
        }
        assert!(prepare_html_base_freeze(input, |_| panic!("invalid inputs")).is_err());
    }
    assert!(prepare_html_base_freeze(
        request(
            tree(Some("child/")),
            HtmlBaseCompatibilityProfile::StandardAgreement
        ),
        |_| Err("unknown policy".into())
    )
    .is_err());
}
