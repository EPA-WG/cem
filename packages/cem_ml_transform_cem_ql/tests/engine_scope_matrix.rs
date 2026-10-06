use cem_ml::{
    diagnostics::Diagnostic,
    engine::{
        CemMlEngine, EngineContext, EngineInput, FailLevel, InputFormat, ValidateProjection,
        ValidateRequest,
    },
    events::cem::CemEventNormalizer,
    parser::{
        builder::CemAstBuilder,
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode,
    },
    real::RealCemMlEngine,
    schema::{
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        document_model::compile_schema_document_model,
        input_validation::{InputValidationOutcome, InputValidationRequest, InputValidationStage},
        registry::CEM_ML_SCHEMA_URI,
        scoping::SchemaSource,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
    value::reference_resolution::{
        resolve_reference, ReferenceResolutionIssueKind, ReferenceResolutionState,
    },
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    schema_references::CemQlSchemaDeclarationHost,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

fn tree(text: &str, uri: &str) -> Arc<RetainedCemTree> {
    let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), text.as_bytes().to_vec()),
    )))
    .build();
    RetainedCemTree::new(document, uri, text, CemTreeSemantics::default(), None).unwrap()
}
fn element(tree: &RetainedCemTree, name: &str) -> u32 {
    tree.ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => Some(*node_id),
            _ => None,
        })
        .unwrap()
}
fn context(tree: &Arc<RetainedCemTree>, node: u32) -> StandaloneExpressionContext {
    StandaloneExpressionContext::default().with_binding(
        "library",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(tree.clone(), node)
                .unwrap()
                .query_item(),
        )),
    )
}
fn source(request: &InputValidationRequest<'_>, node: u32) -> SchemaDeclarationNode {
    SchemaDeclarationNode::new(request.source.ast_owner().clone(), node).unwrap()
}
fn assert_authored(request: &InputValidationRequest<'_>) {
    assert!(request.source.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}
fn run(stage: Arc<dyn InputValidationStage>, format: InputFormat, text: &str) {
    let mut context = EngineContext::default();
    context.schema = Some(CEM_ML_SCHEMA_URI.into());
    context.content_type = Some(
        if format == InputFormat::Xml {
            "application/xml"
        } else {
            "text/cem-ml"
        }
        .into(),
    );
    let mut model = compile_schema_document_model(
        "urn:matrix",
        "{schema | {elements | {element @name=section}}}",
    );
    model.schema_uri = CEM_ML_SCHEMA_URI.into();
    assert!(model.is_ready_for_validation());
    context.schema_document_models.register(model);
    context.input_validation_stage = Some(stage);
    let response = RealCemMlEngine
        .validate(ValidateRequest {
            inputs: vec![EngineInput {
                uri: "matrix.input".into(),
                bytes: text.as_bytes().to_vec(),
                from_format: Some(format),
                identity: None,
                root_scope: Default::default(),
            }],
            projection: ValidateProjection::Cem,
            fail_level: FailLevel::Validate,
            context,
        })
        .unwrap();
    assert!(
        response
            .report
            .report_ast
            .validation
            .as_ref()
            .unwrap()
            .inputs[0]
            .complete,
        "{:?}",
        response.report.diagnostics
    );
}

#[derive(Debug)]
struct MatrixStage {
    first: Arc<RetainedCemTree>,
    second: Arc<RetainedCemTree>,
    calls: AtomicUsize,
}
impl InputValidationStage for MatrixStage {
    fn validate(
        &self,
        request: InputValidationRequest<'_>,
    ) -> Result<InputValidationOutcome, Vec<Diagnostic>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let captured = request.lexical_scopes.as_ref().unwrap();
        assert!(Arc::ptr_eq(captured.document(), request.source.ast_owner()));
        let refs: Vec<_> = captured.occurrences().collect();
        assert_eq!(refs.len(), 6);
        let expected_schema = [
            SchemaSource::Uri("schema://host".into()),
            SchemaSource::Select("missing()".into()),
            SchemaSource::Uri("schema://host".into()),
            SchemaSource::Uri("schema://sibling".into()),
            SchemaSource::Uri("schema://outer".into()),
            SchemaSource::Uri("schema://outer".into()),
        ];
        let expected_ns = [
            "urn:first",
            "urn:inner",
            "urn:first",
            "urn:first",
            "urn:first",
            "urn:second",
        ];
        for (i, node) in refs.iter().enumerate() {
            let snapshot = captured
                .snapshot(request.source.ast_owner(), *node)
                .unwrap();
            assert_eq!(snapshot.schema.active, expected_schema[i]);
            assert_eq!(
                snapshot.namespaces.binding("v").unwrap().namespace_uri,
                expected_ns[i]
            );
        }
        let mut host = CemQlSchemaDeclarationHost::new();
        let parent = host.register_scope(request.source.clone(), None, request.policy.clone());
        let first_scope = host.register_scope(self.first.clone(), None, request.policy.clone());
        let second_scope = host.register_scope(self.second.clone(), None, request.policy.clone());
        let attached = host.attach_captured_lexical_scopes(captured, |node, snapshot, inherited| {
            assert_eq!(inherited, parent);
            assert!(matches!(node.node(), CemAstNode::Reference {expression, targets: None, ..} if expression == "#library"));
            let target = if snapshot.namespaces.binding("v").unwrap().namespace_uri == "urn:second" { &self.second } else { &self.first };
            let ready = !matches!(snapshot.schema.active, SchemaSource::Select(_));
            (ready.then(|| context(target, element(target, "target"))), request.policy.clone())
        }).unwrap();
        // Capture/preparation has not compiled or evaluated any reference yet.
        for node in &refs {
            assert!(host
                .compiled_source_expression(&source(&request, *node))
                .is_none());
        }
        let input = host.source_reference(source(&request, refs[0]));
        let denied = resolve_reference(input.clone(), &mut host, request.policy.limits).unwrap();
        assert!(denied
            .issues
            .iter()
            .any(|issue| issue.kind == ReferenceResolutionIssueKind::ScopeDenied));
        assert!(host.allow_scope_crossing(parent, first_scope));
        assert!(host.allow_scope_crossing(parent, second_scope));
        for i in [0, 2, 3, 4, 5] {
            let result = resolve_reference(
                host.source_reference(source(&request, refs[i])),
                &mut host,
                request.policy.limits,
            )
            .unwrap();
            assert!(result.is_complete(), "{:?}", result.issues);
            let expected = if i == 5 { &self.second } else { &self.first };
            assert!(Arc::ptr_eq(
                host.declaration_node(&result.nodes[0]).unwrap().document(),
                expected.ast_owner()
            ));
        }
        let first_artifact = host
            .compiled_source_expression(&source(&request, refs[0]))
            .unwrap();
        let last_artifact = host
            .compiled_source_expression(&source(&request, refs[5]))
            .unwrap();
        assert!(!Arc::ptr_eq(&first_artifact, &last_artifact));
        let pending = host.source_reference(source(&request, refs[1]));
        assert_eq!(
            resolve_reference(pending.clone(), &mut host, request.policy.limits)
                .unwrap()
                .state,
            ReferenceResolutionState::Pending
        );
        assert!(host
            .compiled_source_expression(&source(&request, refs[1]))
            .is_none());
        // Runtime completion supplies a new environment; it does not interpret
        // the selector at parse time or replace the saved source-position meaning.
        assert!(host.set_context(
            attached[1].1,
            Some(context(&self.second, element(&self.second, "target")))
        ));
        let completed = resolve_reference(pending, &mut host, request.policy.limits).unwrap();
        assert!(completed.is_complete());
        assert!(Arc::ptr_eq(
            host.declaration_node(&completed.nodes[0])
                .unwrap()
                .document(),
            self.second.ast_owner()
        ));
        assert!(Arc::ptr_eq(
            &first_artifact,
            &host
                .compiled_source_expression(&source(&request, refs[0]))
                .unwrap()
        ));
        // An independent lifecycle can use other runtime inputs with the same
        // immutable captured owner without taking over this host's selection.
        let mut other = CemQlSchemaDeclarationHost::new();
        let from = other.register_scope(request.source.clone(), None, request.policy.clone());
        let to = other.register_scope(self.second.clone(), None, request.policy.clone());
        assert!(other.allow_scope_crossing(from, to));
        other
            .attach_captured_lexical_scopes(captured, |_, _, _| {
                (
                    Some(context(&self.second, element(&self.second, "target"))),
                    request.policy.clone(),
                )
            })
            .unwrap();
        let result = resolve_reference(
            other.source_reference(source(&request, refs[0])),
            &mut other,
            request.policy.limits,
        )
        .unwrap();
        assert!(result.is_complete());
        assert!(Arc::ptr_eq(
            other.declaration_node(&result.nodes[0]).unwrap().document(),
            self.second.ast_owner()
        ));
        let original = resolve_reference(input, &mut host, request.policy.limits).unwrap();
        assert!(Arc::ptr_eq(
            host.declaration_node(&original.nodes[0])
                .unwrap()
                .document(),
            self.first.ast_owner()
        ));
        assert_authored(&request);
        Ok(InputValidationOutcome {
            complete: true,
            diagnostics: vec![],
        })
    }
}
#[test]
fn engine_scope_forms_preserve_saved_bindings_and_complete_only_at_runtime() {
    let stage = Arc::new(MatrixStage {
        first: tree("{target}", "first.cem"),
        second: tree("{target}", "second.cem"),
        calls: AtomicUsize::new(0),
    });
    run(stage.clone(), InputFormat::Cem, "@ns v = urn:first\n@schema src=schema://outer\n{section @cem:schema-src=schema://host | {#library} {cem:schema @select='missing()' @xmlns:v=urn:inner | {#library}} {#library} {cem:schema @src=schema://sibling} {#library}} {#library}\n@ns v = urn:second\n{#library}");
    run(stage.clone(), InputFormat::Xml, "<root xmlns:r='https://cem.dev/ns/cem-ml/1' xmlns:v='urn:first' r:schema-src='schema://outer'><section r:schema-src='schema://host'><r:expr>#library</r:expr><r:schema select='missing()' xmlns:v='urn:inner'><r:expr>#library</r:expr></r:schema><r:expr>#library</r:expr><r:schema src='schema://sibling'/><r:expr>#library</r:expr></section><r:expr>#library</r:expr><r:expr xmlns:v='urn:second'>#library</r:expr></root>");
    assert_eq!(stage.calls.load(Ordering::SeqCst), 2);
}

#[derive(Debug)]
struct BudgetStage {
    vendor: Arc<RetainedCemTree>,
    calls: AtomicUsize,
}
impl InputValidationStage for BudgetStage {
    fn validate(
        &self,
        request: InputValidationRequest<'_>,
    ) -> Result<InputValidationOutcome, Vec<Diagnostic>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let captured = request.lexical_scopes.as_ref().unwrap();
        let refs: Vec<_> = captured.occurrences().collect();
        assert_eq!(refs.len(), 2);
        assert_eq!(
            captured
                .snapshot(request.source.ast_owner(), refs[0])
                .unwrap()
                .schema
                .active,
            SchemaSource::Uri("schema://outer".into())
        );
        assert_eq!(
            captured
                .snapshot(request.source.ast_owner(), refs[1])
                .unwrap()
                .schema
                .active,
            SchemaSource::Uri("schema://inner".into())
        );
        let strict = compile_schema_document_model(
            "urn:strict",
            "{schema | {constraints | {constraint @kind=reference-traversal-work @value=1}}}",
        );
        let strict_policy = request.policy.for_scope(&strict).unwrap();
        assert_eq!(strict_policy.limits.max_work, 1);
        for (destination_limited, request_work, request_depth, issue) in [
            (
                true,
                request.policy.limits.max_work,
                request.policy.limits.max_depth,
                Some(ReferenceResolutionIssueKind::WorkLimit),
            ),
            (
                false,
                2,
                request.policy.limits.max_depth,
                Some(ReferenceResolutionIssueKind::WorkLimit),
            ),
            (
                false,
                request.policy.limits.max_work,
                1,
                Some(ReferenceResolutionIssueKind::DepthLimit),
            ),
            (
                false,
                request.policy.limits.max_work,
                request.policy.limits.max_depth,
                None,
            ),
        ] {
            let mut host = CemQlSchemaDeclarationHost::new();
            let from = host.register_scope(request.source.clone(), None, request.policy.clone());
            let to = host.register_scope(self.vendor.clone(), None, request.policy.clone());
            host.attach_captured_lexical_scopes(captured, |node, snapshot, inherited| {
                assert_eq!(inherited, from);
                if node.node_id() == refs[0] {
                    (
                        Some(context(&request.source, refs[1])),
                        request.policy.clone(),
                    )
                } else {
                    assert_eq!(
                        snapshot.schema.active,
                        SchemaSource::Uri("schema://inner".into())
                    );
                    (
                        Some(context(&self.vendor, element(&self.vendor, "target"))),
                        if destination_limited {
                            strict_policy.clone()
                        } else {
                            request.policy.clone()
                        },
                    )
                }
            })
            .unwrap();
            let mut limits = request.policy.limits;
            limits.max_work = request_work;
            limits.max_depth = request_depth;
            let input = host.source_reference(source(&request, refs[0]));
            if issue.is_none() {
                let denied = resolve_reference(input.clone(), &mut host, limits).unwrap();
                assert!(denied
                    .issues
                    .iter()
                    .any(|issue| issue.kind == ReferenceResolutionIssueKind::ScopeDenied));
            }
            // One relationship grant covers both source lexical snapshots; it
            // does not reset depth/work when the chain enters the inner scope.
            assert!(host.allow_scope_crossing(from, to));
            let result = resolve_reference(input, &mut host, limits).unwrap();
            assert!(result.work_used <= request_work);
            if let Some(expected) = issue {
                assert!(!result.is_complete());
                assert!(
                    result.issues.iter().any(|issue| issue.kind == expected),
                    "{:?}",
                    result.issues
                );
                assert!(!result
                    .issues
                    .iter()
                    .any(|issue| issue.kind == ReferenceResolutionIssueKind::ScopeDenied));
            } else {
                assert!(result.is_complete(), "{:?}", result.issues);
                assert!(Arc::ptr_eq(
                    host.declaration_node(&result.nodes[0]).unwrap().document(),
                    self.vendor.ast_owner()
                ));
            }
        }
        assert_authored(&request);
        Ok(InputValidationOutcome {
            complete: true,
            diagnostics: vec![],
        })
    }
}
#[test]
fn engine_scope_chains_keep_request_and_destination_limits_and_explicit_vendor_grants() {
    let stage = Arc::new(BudgetStage {
        vendor: tree("{target}", "vendor.cem"),
        calls: AtomicUsize::new(0),
    });
    run(stage.clone(), InputFormat::Cem, "@schema src=schema://outer\n{section | {#library} {section @cem:schema-src=schema://inner | {#library}}}");
    run(stage.clone(), InputFormat::Xml, "<root xmlns:r='https://cem.dev/ns/cem-ml/1' r:schema-src='schema://outer'><r:expr>#library</r:expr><section r:schema-src='schema://inner'><r:expr>#library</r:expr></section></root>");
    assert_eq!(stage.calls.load(Ordering::SeqCst), 2);
}

#[derive(Debug)]
struct DefaultsStage {
    first_vendor: Arc<RetainedCemTree>,
    second_vendor: Arc<RetainedCemTree>,
    calls: AtomicUsize,
}
impl InputValidationStage for DefaultsStage {
    fn validate(
        &self,
        request: InputValidationRequest<'_>,
    ) -> Result<InputValidationOutcome, Vec<Diagnostic>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let captured = request.lexical_scopes.as_ref().unwrap();
        assert!(Arc::ptr_eq(captured.document(), request.source.ast_owner()));
        let refs: Vec<_> = captured.occurrences().collect();
        assert_eq!(refs.len(), 5);
        let snapshots: Vec<_> = refs
            .iter()
            .map(|node| {
                captured
                    .snapshot(request.source.ast_owner(), *node)
                    .unwrap()
            })
            .collect();
        let expected_namespace = [
            "urn:outer",
            "urn:outer",
            "urn:inner",
            "urn:inner",
            "urn:outer",
        ];
        for (i, snapshot) in snapshots.iter().enumerate() {
            assert_eq!(
                snapshot.namespaces.binding("").unwrap().namespace_uri,
                expected_namespace[i]
            );
            assert_eq!(
                snapshot.namespaces.binding("v").unwrap().namespace_uri,
                expected_namespace[i]
            );
            assert_eq!(
                snapshot.schema.active,
                SchemaSource::Uri(
                    if i == 2 || i == 3 {
                        "schema://inner"
                    } else {
                        "schema://outer"
                    }
                    .into()
                )
            );
        }
        // A declaration becomes visible only after closure. A child inherits
        // the outer declaration until its own declaration closes, then shadows
        // it locally; the next parent occurrence still sees the outer one.
        assert!(snapshots[0].schema.resolve_name("shared").is_none());
        let outer = captured
            .inline_schema(request.source.ast_owner(), refs[1], "shared")
            .unwrap();
        let inner = captured
            .inline_schema(request.source.ast_owner(), refs[3], "shared")
            .unwrap();
        assert_ne!(outer.identity(), inner.identity());
        for i in [2, 4] {
            let inherited = captured
                .inline_schema(request.source.ast_owner(), refs[i], "shared")
                .unwrap();
            assert_eq!(inherited.identity(), outer.identity());
            assert!(Arc::ptr_eq(inherited.document(), request.source.ast_owner()));
        }
        assert!(captured
            .inline_schema(request.source.ast_owner(), refs[0], "shared")
            .is_none());
        for (handle, marker) in [(&outer, "outer"), (&inner, "inner")] {
            let CemAstNode::Element { children, .. } = handle.node() else {
                panic!("native schema declaration")
            };
            assert!(children.iter().any(|node| matches!(handle.document().get(*node), Some(CemAstNode::Element { expanded_name, .. }) if expanded_name.local_name == marker)));
        }
        // Both published vendor parts intentionally have the same authored ID
        // and arena index. Owner identity and explicit grants distinguish them.
        let first = element(&self.first_vendor, "target");
        let second = element(&self.second_vendor, "target");
        assert_eq!(first, second);
        assert!(!Arc::ptr_eq(
            self.first_vendor.ast_owner(),
            self.second_vendor.ast_owner()
        ));
        for vendor in [&self.first_vendor, &self.second_vendor] {
            assert!(vendor.ast().nodes.iter().any(|node| matches!(node, CemAstNode::Attribute { expanded_name, value, .. } if expanded_name.local_name == "id" && value.as_deref() == Some("shared"))));
        }
        assert!(!request.source.ast().nodes.iter().any(|node| matches!(node, CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name == "id")));
        let mut host = CemQlSchemaDeclarationHost::new();
        let root = host.register_scope(request.source.clone(), None, request.policy.clone());
        let first_scope =
            host.register_scope(self.first_vendor.clone(), None, request.policy.clone());
        let second_scope =
            host.register_scope(self.second_vendor.clone(), None, request.policy.clone());
        host.attach_captured_lexical_scopes(captured, |occurrence, _, inherited| {
            assert_eq!(inherited, root);
            let inner_selected = captured
                .inline_schema(occurrence.document(), occurrence.node_id(), "shared")
                .is_some_and(|declaration| declaration.identity() == inner.identity());
            let vendor = if inner_selected {
                &self.second_vendor
            } else {
                &self.first_vendor
            };
            (
                Some(context(vendor, element(vendor, "target"))),
                request.policy.clone(),
            )
        })
        .unwrap();
        assert!(host.allow_scope_crossing(root, first_scope));
        for i in [0, 1, 2, 4] {
            let result = resolve_reference(
                host.source_reference(source(&request, refs[i])),
                &mut host,
                request.policy.limits,
            )
            .unwrap();
            assert!(result.is_complete(), "{:?}", result.issues);
            assert!(Arc::ptr_eq(
                host.declaration_node(&result.nodes[0]).unwrap().document(),
                self.first_vendor.ast_owner()
            ));
        }
        let inner_reference = host.source_reference(source(&request, refs[3]));
        let denied =
            resolve_reference(inner_reference.clone(), &mut host, request.policy.limits).unwrap();
        assert!(!denied.is_complete());
        assert!(denied
            .issues
            .iter()
            .any(|issue| issue.kind == ReferenceResolutionIssueKind::ScopeDenied));
        assert!(host.allow_scope_crossing(root, second_scope));
        let result = resolve_reference(inner_reference, &mut host, request.policy.limits).unwrap();
        assert!(result.is_complete());
        assert!(Arc::ptr_eq(
            host.declaration_node(&result.nodes[0]).unwrap().document(),
            self.second_vendor.ast_owner()
        ));
        // Completing a child vendor crossing neither rebinds the restored parent
        // nor fills target lists on any source occurrence.
        let restored = resolve_reference(
            host.source_reference(source(&request, refs[4])),
            &mut host,
            request.policy.limits,
        )
        .unwrap();
        assert!(Arc::ptr_eq(
            host.declaration_node(&restored.nodes[0])
                .unwrap()
                .document(),
            self.first_vendor.ast_owner()
        ));
        assert_authored(&request);
        Ok(InputValidationOutcome {
            complete: true,
            diagnostics: vec![],
        })
    }
}
#[test]
fn engine_scope_defaults_shadow_and_restore_without_confusing_equal_vendor_ids() {
    let stage = Arc::new(DefaultsStage {
        first_vendor: tree("{target @id=shared}", "vendor-first.cem"),
        second_vendor: tree("{target @id=shared}", "vendor-second.cem"),
        calls: AtomicUsize::new(0),
    });
    run(stage.clone(), InputFormat::Cem, "@ns v = urn:outer\n@default v\n@schema src=schema://outer\n{section | {#library} {cem:schema @cem:name=shared | {outer}} {#library} {section @xmlns=urn:inner @xmlns:v=urn:inner @cem:schema-src=schema://inner | {#library} {cem:schema @cem:name=shared | {inner}} {#library}} {#library}}");
    run(stage.clone(), InputFormat::Xml, "<root xmlns='urn:outer' xmlns:v='urn:outer' xmlns:r='https://cem.dev/ns/cem-ml/1' r:schema-src='schema://outer'><r:expr>#library</r:expr><r:schema r:name='shared'><outer/></r:schema><r:expr>#library</r:expr><section xmlns='urn:inner' xmlns:v='urn:inner' r:schema-src='schema://inner'><r:expr>#library</r:expr><r:schema r:name='shared'><inner/></r:schema><r:expr>#library</r:expr></section><r:expr>#library</r:expr></root>");
    assert_eq!(stage.calls.load(Ordering::SeqCst), 2);
}
