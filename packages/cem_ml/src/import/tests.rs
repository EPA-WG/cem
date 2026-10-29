use super::*;
use crate::{
    engine::EngineContext,
    schema::{
        document_model::{compile_schema_document_model, compile_schema_document_model_from_retained},
        package_compilation::SchemaPackageSourceRegistry,
        registry::{SchemaPackageOrigin, CSV_SCHEMA_URI},
    },
};

const CSV_SCHEMA: &str = include_str!("../../schema-packages/csv/v1/schema/csv.cem");
const SOURCE_URI: &str = "memory://owned/inventory.csv";

// Retained schema publication is explicit here. Pack authentication and host
// replacement grants have their own asset-activation fixtures.
fn csv_context(schema: &str) -> EngineContext {
    let owner = SchemaPackageSourceRegistry::default()
        .retain("memory://owned/csv-schema.cem", schema)
        .unwrap();
    let model = compile_schema_document_model_from_retained(CSV_SCHEMA_URI, &owner);
    assert!(model.is_ready_for_validation(), "{:?}", model.compile_diagnostics);
    assert!(model.parser_contract.as_ref().unwrap().matches_owner(owner.ast_owner()));
    let mut context = EngineContext::default();
    context.schema_document_models.register(model);
    context.schema_registry.set_package_origin(
        "csv", SchemaPackageOrigin::Manifest("memory://owned/csv-package.cem".into()),
    );
    context
}

fn parse_csv(
    source: &str,
    header: CsvHeader,
    context: &EngineContext,
) -> Result<Arc<RetainedCemTree>, ImportFailure> {
    strings::import_string_with_context(ImportStringRequest {
        source,
        source_uri: SOURCE_URI,
        base_uri: Some("memory://owned/report/"),
        profile: ImportStringProfile::CsvWithOptions(CsvImportOptions { header }),
    }, Some(context))
}

fn text_values(tree: &RetainedCemTree) -> Vec<&str> {
    tree.ast().iter().filter_map(|node| match node {
        CemAstNode::Text { data, .. } => Some(data.as_str()),
        _ => None,
    }).collect()
}

#[cfg(feature = "external-schema-assets")]
#[test]
fn csv_string_imports_without_admission_refuse_missing_sources_without_panicking() {
    let read = std::panic::catch_unwind(|| {
        import_data("sku,quantity\nSKU-A,3\n", "csv", "cem", "memory://source.csv")
    });
    assert!(read.is_ok(), "data:read must refuse missing originals without panicking");
    assert!(read.unwrap().unwrap_err().contains("cem.schema_asset.missing"));
    let parse = std::panic::catch_unwind(|| {
        import_string(ImportStringRequest {
            source: "SKU-A,3\n",
            source_uri: "memory://source.csv",
            base_uri: None,
            profile: ImportStringProfile::Csv,
        })
    });
    assert!(parse.is_ok(), "data:parse must refuse missing originals without panicking");
    let failure = parse.unwrap().unwrap_err();
    assert_eq!(failure.kind, ImportFailureKind::Unsupported);
    assert!(failure.message.contains("cem.schema_asset.missing"));
    assert_eq!(failure.diagnostics[0].code, "cem.schema_asset.missing");
    assert_eq!(failure.diagnostics[0].uri.as_deref(), Some("memory://source.csv"));
}

#[test]
fn csv_string_imports_use_owned_contracts_with_headers_quotes_and_original_positions() {
    let context = csv_context(CSV_SCHEMA);
    let source = "sku,name,quantity\nSKU-A,\"Café, cups\",3\nSKU-B,\"Tea \"\"travel\"\" pot\",8\n";
    let read = import_data_with_context(source, "csv", "cem", SOURCE_URI, Some(&context)).unwrap();
    let parsed = parse_csv(source, CsvHeader::Present, &context).unwrap();
    let bytes = import_data_bytes_with_context(source.as_bytes(), "text/csv;header=present", "cem", SOURCE_URI, Some(&context)).unwrap();
    let expected = ["SKU-A", "Café, cups", "3", "SKU-B", "Tea \"travel\" pot", "8"];
    for tree in [&read, &parsed, &bytes] {
        assert_eq!(text_values(tree), expected);
        assert_eq!(tree.source_uri(), SOURCE_URI);
        assert!(!tree.ast().iter().any(|node| matches!(node,
            CemAstNode::Element { expanded_name, .. } if expanded_name.local_name == "number")));
        let field = tree.ast().iter().find_map(|node| match node {
            CemAstNode::Text { node_id, data, .. } if data == "Café, cups" => Some(*node_id),
            _ => None,
        }).unwrap();
        let range = tree.source_node_range(field).unwrap();
        assert_eq!(range.offset, source.find('"').unwrap() as u64);
        assert_eq!((range.line, range.column), (2, 7));
    }
    assert_eq!(parsed.base_uri(), Some("memory://owned/report/"));
    let absent = parse_csv(source, CsvHeader::Absent, &context).unwrap();
    assert_eq!(&text_values(&absent)[..3], ["sku", "name", "quantity"]);
    assert_eq!(text_values(&absent).len(), 9);
    let read_absent = import_data_with_context(source, "text/csv;header=absent", "cem", SOURCE_URI, Some(&context)).unwrap();
    assert_eq!(text_values(&read_absent), text_values(&absent));
}

#[test]
fn csv_string_imports_retain_owned_quote_diagnostics_and_utf8_rejection() {
    let context = csv_context(CSV_SCHEMA);
    for (source, code, column) in [
        ("a,b\nx,\"unterminated", "cem.csv.unclosed_quote", 3),
        ("a,b\nx,A \"B\n", "cem.csv.invalid_quote_escape", 5),
    ] {
        let failure = parse_csv(source, CsvHeader::Present, &context).unwrap_err();
        assert_eq!(failure.kind, ImportFailureKind::Malformed);
        let diagnostic = failure.diagnostics.iter().find(|d| d.code == code).unwrap();
        assert_eq!(diagnostic.severity, crate::diagnostics::Severity::Error);
        assert_eq!(diagnostic.uri.as_deref(), Some(SOURCE_URI));
        assert_eq!((diagnostic.line, diagnostic.column), (Some(2), Some(column)));
        assert_eq!(diagnostic.byte_offset, Some(source.find('"').unwrap() as u64));
        assert!(import_data_with_context(source, "csv", "cem", SOURCE_URI, Some(&context)).unwrap_err().contains(code));
    }
    assert!(import_data_bytes_with_context(b"a,b\nx,\xff\n", "text/csv;charset=utf-8;header=present", "cem", SOURCE_URI, Some(&context)).is_err());
}

#[test]
fn csv_string_imports_obey_selected_contract_policy_and_never_fall_back_from_pending_models() {
    let customized = CSV_SCHEMA.replace(
        "@code=\"cem.csv.inconsistent_field_count\" @severity=\"warning\"",
        "@code=\"cem.csv.inconsistent_field_count\" @severity=\"error\"",
    );
    assert_ne!(customized, CSV_SCHEMA);
    let normal = csv_context(CSV_SCHEMA);
    let strict = csv_context(&customized);
    let source = "a,b\nx\n";
    assert!(parse_csv(source, CsvHeader::Present, &normal).is_ok());
    assert!(import_data_with_context(source, "csv", "cem", SOURCE_URI, Some(&normal)).is_ok());
    let failure = parse_csv(source, CsvHeader::Present, &strict).unwrap_err();
    assert!(failure.diagnostics.iter().any(|d| d.code == "cem.csv.inconsistent_field_count" && d.severity.is_hard_violation()));
    assert!(import_data_with_context(source, "csv", "cem", SOURCE_URI, Some(&strict)).unwrap_err().contains("cem.csv.inconsistent_field_count"));
    let mut pending = EngineContext::default();
    pending.schema_document_models.inspect_candidate(compile_schema_document_model(CSV_SCHEMA_URI, CSV_SCHEMA));
    assert!(parse_csv(source, CsvHeader::Present, &pending).is_err());
    assert!(import_data_with_context(source, "csv", "cem", SOURCE_URI, Some(&pending)).is_err());
}

#[test]
fn csv_string_imports_keep_string_byte_and_value_limits_with_owned_contracts() {
    let context = csv_context(CSV_SCHEMA);
    let oversized = "x".repeat(MAX_BYTES + 1);
    assert_eq!(parse_csv(&oversized, CsvHeader::Absent, &context).unwrap_err().kind, ImportFailureKind::Limit);
    assert!(import_data_with_context(&oversized, "csv", "cem", SOURCE_URI, Some(&context)).unwrap_err().contains("32 KiB"));
    // The outer sequence and every row/cell consume one projected value.
    let exact = "x\n".repeat((MAX_VALUES - 1) / 2);
    assert!(parse_csv(&exact, CsvHeader::Absent, &context).is_ok());
    assert!(import_data_with_context(&exact, "text/csv;header=absent", "cem", SOURCE_URI, Some(&context)).is_ok());
    let excessive = format!("{exact}x\n");
    assert_eq!(parse_csv(&excessive, CsvHeader::Absent, &context).unwrap_err().kind, ImportFailureKind::Limit);
    assert!(import_data_with_context(&excessive, "text/csv;header=absent", "cem", SOURCE_URI, Some(&context)).unwrap_err().contains("4096-value"));
    assert!(import_data_bytes_with_context(excessive.as_bytes(), "text/csv;header=absent", "cem", SOURCE_URI, Some(&context)).is_ok(),
        "the document boundary keeps its larger independent value budget");
}

#[test]
fn json_only_boundary_matches_general_import_and_retains_original_owner() {
    let uri = "app://cart/response.json";
    for bytes in [br#"{"data":{"name":"Tea","qty":2},"errors":[]}"#.as_slice(), br#"[null,true,"\u0061"]"#] {
        let selected = super::import_json_bytes(bytes, uri).unwrap();
        let ordinary = super::import_data_bytes(bytes, "application/json", "cem", uri).unwrap();
        assert!(matches!(selected.native_owner().unwrap().downcast_ref::<crate::lifecycle::LoadedInputAstStream>(), Some(crate::lifecycle::LoadedInputAstStream::JsonDocument(_))));
        assert_eq!(selected.source_uri(), ordinary.source_uri());
        assert_eq!(format!("{:?}",selected.ast()), format!("{:?}",ordinary.ast()));
        for index in 0..selected.ast().nodes.len() {
            assert_eq!(selected.source_key(index as u32), ordinary.source_key(index as u32));
        }
    }
    for bytes in [b"{".as_slice(), b"[1,]", b"{\"a\":}"] {
        assert_eq!(super::import_json_bytes(bytes, uri).unwrap_err(), super::import_data_bytes(bytes, "application/json", "cem", uri).unwrap_err());
    }
}

#[test]
fn explicit_css_stylesheet_import_preserves_mime_source_and_capsule_profile() {
    let source = ":host { --label: 'café'; color: red; }";
    let uri = "https://app.test/styles/theme.css";
    let mime = "text/css; charset=utf-8; mode=stylesheet";
    let root_name = |tree: &RetainedCemTree| {
        tree.node(tree.node(0).unwrap().children[0])
            .unwrap()
            .name
            .as_ref()
            .unwrap()
            .local_name
            .clone()
    };
    let original = import_data_bytes(source.as_bytes(), mime, "cem", uri).unwrap();
    assert_eq!(root_name(&original), "stylesheet");
    let encoded = crate::compiled_css::encode_response(&original, source, mime).unwrap();
    let pin = crate::content_cache::ContentHash::from_blake3(&encoded).header_value();
    let parsed = crate::validation::css::CSS_SOURCE_PARSE_CALLS.with(|n| n.get());
    let restored = crate::compiled_css::admit(&encoded, &pin).unwrap();
    assert_eq!(
        crate::validation::css::CSS_SOURCE_PARSE_CALLS.with(|n| n.get()),
        parsed
    );
    assert_eq!(restored.content_type, mime);
    assert_eq!(restored.source, source);
    assert_eq!(restored.tree.source_uri(), uri);
    assert_eq!(restored.tree.ast().nodes.len(), original.ast().nodes.len());
    for id in 0..original.ast().nodes.len() {
        // The retained view includes semantic source overlays on the document root.
        assert_eq!(
            format!("{:?}", restored.tree.node(id as u32)),
            format!("{:?}", original.node(id as u32))
        );
        assert_eq!(
            restored.tree.source_key(id as u32),
            original.source_key(id as u32)
        );
        let before = original.source_node_range(id as u32).unwrap();
        let after = restored.tree.source_node_range(id as u32).unwrap();
        assert_eq!((after.offset, after.length), (before.offset, before.length));
        // Byte imports can mark selector line/column unknown (zero). The capsule
        // reconstructs range metadata alongside the verified original source text.
        if before.line != 0 {
            assert_eq!((after.line, after.column), (before.line, before.column));
        }
    }
    assert!(crate::compiled_css::admit_inline(&encoded, &pin).is_err());
    for mime in ["text/css", "text/css; mode=scoped-style-block"] {
        let scoped = import_data_bytes(source.as_bytes(), mime, "cem", uri).unwrap();
        assert_eq!(root_name(&scoped), "style-block");
        assert_ne!(scoped.source_key(0), original.source_key(0));
        let invalid = crate::compiled_css::encode_response(&scoped, source, mime).unwrap();
        let hash = crate::content_cache::ContentHash::from_blake3(&invalid).header_value();
        assert!(crate::compiled_css::admit(&invalid, &hash).is_err());
    }
}
