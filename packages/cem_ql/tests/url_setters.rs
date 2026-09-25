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
fn remaining_file_slash_gap_also_occurs_when_parsing_expected_output() {
    // Characterization only: these expected WPT outputs cannot currently be
    // represented by reparsing through the dependency, unlike hostless guards.
    for (expected, actual) in [
        ("file://monkey//", "file://monkey/"),
        ("file://////", "file:///"),
        ("file://///", "file:///"),
    ] {
        assert_eq!(Url::parse(expected).unwrap().as_str(), actual);
    }
}
