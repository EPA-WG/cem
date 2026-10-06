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
        attribute_values: vec![],
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

fn native_attribute_behavior_model() -> SchemaDocumentModel {
    let model = compile_schema_document_model(
        "https://example.test/native-attribute",
        r#"{schema |
      {elements | {element @name=item @optional-attributes=kind}}
      {attributes | {attribute @name=kind @type=schema:node}}
      {behaviors | {behavior @name=check @implementation=function @execution=ast-validation @function=check-result @select=item @match='kind.name == "item"' |
        {inputs | {input-binding @name=candidate @type=schema:node @source=candidate @required=true}}
        {result @type=schema:diagnostic-result}
        {function @name=check-result @returns=object @deterministic=true |
          {param @name=candidate @type=node @required=true}
          {body | {$ {message: "native", details: {observed: $candidate.attributes.value.name}} }}
        }
      }}
      {diagnostics | {diagnostic @code=example.native @severity=warning @behavior=check}}
    }"#,
    );
    assert!(
        model.compile_diagnostics.is_empty(),
        "{:?}",
        model.compile_diagnostics
    );
    model
}
#[test]
fn retained_behaviors_read_native_attribute_values_and_conveniences() {
    use cem_ml::parser::tree::RetainedCemTree;
    use cem_ql::{
        api::{StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::{ItemStream, RetainedCemNode},
        schema_references::CemQlSchemaDeclarationHost,
    };
    let source_text = "{item @kind={#items}}";
    let source = RetainedCemTree::new(
        Arc::try_unwrap(parse(source_text)).unwrap(),
        "input.cem",
        source_text,
        cem_ml::parser::tree::CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    let target_text = "{outside | {item | {#authored}}}";
    let target = RetainedCemTree::new(
        Arc::try_unwrap(parse(target_text)).unwrap(),
        "target.cem",
        target_text,
        cem_ml::parser::tree::CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    let selected = element(target.ast_owner(), "item");
    let context = StandaloneExpressionContext::default().with_binding(
        "items",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(target.clone(), selected.node_id())
                .unwrap()
                .query_item(),
        )),
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    let policy = cem_ml::schema::reference_policy::ReferenceScopePolicy::schema_defaults().unwrap();
    let from = host.register_scope(source.clone(), Some(context), policy.clone());
    let to = host.register_scope(target.clone(), None, policy.clone());
    host.allow_scope_crossing(from, to);
    let model = native_attribute_behavior_model();
    let report = host
        .validate_input_with_behavior_evaluator(
            source.clone(),
            &model,
            policy.limits,
            Some(&CemQlSchemaBehaviorEvaluator),
        )
        .unwrap();
    assert!(report.complete, "{:?}", report.diagnostics);
    assert!(!report.failed, "{:?}", report.diagnostics);
    let diagnostics: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == "example.native")
        .collect();
    assert_eq!(diagnostics.len(), 1, "{:?}", report.diagnostics);
    assert_eq!(diagnostics[0].details.as_ref().unwrap()["observed"], "item");
    assert!(Arc::ptr_eq(
        report.nodes[0].attribute_values[0]
            .access
            .node(report.nodes[0].attribute_values[0].access.roots()[0])
            .unwrap()
            .document(),
        target.ast_owner()
    ));
    // Re-referencing a consumed native view preserves its original owner and
    // registered scope; it never becomes a copied record or a synthetic arena.
    let query_tree =
        cem_ql::validation_structure::RetainedValidationQueryTree::new(report.structure()).unwrap();
    let value = query_tree.roots()[0]
        .view()
        .unwrap()
        .field("attributes")
        .unwrap()[0]
        .view()
        .unwrap()
        .field("value")
        .unwrap()
        .remove(0);
    host.set_context(
        from,
        Some(StandaloneExpressionContext::default().with_binding(
            "items",
            StandaloneExpressionBinding::any(ItemStream::once(value)),
        )),
    );
    let reused = host
        .validate_input_with_behavior_evaluator(
            source.clone(),
            &model,
            policy.limits,
            Some(&CemQlSchemaBehaviorEvaluator),
        )
        .unwrap();
    assert!(
        reused.complete && !reused.failed,
        "{:?}",
        reused.diagnostics
    );
    host.set_context(from, None);
    let pending = host
        .validate_input_with_behavior_evaluator(
            source.clone(),
            &model,
            policy.limits,
            Some(&CemQlSchemaBehaviorEvaluator),
        )
        .unwrap();
    assert!(!pending.complete);
    assert!(!pending
        .diagnostics
        .iter()
        .any(|d| d.code == "example.native"));
    assert!(source
        .ast_owner()
        .nodes
        .iter()
        .filter_map(|node| if let CemAstNode::Reference { targets, .. } = node {
            Some(targets)
        } else {
            None
        })
        .all(Option::is_none));
}

#[test]
fn engine_runtime_stage_consumes_native_attribute_nodes_without_expanding_descendants() {
    use cem_ml::{
        engine::{
            CemMlEngine, EngineContext, EngineInput, FailLevel, InputFormat, ValidateProjection,
            ValidateRequest,
        },
        parser::tree::{CemTreeSemantics, RetainedCemTree},
        real::RealCemMlEngine,
        schema::registry::CEM_ML_SCHEMA_URI,
    };
    let library_text = "{item | {#authored}}";
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
    let mut model = native_attribute_behavior_model();
    model.schema_uri = CEM_ML_SCHEMA_URI.into();
    context.schema_document_models.register(model);
    context.input_validation_stage = Some(stage.clone());
    context.schema_behavior_evaluator = Some(Arc::new(CemQlSchemaBehaviorEvaluator));
    let source =
        "@ns p = \"https://example.test/native-attribute\"\n@default p\n{item @kind={#items}}";
    let input = |uri: &str| EngineInput {
        uri: uri.into(),
        bytes: if uri == "general.cem" {
            source.replace("#items", "items").into_bytes()
        } else {
            source.as_bytes().to_vec()
        },
        from_format: Some(InputFormat::Cem),
        identity: None,
        root_scope: Default::default(),
    };
    let response = RealCemMlEngine
        .validate(ValidateRequest {
            inputs: vec![
                input("complete.cem"),
                input("general.cem"),
                input("pending.cem"),
            ],
            projection: ValidateProjection::Cem,
            fail_level: FailLevel::Validate,
            context,
        })
        .unwrap();
    let inputs = &response
        .report
        .report_ast
        .validation
        .as_ref()
        .unwrap()
        .inputs;
    assert_eq!(
        inputs
            .iter()
            .map(|input| input.complete)
            .collect::<Vec<_>>(),
        vec![true, true, false],
        "{:?}",
        response.report.diagnostics
    );
    let emitted: Vec<_> = response
        .report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "example.native")
        .collect();
    assert_eq!(emitted.len(), 2, "{:?}", response.report.diagnostics);
    for (diagnostic, uri) in emitted.iter().zip(["complete.cem", "general.cem"]) {
        assert_eq!(diagnostic.uri.as_deref(), Some(uri));
        assert_eq!(diagnostic.details.as_ref().unwrap()["observed"], "item");
    }
    assert_eq!(stage.calls.load(std::sync::atomic::Ordering::SeqCst), 3);
    assert_eq!(stage.sources.lock().unwrap().len(), 3);
}

#[test]
fn general_attribute_expressions_consume_native_results_and_preserve_failures() {
    use cem_ml::parser::tree::{CemTreeSemantics, RetainedCemTree};
    use cem_ql::{
        api::{StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::{AtomValue, Item, ItemStream, RetainedCemNode},
        schema_references::CemQlSchemaDeclarationHost,
    };
    let target_text = "{outside | {item | {#authored}}}";
    let target = RetainedCemTree::new(
        Arc::try_unwrap(parse(target_text)).unwrap(),
        "target.cem",
        target_text,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    let selected = element(target.ast_owner(), "item");
    let native = RetainedCemNode::new(target.clone(), selected.node_id())
        .unwrap()
        .query_item();
    let authored_text = "{item @kind={unknown}}";
    let authored = RetainedCemTree::new(
        Arc::try_unwrap(parse(authored_text)).unwrap(),
        "authored.cem",
        authored_text,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    let authored_node = element(authored.ast_owner(), "$");
    let native_expression = RetainedCemNode::new(authored.clone(), authored_node.node_id())
        .unwrap()
        .query_item();
    for (expression, values, ready, granted, expected_complete, expected_failed) in [
        ("items", vec![native.clone()], true, true, true, false),
        ("items", vec![native_expression], true, true, true, false),
        ("(#items)", vec![native.clone()], true, true, true, false),
        ("items", vec![], true, true, true, true),
        (
            "items",
            vec![native.clone(), native.clone()],
            true,
            true,
            true,
            true,
        ),
        (
            "items",
            vec![Item::Atomic(AtomValue::String("scalar".into()))],
            true,
            true,
            false,
            true,
        ),
        ("items", vec![native.clone()], false, true, false, false),
        ("1 +", vec![], true, true, false, true),
        ("items", vec![native.clone()], true, false, false, false),
        (
            "items",
            vec![
                native.clone(),
                Item::Atomic(AtomValue::String("scalar".into())),
            ],
            true,
            true,
            false,
            true,
        ),
    ] {
        let source_text = format!("{{item @kind={{{expression}}}}}");
        let source = RetainedCemTree::new(
            Arc::try_unwrap(parse(&source_text)).unwrap(),
            "input.cem",
            &source_text,
            CemTreeSemantics::default(),
            None,
        )
        .unwrap();
        let context = StandaloneExpressionContext::default().with_binding(
            "items",
            StandaloneExpressionBinding::any(ItemStream::from_items(values)),
        );
        let mut host = CemQlSchemaDeclarationHost::new();
        let policy =
            cem_ml::schema::reference_policy::ReferenceScopePolicy::schema_defaults().unwrap();
        let from = host.register_scope(source.clone(), ready.then_some(context), policy.clone());
        let to = host.register_scope(target.clone(), None, policy.clone());
        let authored_scope = host.register_scope(authored.clone(), None, policy.clone());
        if granted {
            host.allow_scope_crossing(from, to);
            host.allow_scope_crossing(from, authored_scope);
        }
        let report = host
            .validate_input_with_behavior_evaluator(
                source.clone(),
                &native_attribute_behavior_model(),
                policy.limits,
                Some(&CemQlSchemaBehaviorEvaluator),
            )
            .unwrap();
        assert_eq!(
            report.complete, expected_complete,
            "{expression}: {:?}",
            report.diagnostics
        );
        assert_eq!(
            report.failed, expected_failed,
            "{expression}: {:?}",
            report.diagnostics
        );
        if expected_complete && !expected_failed {
            let values = &report.nodes[0].attribute_values[0];
            let target_node = values.access.node(values.access.roots()[0]).unwrap();
            let CemAstNode::Element { expanded_name, .. } = target_node.node() else {
                panic!()
            };
            if expanded_name.local_name == "item" {
                assert!(
                    report
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code == "example.native"),
                    "{:?}",
                    report.diagnostics
                );
                assert!(Arc::ptr_eq(target_node.document(), target.ast_owner()));
            } else {
                assert_eq!(expanded_name.local_name, "$");
                assert!(Arc::ptr_eq(target_node.document(), authored.ast_owner()));
                assert!(!report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == "example.native"));
            }
        }
        if !expected_complete {
            assert!(!report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "example.native"));
        }
    }
}

#[test]
fn engine_cem_capture_retains_closed_child_bindings_for_runtime_validation() {
    use cem_ml::{
        engine::{
            CemMlEngine, EngineContext, EngineInput, FailLevel, InputFormat, ValidateProjection,
            ValidateRequest,
        },
        parser::tree::{CemTreeSemantics, RetainedCemTree},
        real::RealCemMlEngine,
        schema::{
            input_validation::{
                InputValidationOutcome, InputValidationRequest, InputValidationStage,
            },
            registry::CEM_ML_SCHEMA_URI,
        },
    };
    use cem_ql::{
        api::{StandaloneExpressionBinding, StandaloneExpressionContext},
        schema_references::CemQlSchemaDeclarationHost,
    };
    #[derive(Debug)]
    struct CapturedStage {
        targets: Arc<RetainedCemTree>,
    }
    impl InputValidationStage for CapturedStage {
        fn validate(
            &self,
            request: InputValidationRequest<'_>,
        ) -> Result<InputValidationOutcome, Vec<cem_ml::diagnostics::Diagnostic>> {
            let captured = request.lexical_scopes.as_ref().unwrap();
            assert!(Arc::ptr_eq(request.source.ast_owner(), captured.document()));
            assert_eq!(captured.occurrences().count(), 3);
            let ready = request.source.source_uri() != "pending.cem";
            let mut host = CemQlSchemaDeclarationHost::new();
            let root = host.register_scope(request.source.clone(), None, request.policy.clone());
            let target_scope =
                host.register_scope(self.targets.clone(), None, request.policy.clone());
            assert!(host.allow_scope_crossing(root, target_scope));
            let mut bindings = Vec::new();
            host.attach_captured_lexical_scopes(captured, |node, snapshot, parent| {
                assert_eq!(parent, root);
                assert!(matches!(
                    node.node(),
                    CemAstNode::Reference { targets: None, .. }
                ));
                assert_eq!(
                    snapshot
                        .namespaces
                        .binding("inherited")
                        .unwrap()
                        .namespace_uri,
                    "urn:inherited"
                );
                let uri = &snapshot.namespaces.binding("v").unwrap().namespace_uri;
                bindings.push(uri.clone());
                let name = match uri.as_str() {
                    "urn:root" => "first",
                    "urn:child" => "second",
                    _ => panic!("{uri}"),
                };
                let target = element(self.targets.ast_owner(), name);
                let context = ready.then(|| {
                    StandaloneExpressionContext::default().with_binding(
                        "items",
                        StandaloneExpressionBinding::any(cem_ql::eval::ItemStream::once(
                            cem_ql::eval::RetainedCemNode::new(
                                self.targets.clone(),
                                target.node_id(),
                            )
                            .unwrap()
                            .query_item(),
                        )),
                    )
                });
                (context, request.policy.clone())
            })
            .unwrap();
            assert_eq!(bindings, ["urn:root", "urn:child", "urn:root"]);
            let report = host
                .validate_input_with_behavior_evaluator(
                    request.source.clone(),
                    request.model,
                    request.policy.limits,
                    None,
                )
                .unwrap();
            assert_eq!(report.complete, ready, "{:?}", report.diagnostics);
            if ready {
                let selected: Vec<_> = report
                    .nodes
                    .iter()
                    .flat_map(|node| &node.attribute_values)
                    .flat_map(|value| {
                        value
                            .access
                            .roots()
                            .iter()
                            .map(|id| value.access.node(*id).unwrap())
                    })
                    .map(|node| match node.node() {
                        CemAstNode::Element { expanded_name, .. } => {
                            expanded_name.local_name.clone()
                        }
                        _ => panic!("expected native target"),
                    })
                    .collect();
                assert_eq!(selected, ["first", "second", "first"]);
            }
            assert!(request.source.ast().nodes.iter().all(|node| !matches!(
                node,
                CemAstNode::Reference {
                    targets: Some(_),
                    ..
                }
            )));
            Ok(InputValidationOutcome {
                complete: report.complete,
                diagnostics: report.diagnostics,
            })
        }
    }
    let target_text = "{first}{second}";
    let targets = RetainedCemTree::new(
        Arc::try_unwrap(parse(target_text)).unwrap(),
        "targets.cem",
        target_text,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    let mut context = EngineContext::default();
    context.schema = Some(CEM_ML_SCHEMA_URI.into());
    let mut model = compile_schema_document_model(
        "https://example.test/native-attribute",
        r#"{schema | {elements | {element @name=item @optional-attributes="kind v" @children=item}} {attributes | {attribute @name=kind @type=schema:node} {attribute @name=v @type=schema:string}}}"#,
    );
    assert!(
        model.is_ready_for_validation(),
        "{:?}",
        model.compile_diagnostics
    );
    model.schema_uri = CEM_ML_SCHEMA_URI.into();
    context.schema_document_models.register(model);
    context.input_validation_stage = Some(Arc::new(CapturedStage { targets }));
    let source = "@ns v = \"urn:root\"\n{item @kind={#items}}\n{item @xmlns:v=urn:child | {item @kind={#items}}}\n{item @kind={#items}}";
    let input = |uri: &str| {
        let mut root_scope = cem_ml::run_config::ScopeConfig::default();
        root_scope.schema = Some(CEM_ML_SCHEMA_URI.into());
        root_scope
            .namespaces
            .insert("inherited".into(), "urn:inherited".into());
        EngineInput {
            uri: uri.into(),
            bytes: source.as_bytes().to_vec(),
            from_format: Some(InputFormat::Cem),
            identity: None,
            root_scope,
        }
    };
    let response = RealCemMlEngine
        .validate(ValidateRequest {
            inputs: vec![input("complete.cem"), input("pending.cem")],
            projection: ValidateProjection::Cem,
            fail_level: FailLevel::Validate,
            context,
        })
        .unwrap();
    let inputs = &response
        .report
        .report_ast
        .validation
        .as_ref()
        .unwrap()
        .inputs;
    assert_eq!(
        inputs
            .iter()
            .map(|input| input.complete)
            .collect::<Vec<_>>(),
        [true, false],
        "{:?}",
        response.report.diagnostics
    );
}

#[test]
fn region_behavior_filters_execution_while_preserving_cross_model_navigation() {
    use cem_ml::schema::input_references::RetainedBehaviorRegion;
    let (source, nodes) = snapshot();
    let model = make_model(
        "node",
        "nodes.children",
        "candidate.parent.name == \"left\"",
        "$candidate.parent.name",
    );
    let result = CemQlSchemaBehaviorEvaluator.validate_retained_region(
        RetainedBehaviorRegion {
            structure: view(&source, &nodes, &[0, 1], true),
            placements: &[0, 3],
        },
        &model,
    );
    assert!(
        result.complete && result.diagnostics.is_empty(),
        "{:?}",
        result.diagnostics
    );
    let result = CemQlSchemaBehaviorEvaluator.validate_retained_region(
        RetainedBehaviorRegion {
            structure: view(&source, &nodes, &[0, 1], true),
            placements: &[0, 2],
        },
        &model,
    );
    assert!(result.complete);
    assert_eq!(result.diagnostics.len(), 1, "{:?}", result.diagnostics);
    assert_eq!(
        result.diagnostics[0].details.as_ref().unwrap()["observed"],
        "left"
    );
    assert_eq!(
        result.diagnostics[0].details.as_ref().unwrap()["placement"],
        2
    );
}

#[test]
fn region_behavior_rejects_invalid_domains_and_keeps_selection_type_errors() {
    use cem_ml::schema::input_references::RetainedBehaviorRegion;
    let (source, nodes) = snapshot();
    let model = make_model("node", "nodes", "true", "$candidate.name");
    for placements in [&[4usize][..], &[2usize, 2][..]] {
        let report = CemQlSchemaBehaviorEvaluator.validate_retained_region(
            RetainedBehaviorRegion {
                structure: view(&source, &nodes, &[0, 1], true),
                placements,
            },
            &model,
        );
        assert!(report.complete);
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(
            report.diagnostics[0].code,
            "cem.schema_behavior.result_invalid"
        );
    }
    let model = make_model("node", "1", "true", "$candidate.name");
    let report = CemQlSchemaBehaviorEvaluator.validate_retained_region(
        RetainedBehaviorRegion {
            structure: view(&source, &nodes, &[0, 1], true),
            placements: &[2],
        },
        &model,
    );
    assert!(report.complete);
    assert_eq!(
        report.diagnostics[0].code,
        "cem.schema_behavior.result_invalid"
    );
}
