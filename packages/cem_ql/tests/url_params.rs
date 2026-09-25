use cem_ql::eval::{AtomValue, Item, QueryItemView, QueryItemViewKind};
use cem_ql::stdlib::url_params::UrlParams;
use std::{any::Any, collections::BTreeMap};

fn string(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}
fn pair(name: &str, value: &str) -> Item {
    Item::Array(vec![string(name), string(value)])
}
fn record(fields: Vec<(&str, Vec<Item>)>) -> Item {
    Item::Record(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn parse(value: &str) -> UrlParams {
    UrlParams::from_initializer(Some(&[string(value)])).unwrap()
}

#[test]
fn constructor_distinguishes_mapping_pairs_and_entries() {
    assert_eq!(UrlParams::from_initializer(None).unwrap(), parse(""));
    assert_eq!(UrlParams::from_initializer(Some(&[])).unwrap(), parse(""));
    let mapping = record(vec![("b", vec![string("2")]), ("a", vec![string("1")])]);
    assert_eq!(
        UrlParams::from_initializer(Some(&[mapping]))
            .unwrap()
            .serialize(),
        "a=1&b=2"
    );
    let pairs = vec![pair("b", "2"), pair("a", "1"), pair("b", "3")];
    let p = UrlParams::from_initializer(Some(&pairs)).unwrap();
    assert_eq!(p.serialize(), "b=2&a=1&b=3");
    assert_eq!(UrlParams::from_entries(&p.to_entries()).unwrap(), p);
    let entry = record(vec![
        ("name", vec![string("a")]),
        ("value", vec![string("1")]),
    ]);
    assert_eq!(
        UrlParams::from_initializer(Some(&[entry]))
            .unwrap()
            .serialize(),
        "name=a&value=1"
    );
}

#[test]
fn malformed_shapes_fail_without_coercion_or_partial_output() {
    let bad = vec![
        vec![string("a=1"), string("b=2")],
        vec![Item::Array(vec![string("a")])],
        vec![Item::Array(vec![pair("a", "1")])],
        vec![pair("a", "1"), string("bad")],
        vec![record(vec![("a", vec![])])],
        vec![record(vec![("a", vec![string("1"), string("2")])])],
        vec![Item::Atomic(AtomValue::AnyUri("a=1".into()))],
        vec![Item::Atomic(AtomValue::Boolean(true))],
        vec![Item::Node("a=1".into())],
    ];
    for items in bad {
        assert!(
            UrlParams::from_initializer(Some(&items)).is_err(),
            "{items:?}"
        );
    }
    for items in [
        vec![pair("a", "1")],
        vec![record(vec![("name", vec![string("a")])])],
        vec![record(vec![
            ("name", vec![string("a")]),
            ("value", vec![string("1")]),
            ("extra", vec![]),
        ])],
        vec![record(vec![
            ("name", vec![string("a")]),
            ("value", vec![Item::Atomic(AtomValue::Boolean(true))]),
        ])],
    ] {
        assert!(UrlParams::from_entries(&items).is_err());
    }
    assert!(UrlParams::from_entries(&[]).unwrap().is_empty());
}

#[test]
fn form_encoding_preserves_decoded_values_and_recovers_malformed_bytes() {
    for (raw, expected) in [
        ("?x=+&x=%2B&bare", "x=+&x=%2B&bare="),
        ("x=%ZZ&x=%FF", "x=%25ZZ&x=%EF%BF%BD"),
        ("&&a=b=c&&=x&empty=", "a=b%3Dc&=x&empty="),
        ("??a=1", "%3Fa=1"),
        (
            "https://example.test/?x=1",
            "https%3A%2F%2Fexample.test%2F%3Fx=1",
        ),
        ("x=~!*'()_-.", "x=%7E%21*%27%28%29_-."),
    ] {
        let p = parse(raw);
        assert_eq!(p.serialize(), expected, "{raw}");
        assert_eq!(parse(&p.serialize()), p);
    }
    let p = UrlParams::from_initializer(Some(&[pair("%20", "🍒 +\0")])).unwrap();
    assert_eq!(p.serialize(), "%2520=%F0%9F%8D%92+%2B%00");
    assert_eq!(parse(&p.serialize()), p);
}

#[test]
fn operations_preserve_order_duplicates_and_original() {
    let p = parse("b=0&a=1&a=2&c=3&a=1");
    let original = p.clone();
    assert_eq!(p.len(), 5);
    assert_eq!(p.keys().collect::<Vec<_>>(), vec!["b", "a", "a", "c", "a"]);
    assert_eq!(
        p.values().collect::<Vec<_>>(),
        vec!["0", "1", "2", "3", "1"]
    );
    assert_eq!(p.get("a"), Some("1"));
    assert_eq!(p.get("missing"), None);
    assert_eq!(p.get_all("a").collect::<Vec<_>>(), vec!["1", "2", "1"]);
    assert!(p.has("a", Some("2")));
    assert!(!p.has("a", Some("")));
    assert!(p.has("a", None));
    assert!(!p.has("missing", None));
    assert_eq!(p.append("a", "4").serialize(), "b=0&a=1&a=2&c=3&a=1&a=4");
    assert_eq!(p.delete("a", Some("1")).serialize(), "b=0&a=2&c=3");
    assert_eq!(p.delete("a", None).serialize(), "b=0&c=3");
    assert_eq!(p.set("a", "9").serialize(), "b=0&a=9&c=3");
    assert_eq!(p.set("d", "9").serialize(), "b=0&a=1&a=2&c=3&a=1&d=9");
    assert_eq!(p.delete("missing", None), p);
    assert_eq!(p, original);
    assert_eq!(parse("bare&empty=").get("bare"), Some(""));
}

#[test]
fn sort_is_stable_utf16_not_rust_string_order() {
    let p = UrlParams::from_initializer(Some(&[
        pair("\u{e000}", "bmp"),
        pair("\u{10000}", "first"),
        pair("\u{10000}", "second"),
    ]))
    .unwrap();
    let sorted = p.sorted();
    assert_eq!(
        sorted.values().collect::<Vec<_>>(),
        vec!["first", "second", "bmp"]
    );
    assert_eq!(sorted.sorted(), sorted);
    assert_eq!(p.values().next(), Some("bmp"));
}

#[derive(Debug)]
struct NativeRecord(Vec<(String, Vec<Item>)>);
impl QueryItemView for NativeRecord {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "url-params-test"
    }
    fn identity(&self) -> String {
        "record".into()
    }
    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Record
    }
    fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
        Some(self.0.clone())
    }
}
#[test]
fn native_records_are_sorted_and_duplicate_fields_fail_closed() {
    let fields = vec![
        ("z".into(), vec![string("1")]),
        ("a".into(), vec![string("2")]),
    ];
    let native = Item::native(NativeRecord(fields.clone()));
    let owned = Item::Record(fields.into_iter().collect::<BTreeMap<_, _>>());
    assert_eq!(
        UrlParams::from_initializer(Some(&[native])).unwrap(),
        UrlParams::from_initializer(Some(&[owned])).unwrap()
    );
    let duplicate = Item::native(NativeRecord(vec![
        ("a".into(), vec![string("1")]),
        ("a".into(), vec![string("2")]),
    ]));
    assert!(UrlParams::from_initializer(Some(&[duplicate])).is_err());
}

#[derive(Debug)]
struct NativeValue {
    kind: QueryItemViewKind,
    atom: Option<AtomValue>,
    members: Option<Vec<Item>>,
}
impl QueryItemView for NativeValue {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "url-params-test-value"
    }
    fn identity(&self) -> String {
        "value".into()
    }
    fn kind(&self) -> QueryItemViewKind {
        self.kind
    }
    fn atom(&self) -> Option<AtomValue> {
        self.atom.clone()
    }
    fn members(&self) -> Option<Vec<Item>> {
        self.members.clone()
    }
}
fn native(kind: QueryItemViewKind, atom: Option<AtomValue>, members: Option<Vec<Item>>) -> Item {
    Item::native(NativeValue {
        kind,
        atom,
        members,
    })
}

#[test]
fn native_atomic_and_array_values_obey_the_same_boundary_without_node_atomization() {
    let text = native(
        QueryItemViewKind::Atomic,
        Some(AtomValue::String("a=1".into())),
        None,
    );
    assert_eq!(
        UrlParams::from_initializer(Some(&[text])).unwrap(),
        parse("a=1")
    );
    let array = native(
        QueryItemViewKind::Array,
        None,
        Some(vec![string("a"), string("1")]),
    );
    assert_eq!(
        UrlParams::from_initializer(Some(&[array])).unwrap(),
        parse("a=1")
    );
    for value in [
        native(
            QueryItemViewKind::Node,
            Some(AtomValue::String("a=1".into())),
            Some(vec![string("a"), string("1")]),
        ),
        native(
            QueryItemViewKind::Atomic,
            Some(AtomValue::AnyUri("a=1".into())),
            None,
        ),
        native(QueryItemViewKind::Array, None, None),
        native(QueryItemViewKind::Record, None, None),
    ] {
        assert!(UrlParams::from_initializer(Some(&[value])).is_err());
    }
}

#[test]
fn entry_errors_do_not_mutate_input_or_disclose_payloads() {
    let p = parse("a=1&b=2");
    let mut entries = p.to_entries();
    entries.push(record(vec![
        ("name", vec![string("secret")]),
        ("value", vec![]),
    ]));
    let before = entries.clone();
    let error = UrlParams::from_entries(&entries).unwrap_err();
    assert!(!error.to_string().contains("secret"));
    assert_eq!(entries, before);
    assert_eq!(p.serialize(), "a=1&b=2");
    let native_entry = Item::native(NativeRecord(vec![
        ("value".into(), vec![string("2")]),
        ("name".into(), vec![string("b")]),
    ]));
    assert_eq!(
        UrlParams::from_entries(&[native_entry]).unwrap(),
        parse("b=2")
    );
}

#[test]
fn mapping_order_is_distinct_from_parameter_sort_and_empty_values_are_matchable() {
    let mapping = record(vec![
        ("\u{10000}", vec![string("astral")]),
        ("\u{e000}", vec![string("bmp")]),
    ]);
    let p = UrlParams::from_initializer(Some(&[mapping])).unwrap();
    assert_eq!(p.values().collect::<Vec<_>>(), vec!["bmp", "astral"]);
    assert_eq!(
        p.sorted().values().collect::<Vec<_>>(),
        vec!["astral", "bmp"]
    );
    let p = parse("x=&x=1&x");
    assert!(p.has("x", Some("")));
    assert_eq!(p.delete("x", Some("")).serialize(), "x=1");
    assert_eq!(p.get_all("absent").count(), 0);
    let empty = UrlParams::default();
    assert_eq!(empty.set("x", "").serialize(), "x=");
    assert_eq!(empty.delete("x", None).sorted(), empty);
}
