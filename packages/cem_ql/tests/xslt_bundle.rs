//! XSLT-BUNDLE-NATIVE / XSLT-BUNDLE-WASM: deployment, not stylesheet lowering.
use cem_ml::{
    content_cache::ContentHash,
    import::import_data,
    transform_template::TransformTemplateModuleParamType as ParamType,
    validation::{
        xpath::{artifact::XPathCompiledArtifact, XPathAttachment, XPathExpandedName},
        xslt::{xslt_stylesheet_ast_from_source_bytes, XsltSourceValidationRequest},
    },
};
use cem_ql::{
    eval::{imported_cem_tree, AtomValue, Item, ItemStream},
    render::{render_compiled_template, render_plan_to_html, CompileTemplateOptions, TemplateData},
    template_artifact::{compile_template_artifact, TemplateArtifactSourceMapMode},
    xslt::{
        BundleFocus, BundleProgram, BundleSource, BundleVariable, DependencyKind,
        StylesheetDependency, StylesheetSource, XsltBundle, XsltBundleHost, MAX_BUNDLE_BYTES,
    },
};

const ROOT: &str = "<xsl:stylesheet xmlns:xsl=\"http://www.w3.org/1999/XSL/Transform\" version=\"3.0\"><xsl:import href=\"library.xslt\"/><xsl:template match=\"/\"/></xsl:stylesheet>";
const GENERATED: &str =
    r#"{p | {$native:call("xslt.program.0", document, position, size, prefix)}}"#;
const SELECT: &str =
    "string-join(($prefix, string(/*/*[1]), ':', string(position()), '/', string(last())), '')";

fn fixture(select: &str) -> Vec<u8> {
    fixture_with(
        select,
        BundleFocus::Sequence,
        false,
        TemplateArtifactSourceMapMode::Dev,
    )
}

#[test]
fn optional_focus_accepts_only_an_empty_triple_or_a_valid_sequence_focus() {
    let empty = || {
        TemplateData::default()
            .with_binding("document", ItemStream::empty())
            .with_binding("position", ItemStream::empty())
            .with_binding("size", ItemStream::empty())
            .with_binding(
                "prefix",
                ItemStream::once(Item::Atomic(AtomValue::String("ok".into()))),
            )
    };
    for focus in [
        BundleFocus::Singleton,
        BundleFocus::Sequence,
        BundleFocus::OptionalSequence,
    ] {
        let bundle = load(&fixture_with(
            "$prefix",
            focus,
            false,
            TemplateArtifactSourceMapMode::Dev,
        ));
        let plan = bundle.render(&empty());
        if focus == BundleFocus::OptionalSequence {
            assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
            assert_eq!(render_plan_to_html(&plan), "<p>ok</p>");
            for control in ["document", "position", "size"] {
                let plan = bundle.render(&empty().with_binding(
                    control,
                    ItemStream::once(Item::Atomic(AtomValue::Integer(1))),
                ));
                assert!(plan
                    .diagnostics
                    .iter()
                    .any(|d| d.code == "cem.xslt.bundle_argument"));
            }
        } else {
            assert!(plan
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.xslt.bundle_argument"));
        }
        let plan = bundle.render(&data("<input/>", "xml"));
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    }
}
fn fixture_with(
    select: &str,
    focus: BundleFocus,
    namespaced: bool,
    mode: TemplateArtifactSourceMapMode,
) -> Vec<u8> {
    let source = format!(
        r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:v="urn:bundle:variables" version="3.0"><xsl:template match="/"><xsl:value-of select="{select}"/></xsl:template></xsl:stylesheet>"#
    );
    let (parsed, diagnostics) =
        xslt_stylesheet_ast_from_source_bytes(XsltSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "memory:library.xslt",
            content_type: Some("application/xslt+xml"),
        });
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let mut expression = parsed.unwrap().xpath_expressions.pop().unwrap().expression;
    // Compiler-owned binding declarations are part of the typed static context.
    let XPathAttachment::Host(host) = &mut expression.attachment else {
        panic!()
    };
    host.static_context.variable_bindings.insert(
        if namespaced {
            "Q{urn:bundle:variables}prefix"
        } else {
            "prefix"
        }
        .into(),
        "xs:string".into(),
    );
    let generated = match focus {
        BundleFocus::Sequence | BundleFocus::OptionalSequence => GENERATED.to_owned(),
        BundleFocus::Singleton => {
            GENERATED.replace("document, position, size, prefix", "document, prefix")
        }
        BundleFocus::Absent => GENERATED.replace("document, position, size, prefix", "prefix"),
    };
    let artifact =
        XPathCompiledArtifact::compile(&expression, ContentHash::from_blake3(source.as_bytes()))
            .unwrap();
    let template = compile_template_artifact(
        &generated,
        &CompileTemplateOptions {
            host_bindings: vec![
                "document".into(),
                "position".into(),
                "size".into(),
                "prefix".into(),
            ],
            ..Default::default()
        },
        mode,
    );
    XsltBundle::compose(
        &template,
        BundleSource::new("memory:generated.cemt", generated.as_bytes()),
        &[
            StylesheetSource {
                source: BundleSource::new("memory:root.xslt", ROOT.as_bytes()),
                dependencies: vec![StylesheetDependency {
                    kind: DependencyKind::Import,
                    target: 1,
                }],
            },
            StylesheetSource {
                source: BundleSource::new("memory:library.xslt", source.as_bytes()),
                dependencies: vec![],
            },
        ],
        &[BundleProgram {
            group_context: false,
            stylesheet: 1,
            focus,
            variables: vec![BundleVariable {
                name: if namespaced {
                    XPathExpandedName::new(Some("urn:bundle:variables"), "prefix")
                } else {
                    XPathExpandedName::unqualified("prefix")
                },
                kind: ParamType::String,
                nullable: false,
            }],
            artifact,
        }],
    )
    .unwrap()
}
fn root_hash() -> ContentHash {
    ContentHash::from_blake3(ROOT.as_bytes())
}
fn load(bytes: &[u8]) -> XsltBundle {
    XsltBundle::from_bytes(bytes, &ContentHash::from_blake3(bytes), &root_hash()).unwrap()
}
fn data(source: &str, format: &str) -> TemplateData {
    TemplateData::default()
        .with_binding(
            "document",
            ItemStream::once(imported_cem_tree(
                import_data(source, format, "cem", "memory:input").unwrap(),
            )),
        )
        .with_binding(
            "position",
            ItemStream::once(Item::Atomic(AtomValue::Integer(2))),
        )
        .with_binding(
            "size",
            ItemStream::once(Item::Atomic(AtomValue::Integer(3))),
        )
        .with_binding(
            "prefix",
            ItemStream::once(Item::Atomic(AtomValue::String("hello ".into()))),
        )
}

#[test]
fn deterministic_bundle_reloads_and_renders_changed_native_documents() {
    let bytes = fixture(SELECT);
    assert_eq!(bytes, fixture(SELECT));
    let bundle = load(&bytes);
    assert!(bundle.expressions()[0].source_text.is_none());
    assert!(bundle.expressions()[0].tokens.is_empty());
    for value in ["A", "B"] {
        let plan = bundle.render(&data(&format!("<r><item>{value}</item></r>"), "xml"));
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        assert_eq!(
            render_plan_to_html(&plan),
            format!("<p>hello {value}:2/3</p>")
        );
    }
    let isolated = render_compiled_template(bundle.template(), &data("<r/>", "xml"));
    assert!(isolated
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.native_function_unavailable"));
    for (source, format) in [
        (r#"{"label":"from data"}"#, "json"),
        ("label: from data\n", "yaml"),
        ("label\nfrom data\n", "csv"),
    ] {
        let rendered = bundle.render(&data(source, format));
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
        assert_eq!(render_plan_to_html(&rendered), "<p>hello from data:2/3</p>");
    }
    if let Ok(directory) = std::env::var("CEM_XSLT_BUNDLE_FIXTURE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::write(directory.join("bundle.bin"), &bytes).unwrap();
        // Explicit test control manifest, never a runtime document projection.
        std::fs::write(
            directory.join("fixture.json"),
            serde_json::to_vec(&serde_json::json!({
                "contentHash": ContentHash::from_blake3(&bytes).header_value(),
                "rootSourceHash": root_hash().header_value(), "generated": GENERATED,
            }))
            .unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn invalid_focus_diagnostics_keep_imported_stylesheet_coordinates() {
    let bundle = load(&fixture(SELECT));
    let mut input = data("<r/>", "xml");
    input.bindings.insert(
        "position".into(),
        ItemStream::once(Item::Atomic(AtomValue::Integer(4))),
    );
    let plan = bundle.render(&input);
    let diagnostic = plan
        .diagnostics
        .iter()
        .find(|d| d.code == "cem.xpath.focus_invalid")
        .unwrap();
    assert_eq!(diagnostic.uri.as_deref(), Some("memory:library.xslt"));
    assert!(diagnostic.byte_offset.unwrap() > 0);
    assert!(!diagnostic.source_map.as_ref().unwrap().frames.is_empty());
}

fn edit_manifest(bytes: &[u8], change: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let length = u32::from_le_bytes(bytes[9..13].try_into().unwrap()) as usize;
    let mut manifest: serde_json::Value = serde_json::from_slice(&bytes[13..13 + length]).unwrap();
    change(&mut manifest);
    let metadata = serde_json::to_vec(&manifest).unwrap();
    let mut result = bytes[..9].to_vec();
    result.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
    result.extend_from_slice(&metadata);
    result.extend_from_slice(&bytes[13 + length..]);
    result
}

#[test]
fn invalid_identity_members_closures_and_bindings_fail_before_installation() {
    let bytes = fixture(SELECT);
    let changes: &[fn(&mut serde_json::Value)] = &[
        |m| m["version"] = "future".into(),
        |m| m["contentType"] = "application/vnd.cem.xpath-artifact+cem-bin".into(),
        |m| m["cemMlVersion"] = "future".into(),
        |m| m["cemQlVersion"] = "future".into(),
        |m| m["stylesheets"][1]["source"]["uri"] = "memory:wrong".into(),
        |m| m["stylesheets"][1]["source"]["byteLength"] = 1.into(),
        |m| m["stylesheets"][1]["source"]["hash"] = root_hash().header_value().into(),
        |m| m["stylesheets"][0]["dependencies"][0]["target"] = 9.into(),
        |m| m["stylesheets"][0]["dependencies"][0]["target"] = 0.into(),
        |m| m["stylesheets"][0]["dependencies"] = serde_json::json!([]),
        |m| m["programs"][0]["stylesheet"] = 0.into(),
        |m| m["programs"][0]["hash"] = root_hash().header_value().into(),
        |m| m["programs"][0]["variables"][0]["name"]["localName"] = "missing".into(),
        |m| m["programs"][0]["variables"][0]["kind"] = "object".into(),
        |m| m["programs"][0]["group_context"] = "invalid".into(),
        |m| m["templateHash"] = root_hash().header_value().into(),
        |m| m["generatedSource"]["byteLength"] = 1.into(),
        |m| m["hostBindings"] = serde_json::json!([]),
    ];
    let mut rejections = Vec::new();
    for (index, change) in changes.iter().enumerate() {
        let corrupt = edit_manifest(&bytes, change);
        assert!(XsltBundle::from_bytes(
            &corrupt,
            &ContentHash::from_blake3(&corrupt),
            &root_hash()
        )
        .is_err());
        if let Ok(directory) = std::env::var("CEM_XSLT_BUNDLE_FIXTURE_DIR") {
            let name = format!("rejected-{index}.bin");
            std::fs::write(std::path::Path::new(&directory).join(&name), &corrupt).unwrap();
            rejections.push(serde_json::json!({"name": name, "hash": ContentHash::from_blake3(&corrupt).header_value()}));
        }
    }
    if let Ok(directory) = std::env::var("CEM_XSLT_BUNDLE_FIXTURE_DIR") {
        std::fs::write(
            std::path::Path::new(&directory).join("rejections.json"),
            serde_json::to_vec(&rejections).unwrap(),
        )
        .unwrap();
    }
    let changed = fixture("$prefix || 'changed import'");
    assert_ne!(bytes, changed);
    assert!(
        XsltBundle::from_bytes(&changed, &ContentHash::from_blake3(&bytes), &root_hash()).is_err()
    );
    assert!(XsltBundle::from_bytes(
        &bytes,
        &ContentHash::from_blake3(&bytes),
        &ContentHash::from_blake3(b"wrong")
    )
    .is_err());
    for length in [0, 1, 12, bytes.len() / 2, bytes.len() - 1] {
        let truncated = &bytes[..length];
        assert!(XsltBundle::from_bytes(
            truncated,
            &ContentHash::from_blake3(truncated),
            &root_hash()
        )
        .is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(XsltBundle::from_bytes(
        &trailing,
        &ContentHash::from_blake3(&trailing),
        &root_hash()
    )
    .is_err());
    assert_eq!(
        XsltBundle::from_bytes(&vec![0; MAX_BUNDLE_BYTES + 1], &root_hash(), &root_hash())
            .unwrap_err()
            .code,
        "cem.xslt.bundle_limit"
    );
}

#[test]
fn retained_bundles_are_isolated_bounded_and_never_reuse_handles() {
    let bytes = fixture(SELECT);
    let hash = ContentHash::from_blake3(&bytes);
    let mut host = XsltBundleHost::default();
    assert!(host.import(&bytes, &root_hash(), &root_hash()).is_err());
    let first = host.import(&bytes, &hash, &root_hash()).unwrap();
    assert_eq!(first, 1);
    let borrowed = host.get(first).unwrap();
    assert!(host.dispose(first));
    assert!(!host.dispose(first));
    assert!(host.get(first).is_none());
    assert_eq!(
        render_plan_to_html(&borrowed.render(&data("<r><item>A</item></r>", "xml"))),
        "<p>hello A:2/3</p>"
    );
    let ids: Vec<_> = (0..16)
        .map(|_| host.import(&bytes, &hash, &root_hash()).unwrap())
        .collect();
    assert!(ids[0] > first);
    assert_eq!(
        host.import(&bytes, &hash, &root_hash()).unwrap_err().code,
        "cem.xslt.bundle_limit"
    );
    for id in ids {
        assert!(host.dispose(id));
    }
    assert!(host.import(&bytes, &hash, &root_hash()).is_ok());
}

#[test]
fn absent_singleton_namespaced_variables_and_production_maps_reload() {
    for (focus, select, expected) in [
        (
            BundleFocus::Absent,
            "$v:prefix || 'absent'",
            "<p>hello absent</p>",
        ),
        (
            BundleFocus::Singleton,
            "string-join(($v:prefix, string(position()), '/', string(last())), '')",
            "<p>hello 1/1</p>",
        ),
    ] {
        for mode in [
            TemplateArtifactSourceMapMode::Dev,
            TemplateArtifactSourceMapMode::Prod,
        ] {
            let bundle = load(&fixture_with(select, focus, true, mode));
            let rendered = bundle.render(&data("<r/>", "xml"));
            assert!(
                rendered.diagnostics.is_empty(),
                "{:?}",
                rendered.diagnostics
            );
            assert_eq!(render_plan_to_html(&rendered), expected);
        }
    }
}

#[test]
fn retained_node_results_preserve_owners_after_bundle_and_input_drop() {
    use cem_ql::{
        api::{compile, evaluate, CompileContext, EvaluationContext},
        xpath::functions::XPathQueryItem,
    };
    let bundle = load(&fixture("/*/*"));
    let tree = import_data("<r><item>A</item></r>", "xml", "cem", "memory:owned.xml").unwrap();
    let weak = std::sync::Arc::downgrade(&tree);
    let input = data("<r/>", "xml").with_binding(
        "document",
        ItemStream::once(imported_cem_tree(tree.clone())),
    );
    let query = compile(
        r#"native:call("xslt.program.0", document, position, size, prefix)"#,
        &CompileContext {
            policy_bindings: input.bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let result = evaluate(
        &query,
        &EvaluationContext {
            policy_bindings: input.bindings.clone(),
            native_functions: bundle.native_functions(Default::default()).unwrap(),
            ..Default::default()
        },
    );
    assert!(result.error.is_none(), "{result:?}");
    let item = result.items[0]
        .view()
        .unwrap()
        .downcast_ref::<XPathQueryItem>()
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(
        item.xpath_item().native_node().unwrap().owner(),
        &tree
    ));
    assert!(!result.items[0].source_map().unwrap().frames.is_empty());
    drop(tree);
    drop(input);
    drop(bundle);
    assert!(weak.upgrade().is_some());
    assert_eq!(result.items[0].atom(), Some(AtomValue::String("A".into())));
    drop(result);
    drop(query);
    assert!(weak.upgrade().is_none());
}

#[test]
fn xpath_budgets_and_capability_errors_remain_uncatchable() {
    use cem_ml::validation::xpath::XPathEvaluationLimits;
    use cem_ql::{
        api::{compile, evaluate, CompileContext, EvaluationContext},
        eval::{BudgetAxis, EvalError},
    };
    for (select, limits, expected) in [
        (
            SELECT,
            XPathEvaluationLimits {
                max_work_units: Some(1),
                ..Default::default()
            },
            Some(BudgetAxis::XPathWorkUnits),
        ),
        (
            SELECT,
            XPathEvaluationLimits {
                max_text_bytes: Some(1),
                ..Default::default()
            },
            Some(BudgetAxis::XPathTextBytes),
        ),
        (
            "1 to 20",
            XPathEvaluationLimits {
                max_sequence_items: Some(6),
                ..Default::default()
            },
            Some(BudgetAxis::ItemsPerStage),
        ),
        ("current-dateTime()", XPathEvaluationLimits::default(), None),
    ] {
        let bundle = load(&fixture(select));
        let input = data("<r><item>A</item></r>", "xml");
        let query = compile(r#"try { native:call("xslt.program.0", document, position, size, prefix) } catch (code, message) { "caught" }"#, &CompileContext {
            policy_bindings: input.bindings.clone(), ..Default::default()
        }).unwrap();
        let result = evaluate(
            &query,
            &EvaluationContext {
                policy_bindings: input.bindings,
                native_functions: bundle.native_functions(limits).unwrap(),
                ..Default::default()
            },
        );
        if let Some(axis) = expected {
            assert_eq!(
                result.error,
                Some(EvalError::BudgetExceeded(axis)),
                "{result:?}"
            );
        } else {
            assert!(
                matches!(result.error, Some(EvalError::Unsupported(_))),
                "{result:?}"
            );
        }
        assert!(result.items.is_empty());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.uri.as_deref() == Some("memory:library.xslt") && d.source_map.is_some()));
    }
}
