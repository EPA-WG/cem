use cem_ml::ast::reload::ReloadLimits;
use cem_ql::api::{
    reference_lifecycle::ReferenceValidationSession, reference_transport::RetainedReferenceSource,
    StandaloneExpressionBinding, StandaloneExpressionContext,
};
use cem_ql::eval::ItemStream;
fn parse(text: &str, uri: &str) -> RetainedReferenceSource {
    RetainedReferenceSource::parse(text.as_bytes(), "text/cem-ml", uri, ReloadLimits::default())
        .unwrap()
}
fn schema() -> RetainedReferenceSource {
    parse(
        "{schema | {elements | {element @name=item}}}",
        "memory:schema.cem",
    )
}
#[test]
fn sessions_distinguish_pending_empty_and_fresh_foreign_targets() {
    let source = parse("{#items}", "memory:input.cem");
    let owner = source.ingress().source().ast_owner().clone();
    let mut session = ReferenceValidationSession::new(source.clone(), schema());
    assert!(!session.run().unwrap().complete);
    let empty = StandaloneExpressionContext::default().with_binding(
        "items",
        StandaloneExpressionBinding::any(ItemStream::empty()),
    );
    session.set_context(0, Some(empty)).unwrap();
    let ready = session.run().unwrap();
    assert!(ready.complete, "{:?}", ready);
    assert!(ready.diagnostics.is_empty());
    let library = parse("{item}", "memory:library.cem");
    let target = library
        .evaluate("input.children", &Default::default())
        .unwrap()
        .result;
    let destination = session.add_source(library.clone());
    session
        .set_context(destination, Some(Default::default()))
        .unwrap();
    session
        .set_context(
            0,
            Some(
                StandaloneExpressionContext::default()
                    .with_binding("items", StandaloneExpressionBinding::any(target)),
            ),
        )
        .unwrap();
    assert!(!session.run().unwrap().complete);
    session.allow_crossing(0, destination).unwrap();
    let ready = session.run().unwrap();
    assert!(ready.complete, "{:?}", ready);
    assert!(ready.diagnostics.is_empty(), "{:?}", ready.diagnostics);
    let independent = ReferenceValidationSession::new(source.clone(), schema());
    assert!(!independent.run().unwrap().complete);
    assert!(std::sync::Arc::ptr_eq(
        source.ingress().source().ast_owner(),
        &owner
    ));
    assert!(owner.nodes.iter().all(|n| !matches!(
        n,
        cem_ml::parser::CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}
#[test]
fn explicit_namespace_lifecycle_completes_original_names_without_source_writeback() {
    let source = parse(
        "@ns public = urn:vendor\n{item @xmlns:p={#namespace} | {p:item}}",
        "memory:namespace.cem",
    );
    let schema = parse(
        "{schema @namespace=urn:vendor | {elements | {element @name=item @children=item}}}",
        "memory:schema.cem",
    );
    let id = source
        .ingress()
        .source()
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            cem_ml::parser::CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "@ns" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let value = ItemStream::once(
        cem_ql::eval::RetainedCemNode::new(source.ingress().source().clone(), id)
            .unwrap()
            .query_item(),
    );
    let mut session = ReferenceValidationSession::new(source, schema);
    assert!(!session.run().unwrap().complete);
    session
        .set_context(
            0,
            Some(
                StandaloneExpressionContext::default()
                    .with_binding("namespace", StandaloneExpressionBinding::any(value)),
            ),
        )
        .unwrap();
    let report = session.run().unwrap();
    assert!(report.complete, "{report:?}");
}

#[test]
fn late_attachment_requires_an_explicit_new_session_view() {
    use cem_ml::ast::reload::ReferenceReloadBundle;
    use cem_ql::api::reference_lifecycle::ReferenceConsumerDependencyKind;
    let original = parse("{#items}", "memory:late.cem");
    let full = original.export_bundle(ReloadLimits::default()).unwrap();
    let mut bundle = ReferenceReloadBundle::decode(&full, ReloadLimits::default()).unwrap();
    bundle.lexical = None;
    let mut source = RetainedReferenceSource::reload(
        &bundle.encode(ReloadLimits::default()).unwrap(),
        1,
        ReloadLimits::default(),
    )
    .unwrap();
    let old = ReferenceValidationSession::new(source.clone(), schema());
    assert!(matches!(
        old.run().unwrap().dependencies[0].kind,
        ReferenceConsumerDependencyKind::MissingLexicalMetadata
    ));
    source
        .attach_bundle(&full, ReloadLimits::default())
        .unwrap();
    let mut next = ReferenceValidationSession::new(source, schema());
    next.set_context(
        0,
        Some(StandaloneExpressionContext::default().with_binding(
            "items",
            StandaloneExpressionBinding::any(ItemStream::empty()),
        )),
    )
    .unwrap();
    assert!(next.run().unwrap().complete);
    assert!(!old.run().unwrap().complete);
    assert!(next
        .set_limits(
            0,
            cem_ml::schema::reference_traversal::ReferenceTraversalLimits {
                max_depth: 0,
                max_work: 5
            }
        )
        .is_err());
    assert!(next.run().unwrap().complete);
}

#[test]
fn schema_compilation_consumes_referenced_declarations_with_its_own_context_and_grant() {
    let input = parse("{item}", "memory:input.cem");
    let schema = parse(
        "{schema | {elements | {#declarations}}}",
        "memory:schema.cem",
    );
    let library = parse(
        "{schema | {elements | {element @name=item}}}",
        "memory:declarations.cem",
    );
    let id = library
        .ingress()
        .source()
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            cem_ml::parser::CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "element" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let values = ItemStream::once(
        cem_ql::eval::RetainedCemNode::new(library.ingress().source().clone(), id)
            .unwrap()
            .query_item(),
    );
    let mut session = ReferenceValidationSession::new(input, schema);
    let target = session.add_source(library);
    session
        .set_context(target, Some(Default::default()))
        .unwrap();
    session
        .set_context(
            1,
            Some(
                StandaloneExpressionContext::default()
                    .with_binding("declarations", StandaloneExpressionBinding::any(values)),
            ),
        )
        .unwrap();
    assert!(!session.run().unwrap().complete);
    session.allow_crossing(1, target).unwrap();
    let report = session.run().unwrap();
    assert!(report.complete, "{report:?}");
    assert!(!report.failed, "{report:?}");
}
