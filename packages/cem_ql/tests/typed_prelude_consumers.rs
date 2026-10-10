use cem_ml::{
    events::cem::CemEventNormalizer,
    import::ScopedCemImport,
    parser::{
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode,
    },
    schema::{
        declaration_references::SchemaDeclarationNode, machine::CemSchemaMachine,
        reference_policy::ReferenceScopePolicy, vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::{CemTokenizer, TypedPreludePreview},
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    schema_references::CemQlSchemaDeclarationHost,
};
use std::sync::Arc;

#[path = "typed_prelude_consumers/cancellation.rs"]
mod cancellation;
#[path = "typed_prelude_consumers/namespace.rs"]
mod namespace;
#[path = "typed_prelude_consumers/schema.rs"]
mod schema;
#[path = "typed_prelude_consumers/reload.rs"]
mod reload;

fn import(text: &str) -> ScopedCemImport {
    let captured = Arc::new(
        CemSchemaMachine::new(
            CompiledSchema::cem_core(),
            CemEventNormalizer::new(CemTokenizer::from_source_with_typed_prelude_preview(
                BytesSource::new(SourceId(1), text.as_bytes().to_vec()),
                TypedPreludePreview::default(),
            )),
        )
        .build_with_lexical_scopes(),
    );
    let tree = RetainedCemTree::from_shared(
        captured.document().clone(),
        "typed-preludes.cem",
        text,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    ScopedCemImport { captured, tree }
}
fn elements(input: &ScopedCemImport, local: &str) -> Vec<u32> {
    input
        .tree
        .ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == local => Some(*node_id),
            _ => None,
        })
        .collect()
}
fn node(input: &ScopedCemImport, id: u32) -> SchemaDeclarationNode {
    SchemaDeclarationNode::new(input.tree.ast_owner().clone(), id).unwrap()
}
fn policy() -> ReferenceScopePolicy {
    ReferenceScopePolicy::schema_defaults().unwrap()
}
fn context(input: &ScopedCemImport, name: &str, ids: &[u32]) -> StandaloneExpressionContext {
    StandaloneExpressionContext::default().with_binding(
        name,
        StandaloneExpressionBinding::any(ItemStream::from_items(
            ids.iter()
                .map(|id| {
                    RetainedCemNode::new(input.tree.clone(), *id)
                        .unwrap()
                        .query_item()
                })
                .collect(),
        )),
    )
}
fn assert_original(input: &ScopedCemImport) {
    assert!(input.tree.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
    assert!(Arc::ptr_eq(
        input.captured.document(),
        input.tree.ast_owner()
    ));
}
