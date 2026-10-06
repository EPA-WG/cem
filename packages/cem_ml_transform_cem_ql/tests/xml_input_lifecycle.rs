use cem_ml::{
    engine::{
        CemMlEngine, EngineContext, EngineInput, FailLevel, InputFormat, ValidateProjection,
        ValidateRequest,
    },
    parser::{
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode,
    },
    real::RealCemMlEngine,
    schema::{
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        document_model::compile_schema_document_model,
        input_validation::{InputValidationOutcome, InputValidationRequest, InputValidationStage},
        registry::{CEM_ML_SCHEMA_URI, XML_SCHEMA_URI},
    },
    validation::xml::XmlDocumentAst,
    value::reference_resolution::{resolve_reference, ReferenceResolutionIssueKind},
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    schema_references::CemQlSchemaDeclarationHost,
};
use std::sync::{Arc, Mutex};
const XML: &str = "<root xmlns:r='https://cem.dev/ns/cem-ml/1' xmlns:v='urn:outer'><r:expr>#lib&#114;<![CDATA[ary]]></r:expr><section xmlns:v='urn:inner'><r:expr>#library</r:expr></section><r:expr>#library</r:expr><target value='{#library}'/></root>";
#[derive(Debug)]
struct Stage {
    target: Arc<RetainedCemTree>,
    calls: Mutex<Vec<String>>,
}
impl InputValidationStage for Stage {
    fn validate(
        &self,
        request: InputValidationRequest<'_>,
    ) -> Result<InputValidationOutcome, Vec<cem_ml::diagnostics::Diagnostic>> {
        let captured = request.lexical_scopes.as_ref().unwrap();
        assert!(Arc::ptr_eq(request.source.ast_owner(), captured.document()));
        let native = request
            .source
            .native_owner()
            .unwrap()
            .downcast_ref::<XmlDocumentAst>()
            .unwrap();
        assert_eq!(native.source.uri, request.source.source_uri());
        assert!(!native.events.is_empty());
        self.calls
            .lock()
            .unwrap()
            .push(request.source.source_uri().into());
        let refs: Vec<_> = captured.occurrences().collect();
        assert_eq!(refs.len(), 3);
        let first_source = match request.source.ast().get(refs[0]).unwrap() {
            CemAstNode::Reference { source, .. } => source,
            _ => unreachable!(),
        };
        assert!(
            first_source
                .frames
                .iter()
                .filter(|frame| matches!(
                    frame.transform,
                    cem_ml::source_map::TransformKind::ExpressionEmbedding { .. }
                ))
                .count()
                >= 3
        );
        assert!(request.source.ast().nodes.iter().any(
            |node| matches!(node, CemAstNode::Attribute { value, .. } if value.as_deref() == Some("{#library}"))
        ));
        let target = self
            .target
            .ast()
            .nodes
            .iter()
            .find_map(|node| match node {
                CemAstNode::Element { node_id, .. } => Some(*node_id),
                _ => None,
            })
            .unwrap();
        let mut host = CemQlSchemaDeclarationHost::new();
        let root = host.register_scope(request.source.clone(), None, request.policy.clone());
        let destination = host.register_scope(self.target.clone(), None, request.policy.clone());
        let ready = request.source.source_uri() != "pending.xml";
        let granted = request.source.source_uri() != "denied.xml";
        if granted {
            assert!(host.allow_scope_crossing(root, destination));
        }
        let mut bindings = Vec::new();
        host.attach_captured_lexical_scopes(captured, |node, snapshot, parent| {
            assert_eq!(parent, root);
            assert_eq!(snapshot.namespaces.binding("r").unwrap().namespace_uri, CEM_ML_SCHEMA_URI);
            bindings.push(snapshot.namespaces.binding("v").unwrap().namespace_uri.clone());
            assert!(matches!(node.node(), CemAstNode::Reference { expression, targets: None, .. } if expression == "#library"));
            assert!(matches!(node.node(), CemAstNode::Reference { source, .. } if !source.frames.is_empty()));
            let context = ready.then(|| StandaloneExpressionContext::default().with_binding("library", StandaloneExpressionBinding::any(ItemStream::once(RetainedCemNode::new(self.target.clone(), target).unwrap().query_item()))));
            (context, request.policy.clone())
        }).unwrap();
        assert_eq!(bindings, ["urn:outer", "urn:inner", "urn:outer"]);
        let mut complete = true;
        for node in refs {
            let original = SchemaDeclarationNode::new(captured.document().clone(), node).unwrap();
            let input = host.source_reference(original);
            let result = resolve_reference(input, &mut host, request.policy.limits).unwrap();
            assert_eq!(result.is_complete(), ready && granted);
            if ready && !granted {
                assert!(result
                    .issues
                    .iter()
                    .any(|issue| issue.kind == ReferenceResolutionIssueKind::ScopeDenied));
            }
            if result.is_complete() {
                assert!(Arc::ptr_eq(
                    host.declaration_node(&result.nodes[0]).unwrap().document(),
                    self.target.ast_owner()
                ));
            }
            complete &= result.is_complete();
        }
        assert!(request.source.ast().nodes.iter().all(|node| !matches!(
            node,
            CemAstNode::Reference {
                targets: Some(_),
                ..
            }
        )));
        Ok(InputValidationOutcome {
            complete,
            diagnostics: vec![],
        })
    }
}
#[test]
fn xml_input_stage_reuses_native_owner_and_preserves_aliases_literals_and_pending_grants() {
    use cem_ml::{
        events::cem::CemEventNormalizer,
        parser::builder::CemAstBuilder,
        source::{BytesSource, SourceId},
        tokenizer::cem::CemTokenizer,
    };
    let text = "{target}";
    let ast = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), text.as_bytes().to_vec()),
    )))
    .build();
    let stage = Arc::new(Stage {
        target: RetainedCemTree::new(ast, "library.cem", text, CemTreeSemantics::default(), None)
            .unwrap(),
        calls: Mutex::new(vec![]),
    });
    // Both the custom-schema passthrough and already-parsed generic XML adapter
    // must supply the same specialized import and explicit lifecycle contract.
    for schema in [CEM_ML_SCHEMA_URI, XML_SCHEMA_URI] {
        let mut context = EngineContext::default();
        context.schema = Some(schema.into());
        context.content_type = Some("application/xml".into());
        let mut model = compile_schema_document_model(
            "urn:test",
            "{schema | {elements | {element @name=root}}}",
        );
        model.schema_uri = schema.into();
        assert!(model.is_ready_for_validation());
        context.schema_document_models.register(model);
        context.input_validation_stage = Some(stage.clone());
        let input = |uri: &str| EngineInput {
            uri: uri.into(),
            bytes: XML.as_bytes().to_vec(),
            from_format: Some(InputFormat::Xml),
            identity: None,
            root_scope: Default::default(),
        };
        let response = RealCemMlEngine
            .validate(ValidateRequest {
                inputs: vec![
                    input("ready.xml"),
                    input("pending.xml"),
                    input("denied.xml"),
                ],
                projection: ValidateProjection::Cem,
                fail_level: FailLevel::Validate,
                context,
            })
            .unwrap();
        assert!(
            !response
                .report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity.is_hard_violation()),
            "{:?}",
            response.report.diagnostics
        );
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
            [true, false, false]
        );
    }
    assert_eq!(stage.calls.lock().unwrap().len(), 6);
}

#[test]
fn xml_runtime_failure_preserves_native_diagnostics_and_foreign_attribution() {
    #[derive(Debug)]
    struct FailedStage;
    impl InputValidationStage for FailedStage {
        fn validate(
            &self,
            request: InputValidationRequest<'_>,
        ) -> Result<InputValidationOutcome, Vec<cem_ml::diagnostics::Diagnostic>> {
            assert!(request
                .source
                .native_owner()
                .unwrap()
                .is::<XmlDocumentAst>());
            Err(vec![cem_ml::diagnostics::Diagnostic {
                code: "fixture.xml_runtime.warning".into(),
                severity: cem_ml::diagnostics::Severity::Warning,
                uri: Some("vendor.xml".into()),
                line: Some(7),
                column: Some(9),
                message: "runtime preparation is unavailable".into(),
                ..Default::default()
            }])
        }
    }
    let mut context = EngineContext::default();
    context.schema = Some(XML_SCHEMA_URI.into());
    context.content_type = Some("application/xml".into());
    let mut model =
        compile_schema_document_model("urn:test", "{schema | {elements | {element @name=root}}}");
    model.schema_uri = XML_SCHEMA_URI.into();
    context.schema_document_models.register(model);
    context.input_validation_stage = Some(Arc::new(FailedStage));
    let input = EngineInput {
        uri: "broken.xml".into(),
        bytes: b"<root><target value='a' value='b'/></root>".to_vec(),
        from_format: Some(InputFormat::Xml),
        identity: None,
        root_scope: Default::default(),
    };
    let response = RealCemMlEngine
        .validate(ValidateRequest {
            inputs: vec![
                input,
                EngineInput {
                    uri: "ready.xml".into(),
                    bytes: b"<root/>".to_vec(),
                    from_format: Some(InputFormat::Xml),
                    identity: None,
                    root_scope: Default::default(),
                },
            ],
            projection: ValidateProjection::Cem,
            fail_level: FailLevel::Validate,
            context,
        })
        .unwrap();
    let diagnostics = &response.report.diagnostics;
    assert!(
        diagnostics.iter().any(
            |diagnostic| diagnostic.code == "cem.xml.duplicate_attribute"
                && diagnostic.uri.as_deref() == Some("broken.xml")
        ),
        "{diagnostics:?}"
    );
    let foreign = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "fixture.xml_runtime.warning")
        .unwrap();
    assert_eq!(foreign.uri.as_deref(), Some("vendor.xml"));
    assert_eq!((foreign.line, foreign.column), (Some(7), Some(9)));
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code
        == cem_ml::schema::input_validation::RUNTIME_INPUT_VALIDATION_FAILED
        && diagnostic.severity.is_hard_violation()));
    assert!(
        !response
            .report
            .report_ast
            .validation
            .as_ref()
            .unwrap()
            .inputs[0]
            .complete
    );
}
