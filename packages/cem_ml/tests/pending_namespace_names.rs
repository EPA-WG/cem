use cem_ml::{
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    parser::CemAstNode,
    schema::{
        declaration_references::SchemaDeclarationNode,
        namespace_references::{
            admit_namespace_scope_target, NamespaceNameCompletion, NamespaceNameCompletionError,
        },
        vocab::CompiledSchema,
    },
};
use std::{collections::BTreeMap, sync::Arc};
fn import(text: &str) -> ScopedCemImport {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "source.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
}
fn elements(source: &ScopedCemImport, local: &str) -> Vec<u32> {
    source
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
fn declarations(source: &ScopedCemImport) -> Vec<u32> {
    source
        .tree
        .ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Attribute {
                node_id,
                expanded_name,
                value_nodes,
                ..
            } if expanded_name.namespace_uri == "xmlns" && !value_nodes.is_empty() => {
                Some(*node_id)
            }
            _ => None,
        })
        .collect()
}
fn completion(uri: &str) -> cem_ml::schema::namespace_references::NamespaceScopeTarget {
    let (vendor, directive) = if uri.is_empty() {
        (import("@default \"\"\n{item}"), "@default")
    } else {
        (
            import(&format!("@ns public = {uri}\n{{public:item}}")),
            "@ns",
        )
    };
    let id = elements(&vendor, directive)[0];
    admit_namespace_scope_target(
        SchemaDeclarationNode::new(vendor.tree.ast_owner().clone(), id).unwrap(),
        &vendor.captured,
    )
    .unwrap()
}
#[test]
fn native_declarations_mask_inherited_uri_and_keep_uses_at_their_source_position() {
    let source = import("@ns v = urn:outer\n{host @xmlns:v={#library} | {v:first @v:flag=yes} {#later} {inner @xmlns:v=urn:later | {v:second}}}\n{v:outside}");
    let declaration = declarations(&source)[0];
    let first = elements(&source, "first")[0];
    let pending = source
        .captured
        .pending_namespace_name(source.tree.ast_owner(), first)
        .unwrap();
    assert_eq!(pending.declaration, declaration);
    assert_eq!(pending.local_name, "first");
    assert!(source
        .captured
        .expanded_name(source.tree.ast_owner(), first)
        .is_none());
    let attr = match source.tree.ast().get(first).unwrap() {
        CemAstNode::Element { attributes, .. } => attributes[0],
        _ => unreachable!(),
    };
    assert_eq!(
        source
            .captured
            .pending_namespace_name(source.tree.ast_owner(), attr)
            .unwrap()
            .declaration,
        declaration
    );
    for (local, uri) in [("second", "urn:later"), ("outside", "urn:outer")] {
        assert_eq!(
            source
                .captured
                .expanded_name(source.tree.ast_owner(), elements(&source, local)[0])
                .unwrap()
                .namespace_uri,
            uri
        );
    }
    let later = source
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Reference {
                node_id,
                expression,
                ..
            } if expression.contains("later") => Some(*node_id),
            _ => None,
        })
        .unwrap();
    assert!(source
        .captured
        .snapshot(source.tree.ast_owner(), later)
        .unwrap()
        .namespaces
        .binding("v")
        .is_none());
    assert_eq!(
        source
            .captured
            .pending_namespace_bindings(source.tree.ast_owner(), later)
            .unwrap()["v"],
        declaration
    );
}
#[test]
fn completion_is_independent_and_requires_only_selected_namespace_dependencies() {
    let source = import("{host @xmlns:v={#library} | {v:item} {v:other}} {plain}");
    let declaration = declarations(&source)[0];
    let item = elements(&source, "item")[0];
    assert!(
        matches!(NamespaceNameCompletion::new(source.captured.clone(), &[item], BTreeMap::new()), Err(NamespaceNameCompletionError::Pending { declaration: id, .. }) if id == declaration)
    );
    let plain = elements(&source, "plain")[0];
    assert!(
        NamespaceNameCompletion::new(source.captured.clone(), &[plain], BTreeMap::new()).is_ok()
    );
    let first = NamespaceNameCompletion::new(
        source.captured.clone(),
        &[item],
        BTreeMap::from([(declaration, completion("urn:one"))]),
    )
    .unwrap();
    let second = NamespaceNameCompletion::new(
        source.captured.clone(),
        &[item],
        BTreeMap::from([(declaration, completion("urn:two"))]),
    )
    .unwrap();
    assert_eq!(first.expanded_name(item).unwrap().namespace_uri, "urn:one");
    assert_eq!(second.expanded_name(item).unwrap().namespace_uri, "urn:two");
    let reset = NamespaceNameCompletion::new(
        source.captured.clone(),
        &[item],
        BTreeMap::from([(declaration, completion(""))]),
    )
    .unwrap();
    assert_eq!(reset.expanded_name(item).unwrap().namespace_uri, "");
    assert!(!first.contains(elements(&source, "other")[0]));
    assert!(Arc::ptr_eq(
        first.captured().document(),
        source.tree.ast_owner()
    ));
    assert!(source
        .captured
        .expanded_name(source.tree.ast_owner(), item)
        .is_none());
    assert!(source
        .tree
        .ast()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}
#[test]
fn pending_default_aliases_and_child_restoration_use_original_declaration_identity() {
    // A normalized producer exercises block-prelude semantics without claiming
    // that the inline CEM body parser recognizes document-level directives.
    use cem_ml::{
        events::{cem::CemEventNormalizer, EventNormalizer, NormalizedEvent},
        parser::tree::{CemTreeSemantics, RetainedCemTree},
        schema::machine::CemSchemaMachine,
        source::{BytesSource, SourceId},
        tokenizer::cem::CemTokenizer,
    };
    struct Events(std::vec::IntoIter<NormalizedEvent>);
    impl EventNormalizer for Events {
        fn next_event(&mut self) -> Option<NormalizedEvent> {
            self.0.next()
        }
    }
    fn events(text: &str) -> Vec<NormalizedEvent> {
        let mut source = CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
            SourceId(1),
            text.as_bytes().to_vec(),
        )));
        std::iter::from_fn(|| source.next_event()).collect()
    }
    let mut stream = Vec::new();
    for event in events("{host @xmlns:v={#library} | {first | {#later}} {inner @xmlns:v=urn:inner | {second | {#later}}} {third | {#later}} {plain | {#later}}} {outside}") {
        if let NormalizedEvent::OpenScope { name, .. } = &event {
            match name.lexical_name.as_str() {
                "first" | "second" => stream.extend(events("@default v\n")),
                "plain" => stream.extend(events("@default \"\"\n")), _ => {}
            }
        }
        stream.push(event);
    }
    let captured = Arc::new(
        CemSchemaMachine::new(CompiledSchema::cem_core(), Events(stream.into_iter()))
            .build_with_lexical_scopes(),
    );
    let tree = RetainedCemTree::from_shared(
        captured.document().clone(),
        "events.cem",
        "",
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    let source = ScopedCemImport { captured, tree };
    let declaration = declarations(&source)[0];
    let roots = [elements(&source, "host")[0]];
    let completed = NamespaceNameCompletion::new(
        source.captured.clone(),
        &roots,
        BTreeMap::from([(declaration, completion("urn:chosen"))]),
    )
    .unwrap();
    for (local, uri) in [
        ("first", "urn:chosen"),
        ("second", "urn:inner"),
        ("third", "urn:chosen"),
        ("plain", ""),
    ] {
        assert_eq!(
            completed
                .expanded_name(elements(&source, local)[0])
                .unwrap()
                .namespace_uri,
            uri
        );
    }
    let occurrences = source.captured.occurrences().filter(|id| matches!(
        source.tree.ast().get(*id), Some(CemAstNode::Reference {expression, ..}) if expression == "#later"
    )).collect::<Vec<_>>();
    assert_eq!(occurrences.len(), 4);
    for (id, uri) in occurrences
        .iter()
        .zip(["urn:chosen", "urn:inner", "urn:chosen", ""])
    {
        let occurrence = SchemaDeclarationNode::new(source.tree.ast_owner().clone(), *id).unwrap();
        let snapshot = completed.lexical_snapshot(&occurrence).unwrap();
        assert_eq!(snapshot.namespace_uri(""), Some(uri));
    }
    let default_alias = elements(&source, "@default")[0];
    let alias = SchemaDeclarationNode::new(source.tree.ast_owner().clone(), default_alias).unwrap();
    assert_eq!(
        completed.binding_namespace_uri(&alias).unwrap(),
        "urn:chosen"
    );
    let first_occurrence =
        SchemaDeclarationNode::new(source.tree.ast_owner().clone(), occurrences[0]).unwrap();
    let snapshot = completed.lexical_snapshot(&first_occurrence).unwrap();
    assert_eq!(
        snapshot.completed_bindings()[""].declaration.node_id(),
        default_alias
    );
    assert!(snapshot.original().namespaces.binding("").is_none());

    let unrelated = NamespaceNameCompletion::new(
        source.captured.clone(),
        &[elements(&source, "outside")[0]],
        BTreeMap::new(),
    )
    .unwrap();
    assert!(matches!(
        unrelated.binding_namespace_uri(&alias),
        Err(NamespaceNameCompletionError::Pending { node, declaration: id })
            if node == default_alias && id == declaration
    ));
}

#[test]
fn binding_lookup_distinguishes_pending_dependencies_from_ready_selected_names() {
    let source = import("@ns v = urn:outer\n{host @xmlns:v={#library} | {#later}}\n@ns v = urn:later\n@default \"\"\n{plain}");
    let declaration = declarations(&source)[0];
    let pending = SchemaDeclarationNode::new(source.tree.ast_owner().clone(), declaration).unwrap();
    let plain = elements(&source, "plain")[0];
    let names =
        NamespaceNameCompletion::new(source.captured.clone(), &[plain], BTreeMap::new()).unwrap();
    assert!(!names.contains(declaration));
    assert!(
        matches!(names.binding_namespace_uri(&pending), Err(NamespaceNameCompletionError::Pending { node, declaration: id }) if node == declaration && id == declaration)
    );
    for (id, uri) in elements(&source, "@ns")
        .into_iter()
        .zip(["urn:outer", "urn:later"])
    {
        let literal = SchemaDeclarationNode::new(source.tree.ast_owner().clone(), id).unwrap();
        assert_eq!(names.binding_namespace_uri(&literal).unwrap(), uri);
    }
    let reset = SchemaDeclarationNode::new(
        source.tree.ast_owner().clone(),
        elements(&source, "@default")[0],
    )
    .unwrap();
    assert_eq!(names.binding_namespace_uri(&reset).unwrap(), "");
    let chosen = NamespaceNameCompletion::new(
        source.captured.clone(),
        &[plain],
        BTreeMap::from([(declaration, completion("urn:chosen"))]),
    )
    .unwrap();
    assert_eq!(
        chosen.binding_namespace_uri(&pending).unwrap(),
        "urn:chosen"
    );
    assert!(source
        .captured
        .namespace_binding(source.tree.ast_owner(), declaration)
        .is_none());
}

#[test]
fn binding_lookup_checks_original_owner_and_does_not_extract_namespace_from_nodes() {
    let source = import("@ns v = urn:chosen\n{v:item}");
    let names = NamespaceNameCompletion::new(
        source.captured.clone(),
        &[elements(&source, "item")[0]],
        BTreeMap::new(),
    )
    .unwrap();
    let item = SchemaDeclarationNode::new(
        source.tree.ast_owner().clone(),
        elements(&source, "item")[0],
    )
    .unwrap();
    assert_eq!(
        names.binding_namespace_uri(&item),
        Err(NamespaceNameCompletionError::InvalidDeclaration(
            item.node_id()
        ))
    );
    let foreign = import("@ns v = urn:chosen\n{v:item}");
    let declaration = SchemaDeclarationNode::new(
        foreign.tree.ast_owner().clone(),
        elements(&foreign, "@ns")[0],
    )
    .unwrap();
    assert_eq!(
        names.binding_namespace_uri(&declaration),
        Err(NamespaceNameCompletionError::OwnerMismatch)
    );
}
#[test]
fn completion_rejects_bad_roots_wrong_declarations_and_unbound_names() {
    let source = import("{host @xmlns:v={#library} | {v:item}} {missing:item}");
    let item = elements(&source, "item")[0];
    assert!(matches!(
        NamespaceNameCompletion::new(source.captured.clone(), &[u32::MAX], BTreeMap::new()),
        Err(NamespaceNameCompletionError::InvalidRoot(_))
    ));
    assert!(matches!(
        NamespaceNameCompletion::new(source.captured.clone(), &[item, item], BTreeMap::new()),
        Err(NamespaceNameCompletionError::OverlappingRoots(_))
    ));
    assert!(matches!(
        NamespaceNameCompletion::new(
            source.captured.clone(),
            &[item],
            BTreeMap::from([(item, completion("urn:one"))])
        ),
        Err(NamespaceNameCompletionError::InvalidDeclaration(_))
    ));
    let unbound = *elements(&source, "item").last().unwrap();
    assert!(matches!(
        NamespaceNameCompletion::new(source.captured.clone(), &[unbound], BTreeMap::new()),
        Err(NamespaceNameCompletionError::UnboundName(_))
    ));
    let other = import("{host @xmlns:v={#library} | {v:item}}");
    assert!(source
        .captured
        .pending_namespace_name(other.tree.ast_owner(), item)
        .is_none());
}

#[test]
fn completed_occurrence_bindings_require_unused_dependencies_and_preserve_original_snapshots() {
    let source = import("@ns v = urn:outer\n{host @xmlns:v={#library} | {#later} {inner @xmlns:v=urn:inner | {#later}} {#later}} {#outside}");
    let declaration = declarations(&source)[0];
    let host = elements(&source, "host")[0];
    let occurrences = source.captured.occurrences().collect::<Vec<_>>();
    assert_eq!(occurrences.len(), 5);
    let handle = |id| SchemaDeclarationNode::new(source.tree.ast_owner().clone(), id).unwrap();
    let pending =
        NamespaceNameCompletion::new(source.captured.clone(), &[host], BTreeMap::new()).unwrap();
    // The property's own selector still has the pre-declaration binding.
    assert_eq!(
        pending
            .lexical_snapshot(&handle(occurrences[0]))
            .unwrap()
            .namespace_uri("v"),
        Some("urn:outer")
    );
    assert!(
        matches!(pending.lexical_snapshot(&handle(occurrences[1])), Err(NamespaceNameCompletionError::Pending {node, declaration: dep}) if node == occurrences[1] && dep == declaration)
    );
    assert_eq!(
        pending
            .lexical_snapshot(&handle(occurrences[2]))
            .unwrap()
            .namespace_uri("v"),
        Some("urn:inner")
    );
    assert!(
        matches!(pending.lexical_snapshot(&handle(occurrences[4])), Err(NamespaceNameCompletionError::OutsideSelection(id)) if id == occurrences[4])
    );
    assert!(
        matches!(pending.lexical_snapshot(&handle(host)), Err(NamespaceNameCompletionError::InvalidOccurrence(id)) if id == host)
    );
    for uri in ["urn:one", "urn:two", ""] {
        let names = NamespaceNameCompletion::new(
            source.captured.clone(),
            &[host],
            BTreeMap::from([(declaration, completion(uri))]),
        )
        .unwrap();
        for id in [occurrences[1], occurrences[3]] {
            let snapshot = names.lexical_snapshot(&handle(id)).unwrap();
            assert_eq!(snapshot.namespace_uri("v"), Some(uri));
            assert!(snapshot.original().namespaces.binding("v").is_none());
            let binding = &snapshot.completed_bindings()["v"];
            assert_eq!(binding.declaration.node_id(), declaration);
            assert!(Arc::ptr_eq(
                binding.declaration.document(),
                source.tree.ast_owner()
            ));
            assert_eq!(snapshot.namespace_uri("unknown"), None);
        }
    }
    let foreign = import("{#later}");
    let foreign_node = SchemaDeclarationNode::new(
        foreign.tree.ast_owner().clone(),
        foreign.captured.occurrences().next().unwrap(),
    )
    .unwrap();
    assert!(matches!(
        pending.lexical_snapshot(&foreign_node),
        Err(NamespaceNameCompletionError::OwnerMismatch)
    ));
    assert!(source
        .captured
        .snapshot(source.tree.ast_owner(), occurrences[1])
        .unwrap()
        .namespaces
        .binding("v")
        .is_none());
}

#[test]
fn general_expression_waits_for_every_original_pending_prefix() {
    let source = import("{host @xmlns:v={#one} @xmlns:w={#two} @target={library} | }");
    let declarations = declarations(&source);
    assert_eq!(declarations.len(), 2);
    let occurrence = *source
        .captured
        .occurrences()
        .collect::<Vec<_>>()
        .last()
        .unwrap();
    let original = SchemaDeclarationNode::new(source.tree.ast_owner().clone(), occurrence).unwrap();
    assert!(
        matches!(original.node(), CemAstNode::Element {expanded_name, ..} if expanded_name.local_name == "$")
    );
    let root = elements(&source, "host")[0];
    let first = NamespaceNameCompletion::new(
        source.captured.clone(),
        &[root],
        BTreeMap::from([(declarations[0], completion("urn:one"))]),
    )
    .unwrap();
    assert!(
        matches!(first.lexical_snapshot(&original), Err(NamespaceNameCompletionError::Pending {node, declaration}) if node == occurrence && declaration == declarations[1])
    );
    let ready = NamespaceNameCompletion::new(
        source.captured.clone(),
        &[root],
        BTreeMap::from([
            (declarations[0], completion("urn:one")),
            (declarations[1], completion("urn:two")),
        ]),
    )
    .unwrap();
    let snapshot = ready.lexical_snapshot(&original).unwrap();
    assert_eq!(snapshot.namespace_uri("v"), Some("urn:one"));
    assert_eq!(snapshot.namespace_uri("w"), Some("urn:two"));
    assert_eq!(snapshot.completed_bindings().len(), 2);
}
