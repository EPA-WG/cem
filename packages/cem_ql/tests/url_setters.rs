use cem_ql::stdlib::url_setters::{set_pathname, set_port, SetterOutcome};
use serde::Deserialize;
use std::collections::BTreeMap;
use url::{quirks, Url};

#[derive(Deserialize)]
struct Case {
    href: String,
    new_value: String,
    expected: BTreeMap<String, String>,
}

#[test]
fn pinned_ports_and_hostless_paths_match_unchanged_wpt() {
    // Explicit WPT JSON test-data boundary; no runtime document import.
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/url/wpt-url-setters.json")).unwrap();
    for field in ["port", "pathname"] {
        let cases: Vec<Case> = serde_json::from_value(fixture[field].clone()).unwrap();
        for (index, case) in cases.iter().enumerate() {
            if field == "pathname" && !(24..=27).contains(&index) {
                continue;
            }
            let seed = Url::parse(&case.href).unwrap();
            let mut updated = seed.clone();
            if field == "port" {
                set_port(&mut updated, &case.new_value);
            } else {
                set_pathname(&mut updated, &case.new_value);
            }
            for (key, expected) in &case.expected {
                let actual = match key.as_str() {
                    "href" => updated.as_str(),
                    "port" => quirks::port(&updated),
                    "pathname" => updated.path(),
                    "host" => quirks::host(&updated),
                    "hostname" => quirks::hostname(&updated),
                    _ => panic!("unexpected WPT field {key}"),
                };
                assert_eq!(actual, expected, "{field}[{index}] {key}");
            }
            let reparsed = Url::parse(updated.as_str()).unwrap();
            assert_eq!(reparsed.host(), updated.host());
            assert_eq!(reparsed.path(), updated.path());
            assert_eq!(reparsed.as_str(), updated.as_str());
            assert_eq!(seed, Url::parse(&case.href).unwrap());
        }
    }
}

#[test]
fn port_outcome_distinguishes_empty_filtered_empty_equal_prefix_and_invalid() {
    for (value, expected, outcome) in [
        ("", None, SetterOutcome::Applied),
        ("\n\t\r", Some(3000), SetterOutcome::Ignored("port")),
        ("3000", Some(3000), SetterOutcome::Applied),
        ("443", None, SetterOutcome::Applied),
        ("12\t3abc", Some(123), SetterOutcome::Applied),
        ("70000", Some(3000), SetterOutcome::Ignored("port")),
        ("abc", Some(3000), SetterOutcome::Ignored("port")),
        (" 123", Some(3000), SetterOutcome::Ignored("port")),
    ] {
        let mut url = Url::parse("https://example.test:3000/a").unwrap();
        assert_eq!(set_port(&mut url, value), outcome, "{value:?}");
        assert_eq!(url.port(), expected, "{value:?}");
    }
    for seed in [
        "file://host/a",
        "mailto:user@example.test",
        "custom:/a",
        "custom:///a",
    ] {
        let mut url = Url::parse(seed).unwrap();
        assert_eq!(set_port(&mut url, ""), SetterOutcome::Ignored("port"));
        assert_eq!(url.as_str(), seed);
    }
}

#[test]
fn hostless_path_guard_preserves_query_fragment_and_never_introduces_authority() {
    for seed in ["custom:/old?x=1#f", "custom:/.//old?x=1#f"] {
        let mut url = Url::parse(seed).unwrap();
        assert_eq!(
            set_pathname(&mut url, "//evil.test/a?#"),
            SetterOutcome::Applied
        );
        assert_eq!(url.as_str(), "custom:/.//evil.test/a%3F%23?x=1#f");
        assert_eq!(url.path(), "//evil.test/a%3F%23");
        assert_eq!(url.host(), None);
        let reparsed = Url::parse(url.as_str()).unwrap();
        assert_eq!(reparsed, url);
        set_pathname(&mut url, "new");
        assert_eq!(url.as_str(), "custom:/new?x=1#f");
    }
    let mut opaque = Url::parse("mailto:user@example.test").unwrap();
    assert_eq!(
        set_pathname(&mut opaque, "new"),
        SetterOutcome::Ignored("pathname")
    );
    assert_eq!(opaque.as_str(), "mailto:user@example.test");
}

#[test]
fn file_slash_segments_survive_parsing_setters_and_roundtrips() {
    for (seed, path, expected) in [
        ("file://monkey/", "\\\\", "file://monkey//"),
        ("file:///unicorn", "//\\/", "file://////"),
        ("file:///unicorn", "//monkey/..//", "file://///"),
    ] {
        let parsed = Url::parse(expected).unwrap();
        assert_eq!(parsed.as_str(), expected);
        let mut updated = Url::parse(seed).unwrap();
        assert_eq!(set_pathname(&mut updated, path), SetterOutcome::Applied);
        assert_eq!(updated.as_str(), expected);
        assert_eq!(Url::parse(updated.as_str()).unwrap(), updated);
    }
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

fn adapted_differences() -> (usize, usize, Vec<String>) {
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
            if setter == "port" {
                cem_ql::stdlib::url_setters::set_port(&mut url, &case.new_value);
            } else if setter == "pathname" {
                cem_ql::stdlib::url_setters::set_pathname(&mut url, &case.new_value);
            } else if setter == "host" {
                cem_ql::stdlib::url_setters::set_host(&mut url, &case.new_value);
            } else if setter == "protocol" {
                cem_ql::stdlib::url_setters::set_protocol(&mut url, &case.new_value);
            } else {
                let _outcome = apply(&mut url, setter, &case.new_value);
            }
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
fn patched_parser_and_adapters_leave_only_recorded_remaining_gaps() {
    let (count, failed, differences) = adapted_differences();
    assert_eq!((count, failed), (277, 6));
    let remaining = include_str!("../fixtures/url/url-2.5.8-setter-differences.txt")
        .lines()
        .filter(|line| {
            ![
                "port[26]",
                "search[10]",
                "search[11]",
                "search[12]",
                "search[13]",
                "hash[16]",
                "hash[17]",
                "hash[18]",
                "hash[19]",
                "pathname[21]",
                "pathname[22]",
                "pathname[23]",
                "pathname[24]",
                "pathname[25]",
                "pathname[26]",
                "pathname[27]",
            ]
            .iter()
            .any(|id| line.starts_with(id))
        })
        .collect::<Vec<_>>();
    assert_eq!(remaining.len(), 12);
    assert_eq!(differences, remaining);
}

#[test]
fn opaque_boundary_space_is_encoded_before_clearing_query_or_fragment() {
    for seed in [
        "data:space  ?q#f",
        "custom:space  ?q",
        "custom:space  #f",
        "data:space \t\r\n?q",
    ] {
        let mut url = Url::parse(seed).unwrap();
        let expected_path = if seed.contains("\t") {
            "space%20"
        } else {
            "space %20"
        };
        assert_eq!(url.path(), expected_path, "{seed:?}");
        quirks::set_search(&mut url, "");
        quirks::set_hash(&mut url, "");
        assert_eq!(url.path(), expected_path);
        assert_eq!(Url::parse(url.as_str()).unwrap(), url);
    }
    for (input, path) in [
        ("data:inner space", "inner space"),
        ("data:trailing  ", "trailing"),
        ("data:already%20?q", "already%20"),
        ("data:space %3F", "space %3F"),
    ] {
        assert_eq!(Url::parse(input).unwrap().path(), path);
    }
}

#[test]
fn partial_host_outcome_preserves_effects_and_distinguishes_port_rejection() {
    use cem_ql::stdlib::url_setters::set_host;
    for (value, expected, outcome) in [
        (
            "other.test:70000",
            "https://other.test:8443/a",
            SetterOutcome::Ignored("host.port"),
        ),
        (
            "[::1]:70000",
            "https://[::1]:8443/a",
            SetterOutcome::Ignored("host.port"),
        ),
        (
            "other.test:abc",
            "https://other.test:8443/a",
            SetterOutcome::Ignored("host.port"),
        ),
        (
            "other.test:123abc",
            "https://other.test:123/a",
            SetterOutcome::Applied,
        ),
        (
            "other.test:",
            "https://other.test:8443/a",
            SetterOutcome::Applied,
        ),
        (
            "OTHER.test:8443",
            "https://other.test:8443/a",
            SetterOutcome::Applied,
        ),
        (
            "other.test/ignored:70000",
            "https://other.test:8443/a",
            SetterOutcome::Applied,
        ),
        (
            "other.test:\t\n",
            "https://other.test:8443/a",
            SetterOutcome::Applied,
        ),
        (
            "[bad]",
            "https://example.test:8443/a",
            SetterOutcome::Ignored("host"),
        ),
    ] {
        let mut url = Url::parse("https://example.test:8443/a").unwrap();
        assert_eq!(set_host(&mut url, value), outcome, "{value:?}");
        assert_eq!(url.as_str(), expected);
    }
}

#[test]
fn protocol_outcome_covers_equal_normalized_and_file_transitions() {
    use cem_ql::stdlib::url_setters::set_protocol;
    for (seed, value, expected, outcome) in [
        ("file:///a", "FILE:", "file:///a", SetterOutcome::Applied),
        (
            "file://host/a",
            "file:",
            "file://host/a",
            SetterOutcome::Applied,
        ),
        (
            "https://host/a",
            "file:",
            "file://host/a",
            SetterOutcome::Applied,
        ),
        (
            "file://host/a",
            "https:",
            "https://host/a",
            SetterOutcome::Applied,
        ),
        (
            "file:///a",
            "https:",
            "file:///a",
            SetterOutcome::Ignored("protocol"),
        ),
        (
            "https://user@host/a",
            "file:",
            "https://user@host/a",
            SetterOutcome::Ignored("protocol"),
        ),
        (
            "https://host:8443/a",
            "file:",
            "https://host:8443/a",
            SetterOutcome::Ignored("protocol"),
        ),
        (
            "https://host/a",
            "mailto:",
            "https://host/a",
            SetterOutcome::Ignored("protocol"),
        ),
        (
            "https://host/a",
            "1http:",
            "https://host/a",
            SetterOutcome::Ignored("protocol"),
        ),
        (
            "https://host/a",
            "HTTPS:ignored",
            "https://host/a",
            SetterOutcome::Applied,
        ),
    ] {
        let mut url = Url::parse(seed).unwrap();
        assert_eq!(set_protocol(&mut url, value), outcome, "{seed} -> {value}");
        assert_eq!(url.as_str(), expected);
    }
}
