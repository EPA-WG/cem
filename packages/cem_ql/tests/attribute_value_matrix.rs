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

// CEMT-LARGE-INTEGER: query representation must not depend on transport.
#[test]
fn integer_boundaries_keep_atoms_query_types_and_metadata_across_handoff() {
    for (lexical, small) in [
        ("9223372036854775807", true),
        ("-9223372036854775808", true),
        ("9223372036854775808", false),
        ("-9223372036854775809", false),
        ("922337203685477580812345", false),
        ("-922337203685477580812345", false),
    ] {
        let sender = attributes(
            &format!("{{child | {{attribute @name=value @type=integer @value='{lexical}'}}}}"),
            &TemplateData::default(),
        );
        let atom = if small {
            AtomValue::Integer(lexical.parse().unwrap())
        } else {
            AtomValue::Decimal(lexical.into())
        };
        let expected = if small {
            "<p>true|false|false</p>"
        } else {
            "<p>false|true|false</p>"
        };
        for portable in [false, true] {
            let data = transport(sender.clone(), portable);
            assert_eq!(
                data.bindings["value"].items[0].atom(),
                Some(atom.clone()),
                "{lexical} portable={portable}"
            );
            for declaration in ["", "{attribute @name=value @type=integer @required=true}"] {
                let result = render_template(&format!("{declaration}{{p | {{$value is integer}}|{{$value is decimal}}|{{$value is string}}}}"), &data);
                assert!(
                    result.diagnostics.is_empty(),
                    "{lexical}: {:?}",
                    result.diagnostics
                );
                assert_eq!(result.rendered, expected);
                // Export again after optional receiver conversion: query atoms
                // may be decimal while the retained datatype remains integer.
                let output =
                    attributes(&format!("{declaration}{{child @value='{{value}}'}}"), &data);
                let limits = CemValueArtifactLimits::default();
                let graph = CemValueGraph::decode(
                    &encode_values(&output[0].value_stream, &limits).unwrap(),
                    &limits,
                )
                .unwrap();
                let value = &graph.records[graph.roots[0] as usize];
                assert_eq!(value.datatype, "integer");
                assert_eq!(value.lexical, lexical);
            }
        }
        let original = output_attribute(sender[0].clone());
        let source = original.source_map().unwrap();
        assert!(!source.frames.is_empty());
        let limits = CemValueArtifactLimits::default();
        let restored = decode_values(
            &encode_values(&ItemStream::once(original), &limits).unwrap(),
            &limits,
        )
        .unwrap();
        assert_eq!(restored.items[0].source_map().unwrap(), source);
        assert_eq!(
            restored.items[0].view().unwrap().value_contract(),
            sender[0].contract.as_deref().cloned()
        );
    }
}

#[test]
fn large_integer_arithmetic_is_exact_before_and_after_receiver_conversion() {
    for lexical in [
        "9223372036854775808",
        "-9223372036854775809",
        "922337203685477580812345",
        "-922337203685477580812345",
    ] {
        let sender = attributes(
            &format!("{{child | {{attribute @name=value @type=integer @value='{lexical}'}}}}"),
            &TemplateData::default(),
        );
        for portable in [false, true] {
            let data = transport(sender.clone(), portable);
            for declaration in ["", "{attribute @name=value @type=integer}"] {
                let result = render_template(
                    &format!("{declaration}{{p | {{$value * 0.0}}|{{$value + 1.0}}}}"),
                    &data,
                );
                assert!(
                    result.diagnostics.is_empty(),
                    "{lexical}: {:?}",
                    result.diagnostics
                );
                assert_eq!(
                    result.rendered,
                    format!("<p>0|{}</p>", lexical.parse::<i128>().unwrap() + 1)
                );
            }
        }
    }
}

#[test]
fn integer_hook_returns_and_reloaded_templates_use_numeric_atoms() {
    use cem_ql::template_artifact::{
        compile_template_artifact, TemplateArtifactLoadContext, TemplateArtifactSourceMapMode,
    };
    let source = concat!(
        "{template @on=expression @into=attribute @returns=integer | {$value}}",
        "{child @value='{\"922337203685477580812345\"}'}"
    );
    let options = CompileTemplateOptions::default();
    let artifact = compile_template_artifact(source, &options, TemplateArtifactSourceMapMode::Dev);
    let compiled = artifact
        .reload(&TemplateArtifactLoadContext {
            expected_source_hash: Some(cem_ml::content_cache::ContentHash::from_blake3(
                source.as_bytes(),
            )),
            host_bindings: vec![],
            source_map_mode: TemplateArtifactSourceMapMode::Dev,
        })
        .unwrap();
    let plan = render_compiled_template(&compiled, &TemplateData::default());
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    let RenderPlanNode::Element {
        attributes: sender, ..
    } = &plan.nodes[0]
    else {
        panic!()
    };
    assert_eq!(
        sender[0].value_stream.items[0].atom(),
        Some(AtomValue::Decimal("922337203685477580812345".into()))
    );
    assert_eq!(
        sender[0].value_stream.items[0]
            .view()
            .unwrap()
            .field("datatype")
            .unwrap()[0]
            .atom(),
        Some(AtomValue::String("integer".into()))
    );
    let data = transport(sender.clone(), false);
    let result = render_template("{p | {$value + 1.0}}", &data);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.rendered, "<p>922337203685477580812346</p>");
}

#[test]
fn large_integer_transport_preserves_existing_arithmetic_errors() {
    use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
    for (lexical, expression, message) in [
        (
            "9223372036854775808",
            "value + 1",
            "matching numeric operand types",
        ),
        (
            "9223372036854775808",
            "value / 0.0",
            "divide decimal by zero",
        ),
        (
            "170141183460469231731687303715884105727",
            "value + 1.0",
            "overflowed decimal",
        ),
        (
            "170141183460469231731687303715884105728",
            "value + 0.0",
            "finite decimal operands",
        ),
    ] {
        let sender = attributes(
            &format!("{{child | {{attribute @name=value @type=integer @value='{lexical}'}}}}"),
            &TemplateData::default(),
        );
        for portable in [false, true] {
            let data = transport(sender.clone(), portable);
            let query = compile(
                expression,
                &CompileContext {
                    policy_bindings: data.bindings.clone(),
                    ..Default::default()
                },
            )
            .unwrap();
            let result = evaluate(
                &query,
                &EvaluationContext {
                    policy_bindings: data.bindings,
                    ..Default::default()
                },
            );
            assert!(result.error.is_some(), "{lexical}: {expression}");
            assert!(result.items.is_empty());
            assert!(
                result
                    .diagnostics
                    .iter()
                    .any(|d| d.message.contains(message)),
                "{lexical}: {:?}",
                result.diagnostics
            );
        }
    }
}
