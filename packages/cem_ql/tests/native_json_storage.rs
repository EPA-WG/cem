//! Stored JSON enters CEM-ML import and leaves through explicit native export.
use cem_ml::value::{
    artifact::{CemValueArtifactLimits, CemValueGraph},
    json::write_json,
};
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    eval::{portable::encode_values, AtomValue, Item, ItemStream},
};

fn graph(query: &str, source: &str) -> CemValueGraph {
    let context = EvaluationContext {
        policy_bindings: [(
            "source".into(),
            ItemStream::once(Item::Atomic(AtomValue::String(source.into()))),
        )]
        .into(),
        ..Default::default()
    };
    let compiled = compile(
        query,
        &CompileContext {
            policy_bindings: context.policy_bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let result = evaluate(&compiled, &context);
    assert!(result.error.is_none(), "{:?}", result.diagnostics);
    let limits = CemValueArtifactLimits::default();
    CemValueGraph::decode(&encode_values(&result, &limits).unwrap(), &limits).unwrap()
}

#[test]
fn json_documents_export_order_duplicates_and_exact_scalar_lexemes() {
    for source in [
        r#"{"z":922337203685477580812345,"a":-0.0,"z":1.00e+20}"#,
        r#"["quote\"\n🍒",false,0,null,[],{}]"#,
        "false",
        "0",
        "null",
        "\"\"",
    ] {
        let graph = graph("data:read(source, \"json\").root", source);
        assert_eq!(
            write_json(&graph, &CemValueArtifactLimits::default()).unwrap(),
            source
        );
    }
}

#[test]
fn native_references_and_clones_export_without_mutating_their_input() {
    for query in [
        "data:read(source, \"json\").root.children",
        "dom:reference(data:read(source, \"json\").root)",
        "dom:clone(data:read(source, \"json\").root)",
    ] {
        let graph = graph(query, r#"{"fruit":"🍒","count":0}"#);
        let before = graph.encode(&CemValueArtifactLimits::default()).unwrap();
        assert_eq!(
            write_json(&graph, &CemValueArtifactLimits::default()).unwrap(),
            r#"{"fruit":"🍒","count":0}"#
        );
        assert_eq!(
            graph.encode(&CemValueArtifactLimits::default()).unwrap(),
            before
        );
    }
}

#[test]
fn typed_native_scalars_export_at_the_named_json_boundary() {
    for (query, expected) in [
        ("false", "false"),
        ("0", "0"),
        ("1.25", "1.25"),
        ("null", "null"),
        ("source", "\"ABC\""),
    ] {
        assert_eq!(
            write_json(&graph(query, "ABC"), &CemValueArtifactLimits::default()).unwrap(),
            expected
        );
    }
}

#[test]
fn invalid_shapes_and_limits_never_publish_partial_json() {
    for source in [
        "<object/>",
        "<object xmlns='cem:generic-data'><property name='a'/></object>",
        "<object xmlns='cem:generic-data'><property><null/></property></object>",
        "<number xmlns='cem:generic-data'>01</number>",
        "<number xmlns='cem:generic-data'>NaN</number>",
        "<boolean xmlns='cem:generic-data'>yes</boolean>",
        "<null xmlns='cem:generic-data'>not null</null>",
        "<string xmlns='cem:generic-data'><string>nested</string></string>",
        "<array xmlns='cem:generic-data' extra='lost'/>",
        "<array xmlns='cem:generic-data' xmlns:p='urn:other' p:xmlns='lost'/>",
    ] {
        assert!(
            write_json(
                &graph("data:read(source, \"xml\").root", source),
                &CemValueArtifactLimits::default()
            )
            .is_err(),
            "{source}"
        );
    }
    assert!(write_json(&graph("(1, 2)", ""), &CemValueArtifactLimits::default()).is_err());
    let input = graph("data:read(source, \"json\").root", r#"{"a":["value"]}"#);
    for limits in [
        CemValueArtifactLimits {
            max_bytes: 8,
            ..Default::default()
        },
        CemValueArtifactLimits {
            max_values: 2,
            ..Default::default()
        },
        CemValueArtifactLimits {
            max_depth: 1,
            ..Default::default()
        },
    ] {
        assert!(write_json(&input, &limits).is_err());
    }
}

#[test]
fn host_io_and_native_slice_render_keep_documents_out_of_control_records() {
    use cem_ql::{
        api::native_values,
        eval::portable::decode_values,
        render::{render_template, TemplateData},
    };
    let limits = CemValueArtifactLimits::default();
    let source = br#"{"a":1,"b":"B"}"#;
    let bytes = native_values::import_document(source, "application/json", "storage:test", &limits)
        .unwrap();
    assert_eq!(
        native_values::export_json(&bytes, 0, None, &limits)
            .unwrap()
            .as_deref(),
        Some(std::str::from_utf8(source).unwrap())
    );
    let mut data = TemplateData::default();
    // The browser compiles scalar path aliases before the asynchronous import.
    data.bindings.insert(
        "datadom.slices.config".into(),
        ItemStream::once(Item::Atomic(AtomValue::Null)),
    );
    let compiled = cem_ql::render::compile_template(
        "{output | {$dom:text(datadom.slices.config.children.children.children)}}",
        &cem_ql::render::CompileTemplateOptions {
            host_bindings: data.bindings.keys().cloned().collect(),
            ..Default::default()
        },
    );
    data.bind_native_slice("config", decode_values(&bytes, &limits).unwrap())
        .unwrap();
    let plan = cem_ql::render::render_compiled_template(&compiled, &data);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert_eq!(
        cem_ql::render::render_plan_to_html(&plan),
        "<output>1B</output>"
    );
    let rendered = render_template(
        r#"{span | {$dom:text(datadom.slices.config.children.children.children)}-{$dom:text(config.children.children.children)}}"#,
        &data,
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
    assert_eq!(rendered.rendered, "<span>1B-1B</span>");
    assert!(
        native_values::import_document(b"{bad", "application/json", "storage:test", &limits)
            .is_err()
    );
    assert!(native_values::export_json(&bytes, 1, None, &limits).is_err());
    assert!(native_values::export_json(&bytes, 0, Some("slice-value"), &limits).is_err());
    let null = native_values::import_document(b"null", "application/json", "storage:test", &limits)
        .unwrap();
    assert_eq!(
        native_values::export_json(&null, 0, None, &limits)
            .unwrap()
            .as_deref(),
        Some("null")
    );
    for query in ["null", "dom:reference(null)", "dom:reference(())"] {
        let null = graph(query, "").encode(&limits).unwrap();
        assert_eq!(
            native_values::export_json(&null, 0, None, &limits).unwrap(),
            None
        );
    }
}

#[test]
fn constructed_event_values_export_and_rebind_without_mutating_the_source() {
    use cem_ql::{
        api::native_values,
        eval::{output::output_attribute, portable::decode_values},
        render::{
            compile_template, render_compiled_template, render_template, CompileTemplateOptions,
            RenderPlanNode, TemplateData,
        },
    };
    let limits = CemValueArtifactLimits::default();
    let bytes = native_values::import_document(
        br#"{"cherries":12,"lemons":1}"#,
        "application/json",
        "storage:basket",
        &limits,
    )
    .unwrap();
    let mut data = TemplateData::default();
    data.bind_native_slice("basket", decode_values(&bytes, &limits).unwrap())
        .unwrap();
    let demo = include_str!("../../cem-elements/demo/local-storage.html");
    let source = demo
        .split("legend=\"5. Live JSON basket\"")
        .nth(1)
        .unwrap()
        .split("<template type=\"text/cem-ml\">")
        .nth(1)
        .unwrap()
        .split("</template>")
        .next()
        .unwrap();
    let empty = render_template(source, &TemplateData::default());
    assert!(empty.diagnostics.is_empty(), "{:?}", empty.diagnostics);
    let compiled = compile_template(
        source,
        &CompileTemplateOptions {
            host_bindings: data.bindings.keys().cloned().collect(),
            ..Default::default()
        },
    );
    let plan = render_compiled_template(&compiled, &data);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    let RenderPlanNode::Element { attributes, .. } = plan
        .nodes
        .iter()
        .find(|n| matches!(n, RenderPlanNode::Element { tag, .. } if tag == "button"))
        .unwrap()
    else {
        panic!("missing button")
    };
    let attribute = attributes.iter().find(|a| a.name == "slice-value").unwrap();
    let event = encode_values(
        &ItemStream::once(output_attribute(attribute.clone())),
        &limits,
    )
    .unwrap();
    assert_eq!(
        native_values::export_json(&event, 0, Some("slice-value"), &limits)
            .unwrap()
            .as_deref(),
        Some(r#"{"cherries":13,"lemons":1}"#)
    );
    let restored = decode_values(&event, &limits).unwrap();
    data.bind_native_slice(
        "basket",
        native_values::attribute_values(&restored.items[0], "slice-value").unwrap(),
    )
    .unwrap();
    let rendered = render_template(
        "{output | {$dom:text(datadom.slices.basket.children.children)}}",
        &data,
    );
    assert_eq!(
        rendered.rendered.split_whitespace().collect::<String>(),
        "<output>131</output>"
    );
    let plan = render_compiled_template(&compiled, &data);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    let RenderPlanNode::Element { attributes, .. } = plan
        .nodes
        .iter()
        .filter(|n| matches!(n, RenderPlanNode::Element { tag, .. } if tag == "button"))
        .nth(1)
        .unwrap()
    else {
        panic!()
    };
    let attribute = attributes.iter().find(|a| a.name == "slice-value").unwrap();
    let next = encode_values(
        &ItemStream::once(output_attribute(attribute.clone())),
        &limits,
    )
    .unwrap();
    assert_eq!(
        native_values::export_json(&next, 0, Some("slice-value"), &limits)
            .unwrap()
            .as_deref(),
        Some(r#"{"cherries":13,"lemons":2}"#)
    );
    assert_eq!(
        native_values::export_json(&bytes, 0, None, &limits)
            .unwrap()
            .as_deref(),
        Some(r#"{"cherries":12,"lemons":1}"#)
    );
}

#[test]
fn a_one_value_ceiling_accepts_one_atomic_value() {
    let value = graph("1", "");
    assert_eq!(
        write_json(
            &value,
            &CemValueArtifactLimits {
                max_values: 1,
                ..Default::default()
            }
        )
        .unwrap(),
        "1"
    );
}
