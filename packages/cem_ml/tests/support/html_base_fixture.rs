// Explicit test-control JSON boundary. Document bytes still enter native import;
// no AST is serialized or handed to JavaScript.
use cem_ml::html_base::*;
use cem_ml::html_base_evidence::*;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub name: String,
    pub markup: String,
    pub native_snapshot: Option<String>,
    pub context: String,
    pub url: String,
    pub about_base_url: Option<String>,
    pub policy: String,
    pub expected: String,
    pub outcome: String,
    pub selected_href: Option<String>,
    pub chromium_divergence: Option<ChromiumDivergence>,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChromiumDivergence {
    version: String,
    effective_url: String,
    reason: String,
}

pub fn run_cases() -> Vec<serde_json::Value> {
    run_observed_cases(None)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserObservation {
    pub name: String,
    pub base_uri: String,
    pub selected_href: Option<String>,
    pub engine: String,
    pub version: String,
}

pub fn run_observed_cases(observed: Option<&[BrowserObservation]>) -> Vec<serde_json::Value> {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../fixtures/html-base.json")).unwrap();
    if let Some(observed) = observed {
        assert_eq!(observed.len(), cases.len());
    }
    cases
        .into_iter()
        .enumerate()
        .map(|(index, case)| {
            // Well-formed, DOM-equivalent fixture snapshots use existing native XML
            // import, then explicitly opt into HTML execution semantics. The inert
            // template case supplies document children separately from its contents.
            let source = case.native_snapshot.as_deref().unwrap_or(&case.markup);
            let tree = cem_ml::import::import_data_bytes(
                source.as_bytes(),
                "application/xml",
                "cem",
                "studio://fixture/snapshot.xhtml",
            )
            .unwrap();
            let fallback = match case.context.as_str() {
                "document" => HtmlFallbackBase::DocumentUrl(case.url),
                "srcdoc" => HtmlFallbackBase::Srcdoc {
                    about_base_url: case.about_base_url.unwrap(),
                },
                "blank" => HtmlFallbackBase::AboutBlank {
                    document_url: case.url,
                    about_base_url: case.about_base_url,
                },
                other => panic!("unknown fixture context {other}"),
            };
            let mut policy_calls = 0;
            let prepared = prepare_html_base_freeze(
                HtmlBaseFreezeRequest {
                    tree,
                    context: HtmlBaseContext {
                        fallback,
                        encoding: HtmlBaseEncoding::Utf8,
                        policy_stamp: format!("fixture:{}", case.policy),
                    },
                    binding: HtmlBaseEpoch {
                        document_id: format!("fixture:{}", case.name),
                        revision: "1".into(),
                        epoch: 1,
                        kind: if case.selected_href.is_some() {
                            HtmlBaseEpochKind::HrefFreeze
                        } else {
                            HtmlBaseEpochKind::FallbackSnapshot
                        },
                    },
                    profile: HtmlBaseCompatibilityProfile::Chromium148,
                    limits: HtmlBaseLimits {
                        nodes: 1000,
                        url_bytes: 4096,
                    },
                },
                |_| {
                    policy_calls += 1;
                    Ok(match case.policy.as_str() {
                        "allow" => HtmlBasePolicyDecision::Allow,
                        "deny" => HtmlBasePolicyDecision::Deny,
                        other => panic!("unknown fixture policy {other}"),
                    })
                },
            )
            .unwrap_or_else(|error| panic!("{}: {error}", case.name));
            let result = prepared.native_resolution();
            assert_eq!(result.effective_url, case.expected, "{}", case.name);
            assert_eq!(
                format!("{:?}", result.outcome),
                case.outcome,
                "{}",
                case.name
            );
            assert_eq!(
                result.selected.as_ref().map(|s| &s.authored_href),
                case.selected_href.as_ref(),
                "{}",
                case.name
            );
            assert_eq!(
                policy_calls,
                usize::from(matches!(
                    result.outcome,
                    HtmlBaseOutcome::Resolved | HtmlBaseOutcome::PolicyDenied
                )),
                "{}",
                case.name
            );
            let expected_execution = case
                .chromium_divergence
                .as_ref()
                .map_or(result.effective_url.as_str(), |difference| {
                    difference.effective_url.as_str()
                });
            let browser_observed = observed.map(|all| &all[index]);
            if let Some(observed) = browser_observed {
                assert_eq!(observed.name, case.name);
                assert_eq!(observed.selected_href, case.selected_href);
            }
            let captured = prepared
                .confirm(HtmlBaseObservation {
                    binding: prepared.binding().clone(),
                    browser: HtmlBaseBrowser {
                        engine: browser_observed
                            .map_or("chromium", |o| o.engine.as_str())
                            .into(),
                        version: browser_observed
                            .map_or(CHROMIUM_BASE_PROFILE_VERSION, |o| o.version.as_str())
                            .into(),
                    },
                    selected_attribute: result.selected.as_ref().map(|s| s.attribute),
                    policy_stamp: result.context.policy_stamp.clone(),
                    base_uri: browser_observed
                        .map_or(expected_execution, |o| o.base_uri.as_str())
                        .into(),
                })
                .unwrap_or_else(|error| panic!("{}: {error}", case.name));
            assert_eq!(captured.effective_url(), expected_execution);
            serde_json::json!({
                "name": case.name, "effective_url": result.effective_url,
                "fallback_url": result.fallback_url, "outcome": case.outcome,
                "selected_href": case.selected_href, "policy_calls": policy_calls,
                "chromium_divergence": case.chromium_divergence,
                "execution_url": captured.effective_url(),
                "compatibility": format!("{:?}", captured.compatibility()),
            })
        })
        .collect()
}
