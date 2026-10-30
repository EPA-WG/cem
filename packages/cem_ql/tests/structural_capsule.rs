//! The capsule supplies an inert shared model; reference and ownership policy
//! still comes from an explicit fresh host, never from the deployment artifact.
use cem_ml::{
    content_cache::ContentHash,
    import::import_bytes_with_lexical_scopes,
    operation_control::OperationControl,
    parser::{tree::RetainedCemTree, CemAstNode},
    schema::{
        document_model::SchemaDocumentModel,
        reference_policy::ReferenceScopePolicy,
        structural_capsule::{export, verify, SourceIdentity},
        vocab::CompiledSchema,
    },
    source::SourceId,
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    schema_references::CemQlSchemaDeclarationHost,
};
use std::sync::Arc;
const SOURCE: &str = include_str!("../../cem_ml/fixtures/structural-cart.cem");
const SCHEMA: &str = "https://app.test/schema/cart/1";
fn imported(text: &str, uri: &str) -> Arc<RetainedCemTree> {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        uri,
        CompiledSchema::cem_core(),
    )
    .unwrap()
    .tree
}
fn report(
    model: &SchemaDocumentModel,
    source: Arc<RetainedCemTree>,
    library: Arc<RetainedCemTree>,
    context_ready: bool,
    grant: bool,
) -> cem_ml::schema::input_references::StructuralInputValidation<
    cem_ql::schema_references::CemQlSchemaReferenceNode,
> {
    let target = library
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "item" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let item = RetainedCemNode::new(library.clone(), target)
        .unwrap()
        .query_item();
    let context = StandaloneExpressionContext::default().with_binding(
        "items",
        StandaloneExpressionBinding::any(ItemStream::from_items(vec![item.clone(), item])),
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    let policy = ReferenceScopePolicy::schema_defaults().unwrap();
    let from = host.register_scope(
        source.clone(),
        context_ready.then_some(context),
        policy.clone(),
    );
    let to = host.register_scope(library, None, policy.clone());
    if grant {
        assert!(host.allow_scope_crossing(from, to));
    }
    host.validate_input(source, model, policy.limits).unwrap()
}
#[test]
fn shared_reference_host_keeps_readiness_grants_placements_and_original_diagnostics() {
    let uri = "https://app.test/schema/cart.cem";
    let original = import_bytes_with_lexical_scopes(
        SOURCE.as_bytes(),
        "text/cem-ml",
        uri,
        CompiledSchema::cem_core(),
    )
    .unwrap();
    let native = cem_ml::schema::document_model::compile_schema_document_model_from_retained(
        SCHEMA,
        &original.tree,
    );
    let capsule = export(
        &original.captured,
        SourceIdentity::new(uri, SourceId(1), SOURCE.as_bytes()),
        SCHEMA,
        &ContentHash::from_blake3(b"structural registry fixture").header_value(),
        &OperationControl::default(),
    )
    .unwrap();
    let loaded = verify(&capsule.bytes, &capsule.pin, &OperationControl::default()).unwrap();
    assert!(!Arc::ptr_eq(
        original.tree.ast_owner(),
        loaded.source().ast_owner()
    ));
    let source = imported(
        "{cart @title=Tea | {#items}}",
        "https://app.test/data/cart.cem",
    );
    for (ready, grant, quantity) in [
        (false, true, "2"),
        (true, false, "2"),
        (true, true, "2"),
        (true, true, "9"),
    ] {
        let library = imported(
            &format!("\n{{item @quantity={quantity}}}"),
            "https://app.test/data/items.cem",
        );
        let expected = report(&native, source.clone(), library.clone(), ready, grant);
        let actual = report(
            loaded.model(),
            source.clone(),
            library.clone(),
            ready,
            grant,
        );
        assert_eq!(
            (
                actual.complete,
                actual.failed,
                actual.roots.clone(),
                actual.nodes.len()
            ),
            (
                expected.complete,
                expected.failed,
                expected.roots.clone(),
                expected.nodes.len()
            )
        );
        let diagnostics =
            |report: &cem_ml::schema::input_references::StructuralInputValidation<_>| {
                report
                    .diagnostics
                    .iter()
                    .map(|d| {
                        (
                            &d.code,
                            &d.uri,
                            &d.source_map,
                            d.byte_offset,
                            d.line,
                            d.column,
                        )
                    })
                    .map(|v| format!("{v:?}"))
                    .collect::<Vec<_>>()
            };
        assert_eq!(diagnostics(&actual), diagnostics(&expected));
        assert_eq!(actual.complete, ready && grant);
        if ready && grant {
            let placements = actual
                .nodes
                .iter()
                .filter(|node| Arc::ptr_eq(node.source.document(), library.ast_owner()))
                .collect::<Vec<_>>();
            assert_eq!(placements.len(), 2);
            assert_eq!(
                placements[0].source.identity(),
                placements[1].source.identity()
            );
            if quantity == "9" {
                assert!(actual.failed);
                assert!(actual.diagnostics.iter().any(|d| d.uri.as_deref()
                    == Some("https://app.test/data/items.cem")
                    && d.line == Some(2)));
            } else {
                assert!(!actual.failed);
            }
        }
    }
}
