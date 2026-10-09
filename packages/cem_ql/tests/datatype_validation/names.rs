use super::*;
use cem_ml::schema::datatype_contracts::CompiledDatatypeContract;
use cem_ml::schema::declaration_references::SchemaDeclarationHost;
use cem_ql::datatype_names::*;
fn scope(sources: &[DatatypeSource], namespace: &str) -> DatatypeNameScope {
    DatatypeNameScope {
        scope: sources[0].scope().clone(),
        namespace: Some(namespace.into()),
        declarations: sources
            .iter()
            .map(|source| DatatypeNameDeclaration {
                source: source.clone(),
                aliases: BTreeMap::new(),
            })
            .collect(),
        imports: vec![],
    }
}
fn compile_names(
    host: &mut cem_ql::schema_references::CemQlSchemaDeclarationHost,
    sources: &[DatatypeSource],
    scopes: &[DatatypeNameScope],
) -> cem_ml::schema::datatype_contracts::DatatypeCompilation {
    let catalog = DatatypeNameCatalog::collect(scopes, host, Default::default()).unwrap();
    host.install_datatype_names(Arc::new(catalog)).unwrap();
    let mut implementations = DatatypeImplementations::default();
    for source in sources {
        if source.attribute("kind").is_some() {
            implementations
                .register(implementation(
                    source,
                    DatatypeKind::Scalar,
                    ValueRepresentation::Scalar(ScalarRepresentation::String),
                ))
                .unwrap();
        }
    }
    compile_datatypes(
        sources[0].declaration().document().clone(),
        &[sources[0].clone()],
        host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
}
#[test]
fn names_collect_forward_declarations_and_use_each_original_alias() {
    let (mut host, sources) =
        types_fixture("{type @name=derived @base=local:base} {type @name=base @kind=scalar}");
    let mut names = scope(&sources, "urn:one");
    names.declarations[0]
        .aliases
        .insert("local".into(), Some("urn:one".into()));
    names.declarations[1]
        .aliases
        .insert("local".into(), Some("urn:other".into()));
    let result = compile_names(&mut host, &sources, &[names]);
    assert!(result.is_ready(), "{:?}", result.issues);
    assert_eq!(
        compiled(&result, &sources[0])
            .base()
            .unwrap()
            .source()
            .declaration()
            .identity(),
        sources[1].declaration().identity()
    );
}
#[test]
fn names_pending_and_unknown_aliases_never_use_legacy_lookup() {
    for pending in [false, true] {
        let (mut host, sources) =
            types_fixture("{type @name=derived @base=x:base} {type @name=base @kind=scalar}");
        host.bind_literal_datatype(
            sources[0].scope(),
            "x:base",
            sources[1].declaration().clone(),
        )
        .unwrap();
        let mut names = scope(&sources, "urn:one");
        if pending {
            names.declarations[0].aliases.insert("x".into(), None);
        }
        let result = compile_names(&mut host, &sources, &[names]);
        assert!(!result.is_ready());
        assert!(result.contracts.is_empty());
    }
}

#[test]
fn names_exports_preserve_original_dependencies_and_require_scope_grants() {
    use cem_ml::value::reference_resolution::ReferenceResolutionHost;
    for grant in [false, true] {
        let (mut host, consumer) = types_fixture("{type @name=consumer @base=vendor:public}");
        let (vendor_host, vendor) = types_fixture(
            "{type @name=private @kind=scalar} {type @name=exported @base=own:private}",
        );
        let from = host
            .scope(&host.source_reference(consumer[0].declaration().clone()))
            .unwrap();
        let to = host.register_scope(
            vendor_host
                .source_tree(vendor[0].declaration())
                .unwrap()
                .clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        );
        if grant {
            assert!(host.allow_scope_crossing(from, to));
        }
        let mut a = scope(&consumer, "urn:consumer");
        a.declarations[0]
            .aliases
            .insert("vendor".into(), Some("urn:vendor".into()));
        // This alias must never be used by the exported declaration's dependency.
        a.declarations[0]
            .aliases
            .insert("own".into(), Some("urn:wrong".into()));
        a.imports.push(DatatypeExport {
            namespace: "urn:vendor".into(),
            name: "public".into(),
            source: vendor[1].clone(),
        });
        let mut b = scope(&vendor, "urn:vendor");
        b.declarations[1]
            .aliases
            .insert("own".into(), Some("urn:vendor".into()));
        let sources = consumer
            .iter()
            .chain(vendor.iter())
            .cloned()
            .collect::<Vec<_>>();
        let result = compile_names(&mut host, &sources, &[a, b]);
        assert_eq!(result.is_ready(), grant, "{:?}", result.issues);
        if grant {
            assert_eq!(
                compiled(&result, &consumer[0])
                    .base()
                    .unwrap()
                    .base()
                    .unwrap()
                    .source()
                    .declaration()
                    .identity(),
                vendor[0].declaration().identity()
            );
            assert_eq!(result.sources.len(), 3);
        } else {
            assert!(!result.reference_issues.is_empty());
        }
    }
}
#[test]
fn names_unexported_members_and_invalid_qnames_are_not_discovered() {
    let (host, sources) = types_fixture("{type @name=sample @kind=scalar}");
    let mut names = scope(&sources, "urn:one");
    names.declarations[0]
        .aliases
        .insert("x".into(), Some("urn:other".into()));
    let catalog = DatatypeNameCatalog::collect(&[names], &host, Default::default()).unwrap();
    assert!(matches!(
        catalog.lookup(&sources[0], "x:sample"),
        DatatypeNameLookup::Unresolved("unexported-datatype-name")
    ));
    for qname in [
        "https://example.org/types#sample",
        "x:y:z",
        " sample",
        "x:",
        "#sample",
    ] {
        assert!(matches!(
            catalog.lookup(&sources[0], qname),
            DatatypeNameLookup::Unresolved("invalid-datatype-qname")
        ));
    }
}
#[test]
fn names_duplicate_exports_report_both_original_sources_and_reuse_is_valid() {
    let (host, sources) = types_fixture("{type @name=a @kind=scalar} {type @name=b @kind=scalar}");
    let mut names = scope(&sources, "urn:one");
    let export = DatatypeExport {
        namespace: "urn:public".into(),
        name: "public".into(),
        source: sources[0].clone(),
    };
    names.imports = vec![export.clone(), export];
    assert!(DatatypeNameCatalog::collect(&[names.clone()], &host, Default::default()).is_ok());
    names.imports[1].source = sources[1].clone();
    let error = DatatypeNameCatalog::collect(&[names], &host, Default::default()).unwrap_err();
    assert_eq!(error.code, "ambiguous-datatype-export");
    assert_eq!(error.source.identity(), sources[1].declaration().identity());
    assert_eq!(
        error.related.unwrap().identity(),
        sources[0].declaration().identity()
    );
}
#[test]
fn names_duplicate_locals_are_not_source_order_overrides() {
    let (host, a) = types_fixture("{type @name=sample @kind=scalar}");
    let (_, b) = types_fixture("{type @name=sample @kind=scalar}");
    let mut registry = DatatypeRegistry::default();
    registry
        .insert(a[0].scope().clone(), b[0].declaration().clone())
        .unwrap();
    let incoming = registry.source(a[0].scope(), "sample").unwrap();
    let mut names = scope(&a, "urn:one");
    names.declarations.push(DatatypeNameDeclaration {
        source: incoming,
        aliases: BTreeMap::new(),
    });
    let error = DatatypeNameCatalog::collect(&[names], &host, Default::default()).unwrap_err();
    assert_eq!(error.code, "duplicate-datatype-name");
    assert_eq!(
        error.related.unwrap().identity(),
        a[0].declaration().identity()
    );
}
#[test]
fn names_repeated_sources_need_one_consistent_environment() {
    let (host, sources) = types_fixture("{type @name=sample @kind=scalar}");
    let mut names = scope(&sources, "urn:one");
    names.declarations.push(names.declarations[0].clone());
    assert!(DatatypeNameCatalog::collect(&[names.clone()], &host, Default::default()).is_ok());
    names.declarations[1].aliases.insert("x".into(), None);
    assert_eq!(
        DatatypeNameCatalog::collect(&[names], &host, Default::default())
            .unwrap_err()
            .code,
        "conflicting-datatype-name-environment"
    );
}
#[test]
fn names_collection_is_bounded_and_pending_namespace_is_not_empty() {
    let (host, sources) = types_fixture("{type @name=sample @kind=scalar}");
    let mut names = scope(&sources, "urn:one");
    for limits in [
        DatatypeNameLimits {
            max_work: 0,
            ..Default::default()
        },
        DatatypeNameLimits {
            max_name_bytes: 0,
            ..Default::default()
        },
    ] {
        let error = DatatypeNameCatalog::collect(&[names.clone()], &host, limits).unwrap_err();
        assert!(error.pending);
        assert_eq!(error.code, "datatype-name-collection-limit");
    }
    names.namespace = None;
    let catalog = DatatypeNameCatalog::collect(&[names], &host, Default::default()).unwrap();
    assert!(matches!(
        catalog.lookup(&sources[0], "sample"),
        DatatypeNameLookup::Pending("datatype-namespace-pending")
    ));
}
#[test]
fn names_failed_install_preserves_previous_catalog_and_clones_keep_their_snapshot() {
    let (mut host, sources) =
        types_fixture("{type @name=derived @base=base} {type @name=base @kind=scalar}");
    let names = scope(&sources, "urn:one");
    let catalog = Arc::new(
        DatatypeNameCatalog::collect(&[names.clone()], &host, Default::default()).unwrap(),
    );
    host.install_datatype_names(catalog).unwrap();
    let old = host.clone();
    let (foreign_host, foreign) = types_fixture("{type @name=foreign @kind=scalar}");
    let foreign_catalog = Arc::new(
        DatatypeNameCatalog::collect(
            &[scope(&foreign, "urn:foreign")],
            &foreign_host,
            Default::default(),
        )
        .unwrap(),
    );
    assert_eq!(
        host.install_datatype_names(foreign_catalog).unwrap_err(),
        "unregistered-datatype-owner"
    );
    use cem_ml::schema::datatype_registry::DatatypeDependencyHost;
    use cem_ml::value::reference_resolution::ReferenceLinkEvaluation;
    let plan = sources[0].plan();
    let dependency = &plan.dependencies[0];
    assert!(matches!(
        host.lookup_literal_type(&sources[0], dependency, "base"),
        ReferenceLinkEvaluation::Resolved(_)
    ));
    let mut missing = names;
    missing.declarations.pop();
    host.install_datatype_names(Arc::new(
        DatatypeNameCatalog::collect(&[missing], &host, Default::default()).unwrap(),
    ))
    .unwrap();
    assert!(matches!(
        host.lookup_literal_type(&sources[0], dependency, "base"),
        ReferenceLinkEvaluation::Unresolved(_)
    ));
    assert!(matches!(
        old.clone()
            .lookup_literal_type(&sources[0], dependency, "base"),
        ReferenceLinkEvaluation::Resolved(_)
    ));
}

#[test]
fn names_reject_foreign_metamodel_lookalikes_and_uncollected_exports() {
    let (host, sources) = types_fixture("{type @name=sample @kind=scalar}");
    for field in [false, true] {
        let mut document = cem_ml::parser::document::CemDocument {
            nodes: sources[0].declaration().document().nodes.clone(),
            ..Default::default()
        };
        let id = if field {
            sources[0].attribute("name").unwrap().node_id()
        } else {
            sources[0].declaration().node_id()
        };
        match &mut document.nodes[id as usize] {
            CemAstNode::Element { expanded_name, .. }
            | CemAstNode::Attribute { expanded_name, .. } => {
                expanded_name.namespace_uri = "urn:foreign".into()
            }
            _ => panic!(),
        }
        let owner = Arc::new(document);
        let schema =
            SchemaDeclarationNode::new(owner.clone(), sources[0].scope().node_id()).unwrap();
        let mut registry = DatatypeRegistry::default();
        registry
            .insert(
                schema.clone(),
                SchemaDeclarationNode::new(owner, sources[0].declaration().node_id()).unwrap(),
            )
            .unwrap();
        let source = registry.source(&schema, "sample").unwrap();
        let error =
            DatatypeNameCatalog::collect(&[scope(&[source], "urn:one")], &host, Default::default())
                .unwrap_err();
        assert_eq!(error.code, "datatype-metamodel-name");
    }
    let (_, foreign) = types_fixture("{type @name=foreign @kind=scalar}");
    let mut names = scope(&sources, "urn:one");
    names.imports.push(DatatypeExport {
        namespace: "urn:other".into(),
        name: "foreign".into(),
        source: foreign[0].clone(),
    });
    assert_eq!(
        DatatypeNameCatalog::collect(&[names], &host, Default::default())
            .unwrap_err()
            .code,
        "uncollected-datatype-export"
    );
}

#[test]
fn names_equal_namespaces_do_not_merge_independent_scopes_or_rebind_owners() {
    let (mut host, a) = types_fixture("{type @name=sample @kind=scalar}");
    let (other, b) = types_fixture("{type @name=sample @kind=scalar}");
    host.register_scope(
        other.source_tree(b[0].declaration()).unwrap().clone(),
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    let catalog = DatatypeNameCatalog::collect(
        &[scope(&a, "urn:shared"), scope(&b, "urn:shared")],
        &host,
        Default::default(),
    )
    .unwrap();
    for source in [&a[0], &b[0]] {
        let DatatypeNameLookup::Target(found) = catalog.lookup(source, "sample") else {
            panic!()
        };
        assert_eq!(
            found.declaration().identity(),
            source.declaration().identity()
        );
    }
    host.install_datatype_names(Arc::new(catalog)).unwrap();
    let mut registry = DatatypeRegistry::default();
    registry
        .insert(b[0].scope().clone(), a[0].declaration().clone())
        .unwrap();
    let rebound = registry.source(b[0].scope(), "sample").unwrap();
    let replacement = DatatypeNameCatalog::collect(
        &[scope(&[rebound], "urn:shared")],
        &host,
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        host.install_datatype_names(Arc::new(replacement))
            .unwrap_err(),
        "conflicting-datatype-scope"
    );
}
