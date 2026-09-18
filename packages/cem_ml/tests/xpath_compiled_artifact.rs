//! XPATH-COMPILED-ARTIFACT: portable programs belong to XPath, not its hosts.
use cem_ml::{
    content_cache::ContentHash,
    lifecycle::LoadedInputAstStream,
    resolver::{ResolverPolicy, ResolverRegistry},
    schema::registry::{
        SchemaRegistry, CEM_QL_ARTIFACT_CONTENT_TYPE, XPATH_ARTIFACT_CONTENT_TYPE,
        XPATH_CONTENT_TYPE, XPATH_SCHEMA_URI,
    },
    validation::{
        xml::{xml_document_ast_from_source_bytes, XmlSourceValidationRequest},
        xpath::{
            artifact::{XPathArtifactLoadContext, XPathCompiledArtifact},
            XPathAttachment, XPathDynamicContext, XPathEvaluationRequest, XPathInvocationHost,
            XPathNativeNode, XPathResultItem, XsltXPathInvocationAdapter,
        },
        xslt::{xslt_stylesheet_ast_from_source_bytes, XsltSourceValidationRequest},
    },
};
use std::sync::Arc;

const STYLESHEET: &str = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:fruit" version="3.0"><xsl:template match="/"><xsl:value-of select="/f:r/f:item[@qty &gt; 1]/text()"/></xsl:template></xsl:stylesheet>"#;

fn compile() -> XPathCompiledArtifact {
    let (stylesheet, diagnostics) =
        xslt_stylesheet_ast_from_source_bytes(XsltSourceValidationRequest {
            bytes: STYLESHEET.as_bytes(),
            source_uri: "memory:viewer.xslt",
            content_type: Some("application/xslt+xml"),
        });
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let expression = stylesheet
        .unwrap()
        .xpath_expressions
        .into_iter()
        .find(|slot| slot.attribute_name == "select")
        .unwrap()
        .expression;
    XPathCompiledArtifact::compile(&expression, ContentHash::from_blake3(STYLESHEET.as_bytes()))
        .unwrap()
}

fn load_context() -> XPathArtifactLoadContext {
    XPathArtifactLoadContext {
        expected_source_hash: ContentHash::from_blake3(STYLESHEET.as_bytes()),
        invocation_host: XPathInvocationHost::Xslt,
    }
}

#[test]
fn xpath_owns_source_namespace_and_distinct_compiled_media_type() {
    assert_eq!(XPATH_SCHEMA_URI, "https://cem.dev/ns/query/xpath/1");
    assert_eq!(XPATH_CONTENT_TYPE, "application/vnd.cem.xpath");
    assert_eq!(
        XPATH_ARTIFACT_CONTENT_TYPE,
        "application/vnd.cem.xpath-artifact+cem-bin"
    );
    assert_ne!(XPATH_ARTIFACT_CONTENT_TYPE, CEM_QL_ARTIFACT_CONTENT_TYPE);
    let registry = SchemaRegistry::with_builtin_schemas();
    let package = registry
        .resolve_content_type(XPATH_ARTIFACT_CONTENT_TYPE)
        .unwrap();
    assert_eq!(package.schema_uri, XPATH_SCHEMA_URI);
}

#[test]
fn deterministic_binary_reload_retains_typed_host_without_xpath_source() {
    let first = compile();
    let second = compile();
    // Optional cross-target fixtures for the downstream WASM smoke check.
    // These files contain executable bytes and explicit control metadata only.
    if let Some(directory) = std::env::var_os("CEM_XPATH_ARTIFACT_FIXTURE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        let mut standalone = parse("1 + 2");
        standalone.attachment = XPathAttachment::Standalone { source_id: 1 };
        let standalone =
            XPathCompiledArtifact::compile(&standalone, ContentHash::from_blake3(b"1 + 2"))
                .unwrap();
        let container_source =
            "let $m := map {'rows': [(),(1,2)]} return ($m?rows?(2), $m ! ?rows?*)";
        let mut containers = parse(container_source);
        containers.attachment = XPathAttachment::Standalone { source_id: 1 };
        let containers = XPathCompiledArtifact::compile(
            &containers,
            ContentHash::from_blake3(container_source.as_bytes()),
        )
        .unwrap();
        let mut manifest = Vec::new();
        for (name, artifact, host) in [
            ("xslt", &first, "xslt"),
            ("standalone", &standalone, "query"),
            ("containers", &containers, "query"),
        ] {
            std::fs::write(directory.join(format!("{name}.bin")), artifact.bytes()).unwrap();
            manifest.push(serde_json::json!({
                "name": name, "host": host,
                "contentHash": artifact.content_hash().header_value(),
                "sourceHash": artifact.identity().source_hash.header_value(),
            }));
        }
        std::fs::write(
            directory.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
    }
    assert_eq!(first.bytes(), second.bytes());
    assert_eq!(first.content_hash(), second.content_hash());
    assert_eq!(first.identity().content_type, XPATH_ARTIFACT_CONTENT_TYPE);
    let restored =
        XPathCompiledArtifact::from_bytes(first.bytes().to_vec(), first.content_hash()).unwrap();
    let expression = restored.reload(&load_context()).unwrap();
    assert!(expression.source_text.is_none());
    assert!(expression.tokens.is_empty());
    assert!(expression.events.is_empty());
    assert!(!expression.syntax_ast.as_ref().unwrap().events.is_empty());
    let XPathAttachment::Host(host) = &expression.attachment else {
        panic!("XSLT attachment lost")
    };
    assert_eq!(host.owner.source_uri, "memory:viewer.xslt");
    assert_eq!(
        host.static_context.namespaces.get("f").unwrap(),
        "urn:fruit"
    );
    assert!(host.expression_range.start.byte_offset > 0);
    // No retained stylesheet, callback, or source string is needed after load.
    drop(first);
    drop(second);
    for (source, expected) in [
        ("<r xmlns='urn:fruit'><item qty='1'>skip</item><item qty='2'>a<![CDATA[b]]>&amp;</item></r>", "ab&"),
        ("<r xmlns='urn:fruit'><item qty='3'>🍒</item></r>", "🍒"),
    ] {
        let (document, diagnostics) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
            bytes: source.as_bytes(), source_uri: "memory:input.xml", content_type: Some("application/xml"),
        });
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let owner = Arc::new(LoadedInputAstStream::XmlDocument(document.unwrap()));
        let registry = ResolverRegistry::new();
        let policy = ResolverPolicy::new();
        let result = XsltXPathInvocationAdapter.invoke_with_control(XPathEvaluationRequest {
            invocation_host: XPathInvocationHost::Xslt, expression: &expression,
            dynamic_context: XPathDynamicContext {
                context_item: Some(XPathResultItem::from_native_node(XPathNativeNode::xml_document(owner.clone()).unwrap())),
                ..Default::default()
            },
            static_context: host.static_context.clone(), expected_result: host.expected_result.clone(),
            resolver_registry: &registry, resolver_policy: &policy,
            evaluation_limits: Default::default(), safety_policy_stamp: "artifact-test", module_resolution: None,
        }, &Default::default(), cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID).unwrap();
        assert_eq!(result.sequence.items.len(), 1);
        let node = result.sequence.items[0].native_node().unwrap();
        assert!(Arc::ptr_eq(node.source_owner().as_ref().unwrap(), &owner));
        assert_eq!(node.string_value(), expected);
        assert!(!node.source_map().frames.is_empty());
    }
}

#[test]
fn artifact_loader_rejects_corruption_truncation_wrong_host_and_source() {
    let artifact = compile();
    for length in [0, 1, artifact.bytes().len() - 1] {
        let bytes = artifact.bytes()[..length].to_vec();
        assert!(XPathCompiledArtifact::from_bytes(
            bytes.clone(),
            &ContentHash::from_blake3(&bytes)
        )
        .is_err());
    }
    let mut corrupt = artifact.bytes().to_vec();
    *corrupt.last_mut().unwrap() ^= 1;
    assert!(XPathCompiledArtifact::from_bytes(corrupt, artifact.content_hash()).is_err());
    let mut wrong = load_context();
    wrong.invocation_host = XPathInvocationHost::CemQl;
    assert!(artifact.reload(&wrong).is_err());
    wrong = load_context();
    wrong.expected_source_hash = ContentHash::from_blake3(b"different stylesheet");
    assert!(artifact.reload(&wrong).is_err());
    let mut trailing = artifact.bytes().to_vec();
    trailing.push(0);
    assert!(XPathCompiledArtifact::from_bytes(
        trailing.clone(),
        &ContentHash::from_blake3(&trailing)
    )
    .is_err());
    // An arbitrary XML association is not an executable CEMT/XSLT slot.
    // Match the existing invocation adapters; do not infer a language host.
    use cem_ml::validation::xpath::XPathHostNodeKind;
    for kind in [
        XPathHostNodeKind::XmlDocument,
        XPathHostNodeKind::XmlSubtree,
        XPathHostNodeKind::XmlElement,
        XPathHostNodeKind::XmlAttribute,
    ] {
        let mut expression = artifact.reload(&load_context()).unwrap();
        let XPathAttachment::Host(host) = &mut expression.attachment else {
            unreachable!()
        };
        host.owner.node_kind = kind;
        let associated =
            XPathCompiledArtifact::compile(&expression, load_context().expected_source_hash)
                .unwrap();
        for invocation_host in [
            XPathInvocationHost::Query,
            XPathInvocationHost::StandaloneTransform,
            XPathInvocationHost::Cemt,
            XPathInvocationHost::CemQl,
            XPathInvocationHost::Xslt,
        ] {
            assert!(
                associated
                    .reload(&XPathArtifactLoadContext {
                        invocation_host,
                        ..load_context()
                    })
                    .is_err(),
                "{kind:?} must not imply {invocation_host:?} ownership"
            );
        }
    }
}

fn parse(source: &str) -> cem_ml::validation::xpath::XPathExpressionAst {
    use cem_ml::validation::xpath::{xpath_expression_ast_from_source_bytes, XPathSourceRequest};
    xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: source.as_bytes(),
            source_uri: "memory:expression.xpath",
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 19 },
    )
}

#[test]
fn all_typed_syntax_families_roundtrip_without_lossless_source_tokens() {
    // Encoding a typed form does not make an unimplemented evaluator feature
    // executable; this test asserts representation fidelity, not conformance.
    for source in [
        "()",
        "1, 2.30, 4e2, '🍒'",
        "/a/@b",
        "//xml:*",
        "ancestor-or-self::node()[2]",
        "processing-instruction('keep')",
        "Q{urn:fruit}*",
        "*:item",
        "-1 + +2 * 3 div 4 idiv 5 mod 6",
        "1 to 3",
        "'a' || 'b'",
        "1 eq 2 or 3 ne 4 and 5 lt 6",
        "1 le 2, 3 gt 4, 5 ge 6",
        "1 = 2, 1 != 2, 1 < 2, 1 <= 2, 1 > 2, 1 >= 2",
        "/a is /b, /a << /b, /a >> /b",
        "/a union /b intersect /c except /d",
        "(1, 2) ! (. + 1) ! string(.)",
        "'42' cast as xs:integer?",
        "'42' castable as xs:integer",
        "() treat as empty-sequence()",
        "1 instance of item()*",
        "/a instance of element()+",
        "1 instance of (xs:integer)",
        "for $v in (1,2) return $v + 1",
        "let $v := 1 return $v",
        "if (true()) then 1 else 0",
        "some $v in (1,2) satisfies $v = 2",
        "every $v in (1,2) satisfies $v > 0",
        "map {'fruit': '🍒'}?fruit",
        "[(), (1,2)]",
        "array {1,2}",
        "array {}",
        "$f(1)",
        "(1,2)[. > 1]",
    ] {
        let original = parse(source);
        let hash = ContentHash::from_blake3(source.as_bytes());
        let artifact = XPathCompiledArtifact::compile(&original, hash.clone())
            .unwrap_or_else(|error| panic!("{source}: {error}"));
        let reloaded = artifact
            .reload(&XPathArtifactLoadContext {
                expected_source_hash: hash,
                invocation_host: XPathInvocationHost::StandaloneTransform,
            })
            .unwrap();
        assert_eq!(original.syntax_ast, reloaded.syntax_ast, "{source}");
        assert_eq!(original.attachment, reloaded.attachment, "{source}");
        assert_eq!(original.source, reloaded.source, "{source}");
        assert!(reloaded.source_text.is_none());
    }
}

#[test]
fn compiler_and_decoder_bound_nested_programs_and_reject_bad_versions() {
    use cem_ml::validation::xpath::{XPathExpression, XPathExpressionNode, XPathUnaryOperator};
    let mut original = parse("1");
    let root = original.syntax_ast.as_mut().unwrap();
    for _ in 0..150 {
        let operand = root.root.expressions.remove(0);
        root.root.expressions.push(XPathExpressionNode {
            source_range: operand.source_range,
            expression: XPathExpression::Unary {
                operator: XPathUnaryOperator::Plus,
                operand: Box::new(operand),
            },
        });
    }
    assert_eq!(
        XPathCompiledArtifact::compile(&original, ContentHash::from_blake3(b"1"))
            .unwrap_err()
            .code,
        "cem.xpath.artifact_limit"
    );
    assert!(
        XPathCompiledArtifact::compile(&parse("1 +"), ContentHash::from_blake3(b"1 +")).is_err()
    );
    let artifact = compile();
    let mut wrong = artifact.bytes().to_vec();
    let position = wrong
        .windows(b"cem-xpath-program-v2".len())
        .position(|part| part == b"cem-xpath-program-v2")
        .unwrap();
    wrong[position + b"cem-xpath-program-v2".len() - 1] = b'9';
    assert_eq!(
        XPathCompiledArtifact::from_bytes(wrong.clone(), &ContentHash::from_blake3(&wrong))
            .unwrap_err()
            .code,
        "cem.xpath.artifact_identity_mismatch"
    );
    let oversized = vec![0; cem_ml::validation::xpath::artifact::XPATH_ARTIFACT_MAX_BYTES + 1];
    assert_eq!(
        XPathCompiledArtifact::from_bytes(oversized.clone(), &ContentHash::from_blake3(&oversized))
            .unwrap_err()
            .code,
        "cem.xpath.artifact_limit"
    );
}

#[test]
fn malformed_binary_values_fail_without_panics_even_with_matching_hashes() {
    let artifact = compile();
    for index in 0..artifact.bytes().len() {
        let mut bytes = artifact.bytes().to_vec();
        bytes[index] ^= 0xff;
        // Some mutations encode different, well-formed programs. Others must
        // fail safely; a matching hash alone does not validate binary structure.
        let _ = XPathCompiledArtifact::from_bytes(bytes.clone(), &ContentHash::from_blake3(&bytes));
    }
}
