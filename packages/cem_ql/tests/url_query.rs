use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, Item, ItemStream};

fn eval(source: &str) -> ItemStream {
    let query = compile(source, &CompileContext::default()).unwrap();
    evaluate(&query, &EvaluationContext::default())
}
fn uri(source: &str, expected: &str) {
    let result = eval(source);
    assert!(result.error.is_none(), "{:?}", result);
    assert!(
        matches!(result.items.as_slice(),[Item::Atomic(AtomValue::AnyUri(s))] if s==expected),
        "{:?}",
        result
    );
}
#[test]
fn aliases_and_five_registered_operations_execute() {
    uri(r#"url:href("../b", "https://h/a/c")"#, "https://h/b");
    uri(
        r#"import "cem:stdlib/url" as u
u:href("https://h")"#,
        "https://h/",
    );
    uri(
        r#"url:assemble({ href: "https://h", hash: "f" })"#,
        "https://h/#f",
    );
    uri(
        r#"url:with_parts("https://h", { pathname: "/b" })"#,
        "https://h/b",
    );
    assert!(matches!(
        eval(r#"url:can_parse("bad")"#).items.as_slice(),
        [Item::Atomic(AtomValue::Boolean(false))]
    ));
    assert!(eval(r#"url:parse("bad")"#).items.is_empty());
    assert!(matches!(
        eval(r#"url:parse("https://h")"#).items.as_slice(),
        [Item::Record(_)]
    ));
}
#[test]
fn strict_static_types_and_user_function_name_isolation() {
    for source in [
        r#"url:href(42)"#,
        r#"url:assemble("bad")"#,
        r#"url:with_parts("https://h", 42)"#,
    ] {
        assert!(
            compile(source, &CompileContext::default()).is_err(),
            "{source}"
        );
    }
    let result = eval("declare function href(x) { x } href(42)");
    assert!(matches!(
        result.items.as_slice(),
        [Item::Atomic(AtomValue::Integer(42))]
    ));
}
#[test]
fn errors_and_warnings_use_the_call_source_map_and_preserve_partial_effects() {
    let source = r#"url:with_parts("https://secret:password@h:8443/a", { protocol: "mailto:", host: "other:70000", hash: "done" })"#;
    let result = eval(source);
    assert!(result.error.is_none());
    assert!(
        matches!(result.items.as_slice(),[Item::Atomic(AtomValue::AnyUri(s))] if s=="https://secret:password@other:8443/a#done")
    );
    assert_eq!(result.diagnostics.len(), 2);
    for diagnostic in &result.diagnostics {
        assert_eq!(diagnostic.code, "cem.ql.url_setter_ignored");
        assert_eq!(diagnostic.byte_offset, Some(0));
        assert!(diagnostic.source_map.as_ref().unwrap().current().is_some());
        assert!(!diagnostic.message.contains("secret"));
    }
    assert!(result.diagnostics[0].message.contains("protocol"));
    assert!(result.diagnostics[1].message.contains("port"));
    let failure = eval(r#"url:href("https://h","bad")"#);
    assert!(failure.error.is_some());
    assert!(failure.items.is_empty());
    assert_eq!(failure.diagnostics.len(), 1);
    assert_eq!(failure.diagnostics[0].code, "cem.ql.url_base_invalid");
    assert_eq!(failure.diagnostics[0].byte_offset, Some(0));
}
#[test]
fn suppression_never_hides_upstream_errors() {
    for function in ["parse", "can_parse"] {
        let result = eval(&format!(
            r#"url:{function}(report:raise("test.upstream", "failed"))"#
        ));
        assert!(result.error.is_some());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code, "test.upstream");
    }
}

#[test]
fn runtime_cardinality_and_nested_diagnostics_are_not_duplicated() {
    for source in [
        r#"url:href(())"#,
        r#"url:href(("https://h", "https://other"))"#,
    ] {
        let result = eval(source);
        assert!(result.error.is_some());
        assert!(result.items.is_empty());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code, "cem.ql.type_error");
    }
    let result = eval(r#"url:href(url:with_parts("https://h", { port: "bad" }))"#);
    assert!(result.error.is_none());
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "cem.ql.url_setter_ignored");
    let result =
        eval(r#"url:href(report:raise("first", "fail"), report:raise("second", "must not run"))"#);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "first");
}

#[test]
fn return_types_are_concrete_and_custom_signatures_do_not_inherit_url_checks() {
    use cem_ql::types::{AtomType, Type, TypeChecker};
    for (source, expected) in [
        (r#"url:href("https://h")"#, Type::atom(AtomType::AnyUri)),
        (
            r#"url:can_parse("https://h")"#,
            Type::atom(AtomType::Boolean),
        ),
        (
            r#"url:assemble({href: "https://h"})"#,
            Type::atom(AtomType::AnyUri),
        ),
        (
            r#"url:with_parts("https://h", {})"#,
            Type::atom(AtomType::AnyUri),
        ),
    ] {
        let parsed = cem_ql::api::parse(source);
        let mut checker = TypeChecker::new();
        checker.seed_runtime_import_surface(&parsed.module);
        let report = checker.check_surface_module(&parsed.module);
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        assert_eq!(report.root_type, Some(expected));
    }
    let parsed = cem_ql::api::parse(r#"url:parse("https://h")"#);
    let mut checker = TypeChecker::new();
    checker.seed_runtime_import_surface(&parsed.module);
    let report = checker.check_surface_module(&parsed.module);
    let Some(Type::Stream(item)) = report.root_type else {
        panic!("optional stream")
    };
    let Type::Record(fields) = *item else {
        panic!("parsed record")
    };
    assert_eq!(fields.len(), 12);
    assert_eq!(
        fields.iter().find(|f| f.name == "href").unwrap().ty,
        Type::atom(AtomType::AnyUri)
    );

    let result = eval("declare function url:href(x) { x } url:href(42)");
    assert!(matches!(
        result.items.as_slice(),
        [Item::Atomic(AtomValue::Integer(42))]
    ));
    assert!(compile(
        "import \"cem:stdlib/url\" as u\nu:href(42)",
        &CompileContext::default()
    )
    .is_err());
}

#[test]
fn diagnostics_retain_the_complete_embedding_stack() {
    use cem_ml::source::{ByteRange, SourceId};
    use cem_ml::source_map::{FrameSpan, SourceMapFrame, TransformKind};
    let mut query = compile(r#"url:href("bad")"#, &CompileContext::default()).unwrap();
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
    let map = result.diagnostics[0].source_map.as_ref().unwrap();
    assert_eq!(map.origin(), Some(&origin));
    assert_eq!(
        map.current().unwrap().span,
        FrameSpan::Single(ByteRange::new(0, 15))
    );
}
