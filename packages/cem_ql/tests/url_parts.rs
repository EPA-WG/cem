use cem_ql::eval::{AtomValue, Item};
use cem_ql::stdlib::url_params::UrlParams;
use cem_ql::stdlib::url_parts::{assemble, with_parts, PartsResult};
use std::collections::BTreeMap;

fn text(s: &str) -> Vec<Item> {
    vec![Item::Atomic(AtomValue::String(s.into()))]
}
fn parts(fields: &[(&str, &str)]) -> Vec<Item> {
    vec![Item::Record(
        fields.iter().map(|(k, v)| ((*k).into(), text(v))).collect(),
    )]
}
fn href(result: PartsResult) -> String {
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
    let AtomValue::AnyUri(value) = result.href else {
        panic!("expected anyURI")
    };
    value
}

#[test]
fn assembly_requires_explicit_seed_and_validates_even_unused_base() {
    assert_eq!(
        href(assemble(&parts(&[]), Some(&text("HTTPS://EXAMPLE.test:443/a"))).unwrap()),
        "https://example.test/a"
    );
    assert_eq!(
        href(
            assemble(
                &parts(&[("href", "../b"), ("hash", "f")]),
                Some(&text("https://h/a/c"))
            )
            .unwrap()
        ),
        "https://h/b#f"
    );
    assert_eq!(
        href(assemble(&parts(&[("href", "data:payload")]), None).unwrap()),
        "data:payload"
    );
    for fields in [
        parts(&[]),
        parts(&[("protocol", "https"), ("hostname", "h")]),
    ] {
        assert_eq!(
            assemble(&fields, None).unwrap_err().code(),
            "cem.ql.url_parts_invalid"
        );
    }
    assert_eq!(
        assemble(&parts(&[("href", "https://h")]), Some(&text("bad")))
            .unwrap_err()
            .code(),
        "cem.ql.url_base_invalid"
    );
    assert_eq!(
        assemble(&parts(&[("href", "relative")]), None)
            .unwrap_err()
            .code(),
        "cem.ql.url_invalid"
    );
    assert_eq!(
        with_parts(&text("relative"), &parts(&[]))
            .unwrap_err()
            .code(),
        "cem.ql.url_invalid"
    );
}

#[test]
fn fixed_order_and_immutable_inputs_preserve_normalized_successes() {
    let input = text("http://old:8443/old?keep#old");
    let fields = parts(&[
        ("hash", "new"),
        ("pathname", "/a^"),
        ("password", "p"),
        ("username", "u"),
        ("port", "443"),
        ("hostname", "EXAMPLE.test"),
        ("protocol", "https:"),
    ]);
    assert_eq!(
        href(with_parts(&input, &fields).unwrap()),
        "https://u:p@example.test/a%5E?keep#new"
    );
    assert!(
        matches!(&input[0],Item::Atomic(AtomValue::String(s)) if s=="http://old:8443/old?keep#old")
    );
    assert_eq!(
        href(
            with_parts(
                &text("https://h:123/"),
                &parts(&[("port", "123abc"), ("protocol", "HTTPS:")])
            )
            .unwrap()
        ),
        "https://h:123/"
    );
    assert_eq!(
        href(with_parts(&text("https://h/a"), &parts(&[])).unwrap()),
        "https://h/a"
    );
}

#[test]
fn all_conflicts_and_read_only_keys_fail_before_parsing() {
    for fields in [
        parts(&[("host", "h"), ("hostname", "h")]),
        parts(&[("host", "h"), ("port", "443")]),
        vec![Item::Record(BTreeMap::from([
            ("search".into(), text("")),
            ("query".into(), vec![]),
        ]))],
        parts(&[("origin", "null")]),
        parts(&[("unknown", "x")]),
        parts(&[("href", "https://h")]),
    ] {
        assert_eq!(
            with_parts(&text("bad"), &fields).unwrap_err().code(),
            "cem.ql.url_parts_invalid"
        );
    }
    assert_eq!(
        href(
            with_parts(
                &text("https://h"),
                &parts(&[("hostname", "other"), ("port", "8443")])
            )
            .unwrap()
        ),
        "https://other:8443/"
    );
    let parsed = cem_ql::stdlib::url::parse(&text("https://h"), None)
        .unwrap()
        .unwrap();
    assert_eq!(
        assemble(&[parsed], None).unwrap_err().code(),
        "cem.ql.url_parts_invalid"
    );
}

#[test]
fn typed_values_and_validation_precedence() {
    let uri = vec![Item::Atomic(AtomValue::AnyUri("https://h".into()))];
    assert_eq!(href(with_parts(&uri, &parts(&[])).unwrap()), "https://h/");
    let fields = vec![Item::Record(BTreeMap::from([("href".into(), uri.clone())]))];
    assert_eq!(href(assemble(&fields, None).unwrap()), "https://h/");
    for bad in [
        vec![],
        text("x"),
        vec![parts(&[])[0].clone(), parts(&[])[0].clone()],
    ] {
        assert_eq!(
            assemble(&bad, None).unwrap_err().code(),
            "cem.ql.type_error"
        );
    }
    for value in [
        vec![],
        uri,
        text("a").into_iter().chain(text("b")).collect(),
        vec![Item::Atomic(AtomValue::Boolean(true))],
    ] {
        let fields = vec![Item::Record(BTreeMap::from([("port".into(), value)]))];
        assert_eq!(
            with_parts(&text("bad"), &fields).unwrap_err().code(),
            "cem.ql.type_error"
        );
    }
    assert_eq!(
        assemble(&parts(&[("origin", "x")]), Some(&[]))
            .unwrap_err()
            .code(),
        "cem.ql.type_error"
    );
    assert_eq!(
        assemble(&parts(&[("origin", "x")]), Some(&text("bad")))
            .unwrap_err()
            .code(),
        "cem.ql.url_parts_invalid"
    );
}

#[test]
fn query_entries_preserve_duplicates_and_empty_query_clears_delimiter() {
    let query = UrlParams::from_initializer(Some(&text("a=+&a=%2B")))
        .unwrap()
        .to_entries();
    let fields = vec![Item::Record(BTreeMap::from([("query".into(), query)]))];
    assert_eq!(
        href(with_parts(&text("https://h/?old#f"), &fields).unwrap()),
        "https://h/?a=+&a=%2B#f"
    );
    let empty = vec![Item::Record(BTreeMap::from([("query".into(), vec![])]))];
    assert_eq!(
        href(with_parts(&text("https://h/?#f"), &empty).unwrap()),
        "https://h/#f"
    );
    let bad = vec![Item::Record(BTreeMap::from([(
        "query".into(),
        text("a=1"),
    )]))];
    assert_eq!(
        with_parts(&text("https://h"), &bad).unwrap_err().code(),
        "cem.ql.type_error"
    );
}

#[test]
fn warnings_retain_partial_effects_and_continue_in_setter_order() {
    let result = with_parts(
        &text("https://h:8443/a"),
        &parts(&[
            ("hash", "done"),
            ("host", "other:70000"),
            ("protocol", "mailto:"),
        ]),
    )
    .unwrap();
    assert!(matches!(result.href,AtomValue::AnyUri(ref s) if s=="https://other:8443/a#done"));
    assert_eq!(
        result
            .warnings
            .iter()
            .map(|w| (w.field, w.subcomponent))
            .collect::<Vec<_>>(),
        vec![("protocol", None), ("host", Some("port"))]
    );
    for warning in result.warnings {
        assert_eq!(warning.code(), "cem.ql.url_setter_ignored");
    }
    let result = with_parts(
        &text("data:payload"),
        &parts(&[
            ("pathname", "new"),
            ("port", "80"),
            ("username", "u"),
            ("password", "p"),
            ("hostname", "h"),
            ("search", "q"),
            ("hash", "f"),
        ]),
    )
    .unwrap();
    assert!(matches!(result.href,AtomValue::AnyUri(ref s) if s=="data:payload?q#f"));
    assert_eq!(
        result.warnings.iter().map(|w| w.field).collect::<Vec<_>>(),
        vec!["hostname", "port", "username", "password", "pathname"]
    );
}

#[derive(Debug)]
struct NativeRecord(Vec<(String, Vec<Item>)>);
impl cem_ql::eval::QueryItemView for NativeRecord {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "url-parts-test"
    }
    fn identity(&self) -> String {
        "parts".into()
    }
    fn kind(&self) -> cem_ql::eval::QueryItemViewKind {
        cem_ql::eval::QueryItemViewKind::Record
    }
    fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
        Some(self.0.clone())
    }
}

#[test]
fn every_component_pair_has_the_same_contract_for_assembly_and_updates() {
    let fields = [
        ("protocol", text("https")),
        ("host", text("h")),
        ("hostname", text("h")),
        ("port", text("8443")),
        ("username", text("u")),
        ("password", text("p")),
        ("pathname", text("/a")),
        ("search", text("q")),
        ("query", vec![]),
        ("hash", text("f")),
    ];
    for (index, (a, av)) in fields.iter().enumerate() {
        for (b, bv) in &fields[index + 1..] {
            let record = vec![Item::Record(BTreeMap::from([
                ((*a).into(), av.clone()),
                ((*b).into(), bv.clone()),
            ]))];
            let update = with_parts(&text("https://h/"), &record);
            let assembled = assemble(&record, Some(&text("https://h/")));
            if matches!(
                (*a, *b),
                ("host", "hostname") | ("host", "port") | ("search", "query")
            ) {
                assert_eq!(update.unwrap_err().code(), "cem.ql.url_parts_invalid");
                assert_eq!(assembled.unwrap_err().code(), "cem.ql.url_parts_invalid");
            } else {
                assert_eq!(href(update.unwrap()), href(assembled.unwrap()), "{a} {b}");
            }
        }
    }
}

#[test]
fn native_field_order_is_irrelevant_and_duplicate_names_fail_closed() {
    for fields in [
        vec![
            ("port".into(), text("443")),
            ("protocol".into(), text("https")),
        ],
        vec![
            ("protocol".into(), text("https")),
            ("port".into(), text("443")),
        ],
    ] {
        let native = Item::native(NativeRecord(fields));
        assert_eq!(
            href(with_parts(&text("http://h"), &[native]).unwrap()),
            "https://h/"
        );
    }
    let duplicate = Item::native(NativeRecord(vec![
        ("hash".into(), text("a")),
        ("hash".into(), text("b")),
    ]));
    assert_eq!(
        with_parts(&text("https://h"), &[duplicate])
            .unwrap_err()
            .code(),
        "cem.ql.type_error"
    );
}

#[test]
fn empty_assignments_invoke_setters_and_warning_messages_do_not_leak_values() {
    assert_eq!(
        href(
            with_parts(
                &text("https://u:p@h:8443/a?q#f"),
                &parts(&[
                    ("username", ""),
                    ("password", ""),
                    ("port", ""),
                    ("pathname", ""),
                    ("search", ""),
                    ("hash", "")
                ])
            )
            .unwrap()
        ),
        "https://h/"
    );
    let result = with_parts(
        &text("https://secret:password@h/a"),
        &parts(&[("port", "private-invalid-port")]),
    )
    .unwrap();
    assert_eq!(result.warnings.len(), 1);
    let message = result.warnings[0].to_string();
    assert!(message.contains("port"));
    for secret in ["secret", "password", "private-invalid-port", "https://"] {
        assert!(!message.contains(secret));
    }
}
