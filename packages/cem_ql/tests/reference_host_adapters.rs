use cem_ml::{
    ast::reload::ReloadLimits, parser::CemAstNode, resolver::ResolvedRead,
    schema::declaration_references::SchemaDeclarationNode,
};
use cem_ql::{
    api::{
        reference_lifecycle::{resources::ReferenceResourceProgress, ReferenceValidationSession},
        reference_transport::RetainedReferenceSource,
        StandaloneExpressionBinding, StandaloneExpressionContext,
    },
    eval::ItemStream,
};
fn parse(s: &str, uri: &str) -> RetainedReferenceSource {
    RetainedReferenceSource::parse(s.as_bytes(), "text/cem-ml", uri, ReloadLimits::default())
        .unwrap()
}
fn id(source: &RetainedReferenceSource, name: &str) -> u32 {
    source
        .ingress()
        .source()
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => Some(*node_id),
            _ => None,
        })
        .unwrap()
}
fn context(values: ItemStream) -> StandaloneExpressionContext {
    StandaloneExpressionContext::default()
        .with_binding("items", StandaloneExpressionBinding::any(values))
}
#[test]
fn occurrence_contexts_inherit_only_within_their_owning_subtree_and_pending_shadows_default() {
    let source = parse("{one | {#items}}{two | {#items}}", "memory:input.cem");
    let schema = parse("{schema | {elements | {element @name=one @children=item} {element @name=two @children=item} {element @name=item}}}", "memory:schema.cem");
    let library = parse("{item}", "memory:library.cem");
    let targets = library
        .evaluate("input.children", &Default::default())
        .unwrap()
        .result;
    let mut session = ReferenceValidationSession::new(source.clone(), schema);
    session
        .set_context(0, Some(context(ItemStream::empty())))
        .unwrap();
    let dest = session.add_source(library);
    session.set_context(dest, Some(Default::default())).unwrap();
    session.allow_crossing(0, dest).unwrap();
    session
        .set_occurrence_context(0, id(&source, "one"), Some(context(targets)))
        .unwrap();
    assert!(session.run().unwrap().complete);
    session
        .set_occurrence_context(0, id(&source, "two"), None)
        .unwrap();
    assert!(!session.run().unwrap().complete);
    session
        .clear_occurrence_context(0, id(&source, "two"))
        .unwrap();
    assert!(session.run().unwrap().complete);
    assert!(session.set_occurrence_context(0, u32::MAX, None).is_err());
}
fn uri_session() -> ReferenceValidationSession {
    let mut session = ReferenceValidationSession::new(
        parse(
            "{host @schema-src=lib.cem#leaf | {leaf}}",
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
        uri: "https://vendor.test/lib.cem".into(),
        content_type: Some("text/cem-ml".into()),
        bytes:
            b"@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=leaf}}}"
                .to_vec(),
    }
}
#[test]
fn explicit_public_exports_are_original_handles_and_do_not_grant_crossings() {
    let parent = uri_session();
    let mut run = parent.start_resources(Default::default()).unwrap();
    let ReferenceResourceProgress::AwaitResources(requests) = run.advance().unwrap() else {
        panic!()
    };
    assert_eq!(requests[0].public_part.as_deref(), Some("leaf"));
    let loaded = run
        .complete_with_exports(requests[0].id, Ok(response()), |imported, part| {
            assert_eq!(part, "leaf");
            Ok(imported
                .tree
                .ast()
                .nodes
                .iter()
                .filter_map(|n| match n {
                    CemAstNode::Element {
                        node_id,
                        expanded_name,
                        ..
                    } if expanded_name.local_name == "schema" => {
                        SchemaDeclarationNode::new(imported.tree.ast_owner().clone(), *node_id)
                    }
                    _ => None,
                })
                .collect())
        })
        .unwrap()
        .unwrap();
    run.set_loaded_context(loaded.index, Some(Default::default()))
        .unwrap();
    let loaded_snapshot = run.query_snapshot(loaded.index).unwrap();
    let values = loaded_snapshot
        .evaluate("input", &Default::default())
        .unwrap()
        .result;
    assert!(!values.items.is_empty());
    let ReferenceResourceProgress::Finished(report) = run.advance().unwrap() else {
        panic!()
    };
    assert!(!report.complete);
    let snapshot = run.query_snapshot(0).unwrap();
    drop(run);
    drop(parent);
    assert!(snapshot.evaluate("input", &Default::default()).is_ok());
    assert!(values.items[0].view().is_some());
}
#[test]
fn occurrence_mutation_invalidates_pending_resource_completion() {
    let mut parent = uri_session();
    let mut run = parent.start_resources(Default::default()).unwrap();
    let ReferenceResourceProgress::AwaitResources(requests) = run.advance().unwrap() else {
        panic!()
    };
    parent.set_occurrence_context(0, 0, None).unwrap();
    assert!(run.complete(requests[0].id, Ok(response())).is_err());
}
#[test]
fn public_exports_reject_empty_ambiguous_and_foreign_owner_results() {
    for mode in 0..3 {
        let parent = uri_session();
        let mut run = parent.start_resources(Default::default()).unwrap();
        let ReferenceResourceProgress::AwaitResources(requests) = run.advance().unwrap() else {
            panic!()
        };
        let foreign = parse("{schema}", "memory:foreign.cem");
        let loaded = run
            .complete_with_exports(requests[0].id, Ok(response()), |imported, _| {
                let owner = if mode == 2 {
                    foreign.ingress().source().ast_owner().clone()
                } else {
                    imported.tree.ast_owner().clone()
                };
                let node = owner
                    .nodes
                    .iter()
                    .find_map(|n| match n {
                        CemAstNode::Element {
                            node_id,
                            expanded_name,
                            ..
                        } if expanded_name.local_name == "schema" => {
                            SchemaDeclarationNode::new(owner.clone(), *node_id)
                        }
                        _ => None,
                    })
                    .unwrap();
                Ok(match mode {
                    0 => vec![],
                    1 => vec![node.clone(), node],
                    _ => vec![node],
                })
            })
            .unwrap();
        assert!(loaded.is_none());
        let ReferenceResourceProgress::Finished(report) = run.advance().unwrap() else {
            panic!()
        };
        assert!(!report.complete);
    }
}
#[test]
fn public_export_with_explicit_crossing_activates_and_saved_names_outlive_execution() {
    let parent = uri_session();
    let mut run = parent.start_resources(Default::default()).unwrap();
    let ReferenceResourceProgress::AwaitResources(requests) = run.advance().unwrap() else {
        panic!()
    };
    let loaded = run
        .complete_with_exports(requests[0].id, Ok(response()), |imported, _| {
            Ok(imported
                .tree
                .ast()
                .nodes
                .iter()
                .filter_map(|n| match n {
                    CemAstNode::Element {
                        node_id,
                        expanded_name,
                        ..
                    } if expanded_name.local_name == "schema" => {
                        SchemaDeclarationNode::new(imported.tree.ast_owner().clone(), *node_id)
                    }
                    _ => None,
                })
                .collect())
        })
        .unwrap()
        .unwrap();
    run.set_loaded_context(loaded.index, Some(Default::default()))
        .unwrap();
    let snapshot = run.query_snapshot(loaded.index).unwrap();
    let node = snapshot
        .evaluate(
            "seq:where(input, fn(node) => node.kind == \"element\" && node.name == \"schema\")",
            &Default::default(),
        )
        .unwrap()
        .result
        .items[0]
        .clone();
    assert!(run
        .set_loaded_occurrence_value_context(loaded.index, &node, Some(Default::default()))
        .is_ok());
    run.set_loaded_occurrence_value_context(loaded.index, &node, None)
        .unwrap();
    run.clear_loaded_occurrence_value_context(loaded.index, &node)
        .unwrap();
    let foreign = parent
        .query_snapshot()
        .unwrap()
        .evaluate("input.children", &Default::default())
        .unwrap()
        .result
        .items[0]
        .clone();
    assert!(run
        .set_loaded_occurrence_value_context(loaded.index, &foreign, None)
        .is_err());
    run.allow_crossing(0, loaded.index).unwrap();
    let ReferenceResourceProgress::Finished(report) = run.advance().unwrap() else {
        panic!()
    };
    assert!(report.complete, "{report:?}");
    drop(run);
    drop(parent);
    drop(loaded);
    assert!(snapshot
        .evaluate("input.children.name", &Default::default())
        .unwrap()
        .result
        .error
        .is_none());
    assert!(node.view().is_some());
}
#[test]
fn explicit_xml_attribute_slots_reach_bounded_schema_consumers() {
    for expression in ["#items", "items"] {
        let text = format!("<item xmlns:c='https://cem.dev/ns/core/1' c:expression-attributes='target' target='{{{expression}}}' literal='{{#items}}'/>");
        let input = RetainedReferenceSource::parse(
            text.as_bytes(),
            "application/xml",
            "memory:input.xml",
            ReloadLimits::default(),
        )
        .unwrap();
        let owner = input.ingress().source().ast_owner().clone();
        let schema = parse("{schema | {elements | {element @name=item @required-attributes=target @optional-attributes=literal}} {attributes | {attribute @name=target @type=node} {attribute @name=literal @type=string}}}","memory:schema.cem");
        let library = parse("{target | {#not-consumed}}", "memory:library.cem");
        let values = library
            .evaluate("input.children", &Default::default())
            .unwrap()
            .result;
        let mut session = ReferenceValidationSession::new(input, schema);
        session.set_context(0, Some(context(values))).unwrap();
        let dest = session.add_source(library);
        session.set_context(dest, Some(Default::default())).unwrap();
        session.allow_crossing(0, dest).unwrap();
        let report = session.run().unwrap();
        assert!(report.complete, "{report:?}");
        assert!(!report.failed, "{report:?}");
        assert!(owner
            .nodes
            .iter()
            .any(|n| matches!(n,CemAstNode::Attribute{value:Some(value),..} if value=="{#items}")));
        assert!(owner.nodes.iter().all(|n| !matches!(
            n,
            CemAstNode::Reference {
                targets: Some(_),
                ..
            }
        )));
    }
}
#[test]
fn enclosed_host_schema_and_namespace_overrides_restore_together() {
    let input = parse("@ns cem = https://cem.dev/ns/core/1\n@ns public = urn:vendor\n{root | {section @cem:schema-select={#schema} @xmlns:p={#namespace} | {p:chosen}} {after}}", "memory:enclosed.cem");
    let selected = parse("@ns s = https://cem.dev/ns/schema/1\n{s:schema @namespace=urn:vendor | {elements | {element @name=section @children=chosen}{element @name=chosen}}}","memory:selected.cem");
    let schema_values = selected.evaluate("seq:where(input.children, fn(node) => node.kind == \"element\" && node.name == \"schema\")",&Default::default()).unwrap().result;
    let ns_values = input.evaluate("seq:last(seq:where(input.children, fn(node) => node.kind == \"element\" && node.name == \"@ns\"))",&Default::default()).unwrap().result;
    let base = parse("{schema | {elements | {element @name=root @children='section after'} {element @name=section @children=chosen} {element @name=after}}}","memory:base.cem");
    let mut session = ReferenceValidationSession::new(input, base);
    let context = StandaloneExpressionContext::default()
        .with_binding("schema", StandaloneExpressionBinding::any(schema_values))
        .with_binding("namespace", StandaloneExpressionBinding::any(ns_values));
    session.set_context(0, Some(context)).unwrap();
    let destination = session.add_source(selected);
    session
        .set_context(destination, Some(Default::default()))
        .unwrap();
    session.allow_crossing(0, destination).unwrap();
    let report = session.run().unwrap();
    assert!(report.complete, "{report:?}");
    assert!(!report.failed, "{report:?}");
    assert!(session.query_snapshot().unwrap().report().complete);
}
#[test]
fn parsed_block_schema_prelude_uses_existing_runtime_stage_and_restores_siblings() {
    let input = parse(
        "{root |\n @schema select=library\n {chosen}\n}\n{after}",
        "memory:block.cem",
    );
    let selected = parse(
        "@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=chosen}}}",
        "memory:selected.cem",
    );
    let schema_values = selected.evaluate("seq:where(input.children, fn(node) => node.kind == \"element\" && node.name == \"schema\")",&Default::default()).unwrap().result;
    let base = parse(
        "{schema | {elements | {element @name=root @children=chosen} {element @name=after}}}",
        "memory:base.cem",
    );
    let mut session = ReferenceValidationSession::new(input, base);
    session
        .set_context(
            0,
            Some(
                StandaloneExpressionContext::default()
                    .with_binding("library", StandaloneExpressionBinding::any(schema_values)),
            ),
        )
        .unwrap();
    let destination = session.add_source(selected);
    session
        .set_context(destination, Some(Default::default()))
        .unwrap();
    session.allow_crossing(0, destination).unwrap();
    let report = session.run().unwrap();
    assert!(report.complete, "{report:?}");
    assert!(!report.failed, "{report:?}");
}
#[test]
fn loaded_query_snapshots_exclude_pending_names_without_context_evaluation() {
    let parent = uri_session();
    let mut run = parent.start_resources(Default::default()).unwrap();
    let ReferenceResourceProgress::AwaitResources(requests) = run.advance().unwrap() else {
        panic!()
    };
    let mut bytes = response();
    bytes
        .bytes
        .extend_from_slice(b"\n{extras @xmlns:p={#namespace} | {p:item}}");
    let loaded = run
        .complete_with_exports(requests[0].id, Ok(bytes), |imported, _| {
            Ok(imported
                .tree
                .ast()
                .nodes
                .iter()
                .filter_map(|n| match n {
                    CemAstNode::Element {
                        node_id,
                        expanded_name,
                        ..
                    } if expanded_name.local_name == "schema" => {
                        SchemaDeclarationNode::new(imported.tree.ast_owner().clone(), *node_id)
                    }
                    _ => None,
                })
                .collect())
        })
        .unwrap()
        .unwrap();
    let pending = run.query_snapshot(loaded.index).unwrap();
    assert!(!pending.report().complete);
    assert!(!pending.report().dependencies.is_empty());
    let result = pending
        .evaluate(
            "seq:where(input, fn(node) => node.kind == \"element\" && node.name == \"extras\")",
            &Default::default(),
        )
        .unwrap()
        .result;
    assert!(result.error.is_none());
    assert!(result.items.is_empty());
    run.set_loaded_context(loaded.index, Some(Default::default()))
        .unwrap();
    let saved = run.query_snapshot(loaded.index).unwrap();
    assert!(
        !saved.report().complete,
        "setting a context does not evaluate saved names"
    );
    drop(run);
    assert!(!pending.report().complete);
    assert!(pending
        .evaluate("input", &Default::default())
        .unwrap()
        .result
        .error
        .is_none());
}
