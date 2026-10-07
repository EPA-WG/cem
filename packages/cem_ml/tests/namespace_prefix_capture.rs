use cem_ml::{
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    schema::vocab::CompiledSchema,
    validation::{rules::UnboundPrefixRule, RuleContext, SemanticRule},
};
fn import() -> ScopedCemImport {
    import_bytes_with_lexical_scopes(
        b"@ns p = urn:ready\n{host @xmlns:q={#library} @p:flag=yes @q:flag=yes @bogus:flag=yes}",
        "text/cem-ml",
        "memory:prefix.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
}
fn diagnostics(
    source: &ScopedCemImport,
    capture: Option<&cem_ml::schema::machine::LexicallyScopedDocument>,
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    UnboundPrefixRule::default().run(&RuleContext {
        lexical_scopes: capture,
        document: source.tree.ast(),
        schema_uri: None,
        content_type: None,
        source_uri: None,
        resource_reader: None,
        schema_registry: None,
        schema_document_models: None,
        upstream_diagnostics: &[],
        schema_behavior_evaluator: None,
    })
}
#[test]
fn captured_ready_and_pending_namespaces_are_declared_without_evaluation() {
    let source = import();
    let diagnostics = diagnostics(&source, Some(&source.captured));
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert!(diagnostics[0].message.contains("bogus"));
    assert!(source.tree.ast().nodes.iter().all(|n| !matches!(
        n,
        cem_ml::parser::CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}
#[test]
fn foreign_capture_cannot_suppress_source_facts_and_legacy_context_remains_compatible() {
    let source = import();
    let foreign = import();
    assert_eq!(diagnostics(&source, Some(&foreign.captured)).len(), 3);
    assert_eq!(diagnostics(&source, None).len(), 3);
}
