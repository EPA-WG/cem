use std::collections::BTreeMap;
use std::sync::Arc;

use cem_ml::engine::FormatIdentity;
use cem_ml::lifecycle::LoadedInputAstStream;
use cem_ml::query::{
    QueryEvaluatorAdapter, QueryExecutionLimits, QueryExecutionRequest, QueryLanguage,
    QueryNativeBindings,
};
use cem_ml::resolver::{ResolverPolicy, ResolverRegistry};
use cem_ml::scheduler::{AbortSignal, ScopePolicy};
use cem_ml::schema::registry::{
    CSS_SELECTOR_CONTENT_TYPE, HTML_CONTENT_TYPE, HTML_SCHEMA_URI, JSON_CONTENT_TYPE,
    JSON_VALUE_SCHEMA_URI, XML_CONTENT_TYPE, XML_SCHEMA_URI,
};
use cem_ml::validation::css_selector::{
    css_selector_expression_ast_from_source_bytes, CemCssSelectorEvaluator,
    CssSelectorAttributeOperator, CssSelectorCombinator, CssSelectorElementTreeOwner,
    CssSelectorFactKind, CssSelectorNamespace, CssSelectorResultArtifact,
    CssSelectorSimpleSelector, CssSelectorSourceRequest,
};
use cem_ml::validation::html::{html_document_ast_from_source_bytes, HtmlSourceValidationRequest};
use cem_ml::validation::json::{json_document_ast_from_source_bytes, JsonSourceValidationRequest};
use cem_ml::validation::xml::{xml_document_ast_from_source_bytes, XmlSourceValidationRequest};

fn parse_selector(
    source: &str,
    namespaces: &BTreeMap<String, String>,
) -> (
    cem_ml::validation::css_selector::CssSelectorExpressionAst,
    Vec<cem_ml::diagnostics::Diagnostic>,
) {
    let (expression, diagnostics) =
        css_selector_expression_ast_from_source_bytes(CssSelectorSourceRequest {
            bytes: source.as_bytes(),
            source_uri: "memory:query.css-selector",
            content_type: Some(CSS_SELECTOR_CONTENT_TYPE),
            namespace_bindings: namespaces,
        });
    (
        expression.expect("UTF-8 selector source produces an AST owner"),
        diagnostics,
    )
}

fn html_owner(source: &str) -> CssSelectorElementTreeOwner {
    let (document, diagnostics) =
        html_document_ast_from_source_bytes(HtmlSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "memory:data.html",
            content_type: Some(HTML_CONTENT_TYPE),
        });
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.severity.is_hard_violation()),
        "HTML lifecycle source must load cleanly: {diagnostics:#?}"
    );
    CssSelectorElementTreeOwner::from_lifecycle(
        Arc::new(LoadedInputAstStream::HtmlDocument(
            document.expect("HTML lifecycle AST"),
        )),
        FormatIdentity {
            content_type: Some(HTML_CONTENT_TYPE.to_owned()),
            schema: Some(HTML_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        },
    )
    .expect("HTML exposes a borrowed element-tree view")
}

fn xml_owner(source: &str) -> CssSelectorElementTreeOwner {
    let (document, diagnostics) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: source.as_bytes(),
        source_uri: "memory:data.xml",
        content_type: Some(XML_CONTENT_TYPE),
    });
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.severity.is_hard_violation()),
        "XML lifecycle source must load cleanly: {diagnostics:#?}"
    );
    CssSelectorElementTreeOwner::from_lifecycle(
        Arc::new(LoadedInputAstStream::XmlDocument(
            document.expect("XML lifecycle AST"),
        )),
        FormatIdentity {
            content_type: Some(XML_CONTENT_TYPE.to_owned()),
            schema: Some(XML_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        },
    )
    .expect("XML exposes a borrowed element-tree view")
}

fn evaluate(
    expression: &cem_ml::validation::css_selector::CssSelectorExpressionAst,
    owner: &CssSelectorElementTreeOwner,
    namespaces: &BTreeMap<String, String>,
    limits: QueryExecutionLimits,
) -> Result<cem_ml::query::QueryExecutionResult, Vec<cem_ml::diagnostics::Diagnostic>> {
    let bindings = QueryNativeBindings::new();
    let resolver_registry = ResolverRegistry::new();
    let resolver_policy = ResolverPolicy::new();
    let scope_policy = ScopePolicy::host_root();
    let abort_signal = AbortSignal::new();
    let operation_control = cem_ml::operation_control::OperationControl::new(abort_signal.clone());
    CemCssSelectorEvaluator::default().evaluate(QueryExecutionRequest {
        language: QueryLanguage::CssSelector,
        query_ast_owner: expression,
        input_ast_owner: owner,
        context_item: None,
        bindings: &bindings,
        namespace_bindings: namespaces,
        resolver_registry: &resolver_registry,
        resolver_policy: &resolver_policy,
        resolver_policy_stamp: "resolver-policy/1",
        safety_policy_stamp: "query-safety/1",
        scope_policy: &scope_policy,
        operation_control: &operation_control,
        execution_scope: cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
        abort_signal: &abort_signal,
        limits,
    })
}

#[test]
fn parser_retains_lossless_tokens_ranges_and_typed_selector_shape() {
    let source = "main#app > article.card[data-state~=\"ready\"]";
    let (expression, diagnostics) = parse_selector(source, &BTreeMap::new());
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert_eq!(
        expression
            .tokens
            .iter()
            .map(|token| token.lexeme.as_str())
            .collect::<String>(),
        source
    );
    for window in expression.tokens.windows(2) {
        assert_eq!(
            window[0].source_range.end_byte_offset(),
            window[1].source_range.start.byte_offset
        );
    }

    let list = expression
        .selector_list
        .as_ref()
        .expect("typed selector list");
    assert_eq!(list.selectors.len(), 1);
    let selector = &list.selectors[0];
    assert_eq!(selector.compounds.len(), 2);
    assert_eq!(selector.combinators, vec![CssSelectorCombinator::Child]);
    assert!(matches!(
        selector.compounds[0].simple_selectors.as_slice(),
        [CssSelectorSimpleSelector::Type { local_name, .. }, CssSelectorSimpleSelector::Id { value, .. }]
            if local_name == "main" && value == "app"
    ));
    assert!(selector.compounds[1].simple_selectors.iter().any(
        |simple| matches!(simple, CssSelectorSimpleSelector::Attribute {
            local_name,
            operator: Some(CssSelectorAttributeOperator::Includes),
            value: Some(value),
            ..
        } if local_name == "data-state" && value == "ready")
    ));
}

#[test]
fn manifest_examples_prove_source_maps_namespace_matching_and_relational_budget_safety() {
    let source_map_selector = include_str!(
        "../schema-packages/css-selector/v1/examples/source-map-selector.css-selector"
    );
    let (source_map_expression, diagnostics) =
        parse_selector(source_map_selector, &BTreeMap::new());
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert_eq!(
        source_map_expression
            .tokens
            .iter()
            .map(|token| token.lexeme.as_str())
            .collect::<String>(),
        source_map_selector
    );
    let owner = html_owner(
        r#"<catalog><book id="featured-primary" class="featured"></book><book id="featured-secondary"></book></catalog>"#,
    );
    let result = evaluate(
        &source_map_expression,
        &owner,
        &BTreeMap::new(),
        QueryExecutionLimits::default(),
    )
    .expect("manifest source-map selector evaluates");
    let artifact = result
        .native_result
        .as_any()
        .downcast_ref::<CssSelectorResultArtifact>()
        .expect("CSS selector result artifact");
    assert_eq!(artifact.matches.len(), 2);
    assert!(artifact
        .matches
        .windows(2)
        .all(|pair| pair[0].document_order < pair[1].document_order));
    assert!(artifact
        .matches
        .iter()
        .all(|matched| !matched.source_map.frames.is_empty()));

    let namespace_selector =
        include_str!("../schema-packages/css-selector/v1/examples/namespace-wildcard.css-selector");
    let (namespace_expression, diagnostics) = parse_selector(namespace_selector, &BTreeMap::new());
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let namespace_owner = xml_owner(
        r#"<catalog xmlns="urn:catalog"><book id="featured"/><magazine id="monthly"/></catalog>"#,
    );
    let result = evaluate(
        &namespace_expression,
        &namespace_owner,
        &BTreeMap::new(),
        QueryExecutionLimits::default(),
    )
    .expect("manifest namespace wildcard evaluates");
    let artifact = result
        .native_result
        .as_any()
        .downcast_ref::<CssSelectorResultArtifact>()
        .expect("CSS selector result artifact");
    assert_eq!(artifact.matches.len(), 1);
    assert_eq!(artifact.matches[0].namespace_uri, "urn:catalog");

    let relational_selector = include_str!(
        "../schema-packages/css-selector/v1/examples/budgeted-relational.css-selector"
    );
    let (relational_expression, diagnostics) =
        parse_selector(relational_selector, &BTreeMap::new());
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let exhausted = match evaluate(
        &relational_expression,
        &owner,
        &BTreeMap::new(),
        QueryExecutionLimits {
            max_result_items: Some(10),
            max_work_units: Some(1),
            ..Default::default()
        },
    ) {
        Ok(_) => panic!("manifest relational selector must consume an explicit work budget"),
        Err(diagnostics) => diagnostics,
    };
    assert!(exhausted
        .iter()
        .any(|diagnostic| diagnostic.code == "css-selector.budget.exceeded"));
}

#[test]
fn parser_binds_namespaces_and_reports_unbound_prefixes_from_schema_policy() {
    let source = "svg|svg > svg|a[href]";
    let namespaces = BTreeMap::from([("svg".to_owned(), "http://www.w3.org/2000/svg".to_owned())]);
    let (expression, diagnostics) = parse_selector(source, &namespaces);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let first =
        &expression.selector_list.as_ref().unwrap().selectors[0].compounds[0].simple_selectors[0];
    assert!(matches!(
        first,
        CssSelectorSimpleSelector::Type {
            namespace: CssSelectorNamespace::Named { prefix, namespace_uri },
            local_name,
            ..
        } if prefix == "svg"
            && namespace_uri == "http://www.w3.org/2000/svg"
            && local_name == "svg"
    ));

    let (_, unbound) = parse_selector(source, &BTreeMap::new());
    let unbound_diagnostic = unbound
        .iter()
        .find(|diagnostic| diagnostic.code == "css-selector.namespace.unbound")
        .expect("schema maps the namespace fact to a diagnostic");
    assert!(unbound_diagnostic.severity.is_hard_violation());
    assert_eq!(
        unbound_diagnostic
            .details
            .as_ref()
            .and_then(|details| details["schemaBehavior"].as_str()),
        Some("css-selector-report-fact")
    );

    let owner = xml_owner(r#"<svg xmlns="http://www.w3.org/2000/svg"><a href="first"/></svg>"#);
    let mismatch = match evaluate(
        &expression,
        &owner,
        &BTreeMap::new(),
        QueryExecutionLimits::default(),
    ) {
        Ok(_) => panic!("execution must retain the parsed namespace static context"),
        Err(diagnostics) => diagnostics,
    };
    assert!(mismatch
        .iter()
        .any(|diagnostic| diagnostic.code == "css-selector.input.unsupported"));
}

#[test]
fn parser_reports_invalid_and_unsupported_features_without_silent_non_matches() {
    let namespaces = BTreeMap::new();
    let (invalid_utf8, lexical) =
        css_selector_expression_ast_from_source_bytes(CssSelectorSourceRequest {
            bytes: &[0xff],
            source_uri: "memory:invalid.css-selector",
            content_type: Some(CSS_SELECTOR_CONTENT_TYPE),
            namespace_bindings: &namespaces,
        });
    assert!(invalid_utf8.is_none());
    assert!(lexical
        .iter()
        .any(|diagnostic| diagnostic.code == "css-selector.lexical.invalid"));

    let (_, invalid) = parse_selector("article[", &BTreeMap::new());
    assert!(invalid
        .iter()
        .any(|diagnostic| diagnostic.code == "css-selector.parse.invalid"));

    let (_, highlight) = parse_selector("article::highlight(cem-source-name)", &BTreeMap::new());
    assert!(highlight
        .iter()
        .any(|diagnostic| diagnostic.code == "css-selector.feature.unsupported"));
    let (_, unsupported) = parse_selector("article::before", &BTreeMap::new());
    assert!(unsupported
        .iter()
        .any(|diagnostic| diagnostic.code == "css-selector.feature.unsupported"));
    let (_, column) = parse_selector("col || td", &BTreeMap::new());
    assert!(column
        .iter()
        .any(|diagnostic| diagnostic.code == "css-selector.feature.unsupported"));

    for selector in [
        "::before",
        "::MARKER",
        ":before",
        ":after",
        ":first-line",
        ":first-letter",
    ] {
        let (_, diagnostics) = parse_selector(selector, &BTreeMap::new());
        assert!(diagnostics
            .iter()
            .any(|d| d.code == "css-selector.feature.unsupported"));
    }
    for selector in [":host", ":host(.active)"] {
        let (_, diagnostics) = parse_selector(selector, &BTreeMap::new());
        assert!(diagnostics
            .iter()
            .any(|d| d.code == "css-selector.capability.missing"));
    }
    let (_, capability) = parse_selector("a:hover", &BTreeMap::new());
    assert!(capability
        .iter()
        .any(|diagnostic| diagnostic.code == "css-selector.capability.missing"));
}

#[test]
fn evaluator_matches_native_html_elements_in_document_order_and_eliminates_duplicates() {
    let owner = html_owner(
        r#"<main id="app"><article class="card" data-state="not-ready ready"><h2>Ready</h2></article><article id="other" class="card"></article></main>"#,
    );
    let (expression, diagnostics) = parse_selector(
        "main#app > article.card[data-state~=\"ready\"], #other, article.card",
        &BTreeMap::new(),
    );
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let result = evaluate(
        &expression,
        &owner,
        &BTreeMap::new(),
        QueryExecutionLimits::default(),
    )
    .expect("native selector evaluation");
    let artifact = result
        .native_result
        .as_any()
        .downcast_ref::<CssSelectorResultArtifact>()
        .expect("CSS selector result artifact");
    assert_eq!(artifact.matches.len(), 2);
    assert_eq!(
        artifact
            .matches
            .iter()
            .map(|matched| matched.local_name.as_str())
            .collect::<Vec<_>>(),
        vec!["article", "article"]
    );
    assert!(artifact.matches[0].document_order < artifact.matches[1].document_order);
    assert_ne!(artifact.matches[0].node_id, artifact.matches[1].node_id);
    assert!(artifact
        .matches
        .iter()
        .all(|matched| !matched.source_map.frames.is_empty()));
}

#[test]
fn evaluator_supports_budgeted_relational_and_logical_pseudo_classes() {
    let owner = html_owner(
        r#"<main><section><h2>Visible</h2></section><section hidden><h2>Hidden</h2></section><section><p>Other</p></section></main>"#,
    );
    let (expression, diagnostics) =
        parse_selector("section:has(> h2):not([hidden])", &BTreeMap::new());
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let result = evaluate(
        &expression,
        &owner,
        &BTreeMap::new(),
        QueryExecutionLimits {
            max_result_items: Some(10),
            max_work_units: Some(100),
            ..Default::default()
        },
    )
    .expect("relational selector evaluation");
    let artifact = result
        .native_result
        .as_any()
        .downcast_ref::<CssSelectorResultArtifact>()
        .unwrap();
    assert_eq!(artifact.matches.len(), 1);
    assert_eq!(artifact.matches[0].local_name, "section");

    let complex_owner = html_owner(
        r#"<main><section><div><h2 class="deep">Nested</h2></div></section><section><h2>Direct</h2></section></main>"#,
    );
    let (complex_expression, diagnostics) =
        parse_selector("section:has(> div h2.deep)", &BTreeMap::new());
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let complex_result = evaluate(
        &complex_expression,
        &complex_owner,
        &BTreeMap::new(),
        QueryExecutionLimits::default(),
    )
    .expect("complex relative selector evaluation");
    let complex_artifact = complex_result
        .native_result
        .as_any()
        .downcast_ref::<CssSelectorResultArtifact>()
        .unwrap();
    assert_eq!(complex_artifact.matches.len(), 1);

    let exhausted = match evaluate(
        &expression,
        &owner,
        &BTreeMap::new(),
        QueryExecutionLimits {
            max_result_items: Some(10),
            max_work_units: Some(1),
            ..Default::default()
        },
    ) {
        Ok(_) => panic!("relational traversal must consume work budget"),
        Err(diagnostics) => diagnostics,
    };
    assert!(exhausted
        .iter()
        .any(|diagnostic| diagnostic.code == "css-selector.budget.exceeded"));
}

#[test]
fn evaluator_supports_required_combinators_and_attribute_operators() {
    let owner = html_owner(
        r#"<main><article data-exact="Value" data-list="one two" lang="en-US" data-text="Start-MIDDLE-End"><h2></h2><p></p><span></span><aside></aside></article></main>"#,
    );
    for (selector, expected_name) in [
        ("main article", "article"),
        ("main > article", "article"),
        ("h2 + p", "p"),
        ("h2 ~ aside", "aside"),
        (
            "article[data-exact=\"Value\"][data-list~=two][lang|=en][data-text^=start i][data-text$=end i][data-text*=middle i]",
            "article",
        ),
    ] {
        let (expression, diagnostics) = parse_selector(selector, &BTreeMap::new());
        assert!(diagnostics.is_empty(), "{selector}: {diagnostics:#?}");
        let result = evaluate(
            &expression,
            &owner,
            &BTreeMap::new(),
            QueryExecutionLimits::default(),
        )
        .unwrap_or_else(|diagnostics| panic!("{selector}: {diagnostics:#?}"));
        let artifact = result
            .native_result
            .as_any()
            .downcast_ref::<CssSelectorResultArtifact>()
            .expect("CSS selector result artifact");
        assert_eq!(artifact.matches.len(), 1, "{selector}");
        assert_eq!(artifact.matches[0].local_name, expected_name, "{selector}");
    }
}

#[test]
fn evaluator_matches_namespaced_xml_without_serializing_the_lifecycle_ast() {
    let owner =
        xml_owner(r#"<svg xmlns="http://www.w3.org/2000/svg"><a href="first"/><g><a/></g></svg>"#);
    let namespaces = BTreeMap::from([("svg".to_owned(), "http://www.w3.org/2000/svg".to_owned())]);
    let (expression, diagnostics) = parse_selector("svg|svg > svg|a[href]", &namespaces);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let result = evaluate(
        &expression,
        &owner,
        &namespaces,
        QueryExecutionLimits::default(),
    )
    .expect("namespace-aware XML selector evaluation");
    let artifact = result
        .native_result
        .as_any()
        .downcast_ref::<CssSelectorResultArtifact>()
        .expect("CSS selector result artifact");
    assert_eq!(artifact.matches.len(), 1);
    assert_eq!(artifact.matches[0].namespace_uri, namespaces["svg"]);
    assert_eq!(artifact.matches[0].local_name, "a");
    let retained_owner = result
        .input_ast_owner
        .as_any()
        .downcast_ref::<CssSelectorElementTreeOwner>()
        .expect("result retains the CSS selector element-tree owner");
    assert!(Arc::ptr_eq(
        owner.lifecycle_owner(),
        retained_owner.lifecycle_owner()
    ));
}

#[test]
fn element_tree_owner_rejects_lifecycle_inputs_without_an_element_view() {
    let (document, diagnostics) =
        json_document_ast_from_source_bytes(JsonSourceValidationRequest {
            bytes: br#"{"not":"an element tree"}"#,
            source_uri: "memory:data.json",
            content_type: Some(JSON_CONTENT_TYPE),
        });
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let fact = CssSelectorElementTreeOwner::from_lifecycle(
        Arc::new(LoadedInputAstStream::JsonDocument(
            document.expect("JSON lifecycle AST"),
        )),
        FormatIdentity {
            content_type: Some(JSON_CONTENT_TYPE.to_owned()),
            schema: Some(JSON_VALUE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        },
    )
    .expect_err("JSON must not be coerced into an element-tree view");
    assert_eq!(fact.kind, CssSelectorFactKind::InputUnsupported);
}

#[test]
fn stylesheet_literal_and_structural_support_does_not_expand_query_capabilities() {
    for selector in [
        ":dir(rtl)",
        ":dir(ltr)",
        ":lang(en, \"fr\")",
        ":nth-child(odd)",
        ":nth-child(1 of .item)",
        ":nth-last-child(2n)",
        ":nth-of-type(1)",
        ":nth-last-of-type(-n+2)",
    ] {
        let (_, diagnostics) = parse_selector(selector, &BTreeMap::new());
        assert!(!diagnostics.is_empty(), "{selector}");
    }
}

#[test]
fn matches_keep_original_input_uri_and_byte_ranges_separate_from_query_source() {
    use cem_ml::source_map::{FrameSpan, TransformKind};
    for (source, owner, needle) in [
        (
            "<catalog>\r\n  <item title=\"café &amp; tea\"/>\r\n</catalog>",
            xml_owner as fn(&str) -> CssSelectorElementTreeOwner,
            "<item title=\"café &amp; tea\"/>",
        ),
        (
            "<main>\r\n  <item title=\"café &amp; tea\"></item>\r\n</main>",
            html_owner as fn(&str) -> CssSelectorElementTreeOwner,
            "<item title=\"café &amp; tea\">",
        ),
    ] {
        let owner = owner(source);
        assert!(std::ptr::eq(
            owner
                .retained_tree()
                .native_owner()
                .unwrap()
                .downcast_ref::<LoadedInputAstStream>()
                .unwrap(),
            owner.lifecycle_owner().as_ref()
        ));
        let (expression, diagnostics) =
            parse_selector("item[title='café & tea']", &BTreeMap::new());
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let result = evaluate(
            &expression,
            &owner,
            &BTreeMap::new(),
            QueryExecutionLimits::default(),
        )
        .unwrap();
        let artifact = result
            .native_result
            .as_any()
            .downcast_ref::<CssSelectorResultArtifact>()
            .unwrap();
        assert_eq!(artifact.matches.len(), 1);
        let matched = &artifact.matches[0];
        let offset = source.find(needle).unwrap() as u64;
        assert_eq!(
            matched.source_map.origin().unwrap().span,
            FrameSpan::Single(cem_ml::source::ByteRange::new(offset, needle.len() as u32))
        );
        assert_eq!(matched.source_map.frames.len(), 2);
        let uri = if source.starts_with("<catalog>") {
            "memory:data.xml"
        } else {
            "memory:data.html"
        };
        assert!(
            matches!(&matched.source_map.frames[1].transform, TransformKind::SourceBoundary { uri: actual } if actual == uri)
        );
        assert_eq!(
            matched.source_map.frames[0].span,
            matched.source_map.frames[1].span
        );
        // The result's query map is a distinct source, not an input-map fallback.
        assert_eq!(artifact.source_map, expression.source_map);
        assert_ne!(artifact.source_map, matched.source_map);
        let retained = result
            .input_ast_owner
            .as_any()
            .downcast_ref::<CssSelectorElementTreeOwner>()
            .unwrap();
        assert!(Arc::ptr_eq(
            owner.lifecycle_owner(),
            retained.lifecycle_owner()
        ));
    }
}

#[test]
fn literal_element_import_keeps_cem_payloads_as_inert_elements_and_stable_event_ids() {
    let source = r##"<root xmlns:cem="https://cem.dev/ns/cem-ml/1"><cem:expr>#not-an-admitted-query</cem:expr><item cem:query="#also-not-a-query"/></root>"##;
    let owner = xml_owner(source);
    let namespaces = BTreeMap::from([("cem".into(), "https://cem.dev/ns/cem-ml/1".into())]);
    let (expression, diagnostics) = parse_selector("cem|expr + item[cem|query]", &namespaces);
    assert!(diagnostics.is_empty());
    let result = evaluate(
        &expression,
        &owner,
        &namespaces,
        QueryExecutionLimits::default(),
    )
    .unwrap();
    let artifact = result
        .native_result
        .as_any()
        .downcast_ref::<CssSelectorResultArtifact>()
        .unwrap();
    assert_eq!(artifact.matches.len(), 1);
    assert_eq!(artifact.matches[0].node_id, "xml:event:4");
    assert_eq!(artifact.matches[0].local_name, "item");
}

#[test]
fn element_import_refuses_partial_unsafe_and_over_limit_sources() {
    for source in [
        "<root><item></root>".to_owned(),
        format!("{}{}", "<item>".repeat(66), "</item>".repeat(66)),
    ] {
        let (document, _) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "memory:unsafe.xml",
            content_type: Some(XML_CONTENT_TYPE),
        });
        assert!(CssSelectorElementTreeOwner::from_lifecycle(
            Arc::new(LoadedInputAstStream::XmlDocument(document.unwrap())),
            FormatIdentity::default()
        )
        .is_err());
    }
    let (document, _) = html_document_ast_from_source_bytes(HtmlSourceValidationRequest {
        bytes: b"<button onclick='neverRun()'>Unsafe</button>",
        source_uri: "memory:unsafe.html",
        content_type: Some(HTML_CONTENT_TYPE),
    });
    assert!(CssSelectorElementTreeOwner::from_lifecycle(
        Arc::new(LoadedInputAstStream::HtmlDocument(document.unwrap())),
        FormatIdentity::default()
    )
    .is_err());
    let (document, _) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: b"<root/>",
        source_uri: "memory:large.xml",
        content_type: Some(XML_CONTENT_TYPE),
    });
    let mut document = document.unwrap();
    document.source.byte_length = cem_ml::import::MAX_DOCUMENT_BYTES + 1;
    assert!(CssSelectorElementTreeOwner::from_lifecycle(
        Arc::new(LoadedInputAstStream::XmlDocument(document)),
        FormatIdentity::default()
    )
    .unwrap_err()
    .message
    .contains("16 MiB"));
}

#[test]
fn element_import_preserves_xml_family_wrapper_owners_and_compact_rng_refusal() {
    use cem_ml::import::element_tree::retain_element_lifecycle;
    use cem_ml::validation::{mathml, relax_ng, svg, xhtml, xslt};
    let uri = "memory:family.xml";
    let owners = [
        LoadedInputAstStream::XhtmlDocument(xhtml::xhtml_document_ast_from_source_bytes(xhtml::XhtmlSourceValidationRequest {
            bytes: br#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>Example</title></head><body><p>Text</p></body></html>"#,
            source_uri: uri, content_type: Some("application/xhtml+xml"),
        }).0.unwrap()),
        LoadedInputAstStream::SvgDocument(svg::svg_document_ast_from_source_bytes(svg::SvgSourceValidationRequest {
            bytes: br#"<svg xmlns="http://www.w3.org/2000/svg"><title>Example</title></svg>"#,
            source_uri: uri, content_type: Some("image/svg+xml"),
        }).0.unwrap()),
        LoadedInputAstStream::MathMlDocument(mathml::mathml_document_ast_from_source_bytes(mathml::MathMlSourceValidationRequest {
            bytes: br#"<math xmlns="http://www.w3.org/1998/Math/MathML"><mi>x</mi></math>"#,
            source_uri: uri, content_type: Some("application/mathml+xml"),
        }).0.unwrap()),
        LoadedInputAstStream::XsltStylesheet(xslt::xslt_stylesheet_ast_from_source_bytes(xslt::XsltSourceValidationRequest {
            bytes: br#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0"><xsl:template match="/"><output/></xsl:template></xsl:stylesheet>"#,
            source_uri: uri, content_type: Some("application/xslt+xml"),
        }).0.unwrap()),
        LoadedInputAstStream::RelaxNgDocument(relax_ng::relax_ng_document_ast_from_source_bytes(relax_ng::RelaxNgSourceValidationRequest {
            bytes: br#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="item"><text/></element>"#,
            source_uri: uri, content_type: Some("application/relax-ng+xml"),
        }).0.unwrap()),
    ];
    for owner in owners {
        let owner = Arc::new(owner);
        let imported = retain_element_lifecycle(owner.clone()).unwrap();
        assert!(imported.elements.len() >= 2);
        let native = imported
            .tree
            .native_owner()
            .unwrap()
            .clone()
            .downcast::<LoadedInputAstStream>()
            .unwrap();
        assert!(Arc::ptr_eq(&owner, &native));
        for (id, _, _) in imported.elements {
            assert!(imported.tree.node(id).unwrap().name.is_some());
            assert_eq!(imported.tree.node_source_uri(id), Some(uri));
            assert_eq!(imported.tree.node_source_map(id).unwrap().frames.len(), 2);
        }
    }
    let compact = relax_ng::relax_ng_document_ast_from_source_bytes(
        relax_ng::RelaxNgSourceValidationRequest {
            bytes: b"start = element item { text }",
            source_uri: "memory:grammar.rnc",
            content_type: Some("application/relax-ng-compact-syntax"),
        },
    )
    .0
    .unwrap();
    assert!(
        retain_element_lifecycle(Arc::new(LoadedInputAstStream::RelaxNgDocument(compact)))
            .unwrap_err()
            .contains("compact syntax")
    );
}

#[test]
fn imported_attribute_name_policy_preserves_html_case_xml_case_and_namespace_queries() {
    for (owner, selector, namespaces, count) in [
        (html_owner(r#"<main><item data-title="ready"></item></main>"#), "item[DATA-TITLE=ready]", BTreeMap::new(), 1),
        (xml_owner(r#"<root><item data-title="ready"/></root>"#), "item[DATA-TITLE=ready]", BTreeMap::new(), 0),
        (xml_owner(r#"<root xmlns:p="urn:product"><item p:Title="ready"/></root>"#), "item[p|Title=ready]", BTreeMap::from([("p".into(), "urn:product".into())]), 1),
        (xml_owner(r#"<root xmlns:p="urn:product"><item p:Title="ready"/></root>"#), "[*|p]", BTreeMap::new(), 1),
    ] {
        let (expression, diagnostics) = parse_selector(selector, &namespaces);
        assert!(diagnostics.is_empty(), "{selector}: {diagnostics:?}");
        let result = evaluate(&expression, &owner, &namespaces, QueryExecutionLimits::default()).unwrap();
        let artifact = result.native_result.as_any().downcast_ref::<CssSelectorResultArtifact>().unwrap();
        assert_eq!(artifact.matches.len(), count, "{selector}");
    }
}
