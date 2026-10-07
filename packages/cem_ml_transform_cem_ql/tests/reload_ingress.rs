use cem_ml::{
    ast::reload::{
        ReferenceReloadBundle, ReloadDependency, ReloadIngress, ReloadLimits, ReloadSource,
    },
    engine::{EngineInput, FormatIdentity},
    import::import_bytes_with_lexical_scopes,
    query::{run_query_with_source_owner, QueryRunRequest, QuerySource},
    schema::{
        document_model::compile_schema_document_model, reference_policy::ReferenceScopePolicy,
        vocab::CompiledSchema,
    },
    source::SourceId,
};
use cem_ml_transform_cem_ql::{
    engine_context_with_cem_ql_template_adapter, CemQlNativeItemsOwner, CemQlQueryResultArtifact,
};
use cem_ql::eval::retained_cem_node;
use std::sync::Arc;

fn ingress(metadata: bool) -> ReloadIngress {
    let text = "@ns p = urn:vendor\n{p:item}{#input}";
    let imported = import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "memory:original.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap();
    let mut bundle = ReferenceReloadBundle::export(
        &imported.captured,
        vec![
            ReloadSource::new(SourceId(0), "memory:original.cem", text.as_bytes(), true),
            ReloadSource::new(SourceId(1), "memory:original.cem", text.as_bytes(), true),
        ],
        ReloadLimits::default(),
    )
    .unwrap();
    if !metadata {
        bundle.lexical = None;
    }
    ReloadIngress::new(
        Arc::new(bundle.reload(ReloadLimits::default()).unwrap()),
        SourceId(1),
    )
    .unwrap()
}

#[test]
fn repeated_shared_queries_retain_one_decoded_owner_without_reparsing() {
    let ingress = ingress(true);
    for expression in ["input", "#(input, input)", "#()", "input.children"] {
        let result = run_query_with_source_owner(
            QueryRunRequest {
                data: EngineInput {
                    uri: "memory:original.cem".into(),
                    bytes: vec![0xff],
                    identity: Some(FormatIdentity {
                        content_type: Some("text/cem-ml".into()),
                        ..Default::default()
                    }),
                    from_format: None,
                    root_scope: Default::default(),
                },
                query: QuerySource {
                    uri: "memory:query.cemql".into(),
                    bytes: expression.as_bytes().to_vec(),
                    identity: FormatIdentity {
                        content_type: Some(cem_ql::api::CEM_QL_EXPRESSION_CONTENT_TYPE.into()),
                        ..Default::default()
                    },
                },
                context: engine_context_with_cem_ql_template_adapter(),
                context_item: None,
                bindings: Default::default(),
                limits: None,
            },
            ingress.query_source_owner(),
        )
        .unwrap();
        let owner = result
            .result
            .input_ast_owner
            .as_any()
            .downcast_ref::<CemQlNativeItemsOwner>()
            .unwrap();
        let node = retained_cem_node(&owner.stream().items[0]).unwrap();
        assert!(Arc::ptr_eq(
            node.owner().ast_owner(),
            &ingress.reloaded().document
        ));
        assert!(Arc::ptr_eq(node.owner(), ingress.source()));
        assert!(result
            .result
            .native_result
            .as_any()
            .downcast_ref::<CemQlQueryResultArtifact>()
            .is_some());
    }
}

#[test]
fn validation_requires_capture_without_manufacturing_default_bindings() {
    let model = compile_schema_document_model("memory:schema.cem", "@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @name=test @namespace=urn:vendor @version=1.0.0 | {elements | {element @name=item}}}");
    let scope = Default::default();
    let pending = ingress(false);
    assert!(matches!(
        pending.validation_request(
            &model,
            &scope,
            ReferenceScopePolicy::schema_defaults().unwrap(),
            None
        ),
        Err(ReloadDependency::MissingLexicalMetadata)
    ));
    // AST-only query inspection is admitted; validation readiness is a separate contract.
    let _ = pending.query_source_owner();
    let ready = ingress(true);
    let request = ready
        .validation_request(
            &model,
            &scope,
            ReferenceScopePolicy::schema_defaults().unwrap(),
            None,
        )
        .unwrap();
    assert!(Arc::ptr_eq(
        request.source.ast_owner(),
        &ready.reloaded().document
    ));
    assert!(Arc::ptr_eq(
        request.lexical_scopes.as_ref().unwrap().document(),
        &ready.reloaded().document
    ));
}

#[test]
fn reloaded_validation_waits_for_runtime_context_and_explicit_crossing_grants() {
    use cem_ml_transform_cem_ql::RetainedReferenceSource;
    use cem_ql::{
        api::{StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::{ItemStream, RetainedCemNode},
        schema_references::CemQlSchemaDeclarationHost,
    };
    let input = RetainedReferenceSource::parse(
        b"@ns p = urn:vendor\n{#items}",
        "text/cem-ml",
        "memory:request.cem",
        ReloadLimits::default(),
    )
    .unwrap();
    let loaded = RetainedReferenceSource::reload(
        &input.export_bundle(ReloadLimits::default()).unwrap(),
        1,
        ReloadLimits::default(),
    )
    .unwrap();
    let library = import_bytes_with_lexical_scopes(
        b"@ns p = urn:vendor\n{p:item}",
        "text/cem-ml",
        "memory:library.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap();
    let target_id = library
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            cem_ml::parser::CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "item" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let target = RetainedCemNode::new(library.tree.clone(), target_id)
        .unwrap()
        .query_item();
    let model = compile_schema_document_model("memory:schema.cem", "@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @name=test @namespace=urn:vendor @version=1.0.0 | {elements | {element @name=item}}}");
    let root_scope = Default::default();
    let request = loaded
        .ingress()
        .validation_request(
            &model,
            &root_scope,
            ReferenceScopePolicy::schema_defaults().unwrap(),
            None,
        )
        .unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(request.source.clone(), None, request.policy.clone());
    let destination = host.register_scope(library.tree.clone(), None, request.policy.clone());
    let lexical = host
        .attach_captured_lexical_scopes(request.lexical_scopes.as_ref().unwrap(), |_, _, _| {
            (None, request.policy.clone())
        })
        .unwrap();
    let occurrence_scope = lexical[0].1;
    let pending = host
        .validate_input(request.source.clone(), request.model, request.policy.limits)
        .unwrap();
    assert!(!pending.complete);
    host.set_context(
        occurrence_scope,
        Some(StandaloneExpressionContext::default().with_binding(
            "items",
            StandaloneExpressionBinding::any(ItemStream::once(target)),
        )),
    );
    let denied = host
        .validate_input(request.source.clone(), request.model, request.policy.limits)
        .unwrap();
    assert!(!denied.complete);
    assert!(host.allow_scope_crossing(occurrence_scope, destination));
    let ready = host
        .validate_input(request.source.clone(), request.model, request.policy.limits)
        .unwrap();
    assert!(ready.complete, "{:?}", ready.diagnostics);
    assert!(ready.diagnostics.is_empty(), "{:?}", ready.diagnostics);
    assert!(request.source.ast().nodes.iter().all(|n| !matches!(
        n,
        cem_ml::parser::CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}
