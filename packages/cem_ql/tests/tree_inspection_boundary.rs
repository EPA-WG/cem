//! XML-VIEW-3-BOUNDARY: characterize the missing retained-tree presentation call.
//! The empty query output below is a decision probe, not the acceptance contract
//! for the future viewer. Replace it when the shared query API is approved.
use cem_ml::{
    conversion::{
        direct_cem_output_pipeline,
        execute_conversion_output_pipeline_from_cem_tree_with_environment,
        ConversionOutputPipelineEnvironment, ConversionRegistry,
    },
    import::import_data,
    projection::cem_tree_inspection,
    schema::SchemaRegistry,
};
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    eval::{imported_cem_tree, AtomValue, Item, ItemStream},
    render::{render_template, TemplateData},
};
use std::sync::Arc;

#[test]
fn retained_data_reaches_cemt_but_format_does_not_invoke_typed_inspection() {
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
    for (format, source) in [
        ("xml", "<?xml-stylesheet href='https://invalid.test/inert.xsl'?><r empty=''><![CDATA[<raw>🍒]]><?keep inert?></r>"),
        ("json", r#"{"fruit":"<raw>🍒","empty":""}"#),
        ("yaml", "fruit: '<raw>🍒'\nempty: ''"),
        ("csv", "fruit,empty\n<raw>🍒,\n"),
    ] {
        let uri = format!("memory:tree-inspection.{format}");
        let owner = import_data(source, format, "cem", &uri).unwrap();
        let document = imported_cem_tree(owner.clone());
        let rendered = render_template(
            "{p | {$document.kind}}{pre | {$cemml:format(document)}}",
            &TemplateData::default().with_binding("document", ItemStream::once(document)),
        );
        assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);
        assert_eq!(rendered.rendered, "<p>document</p><pre></pre>", "{format}");

        // The same retained owner already has a working native display path.
        // There is no source reparse or parser-AST traversal at this boundary.
        let stream = Arc::new(cem_tree_inspection(owner.clone()));
        let result = execute_conversion_output_pipeline_from_cem_tree_with_environment(
            &environment,
            &pipeline,
            stream,
            Some(owner.node(0).unwrap().source.clone()),
            vec![],
            "tree-inspection-query-boundary",
            None,
            Some(&uri),
        );
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        // Read the named final text-output envelope, never an AST JSON handoff.
        let output = result.output.as_ref().and_then(|value| value.as_str()).unwrap();
        assert!(output.contains("{ast"), "{format}: {output}");
        assert!(output.contains("<raw>🍒"), "{format}: {output}");
        assert!(!output.contains('\u{1b}'), "browser text has no terminal escapes");
        assert!(Arc::ptr_eq(
            result.raw_cem_tree.as_ref().unwrap().owner().source_owner().unwrap(),
            &owner,
        ));
        assert!(Arc::ptr_eq(
            result.formatted_cemt_tree.as_ref().unwrap().owner().source_owner().unwrap(),
            &owner,
        ));
        assert!(!result.output_spans.is_empty());
        if format == "xml" {
            assert!(output.contains("@kind=cdata"));
            assert!(output.contains("@target=xml-stylesheet"));
            assert!(output.contains("@target=keep"));
            assert!(output.contains("@value=inert"));
        }
    }
}

#[test]
fn current_format_string_pass_through_is_not_a_structural_inspection_contract() {
    let source = "{r    @empty='' |  🍒  }";
    let context = CompileContext {
        policy_bindings: std::collections::BTreeMap::from([(
            "source".into(),
            ItemStream::once(Item::Atomic(AtomValue::String(source.into()))),
        )]),
        ..Default::default()
    };
    let query = compile("cemml:format(source)", &context).unwrap();
    let output = evaluate(
        &query,
        &EvaluationContext {
            policy_bindings: context.policy_bindings,
            ..Default::default()
        },
    );
    assert!(output.error.is_none(), "{:?}", output.diagnostics);
    assert_eq!(
        output.items,
        vec![Item::Atomic(AtomValue::String(source.into()))]
    );
}
