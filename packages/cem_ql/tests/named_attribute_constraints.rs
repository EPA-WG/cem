use cem_ml::schema::document_model::{AttributeModel, AttributeValueContract};
use cem_ml::value::artifact::{CemValueArtifactLimits, CemValueGraph};
use cem_ql::eval::{
    output::output_attribute,
    portable::{decode_values, encode_values},
    AtomValue, Item, ItemStream,
};
use cem_ql::render::*;

fn data() -> TemplateData {
    let mut data = TemplateData::default();
    data.value_types.insert(
        "positive-count".into(),
        AttributeValueContract {
            model: AttributeModel {
                value_type: Some("integer".into()),
                min_inclusive: Some("3".into()),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    data.value_types.insert(
        "uppercase-code".into(),
        AttributeValueContract {
            model: AttributeModel {
                value_type: Some("string".into()),
                pattern: Some("[A-Z]+".into()),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    data
}

fn attribute(source: &str, data: &TemplateData) -> RenderPlanAttribute {
    let plan = render_compiled_template(
        &compile_template(source, &CompileTemplateOptions::default()),
        data,
    );
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    let RenderPlanNode::Element { attributes, .. } = &plan.nodes[0] else {
        panic!("element required")
    };
    attributes[0].clone()
}

fn save_fixture(name: &str, bytes: &[u8]) {
    if let Some(directory) = std::env::var_os("CEM_NATIVE_CONSTRAINT_FIXTURES") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join(name), bytes).unwrap();
    }
}

// A correctly hashed but invalid graph proves rejection is semantic, rather
// than merely an integrity mismatch. This bypass is confined to test code.
fn unchecked_artifact(graph: &CemValueGraph) -> Vec<u8> {
    let body = rmp_serde::to_vec_named(graph).unwrap();
    let hash = cem_ml::content_cache::ContentHash::from_blake3(&body).hex;
    let mut bytes = b"CEMV\x03".to_vec();
    bytes.extend(
        (0..hash.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hash[i..i + 2], 16).unwrap()),
    );
    bytes.extend(body);
    bytes
}

#[test]
fn output_cannot_weaken_named_bounds_or_patterns() {
    for source in [
        "{output | {attribute @name=count @type=positive-count @minInclusive=1 @value=2}}",
        "{output | {attribute @name=code @type=uppercase-code @pattern='[A-Za-z]+' @value=abc}}",
        "{output | {attribute @name=code @type=uppercase-code @pattern='[0-9]+' @value=ABC}}",
        "{output | {attribute @name=count @type=positive-count @minInclusive=5 @value=4}}",
    ] {
        let output = render_template(source, &data());
        assert!(
            output.rendered.is_empty(),
            "invalid output: {}",
            output.rendered
        );
        assert!(!output.diagnostics.is_empty());
    }
    let output = render_template(
        "{output | {attribute @name=count @type=positive-count @minInclusive=5 @value=005}}",
        &data(),
    );
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(output.rendered, "<output count=\"5\"></output>");
}

#[test]
fn receiver_and_hook_validate_composed_named_contracts() {
    let mut data = data();
    data.value_types
        .get_mut("positive-count")
        .unwrap()
        .restrictions
        .push(AttributeModel {
            max_inclusive: Some("5".into()),
            ..Default::default()
        });
    for value in [2, 6] {
        let data = data.clone().with_binding(
            "count",
            ItemStream::once(Item::Atomic(AtomValue::Integer(value))),
        );
        for source in [
            "{attribute @name=count @type=positive-count @minInclusive=1 @maxInclusive=10}{p | {$count}}",
            "{template @on=expression @into=attribute @returns=positive-count | {$value}}{p @count='{count}'}",
        ] {
            let output = render_template(source, &data);
            assert!(output.rendered.is_empty(), "{}", output.rendered);
            assert!(!output.diagnostics.is_empty());
        }
    }
}

#[test]
fn portable_contract_retains_every_restriction_and_rejects_tampering() {
    let limits = CemValueArtifactLimits::default();
    for (source, invalid) in [
        ("{output | {attribute @name=count @type=positive-count @minInclusive=1 @maxInclusive=5 @value=3}}", ["2", "6"]),
        ("{output | {attribute @name=code @type=uppercase-code @pattern='[A-Za-z]{3}' @value=ABC}}", ["abc", "ABCD"]),
    ] {
        let attribute = attribute(source, &data());
        assert_eq!(attribute.contract.as_ref().unwrap().restrictions.len(), 1);
        let expected = attribute.contract.as_deref().unwrap().clone();
        let bytes = encode_values(&ItemStream::once(output_attribute(attribute)), &limits).unwrap();
        assert_eq!(bytes[4], 3);
        save_fixture(&format!("{}-valid.cemv", expected.model.name), &bytes);
        let decoded = decode_values(&bytes, &limits).unwrap();
        assert_eq!(decoded.items[0].view().unwrap().value_contract().unwrap(), expected);
        let graph = CemValueGraph::decode(&bytes, &limits).unwrap();
        for (index, invalid) in invalid.into_iter().enumerate() {
            let mut graph = graph.clone();
            let root = graph.roots[0] as usize;
            let value = graph.records[root].values[0] as usize;
            graph.records[value].lexical = invalid.into();
            assert!(graph.validate(&limits).is_err(), "accepted {invalid}");
            let invalid_bytes = unchecked_artifact(&graph);
            assert!(decode_values(&invalid_bytes, &limits).is_err());
            save_fixture(&format!("{}-invalid-{index}.cemv", expected.model.name), &invalid_bytes);
        }
        for version in [1, 2, 4] {
            let mut changed = bytes.clone();
            changed[4] = version;
            assert!(decode_values(&changed, &limits).is_err(), "accepted version {version}");
        }
    }
}

#[test]
fn destination_constraints_are_retained_and_final_conversion_is_revalidated() {
    let mut data = data();
    data.attribute_contracts.insert(
        "count".into(),
        AttributeValueContract {
            model: AttributeModel {
                value_type: Some("integer".into()),
                max_inclusive: Some("5".into()),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let attribute = attribute(
        "{output | {attribute @name=count @type=positive-count @minInclusive=1 @value=3}}",
        &data,
    );
    let contract = attribute.contract.unwrap();
    assert_eq!(contract.restrictions.len(), 2);
    for value in ["2", "6"] {
        assert!(cem_ml::schema::document_model::convert_attribute_value(
            value,
            &contract,
            &Default::default()
        )
        .is_err());
    }
    let output = render_template(
        "{output | {attribute @name=count @type=string @pattern='003' @value=003}}",
        &data,
    );
    assert!(
        output.rendered.is_empty(),
        "local restrictions must check the final integer, not the earlier string"
    );
    assert!(!output.diagnostics.is_empty());
    data.input_attribute_contracts = data.attribute_contracts.clone();
    data.bindings.insert(
        "count".into(),
        ItemStream::once(Item::Atomic(AtomValue::String("003".into()))),
    );
    let output = render_template(
        "{attribute @name=count @type=string @pattern='003'}{p | {$count}}",
        &data,
    );
    assert!(
        output.rendered.is_empty(),
        "receiver restrictions must also check the final value"
    );
    assert!(!output.diagnostics.is_empty());
}

#[test]
fn composed_contract_metadata_obeys_artifact_and_child_memory_limits() {
    use cem_ml::operation_control::{
        ExecutionScopeKind, ExecutionScopeRegistration, OperationControl, ROOT_EXECUTION_SCOPE_ID,
    };
    use cem_ml::scheduler::ScopePolicy;
    let attribute = attribute(
        "{output | {attribute @name=count @type=positive-count @minInclusive=1 @value=3}}",
        &data(),
    );
    let limits = CemValueArtifactLimits::default();
    let bytes = encode_values(&ItemStream::once(output_attribute(attribute)), &limits).unwrap();
    let mut graph = CemValueGraph::decode(&bytes, &limits).unwrap();
    let root = graph.roots[0] as usize;
    let before = graph.accounted_bytes();
    graph.records[root].contract.as_mut().unwrap().restrictions[0].source_map = Default::default();
    graph.records[root].contract.as_mut().unwrap().restrictions[0].max_inclusive =
        Some("9".repeat(32768));
    assert!(graph.accounted_bytes() >= before + 32768);
    assert!(graph
        .validate(&CemValueArtifactLimits {
            max_bytes: 8192,
            ..limits
        })
        .is_err());
    graph.records[root]
        .contract
        .as_mut()
        .unwrap()
        .restrictions
        .extend(vec![
            AttributeModel {
                min_inclusive: Some("1".into()),
                ..Default::default()
            };
            8
        ]);
    assert!(graph
        .validate(&CemValueArtifactLimits {
            max_values: 8,
            ..limits
        })
        .is_err());
    let control = OperationControl::default();
    let child = control
        .register_scope(
            ROOT_EXECUTION_SCOPE_ID,
            ExecutionScopeRegistration::inherited(
                ExecutionScopeKind::Template,
                "constraint-budget",
                ScopePolicy::host_root().with_memory_bytes(8192),
            ),
        )
        .unwrap();
    let bytes = graph.encode(&limits).unwrap();
    assert!(
        cem_ql::eval::portable::decode_values_with_control(&bytes, &limits, &control, child)
            .is_err()
    );
    assert!(control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_ok());
    assert_eq!(control.memory_charged(child).unwrap(), 0);
}

#[test]
fn clone_and_xpath_selection_preserve_native_attribute_constraints() {
    use cem_ml::resolver::{ResolverPolicy, ResolverRegistry};
    use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
    use cem_ql::xpath::functions::CemtXPathFunctions;
    use std::sync::Arc;
    let attribute = attribute(
        "{output | {attribute @name=code @type=uppercase-code @pattern='[A-Za-z]{3}' @value=ABC}}",
        &data(),
    );
    let contract = attribute.contract.as_deref().unwrap().clone();
    let mut context = EvaluationContext::default();
    context.policy_bindings.insert(
        "attribute".into(),
        ItemStream::once(output_attribute(attribute)),
    );
    CemtXPathFunctions::compile(
        r#"@doc cem-ml 1
@ns t = "https://cem.dev/ns/transform/cem/1"
@default t
{module | {function @name=value.keep @visibility=public @returns=any |
    {param @name=value @type=any @required=true}
    {body | {xpath @context=value @sequence-type="node()" | {expression | .}}}}}"#,
        "memory:constraints.cemt",
    )
    .unwrap()
    .install(
        &mut context.native_functions,
        Arc::new(ResolverRegistry::new()),
        Arc::new(ResolverPolicy::new()),
    )
    .unwrap();
    for source in [
        "dom:clone(attribute)",
        "native:call(\"value.keep\", attribute)",
        "dom:clone(native:call(\"value.keep\", attribute))",
    ] {
        let query = compile(
            source,
            &CompileContext {
                policy_bindings: context.policy_bindings.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        let result = evaluate(&query, &context);
        assert!(result.error.is_none(), "{source}: {:?}", result.diagnostics);
        assert_eq!(
            result.items[0].view().unwrap().value_contract().unwrap(),
            contract,
            "{source}"
        );
        let limits = CemValueArtifactLimits::default();
        let restored = decode_values(&encode_values(&result, &limits).unwrap(), &limits).unwrap();
        assert_eq!(
            restored.items[0].view().unwrap().value_contract().unwrap(),
            contract
        );
    }
    let template = compile_template("{result-element @name=output | {result-sequence @select='native:call(\"value.keep\", attribute)'}}", &CompileTemplateOptions { host_bindings: vec!["attribute".into()], ..Default::default() });
    let plan = render_compiled_template(
        &template,
        &TemplateData {
            bindings: context.policy_bindings,
            native_functions: context.native_functions,
            ..Default::default()
        },
    );
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    let RenderPlanNode::Element { attributes, .. } = &plan.nodes[0] else {
        panic!()
    };
    assert_eq!(attributes[0].contract.as_deref().unwrap(), &contract);
}

#[test]
fn legacy_single_model_contracts_remain_readable() {
    let attribute = attribute(
        "{output | {attribute @name=count @type=integer @value=3}}",
        &TemplateData::default(),
    );
    assert!(attribute.contract.as_ref().unwrap().restrictions.is_empty());
    let limits = CemValueArtifactLimits::default();
    let bytes = encode_values(&ItemStream::once(output_attribute(attribute)), &limits).unwrap();
    let mut graph = CemValueGraph::decode(&bytes, &limits).unwrap();
    graph.records[graph.roots[0] as usize].native_content = false;
    let mut legacy = unchecked_artifact(&graph);
    for version in [1, 2] {
        legacy[4] = version;
        let values = decode_values(&legacy, &limits).unwrap();
        assert!(values.items[0]
            .view()
            .unwrap()
            .value_contract()
            .unwrap()
            .restrictions
            .is_empty());
    }
}
