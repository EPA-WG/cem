//! XPATH-DEMO-NAMED-NATIVE: authored XPath functions retain language ownership.
use cem_ml::{
    lifecycle::LoadedInputAstStream,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    resolver::{ResolverPolicy, ResolverRegistry},
    scheduler::ScopePolicy,
    validation::{
        xml::{xml_document_ast_from_source_bytes, XmlSourceValidationRequest},
        xpath::XPathNativeNode,
    },
};
use cem_ql::{
    api::{compile, evaluate, evaluate_with_control, CompileContext, EvaluationContext},
    eval::{AtomValue, BudgetAxis, EvalError, Item, ItemStream},
    ir::{deserialize::IrDeserializer, serialize::IrSerializer},
    native::NativeFunctionRegistry,
    render::{
        compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
        TemplateData,
    },
    xpath::functions::{CemtXPathFunctions, XPathQueryItem},
};
use std::{collections::BTreeMap, sync::Arc};

const SOURCE: &str = r#"@doc cem-ml 1
@ns t = "https://cem.dev/ns/transform/cem/1"
@default t
{module |
    {function @name=demo.label @visibility=public @returns=string |
        {param @name=text @type=string @required=true}
        {body | {xpath @sequence-type="xs:string" |
            {variable @binding=text @local-name=text}
            {expression | $text || ' 🍒'}
    }   }   }
    {function @name=demo.pick @visibility=public @returns=any |
        {param @name=document @type=any @required=true}
        {body | {xpath @context=document @sequence-type="node()*" |
            {expression | /r/item}
    }   }   }
    {function @name=demo.accept @visibility=public @returns=boolean |
        {param @name=candidate @type=any @required=true}
        {param @name=minimum @type=integer @required=true}
        {body | {xpath @context=candidate @sequence-type="xs:boolean" |
            {variable @binding=minimum @local-name=minimum}
            {expression | exists(self::item[@qty >= $minimum])}
    }   }   }
}"#;

fn functions(source: &str) -> NativeFunctionRegistry {
    let functions = CemtXPathFunctions::compile(source, "memory:predicates.cemt").unwrap();
    let mut registry = NativeFunctionRegistry::default();
    functions
        .install(
            &mut registry,
            Arc::new(ResolverRegistry::new()),
            Arc::new(ResolverPolicy::new()),
        )
        .unwrap();
    registry
}

fn run(
    source: &str,
    bindings: BTreeMap<String, ItemStream>,
    functions: NativeFunctionRegistry,
) -> ItemStream {
    let query = compile(
        source,
        &CompileContext {
            policy_bindings: bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    evaluate(
        &query,
        &EvaluationContext {
            policy_bindings: bindings,
            native_functions: functions,
            ..Default::default()
        },
    )
}

fn xml(source: &str) -> (Item, Arc<LoadedInputAstStream>) {
    let (document, diagnostics) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: source.as_bytes(),
        source_uri: "memory:fruit.xml",
        content_type: Some("application/xml"),
    });
    assert!(diagnostics.is_empty());
    let owner = Arc::new(LoadedInputAstStream::XmlDocument(document.unwrap()));
    (
        XPathQueryItem::from_node(XPathNativeNode::xml_document(owner.clone()).unwrap()),
        owner,
    )
}

#[test]
fn authored_function_uses_declared_scalar_and_source_free_program() {
    let library = CemtXPathFunctions::compile(SOURCE, "memory:predicates.cemt").unwrap();
    assert_eq!(library.len(), 3);
    for function in library.functions() {
        assert!(function.expression().source_text.is_none());
        assert!(function.expression().tokens.is_empty());
        assert_eq!(
            function.artifact().identity().content_type,
            "application/vnd.cem.xpath-artifact+cem-bin"
        );
    }
    let functions = functions(SOURCE);
    let artifact = compile_template(
        r#"{p | {$native:call("demo.label", text)}}"#,
        &CompileTemplateOptions {
            host_bindings: vec!["text".into()],
            ..Default::default()
        },
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    for text in ["Hello", "Changed"] {
        let data = TemplateData {
            bindings: BTreeMap::from([(
                "text".into(),
                ItemStream::once(Item::Atomic(AtomValue::String(text.into()))),
            )]),
            native_functions: functions.clone(),
            ..Default::default()
        };
        let plan = render_compiled_template(&artifact, &data);
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        assert_eq!(render_plan_to_html(&plan), format!("<p>{text} 🍒</p>"));
    }
    let result = run(
        r#"native:call("demo.label", "Hi")"#,
        BTreeMap::new(),
        functions,
    );
    assert!(result.error.is_none(), "{result:?}");
    assert_eq!(
        result.items[0].atom(),
        Some(AtomValue::String("Hi 🍒".into()))
    );
    assert!(!result.items[0].source_map().unwrap().frames.is_empty());
}

#[test]
fn same_xpath_predicate_filters_in_ql_and_matches_in_cemt_without_losing_nodes() {
    let functions = functions(SOURCE);
    let template = compile_template(
        r#"{module |
        {template @mode=inspect @match='native:call("demo.accept", node, 2)' |
            {body | {b | {$node}}}}
        {template @mode=inspect @match=true @priority=-10 |
            {body | {i | {$node}}}}
        {body | {apply-templates @mode=inspect @select='native:call("demo.pick", input)'}}
    }"#,
        &CompileTemplateOptions::default(),
    );
    assert!(
        template.diagnostics.is_empty(),
        "{:?}",
        template.diagnostics
    );
    for (source, expected) in [
        (
            "<r><item qty='1'>🍋</item><item qty='2'>🍒</item></r>",
            "<i><item qty=\"1\">🍋</item></i><b><item qty=\"2\">🍒</item></b>",
        ),
        ("<r><item qty='3'>🍇</item></r>", "<b><item qty=\"3\">🍇</item></b>"),
    ] {
        let (input, owner) = xml(source);
        let bindings = BTreeMap::from([("input".into(), ItemStream::once(input))]);
        let filtered = run(
            r#"seq:where(native:call("demo.pick", input), fn(candidate) => native:call("demo.accept", candidate, 2))"#,
            bindings.clone(),
            functions.clone(),
        );
        assert!(filtered.error.is_none(), "{filtered:?}");
        assert_eq!(filtered.items.len(), 1);
        let node = filtered.items[0]
            .view()
            .unwrap()
            .downcast_ref::<XPathQueryItem>()
            .unwrap()
            .xpath_item()
            .native_node()
            .unwrap();
        assert!(Arc::ptr_eq(node.source_owner().as_ref().unwrap(), &owner));
        let plan = render_compiled_template(
            &template,
            &TemplateData {
                bindings,
                native_functions: functions.clone(),
                ..Default::default()
            },
        );
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        assert_eq!(render_plan_to_html(&plan), expected);
    }
}

#[test]
fn explicit_types_reject_generic_shapes_missing_capabilities_and_truthy_non_booleans() {
    for query in [
        r#"native:call("demo.label", 42)"#,
        r#"native:call("demo.label", ())"#,
        r#"native:call("demo.label", ("a", "b"))"#,
        r#"native:call("demo.pick", {kind: "document"})"#,
        r#"native:call("demo.pick", "<r/>")"#,
    ] {
        let result = run(query, BTreeMap::new(), functions(SOURCE));
        assert!(result.error.is_some(), "{query}: {result:?}");
        assert!(result.items.is_empty());
    }
    let missing = run(
        r#"native:call("demo.label", "Hi")"#,
        BTreeMap::new(),
        NativeFunctionRegistry::default(),
    );
    assert_eq!(
        missing.diagnostics[0].code,
        "cem.ql.native_function_unavailable"
    );
    let invalid = SOURCE.replace("exists(self::item[@qty >= $minimum])", "'false'");
    let (input, _) = xml("<r/>");
    let result = run(
        r#"native:call("demo.accept", input, 0)"#,
        BTreeMap::from([("input".into(), ItemStream::once(input))]),
        functions(&invalid),
    );
    assert!(result.error.is_some());
    assert!(result.items.is_empty());
    assert_eq!(
        result.diagnostics[0].code,
        "cem.ql.xpath_function_result_type"
    );
}

const IDENTITY: &str = r#"@doc cem-ml 1
@ns t = "https://cem.dev/ns/transform/cem/1"
@default t
{module |
    {function @name=demo.identity @visibility=public @returns=number |
        {param @name=value @type=number @required=true}
        {body | {xpath @sequence-type="item()*" |
            {variable @binding=value @local-name=value}
            {expression | $value}
    }   }   }
}"#;

#[test]
fn declared_numbers_keep_exact_types_and_nullable_means_empty_not_json_null() {
    let functions = functions(IDENTITY);
    for atom in [
        AtomValue::Integer(i64::MAX),
        AtomValue::Decimal("12345678901234567890.12345678901234567890".into()),
        AtomValue::Double(f64::INFINITY),
        AtomValue::Double(f64::NEG_INFINITY),
    ] {
        let result = run(
            r#"native:call("demo.identity", value)"#,
            BTreeMap::from([("value".into(), ItemStream::once(Item::Atomic(atom.clone())))]),
            functions.clone(),
        );
        assert!(result.error.is_none(), "{result:?}");
        assert_eq!(result.items[0].atom(), Some(atom));
    }
    let result = run(
        r#"native:call("demo.identity", value)"#,
        BTreeMap::from([(
            "value".into(),
            ItemStream::once(Item::Atomic(AtomValue::Decimal("1e3".into()))),
        )]),
        functions,
    );
    assert_eq!(result.diagnostics[0].code, "cem.ql.xpath_function_argument");

    let nullable = IDENTITY
        .replace("@returns=number", "@returns=number @nullable=true")
        .replace("@type=number", "@type=number @nullable=true");
    let result = run(
        r#"native:call("demo.identity", ())"#,
        BTreeMap::new(),
        self::functions(&nullable),
    );
    assert!(result.error.is_none(), "{result:?}");
    assert!(result.items.is_empty());
    for query in [
        r#"native:call("demo.identity", null)"#,
        r#"native:call("demo.identity")"#,
    ] {
        assert!(run(query, BTreeMap::new(), self::functions(&nullable))
            .error
            .is_some());
    }
}

#[test]
fn declaration_errors_private_functions_and_registration_conflicts_are_isolated() {
    for source in [
        SOURCE.replace("@required=true", "@required=false"),
        SOURCE.replace("@type=string", "@type=object"),
        SOURCE.replace("@returns=string", "@returns=array"),
        SOURCE.replace("@type=string", "@type=string @default=hello"),
        SOURCE.replace("@binding=text", "@binding=missing"),
        SOURCE.replace("@context=document", "@context=missing"),
        SOURCE.replace("@name=demo.pick", "@name=demo.label"),
        SOURCE.replace("xs:string", "xs:date"),
        SOURCE.replace("@default t", "@ns xs = \"urn:not-schema\"\n@default t"),
        SOURCE.replace("{module |", "{module | {import @as=other @src=other.cemt}"),
    ] {
        assert!(
            CemtXPathFunctions::compile(&source, "memory:bad.cemt").is_err(),
            "{source}"
        );
    }
    assert!(CemtXPathFunctions::compile(&"x".repeat(32769), "memory:large.cemt").is_err());
    let private = SOURCE.replace("@visibility=public", "@visibility=private");
    assert!(CemtXPathFunctions::compile(&private, "memory:private.cemt")
        .unwrap()
        .is_empty());
    assert_eq!(
        run(
            r#"native:call("demo.label", "x")"#,
            BTreeMap::new(),
            functions(&private)
        )
        .diagnostics[0]
            .code,
        "cem.ql.native_function_unavailable"
    );

    // Collision at the last export must not install the earlier exports.
    let existing = IDENTITY.replace("demo.identity", "demo.accept").replace(
        "{param @name=value",
        "{param @name=unused @type=integer @required=true}\n{param @name=value",
    );
    let mut registry = functions(&existing);
    let library = CemtXPathFunctions::compile(SOURCE, "memory:predicates.cemt").unwrap();
    assert!(library
        .install(
            &mut registry,
            Arc::new(ResolverRegistry::new()),
            Arc::new(ResolverPolicy::new())
        )
        .is_err());
    assert_eq!(
        run(
            r#"native:call("demo.label", "x")"#,
            BTreeMap::new(),
            registry.clone()
        )
        .diagnostics[0]
            .code,
        "cem.ql.native_function_unavailable"
    );
    assert_eq!(
        run(
            r#"native:call("demo.accept", 0, 7)"#,
            BTreeMap::new(),
            registry
        )
        .items[0]
            .atom(),
        Some(AtomValue::Integer(7))
    );
}

#[test]
fn results_enforce_both_declared_return_type_and_xpath_cardinality() {
    for source in [
        IDENTITY.replace("{expression | $value}", "{expression | ($value, $value)}"),
        IDENTITY.replace("{expression | $value}", "{expression | ()}"),
        IDENTITY.replace("item()*", "xs:boolean"),
        IDENTITY.replace("item()*", "node()*"),
        IDENTITY.replace("item()*", "empty-sequence()"),
    ] {
        let result = run(
            r#"native:call("demo.identity", 7)"#,
            BTreeMap::new(),
            functions(&source),
        );
        assert_eq!(
            result.diagnostics[0].code, "cem.ql.xpath_function_result_type",
            "{result:?}"
        );
        assert!(result.items.is_empty());
        assert_eq!(
            result.diagnostics[0].uri.as_deref(),
            Some("memory:predicates.cemt")
        );
        assert!(!result.diagnostics[0]
            .source_map
            .as_ref()
            .unwrap()
            .frames
            .is_empty());
    }
    // A native boolean can feed a second named function without stringification.
    let source = IDENTITY.replace("number", "boolean");
    let result = run(
        r#"native:call("demo.identity", native:call("demo.identity", false))"#,
        BTreeMap::new(),
        functions(&source),
    );
    assert!(result.error.is_none(), "{result:?}");
    assert_eq!(result.items[0].atom(), Some(AtomValue::Boolean(false)));
}

#[test]
fn query_reload_needs_explicit_functions_and_keeps_native_node_identity() {
    let (input, _) = xml("<r><item>🍒</item></r>");
    let bindings = BTreeMap::from([("input".into(), ItemStream::once(input))]);
    let query = compile(
        r#"(native:call("demo.pick", input), native:call("demo.pick", input))"#,
        &CompileContext {
            policy_bindings: bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let reloaded = IrDeserializer::deserialize(&IrSerializer::serialize(&query)).unwrap();
    let context = EvaluationContext {
        policy_bindings: bindings,
        ..Default::default()
    };
    assert_eq!(
        evaluate(&reloaded, &context).diagnostics[0].code,
        "cem.ql.native_function_unavailable"
    );
    let result = evaluate(
        &reloaded,
        &EvaluationContext {
            native_functions: functions(SOURCE),
            ..context
        },
    );
    assert!(result.error.is_none(), "{result:?}");
    assert_eq!(result.items.len(), 2);
    assert_eq!(result.items[0].identity(), result.items[1].identity());
}

#[test]
fn xpath_limits_and_unsupported_capabilities_cannot_be_caught_as_data_errors() {
    let source = IDENTITY
        .replace("@returns=number", "@returns=any")
        .replace("{expression | $value}", "{expression | 1 to 1000}");
    let query = compile(
        r#"try { native:call("demo.identity", 1) } catch (code, message) { "caught" }"#,
        &CompileContext::default(),
    )
    .unwrap();
    let context = EvaluationContext {
        native_functions: functions(&source),
        scope_policy: ScopePolicy::host_root().with_queue_size(16),
        ..Default::default()
    };
    let limited = evaluate(&query, &context);
    assert_eq!(
        limited.error,
        Some(EvalError::BudgetExceeded(BudgetAxis::ItemsPerStage)),
        "{limited:?}"
    );
    assert!(limited.items.is_empty());

    let unsupported = IDENTITY.replace("{expression | $value}", "{expression | current-dateTime()}");
    let result = evaluate(
        &query,
        &EvaluationContext {
            native_functions: functions(&unsupported),
            ..Default::default()
        },
    );
    assert!(
        matches!(result.error, Some(EvalError::Unsupported(_))),
        "{result:?}"
    );
    assert!(result.items.is_empty());
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.xpath.evaluation_unsupported"));

    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    let result = evaluate_with_control(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
    assert!(
        matches!(
            result.error,
            Some(EvalError::Cancelled | EvalError::Control(_))
        ),
        "{result:?}"
    );
    assert!(result.items.is_empty());
}

#[test]
fn installed_xpath_text_and_work_limits_survive_ql_try_catch() {
    use cem_ml::validation::xpath::XPathEvaluationLimits;
    let library = CemtXPathFunctions::compile(SOURCE, "memory:text-functions.cemt").unwrap();
    for (limits, axis, code) in [
        (
            XPathEvaluationLimits {
                max_text_bytes: Some(8),
                ..Default::default()
            },
            BudgetAxis::XPathTextBytes,
            "cem.xpath.text_byte_limit_exceeded",
        ),
        (
            XPathEvaluationLimits {
                max_work_units: Some(8),
                ..Default::default()
            },
            BudgetAxis::XPathWorkUnits,
            "cem.xpath.work_limit_exceeded",
        ),
    ] {
        let mut registry = NativeFunctionRegistry::default();
        library
            .install_with_limits(
                &mut registry,
                Arc::new(ResolverRegistry::new()),
                Arc::new(ResolverPolicy::new()),
                limits,
            )
            .unwrap();
        let result = run(
            r#"try { native:call("demo.label", "hello") } catch (code, message) { "caught" }"#,
            BTreeMap::new(),
            registry,
        );
        assert_eq!(
            result.error,
            Some(EvalError::BudgetExceeded(axis)),
            "{result:?}"
        );
        assert!(result.items.is_empty());
        let diagnostic = result.diagnostics.iter().find(|d| d.code == code).unwrap();
        assert_eq!(
            diagnostic.uri.as_deref(),
            Some("memory:text-functions.cemt")
        );
        assert!(diagnostic.source_map.is_some() && diagnostic.byte_offset.is_some());
    }
}

#[test]
fn authored_text_demo_library_executes_through_native_calls() {
    let library = include_str!("../../cem-elements/demo/xpath-text.cemt");
    for (query, expected) in [
        (
            r#"native:call("text.words", "a a 🍒")"#,
            AtomValue::Integer(3),
        ),
        (
            r#"native:call("text.length", "🍒é")"#,
            AtomValue::Integer(3),
        ),
        (
            r#"native:call("text.normalize", "  a  a  ")"#,
            AtomValue::String("a a".into()),
        ),
        (
            r#"native:call("text.join", "a a b", "/")"#,
            AtomValue::String("a/a/b".into()),
        ),
    ] {
        let result = run(query, BTreeMap::new(), functions(library));
        assert!(result.error.is_none(), "{result:?}");
        assert_eq!(result.items[0].atom(), Some(expected));
    }
}

#[test]
fn node_demo_sorting_preserves_exact_native_nodes_and_navigation() {
    let registry = functions(include_str!("../../cem-elements/demo/xpath-nodes.cemt"));
    let (input, owner) = xml("<r xmlns:a='urn:a' xmlns:b='urn:b'><a:item id='2' qty='10'>Z<![CDATA[est]]></a:item><b:item id='1' qty='2'>Apple</b:item></r>");
    let bindings = BTreeMap::from([("input".into(), ItemStream::once(input))]);
    let rows = run(
        r#"native:call("node.rows", input)"#,
        bindings.clone(),
        registry.clone(),
    );
    let sorted = run(
        r#"seq:sorted(native:call("node.rows", input), fn(row) => native:call("node.value", row), "ascending", "text")"#,
        bindings,
        registry.clone(),
    );
    assert!(sorted.error.is_none(), "{sorted:?}");
    assert_eq!(sorted.items.len(), 2);
    for (item, original) in sorted.items.iter().zip(rows.items.iter().rev()) {
        let node = item
            .view()
            .unwrap()
            .downcast_ref::<XPathQueryItem>()
            .unwrap()
            .xpath_item()
            .native_node()
            .unwrap();
        let original = original
            .view()
            .unwrap()
            .downcast_ref::<XPathQueryItem>()
            .unwrap()
            .xpath_item()
            .native_node()
            .unwrap();
        assert_eq!(node, original);
        assert!(Arc::ptr_eq(node.source_owner().as_ref().unwrap(), &owner));
    }
    let apple = BTreeMap::from([("row".into(), ItemStream::once(sorted.items[0].clone()))]);
    for (function, expected) in [
        ("local", "item"),
        ("parent", "r"),
        ("previous", "Zest"),
        ("id", "1"),
    ] {
        let result = run(
            &format!("native:call(\"node.{function}\", row)"),
            apple.clone(),
            registry.clone(),
        );
        assert!(result.error.is_none(), "{result:?}");
        assert_eq!(
            result.items[0].atom(),
            Some(AtomValue::String(expected.into()))
        );
    }
    let namespace = run(
        r#"native:call("node.namespace", row)"#,
        apple.clone(),
        registry.clone(),
    );
    assert_eq!(
        namespace.items[0].atom(),
        Some(AtomValue::AnyUri("urn:b".into()))
    );
    let quantity = run(r#"native:call("node.quantity", row)"#, apple, registry);
    assert_eq!(
        quantity.items[0].atom(),
        Some(AtomValue::String("2".into()))
    );
}

#[test]
fn authored_node_table_and_tree_render_natively_before_browser_integration() {
    let registry = functions(include_str!("../../cem-elements/demo/xpath-nodes.cemt"));
    let page = include_str!("../../cem-elements/demo/xpath-nodes.html");
    for (index, source) in page
        .split("<template type=\"text/cem-ml\"")
        .skip(1)
        .enumerate()
    {
        let source = source
            .split_once('>')
            .unwrap()
            .1
            .split_once("</template>")
            .unwrap()
            .0
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&");
        let template = compile_template(&source, &CompileTemplateOptions::default());
        assert!(
            template.diagnostics.is_empty(),
            "case {index}: {:?}",
            template.diagnostics
        );
        let input = if index == 0 {
            "<basket xmlns:a='urn:a' xmlns:b='urn:b'><a:item id='2' qty='10'>Z<![CDATA[est]]></a:item><b:item id='1' qty='2'>Apple</b:item></basket>"
        } else {
            "<note xmlns='urn:notes' mood='bright'>Hello <![CDATA[& welcome]]><em xmlns='urn:marks'>friend</em>!</note>"
        };
        let slices = Item::Record(BTreeMap::from_iter([
            (
                "source".into(),
                vec![Item::Atomic(AtomValue::String(input.into()))],
            ),
            (
                "direction".into(),
                vec![Item::Atomic(AtomValue::String("ascending".into()))],
            ),
            (
                "selected".into(),
                vec![Item::Atomic(AtomValue::String("1".into()))],
            ),
        ]));
        let mut data = TemplateData {
            bindings: BTreeMap::from([(
                "datadom".into(),
                ItemStream::once(Item::Record(BTreeMap::from([(
                    "slices".into(),
                    vec![slices],
                )]))),
            )]),
            native_functions: registry.clone(),
            ..Default::default()
        };
        for (name, value) in [
            ("source", input),
            ("direction", "ascending"),
            ("selected", "1"),
        ] {
            data.bindings.insert(
                name.into(),
                ItemStream::once(Item::Atomic(AtomValue::String(value.into()))),
            );
        }
        let plan = render_compiled_template(&template, &data);
        assert!(
            plan.diagnostics.is_empty(),
            "case {index}: {:?}",
            plan.diagnostics
        );
        let html = render_plan_to_html(&plan);
        if index == 0 {
            assert!(html.contains("<output>basket</output>"), "{html}");
            assert!(html.contains("<output>Zest</output>"), "{html}");
            assert!(html.find("<td>Apple</td>").unwrap() < html.find("<td>Zest</td>").unwrap());
            assert!(html.contains("urn:b"));
        } else {
            for expected in [
                "<code>bright</code>",
                "<code>Hello &amp; welcome</code>",
                "<code>friend</code>",
                "urn:marks",
            ] {
                assert!(html.contains(expected), "{html}");
            }
        }
    }
}

#[test]
fn sequence_demo_keeps_first_seen_columns_in_authored_xpath_and_native_cells() {
    let registry = functions(include_str!("../../cem-elements/demo/xpath-sequences.cemt"));
    let (input, owner) = xml("<r xmlns:x='urn:x'><row id='2'><fruit>A</fruit></row><row id='1' qty='3'><fruit>C</fruit><x:fruit>B</x:fruit><fruit>D</fruit><note/></row></r>");
    let bindings = BTreeMap::from([("input".into(), ItemStream::once(input))]);
    let columns = run(
        r#"native:call("table.columns", input)"#,
        bindings.clone(),
        registry.clone(),
    );
    assert!(columns.error.is_none(), "{columns:?}");
    assert_eq!(columns.items.len(), 5);
    for column in &columns.items {
        let native = column
            .view()
            .unwrap()
            .downcast_ref::<XPathQueryItem>()
            .unwrap()
            .xpath_item()
            .native_node()
            .unwrap();
        assert!(Arc::ptr_eq(native.source_owner().as_ref().unwrap(), &owner));
    }
    let headings = run(
        r#"for column in native:call("table.columns", input) { native:call("table.heading", column) }"#,
        bindings.clone(),
        registry.clone(),
    );
    assert!(headings.error.is_none(), "{headings:?}");
    assert_eq!(
        headings
            .items
            .iter()
            .map(|item| item.atom().unwrap())
            .collect::<Vec<_>>(),
        ["@id", "fruit", "@qty", "fruit [urn:x]", "note"].map(|s| AtomValue::String(s.into()))
    );
    let cells = run(
        r#"for row in native:call("table.rows", input) { for column in native:call("table.columns", input) { native:call("table.value", native:call("table.cells", row, column)) } }"#,
        bindings,
        registry.clone(),
    );
    assert!(cells.error.is_none(), "{cells:?}");
    assert_eq!(
        cells
            .items
            .iter()
            .map(|item| item.atom().unwrap())
            .collect::<Vec<_>>(),
        ["2", "A", "∅", "∅", "∅", "1", "C / D", "3", "B", "\"\""]
            .map(|s| AtomValue::String(s.into()))
    );
    for (query, expected) in [
        (
            r#"str:concat(native:call("sequence.window", "a b a c", "2", "2", true), "|")"#,
            "a|b",
        ),
        (
            r#"native:call("sequence.head", native:call("sequence.window", "a b a c", "2", "2", false))"#,
            "b",
        ),
        (
            r#"native:call("sequence.tail", native:call("sequence.window", "a b a c", "2", "2", false))"#,
            "a",
        ),
    ] {
        let result = run(query, BTreeMap::new(), registry.clone());
        assert!(result.error.is_none(), "{result:?}");
        assert_eq!(
            result.items[0].atom(),
            Some(AtomValue::String(expected.into()))
        );
    }
    let count = run(
        r#"native:call("sequence.unique", "a b a c")"#,
        BTreeMap::new(),
        registry,
    );
    assert_eq!(count.items[0].atom(), Some(AtomValue::Integer(3)));
}

#[test]
fn authored_sequence_demo_templates_render_natively() {
    let registry = functions(include_str!("../../cem-elements/demo/xpath-sequences.cemt"));
    let page = include_str!("../../cem-elements/demo/xpath-sequences.html");
    for (index, source) in page
        .split("<template type=\"text/cem-ml\"")
        .skip(1)
        .enumerate()
    {
        let source = source
            .split_once('>')
            .unwrap()
            .1
            .split_once("</template>")
            .unwrap()
            .0
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&");
        let template = compile_template(&source, &CompileTemplateOptions::default());
        assert!(
            template.diagnostics.is_empty(),
            "{index}: {:?}",
            template.diagnostics
        );
        let data = TemplateData {
            native_functions: registry.clone(),
            ..Default::default()
        };
        let result = render_compiled_template(&template, &data);
        assert!(
            result.diagnostics.is_empty(),
            "{index}: {:?}",
            result.diagnostics
        );
        let html = render_plan_to_html(&result);
        if index == 0 {
            assert!(!html.contains(" checked"), "{html}");
        }
        for expected in if index == 0 {
            vec![
                "<output>3</output>",
                "<output>apple | cherry</output>",
                "<output>apple</output>",
                "<output>cherry</output>",
            ]
        } else {
            vec![
                ">@id</th>",
                ">fruit</th>",
                ">@qty</th>",
                ">note</th>",
                "<td>∅</td>",
                "<td>Apple</td>",
            ]
        } {
            assert!(
                html.contains(expected),
                "{index} expected {expected}: {html}"
            );
        }
    }
}

#[test]
fn authored_aggregate_demo_templates_render_natively() {
    let registry = functions(include_str!(
        "../../cem-elements/demo/xpath-aggregates.cemt"
    ));
    let page = include_str!("../../cem-elements/demo/xpath-aggregates.html");
    for (index, source) in page
        .split("<template type=\"text/cem-ml\"")
        .skip(1)
        .enumerate()
    {
        let source = source
            .split_once('>')
            .unwrap()
            .1
            .split_once("</template>")
            .unwrap()
            .0
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&");
        let template = compile_template(&source, &CompileTemplateOptions::default());
        assert!(
            template.diagnostics.is_empty(),
            "{index}: {:?}",
            template.diagnostics
        );
        let data = TemplateData {
            native_functions: registry.clone(),
            ..Default::default()
        };
        let result = render_compiled_template(&template, &data);
        assert!(
            result.diagnostics.is_empty(),
            "{index}: {:?}",
            result.diagnostics
        );
        let html = render_plan_to_html(&result);
        for expected in if index == 0 {
            vec![
                "<output>0.3</output>",
                "<output>0.1</output>",
                "<output>0.2</output>",
                "<output>0.15</output>",
            ]
        } else {
            vec![
                ">apple</th>",
                ">cherry</th>",
                "<td>1.25</td>",
                "<output>3.75</output>",
                "<output>1.875</output>",
            ]
        } {
            assert!(
                html.contains(expected),
                "{index} expected {expected}: {html}"
            );
        }
    }
}

#[test]
fn aggregate_library_validates_before_casting_and_includes_new_native_fruits() {
    let registry = functions(include_str!(
        "../../cem-elements/demo/xpath-aggregates.cemt"
    ));
    for (source, valid, expected) in [
        (
            "<basket><apple>0.1</apple><cherry>0.2</cherry><pear>0.6</pear></basket>",
            true,
            vec!["0.9", "0.1", "0.6", "0.3"],
        ),
        ("<basket/>", true, vec!["0", "∅", "∅", "∅"]),
        ("<basket><pear>bad</pear></basket>", false, vec![]),
        ("<basket><pear>-1</pear></basket>", false, vec![]),
        ("<basket><pear/></basket>", false, vec![]),
        (
            "<basket><pear><nested>2</nested></pear></basket>",
            false,
            vec![],
        ),
        ("<wrong/>", false, vec![]),
    ] {
        let (input, owner) = xml(source);
        let bindings = BTreeMap::from([("input".into(), ItemStream::once(input))]);
        let result = run(
            r#"native:call("basket.valid", input)"#,
            bindings.clone(),
            registry.clone(),
        );
        assert!(result.error.is_none(), "{result:?}");
        assert_eq!(
            result.items[0].atom().unwrap(),
            AtomValue::Boolean(valid),
            "{source}"
        );
        if valid {
            let result = run(
                r#"let values = native:call("basket.values", input); (native:call("aggregate.sum", values), native:call("aggregate.min", values), native:call("aggregate.max", values), native:call("aggregate.avg", values))"#,
                bindings.clone(),
                registry.clone(),
            );
            assert!(result.error.is_none(), "{result:?}");
            assert_eq!(
                result
                    .items
                    .iter()
                    .map(|v| v.atom().unwrap())
                    .collect::<Vec<_>>(),
                expected
                    .into_iter()
                    .map(|v| AtomValue::String(v.into()))
                    .collect::<Vec<_>>()
            );
            let rows = run(
                r#"native:call("basket.rows", input)"#,
                bindings,
                registry.clone(),
            );
            for row in rows.items {
                let native = row
                    .view()
                    .unwrap()
                    .downcast_ref::<XPathQueryItem>()
                    .unwrap()
                    .xpath_item()
                    .native_node()
                    .unwrap();
                assert!(Arc::ptr_eq(native.source_owner().as_ref().unwrap(), &owner));
            }
        }
    }
}

#[test]
fn containers_cross_named_functions_opaquely_and_retain_nested_owners() {
    let registry = functions(include_str!(
        "../../cem-elements/demo/xpath-maps-arrays.cemt"
    ));
    let (input, owner) = xml("<basket><apple>2</apple><pear>3</pear></basket>");
    let bindings = BTreeMap::from([("input".into(), ItemStream::once(input))]);
    let packed = run(
        r#"native:call("basket.pack", input)"#,
        bindings,
        registry.clone(),
    );
    assert!(packed.error.is_none(), "{packed:?}");
    assert_eq!(packed.items.len(), 1);
    assert!(packed.items[0].atom().is_none());
    assert!(packed.items[0].view().unwrap().field("fruits").is_none());
    let bindings = BTreeMap::from([("basket".into(), packed)]);
    let fruits = run(
        r#"native:call("basket.fruits", basket)"#,
        bindings.clone(),
        registry.clone(),
    );
    assert!(fruits.error.is_none(), "{fruits:?}");
    assert_eq!(fruits.items.len(), 1, "an array remains one opaque item");
    assert!(fruits.items[0].atom().is_none());
    let selected = run(
        r#"native:call("basket.pick", basket, "2")"#,
        bindings.clone(),
        registry.clone(),
    );
    assert!(selected.error.is_none(), "{selected:?}");
    assert_eq!(selected.items.len(), 1);
    let node = selected.items[0]
        .view()
        .unwrap()
        .downcast_ref::<XPathQueryItem>()
        .unwrap()
        .xpath_item()
        .native_node()
        .unwrap();
    assert!(Arc::ptr_eq(node.source_owner().as_ref().unwrap(), &owner));
    assert_eq!(node.string_value(), "3");
    for (query, expected) in [
        (
            r#"native:call("basket.note", basket)"#,
            "Empty member (array size 1)",
        ),
        (
            r#"native:call("basket.label", native:call("basket.pick", basket, "0"))"#,
            "No member at this position",
        ),
    ] {
        let result = run(query, bindings.clone(), registry.clone());
        assert!(result.error.is_none(), "{result:?}");
        assert_eq!(
            result.items[0].atom(),
            Some(AtomValue::String(expected.into()))
        );
    }
    for (state, expected, count) in [
        ("absent", "Absent entry", 2),
        ("empty", "Present, empty sequence", 3),
        ("value", "Local preview", 3),
    ] {
        let result = run(
            &format!(
                r#"let filter = native:call("filter.make", "192.0.2.0/24", "allow", "{state}"); (native:call("filter.note", filter), native:call("filter.count", filter))"#
            ),
            BTreeMap::new(),
            registry.clone(),
        );
        assert!(result.error.is_none(), "{result:?}");
        assert_eq!(
            result
                .items
                .iter()
                .map(|i| i.atom().unwrap())
                .collect::<Vec<_>>(),
            [
                AtomValue::String(expected.into()),
                AtomValue::Integer(count)
            ]
        );
    }
}

#[test]
fn authored_map_array_templates_render_natively() {
    let registry = functions(include_str!(
        "../../cem-elements/demo/xpath-maps-arrays.cemt"
    ));
    let page = include_str!("../../cem-elements/demo/xpath-maps-arrays.html");
    for (index, source) in page
        .split("<template type=\"text/cem-ml\"")
        .skip(1)
        .enumerate()
    {
        let source = source
            .split_once('>')
            .unwrap()
            .1
            .split_once("</template>")
            .unwrap()
            .0
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&");
        let template = compile_template(&source, &CompileTemplateOptions::default());
        assert!(
            template.diagnostics.is_empty(),
            "{:?}",
            template.diagnostics
        );
        let plan = render_compiled_template(
            &template,
            &TemplateData {
                native_functions: registry.clone(),
                ..Default::default()
            },
        );
        assert!(
            plan.diagnostics.is_empty(),
            "{index}: {:?}",
            plan.diagnostics
        );
        let html = render_plan_to_html(&plan);
        let expected: &[&str] = match index {
            0 => &["allow: 192.0.2.0/24", "Absent entry", "2"],
            1 => &["2", "apple: 2", "Empty member (array size 1)"],
            _ => &["3 members; numeric total 5", "Null value"],
        };
        for text in expected {
            assert!(html.contains(&format!("<output>{text}</output>")), "{html}");
        }
    }
}

#[test]
fn fenced_xpath_container_errors_keep_original_cemt_coordinates_after_reload() {
    let source = IDENTITY.replace("@returns=number", "@returns=any").replace(
        "{expression | $value}",
        "{expression |```\nmap { 'x': 1, 'x': 2 }\n```}",
    );
    let library = CemtXPathFunctions::compile(&source, "memory:fenced.cemt").unwrap();
    let expression = library.functions()[0].expression();
    assert!(expression.source_text.is_none());
    assert!(expression.tokens.is_empty());
    assert_eq!(
        expression
            .syntax_ast
            .as_ref()
            .unwrap()
            .root
            .source_range
            .start
            .byte_offset,
        source.find("map {").unwrap() as u64
    );
    let result = run(
        r#"native:call("demo.identity", 1)"#,
        BTreeMap::new(),
        functions(&source),
    );
    let diagnostic = &result.diagnostics[0];
    assert_eq!(diagnostic.code, "cem.xpath.map_duplicate_key");
    assert_eq!(
        diagnostic.byte_offset,
        Some(source.rfind("'x'").unwrap() as u64)
    );
    assert!(diagnostic.source_map.is_some());
}

#[test]
fn sort_library_uses_captured_direction_and_preserves_source_selection() {
    let source = include_str!("../../cem-elements/demo/xpath-sort.cemt");
    let functions = functions(source);
    for (numeric, descending, expected) in [
        (false, false, vec!["02", "1", "10", "2", "bad"]),
        (true, false, vec!["1", "2", "02", "10", "bad"]),
        (true, true, vec!["10", "2", "02", "1", "bad"]),
    ] {
        let query =
            format!("native:call(\"sort.words\", \"10 2 02 bad 1\", {numeric}, {descending})");
        let result = run(&query, BTreeMap::new(), functions.clone());
        assert!(result.error.is_none(), "{result:?}");
        assert_eq!(
            result
                .items
                .iter()
                .map(|item| item.atom().unwrap())
                .collect::<Vec<_>>(),
            expected
                .into_iter()
                .map(|value| AtomValue::String(value.into()))
                .collect::<Vec<_>>()
        );
    }
    let (document, _) = xml("<r><row id='a' group='B' qty='2'>Pear</row><row id='b' group='A' qty='10'>Apple</row><row id='c' group='A' qty='2'>Cherry</row><row id='d' group='A' qty='2'>Plum</row><row id='e' group='A' qty='bad'>Kiwi</row><row id='f' group='B'>Mango</row></r>");
    let bindings = BTreeMap::from([("document".into(), ItemStream::once(document))]);
    for (descending, expected) in [
        (
            false,
            vec!["Cherry", "Plum", "Apple", "Pear", "Kiwi", "Mango"],
        ),
        (
            true,
            vec!["Apple", "Cherry", "Plum", "Pear", "Kiwi", "Mango"],
        ),
    ] {
        let query = format!("seq:map(native:call(\"sort.rows\", document, {descending}), fn(row) => native:call(\"row.label\", row))");
        let result = run(&query, bindings.clone(), functions.clone());
        assert!(result.error.is_none(), "{result:?}");
        assert_eq!(
            result
                .items
                .iter()
                .map(|item| item.atom().unwrap())
                .collect::<Vec<_>>(),
            expected
                .into_iter()
                .map(|value| AtomValue::String(value.into()))
                .collect::<Vec<_>>()
        );
    }
    let selected = run("seq:map(native:call(\"sort.selected\", document, \"c\"), fn(row) => native:call(\"row.previous\", row))", bindings, functions);
    assert!(selected.error.is_none(), "{selected:?}");
    assert_eq!(
        selected.items[0].atom(),
        Some(AtomValue::String("Apple".into()))
    );
}

#[test]
fn inline_closures_cannot_escape_the_closed_named_function_contract() {
    for body in [
        "function($x) {$x}",
        "map {'f': function($x) {$x}}",
        "[function($x) {$x}]",
    ] {
        let source = format!(
            r#"@doc cem-ml 1
@ns t = "https://cem.dev/ns/transform/cem/1"
@default t
{{module | {{function @name=test.escape @visibility=public @returns=any |
    {{body | {{xpath @sequence-type="item()*" |
        {{expression | ```{body}```}}
}} }} }} }}"#
        );
        let result = run(
            "native:call(\"test.escape\")",
            BTreeMap::new(),
            functions(&source),
        );
        assert!(result.error.is_some(), "executable closure escaped: {body}");
    }
}

#[test]
fn inline_recursion_budget_cannot_be_caught_as_a_data_error() {
    let source = r#"@doc cem-ml 1
@ns t = "https://cem.dev/ns/transform/cem/1"
@default t
{module | {function @name=test.recurse @visibility=public @returns=any |
    {body | {xpath @sequence-type="item()*" |
        {expression | ```let $f := function($self) {$self($self)} return $f($f)```}
} } } }"#;
    let result = run(
        "try { native:call(\"test.recurse\") } catch (code, message) { \"caught\" }",
        BTreeMap::new(),
        functions(source),
    );
    assert_eq!(
        result.error,
        Some(EvalError::BudgetExceeded(BudgetAxis::CallDepth))
    );
    assert!(result.items.is_empty());
}

#[test]
fn regex_validation_library_combines_lexical_numeric_and_quantified_rules() {
    let functions = functions(include_str!("../../cem-elements/demo/xpath-validation.cemt"));
    for (query, expected) in [
        (r#"native:call("form.user", "Ada_7")"#, "Valid user name"),
        (r#"native:call("form.user", "7Ada")"#, "Use 3–16 ASCII letters, digits or underscores; start with a letter"),
        (r#"native:call("form.age", "21")"#, "Age in range"),
        (r#"native:call("form.age", "17")"#, "Use an age from 18 to 120"),
        (r#"native:call("form.age", "1e2")"#, "Enter an integer age"),
        (r#"native:call("form.tags", "Blue, green; RED")"#, "Blue / green / RED"),
        (r#"native:call("form.tags", "red,,blue")"#, "Use 1–12 ASCII letters per tag, separated by commas or semicolons"),
        (r#"native:call("form.tags", "")"#, "Use 1–12 ASCII letters per tag, separated by commas or semicolons"),
        (r#"native:call("ip.preview", "192.0.2.10/24", "24 32")"#, "Allowed by the local prefix rule"),
        (r#"native:call("ip.preview", "192.0.2.10/16", "24 32")"#, "Blocked by the local prefix rule"),
        (r#"native:call("ip.preview", "255.255.255.255", "32")"#, "Allowed by the local prefix rule"),
        (r#"native:call("ip.preview", "0.0.0.0/0", "0")"#, "Allowed by the local prefix rule"),
        (r#"native:call("ip.preview", "256.0.2.10/24", "24")"#, "Octets must be 0–255 and prefix length 0–32"),
        (r#"native:call("ip.preview", "192.0.2.10/33", "24")"#, "Octets must be 0–255 and prefix length 0–32"),
        (r#"native:call("ip.preview", "192.00.2.10/24", "24")"#, "Enter IPv4 with an optional /prefix; no leading zeros"),
        (r#"native:call("ip.preview", "::1", "32")"#, "Enter IPv4 with an optional /prefix; no leading zeros"),
        (r#"native:call("ip.preview", "192.0.2.10/24", "x")"#, "Enter allowed prefix lengths from 0 to 32"),
    ] {
        let result = run(query, BTreeMap::new(), functions.clone());
        assert!(result.error.is_none(), "{query}: {result:?}");
        assert_eq!(result.items[0].atom(), Some(AtomValue::String(expected.into())), "{query}");
    }
}

#[test]
fn regex_subset_and_resource_errors_cannot_be_caught_as_data_errors() {
    for (body, error) in [
        ("matches('aa', '(a)\\1')", EvalError::Unsupported("XPath capability or operation control unavailable")),
        ("matches('a', 'a{1025}')", EvalError::BudgetExceeded(BudgetAxis::XPathWorkUnits)),
    ] {
        let source = format!(r#"@doc cem-ml 1
@ns t = "https://cem.dev/ns/transform/cem/1"
@default t
{{module | {{function @name=test.regex @visibility=public @returns=boolean |
    {{body | {{xpath @sequence-type="xs:boolean" |
        {{expression | ```{body}```}}
}} }} }} }}"#);
        let result = run(r#"try { native:call("test.regex") } catch (code, message) { "caught" }"#,
            BTreeMap::new(), functions(&source));
        assert_eq!(result.error, Some(error));
        assert!(result.items.is_empty());
    }
}
