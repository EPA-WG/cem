//! Opt-in attribution and a restoring test-only public projection candidate.
use super::*;
use crate::conversion::writer_profile_tests::{measure, Span};
use crate::conversion::{
    writer_profile_tests::{pipeline, profile, write_pipeline},
    ConversionOutputPipelineEnvironment, ConversionOutputPipelineExecution, ConversionRegistry,
};
use crate::{import::import_data, parser::tree::RetainedCemTree, schema::registry::SchemaRegistry};
use std::cell::Cell;
use std::{hint::black_box, time::Instant};

thread_local! {
    static SINGLE_PASS: Cell<bool> = const { Cell::new(false) };
}
fn candidate<T>(run: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            SINGLE_PASS.with(|flag| flag.set(self.0));
        }
    }
    let _reset = Reset(SINGLE_PASS.with(|flag| flag.replace(true)));
    run()
}

pub(super) fn project_sequence(
    sequence: &CemtEvaluatorSequenceRef<'_>,
) -> Option<Result<serde_json::Value, String>> {
    if !SINGLE_PASS.with(Cell::get) {
        return None;
    }
    let CemtEvaluatorSequenceRef::FormattedNodes {
        nodes,
        parent,
        overlay,
    } = sequence
    else {
        return None;
    };
    Some((|| {
        let _profile = Span::new("projection/candidate-sequence-json");
        // Emit directly into the required public array. Borrow native nodes and
        // overlay operations; retain their original owner paths and indices.
        let mut values = Vec::new();
        for before_node in 0..=nodes.len() {
            for (index, operation) in overlay.node_operations.iter().enumerate() {
                if cemt_evaluator_gap_operation_matches(operation, parent.as_ref(), before_node) {
                    values.push(
                        CemtEvaluatorValue::borrowed(CemtEvaluatorValueRef::Record(
                            CemtEvaluatorRecordRef::NodeFormatOperation { operation, index },
                        ))
                        .to_public_json()?,
                    );
                }
            }
            if let Some(node) = nodes.get(before_node) {
                let path = match parent {
                    Some(parent) => parent.child(before_node),
                    None => CemtOwnerPath::root(before_node),
                };
                if overlay.retains_node(&path) {
                    values.push(
                        CemtEvaluatorValue::borrowed(CemtEvaluatorValueRef::Record(
                            CemtEvaluatorRecordRef::FormattedNode {
                                node,
                                path,
                                overlay,
                            },
                        ))
                        .to_public_json()?,
                    );
                }
            }
        }
        Ok(serde_json::Value::Array(values))
    })())
}

#[test]
fn single_pass_candidate_avoids_indexed_formatted_reads() {
    let artifact = fixture(8);
    let (expected, before) = measure(|| artifact.to_public_json().unwrap());
    let (actual, after) = measure(|| candidate(|| artifact.to_public_json().unwrap()));
    assert_eq!(actual, expected);
    assert!(before["projection/formatted-sequence-item"].calls >= 8);
    assert!(!after.contains_key("projection/formatted-sequence-item"));
    assert!(!after.contains_key("projection/formatted-sequence-len"));
    assert_eq!(artifact.to_public_json().unwrap(), expected);
}

fn fixture(count: usize) -> CemtTreeArtifact {
    let owner = Arc::new(CemTreeAstStream::new(
        (0..count)
            .map(|i| CemTreeAstNode::Text {
                value: format!("value-{i}"),
                source: SourceMapStack::default(),
            })
            .collect(),
    ));
    CemtTreeArtifact::formatted(
        owner,
        None,
        CemtFormattedTreeOverlay {
            envelope: CemtTreeEnvelopeMetadata {
                content_type: "application/cem".into(),
                schema: "https://cem.dev/ns/cem-ml/1".into(),
                category: "cem-tree".into(),
                mode: CemtTreeEnvelopeMode::Document,
                canonical: true,
            },
            producer: CemtOverlayProducer {
                function_name: "fixture".into(),
                formatter_profile: Some("tabular".into()),
            },
            operations: vec![],
            node_operations: vec![],
            retained_node_paths: (0..count).map(CemtOwnerPath::root).collect(),
        },
    )
}

fn fragment(target: CemtNodeFormatTarget, value: &str) -> CemtNodeFormatOperation {
    CemtNodeFormatOperation {
        target,
        producer_function: "fixture".into(),
        formatter_role: "fixture.gap".into(),
        color_role: Some("syntax.raw".into()),
        kind: CemtNodeFormatOperationKind::InsertChild {
            fragment: CemtFormatFragment::Whitespace {
                value: value.into(),
            },
        },
        provenance: CemtOverlayProvenance::Generated {
            function_name: "fixture".into(),
            source_map: None,
        },
    }
}

#[test]
fn single_pass_preserves_gap_order_removed_nodes_and_empty_sequences() {
    for count in [0, 1, 4] {
        let mut artifact = fixture(count);
        let overlay = artifact.formatted_overlay.as_mut().unwrap();
        // Hide alternating owner nodes; gaps still use original owner positions.
        overlay
            .retained_node_paths
            .retain(|path| path.root % 2 == 0);
        for gap in 0..=count {
            // Ordinals deliberately disagree with operation order. The public
            // view preserves vector order at each gap, without re-sorting it.
            for ordinal in [7, 2] {
                overlay.node_operations.push(fragment(
                    CemtNodeFormatTarget::RootGap {
                        before_root: gap,
                        ordinal,
                    },
                    &format!("gap-{gap}-{ordinal}"),
                ));
            }
        }
        overlay.node_operations.push(fragment(
            CemtNodeFormatTarget::RootGap {
                before_root: count + 1,
                ordinal: 0,
            },
            "out-of-range",
        ));
        let expected = artifact.to_public_json().unwrap();
        let actual = candidate(|| artifact.to_public_json().unwrap());
        assert_eq!(actual, expected);
        let nodes = actual["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 2 * (count + 1) + count.div_ceil(2));
        assert_eq!(nodes[0]["value"], "gap-0-7");
        assert_eq!(nodes[1]["value"], "gap-0-2");
    }
    let empty = fixture(0);
    assert_eq!(
        candidate(|| empty.to_public_json()).unwrap()["nodes"],
        serde_json::Value::Array(vec![])
    );
}

#[test]
fn single_pass_preserves_nested_and_colored_projections_and_errors() {
    let source = SourceMapStack {
        frames: vec![SourceMapFrame {
            source_id: SourceId(91),
            span: FrameSpan::Single(ByteRange::new(2, 7)),
            transform: TransformKind::TemplateTransform {
                function: "projection-fixture".into(),
            },
        }],
    };
    let owner = Arc::new(CemTreeAstStream::new(vec![CemTreeAstNode::Element {
        name: "outer".into(),
        attributes: vec![CemTreeAstAttribute {
            name: "id".into(),
            value: Some("one".into()),
            source: source.clone(),
        }],
        children: vec![CemTreeAstNode::Element {
            name: "empty".into(),
            attributes: vec![],
            children: vec![],
            source: source.clone(),
        }],
        source: source.clone(),
    }]));
    let mut overlay = fixture(0).formatted_overlay.unwrap();
    let root = CemtOwnerPath::root(0);
    overlay.retained_node_paths = vec![root.clone(), root.child(0)];
    overlay.node_operations = vec![
        fragment(
            CemtNodeFormatTarget::ChildGap {
                parent: root.clone(),
                before_child: 1,
                ordinal: 0,
            },
            "after",
        ),
        fragment(
            CemtNodeFormatTarget::ChildGap {
                parent: root.clone(),
                before_child: 0,
                ordinal: 9,
            },
            "before",
        ),
        fragment(
            CemtNodeFormatTarget::ChildGap {
                parent: root.child(0),
                before_child: 0,
                ordinal: 0,
            },
            "empty-child-gap",
        ),
    ];
    let colors = CemtColoredTreeOverlay {
        producer: CemtColorOverlayProducer {
            function_name: "fixture.color".into(),
            color_profile: Some("html".into()),
        },
        operations: vec![],
        writer_boundaries: vec![],
        node_operations: vec![CemtNodeColorOperation {
            target: CemtColorTarget::Owner(root.clone()),
            producer_function: "fixture.color".into(),
            kind: CemtNodeColorOperationKind::Wrapper {
                name: "mark".into(),
                colorizer_role: "fixture.color".into(),
                color_profile: "html".into(),
                color_role: None,
                style: None,
            },
            provenance: CemtOverlayProvenance::SourceMapped(source.clone()),
        }],
    };
    for artifact in [
        CemtTreeArtifact::raw(owner.clone(), Some(source.clone())),
        CemtTreeArtifact::formatted(owner.clone(), Some(source.clone()), overlay.clone()),
        CemtTreeArtifact::colored(owner.clone(), Some(source.clone()), overlay, colors.clone()),
    ] {
        let expected = artifact.to_public_json().unwrap();
        let actual = candidate(|| artifact.to_public_json().unwrap());
        assert_eq!(actual, expected);
        assert_eq!(
            rmp_serde::to_vec_named(&actual).unwrap(),
            rmp_serde::to_vec_named(&expected).unwrap()
        );
        assert_eq!(artifact.source_map(), Some(&source));
        assert!(Arc::ptr_eq(artifact.owner(), &owner));
    }
    let mut malformed = CemtTreeArtifact::raw(owner.clone(), Some(source));
    malformed.colored_overlay = Some(colors);
    let error = malformed.to_public_json().unwrap_err();
    assert_eq!(candidate(|| malformed.to_public_json()).unwrap_err(), error);
    drop(malformed);
    let weak = Arc::downgrade(&owner);
    drop(owner);
    assert!(weak.upgrade().is_none());
}

#[test]
fn single_pass_falls_back_for_sparse_package_sequences_and_restores_scope() {
    #[derive(Debug)]
    struct Sparse;
    impl CemtEvaluatorSequenceView for Sparse {
        fn len(&self) -> usize {
            2
        }
        fn item(&self, index: usize) -> Option<CemtEvaluatorValueRef<'_>> {
            (index == 0).then_some(CemtEvaluatorValueRef::Null)
        }
    }
    let value = CemtEvaluatorValue::borrowed(CemtEvaluatorValueRef::Sequence(
        CemtEvaluatorSequenceRef::Package { sequence: &Sparse },
    ));
    let expected = value.to_public_json().unwrap_err();
    assert_eq!(expected, "typed evaluator sequence item 1 is unavailable");
    assert_eq!(candidate(|| value.to_public_json()).unwrap_err(), expected);
    let panic = std::panic::catch_unwind(|| {
        candidate(|| {
            candidate(|| assert!(SINGLE_PASS.with(Cell::get)));
            assert!(SINGLE_PASS.with(Cell::get));
            panic!("scope restoration fixture");
        })
    });
    assert!(panic.is_err());
    assert!(!SINGLE_PASS.with(Cell::get));
}

fn assert_execution(
    actual: &ConversionOutputPipelineExecution,
    expected: &ConversionOutputPipelineExecution,
    owner: &Arc<RetainedCemTree>,
) {
    assert!(actual.diagnostics.is_empty(), "{:?}", actual.diagnostics);
    assert_eq!(actual.diagnostics, expected.diagnostics);
    assert_eq!(actual.output, expected.output);
    assert_eq!(actual.source_map, expected.source_map);
    assert_eq!(actual.output_spans, expected.output_spans);
    assert_eq!(actual.format_execution, expected.format_execution);
    assert_eq!(actual.color_execution, expected.color_execution);
    for (actual, expected) in [
        (&actual.formatted_cem_tree, &expected.formatted_cem_tree),
        (&actual.colored_cem_tree, &expected.colored_cem_tree),
    ] {
        // Serialize only the existing explicit API sidecar, never an AST handoff.
        assert_eq!(
            rmp_serde::to_vec_named(actual).unwrap(),
            rmp_serde::to_vec_named(expected).unwrap()
        );
    }
    for artifact in [
        &actual.raw_cem_tree,
        &actual.formatted_cemt_tree,
        &actual.colored_cemt_tree,
    ]
    .into_iter()
    .flatten()
    {
        assert!(Arc::ptr_eq(artifact.owner().source_owner().unwrap(), owner));
    }
}

#[test]
fn single_pass_preserves_four_imports_pipeline_sidecars_and_document_release() {
    let schemas = SchemaRegistry::with_builtin_schemas();
    let conversions = ConversionRegistry::with_builtin_converters();
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &schemas,
        conversion_registry: &conversions,
        package_artifact_reader: None,
        artifact_cache: None,
    };
    for (format, source) in [
        (
            "xml",
            "<?keep inert?><r xmlns:p='urn:p' p:id='1'><![CDATA[<raw>🍒]]><!--note--><a/></r>",
        ),
        (
            "json",
            r#"{"fruit":"<raw>🍒","empty":"","items":[1,true,null]}"#,
        ),
        (
            "yaml",
            "fruit: '<raw>🍒'\nempty: ''\nitems: [1, true, null]",
        ),
        ("csv", "fruit,empty\n<raw>🍒,\npear,two\n"),
    ] {
        let owner = import_data(
            source,
            format,
            "cem",
            &format!("memory:projection.{format}"),
        )
        .unwrap();
        for profile in ["compact", "pretty", "tabular"] {
            for color in [None, Some("terminal")] {
                let mut pipeline = pipeline();
                pipeline.cemt_options.formatter_profile = Some(profile.into());
                pipeline.cemt_insertion_context.formatter_profile = Some(profile.into());
                pipeline.writer_insertion_context.formatter_profile = Some(profile.into());
                pipeline.cemt_options.color_profile = color.map(str::to_owned);
                pipeline.cemt_insertion_context.color_profile = color.map(str::to_owned);
                pipeline.writer_insertion_context.color_profile = color.map(str::to_owned);
                let expected = write_pipeline(&environment, &owner, &pipeline);
                assert!(
                    expected.diagnostics.is_empty(),
                    "{:?}",
                    expected.diagnostics
                );
                let actual = candidate(|| write_pipeline(&environment, &owner, &pipeline));
                assert_execution(&actual, &expected, &owner);
                assert_eq!(actual.colored_cemt_tree.is_some(), color.is_some());
            }
        }
        let weak = Arc::downgrade(&owner);
        drop(owner);
        assert!(weak.upgrade().is_none());
    }
}

// Attribution control only: follow the same indexed field/sequence access, but
// do not construct an export value or serialize source maps. The difference
// from full projection includes all output allocation and leaf conversion; it
// is not an exact subtraction of independently sampled medians.
fn walk(value: CemtEvaluatorValue<'_>) -> usize {
    let mut count = 1;
    match value.kind() {
        CemtEvaluatorValueKind::Record => {
            for name in value.record_field_names("projection walk").unwrap() {
                count += walk(value.field(&name).unwrap());
            }
        }
        CemtEvaluatorValueKind::Sequence => {
            for index in 0..value.length().unwrap() {
                count += walk(value.item(index).unwrap());
            }
        }
        _ => {
            black_box(&value);
        }
    }
    count
}

fn sample<T>(name: &str, mut run: impl FnMut() -> T, check: impl Fn(&T)) {
    let mut recorded = Vec::new();
    for _ in 0..6 {
        let (value, stages) = measure(|| {
            let _span = Span::new("total");
            black_box(run())
        });
        check(&value);
        recorded.push(stages);
    }
    for label in recorded[0].keys() {
        let mut times = recorded[1..]
            .iter()
            .map(|stages| stages[label].elapsed.as_secs_f64() * 1000.0)
            .collect::<Vec<_>>();
        times.sort_by(f64::total_cmp);
        println!(
            "{name}\t{label}\tcalls={}\tmedian_ms={:.3}",
            recorded[1][label].calls, times[2]
        );
    }
    let mut times = Vec::new();
    for _ in 0..6 {
        let start = Instant::now();
        let value = black_box(run());
        times.push(start.elapsed().as_secs_f64() * 1000.0);
        check(&value);
    }
    times.remove(0);
    times.sort_by(f64::total_cmp);
    println!(
        "{name}\tunprofiled_median_ms={:.3}\tmin_ms={:.3}\tmax_ms={:.3}",
        times[2], times[0], times[4]
    );
}

#[test]
#[ignore = "profiling fixture: --release --lib profile_public_projection -- --ignored --nocapture --test-threads=1"]
fn profile_public_projection() {
    let schemas = SchemaRegistry::with_builtin_schemas();
    let conversions = ConversionRegistry::with_builtin_converters();
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &schemas,
        conversion_registry: &conversions,
        package_artifact_reader: None,
        artifact_cache: None,
    };
    let mut sources = vec![(
        "authored".to_owned(),
        include_str!("../../../cem-elements/demo/tree-source.xml").to_owned(),
    )];
    sources.extend([8, 32, 64].map(|rows| {
        (
            format!("{rows}-rows"),
            format!("<r>{}</r>", "<row id='1'>value</row>".repeat(rows)),
        )
    }));
    for (name, source) in sources {
        let owner = import_data(&source, "xml", "cem", "memory:projection-profile").unwrap();
        let pipeline = pipeline();
        let expected_execution = write_pipeline(&environment, &owner, &pipeline);
        assert!(
            expected_execution.diagnostics.is_empty(),
            "{:?}",
            expected_execution.diagnostics
        );
        let artifact = expected_execution.formatted_cemt_tree.as_ref().unwrap();
        let expected = artifact.to_public_json().unwrap();
        let count = walk(CemtEvaluatorValue::borrowed(artifact.evaluator_view()));
        println!("{name}\tvalue_visits={count}\toverlay_operations={}\tretained_paths={}\tpublic_bytes={}", artifact.formatted_overlay().unwrap().node_operations.len(), artifact.formatted_overlay().unwrap().retained_node_paths().len(), rmp_serde::to_vec_named(&expected).unwrap().len());
        sample(
            &format!("{name}/indexed-projection"),
            || artifact.to_public_json().unwrap(),
            |actual| assert_eq!(actual, &expected),
        );
        sample(
            &format!("{name}/single-pass-projection"),
            || candidate(|| artifact.to_public_json().unwrap()),
            |actual| assert_eq!(actual, &expected),
        );
        sample(
            &format!("{name}/walk-only"),
            || walk(CemtEvaluatorValue::borrowed(artifact.evaluator_view())),
            |actual| assert_eq!(*actual, count),
        );
        sample(
            &format!("{name}/source-map-clone"),
            || artifact.source_map().cloned(),
            |actual| assert_eq!(actual.as_ref(), artifact.source_map()),
        );
        if name == "authored" || name == "32-rows" {
            profile(
                &format!("{name}/indexed-pipeline"),
                || write_pipeline(&environment, &owner, &pipeline),
                &expected_execution,
                &owner,
            );
            profile(
                &format!("{name}/single-pass-pipeline"),
                || candidate(|| write_pipeline(&environment, &owner, &pipeline)),
                &expected_execution,
                &owner,
            );
        }
        drop(expected_execution);
        let weak = Arc::downgrade(&owner);
        drop(owner);
        assert!(weak.upgrade().is_none());
    }
}
