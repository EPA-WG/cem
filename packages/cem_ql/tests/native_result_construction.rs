//! XSLT-OUTPUT-NATIVE: explicit native results, independent of XSLT authoring.
use cem_ml::{import::import_data, validation::xpath::XPathNativeNode};
use cem_ql::{
    eval::ItemStream,
    render::{
        compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
        TemplateData,
    },
    xpath::functions::XPathQueryItem,
};

fn html(source: &str, data: TemplateData) -> String {
    let artifact = compile_template(
        source,
        &CompileTemplateOptions {
            host_bindings: data.bindings.keys().cloned().collect(),
            ..Default::default()
        },
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    let plan = render_compiled_template(&artifact, &data);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    render_plan_to_html(&plan)
}

#[test]
fn explicit_results_separate_atomics_and_merge_text_without_changing_interpolation() {
    assert_eq!(html("{result-document |{result-element @name=p |{result-sequence @select='(1, 2)'}{$ \"x\"}{result-sequence @select='(3, 4)'}}}", TemplateData::default()), "<p>1 2x3 4</p>");
    assert_eq!(
        html("{p |{$ 1}{$ 2}}", TemplateData::default()),
        "<p>12</p>"
    );
}

#[test]
fn native_subtrees_are_copied_without_stringification_or_document_reparse() {
    let tree = import_data(
        "<r><b id='a'>one</b><b/><!--end--></r>",
        "xml",
        "cem",
        "memory:input",
    )
    .unwrap();
    let data = TemplateData::default().with_binding(
        "selected",
        ItemStream::once(XPathQueryItem::from_node(XPathNativeNode::cem_document(
            tree,
        ))),
    );
    assert_eq!(
        html(
            "{result-document |{result-sequence @select=selected}}",
            data
        ),
        "<r><b id=\"a\">one</b><b></b><!--end--></r>"
    );
}

#[test]
fn ordered_attributes_report_a_typed_construction_failure() {
    let artifact = compile_template("{result-document |{result-element @name=p |{$ \"body\"}{result-attribute @name=title |late}}}", &CompileTemplateOptions::default());
    let plan = render_compiled_template(&artifact, &TemplateData::default());
    assert!(plan.nodes.is_empty());
    assert!(
        plan.diagnostics
            .iter()
            .any(|d| d.code == "cem.ql.result.attribute_order"),
        "{:?}",
        plan.diagnostics
    );
}

#[test]
fn calls_and_recovery_retain_pending_values_until_the_parent_is_constructed() {
    assert_eq!(
        html(
            r#"{template @name=values |{result-sequence @select='(2, 3)'}}{result-document |{result-element @name=p |{result-sequence @select=1}{call @template=values}{try |{result-sequence @select=99}{$ 1 / 0}{catch |{result-sequence @select=4}}}{result-document |}{result-sequence @select=5}}}"#,
            TemplateData::default()
        ),
        "<p>1 2 3 45</p>"
    );
    assert_eq!(
        html(
            r#"{result-document |{result-element @name=p |{result-attribute @name=a |first}{result-attribute @name=a |last}{$ ""}{result-attribute @name=b |empty-text-is-removed}}}"#,
            TemplateData::default()
        ),
        "<p a=\"last\" b=\"empty-text-is-removed\"></p>"
    );
}

#[test]
fn typed_result_instructions_survive_portable_binary_reload() {
    use cem_ql::template_artifact::{
        compile_template_artifact, CompiledTemplateArtifact, TemplateArtifactLoadContext,
        TemplateArtifactSourceMapMode,
    };
    let source = "{result-document |{result-element @name=p |{result-sequence @select='(1, 2)'}}}";
    let compiled = compile_template_artifact(
        source,
        &CompileTemplateOptions::default(),
        TemplateArtifactSourceMapMode::Dev,
    );
    let artifact = CompiledTemplateArtifact::from_bytes(compiled.bytes)
        .unwrap()
        .reload(&TemplateArtifactLoadContext {
            expected_source_hash: Some(cem_ml::content_cache::ContentHash::from_blake3(
                source.as_bytes(),
            )),
            host_bindings: vec![],
            source_map_mode: TemplateArtifactSourceMapMode::Dev,
        })
        .unwrap();
    let plan = render_compiled_template(&artifact, &TemplateData::default());
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert_eq!(render_plan_to_html(&plan), "<p>1 2</p>");
}

#[test]
fn copying_uses_semantic_views_for_all_import_formats_and_retains_provenance() {
    use cem_ql::{eval::imported_cem_tree, render::RenderPlanNode};
    let artifact = compile_template(
        "{result-document |{result-sequence @select=selected}}",
        &CompileTemplateOptions {
            host_bindings: vec!["selected".into()],
            ..Default::default()
        },
    );
    for (source, format) in [
        ("<r><a>one</a></r>", "xml"),
        ("{\"a\":\"one\"}", "json"),
        ("a: one", "yaml"),
        ("a\none", "csv"),
    ] {
        let tree = import_data(source, format, "cem", "memory:input").unwrap();
        let plan = render_compiled_template(
            &artifact,
            &TemplateData::default()
                .with_binding("selected", ItemStream::once(imported_cem_tree(tree))),
        );
        assert!(
            plan.diagnostics.is_empty(),
            "{format}: {:?}",
            plan.diagnostics
        );
        assert!(plan.nodes.iter().any(|node| matches!(node, RenderPlanNode::Element { source_map, .. } if !source_map.frames.is_empty())), "{format}: {:?}", plan.nodes);
        assert!(render_plan_to_html(&plan).contains("one"));
    }
}

#[test]
fn result_budgets_and_cancellation_cannot_be_caught() {
    use cem_ml::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
    use cem_ql::{
        eval::{AtomValue, Item},
        render::render_compiled_template_with_control,
    };
    let source =
        "{try |{result-document |{result-sequence @select=selected}}{catch |must-not-recover}}";
    let artifact = compile_template(
        source,
        &CompileTemplateOptions {
            host_bindings: vec!["selected".into()],
            ..Default::default()
        },
    );
    let data = TemplateData::default().with_binding(
        "selected",
        ItemStream::once(Item::Atomic(AtomValue::String("x".repeat(1024 * 1024 + 1)))),
    );
    let plan = render_compiled_template(&artifact, &data);
    assert!(plan.nodes.is_empty());
    assert!(
        plan.diagnostics
            .iter()
            .any(|d| d.code == "cem.ql.result.limit"),
        "{:?}",
        plan.diagnostics
    );
    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    let plan =
        render_compiled_template_with_control(&artifact, &data, &control, ROOT_EXECUTION_SCOPE_ID);
    assert!(plan.nodes.is_empty());
    assert!(plan
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.render.control_failure"));
}

#[test]
fn deep_results_and_invalid_instruction_shapes_fail_without_partial_output() {
    use cem_ql::eval::{AtomValue, Item};
    let mut value = Item::Atomic(AtomValue::Integer(1));
    for _ in 0..150 {
        value = Item::Array(vec![value]);
    }
    let artifact = compile_template(
        "{try |{result-document |{result-sequence @select=selected}}{catch |must-not-recover}}",
        &CompileTemplateOptions {
            host_bindings: vec!["selected".into()],
            ..Default::default()
        },
    );
    let plan = render_compiled_template(
        &artifact,
        &TemplateData::default().with_binding("selected", ItemStream::once(value)),
    );
    assert!(plan.nodes.is_empty());
    assert!(plan
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.result.limit"));
    for source in [
        "{result-sequence @select=1 |ignored}",
        "{result-document @select=1}",
        "{result-element |missing-name}",
    ] {
        let artifact = compile_template(source, &CompileTemplateOptions::default());
        assert!(artifact
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.ql.result.instruction"));
        assert!(
            render_compiled_template(&artifact, &TemplateData::default())
                .nodes
                .is_empty()
        );
    }
}

#[test]
fn simple_content_keeps_array_member_text_boundaries() {
    use cem_ql::eval::Item;
    let tree = import_data("<r>a<x/>b</r>", "xml", "cem", "memory:input").unwrap();
    let children = XPathNativeNode::cem_document(tree).child_nodes()[0].child_nodes();
    let value = Item::Array(vec![
        XPathQueryItem::from_node(children[0].clone()),
        XPathQueryItem::from_node(children[2].clone()),
    ]);
    let data = TemplateData::default().with_binding("selected", ItemStream::once(value));
    assert_eq!(html("{result-document |{result-element @name=p |{result-attribute @name=title |{result-sequence @select=selected}}}}", data), "<p title=\"a b\"></p>");
}
