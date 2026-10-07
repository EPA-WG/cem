use cem_ml::{
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    parser::CemAstNode,
    schema::{
        declaration_references::SchemaDeclarationNode,
        namespace_references::{decode_native_namespace_property, NativeNamespacePropertyError},
        vocab::CompiledSchema,
    },
};
use std::sync::Arc;
fn import(text: &str) -> ScopedCemImport {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "properties.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
}
fn node(input: &ScopedCemImport, id: u32) -> SchemaDeclarationNode {
    SchemaDeclarationNode::new(input.tree.ast_owner().clone(), id).unwrap()
}
fn attributes(input: &ScopedCemImport) -> Vec<u32> {
    input
        .tree
        .ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Attribute { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .collect()
}
#[test]
fn native_namespace_properties_decode_original_reference_and_general_expression_slots() {
    for (text, prefix, reference) in [
        ("{host @xmlns:v={#library} | {v:item}}", "v", true),
        ("{host @xmlns={library} | {item}}", "", false),
    ] {
        let input = import(text);
        let attribute = attributes(&input)[0];
        let property =
            decode_native_namespace_property(node(&input, attribute), &input.captured).unwrap();
        assert_eq!(property.prefix, prefix);
        assert_eq!(property.declaration.node_id(), attribute);
        assert!(Arc::ptr_eq(
            property.value.document(),
            input.tree.ast_owner()
        ));
        assert_eq!(
            property.value.node_id(),
            input.captured.occurrences().next().unwrap()
        );
        assert_eq!(
            matches!(
                property.value.node(),
                CemAstNode::Reference { targets: None, .. }
            ),
            reference
        );
        assert!(input
            .captured
            .namespace_binding(input.tree.ast_owner(), attribute)
            .is_none());
        assert!(input
            .captured
            .snapshot(input.tree.ast_owner(), property.value.node_id())
            .is_some());
    }
}
#[test]
fn ordinary_attributes_literal_namespace_forms_and_foreign_owners_are_not_native_properties() {
    let input =
        import("@ns public = urn:public\n{host @xmlns:v=urn:literal @target={#library} | {item}}");
    for id in attributes(&input) {
        assert!(matches!(
            decode_native_namespace_property(node(&input, id), &input.captured),
            Err(NativeNamespacePropertyError::NotNativeNamespaceDeclaration)
        ));
    }
    let native = import("{host @xmlns:v={#library}}");
    assert!(matches!(
        decode_native_namespace_property(node(&native, attributes(&native)[0]), &input.captured),
        Err(NativeNamespacePropertyError::OwnerMismatch)
    ));
    let xml = import_bytes_with_lexical_scopes(
        b"<host xmlns:v='{#library}'/>",
        "application/xml",
        "literal.xml",
        CompiledSchema::cem_core(),
    )
    .unwrap();
    for id in attributes(&xml) {
        assert!(matches!(
            decode_native_namespace_property(node(&xml, id), &xml.captured),
            Err(NativeNamespacePropertyError::NotNativeNamespaceDeclaration)
        ));
    }
}
