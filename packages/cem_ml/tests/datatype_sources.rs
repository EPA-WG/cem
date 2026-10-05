//! Source descriptors retain authored fields; dependency evaluation is separate.
use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::{builder::CemAstBuilder, document::CemDocument, CemAstNode},
    schema::{datatype_registry::DatatypeRegistry, declaration_references::SchemaDeclarationNode},
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use std::sync::Arc;

fn parse(text: &str) -> Arc<CemDocument> {
    let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), text.as_bytes().to_vec()),
    )))
    .build();
    assert!(
        document.diagnostics.is_empty(),
        "{:?}",
        document.diagnostics
    );
    Arc::new(document)
}
fn element(owner: &Arc<CemDocument>, name: &str) -> SchemaDeclarationNode {
    let id = owner
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => Some(*node_id),
            _ => None,
        })
        .unwrap();
    SchemaDeclarationNode::new(owner.clone(), id).unwrap()
}
fn scalar(attribute: &SchemaDeclarationNode) -> Option<&str> {
    let CemAstNode::Attribute { value, .. } = attribute.node() else {
        panic!("not an attribute")
    };
    value.as_deref()
}

#[test]
fn native_dependency_fields_remain_authored_and_unknown_fields_remain_visible() {
    let owner = parse("{schema | {type @name=derived @base={#datatype} @rule={#behavior} @vendor-extension=kept}}");
    let scope = element(&owner, "schema");
    let declaration = element(&owner, "type");
    let mut registry = DatatypeRegistry::default();
    registry.insert(scope.clone(), declaration.clone()).unwrap();
    let source = registry.source(&scope, "derived").unwrap();
    assert_eq!(source.attributes().len(), 4);
    assert_eq!(source.scope().identity(), scope.identity());
    assert_eq!(source.declaration().identity(), declaration.identity());
    for name in ["base", "rule"] {
        let attribute = source.attribute(name).unwrap();
        assert!(Arc::ptr_eq(attribute.document(), &owner));
        let CemAstNode::Attribute { value_nodes, .. } = attribute.node() else {
            panic!()
        };
        assert_eq!(value_nodes.len(), 1);
        assert!(matches!(
            owner.get(value_nodes[0]),
            Some(CemAstNode::Reference { .. })
        ));
    }
    assert_eq!(
        scalar(source.attribute("vendor-extension").unwrap()),
        Some("kept")
    );
    assert!(source.attribute("kind").is_none());
}

#[test]
fn omitted_and_explicit_empty_kind_are_not_conflated() {
    for (text, present) in [
        ("{type @name=derived @base=integer}", false),
        ("{type @name=derived @kind='' @base=integer}", true),
    ] {
        let scope_owner = parse("{schema}");
        let owner = parse(text);
        let scope = element(&scope_owner, "schema");
        let mut registry = DatatypeRegistry::default();
        registry
            .insert(scope.clone(), element(&owner, "type"))
            .unwrap();
        let source = registry.source(&scope, "derived").unwrap();
        assert_eq!(source.attribute("kind").is_some(), present);
        assert_eq!(scalar(source.attribute("base").unwrap()), Some("integer"));
        if present {
            assert_eq!(scalar(source.attribute("kind").unwrap()), Some(""));
        }
    }
}

#[test]
fn source_descriptor_keeps_original_owners_after_registry_release() {
    let scope_owner = parse("{schema}");
    let owner = parse("{type @name=derived @base=integer}");
    let weak_scope = Arc::downgrade(&scope_owner);
    let weak_owner = Arc::downgrade(&owner);
    let scope = element(&scope_owner, "schema");
    let declaration = element(&owner, "type");
    let mut registry = DatatypeRegistry::default();
    registry.insert(scope.clone(), declaration.clone()).unwrap();
    let source = registry.source(&scope, "derived").unwrap();
    drop(registry);
    drop(scope);
    drop(declaration);
    drop(scope_owner);
    drop(owner);
    assert!(weak_scope.upgrade().is_some());
    assert!(weak_owner.upgrade().is_some());
    assert_eq!(scalar(source.attribute("base").unwrap()), Some("integer"));
    drop(source);
    assert!(weak_scope.upgrade().is_none());
    assert!(weak_owner.upgrade().is_none());
}

#[test]
fn last_authored_attribute_handle_is_preserved_without_rewriting_source() {
    let owner = parse("{schema | {type @name=derived @rule=first @rule={#behavior}}}");
    let scope = element(&owner, "schema");
    let declaration = element(&owner, "type");
    let CemAstNode::Element { attributes, .. } = declaration.node() else {
        panic!()
    };
    let rules: Vec<_> = attributes.iter().filter(|id| matches!(owner.get(**id), Some(CemAstNode::Attribute {expanded_name, ..}) if expanded_name.local_name == "rule")).copied().collect();
    assert_eq!(rules.len(), 2);
    let mut registry = DatatypeRegistry::default();
    registry.insert(scope.clone(), declaration).unwrap();
    let source = registry.source(&scope, "derived").unwrap();
    assert_eq!(source.attribute("rule").unwrap().node_id(), rules[1]);
    let retained_rules: Vec<_> = source.attributes().iter().filter_map(|attribute| {
        matches!(attribute.node(), CemAstNode::Attribute {expanded_name, ..} if expanded_name.local_name == "rule").then_some(attribute.node_id())
    }).collect();
    assert_eq!(retained_rules, rules);
    let original = SchemaDeclarationNode::new(owner, rules[0]).unwrap();
    assert_eq!(scalar(&original), Some("first"));
    assert!(registry.source(&scope, "unknown").is_none());
}
