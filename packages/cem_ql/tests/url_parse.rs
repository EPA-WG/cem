use cem_ql::eval::{AtomValue, Item, QueryItemView, QueryItemViewKind};
use cem_ql::stdlib::url::{can_parse, href, parse, UrlError};
use cem_ql::stdlib::url_params::UrlParams;
use std::any::Any;

fn text(s: &str) -> Vec<Item> {
    vec![Item::Atomic(AtomValue::String(s.into()))]
}
fn field(record: &Item, key: &str) -> String {
    let Item::Record(fields) = record else {
        panic!("expected record")
    };
    match fields[key].as_slice() {
        [Item::Atomic(AtomValue::String(s) | AtomValue::AnyUri(s))] => s.clone(),
        _ => panic!("expected text field {key}"),
    }
}

#[test]
fn resolves_only_against_explicit_valid_base_and_retains_delimiters() {
    let input = text("../b?#");
    assert!(!can_parse(&input, None).unwrap());
    assert!(parse(&input, None).unwrap().is_none());
    assert_eq!(href(&input, None).unwrap_err(), UrlError::InvalidUrl);
    let base = text("https://example.org/a/c");
    assert!(can_parse(&text(""), Some(&base)).unwrap());
    assert_eq!(
        href(&input, Some(&text(""))).unwrap_err(),
        UrlError::InvalidBase
    );
    let parsed = parse(&input, Some(&base)).unwrap().unwrap();
    assert_eq!(field(&parsed, "href"), "https://example.org/b?#");
    assert_eq!(field(&parsed, "search"), "");
    assert_eq!(field(&parsed, "hash"), "");
    assert!(
        matches!(href(&input, Some(&base)).unwrap(), AtomValue::AnyUri(s) if s == "https://example.org/b?#")
    );
    assert!(
        matches!(href(&text(&field(&parsed, "href")), None).unwrap(), AtomValue::AnyUri(s) if s == field(&parsed, "href"))
    );
    assert!(!can_parse(&text("child"), Some(&text("mailto:user@example.org"))).unwrap());
    assert!(can_parse(&text("#f"), Some(&text("mailto:user@example.org"))).unwrap());
}

#[test]
fn invalid_base_precedes_input_syntax_even_for_absolute_input() {
    for input in ["https://example.org/", "https://", "relative"] {
        assert_eq!(
            href(&text(input), Some(&text("not a base"))).unwrap_err(),
            UrlError::InvalidBase
        );
        assert!(!can_parse(&text(input), Some(&text("not a base"))).unwrap());
        assert!(parse(&text(input), Some(&text("not a base")))
            .unwrap()
            .is_none());
    }
}

#[test]
fn parsed_record_has_typed_fields_and_duplicate_preserving_query() {
    let record = parse(
        &text("HTTPS://user:pass@EXAMPLE.org:443/a/../b?x=+&x=%2B&bare#frag"),
        None,
    )
    .unwrap()
    .unwrap();
    let Item::Record(fields) = &record else {
        panic!("expected record")
    };
    assert_eq!(fields.len(), 12);
    assert!(matches!(
        fields["href"].as_slice(),
        [Item::Atomic(AtomValue::AnyUri(_))]
    ));
    for (name, expected) in [
        ("origin", "https://example.org"),
        ("protocol", "https:"),
        ("username", "user"),
        ("password", "pass"),
        ("host", "example.org"),
        ("hostname", "example.org"),
        ("port", ""),
        ("pathname", "/b"),
        ("search", "?x=+&x=%2B&bare"),
        ("hash", "#frag"),
    ] {
        assert_eq!(field(&record, name), expected, "{name}");
    }
    let query = UrlParams::from_entries(&fields["query"]).unwrap();
    assert_eq!(query.len(), 3);
    assert_eq!(query.get_all("x").collect::<Vec<_>>(), [" ", "+"]);
    assert_eq!(query.get("bare"), Some(""));
    let empty = parse(&text("https://example.org/?"), None)
        .unwrap()
        .unwrap();
    let Item::Record(fields) = empty else {
        unreachable!()
    };
    assert!(fields["query"].is_empty());
}

#[test]
fn origin_adapter_and_recoverable_encoding_are_used() {
    for input in [
        "blob:blob:https://example.org/",
        "blob:ftp://host/path",
        "blob:ws://example.org/",
        "blob:wss://example.org/",
        "file:///tmp/a",
    ] {
        assert_eq!(
            field(&parse(&text(input), None).unwrap().unwrap(), "origin"),
            "null"
        );
    }
    let record = parse(&text("https://example.org/a b?bad=%FF&literal=%zz"), None)
        .unwrap()
        .unwrap();
    assert_eq!(field(&record, "pathname"), "/a%20b");
    let Item::Record(fields) = record else {
        unreachable!()
    };
    let query = UrlParams::from_entries(&fields["query"]).unwrap();
    assert_eq!(query.get("bad"), Some("\u{fffd}"));
    assert_eq!(query.get("literal"), Some("%zz"));
}

#[derive(Debug)]
struct NativeText {
    kind: QueryItemViewKind,
    value: AtomValue,
}
impl QueryItemView for NativeText {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "url-parse-test"
    }
    fn identity(&self) -> String {
        "url-parse-test".into()
    }
    fn kind(&self) -> QueryItemViewKind {
        self.kind
    }
    fn atom(&self) -> Option<AtomValue> {
        Some(self.value.clone())
    }
}

#[test]
fn strict_typed_boundary_validates_all_shapes_before_syntax() {
    for input in [
        vec![],
        vec![text("a").remove(0), text("b").remove(0)],
        vec![Item::Atomic(AtomValue::Boolean(true))],
        vec![Item::Array(text("https://example.org/"))],
        vec![Item::native(NativeText {
            kind: QueryItemViewKind::Record,
            value: AtomValue::String("https://example.org/".into()),
        })],
        vec![Item::native(NativeText {
            kind: QueryItemViewKind::Node,
            value: AtomValue::String("https://example.org/".into()),
        })],
    ] {
        assert_eq!(
            can_parse(&input, None).unwrap_err(),
            UrlError::Type("input")
        );
        assert_eq!(parse(&input, None).unwrap_err(), UrlError::Type("input"));
        assert_eq!(
            href(&input, Some(&[])).unwrap_err(),
            UrlError::Type("input")
        );
    }
    assert_eq!(
        href(&text("invalid"), Some(&[])).unwrap_err(),
        UrlError::Type("base")
    );
    assert_eq!(
        can_parse(&text("invalid"), Some(&[])).unwrap_err(),
        UrlError::Type("base")
    );
    assert_eq!(
        parse(&text("invalid"), Some(&[])).unwrap_err(),
        UrlError::Type("base")
    );
    for item in [
        Item::Atomic(AtomValue::AnyUri("https://example.org/".into())),
        Item::native(NativeText {
            kind: QueryItemViewKind::Atomic,
            value: AtomValue::AnyUri("https://example.org/".into()),
        }),
    ] {
        assert!(can_parse(&[item], None).unwrap());
    }
}

#[test]
fn errors_are_classified_without_echoing_input() {
    let error = href(&text("https://user:secret@/"), None).unwrap_err();
    assert_eq!(error.code(), "cem.ql.url_invalid");
    assert_eq!(UrlError::InvalidBase.code(), "cem.ql.url_base_invalid");
    assert_eq!(UrlError::Type("base").code(), "cem.ql.type_error");
    assert!(!error.to_string().contains("secret"));
}

#[test]
fn selected_wpt_through_core_retains_only_deferred_parser_gaps() {
    // Explicit pinned WPT JSON test-data boundary, not runtime document import.
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/url/wpt-url-parse.json")).unwrap();
    let fixtures = fixtures.as_array().unwrap();
    assert_eq!(fixtures.len(), 51);
    let mut differences = Vec::new();
    for fixture in fixtures {
        let index = fixture["upstream_index"].as_u64().unwrap();
        let case = &fixture["case"];
        let input = text(case["input"].as_str().unwrap());
        let base = case["base"].as_str().map(text);
        let result = parse(&input, base.as_deref()).unwrap();
        assert_eq!(
            can_parse(&input, base.as_deref()).unwrap(),
            result.is_some()
        );
        if case["failure"] == true {
            if result.is_some() {
                differences.push(format!("{index}: unexpectedly accepted"));
            }
            continue;
        }
        let Some(record) = result else {
            differences.push(format!("{index}: unexpectedly rejected"));
            continue;
        };
        let serialized = field(&record, "href");
        assert!(
            matches!(href(&text(&serialized), None).unwrap(), AtomValue::AnyUri(s) if s == serialized)
        );
        for name in [
            "href", "origin", "protocol", "username", "password", "host", "hostname", "port",
            "pathname", "search", "hash",
        ] {
            if let Some(expected) = case[name].as_str() {
                let actual = field(&record, name);
                if actual != expected {
                    differences.push(format!(
                        "{index}: {name}: expected {expected:?}, received {actual:?}"
                    ));
                }
            }
        }
    }
    // Known debt only: unchanged raw-parser evidence minus the fixed origin gaps.
    // This characterization is not a full-conformance claim or production allowlist.
    let expected = include_str!("../fixtures/url/url-2.5.8-differences.txt")
        .lines()
        .filter(|line| {
            ["664:", "839:", "921:"]
                .iter()
                .any(|id| line.starts_with(id))
        })
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 5);
    assert_eq!(differences, expected);
}
