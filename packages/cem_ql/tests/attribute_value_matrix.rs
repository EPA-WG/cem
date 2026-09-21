//! CEMT-ATTRIBUTE-MATRIX: native conversion, handoff and final representation.
use cem_ml::value::artifact::{CemValueArtifactLimits, CemValueGraph};
use cem_ql::eval::{
    output::output_attribute,
    portable::{decode_values, encode_values},
    AtomValue, Item, ItemStream,
};
use cem_ql::render::*;

fn attributes(source: &str, data: &TemplateData) -> Vec<RenderPlanAttribute> {
    let options = CompileTemplateOptions {
        host_bindings: data.bindings.keys().cloned().collect(),
        ..Default::default()
    };
    let plan = render_compiled_template(&compile_template(source, &options), data);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    let RenderPlanNode::Element { attributes, .. } = &plan.nodes[0] else {
        panic!()
    };
    attributes.clone()
}

fn transport(attributes: Vec<RenderPlanAttribute>, portable: bool) -> TemplateData {
    let mut values = ItemStream::from_items(attributes.into_iter().map(output_attribute).collect());
    if portable {
        let limits = CemValueArtifactLimits::default();
        values = decode_values(&encode_values(&values, &limits).unwrap(), &limits).unwrap();
    }
    let mut data = TemplateData::default();
    for value in values.items {
        data.bind_native_attribute(value).unwrap();
    }
    data
}

#[test]
fn scalar_lexical_values_and_datatypes_survive_component_handoff() {
    for (datatype, input, stored_type, expected) in [
        ("integer", "+002", "integer", "2"),
        (
            "integer",
            "922337203685477580812345",
            "integer",
            "922337203685477580812345",
        ),
        (
            "decimal",
            "12345678901234567890.123456789",
            "decimal",
            "12345678901234567890.123456789",
        ),
        ("number", "1.25e2", "decimal", "1.25e2"),
        ("boolean", "1", "boolean", "true"),
        ("boolean", "0", "boolean", "false"),
        ("date", "2024-02-29-14:00", "date", "2024-02-29-14:00"),
        ("time", "23:59:59.123456789Z", "time", "23:59:59.123456789Z"),
        (
            "dateTime",
            "2024-02-29T12:00:00+05:30",
            "dateTime",
            "2024-02-29T12:00:00+05:30",
        ),
        (
            "datetime",
            "2024-02-29T12:00:00",
            "datetime",
            "2024-02-29T12:00:00",
        ),
    ] {
        let sender =
            format!("{{child | {{attribute @name=value @type={datatype} @value='{input}'}}}}");
        let receiver =
            format!("{{attribute @name=value @type={datatype} @required=true}}{{p | {{$value}}}}");
        let sender = attributes(&sender, &TemplateData::default());
        let limits = CemValueArtifactLimits::default();
        let bytes = encode_values(&sender[0].value_stream, &limits).unwrap();
        let graph = CemValueGraph::decode(&bytes, &limits).unwrap();
        assert_eq!(graph.records[graph.roots[0] as usize].datatype, stored_type);
        assert_eq!(graph.records[graph.roots[0] as usize].lexical, expected);
        for portable in [false, true] {
            let data = transport(sender.clone(), portable);
            let plan = render_template(&receiver, &data);
            assert!(
                plan.diagnostics.is_empty(),
                "{datatype} portable={portable}: {:?}",
                plan.diagnostics
            );
            assert_eq!(
                plan.rendered,
                format!("<p>{expected}</p>"),
                "{datatype} portable={portable}"
            );
        }
    }
}

#[test]
fn invalid_conversion_fails_output_receiver_and_hook_without_partial_results() {
    for (datatype, value) in [
        ("integer", "1.5"),
        ("decimal", "NaN"),
        ("number", "Infinity"),
        ("boolean", "maybe"),
        ("date", "1900-02-29"),
        ("time", "24:00:00"),
        ("dateTime", "2024-01-01T12:00:00+14:01"),
    ] {
        let binding = TemplateData::default().with_binding(
            "value",
            ItemStream::once(Item::Atomic(AtomValue::String(value.into()))),
        );
        for source in [
            format!("{{p | prefix}}{{p | {{attribute @name=value @type={datatype} @value='{value}'}}}}{{p | suffix}}"),
            format!("{{attribute @name=value @type={datatype}}}{{p | prefix}}{{p | {{$value}}}}"),
            format!("{{template @on=expression @into=attribute @returns={datatype} | {{$value}}}}{{p | prefix}}{{p @title='{{value}}'}}"),
        ] {
            let result = render_template(&source, &binding);
            assert!(result.rendered.is_empty(), "{datatype}: {}", result.rendered);
            assert!(result.diagnostics.iter().any(|d| d.severity == cem_ml::diagnostics::Severity::Error), "{datatype}: {:?}", result.diagnostics);
        }
    }
}

#[test]
fn mixed_native_attributes_retain_segments_until_the_requested_projection() {
    use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
    let query = compile(
        r#"data:read("<name>ivy<em>saur</em></name>", "xml").root.children"#,
        &CompileContext::default(),
    )
    .unwrap();
    let node = evaluate(&query, &EvaluationContext::default());
    assert!(node.error.is_none());
    let data = TemplateData::default().with_binding("node", node.clone());
    for (content_type, expected) in [
        ("text/plain", "before<&ivysaur2trueafter"),
        (
            "text/html",
            "before&lt;&amp;<name>ivy<em>saur</em></name>2trueafter",
        ),
        (
            "application/xml",
            "before&lt;&amp;<name>ivy<em>saur</em></name>2trueafter",
        ),
        (
            "text/xml",
            "before&lt;&amp;<name>ivy<em>saur</em></name>2trueafter",
        ),
    ] {
        let source = format!("{{child | {{attribute @name=label @type=any @content-type={content_type} | {{$\"before<&\"}}{{$node}}{{$2}}{{$true}}after}}}}");
        let sender = attributes(&source, &data);
        let values = &sender[0].value_stream.items;
        assert_eq!(values.len(), 5);
        assert_eq!(values[1].identity(), node.items[0].identity());
        assert_eq!(values[2].atom(), Some(AtomValue::Integer(2)));
        assert_eq!(values[3].atom(), Some(AtomValue::Boolean(true)));
        assert_eq!(
            project_attribute_value(&sender[0]),
            expected,
            "{content_type}"
        );
        for portable in [false, true] {
            let receiver = transport(sender.clone(), portable);
            assert_eq!(receiver.bindings["label"].items.len(), 5);
            let result = render_template(
                "{attribute @name=label @type=any @required=true}{p | {$label}}",
                &receiver,
            );
            assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
            assert_eq!(
                result.rendered,
                "<p>before&lt;&amp;<name>ivy<em>saur</em></name>2trueafter</p>"
            );
        }
        assert_eq!(values[1].identity(), node.items[0].identity());
    }
}
