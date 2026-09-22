//! No-handler dispatch preserves native values; eligible predicates remain observable.
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    eval::{AtomValue, Item, ItemStream},
    render::{
        compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
        RenderPlanNode, TemplateArtifact, TemplateData,
    },
    template_artifact::{
        compile_template_artifact, TemplateArtifactLoadContext, TemplateArtifactSourceMapMode,
    },
};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

fn artifacts(source: &str, bindings: Vec<String>) -> [TemplateArtifact; 2] {
    let options = CompileTemplateOptions {
        host_bindings: bindings.clone(),
        ..Default::default()
    };
    let direct = compile_template(source, &options);
    assert!(direct.diagnostics.is_empty(), "{:?}", direct.diagnostics);
    let loaded = compile_template_artifact(source, &options, TemplateArtifactSourceMapMode::Dev)
        .reload(&TemplateArtifactLoadContext {
            host_bindings: bindings,
            expected_source_hash: Some(cem_ml::content_cache::ContentHash::from_blake3(
                source.as_bytes(),
            )),
            source_map_mode: TemplateArtifactSourceMapMode::Dev,
        })
        .unwrap();
    [direct, loaded]
}

#[test]
fn no_handler_preserves_native_content_and_attribute_types_after_reload() {
    let node = evaluate(
        &compile(
            r#"data:read("<name>ivy<em>saur</em></name>", "xml").root.children"#,
            &CompileContext::default(),
        )
        .unwrap(),
        &EvaluationContext::default(),
    );
    assert!(node.error.is_none());
    assert_eq!(node.items.len(), 1);
    assert!(node.items[0].identity().is_some());
    let input = TemplateData::default().with_binding("node", node.clone());
    for hooks in [
        "",
        "{template @on=expression @into=content @match=false | {$1 / 0}}",
        "{template @on=expression @into=attribute @match=false | {$1 / 0}}",
    ] {
        let source = format!(
            "{hooks}{}",
            concat!(
                "{p | {$node}}",
                "{child | {attribute @name=count @type=integer @minInclusive=1 | {$\"002\"}}",
                "{attribute @name=mixed @type=any | {$(1, true, node)}}",
                "{attribute @name=empty @type=any | {$()}}}"
            )
        );
        for artifact in artifacts(&source, vec!["node".into()]) {
            let output = render_compiled_template(&artifact, &input);
            assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
            let RenderPlanNode::Element { children, .. } = &output.nodes[0] else {
                panic!()
            };
            let RenderPlanNode::Reference { reference, .. } = &children[0] else {
                panic!()
            };
            assert_eq!(reference.values()[0].identity(), node.items[0].identity());
            let RenderPlanNode::Element { attributes, .. } = &output.nodes[1] else {
                panic!()
            };
            assert_eq!(
                attributes[0].value_stream.items[0].atom(),
                Some(AtomValue::Integer(2))
            );
            let mixed = &attributes[1].value_stream.items;
            assert_eq!(mixed.len(), 3);
            assert_eq!(mixed[0].atom(), Some(AtomValue::Integer(1)));
            assert_eq!(mixed[1].atom(), Some(AtomValue::Boolean(true)));
            assert_eq!(mixed[2].identity(), node.items[0].identity());
            assert!(attributes[2].value_stream.items.is_empty());
            assert!(render_plan_to_html(&output)
                .starts_with("<p><name>ivy<em>saur</em></name></p><child count=\"2\""));
        }
        for artifact in artifacts(
            &source.replace("@minInclusive=1", "@minInclusive=3"),
            vec!["node".into()],
        ) {
            let output = render_compiled_template(&artifact, &input);
            assert!(output.nodes.is_empty(), "typed validation must still run");
            assert!(output
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()));
        }
    }
}

#[derive(Debug)]
struct Predicate {
    calls: Arc<AtomicUsize>,
    raises: bool,
}
impl cem_ql::native::NativeQueryFunction for Predicate {
    fn call(&self, request: cem_ql::native::NativeQueryRequest<'_>) -> ItemStream {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(
            request.arguments[0].items[0].atom(),
            Some(AtomValue::String("kept".into()))
        );
        let Item::Record(fields) = &request.arguments[1].items[0] else {
            panic!()
        };
        assert_eq!(
            fields.keys().map(String::as_str).collect::<Vec<_>>(),
            ["extra", "label"]
        );
        assert!(request.current_item.is_some());
        if self.raises {
            return request.raise("fixture.predicate", "recover me");
        }
        let mut result = ItemStream::once(Item::Atomic(AtomValue::Boolean(false)));
        result.diagnostics.push(cem_ml::diagnostics::Diagnostic {
            code: "fixture.predicate_observed".into(),
            severity: cem_ml::diagnostics::Severity::Warning,
            source_map: Some(request.source_map.clone()),
            ..Default::default()
        });
        result
    }
}

#[test]
fn eligible_false_and_failing_predicates_keep_diagnostics_scope_and_recovery() {
    for (into, body, expected) in [
        ("content", "{p | {$\"kept\"}}", "<p>kept</p>"),
        (
            "attribute",
            "{p @title='{\"kept\"}'}",
            "<p title=\"kept\"></p>",
        ),
    ] {
        for raises in [false, true] {
            let source = format!("{{try | {{template @on=expression @into={into} @match='native:call(\"fixture.match\", value, island)' | BAD}}{body}{{catch @as=e | {{p | caught}}}}}}");
            let calls = Arc::new(AtomicUsize::new(0));
            let mut data = TemplateData::default().with_binding(
                "island",
                ItemStream::once(Item::Record(BTreeMap::from([
                    ("extra".into(), vec![Item::Atomic(AtomValue::Integer(1))]),
                    (
                        "label".into(),
                        vec![Item::Atomic(AtomValue::String("retained".into()))],
                    ),
                ]))),
            );
            data.native_functions
                .register(
                    "fixture.match",
                    2,
                    Predicate {
                        calls: calls.clone(),
                        raises,
                    },
                )
                .unwrap();
            for artifact in artifacts(&source, vec!["island".into()]) {
                let output = render_compiled_template(&artifact, &data);
                assert_eq!(
                    render_plan_to_html(&output),
                    if raises { "<p>caught</p>" } else { expected }
                );
                assert!(
                    !output
                        .diagnostics
                        .iter()
                        .any(|d| d.severity.is_hard_violation()),
                    "{:?}",
                    output.diagnostics
                );
                assert_eq!(
                    output
                        .diagnostics
                        .iter()
                        .filter(|d| d.code == "fixture.predicate_observed")
                        .count(),
                    usize::from(!raises)
                );
            }
            assert_eq!(
                calls.load(Ordering::SeqCst),
                2,
                "one eligible predicate per render"
            );
        }
    }
}
