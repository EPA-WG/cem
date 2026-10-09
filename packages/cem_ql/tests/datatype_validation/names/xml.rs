use super::*;
use cem_ml::{
    ast::reload::{ReferenceReloadBundle, ReloadLimits, ReloadSource},
    import::import_bytes_with_lexical_scopes,
};
fn discover_xml(
    captured: Arc<cem_ml::schema::machine::LexicallyScopedDocument>,
    tree: Arc<RetainedCemTree>,
) -> Result<DatatypeNameCatalog, DatatypeNameError> {
    let owner = captured.document();
    let schema = owner
        .nodes
        .iter()
        .find_map(|node| match node {
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
    let mut host = cem_ql::schema_references::CemQlSchemaDeclarationHost::new();
    host.register_scope(
        tree,
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    DatatypeNameCatalog::discover(
        &[DatatypeSchemaSource {
            schema,
            captured,
            imports: vec![],
        }],
        &mut host,
        Default::default(),
    )
}
fn named_source<'a>(catalog: &'a DatatypeNameCatalog, name: &str) -> &'a DatatypeSource {
    catalog.sources().find(|source|matches!(source.attribute("name").unwrap().node(),CemAstNode::Attribute {value:Some(value),..} if value == name)).unwrap()
}
#[test]
fn xml_discovery_uses_native_catalog_with_original_bindings_and_reload() {
    let text = "<schema xmlns='https://cem.dev/ns/schema/1' namespace='urn:own' xmlns:p='urn:own'><types><type name='first' base='p:base'/><type name='other' base='p:base' xmlns:p='urn:other'/><type name='last' base='p:base'/><type name='base' kind='scalar'/></types></schema>";
    let imported = import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "application/xml",
        "schema.xml",
        CompiledSchema::cem_core(),
    )
    .unwrap();
    let catalog = discover_xml(imported.captured.clone(), imported.tree.clone()).unwrap();
    for name in ["first", "last"] {
        let DatatypeNameLookup::Target(base) =
            catalog.lookup(named_source(&catalog, name), "p:base")
        else {
            panic!("missing {name}")
        };
        assert_eq!(
            base.declaration().identity(),
            named_source(&catalog, "base").declaration().identity()
        );
        assert!(Arc::ptr_eq(
            base.declaration().document(),
            imported.captured.document()
        ));
    }
    assert!(matches!(
        catalog.lookup(named_source(&catalog, "other"), "p:base"),
        DatatypeNameLookup::Unresolved(_)
    ));
    let mut bundle = ReferenceReloadBundle::export(
        &imported.captured,
        vec![ReloadSource::new(
            SourceId(1),
            "schema.xml",
            text.as_bytes(),
            true,
        )],
        ReloadLimits::default(),
    )
    .unwrap();
    let reload = bundle.reload(ReloadLimits::default()).unwrap();
    let captured = reload.require_lexical().unwrap().clone();
    let tree = RetainedCemTree::from_shared(
        reload.document.clone(),
        "schema.xml",
        "",
        Default::default(),
        None,
    )
    .unwrap();
    let reloaded = discover_xml(captured, tree).unwrap();
    assert!(matches!(
        reloaded.lookup(named_source(&reloaded, "first"), "p:base"),
        DatatypeNameLookup::Target(_)
    ));
    bundle
        .lexical
        .as_mut()
        .unwrap()
        .attribute_namespaces
        .clear();
    let reload = bundle.reload(ReloadLimits::default()).unwrap();
    let captured = reload.require_lexical().unwrap().clone();
    let tree = RetainedCemTree::from_shared(
        reload.document.clone(),
        "schema.xml",
        "",
        Default::default(),
        None,
    )
    .unwrap();
    let error = discover_xml(captured, tree).unwrap_err();
    assert_eq!(error.code, "datatype-literal-namespace-context-unavailable");
    assert!(error.pending);
}

#[test]
fn xml_literal_prefixes_conflict_with_uses_and_reserved_namespace_expressions_stay_rejected() {
    let text="<schema xmlns='https://cem.dev/ns/schema/1' namespace='urn:own' xmlns:p='urn:other'><uses><use as='p' schema='urn:own'/></uses><types><type name='derived' base='p:base'/><type name='base' kind='scalar'/></types></schema>";
    let imported = import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "application/xml",
        "schema.xml",
        CompiledSchema::cem_core(),
    )
    .unwrap();
    let error = discover_xml(imported.captured, imported.tree).unwrap_err();
    assert_eq!(error.code, "conflicting-datatype-alias");
    assert!(error.related.is_some());
    let uses_only = text.replace(" xmlns:p='urn:other'", "");
    let imported = import_bytes_with_lexical_scopes(
        uses_only.as_bytes(),
        "application/xml",
        "schema.xml",
        CompiledSchema::cem_core(),
    )
    .unwrap();
    let catalog = discover_xml(imported.captured, imported.tree).unwrap();
    assert!(matches!(
        catalog.lookup(named_source(&catalog, "derived"), "p:base"),
        DatatypeNameLookup::Target(_)
    ));

    let text="<schema xmlns:c='https://cem.dev/ns/core/1' xmlns:p='{#namespace}' c:expression-attributes='xmlns:p'/>";
    assert!(import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "application/xml",
        "schema.xml",
        CompiledSchema::cem_core()
    )
    .is_err());
}
