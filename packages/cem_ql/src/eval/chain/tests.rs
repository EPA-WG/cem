use crate::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    eval::{AtomValue, ItemStream},
};

fn run(query: &str) -> ItemStream {
    let compiled =
        compile(query, &CompileContext::default()).unwrap_or_else(|e| panic!("{query}: {e:?}"));
    let result = evaluate(&compiled, &EvaluationContext::default());
    assert!(result.error.is_none(), "{query}: {:?}", result.diagnostics);
    result
}

fn texts(query: &str) -> Vec<String> {
    run(query)
        .items
        .iter()
        .map(|item| match item.atom().unwrap() {
            AtomValue::String(value) => value,
            other => panic!("{other:?}"),
        })
        .collect()
}

#[test]
fn rust_string_split_and_zero_based_selection() {
    for value in ["", "abc", "a🍒é", "/a//b/"] {
        for separator in ["", "/", "a"] {
            let expression = format!("{value:?}.split({separator:?})");
            let expected: Vec<_> = value.split(separator).collect();
            assert_eq!(texts(&expression), expected);
            for index in [0, 1, 6, 50, 4_294_967_296, i64::MAX as usize] {
                assert_eq!(
                    texts(&format!("{expression}.nth({index})")),
                    value
                        .split(separator)
                        .nth(index)
                        .into_iter()
                        .collect::<Vec<_>>()
                );
            }
        }
    }
    assert_eq!(
        texts(r#""https://pokeapi.co/api/v2/pokemon/1/".split("/").nth(6)"#),
        ["1"]
    );
    assert_eq!(texts(r#""a/b".split("/").skip(1).next()"#), ["b"]);
    assert_eq!(texts(r#"seq:nth(("a", "b"), 0)"#), ["a"]);
    assert_eq!(texts(r#"("a", "b").nth(1)"#), ["b"]);
    assert!(texts(r#"dom::chain(()).split("/").nth(6)"#).is_empty());
    assert_eq!(
        texts(
            r#"dom::chain(data:read("<url>a/b</url>", "xml").root.children).text().split("/").nth(1)"#
        ),
        ["b"]
    );
}

#[test]
fn rust_reverse_search_short_circuits_from_the_back() {
    assert_eq!(texts(r#"dom::chain(("a", "b", "c")).rev().next()"#), ["c"]);
    assert_eq!(
        texts(
            r#"dom::chain(("a", "b", "c")).rfind(|v| if v == "c" { true } else { report:raise("unreachable", "wrong direction") })"#
        ),
        ["c"]
    );
    assert_eq!(
        texts(r#"dom::chain(("a", "b", "c")).rfind(|v| v != "c")"#),
        ["b"]
    );
}

#[test]
fn invalid_split_and_indices_fail_even_on_empty_input() {
    for query in [
        "dom::chain(()).nth(-1)",
        "seq:nth((), -1)",
        "dom::chain(()).nth(1.5)",
        "dom::chain(()).nth(())",
        "dom::chain(()).split(1)",
        "dom::chain(()).split(())",
        "dom::chain(null).split(\"/\")",
        "dom::chain(42).split(\"/\")",
        "dom::chain(()).nth()",
    ] {
        if let Ok(compiled) = compile(query, &CompileContext::default()) {
            let result = evaluate(&compiled, &EvaluationContext::default());
            assert!(result.error.is_some(), "{query}");
            assert!(result.items.is_empty(), "{query}");
        }
    }
}

#[test]
fn native_sibling_chain_and_empty_propagation() {
    assert_eq!(
        texts(
            r#"let row = data:read("<r><name>ivy</name><id>2</id></r>", "xml").root.children;
        dom::chain(row).children().find(|n| n.name() == "name")
            .parent().children().find(|n| n.name() == "id").text()"#
        ),
        ["2"]
    );
    assert!(
        run(r#"dom::chain(()).parent().children().find(|n| n.name() == "id").text()"#)
            .items
            .is_empty()
    );
}

#[test]
fn collection_selection_and_terminals() {
    assert_eq!(
        texts(r#"dom::chain(("c", "b", "a", "b")).filter(|v| v != "c").sorted().take(2)"#),
        ["a", "b"]
    );
    assert_eq!(
        texts(r#"dom::chain(("a", "b", "c")).rfind(|v| v != "c")"#),
        ["b"]
    );
    assert_eq!(
        run("dom::chain(()).any(|v| true)").items[0].atom(),
        Some(AtomValue::Boolean(false))
    );
    assert_eq!(
        run("dom::chain(()).all(|v| false)").items[0].atom(),
        Some(AtomValue::Boolean(true))
    );
}

#[test]
fn navigation_namespaces_and_closest_per_input() {
    let setup = r#"let doc = data::read("<r xmlns:p='urn:p'><section id='one' p:id='two'>text<a/><b><leaf/></b><!--comment--></section></r>", "xml").root;
        let section = dom::chain(doc).children().children(); "#;
    assert_eq!(
        texts(&format!("{setup} section.children().name()")),
        ["a", "b"]
    );
    assert_eq!(
        run(&format!("{setup} section.child_nodes().count()")).items[0].atom(),
        Some(AtomValue::Integer(4))
    );
    assert_eq!(
        texts(&format!(r#"{setup} section.attribute("id").text()"#)),
        ["one"]
    );
    assert_eq!(
        texts(&format!(
            r#"{setup} section.attribute({{namespace: "urn:p", name: "id"}}).text()"#
        )),
        ["two"]
    );
    assert_eq!(
        texts(&format!(
            r#"{setup} section.children().closest(|n| n.name() == "section").name()"#
        )),
        ["section", "section"]
    );
    assert_eq!(
        texts(&format!(
            r#"{setup} section.closest(|n| n.name() == "section").name()"#
        )),
        ["section"]
    );
    assert_eq!(
        texts(&format!(
            r#"{setup} section.children().last().children().ancestors().name()"#
        )),
        ["b", "section", "r"]
    );
    assert!(run(&format!(
        r#"{setup} section.parent().closest(|n| n.name() == "section")"#
    ))
    .items
    .is_empty());
}

#[test]
fn functional_chains_and_scalar_terminals() {
    assert_eq!(
        texts(r#"let suffix = "!"; dom::chain(("a", "b", "c")).skip(1).map(|v| v + suffix).rev()"#),
        ["c!", "b!"]
    );
    assert_eq!(
        texts(r#"dom::chain(("a", "b")).flat_map(|v| (v, v)).skip(1).take(2)"#),
        ["a", "b"]
    );
    assert_eq!(texts(r#"dom::chain(("a", "b")).next()"#), ["a"]);
    assert_eq!(texts(r#"dom::chain(("a", "b")).last()"#), ["b"]);
    assert_eq!(
        run("dom::chain(()).is_empty()").items[0].atom(),
        Some(AtomValue::Boolean(true))
    );
    assert_eq!(
        run("dom::chain((1, 2, 3)).count()").items[0].atom(),
        Some(AtomValue::Integer(3))
    );
    assert_eq!(
        run("dom::chain((1, 2)).any(|v| v == 2)").items[0].atom(),
        Some(AtomValue::Boolean(true))
    );
    assert_eq!(
        run("dom::chain((1, 2)).all(|v| v == 1)").items[0].atom(),
        Some(AtomValue::Boolean(false))
    );
    assert_eq!(
        run("dom::chain((1, 2)).map(|v| dom::chain((v, v)))")
            .items
            .len(),
        2
    );
    assert_eq!(
        run("dom::chain((1, 2)).map(|v| dom::chain((v, v))).flat_map(|v| v)")
            .items
            .len(),
        4
    );
}

#[test]
fn stable_order_preserves_identity_parent_and_source() {
    let setup = r#"let row = data:read("<r><b>first</b><a>second</a><b>third</b></r>", "xml").root.children;
        let nodes = dom::chain(row).children(); "#;
    assert_eq!(
        texts(&format!("{setup} nodes.sorted_by_key(|n| n.name()).text()")),
        ["second", "first", "third"]
    );
    assert_eq!(
        texts(&format!(
            r#"{setup} nodes.sorted_by_key(|n| n.name(), "descending").text()"#
        )),
        ["first", "third", "second"]
    );
    assert_eq!(
        texts(r#"dom::chain(("10", "2", "bad")).sorted("ascending", "number")"#),
        ["2", "10", "bad"]
    );
    assert_eq!(
        texts(&format!(
            "{setup} nodes.sorted_by_key(|n| n.name()).parent().name()"
        )),
        ["r", "r", "r"]
    );
    let query = format!("{setup} (nodes, nodes.sorted_by_key(|n| n.name()))");
    let values = run(&query).items;
    for (a, b) in [(0, 4), (1, 3), (2, 5)] {
        assert_eq!(values[a].identity(), values[b].identity());
        assert_eq!(values[a].source_map(), values[b].source_map());
    }
}

#[test]
fn syntax_and_static_errors_are_explicit() {
    assert_eq!(
        texts(r#"dom::chain(("a", "b")).filter(|v| { v == "a" })"#),
        ["a"]
    );
    assert_eq!(
        run("false || true").items[0].atom(),
        Some(AtomValue::Boolean(true))
    );
    assert_eq!(run("(1, 2) | (2, 3)").items.len(), 3);
    for query in [
        "dom::chain(1).children()",
        "dom::chain(()).missing()",
        "dom::chain(()).take()",
        "dom::chain(()).find(|| true)",
        "dom::chain(()).find(|a, b| true)",
        "dom::chain(()).filter(|v| 1)",
    ] {
        assert!(
            compile(query, &CompileContext::default()).is_err(),
            "{query}"
        );
    }
    // Field access retains its existing contract; parentheses are not discarded.
    assert_eq!(texts(r#"{name: "value"}.name"#), ["value"]);
    let compiled = compile(r#"{name: "value"}.name()"#, &CompileContext::default());
    assert!(compiled.is_err());
}

#[test]
fn callback_errors_never_return_partial_results() {
    for method in ["filter", "rfind", "sorted_by_key", "map"] {
        let query = format!(
            r#"dom::chain((1, 2)).{method}(|v| if v == 1 {{ true }} else {{ report:raise("test.failed", "failed") }})"#
        );
        let compiled = compile(&query, &CompileContext::default()).unwrap();
        let result = evaluate(&compiled, &EvaluationContext::default());
        assert!(result.error.is_some(), "{query}");
        assert!(result.items.is_empty(), "{query}: partial output");
    }
}

#[test]
fn flat_map_flattens_the_callback_collection_once() {
    use crate::eval::Item;
    let values = run("dom::chain((1,2)).flat_map(|v| dom::chain((v,v)).map(|n| (n,n)))").items;
    assert_eq!(values.len(), 4);
    assert!(values
        .iter()
        .all(|value| matches!(value, Item::Array(members) if members.len() == 2)));
}

#[test]
fn existing_sorted_function_still_requires_a_callback() {
    let compiled = compile("seq:sorted((1,2), ())", &CompileContext::default()).unwrap();
    let result = evaluate(&compiled, &EvaluationContext::default());
    assert!(result.error.is_some() && result.items.is_empty());
}
