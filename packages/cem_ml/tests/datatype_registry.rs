//! Retained collection invariants; these fixtures do not enable datatype execution.
use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::{builder::CemAstBuilder, document::CemDocument, CemAstNode},
    schema::{
        datatype_registry::{DatatypeRegistration, DatatypeRegistry, DatatypeRegistryError},
        declaration_references::SchemaDeclarationNode,
    },
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

fn nodes(owner: &Arc<CemDocument>, name: &str) -> Vec<SchemaDeclarationNode> {
    owner
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => {
                SchemaDeclarationNode::new(owner.clone(), *node_id)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn repeated_original_declaration_reuses_name_and_native_binding() {
    let owner = parse("{schema | {type @name=integer @kind=scalar}}");
    let scope = nodes(&owner, "schema").remove(0);
    let declaration = nodes(&owner, "type").remove(0);
    let mut registry = DatatypeRegistry::default();
    assert_eq!(
        registry.insert(scope.clone(), declaration.clone()).unwrap(),
        DatatypeRegistration::Inserted
    );
    assert_eq!(
        registry.insert(scope.clone(), declaration.clone()).unwrap(),
        DatatypeRegistration::Reused
    );
    assert_eq!(
        registry.get(&scope, "integer").unwrap().identity(),
        declaration.identity()
    );
    assert_eq!(
        registry
            .get_by_declaration(&declaration)
            .unwrap()
            .identity(),
        declaration.identity()
    );
    let other = parse("{type @name=integer @kind=scalar}");
    assert!(registry
        .get_by_declaration(&nodes(&other, "type")[0])
        .is_none());
}

#[test]
fn collision_preserves_both_sources_and_the_first_binding() {
    let owner =
        parse("{schema | {type @name=choice @kind=scalar} {type @name=choice @kind=lexical}}");
    let scope = nodes(&owner, "schema").remove(0);
    let declarations = nodes(&owner, "type");
    let mut registry = DatatypeRegistry::default();
    registry
        .insert(scope.clone(), declarations[0].clone())
        .unwrap();
    let DatatypeRegistryError::Duplicate {
        name,
        scope: reported_scope,
        existing,
        incoming,
    } = registry
        .insert(scope.clone(), declarations[1].clone())
        .unwrap_err()
    else {
        panic!("expected a collision")
    };
    assert_eq!(name, "choice");
    assert_eq!(reported_scope.identity(), scope.identity());
    assert_eq!(existing.identity(), declarations[0].identity());
    assert_eq!(incoming.identity(), declarations[1].identity());
    assert_eq!(
        registry.get(&scope, "choice").unwrap().identity(),
        declarations[0].identity()
    );
    assert!(registry.get_by_declaration(&declarations[1]).is_none());
}

#[test]
fn distinct_owner_with_same_node_address_is_not_declaration_reuse() {
    let first = parse("{schema | {type @name=choice}}");
    let second = parse("{schema | {type @name=choice}}");
    let scope = nodes(&first, "schema").remove(0);
    let a = nodes(&first, "type").remove(0);
    let b = nodes(&second, "type").remove(0);
    assert_eq!(a.node_id(), b.node_id());
    let mut registry = DatatypeRegistry::default();
    registry.insert(scope.clone(), a).unwrap();
    assert!(matches!(
        registry.insert(scope, b),
        Err(DatatypeRegistryError::Duplicate { .. })
    ));
}

#[test]
fn equal_names_in_distinct_given_scopes_are_independent_without_ids() {
    let owner = parse("{schema | {type @name=choice}} {schema | {type @name=choice}}");
    let scopes = nodes(&owner, "schema");
    let declarations = nodes(&owner, "type");
    let mut registry = DatatypeRegistry::default();
    for index in 0..2 {
        registry
            .insert(scopes[index].clone(), declarations[index].clone())
            .unwrap();
    }
    for index in 0..2 {
        assert_eq!(
            registry.get(&scopes[index], "choice").unwrap().identity(),
            declarations[index].identity()
        );
    }
    assert!(registry.get(&scopes[0], "unknown").is_none());
}

#[test]
fn registry_keeps_original_scope_and_vendor_declaration_owners_alive() {
    let scope_owner = parse("{schema}");
    let type_owner = parse("{type @name=vendor @kind=scalar @rule='vendor description'}");
    let weak_scope = Arc::downgrade(&scope_owner);
    let weak_type = Arc::downgrade(&type_owner);
    let scope = nodes(&scope_owner, "schema").remove(0);
    let declaration = nodes(&type_owner, "type").remove(0);
    let mut registry = DatatypeRegistry::default();
    registry.insert(scope, declaration).unwrap();
    drop(scope_owner);
    drop(type_owner);
    let retained_scope = weak_scope.upgrade().expect("original scope retained");
    let retained_type = weak_type.upgrade().expect("original declaration retained");
    let scope = nodes(&retained_scope, "schema").remove(0);
    let target = registry.get(&scope, "vendor").unwrap();
    assert!(Arc::ptr_eq(target.document(), &retained_type));
    drop(retained_scope);
    drop(retained_type);
    drop(scope);
    drop(registry);
    assert!(weak_scope.upgrade().is_none());
    assert!(weak_type.upgrade().is_none());
}

#[test]
fn malformed_declarations_do_not_create_bindings() {
    let scope_owner = parse("{schema}");
    let scope = nodes(&scope_owner, "schema").remove(0);
    let mut registry = DatatypeRegistry::default();
    for text in [
        "{type}",
        "{type @name=' '}",
        "{type @name={#datatype}}",
        "{element @name=choice}",
    ] {
        let owner = parse(text);
        let node = if text.starts_with("{element") {
            "element"
        } else {
            "type"
        };
        let declaration = nodes(&owner, node).remove(0);
        let identity = declaration.identity();
        let DatatypeRegistryError::InvalidDeclaration { declaration } =
            registry.insert(scope.clone(), declaration).unwrap_err()
        else {
            panic!("expected invalid declaration: {text}")
        };
        assert_eq!(declaration.identity(), identity);
        assert!(registry.get_by_declaration(&declaration).is_none());
    }
    assert!(registry.get(&scope, "choice").is_none());
}
