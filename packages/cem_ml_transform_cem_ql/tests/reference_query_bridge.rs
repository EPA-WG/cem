use cem_ml::{
    engine::{EngineInput, FormatIdentity, InputFormat},
    lifecycle::LoadedInputAstStream,
    parser::CemAstNode,
    query::{run_query, QueryRunRequest, QueryRunResponse, QuerySource},
    run_config::ScopeConfig,
    schema::registry::CEM_QL_EXPRESSION_CONTENT_TYPE,
};
use cem_ml_transform_cem_ql::{
    engine_context_with_cem_ql_template_adapter, CemQlNativeItemsOwner, CemQlQueryResultArtifact,
};
use cem_ql::eval::{retained_cem_node, Item, QueryContextScope, QueryItemViewKind};
use std::sync::Arc;

fn run(
    format: Option<InputFormat>,
    media: &str,
    source: &str,
    expression: &str,
) -> QueryRunResponse {
    run_query(QueryRunRequest {
        data: EngineInput {
            uri: "memory:reference-bridge".into(),
            bytes: source.as_bytes().to_vec(),
            from_format: format,
            identity: Some(FormatIdentity {
                content_type: Some(media.into()),
                ..Default::default()
            }),
            root_scope: ScopeConfig {
                default_content_type: Some(media.into()),
                ..Default::default()
            },
        },
        query: QuerySource {
            uri: "memory:reference-query.cemql".into(),
            bytes: expression.as_bytes().to_vec(),
            identity: FormatIdentity {
                content_type: Some(CEM_QL_EXPRESSION_CONTENT_TYPE.into()),
                ..Default::default()
            },
        },
        context: engine_context_with_cem_ql_template_adapter(),
        context_item: None,
        bindings: Default::default(),
        limits: None,
    })
    .unwrap()
}
fn targets(item: &Item) -> Vec<Item> {
    let view = item.view().expect("native reference");
    assert_eq!(view.kind(), QueryItemViewKind::Node);
    assert_eq!(
        view.field("kind").unwrap()[0].atom(),
        Some(cem_ql::eval::AtomValue::String("reference".into()))
    );
    assert!(view
        .children(QueryContextScope(0))
        .unwrap()
        .next()
        .is_none());
    view.field("targets").expect("constructed targets")
}
#[test]
fn lifecycle_query_bridge_constructs_references_to_original_native_owners_across_formats() {
    for (format, media, source) in [
        (Some(InputFormat::Xml), "application/xml", "<root xmlns:r='https://cem.dev/ns/cem-ml/1'><r:expr>#missing</r:expr><value ref='{#missing}'>#missing</value></root>"),
        (None, "application/json", r##"{"value":"#missing","ref":"{#missing}"}"##),
        (None, "application/yaml", "value: '#missing'\nref: '{#missing}'\n"),
        (None, "text/csv", "value,ref\n#missing,{#missing}\n"),
    ] {
        let response = run(format, media, source, "(input, #(input, input), ##input, #())");
        assert!(response.diagnostics.iter().all(|d| !d.severity.is_hard_violation()), "{:?}", response.diagnostics);
        let input = response.result.input_ast_owner.as_any().downcast_ref::<CemQlNativeItemsOwner>().unwrap();
        let result = response.result.native_result.as_any().downcast_ref::<CemQlQueryResultArtifact>().unwrap();
        let items = &result.stream().items;
        assert_eq!(items.len(), 4);
        let original = retained_cem_node(&input.stream().items[0]).unwrap();
        let returned = retained_cem_node(&items[0]).unwrap();
        assert!(Arc::ptr_eq(original.owner(), returned.owner()));
        assert_eq!(returned.node_id(), 0);
        let native = original.owner().native_owner().unwrap().downcast_ref::<LoadedInputAstStream>().unwrap();
        assert!(std::ptr::eq(native, input.lifecycle_owner().unwrap().as_ref()));
        assert_eq!(original.owner().source_uri(), "memory:reference-bridge");
        assert!(!items[0].source_map().unwrap().frames.is_empty());
        let repeated = targets(&items[1]);
        assert_eq!(repeated.len(), 2);
        for item in repeated {
            let target = retained_cem_node(&item).unwrap();
            assert!(Arc::ptr_eq(target.owner(), original.owner()));
            assert_eq!(target.node_id(), original.node_id());
        }
        let nested = targets(&items[2]);
        assert_eq!(nested.len(), 1);
        let inner = targets(&nested[0]);
        assert_eq!(inner.len(), 1);
        assert!(Arc::ptr_eq(retained_cem_node(&inner[0]).unwrap().owner(), original.owner()));
        assert!(targets(&items[3]).is_empty());
        let references: Vec<_> = original.owner().ast().nodes.iter().filter(|node| matches!(node, CemAstNode::Reference { .. })).collect();
        assert_eq!(references.len(), usize::from(format == Some(InputFormat::Xml)));
        for node in references {
            assert!(matches!(node, CemAstNode::Reference { expression, targets: None, .. } if expression == "#missing"));
        }
        assert!(original.owner().ast().nodes.iter().any(|node| matches!(node,
            CemAstNode::Text { data, .. } if data == "#missing")));
    }
}
#[test]
fn lifecycle_query_bridge_can_reference_an_authored_reference_without_evaluating_it() {
    let response = run(
        Some(InputFormat::Xml),
        "application/xml",
        "<root xmlns:r='https://cem.dev/ns/cem-ml/1'><r:expr>#unavailable</r:expr><target/></root>",
        "#(input.children.children, input.children.children)",
    );
    let input = response
        .result
        .input_ast_owner
        .as_any()
        .downcast_ref::<CemQlNativeItemsOwner>()
        .unwrap();
    let original = retained_cem_node(&input.stream().items[0]).unwrap();
    let result = response
        .result
        .native_result
        .as_any()
        .downcast_ref::<CemQlQueryResultArtifact>()
        .unwrap();
    let selected = targets(&result.stream().items[0]);
    assert_eq!(selected.len(), 4);
    for item in &selected {
        let target = retained_cem_node(item).unwrap();
        assert!(Arc::ptr_eq(target.owner(), original.owner()));
    }
    assert_eq!(
        selected[0].view().unwrap().identity(),
        selected[2].view().unwrap().identity()
    );
    let authored = retained_cem_node(&selected[0]).unwrap();
    assert!(
        matches!(authored.node(), CemAstNode::Reference { expression, targets: None, .. } if expression == "#unavailable")
    );
    assert!(selected[0].view().unwrap().field("targets").is_none());
}

#[test]
fn supplied_native_sources_and_reference_operands_do_not_fetch_uri_parts() {
    use cem_ml::{
        query::QueryRunError,
        resolver::{
            ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedRead, ResolvedWrite,
            ResolverDiagnostic, ResourceResolver,
        },
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct CountReads(Arc<AtomicUsize>);
    impl ResourceResolver for CountReads {
        fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Err(ResolverDiagnostic::Io {
                uri: request.uri.clone(),
                message: "unexpected resource read".into(),
            })
        }
        fn write(
            &self,
            request: &ResolveRequest,
            _: &[u8],
        ) -> Result<ResolvedWrite, ResolverDiagnostic> {
            Err(ResolverDiagnostic::Io {
                uri: request.uri.clone(),
                message: "unexpected resource write".into(),
            })
        }
    }
    let reads = Arc::new(AtomicUsize::new(0));
    for (expression, succeeds) in [
        ("#input", true),
        ("#seq:map(seq:where(input.descendants, fn(node) => node.kind == \"text\"), fn(node) => node.value)", false),
    ] {
        let mut context = engine_context_with_cem_ql_template_adapter();
        for purpose in [ResolvePurpose::Input, ResolvePurpose::Query, ResolvePurpose::Template, ResolvePurpose::ModuleMap] {
            context.resolver_registry.register("https", purpose, ResolveDirection::Read, CountReads(reads.clone()));
        }
        let response = run_query(QueryRunRequest {
            data: EngineInput {
                uri: "https://vendor.invalid/source.xml".into(),
                bytes: b"<root>https://vendor.invalid/document#part</root>".to_vec(),
                from_format: Some(InputFormat::Xml),
                identity: Some(FormatIdentity { content_type: Some("application/xml".into()), ..Default::default() }),
                root_scope: ScopeConfig { default_content_type: Some("application/xml".into()), ..Default::default() },
            },
            query: QuerySource {
                uri: "memory:reference-resource-boundary.cemql".into(), bytes: expression.as_bytes().to_vec(),
                identity: FormatIdentity { content_type: Some(CEM_QL_EXPRESSION_CONTENT_TYPE.into()), ..Default::default() },
            },
            context, context_item: None, bindings: Default::default(), limits: None,
        });
        if succeeds {
            let response = response.unwrap();
            let result = response.result.native_result.as_any().downcast_ref::<CemQlQueryResultArtifact>().unwrap();
            let target = targets(&result.stream().items[0]).remove(0);
            let node = retained_cem_node(&target).unwrap();
            assert_eq!(node.owner().source_uri(), "https://vendor.invalid/source.xml");
        } else {
            let QueryRunError::Execution(failure) = response.unwrap_err() else { panic!("native operand failure") };
            let diagnostic = failure.diagnostics.iter().find(|d| d.code == "cem.ql.type_error").unwrap_or_else(|| panic!("unexpected diagnostics: {:?}", failure.diagnostics));
            assert_eq!(diagnostic.uri.as_deref(), Some("memory:reference-resource-boundary.cemql"));
            assert!(!diagnostic.source_map.as_ref().unwrap().frames.is_empty());
        }
        assert_eq!(reads.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn cem_query_ingress_retains_original_parser_owner_and_captured_bindings_for_explicit_consumers() {
    use cem_ml::{
        query::QuerySourceOwner,
        schema::{
            declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
            reference_policy::ReferenceScopePolicy,
        },
        value::reference_resolution::resolve_reference,
    };
    use cem_ql::{
        api::{StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::{ItemStream, RetainedCemNode},
        schema_references::CemQlSchemaDeclarationHost,
    };
    let text = "@ns v = urn:first\n{outer @target={#library} | {#library} {target}}\n@ns v = urn:second\n{inner | {#library} {target}}";
    let response = run(
        Some(InputFormat::Cem),
        "application/cem",
        text,
        "#(input, input)",
    );
    let input = response
        .result
        .input_ast_owner
        .as_any()
        .downcast_ref::<CemQlNativeItemsOwner>()
        .unwrap();
    assert!(input.lifecycle_owner().is_none());
    let QuerySourceOwner::Cem {
        source,
        lexical_scopes,
    } = input.source_owner()
    else {
        panic!("original CEM source")
    };
    let captured = lexical_scopes.as_ref().unwrap();
    assert!(Arc::ptr_eq(source.ast_owner(), captured.document()));
    let item = retained_cem_node(&input.stream().items[0]).unwrap();
    assert!(Arc::ptr_eq(item.owner(), source));
    let result = response
        .result
        .native_result
        .as_any()
        .downcast_ref::<CemQlQueryResultArtifact>()
        .unwrap();
    let result_targets = targets(&result.stream().items[0]);
    assert_eq!(result_targets.len(), 2);
    for result_target in result_targets {
        assert!(Arc::ptr_eq(
            retained_cem_node(&result_target).unwrap().owner(),
            source
        ));
    }
    let target_ids: Vec<_> = source
        .ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "target" => Some(*node_id),
            _ => None,
        })
        .collect();
    assert_eq!(target_ids.len(), 2);
    let refs: Vec<_> = captured.occurrences().collect();
    assert_eq!(refs.len(), 3);
    let policy = ReferenceScopePolicy::schema_defaults().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let root = host.register_scope(source.clone(), None, policy.clone());
    let mut bindings = vec![];
    host.attach_captured_lexical_scopes(captured, |node, snapshot, parent| {
        assert_eq!(parent, root);
        let namespace = &snapshot.namespaces.binding("v").unwrap().namespace_uri;
        bindings.push(namespace.clone());
        assert!(Arc::ptr_eq(node.document(), source.ast_owner()));
        let target = target_ids[usize::from(namespace == "urn:second")];
        let context = StandaloneExpressionContext::default().with_binding(
            "library",
            StandaloneExpressionBinding::any(ItemStream::once(
                RetainedCemNode::new(source.clone(), target)
                    .unwrap()
                    .query_item(),
            )),
        );
        (Some(context), policy.clone())
    })
    .unwrap();
    assert_eq!(bindings, ["urn:first", "urn:first", "urn:second"]);
    for (index, &id) in refs.iter().enumerate() {
        let original = SchemaDeclarationNode::new(source.ast_owner().clone(), id).unwrap();
        assert!(host.compiled_source_expression(&original).is_none());
        let reference = host.source_reference(original);
        let result = resolve_reference(reference, &mut host, policy.limits).unwrap();
        assert!(result.is_complete());
        let selected = host.declaration_node(&result.nodes[0]).unwrap();
        assert!(Arc::ptr_eq(selected.document(), source.ast_owner()));
        assert_eq!(selected.node_id(), target_ids[usize::from(index == 2)]);
        assert!(matches!(
            source.ast().get(id),
            Some(CemAstNode::Reference { targets: None, .. })
        ));
    }
}
