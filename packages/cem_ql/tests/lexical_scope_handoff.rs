use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::{
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode,
    },
    schema::{
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        machine::{CemSchemaMachine, LexicallyScopedDocument},
        reference_policy::ReferenceScopePolicy,
        scoping::SchemaSource,
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
    value::reference_resolution::{
        resolve_reference, ReferenceResolutionIssueKind, ReferenceResolutionState,
    },
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    schema_references::{CemQlSchemaDeclarationHost, LexicalScopeHandoffError},
};
use std::sync::Arc;

fn captured(text: &str) -> (LexicallyScopedDocument, Arc<RetainedCemTree>) {
    let events = CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
        SourceId(1),
        text.as_bytes().to_vec(),
    )));
    let captured =
        CemSchemaMachine::new(CompiledSchema::cem_core(), events).build_with_lexical_scopes();
    let tree = RetainedCemTree::from_shared(
        captured.document().clone(),
        "scope.cem",
        text,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    assert!(Arc::ptr_eq(captured.document(), tree.ast_owner()));
    (captured, tree)
}
fn policy() -> ReferenceScopePolicy {
    ReferenceScopePolicy::schema_defaults().unwrap()
}
fn context(tree: &Arc<RetainedCemTree>, target: u32) -> StandaloneExpressionContext {
    StandaloneExpressionContext::default().with_binding(
        "library",
        StandaloneExpressionBinding::any(ItemStream::from_items(vec![RetainedCemNode::new(
            tree.clone(),
            target,
        )
        .unwrap()
        .query_item()])),
    )
}
fn elements(tree: &Arc<RetainedCemTree>, name: &str) -> Vec<u32> {
    tree.ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => Some(*node_id),
            _ => None,
        })
        .collect()
}
fn source(captured: &LexicallyScopedDocument, node: u32) -> SchemaDeclarationNode {
    SchemaDeclarationNode::new(captured.document().clone(), node).unwrap()
}

#[test]
fn saved_bindings_prepare_distinct_lifecycle_contexts_and_pending_selectors_without_evaluation() {
    let (captured, tree) = captured("@ns v = urn:first\n{outer @target={#library} | {#library} {target}}\n@ns v = urn:second\n{inner @cem:schema-select='missing()' | {#library} {target}}");
    let targets = elements(&tree, "target");
    let refs: Vec<_> = captured.occurrences().collect();
    assert_eq!(refs.len(), 3);
    let mut host = CemQlSchemaDeclarationHost::new();
    let parent = host.register_scope(tree.clone(), None, policy());
    let attached = host
        .attach_captured_lexical_scopes(&captured, |node, snapshot, inherited| {
            assert_eq!(inherited, parent);
            assert!(matches!(
                node.node(),
                CemAstNode::Reference { targets: None, .. }
            ));
            if snapshot.namespaces.binding("v").unwrap().namespace_uri == "urn:first" {
                (Some(context(&tree, targets[0])), policy())
            } else {
                assert_eq!(
                    snapshot.schema.active,
                    SchemaSource::Select("missing()".into())
                );
                (None, policy())
            }
        })
        .unwrap();
    for node in &refs {
        assert!(host
            .compiled_source_expression(&source(&captured, *node))
            .is_none());
    }
    for node in &refs[..2] {
        let input = host.source_reference(source(&captured, *node));
        let resolved = resolve_reference(input, &mut host, policy().limits).unwrap();
        assert!(resolved.is_complete(), "{:?}", resolved.issues);
        let target = host.declaration_node(&resolved.nodes[0]).unwrap();
        assert_eq!(target.node_id(), targets[0]);
        assert!(Arc::ptr_eq(target.document(), captured.document()));
    }
    let first_artifact = host
        .compiled_source_expression(&source(&captured, refs[0]))
        .unwrap();
    let pending = host.source_reference(source(&captured, refs[2]));
    assert_eq!(
        resolve_reference(pending.clone(), &mut host, policy().limits)
            .unwrap()
            .state,
        ReferenceResolutionState::Pending
    );
    assert!(host.set_context(attached[2].1, Some(context(&tree, targets[1]))));
    let result = resolve_reference(pending, &mut host, policy().limits).unwrap();
    assert!(result.is_complete(), "{:?}", result.issues);
    assert_eq!(
        host.declaration_node(&result.nodes[0]).unwrap().node_id(),
        targets[1]
    );
    assert!(Arc::ptr_eq(
        &first_artifact,
        &host
            .compiled_source_expression(&source(&captured, refs[0]))
            .unwrap()
    ));
    assert!(tree.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
    // A second execution shares source and snapshots, never the first target list.
    let mut other = CemQlSchemaDeclarationHost::new();
    other.register_scope(tree.clone(), None, policy());
    other
        .attach_captured_lexical_scopes(&captured, |_, _, _| {
            (Some(context(&tree, targets[1])), policy())
        })
        .unwrap();
    let input = other.source_reference(source(&captured, refs[0]));
    let result = resolve_reference(input, &mut other, policy().limits).unwrap();
    assert_eq!(
        other.declaration_node(&result.nodes[0]).unwrap().node_id(),
        targets[1]
    );
}

#[test]
fn captured_handoff_keeps_explicit_boundaries_and_rejects_foreign_or_repeat_attachment() {
    let text = "{outer | {#library} {target}} {inner | {#library}}";
    let (captured, tree) = captured(text);
    let (foreign, _) = self::captured(text);
    let refs: Vec<_> = captured.occurrences().collect();
    let target = elements(&tree, "target")[0];
    let inner = elements(&tree, "inner")[0];
    let mut host = CemQlSchemaDeclarationHost::new();
    let outer = host.register_scope(tree.clone(), None, policy());
    let explicit = host.register_scope(tree.clone(), None, policy());
    assert!(host.assign_subtree_scope(&tree, inner, explicit));
    assert_eq!(
        host.attach_captured_lexical_scopes(&foreign, |_, _, _| panic!("foreign preparation")),
        Err(LexicalScopeHandoffError::UnregisteredOwner)
    );
    // A conflict at a later occurrence must reject before preparing the first.
    let mut conflicting = host.clone();
    assert!(conflicting.assign_subtree_scope(&tree, refs[1], explicit));
    assert_eq!(
        conflicting
            .attach_captured_lexical_scopes(&captured, |_, _, _| panic!("partial preparation")),
        Err(LexicalScopeHandoffError::OccurrenceAlreadyAssigned(refs[1]))
    );
    let unchanged = conflicting.source_reference(source(&captured, refs[0]));
    assert_eq!(
        resolve_reference(unchanged, &mut conflicting, policy().limits)
            .unwrap()
            .state,
        ReferenceResolutionState::Pending
    );
    let attached = host
        .attach_captured_lexical_scopes(&captured, |node, _, inherited| {
            assert_eq!(
                inherited,
                if node.node_id() == refs[0] {
                    outer
                } else {
                    explicit
                }
            );
            (Some(context(&tree, target)), policy())
        })
        .unwrap();
    assert_eq!(
        host.attach_captured_lexical_scopes(&captured, |_, _, _| panic!("repeat preparation")),
        Err(LexicalScopeHandoffError::OccurrenceAlreadyAssigned(refs[0]))
    );
    let first = host.source_reference(source(&captured, refs[0]));
    assert!(resolve_reference(first, &mut host, policy().limits)
        .unwrap()
        .is_complete());
    let second = host.source_reference(source(&captured, refs[1]));
    let denied = resolve_reference(second.clone(), &mut host, policy().limits).unwrap();
    assert!(denied
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::ScopeDenied));
    assert!(host.allow_scope_crossing(outer, explicit));
    assert!(
        !resolve_reference(second.clone(), &mut host, policy().limits)
            .unwrap()
            .is_complete()
    );
    assert!(host.allow_scope_crossing(attached[1].1, outer));
    assert!(resolve_reference(second, &mut host, policy().limits)
        .unwrap()
        .is_complete());
}

#[test]
fn captured_chain_applies_destination_and_request_limits_without_resetting_work() {
    let (captured, tree) = captured("{outer | {#library}} {inner | {#library}} {target}");
    let refs: Vec<_> = captured.occurrences().collect();
    let target = elements(&tree, "target")[0];
    for (destination_work, request_work) in
        [(1, policy().limits.max_work), (policy().limits.max_work, 2)]
    {
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(tree.clone(), None, policy());
        host.attach_captured_lexical_scopes(&captured, |node, _, _| {
            let mut effective = policy();
            let target = if node.node_id() == refs[0] {
                refs[1]
            } else {
                effective.limits.max_work = destination_work;
                target
            };
            (Some(context(&tree, target)), effective)
        })
        .unwrap();
        let input = host.source_reference(source(&captured, refs[0]));
        let mut limits = policy().limits;
        limits.max_work = request_work;
        let result = resolve_reference(input, &mut host, limits).unwrap();
        assert!(!result.is_complete());
        assert!(result.work_used <= request_work);
        assert!(result
            .issues
            .iter()
            .any(|issue| issue.kind == ReferenceResolutionIssueKind::WorkLimit));
        assert!(!result
            .issues
            .iter()
            .any(|issue| issue.kind == ReferenceResolutionIssueKind::ScopeDenied));
    }
}

#[test]
fn captured_general_attribute_expression_waits_for_its_explicit_input_lifecycle_hook() {
    use cem_ml::value::reference_resolution::ReferenceLinkEvaluation;
    let (captured, tree) = captured("{outer @target={library} | {target}}");
    let expression = captured.occurrences().next().unwrap();
    let target = elements(&tree, "target")[0];
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(tree.clone(), None, policy());
    let attached = host
        .attach_captured_lexical_scopes(&captured, |node, _, _| {
            assert!(
                matches!(node.node(), CemAstNode::Element { expanded_name, .. }
            if expanded_name.local_name == "$")
            );
            (None, policy())
        })
        .unwrap();
    let original = source(&captured, expression);
    assert!(host.compiled_source_expression(&original).is_none());
    let input = host.source_reference(original.clone());
    assert!(matches!(
        host.evaluate_input_expression(&input),
        ReferenceLinkEvaluation::Pending(_)
    ));
    assert!(host.set_context(attached[0].1, Some(context(&tree, target))));
    assert!(host.compiled_source_expression(&original).is_none());
    let ReferenceLinkEvaluation::Resolved(nodes) = host.evaluate_input_expression(&input) else {
        panic!("ready general attribute expression did not select native nodes");
    };
    assert_eq!(nodes.len(), 1);
    let selected = host.declaration_node(&nodes[0]).unwrap();
    assert_eq!(selected.node_id(), target);
    assert!(Arc::ptr_eq(selected.document(), captured.document()));
    assert!(host.compiled_source_expression(&original).is_some());
    assert!(
        matches!(original.node(), CemAstNode::Element { expanded_name, .. }
        if expanded_name.local_name == "$")
    );
}

#[test]
fn imported_xml_capture_hands_original_aliases_and_payload_sources_to_the_same_lifecycle() {
    use cem_ml::{
        import::import_xml_ast_with_lexical_scopes,
        validation::xml::{xml_document_ast_from_source_bytes, XmlSourceValidationRequest},
    };
    let xml = "<root xmlns:r='https://cem.dev/ns/cem-ml/1' r:schema-src='ready'><r:expr>#lib&#114;<![CDATA[ary]]></r:expr><r:schema select='missing()'><r:expr>#library</r:expr></r:schema></root>";
    let (document, _) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: xml.as_bytes(),
        source_uri: "scope.xml",
        content_type: Some("application/xml"),
    });
    let imported =
        import_xml_ast_with_lexical_scopes(&document.unwrap(), CompiledSchema::cem_core()).unwrap();
    let captured = imported.captured;
    let tree = RetainedCemTree::from_shared(
        captured.document().clone(),
        "scope.xml",
        xml,
        imported.semantics,
        None,
    )
    .unwrap();
    let (_, library) = self::captured("{target}");
    let target = elements(&library, "target")[0];
    let refs = captured.occurrences().collect::<Vec<_>>();
    assert_eq!(refs.len(), 2);
    let mut host = CemQlSchemaDeclarationHost::new();
    let parent = host.register_scope(tree.clone(), None, policy());
    let destination = host.register_scope(library.clone(), None, policy());
    let attached = host.attach_captured_lexical_scopes(&captured, |node, snapshot, inherited| {
        assert_eq!(inherited, parent);
        assert_eq!(snapshot.namespaces.binding("r").unwrap().namespace_uri, "https://cem.dev/ns/cem-ml/1");
        assert!(matches!(node.node(), CemAstNode::Reference {expression, targets: None, ..} if expression == "#library"));
        let ready = matches!(snapshot.schema.active, SchemaSource::Uri(ref uri) if uri == "ready");
        (ready.then(|| context(&library, target)), policy())
    }).unwrap();
    assert!(host
        .compiled_source_expression(&source(&captured, refs[0]))
        .is_none());
    let first = host.source_reference(source(&captured, refs[0]));
    assert!(resolve_reference(first.clone(), &mut host, policy().limits)
        .unwrap()
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::ScopeDenied));
    assert!(host.allow_scope_crossing(attached[0].1, destination));
    let resolved = resolve_reference(first, &mut host, policy().limits).unwrap();
    assert!(resolved.is_complete(), "{:?}", resolved.issues);
    let selected = host.declaration_node(&resolved.nodes[0]).unwrap();
    assert!(Arc::ptr_eq(selected.document(), library.ast_owner()));
    let second = host.source_reference(source(&captured, refs[1]));
    assert_eq!(
        resolve_reference(second.clone(), &mut host, policy().limits)
            .unwrap()
            .state,
        ReferenceResolutionState::Pending
    );
    assert!(host.set_context(attached[1].1, Some(context(&library, target))));
    assert!(resolve_reference(second, &mut host, policy().limits)
        .unwrap()
        .is_complete());
    let CemAstNode::Reference {
        source: provenance,
        targets,
        ..
    } = captured.document().get(refs[0]).unwrap()
    else {
        unreachable!()
    };
    assert!(targets.is_none());
    assert!(
        provenance
            .frames
            .iter()
            .filter(|frame| matches!(
                frame.transform,
                cem_ml::source_map::TransformKind::ExpressionEmbedding { .. }
            ))
            .count()
            >= 3
    );
    assert!(Arc::ptr_eq(tree.ast_owner(), captured.document()));
}
