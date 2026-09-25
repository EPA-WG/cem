//! Pure origin strings: no host blob store or reusable origin identities.
use cem_ql::stdlib::url_origin::serialize_origin;
use serde::Deserialize;
use url::Url;

#[test]
fn blob_origin_is_bounded_to_one_http_or_https_inner_url() {
    for (input, expected) in [
        ("blob:https://example.org/id", "https://example.org"),
        (
            "blob:http://user:pass@EXAMPLE.org:80/id",
            "http://example.org",
        ),
        (
            "blob:https://example.org:8443/id",
            "https://example.org:8443",
        ),
        ("blob:https://[::1]:443/id", "https://[::1]"),
        ("blob:blob:https://example.org/", "null"),
        ("blob:ftp://host/path", "null"),
        ("blob:ws://example.org/", "null"),
        ("blob:wss://example.org/", "null"),
        ("blob:file:///tmp/a", "null"),
        ("blob:custom://host/a", "null"),
        ("blob:null/id", "null"),
        ("blob:/relative", "null"),
        ("blob:https://", "null"),
        ("blob:https%3A%2F%2Fexample.org/id", "null"),
    ] {
        let parsed = Url::parse(input).unwrap();
        let original = parsed.as_str().to_owned();
        assert_eq!(serialize_origin(&parsed), expected, "{input}");
        assert_eq!(parsed.as_str(), original);
    }
}

#[test]
fn direct_origins_preserve_tuples_and_serialize_opaque_as_null() {
    for (input, expected) in [
        ("http://EXAMPLE.org:80/a", "http://example.org"),
        (
            "https://user:pass@example.org:8443/a?#",
            "https://example.org:8443",
        ),
        ("ftp://example.org:21/a", "ftp://example.org"),
        ("ws://example.org:80/a", "ws://example.org"),
        ("wss://example.org:443/a", "wss://example.org"),
        ("file:///tmp/a", "null"),
        ("file://host/a", "null"),
        ("mailto:user@example.org", "null"),
        ("data:text/plain,hi", "null"),
        ("custom://example.org/a", "null"),
    ] {
        assert_eq!(
            serialize_origin(&Url::parse(input).unwrap()),
            expected,
            "{input}"
        );
    }
}

#[test]
fn selected_wpt_blob_origins_match_unchanged_expectations() {
    // Explicit JSON test-data boundary; no runtime document conversion.
    #[derive(Deserialize)]
    struct Fixture {
        upstream_index: usize,
        case: Case,
    }
    #[derive(Deserialize)]
    struct Case {
        input: String,
        origin: Option<String>,
    }
    let fixtures: Vec<Fixture> =
        serde_json::from_str(include_str!("../fixtures/url/wpt-url-parse.json")).unwrap();
    let mut checked = Vec::new();
    for fixture in fixtures {
        if fixture.case.input.starts_with("blob:") {
            if let Some(expected) = fixture.case.origin {
                assert_eq!(
                    serialize_origin(&Url::parse(&fixture.case.input).unwrap()),
                    expected,
                    "WPT {}",
                    fixture.upstream_index
                );
                checked.push(fixture.upstream_index);
            }
        }
    }
    assert_eq!(
        checked,
        [776, 777, 778, 779, 781, 782, 784, 786, 787, 788, 790]
    );
}
