use cem_ml::{
    events::cem::CemEventNormalizer,
    import::import_xml_ast_with_lexical_scopes,
    parser::CemAstNode,
    schema::{
        machine::{CemSchemaMachine, LexicallyScopedDocument},
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
    validation::xml::{xml_document_ast_from_source_bytes, XmlSourceValidationRequest},
};
use std::sync::Arc;

fn cem(text: &str) -> LexicallyScopedDocument {
    let events = CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
        SourceId(17),
        text.as_bytes().to_vec(),
    )));
    CemSchemaMachine::new(CompiledSchema::cem_core(), events).build_with_lexical_scopes()
}
fn xml(text: &str) -> LexicallyScopedDocument {
    let (document, diagnostics) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: text.as_bytes(),
        source_uri: "bindings.xml",
        content_type: Some("application/xml"),
    });
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    import_xml_ast_with_lexical_scopes(&document.unwrap(), CompiledSchema::cem_core())
        .unwrap()
        .captured
}
fn verify(
    captured: &LexicallyScopedDocument,
    foreign: &LexicallyScopedDocument,
    schema_namespace: &str,
) {
    let owner = captured.document();
    assert!(owner.diagnostics.is_empty(), "{:?}", owner.diagnostics);
    let refs: Vec<_> = captured.occurrences().collect();
    let expected = [
        None,
        None,
        Some("outer"),
        Some("outer"),
        Some("outer"),
        Some("inner"),
        Some("outer"),
        Some("later"),
    ];
    assert_eq!(refs.len(), expected.len());
    let mut identities = Vec::new();
    for (node, marker) in refs.iter().zip(expected) {
        assert!(matches!(
            owner.get(*node),
            Some(CemAstNode::Reference { targets: None, .. })
        ));
        assert!(captured
            .inline_schema(foreign.document(), *node, "shared")
            .is_none());
        assert!(captured.inline_schema(owner, *node, "unknown").is_none());
        let handle = captured.inline_schema(owner, *node, "shared");
        if let Some(marker) = marker {
            let handle = handle.expect("completed visible declaration");
            assert!(Arc::ptr_eq(handle.document(), owner));
            let CemAstNode::Element {
                expanded_name,
                children,
                ..
            } = handle.node()
            else {
                panic!("native declaration element")
            };
            assert_eq!(expanded_name.namespace_uri, schema_namespace);
            assert_eq!(expanded_name.local_name, "schema");
            assert!(children.iter().any(|child| matches!(owner.get(*child), Some(CemAstNode::Element { expanded_name, .. }) if expanded_name.local_name == marker)));
            let metadata = captured
                .snapshot(owner, *node)
                .unwrap()
                .schema
                .resolve_name("shared")
                .unwrap();
            assert_eq!(metadata.source_node, Some(handle.node_id()));
            assert!(!metadata.source_map.frames.is_empty());
            identities.push(Some(handle.identity()));
        } else {
            assert!(
                handle.is_none(),
                "declaration is not visible before closure"
            );
            identities.push(None);
        }
    }
    assert_eq!(identities[2], identities[3]);
    assert_eq!(identities[2], identities[4]);
    assert_eq!(identities[2], identities[6]);
    assert_ne!(identities[2], identities[5]);
    assert_ne!(identities[2], identities[7]);
    let outer = captured.inline_schema(owner, refs[2], "shared").unwrap();
    assert!(captured
        .inline_schema(owner, outer.node_id(), "shared")
        .is_none());
    let other = foreign
        .inline_schema(foreign.document(), refs[2], "shared")
        .unwrap();
    assert_eq!(outer.node_id(), other.node_id());
    assert_ne!(outer.identity(), other.identity());
}
#[test]
fn cem_inline_bindings_retain_original_nodes_across_closure_shadowing_and_later_replacement() {
    let text = "{section | {#before} {cem:schema @cem:name=shared | {outer} {#during}} {#outer} {section | {#inherited} {cem:schema @cem:name=shared | {inner} {#during}} {#inner}} {#restored} {cem:schema @cem:name=shared | {later}} {#later}}";
    verify(&cem(text), &cem(text), "cem");
}
#[test]
fn xml_inline_bindings_retain_original_nodes_across_closure_shadowing_and_later_replacement() {
    let text = "<root xmlns:r='https://cem.dev/ns/cem-ml/1' xmlns:cem='urn:foreign'><r:expr>#before</r:expr><r:schema r:name='shared'><outer/><r:expr>#during</r:expr></r:schema><r:expr>#outer</r:expr><section><r:expr>#inherited</r:expr><r:schema r:name='shared'><inner/><r:expr>#during</r:expr></r:schema><r:expr>#inner</r:expr></section><r:expr>#restored</r:expr><cem:schema r:name='shared'/><r:schema r:name='shared'><later/></r:schema><r:expr>#later</r:expr></root>";
    verify(&xml(text), &xml(text), "https://cem.dev/ns/cem-ml/1");
}

#[test]
fn empty_declarations_have_original_handles_only_when_an_owner_is_captured() {
    for captured in [
        cem("{cem:schema @cem:name=shared} {#after}"),
        xml("<root xmlns:r='https://cem.dev/ns/cem-ml/1'><r:schema r:name='shared'/><r:expr>#after</r:expr></root>"),
    ] {
        let occurrence = captured.occurrences().next().unwrap();
        let handle = captured.inline_schema(captured.document(), occurrence, "shared").unwrap();
        assert!(Arc::ptr_eq(handle.document(), captured.document()));
        assert!(matches!(handle.node(), CemAstNode::Element { expanded_name, children, .. } if expanded_name.local_name == "schema" && children.is_empty()));
    }
    let text = "{cem:schema @cem:name=shared} {#after}";
    let events = CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
        SourceId(17),
        text.as_bytes().to_vec(),
    )));
    let mut observed = None;
    CemSchemaMachine::new(CompiledSchema::cem_core(), events).run_with_observer(|machine| {
        if let Some(declaration) = machine.lexical_snapshot().schema.resolve_name("shared") {
            observed = Some(declaration.source_node);
        }
    });
    assert_eq!(observed, Some(None));
}
