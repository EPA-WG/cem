//! Native parser ownership is shared by imports, names and bounded compilation.
use cem_ml::{
    content_cache::ContentHash,
    events::cem::CemEventNormalizer,
    parser::{
        builder::CemAstBuilder,
        tree::{CemTreeSemantics, RetainedCemTree},
    },
    schema::{
        document_model::{
            compile_schema_document_model, compile_schema_document_model_from_retained,
        },
        package_sources::builtin_schema_package_source,
        registry::XSLT_SCHEMA_URI,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
    validation::{xpath::XPathExpandedName, xslt::xslt_parser_contract_from_context},
};
use cem_ql::{
    eval::{imported_cem_tree, ItemStream},
    render::{render_plan_to_html, TemplateData},
    xslt::{compiler::*, XsltBundle},
};

const SCHEMA: &str = include_str!("../../cem_ml/schema-packages/xslt/v1/schema/xslt.cem");
const URI: &str = "https://example.test/view.xsl";
const CHILD_URI: &str = "https://example.test/child.xsl";
const ROOT: &str = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:controls" version="3.0"><xsl:import href="./child.xsl"/></xsl:stylesheet>"#;
const CHILD: &str = "<xsl:stylesheet xmlns:xsl=\"http://www.w3.org/1999/XSL/Transform\" version=\"3.0\">\r\n<xsl:template match=\"/\"><p data-literal=\"{\"><xsl:value-of select=\"'Café 🍒'\"/></p></xsl:template></xsl:stylesheet>";

fn owned(schema: &str) -> cem_ml::schema::document_model::SchemaDocumentModel {
    let ast = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), schema.as_bytes().to_vec()),
    )))
    .build();
    let source = RetainedCemTree::new(
        ast,
        "memory:owned-xslt-schema.cem",
        schema,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    let model = compile_schema_document_model_from_retained(XSLT_SCHEMA_URI, &source);
    assert!(
        model.is_ready_for_validation(),
        "{:?}",
        model.compile_diagnostics
    );
    assert!(model
        .parser_contract
        .as_ref()
        .unwrap()
        .matches_owner(source.ast_owner()));
    model
}
fn options(child: &str) -> XsltCompileOptions {
    XsltCompileOptions {
        modules: vec![XsltModuleSource {
            parent_uri: URI.into(),
            href: "./child.xsl".into(),
            uri: CHILD_URI.into(),
            source: child.into(),
            content_hash: ContentHash::from_blake3(child.as_bytes()),
        }],
        ..Default::default()
    }
}

#[test]
fn owned_grammar_drives_imports_names_every_module_and_portable_bundle_provenance() {
    let schema = SCHEMA
        .replace("literal-result@*", "literal-result@title")
        .replace("cem.xslt.external_uri_rejected", "fixture.owned_uri");
    let mut model = owned(&schema);
    let contract = model.parser_contract.as_ref().unwrap().clone();
    // Later inspection data is not the immutable executable grammar snapshot.
    model.constraints.clear();
    assert_eq!(
        stylesheet_imports_with_parser_contract(ROOT, URI, Some(&contract)).unwrap(),
        ["./child.xsl"]
    );
    assert_eq!(
        resolve_xslt_names_with_parser_contract(ROOT, URI, &["p:label"], Some(&contract)).unwrap(),
        [XPathExpandedName {
            namespace_uri: Some("urn:controls".into()),
            local_name: "label".into()
        }]
    );
    let compiled = compile_xslt_bundle_with_options_and_parser_contract(
        ROOT,
        URI,
        &options(CHILD),
        Some(&contract),
    )
    .unwrap();
    let bundle = XsltBundle::from_bytes(
        &compiled.bytes,
        &compiled.content_hash,
        &compiled.source_hash,
    )
    .unwrap();
    let data = TemplateData::default().with_binding(
        "document",
        ItemStream::once(imported_cem_tree(
            cem_ml::import::import_data("<r/>", "xml", "cem", "memory:input.xml").unwrap(),
        )),
    );
    let result = bundle.render(&data);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(
        render_plan_to_html(&result),
        "<p data-literal=\"{\">Café 🍒</p>"
    );
    assert_eq!(bundle.stylesheets().len(), 2);
    assert_eq!(bundle.stylesheets()[1].source.uri, CHILD_URI);
    assert_eq!(
        bundle.stylesheets()[1].source.hash,
        ContentHash::from_blake3(CHILD.as_bytes()).header_value()
    );
    assert!(!bundle.expressions().is_empty());
    assert!(bundle
        .expressions()
        .iter()
        .any(|expression| expression.source.uri == CHILD_URI));
    for expression in bundle.expressions() {
        let original = if expression.source.uri == CHILD_URI {
            CHILD
        } else {
            assert_eq!(expression.source.uri, URI);
            ROOT
        };
        let cem_ml::validation::xpath::XPathAttachment::Host(host) = &expression.attachment else {
            panic!("retained XSLT source owner")
        };
        assert_eq!(host.owner.source_uri, expression.source.uri);
        let range = host.expression_range;
        assert!(original
            .get(
                range.start.byte_offset as usize
                    ..(range.start.byte_offset + range.byte_length) as usize
            )
            .is_some());
    }
}

#[test]
fn all_authoring_entrypoints_keep_owned_diagnostics_and_original_ranges() {
    let model = owned(&SCHEMA.replace("cem.xslt.version_missing", "fixture.owned_version"));
    let contract = model.parser_contract.as_deref();
    let source = CHILD.replace(" version=\"3.0\"", "");
    for errors in [
        stylesheet_imports_with_parser_contract(&source, CHILD_URI, contract).unwrap_err(),
        resolve_xslt_names_with_parser_contract(&source, CHILD_URI, &[], contract).unwrap_err(),
        compile_xslt_bundle_with_options_and_parser_contract(
            &source,
            CHILD_URI,
            &Default::default(),
            contract,
        )
        .unwrap_err(),
        compile_xslt_bundle_with_options_and_parser_contract(
            ROOT,
            URI,
            &options(&source),
            contract,
        )
        .unwrap_err(),
    ] {
        let diagnostic = errors
            .iter()
            .find(|d| d.code == "fixture.owned_version")
            .expect("original-owned diagnostic");
        assert_eq!(diagnostic.uri.as_deref(), Some(CHILD_URI));
        assert!(diagnostic.severity.is_hard_violation());
        assert!(!diagnostic.source_map.as_ref().unwrap().frames.is_empty());
        assert!(diagnostic.byte_offset.unwrap() < source.len() as u64);
    }
}

#[test]
fn explicit_authority_preserves_limits_and_contextless_profile_refusal() {
    let model = owned(SCHEMA);
    let contract = model.parser_contract.as_deref();
    let simple = CHILD.replace(" data-literal=\"{\"", "");
    let legacy = compile_xslt_bundle(&simple, URI);
    if builtin_schema_package_source("xslt").is_none() {
        assert!(legacy
            .unwrap_err()
            .iter()
            .any(|d| d.code == "cem.schema_asset.missing"));
        assert!(stylesheet_imports(ROOT, URI)
            .unwrap_err()
            .iter()
            .any(|d| d.code == "cem.schema_asset.missing"));
        assert!(resolve_xslt_names(ROOT, URI, &[])
            .unwrap_err()
            .iter()
            .any(|d| d.code == "cem.schema_asset.missing"));
    } else {
        legacy.unwrap();
    }
    let mut too_many = options(&simple);
    too_many.modules = vec![too_many.modules[0].clone(); 129];
    assert!(
        compile_xslt_bundle_with_options_and_parser_contract(ROOT, URI, &too_many, contract)
            .is_err()
    );
    let mut corrupt = options(&simple);
    corrupt.modules[0].content_hash = ContentHash::from_blake3(b"changed");
    assert!(
        compile_xslt_bundle_with_options_and_parser_contract(ROOT, URI, &corrupt, contract)
            .is_err()
    );
    assert!(compile_xslt_bundle_with_options_and_parser_contract(
        &" ".repeat(128 * 1024 + 1),
        URI,
        &Default::default(),
        contract
    )
    .is_err());
    let mut context = cem_ml::engine::EngineContext::default();
    context
        .schema_document_models
        .register(compile_schema_document_model(XSLT_SCHEMA_URI, SCHEMA));
    assert!(xslt_parser_contract_from_context(&context, URI).is_err());
}
