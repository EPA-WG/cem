use super::*;
use cem_ml::schema::machine::CemSchemaMachine;
fn authored(
    text: &str,
) -> (
    cem_ql::schema_references::CemQlSchemaDeclarationHost,
    DatatypeSchemaSource,
) {
    let captured = Arc::new(
        CemSchemaMachine::new(
            CompiledSchema::cem_core(),
            CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                SourceId(1),
                text.as_bytes().to_vec(),
            ))),
        )
        .build_with_lexical_scopes(),
    );
    let owner = captured.document();
    assert!(owner.diagnostics.is_empty(), "{:?}", owner.diagnostics);
    let schema = owner
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
    let tree = RetainedCemTree::from_shared(
        owner.clone(),
        "authored.cem",
        text,
        Default::default(),
        None,
    )
    .unwrap();
    let mut host = cem_ql::schema_references::CemQlSchemaDeclarationHost::new();
    host.register_scope(
        tree,
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    (
        host,
        DatatypeSchemaSource {
            schema,
            captured,
            imports: vec![],
        },
    )
}
#[test]
fn discovers_forward_sources_and_source_position_prefix_rebinding() {
    let (mut host,input)=authored("@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @name=test @namespace=urn:own | {types @xmlns:p=urn:own | {type @name=first @base=p:base} {type @xmlns:p=urn:other @name=second @base=p:base} {type @name=base @kind=scalar}}}");
    let catalog = DatatypeNameCatalog::discover(&[input], &mut host, Default::default()).unwrap();
    let first=catalog.sources().find(|s|matches!(s.attribute("name").unwrap().node(),CemAstNode::Attribute{value:Some(v),..} if v=="first")).unwrap();
    let second=catalog.sources().find(|s|matches!(s.attribute("name").unwrap().node(),CemAstNode::Attribute{value:Some(v),..} if v=="second")).unwrap();
    assert!(matches!(
        catalog.lookup(first, "p:base"),
        DatatypeNameLookup::Target(_)
    ));
    assert!(matches!(
        catalog.lookup(second, "p:base"),
        DatatypeNameLookup::Unresolved(_)
    ));
}
#[test]
fn discovers_uses_without_overriding_pending_or_conflicting_lexical_bindings() {
    for (declaration, header, expected) in [
        ("", "", 0),
        ("@ns p = urn:other\n", "", 1),
        ("", "@xmlns:p={#namespace}", 2),
    ] {
        let text=format!("{declaration}{{schema @name=test @namespace=urn:own {header} | {{uses | {{use @as=p @schema=urn:own}}}} {{types | {{type @name=derived @base=p:base}} {{type @name=base @kind=scalar}}}}}}");
        let (mut host, input) = authored(&text);
        let result = DatatypeNameCatalog::discover(&[input], &mut host, Default::default());
        if expected == 1 {
            assert_eq!(result.unwrap_err().code, "conflicting-datatype-alias");
            continue;
        }
        let catalog = result.unwrap();
        let source = catalog
            .sources()
            .find(|s| s.attribute("base").is_some())
            .unwrap();
        assert_eq!(
            matches!(
                catalog.lookup(source, "p:base"),
                DatatypeNameLookup::Target(_)
            ),
            expected == 0
        );
        if expected == 2 {
            assert!(matches!(
                catalog.lookup(source, "p:base"),
                DatatypeNameLookup::Pending(_)
            ));
        }
    }
}
#[test]
fn discovery_does_not_ignore_native_collection_slots_or_duplicate_declarations() {
    for (types, code, pending) in [
        (
            "{#library}",
            "datatype-collection-selection-unavailable",
            true,
        ),
        (
            "{type @name=same @kind=scalar}{type @name=same @kind=scalar}",
            "duplicate-datatype-name",
            false,
        ),
    ] {
        let (mut host, input) = authored(&format!(
            "{{schema @name=test @namespace=urn:own | {{types | {types}}}}}"
        ));
        let error =
            DatatypeNameCatalog::discover(&[input], &mut host, Default::default()).unwrap_err();
        assert_eq!(error.code, code);
        assert_eq!(error.pending, pending);
    }
}
#[test]
fn discovery_bounds_and_original_capture_owner_are_required() {
    let (mut host, input) = authored(
        "{schema @name=test @namespace=urn:own | {types | {type @name=sample @kind=scalar}}}",
    );
    let error = DatatypeNameCatalog::discover(
        &[input.clone()],
        &mut host,
        DatatypeNameLimits {
            max_work: 1,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(error.pending);
    let (_, other) = authored("{schema @name=test @namespace=urn:own}");
    let error = DatatypeNameCatalog::discover(
        &[DatatypeSchemaSource {
            captured: other.captured,
            ..input
        }],
        &mut host,
        Default::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, "datatype-capture-owner");
}

#[test]
fn discovery_consumes_only_the_current_completed_namespace_view() {
    use cem_ml::schema::namespace_references::{
        admit_namespace_scope_target, NamespaceNameCompletion,
    };
    let (mut host,input)=authored("@ns public = urn:own\n{schema @xmlns:p={#namespace} @name=test @namespace=urn:own | {types | {type @name=derived @base=p:base} {type @name=base @kind=scalar}}}");
    let owner = input.captured.document();
    let provider = owner
        .nodes
        .iter()
        .find_map(|n| {
            let id = match n {
                CemAstNode::Element { node_id, .. } | CemAstNode::Attribute { node_id, .. } => {
                    *node_id
                }
                _ => return None,
            };
            input
                .captured
                .namespace_binding(owner, id)
                .filter(|b| b.name == "public")
                .map(|_| SchemaDeclarationNode::new(owner.clone(), id).unwrap())
        })
        .unwrap();
    let pending = owner
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Attribute { node_id, .. }
                if input
                    .captured
                    .pending_namespace_declaration(owner, *node_id)
                    .is_some() =>
            {
                Some(*node_id)
            }
            _ => None,
        })
        .unwrap();
    let target = admit_namespace_scope_target(provider, &input.captured).unwrap();
    let completion = Arc::new(
        NamespaceNameCompletion::new(
            input.captured.clone(),
            &[input.schema.node_id()],
            BTreeMap::from([(pending, target)]),
        )
        .unwrap(),
    );
    let ready = host
        .with_completed_namespace_names(completion, |host| {
            DatatypeNameCatalog::discover(&[input.clone()], host, Default::default())
        })
        .unwrap()
        .unwrap();
    let source = ready
        .sources()
        .find(|s| s.attribute("base").is_some())
        .unwrap();
    assert!(matches!(
        ready.lookup(source, "p:base"),
        DatatypeNameLookup::Target(_)
    ));
    let original = DatatypeNameCatalog::discover(&[input], &mut host, Default::default()).unwrap();
    assert!(matches!(
        original.lookup(source, "p:base"),
        DatatypeNameLookup::Pending(_)
    ));
}
