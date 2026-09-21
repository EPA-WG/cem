use cem_ml::value::artifact::CemValueArtifactLimits;
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::portable::{decode_values, encode_values};
use cem_ql::eval::{Item, ItemStream};

fn eval(source: &str, values: ItemStream) -> ItemStream {
    let mut compile_context = CompileContext::default();
    compile_context
        .policy_bindings
        .insert("values".into(), values.clone());
    let query = compile(source, &compile_context).unwrap();
    let mut context = EvaluationContext::default();
    context.policy_bindings.insert("values".into(), values);
    let result = evaluate(&query, &context);
    assert!(result.error.is_none(), "{:?}", result.diagnostics);
    result
}

#[test]
fn portable_reference_preserves_targets_source_parent_and_text() {
    let values = eval(
        r#"let doc = data:read("<r><name>ivy<em>saur</em></name><id>2</id></r>", "xml");
        let n = seq:first(doc.root.children.children);
        dom:reference((n, n))"#,
        ItemStream::empty(),
    );
    let limits = CemValueArtifactLimits::default();
    let source_key = eval("data:node_key(seq:first(values.targets))", values.clone());
    let bytes = encode_values(&values, &limits).unwrap();
    assert!(bytes.starts_with(b"CEMV"));
    let restored = decode_values(&bytes, &limits).unwrap();
    assert_eq!(
        eval("data:node_key(seq:first(values.targets))", restored.clone()).items,
        source_key.items
    );
    let targets = restored.items[0].view().unwrap().field("targets").unwrap();
    assert_eq!(targets[0].identity(), targets[1].identity());
    assert_ne!(restored.items[0].identity(), targets[0].identity());
    let text = eval("dom:text(dom:parent(seq:first(values.targets)))", restored);
    assert_eq!(text.items, eval("\"ivysaur2\"", ItemStream::empty()).items);
}

#[test]
fn portable_values_reject_corruption_and_lowered_limits() {
    let limits = CemValueArtifactLimits::default();
    let bytes =
        encode_values(&eval("dom:reference((1, 2))", ItemStream::empty()), &limits).unwrap();
    assert!(decode_values(&bytes[..bytes.len() - 1], &limits).is_err());
    assert!(decode_values(
        &bytes,
        &CemValueArtifactLimits {
            max_bytes: 8,
            ..limits
        }
    )
    .is_err());
    let _ = std::mem::size_of::<Item>();
}

#[test]
fn typed_attribute_artifact_can_feed_another_component() {
    use cem_ql::render::{
        compile_template, render_compiled_template, render_template, CompileTemplateOptions,
        TemplateData,
    };
    let template = compile_template(
        r#"{child | {attribute @name=count @type=integer @value='002'}{attribute @name=day @type=date @value='2024-02-29'}{attribute @name=label @type=node @content-type=text/html @value='{data:read("<name>ivy<em>saur</em></name>", "xml").root.children}'}}"#,
        &CompileTemplateOptions::default(),
    );
    let plan = render_compiled_template(&template, &TemplateData::default());
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    let cem_ql::render::RenderPlanNode::Element { attributes, .. } = &plan.nodes[0] else {
        panic!()
    };
    let values = ItemStream::from_items(
        attributes
            .iter()
            .cloned()
            .map(cem_ql::eval::output::output_attribute)
            .collect(),
    );
    let limits = CemValueArtifactLimits::default();
    let bytes = encode_values(&values, &limits).unwrap();
    let restored = decode_values(&bytes, &limits).unwrap();
    let mut data = TemplateData::default();
    for value in restored.items {
        data.bind_native_attribute(value).unwrap();
    }
    let output = render_template("{output @day='{day}' | {$count + 1}{$label}}", &data);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(
        output.rendered,
        "<output day=\"2024-02-29\">3<name>ivy<em>saur</em></name></output>"
    );
    assert_eq!(
        data.bindings["day"].items[0]
            .view()
            .unwrap()
            .field("datatype")
            .unwrap()[0]
            .atom(),
        Some(cem_ql::eval::AtomValue::String("date".into()))
    );
}

#[test]
fn portable_graph_limits_follow_shared_paths_and_expansion() {
    use cem_ml::value::artifact::{CemValueGraph, CemValueRecord};
    let reference = |targets| CemValueRecord {
        kind: "reference".into(),
        targets,
        ..Default::default()
    };
    // Record zero has already been validated when the longer path reaches it.
    let graph = CemValueGraph {
        roots: vec![3],
        records: vec![
            CemValueRecord::default(),
            reference(vec![0]),
            reference(vec![1]),
            reference(vec![2]),
        ],
    };
    let limits = CemValueArtifactLimits {
        max_depth: 2,
        ..Default::default()
    };
    assert!(graph.validate(&limits).is_err());
    let graph = CemValueGraph {
        roots: vec![3],
        records: vec![
            CemValueRecord::default(),
            reference(vec![0, 0]),
            reference(vec![1, 1]),
            reference(vec![2, 2]),
        ],
    };
    assert!(graph
        .validate(&CemValueArtifactLimits {
            max_values: 10,
            ..Default::default()
        })
        .is_err());
}

#[test]
fn portable_graph_rejects_invalid_typed_lexical_values() {
    use cem_ml::value::artifact::{CemValueGraph, CemValueRecord};
    for (datatype, lexical) in [
        ("integer", "bad"),
        ("decimal", "NaN"),
        ("number", "Infinity"),
        ("boolean", "maybe"),
        ("date", "2023-02-29"),
        ("time", "24:00:00"),
        ("dateTime", "2024-02-29T12:00:00+14:01"),
        ("datetime", "2023-02-29T12:00:00"),
    ] {
        let graph = CemValueGraph {
            roots: vec![0],
            records: vec![CemValueRecord {
                datatype: datatype.into(),
                lexical: lexical.into(),
                ..Default::default()
            }],
        };
        assert!(
            graph.validate(&CemValueArtifactLimits::default()).is_err(),
            "{datatype}"
        );
    }
}

#[test]
fn portable_graph_revalidates_attribute_constraints() {
    use cem_ml::schema::document_model::{AttributeModel, AttributeValueContract};
    use cem_ml::value::artifact::{CemValueGraph, CemValueRecord};
    let graph = CemValueGraph {
        roots: vec![0],
        records: vec![
            CemValueRecord {
                kind: "attribute".into(),
                values: vec![1],
                contract: Some(AttributeValueContract {
                    model: AttributeModel {
                        value_type: Some("integer".into()),
                        min_inclusive: Some("1".into()),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                ..Default::default()
            },
            CemValueRecord {
                datatype: "integer".into(),
                lexical: "0".into(),
                ..Default::default()
            },
        ],
    };
    assert!(graph.validate(&CemValueArtifactLimits::default()).is_err());
}

#[test]
fn all_document_formats_round_trip_through_the_same_native_graph() {
    for (format, source) in [
        ("xml", "<r><name>ivysaur</name><id>2</id></r>"),
        ("json", r#"{"name":"ivysaur","id":2}"#),
        ("yaml", "name: ivysaur\nid: 2\n"),
        ("csv", "name,id\nivysaur,2\n"),
    ] {
        let values = eval(
            &format!("data:read({source:?}, {format:?}).root"),
            ItemStream::empty(),
        );
        let before = eval("dom:text(values)", values.clone());
        let limits = CemValueArtifactLimits::default();
        let restored = decode_values(&encode_values(&values, &limits).unwrap(), &limits).unwrap();
        assert_eq!(
            eval("dom:text(values)", restored).items,
            before.items,
            "{format}"
        );
    }
}

#[test]
fn native_artifact_v3_keeps_occurrences_and_reads_legacy_values() {
    use cem_ql::{eval::output::output_nodes, render::*};
    let limits = CemValueArtifactLimits::default();
    let scalar = eval("2", ItemStream::empty());
    let mut legacy = encode_values(&scalar, &limits).unwrap();
    assert_eq!(legacy[4], 3);
    // Records without new fields retain the legacy body layout.
    for version in [1, 2] {
        legacy[4] = version;
        assert_eq!(decode_values(&legacy, &limits).unwrap().items[0].atom(), scalar.items[0].atom());
    }
    let artifact = compile_template(
        r#"{$data:read("<r/>", "xml").root.children}"#,
        &CompileTemplateOptions::default(),
    );
    let values = output_nodes(render_compiled_template(&artifact, &TemplateData::default()).nodes);
    let bytes = encode_values(&values, &limits).unwrap();
    let restored = decode_values(&bytes, &limits).unwrap();
    assert_eq!(
        restored.items[0]
            .view()
            .unwrap()
            .field("occurrence")
            .unwrap()[0]
            .atom(),
        Some(cem_ql::eval::AtomValue::Boolean(true))
    );
    let mut downgraded = bytes;
    downgraded[4] = 2;
    assert!(decode_values(&downgraded, &limits).is_ok());
    downgraded[4] = 1;
    assert!(decode_values(&downgraded, &limits).is_err());
}

#[test]
fn portable_source_attributes_bind_lexical_values_without_losing_explicit_empty_sequences() {
    use cem_ql::render::*;
    let values = eval(
        r#"data:read("<r count=\"2\"/>", "xml").root.children.attributes"#,
        ItemStream::empty(),
    );
    let limits = CemValueArtifactLimits::default();
    let restored = decode_values(&encode_values(&values, &limits).unwrap(), &limits).unwrap();
    let mut data = TemplateData::default();
    data.bind_native_attribute(restored.items[0].clone())
        .unwrap();
    let result = render_template(
        "{attribute @name=count @type=integer @required=true}{p | {$count + 1}}",
        &data,
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.rendered, "<p>3</p>");
    let attribute = RenderPlanAttribute {
        name: "count".into(),
        namespace: None,
        qualified_name: None,
        value: "stale projection".into(),
        value_stream: ItemStream::empty(),
        contract: None,
        source_map: Default::default(),
    };
    let native = cem_ql::eval::output::output_attribute(attribute);
    let restored = decode_values(
        &encode_values(&ItemStream::once(native.clone()), &limits).unwrap(),
        &limits,
    )
    .unwrap();
    for attribute in [native, restored.items[0].clone()] {
        data.bind_native_attribute(attribute).unwrap();
        assert!(data.bindings["count"].items.is_empty());
    }
}
