//! XPATH-DEMO-COMPANION / XPATH-DEMO-COMPANION-WASM.
use cem_ml::{
    content_cache::ContentHash,
    resolver::{ResolverPolicy, ResolverRegistry},
};
use cem_ql::{
    eval::{AtomValue, Item, ItemStream},
    native::NativeFunctionRegistry,
    render::{
        compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
        TemplateData,
    },
    xpath::functions::{
        companion::{CemtXPathCompanions, COMPANION_CONTENT_TYPE, MAX_COMPANION_BYTES},
        CemtXPathFunctions,
    },
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
        }}
    }
    {function @name=demo.accept @visibility=public @returns=boolean |
        {param @name=quantity @type=integer @required=true}
        {body | {xpath @sequence-type="xs:boolean" |
            {variable @binding=quantity @local-name=quantity}
            {expression | $quantity >= 2}
        }}
    }
}"#;
const URI: &str = "memory:companion.cemt";

fn source_hash() -> ContentHash {
    ContentHash::from_blake3(SOURCE.as_bytes())
}
fn compiled() -> CemtXPathFunctions {
    CemtXPathFunctions::compile(SOURCE, URI).unwrap()
}
fn reload(bytes: &[u8]) -> CemtXPathFunctions {
    CemtXPathFunctions::from_companion_bytes(
        bytes,
        &ContentHash::from_blake3(bytes),
        &source_hash(),
    )
    .unwrap()
}

#[test]
fn deterministic_companion_reloads_without_source_and_rebinds_only_explicitly() {
    let original = compiled();
    let bytes = original.to_companion_bytes().unwrap();
    let loaded = reload(&bytes);
    assert_eq!(bytes, loaded.to_companion_bytes().unwrap());
    assert_eq!(bytes, compiled().to_companion_bytes().unwrap());
    assert_eq!(loaded.source_hash(), &source_hash());
    assert_eq!(loaded.source_uri(), URI);
    for (before, after) in original.functions().iter().zip(loaded.functions()) {
        assert_eq!(before.artifact().bytes(), after.artifact().bytes());
        assert_eq!(before.expression(), after.expression());
        assert!(after.expression().source_text.is_none());
        assert!(after.expression().tokens.is_empty());
    }
    let template = compile_template(
        r#"{module |
        {template @mode=fruit @match='native:call("demo.accept", node)' |
            {body | {b | {$native:call("demo.label", text)}}}}
        {template @mode=fruit @match=true @priority=-10 | {body | {i | 🍋}}}
        {body | {apply-templates @mode=fruit @select=quantity}}
    }"#,
        &CompileTemplateOptions {
            host_bindings: vec!["text".into(), "quantity".into()],
            ..Default::default()
        },
    );
    assert!(
        template.diagnostics.is_empty(),
        "{:?}",
        template.diagnostics
    );
    let mut functions = NativeFunctionRegistry::default();
    loaded
        .install(
            &mut functions,
            Arc::new(ResolverRegistry::new()),
            Arc::new(ResolverPolicy::new()),
        )
        .unwrap();
    for (quantity, text, expected) in [
        (1, "one", "<i>🍋</i>"),
        (2, "two", "<b>two 🍒</b>"),
        (3, "changed", "<b>changed 🍒</b>"),
    ] {
        let data = TemplateData {
            bindings: BTreeMap::from([
                (
                    "text".into(),
                    ItemStream::once(Item::Atomic(AtomValue::String(text.into()))),
                ),
                (
                    "quantity".into(),
                    ItemStream::once(Item::Atomic(AtomValue::Integer(quantity))),
                ),
            ]),
            native_functions: functions.clone(),
            ..Default::default()
        };
        let result = render_compiled_template(&template, &data);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(render_plan_to_html(&result), expected);
        let missing = render_compiled_template(
            &template,
            &TemplateData {
                native_functions: Default::default(),
                ..data
            },
        );
        assert!(missing
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.ql.native_function_unavailable"));
    }
    // Optional, explicit test transport for the native/WASM parity script.
    if let Ok(directory) = std::env::var("CEM_XPATH_COMPANION_FIXTURE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::write(directory.join("companion.bin"), &bytes).unwrap();
        std::fs::write(
            directory.join("fixture.json"),
            serde_json::to_vec(&serde_json::json!({
                "source": SOURCE, "sourceUri": URI, "contentType": COMPANION_CONTENT_TYPE,
                "contentHash": ContentHash::from_blake3(&bytes).header_value(),
                "sourceHash": source_hash().header_value(),
            }))
            .unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn wrong_hashes_truncation_trailing_bytes_and_limits_fail_closed() {
    let bytes = compiled().to_companion_bytes().unwrap();
    let hash = ContentHash::from_blake3(&bytes);
    let wrong = ContentHash::from_blake3(b"wrong");
    for (content, source) in [(&wrong, &source_hash()), (&hash, &wrong)] {
        assert!(CemtXPathFunctions::from_companion_bytes(&bytes, content, source).is_err());
    }
    for end in [0, 1, 10, bytes.len() / 2, bytes.len() - 1] {
        let truncated = &bytes[..end];
        assert!(CemtXPathFunctions::from_companion_bytes(
            truncated,
            &ContentHash::from_blake3(truncated),
            &source_hash()
        )
        .is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(CemtXPathFunctions::from_companion_bytes(
        &extra,
        &ContentHash::from_blake3(&extra),
        &source_hash()
    )
    .is_err());
    let oversized = vec![0; MAX_COMPANION_BYTES + 1];
    assert_eq!(
        CemtXPathFunctions::from_companion_bytes(&oversized, &hash, &source_hash())
            .unwrap_err()
            .code,
        "cem.ql.xpath_companion_limit"
    );
}

#[test]
fn invalid_metadata_cannot_relabel_programs_or_invent_bindings() {
    let bytes = compiled().to_companion_bytes().unwrap();
    // Framing: magic (9 bytes), metadata byte length (u32 LE), explicit JSON
    // control manifest, then length-delimited opaque XPath programs.
    let length = u32::from_le_bytes(bytes[9..13].try_into().unwrap()) as usize;
    let manifest: serde_json::Value = serde_json::from_slice(&bytes[13..13 + length]).unwrap();
    let changes: &[fn(&mut serde_json::Value)] = &[
        |m| m["contentType"] = "application/vnd.cem.xpath-artifact+cem-bin".into(),
        |m| m["version"] = "future".into(),
        |m| m["sourceUri"] = "memory:other.cemt".into(),
        |m| m["functions"][0]["name"] = "demo.renamed".into(),
        |m| m["functions"][0]["parameters"][0]["kind"] = "object".into(),
        |m| m["functions"][0]["context"] = "missing".into(),
        |m| m["functions"][0]["variables"][0]["binding"] = "missing".into(),
        |m| m["functions"][0]["variables"][0]["localName"] = "missing".into(),
        |m| m["functions"][0]["sourceMap"]["frames"] = serde_json::json!([]),
        |m| m["functions"][0]["programHash"] = "blake3-256:bad".into(),
    ];
    for change in changes {
        let mut modified = manifest.clone();
        change(&mut modified);
        let metadata = serde_json::to_vec(&modified).unwrap();
        let mut corrupt = bytes[..9].to_vec();
        corrupt.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
        corrupt.extend_from_slice(&metadata);
        corrupt.extend_from_slice(&bytes[13 + length..]);
        assert!(
            CemtXPathFunctions::from_companion_bytes(
                &corrupt,
                &ContentHash::from_blake3(&corrupt),
                &source_hash()
            )
            .is_err(),
            "{modified}"
        );
    }
}

#[test]
fn companion_handles_are_bounded_isolated_and_never_reused() {
    let bytes = compiled().to_companion_bytes().unwrap();
    let hash = ContentHash::from_blake3(&bytes);
    let mut host = CemtXPathCompanions::default();
    assert!(host.functions(0).is_none());
    assert!(host
        .import(&bytes, &ContentHash::from_blake3(b"wrong"), &source_hash())
        .is_err());
    let first = host.import(&bytes, &hash, &source_hash()).unwrap();
    assert_eq!(first, 1, "failed imports must not allocate handles");
    assert!(host.functions(first).is_some());
    assert!(CemtXPathCompanions::default().functions(first).is_none());
    assert!(host.dispose(first));
    assert!(!host.dispose(first));
    assert!(host.functions(first).is_none());
    let ids: Vec<_> = (0..64)
        .map(|_| host.import(&bytes, &hash, &source_hash()).unwrap())
        .collect();
    assert_eq!(
        host.import(&bytes, &hash, &source_hash()).unwrap_err().code,
        "cem.ql.xpath_companion_limit"
    );
    for id in &ids {
        assert!(host.dispose(*id));
    }
    assert!(host.import(&bytes, &hash, &source_hash()).unwrap() > *ids.last().unwrap());
}

#[test]
fn companion_keeps_namespace_bindings_and_original_xml_owners() {
    use cem_ml::{
        lifecycle::LoadedInputAstStream,
        validation::{
            xml::{xml_document_ast_from_source_bytes, XmlSourceValidationRequest},
            xpath::XPathNativeNode,
        },
    };
    use cem_ql::{
        api::{compile, evaluate, CompileContext, EvaluationContext},
        xpath::functions::XPathQueryItem,
    };
    let source = SOURCE
        .replace("@default t", "@ns v = \"urn:variables\"\n@default t")
        .replace("@returns=string", "@returns=any")
        .replace("@type=string", "@type=any")
        .replace("@sequence-type=\"xs:string\"", "@sequence-type=\"node()*\"")
        .replace(
            "@binding=text @local-name=text",
            "@binding=text @local-name=text @namespace-uri=urn:variables",
        )
        .replace("$text || ' 🍒'", "$v:text/r/item");
    let library = CemtXPathFunctions::compile(&source, URI).unwrap();
    let bytes = library.to_companion_bytes().unwrap();
    let reloaded = CemtXPathFunctions::from_companion_bytes(
        &bytes,
        &ContentHash::from_blake3(&bytes),
        library.source_hash(),
    )
    .unwrap();
    let mut registry = NativeFunctionRegistry::default();
    reloaded
        .install(
            &mut registry,
            Arc::new(ResolverRegistry::new()),
            Arc::new(ResolverPolicy::new()),
        )
        .unwrap();
    let query = compile(
        r#"native:call("demo.label", input)"#,
        &CompileContext {
            policy_bindings: BTreeMap::from([("input".into(), ItemStream::default())]),
            ..Default::default()
        },
    )
    .unwrap();
    for source in ["<r><item>🍒</item></r>", "<r><item>🍇</item></r>"] {
        let (document, diagnostics) =
            xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "memory:fruit.xml",
                content_type: Some("application/xml"),
            });
        assert!(diagnostics.is_empty());
        let owner = Arc::new(LoadedInputAstStream::XmlDocument(document.unwrap()));
        let input =
            XPathQueryItem::from_node(XPathNativeNode::xml_document(owner.clone()).unwrap());
        let result = evaluate(
            &query,
            &EvaluationContext {
                native_functions: registry.clone(),
                policy_bindings: BTreeMap::from([("input".into(), ItemStream::once(input))]),
                ..Default::default()
            },
        );
        assert!(result.error.is_none(), "{result:?}");
        assert_eq!(result.items.len(), 1);
        let retained = result.items[0]
            .view()
            .unwrap()
            .downcast_ref::<XPathQueryItem>()
            .unwrap();
        assert!(Arc::ptr_eq(
            retained
                .xpath_item()
                .native_node()
                .unwrap()
                .source_owner()
                .as_ref()
                .unwrap(),
            &owner
        ));
        assert!(!result.items[0].source_map().unwrap().frames.is_empty());
    }
}

// XPATH-DEMO-LIBRARY-BROWSER: one external library is independent of its consumers.
#[test]
fn separate_library_keeps_its_identity_across_consumers_and_replacement() {
    let mut host = CemtXPathCompanions::default();
    let bytes = compiled().to_companion_bytes().unwrap();
    let id = host
        .import(&bytes, &ContentHash::from_blake3(&bytes), &source_hash())
        .unwrap();
    let replacement_source = SOURCE.replace("' 🍒'", "' 🍇'");
    let replacement = CemtXPathFunctions::compile(&replacement_source, URI).unwrap();
    let replacement_bytes = replacement.to_companion_bytes().unwrap();
    let replacement_id = host
        .import(
            &replacement_bytes,
            &ContentHash::from_blake3(&replacement_bytes),
            replacement.source_hash(),
        )
        .unwrap();
    assert_ne!(id, replacement_id);
    for tag in ["p", "output"] {
        let template = compile_template(
            &format!("{{{tag} | {{$native:call(\"demo.label\", text)}}}}"),
            &CompileTemplateOptions {
                host_bindings: vec!["text".into()],
                ..Default::default()
            },
        );
        assert!(
            template.diagnostics.is_empty(),
            "{:?}",
            template.diagnostics
        );
        for (handle, suffix) in [(id, "🍒"), (replacement_id, "🍇")] {
            for text in ["First", "Changed"] {
                let plan = render_compiled_template(
                    &template,
                    &TemplateData {
                        bindings: BTreeMap::from([(
                            "text".into(),
                            ItemStream::once(Item::Atomic(AtomValue::String(text.into()))),
                        )]),
                        native_functions: host.functions(handle).unwrap(),
                        ..Default::default()
                    },
                );
                assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
                assert_eq!(
                    render_plan_to_html(&plan),
                    format!("<{tag}>{text} {suffix}</{tag}>")
                );
            }
        }
    }
    assert!(host.dispose(id));
    assert!(host.functions(id).is_none());
    assert!(host.functions(replacement_id).is_some());
    assert!(host.dispose(replacement_id));
}
