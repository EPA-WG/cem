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
    for event in events("{host @xmlns:v={#library} | {first} {inner @xmlns:v=urn:inner | {second}} {third} {plain}} {outside}") {
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
