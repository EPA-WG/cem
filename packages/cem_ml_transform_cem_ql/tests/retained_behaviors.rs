use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::{builder::CemAstBuilder, document::CemDocument, CemAstNode},
    schema::{
        declaration_references::SchemaDeclarationNode,
        document_model::{
            compile_schema_document_model, SchemaBehaviorEvaluator, SchemaDocumentModel,
        },
        input_references::{RetainedValidationStructure, StructuralValidationNode},
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use cem_ml_transform_cem_ql::CemQlSchemaBehaviorEvaluator;
use std::sync::Arc;
fn parse(text: &str) -> Arc<CemDocument> {
    let doc = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), text.as_bytes().to_vec()),
    )))
    .build();
    assert!(doc.diagnostics.is_empty(), "{:?}", doc.diagnostics);
    Arc::new(doc)
}
fn element(doc: &Arc<CemDocument>, name: &str) -> SchemaDeclarationNode {
    let id = doc
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => Some(*node_id),
            _ => None,
        })
        .unwrap();
    SchemaDeclarationNode::new(doc.clone(), id).unwrap()
}
fn make_model(param: &str, select: &str, query: &str, detail: &str) -> SchemaDocumentModel {
    let text = format!(
        r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema
{{schema @name=placement @namespace="https://example.test/placement" @version="1.0.0" |
 {{elements | {{element @name=left @children=item}}{{element @name=right @children=item}}{{element @name=item @optional-attributes=kind}}}}
 {{attributes | {{attribute @name=kind @type="schema:string"}}}}
 {{behaviors | {{behavior @name=check @implementation=function @execution=ast-validation @function=check-result @select='{select}' @match='{query}' |
 {{inputs | {{input-binding @name=candidate @type="schema:node" @source=candidate @required=true}}}}
 {{result @type="schema:diagnostic-result"}}
 {{function @name=check-result @returns=object @deterministic=true |
 {{param @name=candidate @type={param} @required=true}}
 {{body | {{$ {{message: "placement", details: {{observed: {detail}}}}} }} }}
 }} }} }}
 {{diagnostics | {{diagnostic @code="example.placement" @severity=warning @behavior=check}}}}
}}"#
    );
    let model = compile_schema_document_model("https://example.test/placement", &text);
    assert!(
        model.compile_diagnostics.is_empty(),
        "{:?}",
        model.compile_diagnostics
    );
    model
}
fn snapshot() -> (Arc<CemDocument>, Vec<StructuralValidationNode>) {
    let source = parse("{left | {#items}}{right | {#items}}");
    let library = parse("{item @kind=page}");
    let node = |source, children| StructuralValidationNode {
        source,
        children,
        declaring_schema: None,
        children_complete: true,
    };
    (
        source.clone(),
        vec![
            node(element(&source, "left"), vec![2]),
            node(element(&source, "right"), vec![3]),
            node(element(&library, "item"), vec![]),
            node(element(&library, "item"), vec![]),
        ],
    )
}
fn view<'a>(
    source: &'a Arc<CemDocument>,
    nodes: &'a [StructuralValidationNode],
    roots: &'a [usize],
    complete: bool,
) -> RetainedValidationStructure<'a> {
    RetainedValidationStructure {
        source,
        nodes,
        roots,
        complete,
    }
}
#[test]
fn native_behavior_checks_each_placement_and_extracts_consumed_parent() {
    let (source, nodes) = snapshot();
    let model=make_model("node","item","true","{parent: $candidate.parent.name, name: $candidate.name, kind: $candidate.attributes.value}");
    let report = CemQlSchemaBehaviorEvaluator
        .validate_retained_structure(view(&source, &nodes, &[0, 1], true), &model);
    assert!(report.complete);
    assert_eq!(report.diagnostics.len(), 2, "{:?}", report.diagnostics);
    for (diag, parent) in report.diagnostics.iter().zip(["left", "right"]) {
        assert_eq!(diag.code, "example.placement");
        let details = diag.details.as_ref().unwrap();
        assert_eq!(details["observed"]["parent"], parent);
        assert_eq!(details["observed"]["name"], "item");
        assert_eq!(details["observed"]["kind"], "page");
    }
    let model = make_model(
        "schema:node",
        "item",
        "candidate.parent.name == \"left\"",
        "$candidate.name",
    );
    let report = CemQlSchemaBehaviorEvaluator
        .validate_retained_structure(view(&source, &nodes, &[0, 1], true), &model);
    assert!(report.complete);
    assert_eq!(report.diagnostics.len(), 1, "{:?}", report.diagnostics);
    assert!(source
        .nodes
        .iter()
        .filter_map(|n| if let CemAstNode::Reference { targets, .. } = n {
            Some(targets)
        } else {
            None
        })
        .all(|t| t.is_none()));
}
#[test]
fn retained_object_signature_fails_while_legacy_object_stays_compatible() {
    let (source, nodes) = snapshot();
    let model = make_model("object", "item", "true", "$candidate.name");
    let report = CemQlSchemaBehaviorEvaluator
        .validate_retained_structure(view(&source, &nodes, &[0, 1], true), &model);
    assert!(report.complete);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema_behavior.function_failed"),
        "{:?}",
        report.diagnostics
    );
    let legacy =
        CemQlSchemaBehaviorEvaluator.validate_document(&parse("{item @kind=page}"), &model);
    assert_eq!(legacy.len(), 1, "{:?}", legacy);
    assert_eq!(legacy[0].code, "example.placement");
}
#[test]
fn diagnostic_output_cannot_implicitly_copy_native_candidate() {
    let (source, nodes) = snapshot();
    let model = make_model("node", "item", "true", "$candidate");
    let report = CemQlSchemaBehaviorEvaluator
        .validate_retained_structure(view(&source, &nodes, &[0, 1], true), &model);
    assert!(report.complete);
    assert_eq!(report.diagnostics.len(), 2);
    assert!(report
        .diagnostics
        .iter()
        .all(|d| d.code == "cem.schema_behavior.function_failed"));
}
#[test]
fn pending_and_empty_stages_and_invalid_selection_remain_distinct() {
    let (source, nodes) = snapshot();
    let model = make_model("node", "item", "true", "$candidate.name");
    assert!(
        !CemQlSchemaBehaviorEvaluator
            .validate_retained_structure(view(&source, &[], &[], false), &model)
            .complete
    );
    let empty = CemQlSchemaBehaviorEvaluator
        .validate_retained_structure(view(&source, &[], &[], true), &model);
    assert!(empty.complete && empty.diagnostics.is_empty());
    let model = make_model("node", "1", "true", "$candidate.name");
    let bad = CemQlSchemaBehaviorEvaluator
        .validate_retained_structure(view(&source, &nodes, &[0, 1], true), &model);
    assert!(bad.complete);
    assert!(
        bad.diagnostics
            .iter()
            .any(|d| d.code == "cem.schema_behavior.result_invalid"),
        "{:?}",
        bad.diagnostics
    );
}

#[test]
fn real_host_retained_behavior_obeys_readiness_grants_and_original_owners() {
    use cem_ml::{
        parser::tree::{CemTreeSemantics, RetainedCemTree},
        schema::reference_policy::ReferenceScopePolicy,
    };
    use cem_ql::{
        api::{StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::{ItemStream, RetainedCemNode},
        schema_references::CemQlSchemaDeclarationHost,
    };
    let retained = |text: &str| {
        RetainedCemTree::new(
            Arc::try_unwrap(parse(text)).unwrap(),
            "fixture.cem",
            text,
            CemTreeSemantics::default(),
            None,
        )
        .unwrap()
    };
    let source = retained("{left | {#items}}{right | {#items}}");
    let first = retained("{item @kind=first}");
    let second = retained("{item @kind=second}");
    let first_node = element(first.ast_owner(), "item");
    let second_node = element(second.ast_owner(), "item");
    assert_eq!(first_node.node_id(), second_node.node_id());
    let original_maps = [first_node.node(), second_node.node()].map(|node| {
        let CemAstNode::Element { source, .. } = node else {
            panic!()
        };
        source.clone()
    });
    let targets = vec![
        RetainedCemNode::new(first.clone(), first_node.node_id())
            .unwrap()
            .query_item(),
        RetainedCemNode::new(second.clone(), second_node.node_id())
            .unwrap()
            .query_item(),
    ];
    let context = |items| {
        StandaloneExpressionContext::default().with_binding(
            "items",
            StandaloneExpressionBinding::any(ItemStream::from_items(items)),
        )
    };
    let policy = ReferenceScopePolicy::schema_defaults().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let from = host.register_scope(source.clone(), Some(context(targets)), policy.clone());
    let to_first = host.register_scope(first, None, policy.clone());
    let to_second = host.register_scope(second, None, policy.clone());
    let model = make_model(
        "node",
        "item",
        "true",
        "{parent: $candidate.parent.name, kind: $candidate.attributes.value}",
    );
    let evaluate = |host: &mut CemQlSchemaDeclarationHost| {
        host.validate_input_with_behavior_evaluator(
            source.clone(),
            &model,
            policy.limits,
            Some(&CemQlSchemaBehaviorEvaluator),
        )
        .unwrap()
    };
    let denied = evaluate(&mut host);
    assert!(!denied.complete);
    assert!(!denied
        .diagnostics
        .iter()
        .any(|d| d.code == "example.placement"));
    assert!(host.allow_scope_crossing(from, to_first));
    assert!(host.allow_scope_crossing(from, to_second));
    let selected = evaluate(&mut host);
    assert!(selected.complete, "{:?}", selected.diagnostics);
    let emitted: Vec<_> = selected
        .diagnostics
        .iter()
        .filter(|d| d.code == "example.placement")
        .collect();
    assert_eq!(emitted.len(), 4, "{:?}", selected.diagnostics);
    for ((diag, parent), kind) in emitted
        .iter()
        .zip(["left", "left", "right", "right"])
        .zip(["first", "second", "first", "second"])
    {
        let details = diag.details.as_ref().unwrap();
        assert_eq!(details["observed"]["parent"], parent);
        assert_eq!(details["observed"]["kind"], kind);
        assert_eq!(
            diag.source_map.as_ref().unwrap(),
            &original_maps[usize::from(kind == "second")]
        );
    }
    assert!(host.set_context(from, Some(context(vec![]))));
    let empty = evaluate(&mut host);
    assert!(empty.complete && !empty.failed, "{:?}", empty.diagnostics);
    assert!(!empty
        .diagnostics
        .iter()
        .any(|d| d.code == "example.placement"));
    assert!(host.set_context(from, None));
    assert!(!evaluate(&mut host).complete);
    assert!(source
        .ast_owner()
        .nodes
        .iter()
        .filter_map(|n| if let CemAstNode::Reference { targets, .. } = n {
            Some(targets)
        } else {
            None
        })
        .all(|targets| targets.is_none()));
}

#[test]
fn signature_validation_and_attribute_selection_do_not_depend_on_candidates() {
    let (source, nodes) = snapshot();
    let model = make_model("object", "item", "true", "$candidate.name");
    let empty = CemQlSchemaBehaviorEvaluator
        .validate_retained_structure(view(&source, &[], &[], true), &model);
    assert!(empty.complete);
    assert!(empty
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.schema_behavior.function_failed"));
    let model = make_model("node", "item.attributes", "true", "$candidate.name");
    let attributes = CemQlSchemaBehaviorEvaluator
        .validate_retained_structure(view(&source, &nodes, &[0, 1], true), &model);
    assert!(attributes.complete);
    assert!(attributes
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.schema_behavior.result_invalid"));
}

#[derive(Debug)]
struct EngineStage {
    library: Arc<cem_ml::parser::tree::RetainedCemTree>,
    library_text: String,
    calls: std::sync::atomic::AtomicUsize,
    sources: std::sync::Mutex<Vec<Arc<cem_ml::parser::tree::RetainedCemTree>>>,
}
impl cem_ml::schema::input_validation::InputValidationStage for EngineStage {
    fn validate(
        &self,
        request: cem_ml::schema::input_validation::InputValidationRequest<'_>,
    ) -> Result<
        cem_ml::schema::input_validation::InputValidationOutcome,
        Vec<cem_ml::diagnostics::Diagnostic>,
    > {
        use cem_ql::{
            api::{StandaloneExpressionBinding, StandaloneExpressionContext},
            eval::{ItemStream, RetainedCemNode},
            schema_references::CemQlSchemaDeclarationHost,
        };
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.sources.lock().unwrap().push(request.source.clone());
        let target = element(self.library.ast_owner(), "item");
        let context = StandaloneExpressionContext::default().with_binding(
            "items",
            StandaloneExpressionBinding::any(ItemStream::once(
                RetainedCemNode::new(self.library.clone(), target.node_id())
                    .unwrap()
                    .query_item(),
            )),
        );
        let mut host = CemQlSchemaDeclarationHost::new();
        let from = host.register_scope(
            request.source.clone(),
            Some(context),
            request.policy.clone(),
        );
        let to = host.register_scope(self.library.clone(), None, request.policy.clone());
        host.allow_scope_crossing(from, to);
        if request.source.source_uri() == "pending.cem" {
            host.set_context(from, None);
        }
        let mut report = host
            .validate_input_with_behavior_evaluator(
                request.source.clone(),
                request.model,
                request.policy.limits,
                request.behavior_evaluator,
            )
            .map_err(|error| {
                vec![cem_ml::diagnostics::Diagnostic {
                    code: "fixture.stage".into(),
                    severity: cem_ml::diagnostics::Severity::Error,
                    message: format!("{error:?}"),
                    ..Default::default()
                }]
            })?;
        // The runtime owns each arena's provenance. Project explicit report
        // diagnostics at that owner before the engine aggregates the results.
        for diagnostic in &mut report.diagnostics {
            if let Some(index) = diagnostic
                .details
                .as_ref()
                .and_then(|v| v.get("placement"))
                .and_then(|v| v.as_u64())
            {
                let source = &report.nodes[index as usize].source;
                if Arc::ptr_eq(source.document(), self.library.ast_owner()) {
                    cem_ml::diagnostics::project_diagnostics_for_source(
                        std::slice::from_mut(diagnostic),
                        self.library_text.as_bytes(),
                    );
                    diagnostic.uri = Some(self.library.source_uri().to_owned());
                }
            }
        }
        assert!(request
            .source
            .ast_owner()
            .nodes
            .iter()
            .filter_map(|node| if let CemAstNode::Reference { targets, .. } = node {
                Some(targets)
            } else {
                None
            })
            .all(|targets| targets.is_none()));
        Ok(cem_ml::schema::input_validation::InputValidationOutcome {
            complete: report.complete,
            diagnostics: report.diagnostics,
        })
    }
}
#[test]
fn engine_runtime_stage_executes_native_ql_behaviors_with_original_attribution() {
    use cem_ml::{
        engine::{
            CemMlEngine, EngineContext, EngineInput, FailLevel, InputFormat, ValidateProjection,
            ValidateRequest,
        },
        parser::tree::{CemTreeSemantics, RetainedCemTree},
        real::RealCemMlEngine,
        schema::registry::CEM_ML_SCHEMA_URI,
    };
    let library_text = "preface\n\n{item @kind=library}";
    let library = RetainedCemTree::new(
        Arc::try_unwrap(parse(library_text)).unwrap(),
        "https://vendor.test/library.cem",
        library_text,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    let stage = Arc::new(EngineStage {
        library,
        library_text: library_text.into(),
        calls: Default::default(),
        sources: Default::default(),
    });
    let mut context = EngineContext::default();
    context.schema = Some(CEM_ML_SCHEMA_URI.into());
    let mut model = make_model(
        "node",
        "item",
        "true",
        "{parent: $candidate.parent.name, kind: $candidate.attributes.value}",
    );
    model.schema_uri = CEM_ML_SCHEMA_URI.into();
    context.schema_document_models.register(model);
    context.input_validation_stage = Some(stage.clone());
    context.schema_behavior_evaluator = Some(Arc::new(CemQlSchemaBehaviorEvaluator));
    let source = r#"@ns p = "https://example.test/placement"
@default p
{left | {#items}}{right | {#items}}"#;
    let input = |uri: &str| EngineInput {
        uri: uri.into(),
        bytes: source.as_bytes().to_vec(),
        from_format: Some(InputFormat::Cem),
        identity: None,
        root_scope: Default::default(),
    };
    let response = RealCemMlEngine
        .validate(ValidateRequest {
            inputs: vec![input("complete.cem"), input("pending.cem")],
            projection: ValidateProjection::Cem,
            fail_level: FailLevel::Validate,
            context,
        })
        .unwrap();
    let completion = &response
        .report
        .report_ast
        .validation
        .as_ref()
        .unwrap()
        .inputs;
    assert_eq!(
        completion.iter().map(|c| c.complete).collect::<Vec<_>>(),
        vec![true, false]
    );
    let emitted: Vec<_> = response
        .report
        .diagnostics
        .iter()
        .filter(|d| d.code == "example.placement")
        .collect();
    assert_eq!(emitted.len(), 2, "{:?}", response.report.diagnostics);
    for (diagnostic, parent) in emitted.iter().zip(["left", "right"]) {
        assert_eq!(
            diagnostic.uri.as_deref(),
            Some("https://vendor.test/library.cem")
        );
        assert_eq!(diagnostic.line, Some(3), "{diagnostic:#?}");
        assert_eq!(
            diagnostic.details.as_ref().unwrap()["observed"]["parent"],
            parent
        );
    }
    assert_eq!(stage.calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(stage.sources.lock().unwrap().len(), 2);
}
