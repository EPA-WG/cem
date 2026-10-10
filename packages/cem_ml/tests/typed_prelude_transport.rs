use cem_ml::{
    ast::{
        reload::{ReferenceReloadBundle, ReloadLimits, ReloadSource},
        DebugBinaryDecoder, DebugBinaryEncoder,
    },
    events::cem::CemEventNormalizer,
    schema::{
        machine::{CemSchemaMachine, LexicallyScopedDocument},
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::{CemTokenizer, TypedPreludePreview},
};
use std::sync::Arc;

const TEXT: &str = "@ns public = urn:provider\n@ns ui = {#library}\n@default ui\n@schema select={$ schemaChoice}\n{ui:item}";
fn parse(text: &str) -> LexicallyScopedDocument {
    CemSchemaMachine::new(
        CompiledSchema::cem_core(),
        CemEventNormalizer::new(CemTokenizer::from_source_with_typed_prelude_preview(
            BytesSource::new(SourceId(1), text.as_bytes().to_vec()),
            TypedPreludePreview::default(),
        )),
    )
    .build_with_lexical_scopes()
}
fn bundle(capture: &LexicallyScopedDocument, text: &str) -> ReferenceReloadBundle {
    ReferenceReloadBundle::export(
        capture,
        vec![
            ReloadSource::new(SourceId(0), "typed.cem", text.as_bytes(), true),
            ReloadSource::new(SourceId(1), "typed.cem", text.as_bytes(), true),
        ],
        ReloadLimits::default(),
    )
    .unwrap()
}

#[test]
fn binary_and_executable_reload_keep_typed_slots_and_original_edges() {
    let original = parse(TEXT);
    let wire = DebugBinaryEncoder::new().encode(original.document());
    let native = DebugBinaryDecoder::new().decode(&wire.bytes).unwrap();
    assert_eq!(native.typed_preludes.len(), 2);
    let export = bundle(&original, TEXT);
    let encoded = export.encode(ReloadLimits::default()).unwrap();
    let (_, reloaded) =
        ReferenceReloadBundle::decode_with_document(&encoded, ReloadLimits::default()).unwrap();
    let capture = reloaded.require_lexical().unwrap();
    assert!(!Arc::ptr_eq(capture.document(), original.document()));
    for (&id, syntax) in &original.document().typed_preludes {
        let old = original.typed_prelude(original.document(), id).unwrap();
        let new = capture.typed_prelude(capture.document(), id).unwrap();
        assert_eq!(old.value, new.value);
        assert_eq!(old.syntax, new.syntax);
        assert_eq!(capture.document().typed_preludes[&id], *syntax);
        assert!(original.typed_prelude(capture.document(), id).is_none());
    }
    assert_eq!(reloaded.source_text(SourceId(1)).unwrap(), Some(TEXT));
}

#[test]
fn partial_or_unknown_typed_metadata_cannot_reload_as_literal() {
    let original = parse(TEXT);
    let mut export = bundle(&original, TEXT);
    export.lexical = None;
    assert!(export.reload(ReloadLimits::default()).is_err());
    let mut export = bundle(&original, TEXT);
    export.lexical.as_mut().unwrap().typed_preludes.clear();
    assert!(export.reload(ReloadLimits::default()).is_err());
    let mut export = bundle(&original, TEXT);
    export.lexical.as_mut().unwrap().version = u16::MAX;
    assert!(export.reload(ReloadLimits::default()).is_err());
    let mut export = bundle(&original, TEXT);
    export
        .lexical
        .as_mut()
        .unwrap()
        .typed_preludes
        .values_mut()
        .next()
        .unwrap()
        .value = 0;
    assert!(export.reload(ReloadLimits::default()).is_err());
}

#[test]
fn canonical_output_distinguishes_native_and_quoted_values_and_rejects_downgrade() {
    use cem_ml::{formatter::format_with_profile, schema::ir::SemVer};
    let text = "@ns ui = {#library}\n@default {$ library}\n@schema select=\"{#library}\"\n{host |\n @schema select={$ schemaChoice}\n {item}\n}";
    let capture = parse(text);
    let output = format_with_profile(capture.document(), SemVer::new(1, 1, 0)).unwrap();
    assert!(output.contains("@ns ui = {#library}"), "{output}");
    assert!(output.contains("@default {$ library}"), "{output}");
    assert!(output.contains("@schema select=\"{#library}\""), "{output}");
    let again = parse(&output);
    assert_eq!(again.document().typed_preludes.len(), 3);
    assert_eq!(
        output,
        format_with_profile(again.document(), SemVer::new(1, 1, 0)).unwrap()
    );
    assert!(format_with_profile(capture.document(), SemVer::new(1, 0, 0)).is_err());
}

#[test]
fn public_document_and_explicit_fragment_profiles_admit_1_1_only() {
    use cem_ml::{parser::builder::CemAstBuilder, schema::ir::SemVer};
    for (header, count) in [("1.1", 1), ("1.1.0", 1), ("1", 0), ("1.0", 0), ("1.2", 0)] {
        let input = format!("@doc cem-ml {header}\n@ns ui = {{#library}}");
        let doc = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
            BytesSource::new(SourceId(1), input.into_bytes()),
        )))
        .top_level(true)
        .build();
        assert_eq!(doc.typed_preludes.len(), count, "{header}");
        assert_eq!(doc.format_identity.is_some(), header != "1.2");
    }
    let source = || BytesSource::new(SourceId(1), b"@default {#library}".to_vec());
    let doc =
        CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(source()))).build();
    assert!(doc.typed_preludes.is_empty());
    let tokens =
        CemTokenizer::from_source_with_format_profile(source(), SemVer::new(1, 1, 0)).unwrap();
    let doc = CemAstBuilder::new(CemEventNormalizer::new(tokens)).build();
    assert_eq!(doc.typed_preludes.len(), 1);
    assert!(CemTokenizer::from_source_with_format_profile(source(), SemVer::new(1, 2, 0)).is_err());
    let late = "{item}\n@doc cem-ml 1.1\n@default {#library}";
    let doc = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), late.as_bytes().to_vec()),
    )))
    .build();
    assert!(
        doc.typed_preludes.is_empty(),
        "late header must not change lexical mode"
    );
}

#[test]
fn tabular_native_tree_writer_preserves_nested_typed_directives() {
    use cem_ml::{
        conversion::{
            direct_cem_output_pipeline,
            execute_conversion_output_pipeline_from_cem_tree_with_environment,
            ConversionOutputPipelineEnvironment, ConversionRegistry,
        },
        projection::cem_tree_nodes,
        schema::SchemaRegistry,
    };
    let text = "@doc cem-ml 1.1\n@ns ui = {#library}\n{host |\n @schema select={$ schemaChoice}\n {item}\n}";
    let capture = parse(text);
    let registry = SchemaRegistry::with_builtin_schemas();
    let conversions = ConversionRegistry::with_builtin_converters();
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &registry,
        conversion_registry: &conversions,
        package_artifact_reader: None,
        artifact_cache: None,
    };
    let mut pipeline = direct_cem_output_pipeline();
    pipeline.cemt_options.formatter_profile = Some("tabular".into());
    pipeline.cemt_insertion_context.formatter_profile = Some("tabular".into());
    pipeline.writer_insertion_context.formatter_profile = Some("tabular".into());
    let result = execute_conversion_output_pipeline_from_cem_tree_with_environment(
        &environment,
        &pipeline,
        Arc::new(cem_tree_nodes(capture.document())),
        None,
        vec![],
        "typed-prelude",
        None,
        Some("typed.cem"),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let output = result.output.as_ref().and_then(|v| v.as_str()).unwrap();
    assert!(output.contains("@ns ui = {#library}"), "{output}");
    assert!(
        output.contains("@schema select={$ schemaChoice}"),
        "{output}"
    );
    assert_eq!(parse(output).document().typed_preludes.len(), 2, "{output}");
}

#[test]
fn missing_native_metadata_changed_edges_and_multiline_output_are_rejected() {
    use cem_ml::{formatter::format_with_profile, parser::CemAstNode, schema::ir::SemVer};
    let capture = parse(TEXT);
    let mut stripped = DebugBinaryDecoder::new()
        .decode(&DebugBinaryEncoder::new().encode(capture.document()).bytes)
        .unwrap();
    stripped.typed_preludes.clear();
    assert!(DebugBinaryDecoder::new()
        .decode(&DebugBinaryEncoder::new().encode(&stripped).bytes)
        .is_err());
    assert!(format_with_profile(&stripped, SemVer::new(1, 1, 0)).is_err());
    let mut altered = DebugBinaryDecoder::new()
        .decode(&DebugBinaryEncoder::new().encode(capture.document()).bytes)
        .unwrap();
    let id = *altered.typed_preludes.keys().next().unwrap();
    let child = capture.typed_prelude(capture.document(), id).unwrap().value;
    if let CemAstNode::Reference { expression, .. } = &mut altered.nodes[child as usize] {
        *expression = "#different".into();
    }
    assert!(DebugBinaryDecoder::new()
        .decode(&DebugBinaryEncoder::new().encode(&altered).bytes)
        .is_err());
    let mut multiline = DebugBinaryDecoder::new()
        .decode(&DebugBinaryEncoder::new().encode(capture.document()).bytes)
        .unwrap();
    multiline
        .typed_preludes
        .get_mut(&id)
        .unwrap()
        .expression
        .push('\n');
    assert!(format_with_profile(&multiline, SemVer::new(1, 1, 0)).is_err());
    let mut missing_pending = bundle(&capture, TEXT);
    missing_pending
        .lexical
        .as_mut()
        .unwrap()
        .pending_namespace_declarations
        .remove(&id);
    assert!(missing_pending.reload(ReloadLimits::default()).is_err());
}

#[test]
fn lexical_import_admits_explicit_profiles_and_rejects_unsupported_headers() {
    use cem_ml::{
        import::{import_bytes_with_lexical_scopes, import_bytes_with_lexical_scopes_and_profile},
        schema::ir::SemVer,
    };
    let source = b"@default {#library}";
    let legacy = import_bytes_with_lexical_scopes(
        source,
        "text/cem-ml",
        "typed.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap();
    assert!(!legacy.captured.has_typed_preludes());
    let typed = import_bytes_with_lexical_scopes_and_profile(
        source,
        "text/cem-ml",
        "typed.cem",
        CompiledSchema::cem_core(),
        Some(SemVer::new(1, 1, 0)),
    )
    .unwrap();
    assert!(typed.captured.has_typed_preludes());
    for version in ["1.2", "2", "1.1.0-rc.1"] {
        let source = format!("@doc cem-ml {version}\n@default {{#library}}");
        assert!(import_bytes_with_lexical_scopes(
            source.as_bytes(),
            "text/cem-ml",
            "typed.cem",
            CompiledSchema::cem_core()
        )
        .is_err());
    }
}

#[test]
fn legacy_lexical_metadata_cannot_claim_an_unsupported_required_profile() {
    use cem_ml::{parser::format::DocumentFormatIdentity, schema::ir::SemVer};
    let original = parse("{item}");
    let mut export = bundle(&original, "{item}");
    let lexical = export.lexical.as_mut().unwrap();
    lexical.version = 1;
    lexical.format_identity = Some(DocumentFormatIdentity {
        format_id: "cem-ml".into(),
        content_type: "text/cem-ml".into(),
        format_version: SemVer::new(2, 0, 0),
    });
    assert!(export.reload(ReloadLimits::default()).is_err());
}
