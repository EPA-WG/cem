use cem_ml::{
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    parser::CemAstNode,
    resolver::{ResolvedRead, ResolverPolicy},
    schema::{
        declaration_references::SchemaDeclarationNode,
        reference_policy::ReferenceScopePolicy,
        scope_controls::{decode_schema_host_control, SchemaHostControl},
        vocab::CompiledSchema,
    },
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    schema_references::CemQlSchemaDeclarationHost,
};
fn policy() -> ReferenceScopePolicy {
    ReferenceScopePolicy::schema_defaults().unwrap()
}
fn imported(text: &str) -> ScopedCemImport {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "https://vendor.test/main.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
}
fn node(import: &ScopedCemImport, name: &str) -> SchemaDeclarationNode {
    import
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => {
                SchemaDeclarationNode::new(import.tree.ast_owner().clone(), *node_id)
            }
            _ => None,
        })
        .unwrap()
}
fn control(import: &ScopedCemImport) -> SchemaHostControl {
    decode_schema_host_control(node(import, "host"), |source| {
        import
            .captured
            .expanded_name(source.document(), source.node_id())
            .cloned()
    })
    .unwrap()
    .unwrap()
}
fn response(text: &str) -> Result<ResolvedRead, cem_ml::diagnostics::Diagnostic> {
    Ok(ResolvedRead {
        uri: "https://cdn.test/schema.cem".into(),
        bytes: text.as_bytes().to_vec(),
        content_type: Some("text/cem-ml".into()),
    })
}
fn no_exports(_: &ScopedCemImport, _: &str) -> Result<Vec<SchemaDeclarationNode>, String> {
    Err("no exports".into())
}
const SCHEMA: &str =
    "@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child}}}";
#[test]
fn resource_generations_reject_stale_and_foreign_completions_before_import_or_inputs() {
    let source = imported("{host @schema-src=./schema.cem}");
    let control = control(&source);
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(
        source.tree.clone(),
        Some(StandaloneExpressionContext::default()),
        policy(),
    );
    host.attach_captured_names(&source.captured).unwrap();
    let old = host
        .begin_schema_uri_resource(
            &control,
            source.tree.source_uri(),
            None,
            &ResolverPolicy::new(),
        )
        .unwrap();
    let current = host
        .begin_schema_uri_resource(
            &control,
            source.tree.source_uri(),
            None,
            &ResolverPolicy::new(),
        )
        .unwrap();
    assert!(host
        .complete_schema_uri_resource(
            &old,
            response(SCHEMA),
            policy(),
            |_| panic!("stale context"),
            |_, _| panic!("stale export")
        )
        .is_err());
    let mut foreign = CemQlSchemaDeclarationHost::new();
    assert!(foreign
        .complete_schema_uri_resource(
            &current,
            response(SCHEMA),
            policy(),
            |_| panic!("foreign context"),
            no_exports
        )
        .is_err());
    let loaded = host
        .complete_schema_uri_resource(&current, response(SCHEMA), policy(), |_| None, no_exports)
        .unwrap();
    assert!(!host
        .prepare_schema_host_region("child", control.host.clone(), policy().limits)
        .unwrap()
        .is_ready());
    host.allow_scope_crossing(origin, loaded.scope);
    let pending = host
        .prepare_schema_host_region("child", control.host.clone(), policy().limits)
        .unwrap();
    assert!(!pending.is_ready());
    host.set_context(loaded.scope, Some(StandaloneExpressionContext::default()));
    assert!(host
        .prepare_schema_host_region("child", control.host.clone(), policy().limits)
        .unwrap()
        .is_ready());
    assert!(host
        .complete_schema_uri_resource(
            &current,
            response(SCHEMA),
            policy(),
            |_| panic!("duplicate context"),
            no_exports
        )
        .is_err());
    let fresh = host
        .begin_schema_uri_resource(
            &control,
            source.tree.source_uri(),
            None,
            &ResolverPolicy::new(),
        )
        .unwrap();
    assert!(!host
        .prepare_schema_host_region("child", control.host.clone(), policy().limits)
        .unwrap()
        .is_ready());
    assert!(host
        .complete_schema_uri_resource(
            &fresh,
            response("{broken"),
            policy(),
            |_| panic!("invalid import inputs"),
            no_exports
        )
        .is_err());
    assert!(!host
        .prepare_schema_host_region("child", control.host, policy().limits)
        .unwrap()
        .is_ready());
}
#[test]
fn loaded_references_use_explicit_lexical_contexts_and_then_activate_original_input_regions() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let source = imported("{host @schema-src=./schema.cem | {child}}");
    let control = control(&source);
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(
        source.tree.clone(),
        Some(StandaloneExpressionContext::default()),
        policy(),
    );
    host.attach_captured_names(&source.captured).unwrap();
    let ticket = host
        .begin_schema_uri_resource(
            &control,
            source.tree.source_uri(),
            Some("text/cem-ml"),
            &ResolverPolicy::new(),
        )
        .unwrap();
    assert_eq!(
        ticket.request().request().uri,
        "https://vendor.test/schema.cem"
    );
    let loaded=host.complete_schema_uri_resource(&ticket,response("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {#library}}} {s:element @name=child}"),policy(),|_|Some(StandaloneExpressionContext::default()),no_exports).unwrap();
    let target = node(&loaded.resource.imported, "element");
    let context = StandaloneExpressionContext::default().with_binding(
        "library",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(loaded.resource.imported.tree.clone(), target.node_id())
                .unwrap()
                .query_item(),
        )),
    );
    host.attach_captured_lexical_scopes(&loaded.resource.imported.captured, |_, _, _| {
        (Some(context.clone()), policy())
    })
    .unwrap();
    host.allow_scope_crossing(origin, loaded.scope);
    let outer = compile_schema_document_model(
        "base",
        "{schema | {elements | {element @name=host @children=child}}}",
    );
    let report = host
        .validate_input_runtime_host_regions(
            "child",
            source.tree.clone(),
            &[control.host.node_id()],
            &outer,
            policy().limits,
            |_| Some(StandaloneExpressionContext::default()),
        )
        .unwrap();
    assert!(
        report.validation.complete && !report.validation.failed,
        "{:?}",
        report.validation.diagnostics
    );
    assert_eq!(report.scopes.len(), 1);
    assert!(report
        .validation
        .nodes
        .iter()
        .all(|n| std::sync::Arc::ptr_eq(n.source.document(), source.tree.ast_owner())));
    assert!(loaded
        .resource
        .imported
        .tree
        .ast()
        .nodes
        .iter()
        .all(|node| !matches!(
            node,
            CemAstNode::Reference {
                targets: Some(_),
                ..
            }
        )));
}

#[test]
fn controlled_resolver_failures_retry_without_activation_then_keep_explicit_public_parts() {
    use cem_ml::{
        operation_control::OperationControl,
        resolver::{
            ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedWrite, ResolverDiagnostic,
            ResolverRegistry, ResourceResolver,
        },
        scheduler::AbortSignal,
        schema::document_model::compile_schema_document_model,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    struct Reader(Arc<AtomicUsize>);
    impl ResourceResolver for Reader {
        fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
            assert_eq!(request.uri, "https://vendor.test/schema.cem");
            if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
                return Err(ResolverDiagnostic::Io {
                    uri: request.uri.clone(),
                    message: "transport unavailable".into(),
                });
            }
            Ok(ResolvedRead { uri:"https://cdn.test/schema.xml".into(),bytes:b"<s:schema xmlns:s='https://cem.dev/ns/schema/1'><s:elements><s:element name='child'/></s:elements></s:schema>".to_vec(),content_type:Some("application/xml".into()) })
        }
        fn write(&self, _: &ResolveRequest, _: &[u8]) -> Result<ResolvedWrite, ResolverDiagnostic> {
            unreachable!()
        }
    }
    let source = imported("{host @schema-src='./schema.cem#public' | {child}}");
    let control = control(&source);
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(
        source.tree.clone(),
        Some(StandaloneExpressionContext::default()),
        policy(),
    );
    host.attach_captured_names(&source.captured).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = ResolverRegistry::new();
    registry.register(
        "https",
        ResolvePurpose::Template,
        ResolveDirection::Read,
        Reader(calls.clone()),
    );
    for cancelled in [true, false] {
        let ticket = host
            .begin_schema_uri_resource(
                &control,
                source.tree.source_uri(),
                None,
                &ResolverPolicy::new(),
            )
            .unwrap();
        let abort = AbortSignal::new();
        if cancelled {
            abort.abort();
        }
        let operation = OperationControl::new(abort);
        let outcome = ticket
            .request()
            .read(&registry, &operation, operation.root_scope());
        let error = host
            .complete_schema_uri_resource(
                &ticket,
                outcome,
                policy(),
                |_| panic!("failed transport inputs"),
                |_, _| panic!("failed transport exports"),
            )
            .unwrap_err();
        assert_eq!(
            error.code,
            if cancelled {
                "cem.resolver.cancelled"
            } else {
                "cem.resolver.io"
            }
        );
        let prepared = host
            .prepare_schema_host_region("child", control.host.clone(), policy().limits)
            .unwrap();
        assert!(!prepared.is_ready());
        assert!(prepared.preparation.unwrap().selection.failed);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let ticket = host
        .begin_schema_uri_resource(
            &control,
            source.tree.source_uri(),
            None,
            &ResolverPolicy::new(),
        )
        .unwrap();
    let operation = OperationControl::new(AbortSignal::new());
    let outcome = ticket
        .request()
        .read(&registry, &operation, operation.root_scope());
    let loaded = host
        .complete_schema_uri_resource(
            &ticket,
            outcome,
            policy(),
            |_| Some(StandaloneExpressionContext::default()),
            |imported, part| {
                assert_eq!(part, "public");
                // The embedding loader declares this export; there is no internal ID scan.
                Ok(vec![node(imported, "schema")])
            },
        )
        .unwrap();
    assert_eq!(
        loaded.resource.imported.tree.source_uri(),
        "https://cdn.test/schema.xml"
    );
    assert!(!host
        .prepare_schema_host_region("child", control.host.clone(), policy().limits)
        .unwrap()
        .is_ready());
    host.allow_scope_crossing(origin, loaded.scope);
    let outer = compile_schema_document_model(
        "base",
        "{schema | {elements | {element @name=host @children=child}}}",
    );
    let report = host
        .validate_input_runtime_host_regions(
            "child",
            source.tree.clone(),
            &[control.host.node_id()],
            &outer,
            policy().limits,
            |_| Some(StandaloneExpressionContext::default()),
        )
        .unwrap();
    assert!(
        report.validation.complete && !report.validation.failed,
        "{:?}",
        report.validation.diagnostics
    );
    assert_eq!(report.scopes.len(), 1);
    // Explicit release/manual snapshots also supersede outstanding byte responses.
    let released = host
        .begin_schema_uri_resource(
            &control,
            source.tree.source_uri(),
            None,
            &ResolverPolicy::new(),
        )
        .unwrap();
    host.clear_schema_uri_load(&control.host, "./schema.cem#public");
    assert!(host
        .complete_schema_uri_resource(
            &released,
            response(SCHEMA),
            policy(),
            |_| panic!("released inputs"),
            no_exports
        )
        .is_err());
}
