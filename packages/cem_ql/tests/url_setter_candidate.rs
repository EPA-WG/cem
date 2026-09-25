//! Explicit pinned WPT JSON test-data boundary; no production setters.
use serde::Deserialize;
use std::collections::BTreeMap;
use url_unpatched::{quirks, Url};

#[derive(Deserialize)]
struct Case {
    href: String,
    new_value: String,
    expected: BTreeMap<String, String>,
}

fn apply(url: &mut Url, field: &str, value: &str) -> Result<(), ()> {
    match field {
        "protocol" => quirks::set_protocol(url, value),
        "username" => quirks::set_username(url, value),
        "password" => quirks::set_password(url, value),
        "host" => quirks::set_host(url, value),
        "hostname" => quirks::set_hostname(url, value),
        "port" => quirks::set_port(url, value),
        "pathname" => {
            quirks::set_pathname(url, value);
            Ok(())
        }
        "search" => {
            quirks::set_search(url, value);
            Ok(())
        }
        "hash" => {
            quirks::set_hash(url, value);
            Ok(())
        }
        _ => panic!("unsupported setter {field}"),
    }
}
fn get(url: &Url, field: &str) -> String {
    match field {
        "href" => quirks::href(url),
        "protocol" => quirks::protocol(url),
        "username" => quirks::username(url),
        "password" => quirks::password(url),
        "host" => quirks::host(url),
        "hostname" => quirks::hostname(url),
        "port" => quirks::port(url),
        "pathname" => quirks::pathname(url),
        "search" => quirks::search(url),
        "hash" => quirks::hash(url),
        _ => panic!("unsupported getter {field}"),
    }
    .to_owned()
}

fn candidate_differences() -> (usize, usize, Vec<String>) {
    // Select component arrays only; href assignment is not the CEM parts API.
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/url/wpt-url-setters.json")).unwrap();
    let mut differences = Vec::new();
    let mut count = 0;
    let mut failed = 0;
    for setter in [
        "protocol", "username", "password", "host", "hostname", "port", "pathname", "search",
        "hash",
    ] {
        let cases: Vec<Case> = serde_json::from_value(fixture[setter].clone()).unwrap();
        count += cases.len();
        for (index, case) in cases.into_iter().enumerate() {
            let mut url = Url::parse(&case.href).unwrap();
            let _outcome = apply(&mut url, setter, &case.new_value);
            let before = differences.len();
            for (field, expected) in case.expected {
                let actual = get(&url, &field);
                if actual != expected {
                    differences.push(format!(
                        "{setter}[{index}] {field}: expected {expected:?}, received {actual:?}"
                    ));
                }
            }
            if differences.len() != before {
                failed += 1;
            }
        }
    }
    (count, failed, differences)
}

#[test]
fn pinned_component_setters_record_candidate_differences() {
    let (count, failed, differences) = candidate_differences();
    assert_eq!((count, failed), (277, 22));
    // Raw characterization remains unchanged, not a conformance waiver.
    assert_eq!(
        differences.join("\n"),
        include_str!("../fixtures/url/url-2.5.8-setter-differences.txt").trim_end()
    );
}

#[test]
fn partial_host_effect_succeeds_without_reporting_invalid_port() {
    let mut url = Url::parse("https://example.test:8443/a").unwrap();
    assert_eq!(quirks::set_host(&mut url, "other.test:70000"), Ok(()));
    assert_eq!(url.as_str(), "https://other.test:8443/a");
    // Required host.port warning cannot be inferred from this success return.
}

#[test]
fn equal_normalized_prefix_and_ignored_setters_have_distinct_outcomes() {
    for (field, value, expected, accepted) in [
        (
            "hostname",
            "EXAMPLE.test",
            "https://example.test:8443/a",
            true,
        ),
        ("port", "8443", "https://example.test:8443/a", true),
        ("port", "123abc", "https://example.test:123/a", true),
        ("port", "70000", "https://example.test:8443/a", false),
        ("protocol", "mailto:", "https://example.test:8443/a", false),
        ("host", "other.test", "https://other.test:8443/a", true),
    ] {
        let mut url = Url::parse("https://example.test:8443/a").unwrap();
        assert_eq!(
            apply(&mut url, field, value).is_ok(),
            accepted,
            "{field} = {value}"
        );
        assert_eq!(url.as_str(), expected);
    }
    let mut opaque = Url::parse("mailto:user@example.test").unwrap();
    quirks::set_pathname(&mut opaque, "/replacement");
    assert_eq!(opaque.as_str(), "mailto:user@example.test");
    // The void pathname setter gives no signal for this inapplicable update.
}

#[test]
fn file_protocol_candidate_rejects_equal_and_eligible_assignments() {
    // Additional authored characterization, outside the 277 pinned WPT cases.
    // D2 forbids treating an equal valid assignment as a rejected setter.
    for seed in ["file://host/a", "https://example.test/a"] {
        let mut url = Url::parse(seed).unwrap();
        assert_eq!(quirks::set_protocol(&mut url, "file:"), Err(()));
        assert_eq!(url.as_str(), seed);
    }
}
