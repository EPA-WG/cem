//! Explicit pinned-WPT JSON test-data boundary; no query/runtime registration.
use serde::Deserialize;
use url::{quirks, Url};
use url_unpatched as url;

#[derive(Deserialize)]
struct Fixture {
    upstream_index: usize,
    case: Case,
}
#[derive(Deserialize)]
struct Case {
    input: String,
    base: Option<String>,
    #[serde(default)]
    failure: bool,
    href: Option<String>,
    origin: Option<String>,
    protocol: Option<String>,
    username: Option<String>,
    password: Option<String>,
    host: Option<String>,
    hostname: Option<String>,
    port: Option<String>,
    pathname: Option<String>,
    search: Option<String>,
    hash: Option<String>,
}
fn candidate(input: &str, base: Option<&str>) -> Result<Url, url::ParseError> {
    // The CEM contract validates a supplied base even for absolute input.
    let base = base.map(Url::parse).transpose()?;
    Url::options().base_url(base.as_ref()).parse(input)
}

#[test]
fn selected_wpt_records_pinned_candidate_gaps() {
    let fixtures: Vec<Fixture> =
        serde_json::from_str(include_str!("../fixtures/url/wpt-url-parse.json")).unwrap();
    assert_eq!(fixtures.len(), 51);
    let mut differences = Vec::new();
    for Fixture {
        upstream_index: index,
        case,
    } in fixtures
    {
        let result = candidate(&case.input, case.base.as_deref());
        if case.failure {
            if result.is_ok() {
                differences.push(format!("{index}: unexpectedly accepted"));
            }
            continue;
        }
        let Ok(parsed) = result else {
            differences.push(format!("{index}: unexpectedly rejected"));
            continue;
        };
        // Successful native serialization must be an idempotent absolute seed.
        assert_eq!(
            candidate(parsed.as_str(), None).unwrap().as_str(),
            parsed.as_str()
        );
        let origin = quirks::origin(&parsed);
        for (field, expected, actual) in [
            ("href", case.href.as_deref(), quirks::href(&parsed)),
            ("origin", case.origin.as_deref(), origin.as_str()),
            (
                "protocol",
                case.protocol.as_deref(),
                quirks::protocol(&parsed),
            ),
            (
                "username",
                case.username.as_deref(),
                quirks::username(&parsed),
            ),
            (
                "password",
                case.password.as_deref(),
                quirks::password(&parsed),
            ),
            ("host", case.host.as_deref(), quirks::host(&parsed)),
            (
                "hostname",
                case.hostname.as_deref(),
                quirks::hostname(&parsed),
            ),
            ("port", case.port.as_deref(), quirks::port(&parsed)),
            (
                "pathname",
                case.pathname.as_deref(),
                quirks::pathname(&parsed),
            ),
            ("search", case.search.as_deref(), quirks::search(&parsed)),
            ("hash", case.hash.as_deref(), quirks::hash(&parsed)),
        ] {
            if let Some(expected) = expected {
                if actual != expected {
                    differences.push(format!(
                        "{index}: {field}: expected {expected:?}, received {actual:?}"
                    ));
                }
            }
        }
    }
    // Characterization only: the upstream expectations remain authoritative.
    // These gaps must be resolved before adopting the candidate in production.
    assert_eq!(
        differences.join("\n"),
        include_str!("../fixtures/url/url-2.5.8-differences.txt").trim_end(),
        "candidate behavior changed; review against the unchanged WPT expectations"
    );
}

#[test]
fn explicit_bases_delimiters_and_query_decoding() {
    assert!(candidate("https://example.test/", Some("not a base")).is_err());
    assert!(candidate("relative", None).is_err());
    assert!(candidate("child", Some("mailto:user@example.test")).is_err());
    assert_eq!(
        candidate("../b", Some("https://example.test/a/c"))
            .unwrap()
            .as_str(),
        "https://example.test/b"
    );
    let parsed = candidate("https://example.test/?#", None).unwrap();
    assert_eq!(parsed.as_str(), "https://example.test/?#");
    assert_eq!(quirks::search(&parsed), "");
    assert_eq!(quirks::hash(&parsed), "");
    let parsed = candidate("https://example.test/?x=+&x=%2B&bare", None).unwrap();
    assert_eq!(
        parsed.query_pairs().into_owned().collect::<Vec<_>>(),
        vec![
            ("x".into(), " ".into()),
            ("x".into(), "+".into()),
            ("bare".into(), "".into())
        ]
    );
}

#[test]
fn candidate_file_and_non_blob_origins_match_the_pure_profile() {
    for (input, expected) in [
        ("file:///tmp/a", "null"),
        ("file://host/a", "null"),
        ("https://example.test:443/a", "https://example.test"),
        ("ftp://example.test:21/a", "ftp://example.test"),
        ("mailto:user@example.test", "null"),
        ("custom://example.test/a", "null"),
    ] {
        assert_eq!(quirks::origin(&candidate(input, None).unwrap()), expected);
    }
}
