//! Namespace targets are explicit original declarations, independent of schemas.
use cem_ml::{
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    parser::CemAstNode,
    schema::{
        declaration_references::SchemaDeclarationNode,
        namespace_references::{admit_namespace_scope_target, NamespaceScopeTargetError},
        vocab::CompiledSchema,
    },
};
use std::sync::Arc;
fn import(text: &str, mime: &str) -> ScopedCemImport {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        mime,
        "vendor.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
}
fn targets(imported: &ScopedCemImport) -> Vec<SchemaDeclarationNode> {
    imported
        .tree
        .ast()
        .nodes
        .iter()
        .filter_map(|node| {
            let id = match node {
                CemAstNode::Element { node_id, .. } | CemAstNode::Attribute { node_id, .. } => {
                    *node_id
                }
                _ => return None,
            };
            imported
                .captured
                .namespace_binding(imported.tree.ast_owner(), id)?;
            SchemaDeclarationNode::new(imported.tree.ast_owner().clone(), id)
        })
        .collect()
}
#[test]
fn cem_directives_capture_original_bindings_and_default_alias_resets() {
    let imported = import(
        "@ns v = urn:first\n@default v\n{v:item}\n@ns v = urn:later\n@default \"\"\n{item}",
        "text/cem-ml",
    );
    let targets = targets(&imported);
    assert_eq!(targets.len(), 4);
    for (target, (prefix, uri)) in targets.into_iter().zip([
        ("v", "urn:first"),
        ("", "urn:first"),
        ("v", "urn:later"),
        ("", ""),
    ]) {
        let admitted = admit_namespace_scope_target(target.clone(), &imported.captured).unwrap();
        assert!(Arc::ptr_eq(
            admitted.selected.document(),
            imported.tree.ast_owner()
        ));
        assert_eq!(admitted.selected.node_id(), target.node_id());
        assert_eq!(admitted.binding().name, prefix);
        assert_eq!(admitted.namespace_uri(), uri);
    }
    let names: Vec<_> = imported
        .tree
        .ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "item" => Some(
                imported
                    .captured
                    .expanded_name(imported.tree.ast_owner(), *node_id)
                    .unwrap()
                    .namespace_uri
                    .as_str(),
            ),
            _ => None,
        })
        .collect();
    assert_eq!(names, ["urn:first", ""]);
}
#[test]
fn cem_namespace_attributes_keep_bindings_at_their_own_value_events() {
    let imported = import("{root @xmlns:v=urn:first @xmlns=urn:default | {v:item} {inner @xmlns:v=urn:inner @xmlns=\"\" | {v:item}} {v:item}}", "text/cem-ml");
    let values: Vec<_> = targets(&imported)
        .into_iter()
        .map(|target| {
            let admitted = admit_namespace_scope_target(target, &imported.captured).unwrap();
            (
                admitted.binding().name.clone(),
                admitted.namespace_uri().to_owned(),
            )
        })
        .collect();
    assert_eq!(
        values,
        [
            ("v".into(), "urn:first".into()),
            ("".into(), "urn:default".into()),
            ("v".into(), "urn:inner".into()),
            ("".into(), "".into())
        ]
    );
}
#[test]
fn xml_namespace_targets_keep_decoding_source_attributes_and_child_restoration() {
    let imported = import("<root xmlns:v='urn:a&amp;b' xmlns='urn:default'><v:item/><inner xmlns:v='urn:inner' xmlns=''><v:item/></inner><v:item/></root>", "application/xml");
    let targets = targets(&imported);
    assert_eq!(targets.len(), 4);
    for (target, expected) in targets
        .iter()
        .zip(["urn:a&b", "urn:default", "urn:inner", ""])
    {
        let admitted = admit_namespace_scope_target(target.clone(), &imported.captured).unwrap();
        assert_eq!(admitted.namespace_uri(), expected);
        assert!(matches!(
            admitted.selected.node(),
            CemAstNode::Attribute { .. }
        ));
    }
    let names: Vec<_> = imported
        .tree
        .ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element { expanded_name, .. } if expanded_name.local_name == "item" => {
                Some(expanded_name.namespace_uri.as_str())
            }
            _ => None,
        })
        .collect();
    // The source AST keeps its authored namespace spelling. Target admission
    // consumes the parser's decoded binding, without rewriting those names.
    assert_eq!(names, ["urn:a&amp;b", "urn:inner", "urn:a&amp;b"]);
}
#[test]
fn sibling_binding_ids_and_other_arena_ids_do_not_merge_original_declarations() {
    let imported = import(
        "{left @xmlns:v=urn:left}{right @xmlns:v=urn:right}",
        "text/cem-ml",
    );
    let nodes = targets(&imported);
    let left = admit_namespace_scope_target(nodes[0].clone(), &imported.captured).unwrap();
    let right = admit_namespace_scope_target(nodes[1].clone(), &imported.captured).unwrap();
    assert_eq!(left.binding().binding_id, right.binding().binding_id);
    assert_ne!(left.selected.node_id(), right.selected.node_id());
    assert_ne!(left.namespace_uri(), right.namespace_uri());
    let other = import(
        "{left @xmlns:v=urn:left}{right @xmlns:v=urn:right}",
        "text/cem-ml",
    );
    assert_eq!(
        admit_namespace_scope_target(nodes[0].clone(), &other.captured).unwrap_err(),
        NamespaceScopeTargetError::OwnerMismatch
    );
    assert!(imported
        .captured
        .namespace_binding(other.tree.ast_owner(), nodes[0].node_id())
        .is_none());
}
#[test]
fn admission_rejects_schema_data_and_uncompleted_declarations() {
    for source in [
        "{schema @uri=urn:pretend}",
        "{namespace @uri=urn:pretend}",
        "{data @name=v @value=urn:pretend}",
        "@ns broken\n{item}",
        "{root @xmlns:v={#later} | {item}}",
        "{root @xmlns:v | {item}}",
    ] {
        let imported = import(source, "text/cem-ml");
        assert!(targets(&imported).is_empty(), "{source}");
        for node in &imported.tree.ast().nodes {
            let id = match node {
                CemAstNode::Element { node_id, .. } | CemAstNode::Attribute { node_id, .. } => {
                    *node_id
                }
                _ => continue,
            };
            assert_eq!(
                admit_namespace_scope_target(
                    SchemaDeclarationNode::new(imported.tree.ast_owner().clone(), id).unwrap(),
                    &imported.captured
                )
                .unwrap_err(),
                NamespaceScopeTargetError::NotNamespaceDeclaration
            );
        }
        assert!(imported.tree.ast().nodes.iter().all(|node| !matches!(
            node,
            CemAstNode::Reference {
                targets: Some(_),
                ..
            }
        )));
    }
}

#[test]
fn given_root_bindings_resolve_names_without_synthesized_source_declarations() {
    use cem_ml::{
        events::cem::CemEventNormalizer,
        schema::machine::CemSchemaMachine,
        source::{BytesSource, SourceId},
        tokenizer::cem::CemTokenizer,
    };
    let source = "{v:item}{item}";
    let captured = CemSchemaMachine::new(
        CompiledSchema::cem_core(),
        CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
            SourceId(1),
            source.as_bytes().to_vec(),
        ))),
    )
    .with_root_namespace_bindings(
        Some("urn:default"),
        &std::collections::BTreeMap::from([("v".into(), "urn:runtime".into())]),
    )
    .build_with_lexical_scopes();
    let mut namespaces = vec![];
    for node in &captured.document().nodes {
        if let CemAstNode::Element {
            node_id,
            expanded_name,
            ..
        } = node
        {
            assert_eq!(expanded_name.local_name, "item");
            assert!(captured
                .namespace_binding(captured.document(), *node_id)
                .is_none());
            namespaces.push(
                captured
                    .expanded_name(captured.document(), *node_id)
                    .unwrap()
                    .namespace_uri
                    .as_str(),
            );
        }
    }
    assert_eq!(namespaces, ["urn:runtime", "urn:default"]);
}
