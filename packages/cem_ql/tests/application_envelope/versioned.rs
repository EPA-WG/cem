use super::OperationControl;
use cem_ml::{
    content_cache::ContentHash,
    events::cem::CemEventNormalizer,
    parser::{AstNodeId, CemAstNode},
    schema::{
        datatype_registry::DatatypeRegistry,
        datatype_validation::ScalarRepresentation,
        declaration_references::SchemaDeclarationNode,
        machine::{CemSchemaMachine, LexicallyScopedDocument},
        structural_capsule::SourceIdentity,
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use cem_ql::{
    application_envelope::v3::{self as envelope, AssemblyRoot, EnvelopeBuilder},
    datatype_module_capsule::{v2 as assembly, ModuleSource, ModuleTarget},
    native_registrations::{
        dependency_sources, profiles::v2 as profiles, profiles::OperationRole,
        ImplementationIdentity, OperationMode,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[allow(dead_code)]
#[path = "../support/datatype_module_sources.rs"]
mod module_sources;
use module_sources::*;
#[path = "../support/datatype_assembly_sources.rs"]
mod assembly_sources;
use assembly_sources::*;
fn base() -> cem_ql::application_envelope::BuiltApplicationEnvelope {
    let mut builder = super::EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    builder
        .add_component(
            super::declaration("cem-app", "{p | Hello}"),
            BTreeMap::new(),
        )
        .unwrap();
    builder
        .build(
            BTreeSet::from(["cem-app".into()]),
            BTreeSet::new(),
            &OperationControl::default(),
        )
        .unwrap()
}
fn built() -> envelope::BuiltApplicationEnvelope {
    let (modules, profiles) = fixture();
    let root = datatype(&modules["app"].types.capture, "choice");
    let c = OperationControl::default();
    let graph = assembly::export("app", &modules, &[], &profiles, &hash(b"registry"), &c).unwrap();
    let mut builder = EnvelopeBuilder::new(base()).unwrap();
    let id = builder.add_assembly(graph, vec![], profiles).unwrap();
    builder
        .build(
            BTreeMap::from([(
                "cart-choice".into(),
                AssemblyRoot {
                    assembly: id,
                    datatype: root,
                },
            )]),
            &c,
        )
        .unwrap()
}
#[test]
fn versioned_application_roundtrips_explicit_assembly_roots_and_fresh_inert_owners() {
    let b = built();
    let c = OperationControl::default();
    let v = envelope::verify(&b.bytes, &b.pin, &c).unwrap();
    let copy = envelope::verify(&b.bytes, &b.pin, &c).unwrap();
    assert!(b.bytes.starts_with(envelope::MAGIC));
    assert_eq!(v.application().component_count(), 1);
    assert_eq!(v.assembly_count(), 1);
    assert!(!v.selections_verified());
    assert!(!v.executable_closure_verified());
    assert!(!v.native_link_closure_verified());
    let r = &v.roots()["cart-choice"];
    let a = v.assembly(&r.assembly).unwrap();
    assert_eq!(a.module_order(), &["core", "palette", "app"]);
    let m = a.module("app").unwrap();
    assert!(m.roots().contains(&r.datatype));
    for requirement in m.profiles()[&r.datatype].requirements().values() {
        assert!(Arc::ptr_eq(
            requirement.capture().document(),
            m.capture().document()
        ));
    }
    assert!(!Arc::ptr_eq(
        m.capture().document(),
        copy.assembly(&r.assembly)
            .unwrap()
            .module("app")
            .unwrap()
            .capture()
            .document()
    ));
    assert_eq!(b.bytes, built().bytes);
    assert!(cem_ql::application_envelope::verify(&b.bytes, &base().pin, &c).is_err());
    assert!(cem_ql::application_declaration::admit(&b.bytes, "{}", &hash(b"{}")).is_err());
}
fn graph(
    modules: &BTreeMap<String, assembly::ModuleSource>,
    profiles: &[assembly::ProfileArtifact],
) -> assembly::BuiltDatatypeModules {
    assembly::export(
        "app",
        modules,
        &[],
        profiles,
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap()
}
fn one(
    builder: EnvelopeBuilder,
    id: String,
    root: u32,
) -> Result<envelope::BuiltApplicationEnvelope, String> {
    builder.build(
        BTreeMap::from([(
            "selected".into(),
            AssemblyRoot {
                assembly: id,
                datatype: root,
            },
        )]),
        &OperationControl::default(),
    )
}
#[test]
fn shared_profiles_and_operation_bytes_are_deduplicated_across_assemblies() {
    let (m, p) = fixture();
    let root = datatype(&m["app"].types.capture, "choice");
    let mut builder = EnvelopeBuilder::new(base()).unwrap();
    let first = builder.add_assembly(graph(&m, &p), vec![], p).unwrap();
    let (mut m, p) = fixture();
    let app = m.remove("app").unwrap();
    m.insert("shop".into(), app);
    let b = assembly::export(
        "shop",
        &m,
        &[],
        &p,
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    let second = builder.add_assembly(b, vec![], p).unwrap();
    let b = builder
        .build(
            BTreeMap::from([
                (
                    "cart".into(),
                    AssemblyRoot {
                        assembly: first,
                        datatype: root,
                    },
                ),
                (
                    "checkout".into(),
                    AssemblyRoot {
                        assembly: second,
                        datatype: root,
                    },
                ),
            ]),
            &OperationControl::default(),
        )
        .unwrap();
    let v = envelope::verify(&b.bytes, &b.pin, &OperationControl::default()).unwrap();
    assert_eq!(v.assembly_count(), 2);
    assert_eq!(
        v.dependency_order()
            .iter()
            .filter(|x| matches!(x, envelope::DependencyKey::Profile(_)))
            .count(),
        1
    );
    assert_eq!(
        v.dependency_order()
            .iter()
            .filter(|x| matches!(x, envelope::DependencyKey::OperationSource(_)))
            .count(),
        1
    );
    assert!(matches!(
        v.dependency_order().last(),
        Some(envelope::DependencyKey::Assembly(_))
    ));
}
#[test]
fn conflicting_complete_sources_across_individually_valid_assemblies_refuse() {
    for keep_claimed_identity in [false, true] {
        let (m, p) = fixture();
        let root = datatype(&m["app"].types.capture, "choice");
        let mut builder = EnvelopeBuilder::new(base()).unwrap();
        let first = builder.add_assembly(graph(&m, &p), vec![], p).unwrap();
        let (mut m, p) = fixture();
        let text = "@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:twin | {types | {type @name=scalar @kind=scalar}}}";
        let core = m.get_mut("core").unwrap();
        core.types.capture = capture(text);
        if !keep_claimed_identity {
            core.types.identity =
                SourceIdentity::new(&core.types.identity.uri, SourceId(1), text.as_bytes());
        }
        let second = builder.add_assembly(graph(&m, &p), vec![], p).unwrap();
        assert!(builder
            .build(
                BTreeMap::from([
                    (
                        "first".into(),
                        AssemblyRoot {
                            assembly: first,
                            datatype: root
                        }
                    ),
                    (
                        "second".into(),
                        AssemblyRoot {
                            assembly: second,
                            datatype: root
                        }
                    ),
                ]),
                &OperationControl::default()
            )
            .is_err());
    }
}
#[test]
fn base_contract_source_and_implementation_conflicts_with_assemblies_refuse() {
    for source_conflict in [false, true] {
        let mut base = super::EnvelopeBuilder::new(&hash(b"registry")).unwrap();
        let contract = base
            .add_contract(
                super::native(if source_conflict { "other" } else { "constant" }),
                BTreeSet::new(),
            )
            .unwrap();
        base.add_component(
            super::declaration("cem-app", "{p | Hello}"),
            BTreeMap::new(),
        )
        .unwrap();
        let base = base
            .build(
                BTreeSet::from(["cem-app".into()]),
                BTreeSet::from([contract]),
                &OperationControl::default(),
            )
            .unwrap();
        let (mut m, p) = fixture();
        if source_conflict {
            m.get_mut("core").unwrap().types.identity.uri = "file:///apps/example/types.cem".into();
        }
        let root = datatype(&m["app"].types.capture, "choice");
        let mut builder = EnvelopeBuilder::new(base).unwrap();
        let id = builder.add_assembly(graph(&m, &p), vec![], p).unwrap();
        assert!(one(builder, id, root).is_err());
    }
}
#[test]
fn cancelled_or_incompatible_envelopes_do_not_invalidate_prior_verified_owners() {
    let b = built();
    let c = OperationControl::default();
    let v = envelope::verify(&b.bytes, &b.pin, &c).unwrap();
    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    assert!(envelope::verify(&b.bytes, &b.pin, &control).is_err());
    let mut bad = b.pin.clone();
    bad.registry_hash = hash(b"foreign");
    assert!(envelope::verify(&b.bytes, &bad, &c).is_err());
    let legacy = base();
    let pin = envelope::ApplicationEnvelopePin {
        hash: legacy.pin.hash,
        manifest_hash: legacy.pin.manifest_hash,
        registry_hash: legacy.pin.registry_hash,
    };
    assert!(envelope::verify(&legacy.bytes, &pin, &c).is_err());
    assert_eq!(v.assembly_count(), 1);
    assert!(v.application().artifact("cem-app").is_some());
}
#[test]
fn datatype_leaf_bytes_and_pins_are_carried_without_reconstructing_source_ast() {
    use cem_ql::{datatype_dependency_capsule::DependencyArtifact, datatype_source_capsule};
    let c = OperationControl::default();
    let core = leaf();
    let core_root = *core.roots.first().unwrap();
    let built = datatype_source_capsule::export(
        &core.capture,
        core.identity.clone(),
        &hash(b"registry"),
        core.roots.clone(),
        &c,
    )
    .unwrap();
    let mut app = derived("app", "core", core_root);
    let root = *app.roots.first().unwrap();
    *app.bindings.values_mut().next().unwrap() = ModuleTarget::Datatype {
        pin: built.pin.clone(),
        root: core_root,
    };
    let leaves = vec![DependencyArtifact::datatype(built)];
    let modules = BTreeMap::from([("app".into(), assembly::ModuleSource::new(app))]);
    let graph = assembly::export("app", &modules, &leaves, &[], &hash(b"registry"), &c).unwrap();
    let mut b = EnvelopeBuilder::new(base()).unwrap();
    let id = b.add_assembly(graph, leaves, vec![]).unwrap();
    let b = one(b, id, root).unwrap();
    let v = envelope::verify(&b.bytes, &b.pin, &c).unwrap();
    assert_eq!(
        v.dependency_order()
            .iter()
            .filter(|d| matches!(d, envelope::DependencyKey::Leaf(_)))
            .count(),
        1
    );
    let root = &v.roots()["selected"];
    let graph = v.assembly(&root.assembly).unwrap();
    let id = graph.leaf_ids().next().unwrap();
    let leaf = graph.leaf_datatypes(id).unwrap();
    assert!(leaf.contains_key(&core_root));
    assert!(!Arc::ptr_eq(
        graph.leaf_capture(id).unwrap().document(),
        core.capture.document()
    ));
}
#[test]
fn structural_named_receivers_and_query_contracts_keep_their_existing_contracts() {
    let c = OperationControl::default();
    let mut app = super::EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    let structural = app
        .add_contract(super::structural(), BTreeSet::new())
        .unwrap();
    let query = app.add_contract(super::query(), BTreeSet::new()).unwrap();
    app.add_component(
        super::declaration(
            "cem-app",
            "{attribute @name=items @type=cart-lines}{p | Hello}",
        ),
        BTreeMap::from([(
            "cart-lines".into(),
            super::NodeReceiverRequirement {
                contract: structural,
                schema_uri: super::SCHEMA.into(),
            },
        )]),
    )
    .unwrap();
    let app = app
        .build(
            BTreeSet::from(["cem-app".into()]),
            BTreeSet::from([query.clone()]),
            &c,
        )
        .unwrap();
    let (m, p) = fixture();
    let root = datatype(&m["app"].types.capture, "choice");
    let mut builder = EnvelopeBuilder::new(app).unwrap();
    let id = builder.add_assembly(graph(&m, &p), vec![], p).unwrap();
    let b = one(builder, id, root).unwrap();
    let v = envelope::verify(&b.bytes, &b.pin, &c).unwrap();
    assert_eq!(
        v.application()
            .receiver("cem-app", "cart-lines")
            .unwrap()
            .schema_uri,
        super::SCHEMA
    );
    assert!(v.application().query(&query).is_some());
    assert_eq!(v.application().contract_count(), 2);
}
#[test]
fn aggregate_module_inventory_is_bounded_even_with_shared_original_sources() {
    for count in [8, 9] {
        let mut builder = EnvelopeBuilder::new(base()).unwrap();
        let mut roots = BTreeMap::new();
        for instance in 0..8 {
            let core = leaf();
            let mut parent = "core".to_string();
            let mut root = *core.roots.first().unwrap();
            let mut modules = BTreeMap::from([("core".into(), assembly::ModuleSource::new(core))]);
            for i in 1..count {
                let name = format!("part{i}");
                let source = derived(&name, &parent, root);
                root = *source.roots.first().unwrap();
                let mut source = assembly::ModuleSource::new(source);
                source.imports.insert(parent);
                // Each graph has a distinct root alias while original source and
                // captures stay identical. Inner source agreement cannot bound
                // the aggregate restored module inventory on its own.
                parent = if i + 1 == count {
                    format!("root{instance}")
                } else {
                    name
                };
                modules.insert(parent.clone(), source);
            }
            let b = assembly::export(
                &parent,
                &modules,
                &[],
                &[],
                &hash(b"registry"),
                &OperationControl::default(),
            )
            .unwrap();
            let id = builder.add_assembly(b, vec![], vec![]).unwrap();
            roots.insert(
                format!("app{instance}"),
                AssemblyRoot {
                    assembly: id,
                    datatype: root,
                },
            );
        }
        assert_eq!(
            builder.build(roots, &OperationControl::default()).is_err(),
            count > 8
        );
    }
}

#[path = "mixed.rs"]
mod mixed;
