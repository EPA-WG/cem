use cem_ml::{ast::reload::ReloadLimits, parser::CemAstNode};
use cem_ql::{
    api::{
        reference_lifecycle::ReferenceValidationSession,
        reference_transport::RetainedReferenceSource, StandaloneExpressionBinding,
        StandaloneExpressionContext,
    },
    eval::{ItemStream, RetainedCemNode},
};
fn source(s: &str) -> RetainedReferenceSource {
    RetainedReferenceSource::parse(
        s.as_bytes(),
        "text/cem-ml",
        "memory:query.cem",
        ReloadLimits::default(),
    )
    .unwrap()
}
fn session(s: RetainedReferenceSource) -> ReferenceValidationSession {
    ReferenceValidationSession::new(s, source("{schema | {elements | {element @name=item}}}"))
}
#[test]
fn pending_forests_are_empty_without_falling_back_to_authored_input() {
    let pending_session = session(source("{item @xmlns:p={#namespace} | {p:item}}"));
    let pending = pending_session.query_snapshot().unwrap();
    assert!(!pending.report().complete);
    assert!(pending
        .evaluate("input", &Default::default())
        .unwrap()
        .result
        .items
        .is_empty());
    let ready = session(source("{item}")).query_snapshot().unwrap();
    assert!(ready.report().complete);
    assert_eq!(
        ready
            .evaluate("input", &Default::default())
            .unwrap()
            .result
            .items
            .len(),
        1
    );
    drop(pending_session);
    assert!(pending
        .evaluate("input", &Default::default())
        .unwrap()
        .result
        .items
        .is_empty());
    assert_eq!(
        ready
            .evaluate("input", &Default::default())
            .unwrap()
            .result
            .items
            .len(),
        1
    );
}
#[test]
fn immutable_views_retain_distinct_completed_names_and_original_owner() {
    let source = source(
        "@ns one = urn:one\n@ns two = urn:two\n{item @xmlns:p={#namespace} | {p:item} {#later}}",
    );
    let mut session = session(source.clone());
    let mut snapshots = vec![];
    let ids: Vec<_> = source
        .ingress()
        .source()
        .ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "@ns" => Some(*node_id),
            _ => None,
        })
        .collect();
    assert_eq!(ids.len(), 2);
    for id in ids {
        session
            .set_context(
                0,
                Some(
                    StandaloneExpressionContext::default().with_binding(
                        "namespace",
                        StandaloneExpressionBinding::any(ItemStream::once(
                            RetainedCemNode::new(source.ingress().source().clone(), id)
                                .unwrap()
                                .query_item(),
                        )),
                    ),
                ),
            )
            .unwrap();
        snapshots.push(session.query_snapshot().unwrap());
    }
    for (snapshot, uri) in snapshots.iter().zip(["urn:one", "urn:two"]) {
        assert!(snapshot.report().complete, "{:?}", snapshot.report());
        let result = snapshot
            .evaluate("input.children", &Default::default())
            .unwrap()
            .result;
        assert!(result.error.is_none(), "{:?}", result.error);
        let namespaces = snapshot
            .evaluate("input.children.namespace", &Default::default())
            .unwrap()
            .result;
        assert!(namespaces.items.iter().any(|item| matches!(item, cem_ql::eval::Item::Atomic(cem_ql::eval::AtomValue::String(value)) if value == uri)), "{namespaces:?}");
    }
    assert!(source.ingress().source().ast().nodes.iter().any(|n| matches!(n, CemAstNode::Element {expanded_name, ..} if expanded_name.local_name == "item" && expanded_name.namespace_uri.is_empty())));
}
#[test]
fn selected_ready_forest_and_authored_references_survive_snapshot_disposal() {
    let mut parent = session(source(
        "{ready | {#later}}{pending @xmlns:p={#namespace} | {p:item}}",
    ));
    parent.set_context(0, Some(Default::default())).unwrap();
    let snapshot = parent.query_snapshot().unwrap();
    assert!(!snapshot.report().complete);
    assert!(snapshot.report().placements >= 1);
    let result = snapshot
        .evaluate("input", &Default::default())
        .unwrap()
        .result;
    assert_eq!(result.items.len(), 1);
    let descendants = snapshot
        .evaluate("input.children", &Default::default())
        .unwrap()
        .result;
    assert!(descendants
        .items
        .iter()
        .any(|item| item.view().is_some_and(|v| v.field("kind")
            == Some(vec![cem_ql::eval::Item::Atomic(
                cem_ql::eval::AtomValue::String("reference".into())
            )]))));
    drop(snapshot);
    drop(parent);
    assert_eq!(
        result.items[0].view().unwrap().field("name").unwrap(),
        vec![cem_ql::eval::Item::Atomic(cem_ql::eval::AtomValue::String(
            "ready".into()
        ))]
    );
    let empty = session(source("")).query_snapshot().unwrap();
    assert!(empty.report().complete);
    assert!(empty
        .evaluate("input", &Default::default())
        .unwrap()
        .result
        .items
        .is_empty());
}
