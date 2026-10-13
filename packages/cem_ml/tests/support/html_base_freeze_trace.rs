// Named fixture-control boundary: immutable source bytes and scalar execution
// observations. Native import owns document semantics; no browser DOM AST crosses.
use cem_ml::{html_base::*, html_base_evidence::*};
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    name: String,
    action: String,
    url: String,
    href: Option<String>,
    fallback: String,
    policy: String,
    epoch: u64,
    revision: String,
    capture: bool,
    expected: String,
    browser_expected: Option<String>,
    confirmation_expected: bool,
    markup: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceObservation {
    name: String,
    url: String,
    selected_href: Option<String>,
    base_uri: String,
    freeze_fallback: String,
    freeze_policy: String,
    current_policy: String,
    epoch: u64,
    revision: String,
    engine: String,
    version: String,
}

fn cases() -> Vec<Case> {
    serde_json::from_str(include_str!("../fixtures/html-base-freezes.json")).unwrap()
}

#[cfg(test)]
pub fn fixture_observations() -> Vec<TraceObservation> {
    let mut freeze_policy = String::new();
    cases()
        .into_iter()
        .map(|case| {
            if case.capture {
                freeze_policy = case.policy.clone();
            }
            TraceObservation {
                name: case.name,
                url: case.url,
                selected_href: case.href,
                base_uri: case.browser_expected.unwrap_or(case.expected),
                freeze_fallback: case.fallback,
                freeze_policy: freeze_policy.clone(),
                current_policy: case.policy,
                epoch: case.epoch,
                revision: case.revision,
                engine: "chromium".into(),
                version: CHROMIUM_BASE_PROFILE_VERSION.into(),
            }
        })
        .collect()
}

pub fn verify_trace(observed: &[TraceObservation]) -> Vec<serde_json::Value> {
    let cases = cases();
    assert_eq!(observed.len(), cases.len());
    let mut evidence: Option<CapturedHtmlBaseEvidence> = None;
    let mut results = Vec::new();
    let mut policy_calls = 0;
    for (case, observed) in cases.into_iter().zip(observed) {
        assert!(matches!(
            case.action.as_str(),
            "load" | "history" | "href" | "policy" | "remove"
        ));
        assert_eq!(observed.name, case.name);
        assert_eq!(observed.url, case.url);
        assert_eq!(observed.selected_href, case.href);
        assert_eq!(
            observed.base_uri,
            case.browser_expected
                .as_ref()
                .unwrap_or(&case.expected)
                .as_str(),
            "{}",
            case.name
        );
        assert_eq!(observed.freeze_fallback, case.fallback);
        assert_eq!(observed.current_policy, case.policy);
        assert_eq!(observed.epoch, case.epoch);
        assert_eq!(observed.revision, case.revision);
        let binding = HtmlBaseEpoch {
            document_id: "fixture:live-document".into(),
            revision: observed.revision.clone(),
            epoch: observed.epoch,
            kind: if observed.selected_href.is_some() {
                HtmlBaseEpochKind::HrefFreeze
            } else {
                HtmlBaseEpochKind::FallbackSnapshot
            },
        };
        let browser = HtmlBaseBrowser {
            engine: observed.engine.clone(),
            version: observed.version.clone(),
        };
        if case.capture {
            assert_eq!(observed.freeze_policy, observed.current_policy);
            let tree = cem_ml::import::import_data_bytes(
                case.markup.as_bytes(),
                "application/xml",
                "cem",
                "studio://fixture/live.xhtml",
            )
            .unwrap();
            let prepared = prepare_html_base_freeze(
                HtmlBaseFreezeRequest {
                    tree,
                    context: HtmlBaseContext {
                        fallback: HtmlFallbackBase::DocumentUrl(observed.freeze_fallback.clone()),
                        encoding: HtmlBaseEncoding::Utf8,
                        policy_stamp: observed.freeze_policy.clone(),
                    },
                    binding: binding.clone(),
                    profile: HtmlBaseCompatibilityProfile::Chromium148,
                    limits: HtmlBaseLimits {
                        nodes: 100,
                        url_bytes: 4096,
                    },
                },
                |query| {
                    policy_calls += 1;
                    assert_eq!(query.policy_stamp, observed.freeze_policy);
                    Ok(match observed.freeze_policy.as_str() {
                        "allow:1" => HtmlBasePolicyDecision::Allow,
                        "deny:2" => HtmlBasePolicyDecision::Deny,
                        _ => panic!("unknown fixture policy"),
                    })
                },
            )
            .unwrap();
            let selected_attribute = prepared
                .native_resolution()
                .selected
                .as_ref()
                .map(|s| s.attribute);
            assert_eq!(prepared.native_resolution().effective_url, case.expected);
            let confirmation = prepared.confirm(HtmlBaseObservation {
                binding: binding.clone(),
                browser: browser.clone(),
                selected_attribute,
                policy_stamp: observed.freeze_policy.clone(),
                base_uri: observed.base_uri.clone(),
            });
            if !case.confirmation_expected {
                let error = confirmation.unwrap_err();
                assert!(error.contains("disagrees with the selected compatibility profile"));
                results.push(serde_json::json!({
                    "name": case.name, "captured": false, "capture_attempted": true,
                    "read_set_valid": false, "effective_url": null,
                    "native_url": prepared.native_resolution().effective_url,
                    "observed_url": observed.base_uri, "rejection": error,
                }));
                continue;
            }
            evidence = Some(confirmation.unwrap());
        }
        let captured = evidence.as_ref().unwrap();
        // History and policy changes do not overwrite captured freeze inputs.
        assert_eq!(
            captured.native_resolution().fallback_url,
            observed.freeze_fallback
        );
        assert_eq!(
            captured.native_resolution().context.policy_stamp,
            observed.freeze_policy
        );
        let current = HtmlBaseObservation {
            binding,
            browser,
            selected_attribute: captured.observation().selected_attribute,
            policy_stamp: observed.freeze_policy.clone(),
            base_uri: observed.base_uri.clone(),
        };
        let valid = captured
            .validate_current(
                &captured.native_resolution().tree,
                &current,
                &observed.current_policy,
            )
            .is_ok();
        assert_eq!(valid, case.capture, "{}: current admission", case.name);
        results.push(serde_json::json!({
            "name": case.name, "captured": case.capture, "capture_attempted": case.capture, "read_set_valid": valid,
            "effective_url": captured.effective_url(), "freeze_fallback": observed.freeze_fallback,
            "freeze_policy": observed.freeze_policy, "current_policy": observed.current_policy,
            "epoch": observed.epoch,
        }));
    }
    assert_eq!(policy_calls, 4);
    results
}
