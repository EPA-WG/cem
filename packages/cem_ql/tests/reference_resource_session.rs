use cem_ml::{ast::reload::ReloadLimits, resolver::ResolvedRead};
use cem_ql::api::{
    reference_lifecycle::{resources::ReferenceResourceProgress, ReferenceValidationSession},
    reference_transport::RetainedReferenceSource,
};
fn parse(s: &str, uri: &str) -> RetainedReferenceSource {
    RetainedReferenceSource::parse(s.as_bytes(), "text/cem-ml", uri, ReloadLimits::default())
        .unwrap()
}
fn session() -> ReferenceValidationSession {
    let mut session = ReferenceValidationSession::new(
        parse(
            "{host @schema-src=child.cem | {leaf}}",
            "https://vendor.test/main.cem",
        ),
        parse(
            "{schema | {elements | {element @name=host @children=leaf}}}",
            "memory:base.cem",
        ),
    );
    session.set_context(0, Some(Default::default())).unwrap();
    session
}
fn response() -> ResolvedRead {
    ResolvedRead {
        uri: "https://vendor.test/child.cem".into(),
        content_type: Some("text/cem-ml".into()),
        bytes:
            b"@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=leaf}}}"
                .to_vec(),
    }
}
#[test]
fn staged_import_retains_owner_and_requires_separate_context_and_grant() {
    let mut run = session().start_resources(Default::default()).unwrap();
    let ReferenceResourceProgress::AwaitResources(requests) = run.advance().unwrap() else {
        panic!("Expected URI request")
    };
    assert_eq!(requests[0].uri, "https://vendor.test/child.cem");
    assert!(run.complete(999, Ok(response())).is_err());
    assert!(run.advance().is_err());
    let loaded = run
        .complete(requests[0].id, Ok(response()))
        .unwrap()
        .unwrap();
    let owner = loaded.source.ingress().source().ast_owner().clone();
    assert!(run.complete(requests[0].id, Ok(response())).is_err());
    run.set_loaded_context(loaded.index, Some(Default::default()))
        .unwrap();
    run.allow_crossing(0, loaded.index).unwrap();
    let ReferenceResourceProgress::Finished(report) = run.advance().unwrap() else {
        panic!("Unexpected request")
    };
    assert!(report.complete, "{report:?}");
    assert!(std::sync::Arc::ptr_eq(
        &owner,
        loaded.source.ingress().source().ast_owner()
    ));
    assert!(owner.nodes.iter().all(|n| !matches!(
        n,
        cem_ml::parser::CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
    assert!(run.advance().is_err());
}
#[test]
fn missing_loaded_inputs_do_not_inherit_and_parent_mutation_or_cancel_rejects_completion() {
    for mode in 0..4 {
        let mut parent = session();
        let mut run = parent.start_resources(Default::default()).unwrap();
        let ReferenceResourceProgress::AwaitResources(requests) = run.advance().unwrap() else {
            panic!()
        };
        if mode == 2 {
            parent.set_context(0, None).unwrap();
        }
        if mode == 3 {
            run.cancel();
        }
        let loaded = run.complete(requests[0].id, Ok(response()));
        if mode >= 2 {
            assert!(loaded.is_err());
            continue;
        }
        let loaded = loaded.unwrap().unwrap();
        if mode == 0 {
            run.set_loaded_context(loaded.index, Some(Default::default()))
                .unwrap();
        } else {
            run.allow_crossing(0, loaded.index).unwrap();
        }
        let ReferenceResourceProgress::Finished(report) = run.advance().unwrap() else {
            panic!()
        };
        assert!(!report.complete, "{report:?}");
    }
}
#[test]
fn multiple_resources_require_full_settlement_before_host_preparation() {
    let mut parent = ReferenceValidationSession::new(
        parse(
            "{host @schema-src=one.cem | {leaf}} {host @schema-src=two.cem | {leaf}}",
            "https://vendor.test/main.cem",
        ),
        parse(
            "{schema | {elements | {element @name=host @children=leaf}}}",
            "memory:base.cem",
        ),
    );
    parent.set_context(0, Some(Default::default())).unwrap();
    let mut run = parent.start_resources(Default::default()).unwrap();
    let ReferenceResourceProgress::AwaitResources(requests) = run.advance().unwrap() else {
        panic!()
    };
    assert_eq!(requests.len(), 2);
    let mut loaded = vec![];
    for request in &requests {
        let mut response = response();
        response.uri = request.uri.clone();
        loaded.push(run.complete(request.id, Ok(response)).unwrap().unwrap());
        if loaded.len() == 1 {
            assert!(run.advance().is_err());
            assert!(run
                .set_loaded_context(loaded[0].index, Some(Default::default()))
                .is_err());
            assert!(run.allow_crossing(0, loaded[0].index).is_err());
        }
    }
    for loaded in loaded {
        run.set_loaded_context(loaded.index, Some(Default::default()))
            .unwrap();
        run.allow_crossing(0, loaded.index).unwrap();
    }
    let ReferenceResourceProgress::Finished(report) = run.advance().unwrap() else {
        panic!()
    };
    assert!(report.complete, "{report:?}");
}
#[test]
fn failed_transport_finishes_incomplete_and_bounds_do_not_reset_after_loading() {
    let parent = session();
    let mut run = parent.start_resources(Default::default()).unwrap();
    let ReferenceResourceProgress::AwaitResources(requests) = run.advance().unwrap() else {
        panic!()
    };
    assert!(run
        .complete(
            requests[0].id,
            Err(cem_ml::diagnostics::Diagnostic {
                code: "test.offline".into(),
                message: "offline".into(),
                ..Default::default()
            })
        )
        .unwrap()
        .is_none());
    let ReferenceResourceProgress::Finished(report) = run.advance().unwrap() else {
        panic!()
    };
    assert!(!report.complete);
    assert!(report.diagnostics.iter().any(|d| d.code == "test.offline"));
    let mut parent = session();
    parent
        .set_limits(
            0,
            cem_ml::schema::reference_traversal::ReferenceTraversalLimits {
                max_depth: 128,
                max_work: 1,
            },
        )
        .unwrap();
    let mut run = parent.start_resources(Default::default()).unwrap();
    assert!(run.advance().unwrap_err().contains("WorkLimit"));
}
#[test]
fn loaded_source_query_bindings_reuse_the_coordinators_exact_imported_owner() {
    use cem_ql::api::{StandaloneExpressionBinding, StandaloneExpressionContext};
    let mut run = session().start_resources(Default::default()).unwrap();
    let ReferenceResourceProgress::AwaitResources(requests) = run.advance().unwrap() else {
        panic!()
    };
    let mut response = response();
    response.bytes = b"@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {#declarations}}}{s:element @name=leaf}".to_vec();
    let loaded = run.complete(requests[0].id, Ok(response)).unwrap().unwrap();
    let values = loaded.source.evaluate("seq:where(input.children, fn(node) => node.kind == \"element\" && node.name == \"element\")", &Default::default()).unwrap().result;
    assert!(values.error.is_none(), "{values:?}");
    assert_eq!(values.items.len(), 1);
    run.set_loaded_context(
        loaded.index,
        Some(
            StandaloneExpressionContext::default()
                .with_binding("declarations", StandaloneExpressionBinding::any(values)),
        ),
    )
    .unwrap();
    run.allow_crossing(0, loaded.index).unwrap();
    let ReferenceResourceProgress::Finished(report) = run.advance().unwrap() else {
        panic!()
    };
    assert!(report.complete, "{report:?}");
}
