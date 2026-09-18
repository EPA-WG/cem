//! XPATH-REGEX-NATIVE: explicit XPath syntax, bounded engine calls, portable programs.
use cem_ml::{
    content_cache::ContentHash,
    diagnostics::Diagnostic,
    import::import_data,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    resolver::{ResolverPolicy, ResolverRegistry},
    validation::xpath::{artifact::*, *},
};

fn parse(source: &str) -> XPathExpressionAst {
    xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: source.as_bytes(),
            source_uri: "memory:regex.xpath",
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 7 },
    )
}
fn evaluate(
    expression: &XPathExpressionAst,
    item: Option<XPathResultItem>,
    limits: XPathEvaluationLimits,
    control: &OperationControl,
) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
    CemXPathEvaluator::default().evaluate_with_control(
        XPathEvaluationRequest {
            invocation_host: XPathInvocationHost::StandaloneTransform,
            expression,
            dynamic_context: XPathDynamicContext {
                context_item: item,
                ..Default::default()
            },
            static_context: Default::default(),
            expected_result: None,
            resolver_registry: &ResolverRegistry::new(),
            resolver_policy: &ResolverPolicy::new(),
            evaluation_limits: limits,
            safety_policy_stamp: "regex-test",
            module_resolution: None,
        },
        control,
        ROOT_EXECUTION_SCOPE_ID,
    )
}
fn eval(source: &str) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
    evaluate(
        &parse(source),
        None,
        Default::default(),
        &Default::default(),
    )
}
fn strings(source: &str) -> Vec<String> {
    eval(source)
        .unwrap_or_else(|e| panic!("{source}: {e:?}"))
        .sequence
        .items
        .into_iter()
        .map(|i| match i {
            XPathResultItem::Atomic { value, .. } => value.lexical_value,
            other => panic!("{other:?}"),
        })
        .collect()
}

#[test]
fn matching_uses_xpath_anchors_xml_whitespace_unicode_and_literal_flags() {
    for (source, expected) in [
        ("matches('abracadabra', 'bra')", "true"),
        ("matches('abracadabra', '^bra')", "false"),
        ("matches((), '^$')", "true"),
        ("matches('a\n', '^a$')", "false"),
        ("matches('\r', '.')", "false"),
        ("matches('\r\n', '^..$', 'ss')", "true"),
        (r"matches('٣', '^\d$')", "true"),
        (r"matches('_', '^\w$')", "false"),
        (r"matches('é', '^\w$')", "true"),
        (r"matches('©', '^\w$')", "true"),
        ("matches('\u{a0}', '\\s')", "false"),
        ("matches(' ', '\\s')", "true"),
        (r"matches('hello world', 'hello\ sworld', 'x')", "true"),
        ("matches('helloworld', 'hello[ ]world', 'x')", "false"),
        ("matches('a#b', 'a # b', 'x')", "true"),
        ("matches('a\u{a0}b', 'a\u{a0}b', 'x')", "true"),
        (r"matches('é', '^\p{L}$')", "true"),
        (r"matches('3', '^[\p{L}\d]$')", "true"),
        (r"matches('&', '^[a&&b]$')", "true"),
        (r"matches('~', '^[a~~b]$')", "true"),
        (r"matches('-', '^[-a]$')", "true"),
        (r"matches('b', '^[a-c]$')", "true"),
        (r"matches('-', '^[\--a]$')", "true"),
        (r"matches('a', '^\p{Cs}$')", "false"),
        (r"matches('a', '^\P{Cs}$')", "true"),
        (r"matches('.*', '.*', 'q')", "true"),
        (r"matches('anything', '.*', 'q')", "false"),
        (r"matches('a b', 'a b', 'qsxm')", "true"),
        (r"matches('abab', '^(?:ab){2}$')", "true"),
    ] {
        assert_eq!(strings(source), [expected], "{source}");
    }
}

#[test]
fn replace_implements_xpath_capture_numbers_and_escaping() {
    for (source, expected) in [
        (r"replace('abracadabra', 'a.*?a', '*')", "*c*bra"),
        (
            r"replace('abcd', '(ab)|(a)', '[1=$1][2=$2]')",
            "[1=ab][2=]cd",
        ),
        (
            r"replace('a', '(a)', '$0/$1/$2/$12/$0001/$999999999999999999999')",
            "a/a//a2/a/99999999999999999999",
        ),
        (r"replace('b', '(a)?b', '$1')", ""),
        (r"replace('a', 'a', '\$\\')", "$\\"),
        (r"replace('a/b/c', '/', '$\', 'q')", "a$\\b$\\c"),
        (r"replace((), 'x', 'y')", ""),
        (r"replace('aaa', '^a', 'x')", "xaa"),
        (r"replace('aaaa', 'a+?', 'x')", "xxxx"),
        (r"replace('abc', '[abc]', '🍒')", "🍒🍒🍒"),
    ] {
        assert_eq!(strings(source), [expected], "{source}");
    }
}

#[test]
fn tokenize_retains_empty_tokens_but_not_captures_and_keeps_arity_one() {
    assert_eq!(
        strings(r"tokenize('1,15,,24,', ',')"),
        ["1", "15", "", "24", ""]
    );
    assert_eq!(strings(r"tokenize(' a b ', '\s+')"), ["", "a", "b", ""]);
    assert_eq!(strings(r"tokenize(' a b ')"), ["a", "b"]);
    assert_eq!(strings(r"tokenize('a,b', '(,)')"), ["a", "b"]);
    assert_eq!(strings(r"tokenize('1.2.3', '.', 'q')"), ["1", "2", "3"]);
    assert_eq!(
        strings(r"tokenize('abracadabra', '(ab)|(a)')"),
        ["", "r", "c", "d", "r", ""]
    );
    assert!(strings(r"tokenize('', ',')").is_empty());
    assert!(strings(r"tokenize((), ',')").is_empty());
}

#[test]
fn standard_errors_are_distinct_from_explicit_subset_exclusions() {
    for (source, code) in [
        (r"matches('a', 'a', 'z')", "FORX0001"),
        (r"matches('a', '[')", "FORX0002"),
        (r"matches('a', '(?i)a')", "FORX0002"),
        (r"matches('a', '\b')", "FORX0002"),
        (r"matches('a', '\x61')", "FORX0002"),
        (r"matches('a', '[[:alpha:]]')", "FORX0002"),
        (r"matches('a', 'a++')", "FORX0002"),
        (r"matches('a', '[z-a]')", "FORX0002"),
        (r"matches('a', '\p{Alphabetic}')", "FORX0002"),
        (r"replace('abc', 'a*', 'x')", "FORX0003"),
        (r"tokenize('', '')", "FORX0003"),
        (r"replace('none', 'x', '$')", "FORX0004"),
        (r"replace('none', 'x', '\n')", "FORX0004"),
        (r"matches(1, '1')", "XPTY0004"),
        (r"matches('a', ())", "XPTY0004"),
        (r"replace('a', 'a', ())", "XPTY0004"),
        (r"tokenize('a', 'a', ())", "XPTY0004"),
    ] {
        let errors = eval(source).unwrap_err();
        assert!(errors[0].message.contains(code), "{source}: {errors:?}");
        assert!(errors[0].source_map.is_some());
        assert!(errors[0].byte_offset.unwrap() > 0, "{source}: {errors:?}");
    }
    for source in [
        r"matches('aa', '(a)\1')",
        r"matches('a', '[a-z-[aeiou]]')",
        r"matches('a', '\i')",
        r"matches('a', '\p{IsBasicLatin}')",
        r"matches('a', 'a', 'i')",
        r"matches('a', '^a$', 'm')",
    ] {
        let errors = eval(source).unwrap_err();
        assert_eq!(
            errors[0].code, "cem.xpath.regex_unsupported",
            "{source}: {errors:?}"
        );
        assert!(!errors[0].message.contains("FORX0002"));
    }
}

#[test]
fn regex_limits_cover_compile_search_repeated_search_output_and_cancellation() {
    for (source, limits, code) in [
        (
            r"matches('abcdefgh', '(a|b|c)+')",
            XPathEvaluationLimits {
                max_work_units: Some(100),
                ..Default::default()
            },
            "cem.xpath.work_limit_exceeded",
        ),
        (
            r"replace('aaa', 'a', 'xxxx')",
            XPathEvaluationLimits {
                max_text_bytes: Some(5),
                ..Default::default()
            },
            "cem.xpath.text_byte_limit_exceeded",
        ),
        (
            r"tokenize('a,b,c', ',')",
            XPathEvaluationLimits {
                max_sequence_items: Some(2),
                ..Default::default()
            },
            "cem.xpath.sequence_item_limit_exceeded",
        ),
        (
            r"matches('a', 'a{999999999}')",
            Default::default(),
            "cem.xpath.regex_limit_exceeded",
        ),
    ] {
        let errors = evaluate(&parse(source), None, limits, &Default::default()).unwrap_err();
        assert_eq!(errors[0].code, code, "{source}: {errors:?}");
    }
    for source in [
        format!("matches('a', '{}')", "a".repeat(2049)),
        format!("matches('{}', 'a')", "a".repeat(16385)),
        format!("matches('a', '{}a{}')", "(".repeat(33), ")".repeat(33)),
        "matches('a', '(a{1024}){1024}')".to_owned(),
    ] {
        let errors = eval(&source).unwrap_err();
        assert_eq!(
            errors[0].code, "cem.xpath.regex_limit_exceeded",
            "{errors:?}"
        );
    }
    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    let errors = evaluate(
        &parse("matches('a', 'a')"),
        None,
        Default::default(),
        &control,
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "cem.xpath.control_failure");
}

#[test]
fn portable_programs_atomize_imported_trees_without_format_specific_branches() {
    let source = r"replace(/*/*, '([0-9]+)', 'n=$1')";
    let ast = parse(source);
    let hash = ContentHash::from_blake3(source.as_bytes());
    let compiled = XPathCompiledArtifact::compile(&ast, hash.clone()).unwrap();
    let loaded =
        XPathCompiledArtifact::from_bytes(compiled.bytes().to_vec(), compiled.content_hash())
            .unwrap()
            .reload(&XPathArtifactLoadContext {
                expected_source_hash: hash,
                invocation_host: XPathInvocationHost::StandaloneTransform,
            })
            .unwrap();
    assert!(loaded.source_text.is_none());
    for (source, format) in [
        ("<r><value>12</value></r>", "xml"),
        (r#"{"value":"12"}"#, "json"),
        ("value: '12'", "yaml"),
    ] {
        let tree = import_data(source, format, "cem", "memory:regex-input").unwrap();
        let item = XPathResultItem::from_native_node(XPathNativeNode::cem_document(tree));
        let result =
            evaluate(&loaded, Some(item), Default::default(), &Default::default()).unwrap();
        let XPathResultItem::Atomic { value, .. } = &result.sequence.items[0] else {
            panic!()
        };
        assert_eq!(value.lexical_value, "n=12");
    }
}

#[test]
fn repeated_searches_are_charged_even_when_the_output_is_empty() {
    let input = "a".repeat(4096);
    let limits = XPathEvaluationLimits {
        max_work_units: None,
        ..Default::default()
    };
    evaluate(
        &parse(&format!("matches('{input}', 'a')")),
        None,
        limits,
        &Default::default(),
    )
    .unwrap();
    let errors = evaluate(
        &parse(&format!("replace('{input}', 'a', '')")),
        None,
        limits,
        &Default::default(),
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "cem.xpath.regex_limit_exceeded");
}
