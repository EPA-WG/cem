use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::types::{AtomType, Type, TypeChecker};

fn eval(source: &str) -> ItemStream {
    let query = compile(source, &CompileContext::default())
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    evaluate(&query, &EvaluationContext::default())
}
fn strings(source: &str) -> Vec<String> {
    let result = eval(source);
    assert!(result.error.is_none(), "{result:?}");
    assert!(result.diagnostics.is_empty(), "{result:?}");
    result
        .items
        .iter()
        .map(|item| match item {
            Item::Atomic(AtomValue::String(value)) => value.clone(),
            _ => panic!("expected strings: {result:?}"),
        })
        .collect()
}
fn string(source: &str, expected: &str) {
    assert_eq!(strings(source), vec![expected]);
}

#[test]
fn constructors_and_import_aliases_preserve_their_order_contracts() {
    for source in ["url:params()", "url:params(())"] {
        assert!(eval(source).items.is_empty());
    }
    string(
        r#"url:params_string(url:params({b: "2", a: "1"}))"#,
        "a=1&b=2",
    );
    string(
        "import \"cem:stdlib/url\" as u\nu:params_string(u:params(\"?a=+&a=%2B&bare\"))",
        "a=+&a=%2B&bare=",
    );
    string(r#"url:params_string(url:params({x: "a b"}))"#, "x=a+b");
}
#[test]
fn every_read_operation_keeps_duplicates_and_optional_values() {
    let p = r#"url:params("a=1&a=2&b=3")"#;
    assert!(matches!(
        eval(&format!("url:params_size({p})")).items.as_slice(),
        [Item::Atomic(AtomValue::Integer(3))]
    ));
    assert_eq!(
        strings(&format!("url:params_keys({p})")),
        vec!["a", "a", "b"]
    );
    assert_eq!(
        strings(&format!("url:params_values({p})")),
        vec!["1", "2", "3"]
    );
    assert_eq!(
        strings(&format!("url:params_get_all({p}, \"a\")")),
        vec!["1", "2"]
    );
    string(&format!("url:params_get({p}, \"a\")"), "1");
    assert!(strings(&format!("url:params_get({p}, \"missing\")")).is_empty());
    string(
        &format!("url:params_string(url:params_entries({p}))"),
        "a=1&a=2&b=3",
    );
    for (tail, expected) in [
        (r#""a""#, true),
        (r#""a", "2""#, true),
        (r#""a", "3""#, false),
    ] {
        assert!(
            matches!(eval(&format!("url:params_has({p}, {tail})")).items.as_slice(),[Item::Atomic(AtomValue::Boolean(value))] if *value==expected)
        );
    }
}
#[test]
fn updates_are_immutable_and_sort_is_stable_by_utf16() {
    let p = r#"url:params("b=0&a=1&a=2")"#;
    for (operation, expected) in [
        (r#"params_append(P, "a", "3")"#, "b=0&a=1&a=2&a=3"),
        (r#"params_set(P, "a", "3")"#, "b=0&a=3"),
        (r#"params_delete(P, "a")"#, "b=0"),
        (r#"params_delete(P, "a", "1")"#, "b=0&a=2"),
        ("params_sort(P)", "a=1&a=2&b=0"),
    ] {
        string(
            &format!("url:params_string(url:{})", operation.replace('P', p)),
            expected,
        );
    }
    string(
        r#"url:params_string(url:params_sort(url:params("=last&😀=first&😀=second")))"#,
        "%F0%9F%98%80=first&%F0%9F%98%80=second&%EE%80%80=last",
    );
    string(&format!("url:params_string({p})"), "b=0&a=1&a=2");
}
#[test]
fn strict_static_inputs_and_runtime_cardinality_do_not_coerce() {
    for source in [
        "url:params(42)",
        "url:params_get((), 42)",
        r#"url:params_get((), url:href("https://h"))"#,
        r#"url:params_size("a=1")"#,
        r#"url:params_size({name: "a", value: 2})"#,
    ] {
        assert!(
            compile(source, &CompileContext::default()).is_err(),
            "{source}"
        );
    }
    for source in [
        r#"url:params_get((), ())"#,
        r#"url:params_has((), "x", ())"#,
        r#"url:params_delete((), "x", ("1","2"))"#,
        r#"url:params({a: ()})"#,
    ] {
        let result = eval(source);
        assert!(result.error.is_some(), "{source}");
        assert!(result.items.is_empty());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code, "cem.ql.type_error");
        assert_eq!(result.diagnostics[0].byte_offset, Some(0));
    }
}
#[test]
fn result_types_and_user_function_isolation_remain_precise() {
    for (source, expected) in [
        ("url:params_size(())", Type::atom(AtomType::Integer)),
        (r#"url:params_has((), "x")"#, Type::atom(AtomType::Boolean)),
        ("url:params_string(())", Type::atom(AtomType::String)),
        (
            r#"url:params_get((), "x")"#,
            Type::stream(Type::atom(AtomType::String)),
        ),
        (
            "url:params_keys(())",
            Type::stream(Type::atom(AtomType::String)),
        ),
    ] {
        let parsed = cem_ql::api::parse(source);
        let mut checker = TypeChecker::new();
        checker.seed_runtime_import_surface(&parsed.module);
        let report = checker.check_surface_module(&parsed.module);
        assert!(report.diagnostics.is_empty());
        assert_eq!(report.root_type, Some(expected));
    }
    let result = eval("declare function url:params_size(x) { x } url:params_size(42)");
    assert!(matches!(
        result.items.as_slice(),
        [Item::Atomic(AtomValue::Integer(42))]
    ));
    let failure = eval(r#"url:params_string(report:raise("upstream", "failed"))"#);
    assert_eq!(failure.diagnostics.len(), 1);
    assert_eq!(failure.diagnostics[0].code, "upstream");
}

fn eval_bound(source: &str, items: Vec<Item>) -> ItemStream {
    let mut compile_context = CompileContext::default();
    compile_context
        .policy_bindings
        .insert("input".into(), ItemStream::from_items(items.clone()));
    let query = compile(source, &compile_context).unwrap();
    let mut context = EvaluationContext::default();
    context
        .policy_bindings
        .insert("input".into(), ItemStream::from_items(items));
    evaluate(&query, &context)
}

#[test]
fn typed_pair_bindings_validate_shape_and_preserve_order() {
    let pair = |values: &[&str]| {
        Item::Array(
            values
                .iter()
                .map(|value| Item::Atomic(AtomValue::String((*value).into())))
                .collect(),
        )
    };
    let result = eval_bound(
        "url:params_string(url:params($input))",
        vec![pair(&["b", "2"]), pair(&["a", "1"]), pair(&["b", "3"])],
    );
    assert!(result.error.is_none(), "{result:?}");
    assert!(
        matches!(result.items.as_slice(), [Item::Atomic(AtomValue::String(s))] if s == "b=2&a=1&b=3")
    );
    for items in [
        vec![pair(&["one"])],
        vec![Item::Array(vec![pair(&["x", "1"])])],
        vec![
            pair(&["x", "1"]),
            Item::Atomic(AtomValue::String("bad".into())),
        ],
    ] {
        let failure = eval_bound("url:params($input)", items);
        assert!(failure.error.is_some(), "{failure:?}");
        assert_eq!(failure.diagnostics.len(), 1);
        assert_eq!(failure.diagnostics[0].code, "cem.ql.type_error");
        assert_eq!(failure.diagnostics[0].byte_offset, Some(0));
    }
}

#[test]
fn dynamic_entry_shapes_and_exact_string_types_are_checked() {
    for source in [
        "url:params_get((), $input)",
        "url:params($input)",
        "url:params_entries($input)",
    ] {
        let failure = eval_bound(
            source,
            vec![Item::Atomic(AtomValue::AnyUri("https://h".into()))],
        );
        assert!(failure.error.is_some(), "{source}: {failure:?}");
        assert_eq!(failure.diagnostics.len(), 1);
        assert_eq!(failure.diagnostics[0].code, "cem.ql.type_error");
    }
    for source in [
        r#"url:params_size({ name: "a", value: "1", extra: "x" })"#,
        r#"url:params_size({ name: "a" })"#,
    ] {
        assert!(compile(source, &CompileContext::default()).is_err());
    }
    let parsed = eval(r#"url:parse("https://h/?a=1&a=2")"#);
    let Item::Record(fields) = &parsed.items[0] else {
        panic!("parsed record")
    };
    let result = eval_bound("url:params_string($input)", fields["query"].clone());
    assert!(result.error.is_none(), "{result:?}");
    assert!(
        matches!(result.items.as_slice(), [Item::Atomic(AtomValue::String(s))] if s == "a=1&a=2")
    );
    let uri = eval(r#"url:with_parts("https://h", {query: url:params("a=1&a=2")})"#);
    assert!(uri.error.is_none(), "{uri:?}");
    assert!(
        matches!(uri.items.as_slice(), [Item::Atomic(AtomValue::AnyUri(s))] if s == "https://h/?a=1&a=2")
    );
}

#[test]
fn calls_preserve_bound_values_and_embedding_source_maps() {
    let entries = eval(r#"url:params("b=0&a=1&a=2")"#).items;
    let result = eval_bound(
        r#"(url:params_string(url:params_set($input, "a", "3")), url:params_string($input))"#,
        entries,
    );
    assert!(result.error.is_none(), "{result:?}");
    assert!(matches!(result.items.as_slice(),
        [Item::Atomic(AtomValue::String(updated)), Item::Atomic(AtomValue::String(original))]
        if updated == "b=0&a=3" && original == "b=0&a=1&a=2"));

    use cem_ml::source::{ByteRange, SourceId};
    use cem_ml::source_map::{FrameSpan, SourceMapFrame, TransformKind};
    let source = r#"url:params_has((), "x", ())"#;
    let mut query = compile(source, &CompileContext::default()).unwrap();
    let origin = SourceMapFrame {
        source_id: SourceId(42),
        span: FrameSpan::Single(ByteRange::new(100, 120)),
        transform: TransformKind::TemplateEmbedding {
            host: ByteRange::new(100, 120),
        },
    };
    for map in &mut query.tree.source_maps {
        map.frames.insert(0, origin.clone());
    }
    let result = evaluate(&query, &EvaluationContext::default());
    assert!(result.error.is_some());
    assert_eq!(result.diagnostics.len(), 1);
    let map = result.diagnostics[0].source_map.as_ref().unwrap();
    assert_eq!(map.origin(), Some(&origin));
    assert_eq!(
        map.current().unwrap().span,
        FrameSpan::Single(ByteRange::new(0, source.len() as u32))
    );

    let parsed = cem_ql::api::parse("url:params_entries(())");
    let mut checker = TypeChecker::new();
    checker.seed_runtime_import_surface(&parsed.module);
    let report = checker.check_surface_module(&parsed.module);
    assert!(report.diagnostics.is_empty());
    let Some(Type::Stream(item)) = report.root_type else {
        panic!("entry stream")
    };
    let Type::Record(fields) = *item else {
        panic!("entry record")
    };
    assert_eq!(fields.len(), 2);
    for name in ["name", "value"] {
        assert_eq!(
            fields.iter().find(|field| field.name == name).unwrap().ty,
            Type::atom(AtomType::String)
        );
    }
}
