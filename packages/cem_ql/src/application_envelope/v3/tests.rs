use super::*;
use cem_ml::{
    parser::CemAstNode,
    schema::{structural_capsule::SourceIdentity, vocab::CompiledSchema},
    source::SourceId,
};
fn fixture() -> EnvelopeBuilder {
    let c = OperationControl::default();
    let registry = hash(b"registry");
    let d = crate::declaration_artifact::build_declaration_artifact(
        "<cem-element tag=\"cem-app\"><template id=\"cem-app\" type=\"text/cem-ml\">{p | Hello}</template></cem-element>",
        "file:///app/main.xhtml", crate::template_artifact::TemplateArtifactSourceMapMode::Dev, None,
    ).unwrap();
    let mut app = super::super::EnvelopeBuilder::new(&registry).unwrap();
    app.add_component(d, BTreeMap::new()).unwrap();
    let app = app
        .build(BTreeSet::from(["cem-app".into()]), BTreeSet::new(), &c)
        .unwrap();
    let mut builder = EnvelopeBuilder::new(app).unwrap();
    let text = "@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=scalar @kind=scalar}}}";
    let capture = cem_ml::import::import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "file:///app/types.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
    .captured;
    let root = capture
        .document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "type" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let module = crate::datatype_module_capsule::ModuleSource {
        identity: SourceIdentity::new("file:///app/types.cem", SourceId(1), text.as_bytes()),
        capture,
        roots: BTreeSet::from([root]),
        bindings: BTreeMap::new(),
    };
    let mut module = assembly::ModuleSource::new(module);
    let profile = profile(&module, root);
    module.profiles.insert(
        root,
        assembly::ProfileSelection {
            pin: profile.pin.clone(),
            id: "constants".into(),
        },
    );
    let modules = BTreeMap::from([("app".into(), module)]);
    let graph = assembly::export(
        "app",
        &modules,
        &[],
        std::slice::from_ref(&profile),
        &registry,
        &c,
    )
    .unwrap();
    let id = builder.add_assembly(graph, vec![], vec![profile]).unwrap();
    builder.manifest.roots.insert(
        "main".into(),
        AssemblyRoot {
            assembly: id,
            datatype: root,
        },
    );
    builder
}
fn profile(module: &assembly::ModuleSource, root: AstNodeId) -> assembly::ProfileArtifact {
    use crate::native_registrations::{
        profiles::OperationRole, ImplementationIdentity, OperationMode,
    };
    use cem_ml::schema::{
        datatype_registry::{DatatypeKind, DatatypeRegistry},
        datatype_validation::{ScalarRepresentation, ValueRepresentation},
        declaration_references::SchemaDeclarationNode,
    };
    let c = &module.types.capture;
    let schema = c
        .document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "schema" => {
                SchemaDeclarationNode::new(c.document().clone(), *node_id)
            }
            _ => None,
        })
        .unwrap();
    let mut registry = DatatypeRegistry::default();
    registry
        .insert(
            schema.clone(),
            SchemaDeclarationNode::new(c.document().clone(), root).unwrap(),
        )
        .unwrap();
    let source = registry.source(&schema, "scalar").unwrap();
    let control = OperationControl::default();
    let operation = operations::export_operation(
        c,
        &source,
        OperationMode::Constant,
        module.types.identity.clone(),
        ImplementationIdentity::constant_interpreter(
            "constant",
            hash(b"native"),
            "constant",
            ScalarRepresentation::String,
        ),
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    let record = profiles::OperationProfile {
        source: module.types.identity.clone(),
        scope: schema.node_id(),
        declaration: root,
        kind: DatatypeKind::Scalar,
        representation: ValueRepresentation::Scalar(ScalarRepresentation::String),
        dependencies: BTreeMap::from([(OperationRole::Constant, operation.pin.clone())]),
    };
    let sources = vec![operation];
    let profile = profiles::export(
        &BTreeMap::from([("constants".into(), record)]),
        &sources,
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    assembly::ProfileArtifact {
        bytes: profile.bytes,
        pin: profile.pin,
        sources,
    }
}
fn packed(e: &Envelope) -> BuiltApplicationEnvelope {
    let bytes = [MAGIC, &encode(e).unwrap()].concat();
    BuiltApplicationEnvelope {
        pin: ApplicationEnvelopePin {
            hash: hash(&bytes),
            manifest_hash: hash(&encode(&e.manifest).unwrap()),
            registry_hash: e.manifest.registry_hash.clone(),
        },
        bytes,
    }
}
fn raw() -> Envelope {
    let b = fixture();
    Envelope {
        manifest: b.manifest,
        blobs: b.blobs,
    }
}
#[test]
fn assembly_application_repinned_manifest_inventory_and_root_changes_refuse() {
    for case in 0..28 {
        let mut e = raw();
        let id = e.manifest.assemblies.keys().next().unwrap().clone();
        match case {
            0 => e.manifest.abi.push_str("future"),
            1 => e.manifest.ml_version.push_str("future"),
            2 => e.manifest.ql_version.push_str("future"),
            3 => e.manifest.application.manifest_hash = hash(b"other"),
            4 => e.manifest.application.registry_hash = hash(b"other"),
            5 => e.manifest.roots.clear(),
            6 => e.manifest.roots.get_mut("main").unwrap().datatype = u32::MAX,
            7 => e.manifest.roots.get_mut("main").unwrap().assembly = hash(b"missing"),
            8 => {
                e.manifest
                    .roots
                    .insert("bad root".into(), e.manifest.roots["main"].clone());
            }
            9 => e.manifest.assemblies.get_mut(&id).unwrap().pin.root = "absent".into(),
            10 => {
                e.manifest
                    .assemblies
                    .get_mut(&id)
                    .unwrap()
                    .pin
                    .registry_hash = hash(b"other")
            }
            11 => e.manifest.assemblies.get_mut(&id).unwrap().pin.hash = hash(b"other"),
            12 => {
                e.manifest
                    .assemblies
                    .get_mut(&id)
                    .unwrap()
                    .leaves
                    .insert(hash(b"missing"));
            }
            13 => {
                e.manifest
                    .assemblies
                    .get_mut(&id)
                    .unwrap()
                    .profiles
                    .insert(hash(b"missing"));
            }
            14 => {
                e.blobs.insert(hash(b"orphan"), Blob(b"orphan".to_vec()));
            }
            15 => {
                e.blobs.remove(&id);
            }
            16 => e.blobs.get_mut(&id).unwrap().0.push(0),
            17 => {
                let record = e.manifest.assemblies.remove(&id).unwrap();
                e.manifest.assemblies.insert(hash(b"wrong-key"), record);
            }
            18 => {
                e.manifest.operations.insert(
                    hash(b"unused"),
                    operations::OperationSourcePin {
                        hash: hash(b"unused"),
                        source: SourceIdentity::new("file:///unused.cem", SourceId(1), b"unused"),
                        registry_hash: e.manifest.registry_hash.clone(),
                    },
                );
            }
            19 => {
                e.manifest.profiles.values_mut().next().unwrap().pin.hash = hash(b"wrong profile")
            }
            20 => {
                e.manifest
                    .profiles
                    .values_mut()
                    .next()
                    .unwrap()
                    .pin
                    .registry_hash = hash(b"wrong registry")
            }
            21 => e
                .manifest
                .profiles
                .values_mut()
                .next()
                .unwrap()
                .sources
                .clear(),
            22 => {
                e.manifest.operations.values_mut().next().unwrap().hash = hash(b"wrong operation")
            }
            23 => {
                e.manifest
                    .operations
                    .values_mut()
                    .next()
                    .unwrap()
                    .source
                    .hash = hash(b"wrong source")
            }
            24 => {
                e.manifest
                    .operations
                    .values_mut()
                    .next()
                    .unwrap()
                    .registry_hash = hash(b"wrong registry")
            }
            25 => {
                let key = e.manifest.profiles.keys().next().unwrap().clone();
                e.manifest.profiles.remove(&key);
            }
            26 => {
                let key = e.manifest.operations.keys().next().unwrap().clone();
                e.manifest.operations.remove(&key);
            }
            _ => {
                let key = e.manifest.operations.keys().next().unwrap().clone();
                let mut record = e.manifest.operations[&key].clone();
                record.hash = hash(b"unused operation");
                e.manifest.operations.insert(record.hash.clone(), record);
            }
        }
        let b = packed(&e);
        assert!(
            verify(&b.bytes, &b.pin, &OperationControl::default()).is_err(),
            "case {case}"
        );
    }
}
#[test]
fn assembly_application_opaque_binary_unknown_trailing_deep_and_oversized_inputs_refuse() {
    let e = raw();
    let built = packed(&e);
    let mut variants = vec![
        [built.bytes.as_slice(), &[0]].concat(),
        [MAGIC, &vec![0x91; 65], &[0xc0]].concat(),
        [MAGIC, &[0xdf, 0xff, 0xff, 0xff, 0xff]].concat(),
        vec![0; MAX_BYTES + 1],
    ];
    #[derive(Serialize)]
    struct Unknown<'a> {
        manifest: &'a Manifest,
        blobs: &'a BTreeMap<String, Blob>,
        unknown: bool,
    }
    variants.push(
        [
            MAGIC,
            &encode(&Unknown {
                manifest: &e.manifest,
                blobs: &e.blobs,
                unknown: true,
            })
            .unwrap(),
        ]
        .concat(),
    );
    #[derive(Serialize)]
    struct Arrays<'a> {
        manifest: &'a Manifest,
        blobs: BTreeMap<String, Vec<u8>>,
    }
    variants.push(
        [
            MAGIC,
            &encode(&Arrays {
                manifest: &e.manifest,
                blobs: e
                    .blobs
                    .iter()
                    .map(|(id, b)| (id.clone(), b.0.clone()))
                    .collect(),
            })
            .unwrap(),
        ]
        .concat(),
    );
    for bytes in variants {
        let pin = ApplicationEnvelopePin {
            hash: hash(&bytes),
            ..built.pin.clone()
        };
        assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
    }
    let mut changed = raw();
    changed
        .manifest
        .roots
        .insert("alias".into(), changed.manifest.roots["main"].clone());
    let bytes = packed(&changed).bytes;
    let pin = ApplicationEnvelopePin {
        hash: hash(&bytes),
        ..built.pin
    };
    assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
}
#[test]
fn assembly_application_root_limits_and_failed_staging_preserve_usable_builder() {
    let mut b = fixture();
    let root = b.manifest.roots["main"].clone();
    let roots = (0..64)
        .map(|i| (format!("root-{i}"), root.clone()))
        .collect::<BTreeMap<_, _>>();
    assert!(b
        .clone()
        .build(roots.clone(), &OperationControl::default())
        .is_ok());
    let mut too_many = roots;
    too_many.insert("overflow".into(), root.clone());
    assert!(b
        .clone()
        .build(too_many, &OperationControl::default())
        .is_err());
    let previous = encode(&b.manifest).unwrap();
    let record = b.manifest.assemblies[&root.assembly].clone();
    let mut bytes = b.blobs[&root.assembly].0.clone();
    bytes.push(0);
    assert!(b
        .add_assembly(
            assembly::BuiltDatatypeModules {
                bytes,
                pin: record.pin
            },
            vec![],
            vec![]
        )
        .is_err());
    assert_eq!(encode(&b.manifest).unwrap(), previous);
    assert!(b
        .build(
            BTreeMap::from([("main".into(), root)]),
            &OperationControl::default()
        )
        .is_ok());
}
#[test]
fn assembly_application_inventory_count_edges_and_byte_limits_are_independent() {
    let b = fixture();
    let a = b.manifest.assemblies.values().next().unwrap().clone();
    for count in [8, 9] {
        let mut m = b.manifest.clone();
        m.assemblies = (0..count).map(|i| (hash(&[i]), a.clone())).collect();
        assert_eq!(inventory::shape(&m).is_err(), count > 8);
    }
    let mut m = b.manifest.clone();
    let p = ProfileRecord {
        pin: profiles::OperationProfilesPin {
            hash: hash(b"profile"),
            registry_hash: m.registry_hash.clone(),
        },
        sources: BTreeSet::new(),
    };
    for count in [32, 33] {
        m.profiles = (0..count).map(|i| (hash(&[i]), p.clone())).collect();
        assert_eq!(inventory::shape(&m).is_err(), count > 32);
    }
    m.profiles.clear();
    let source = operations::OperationSourcePin {
        hash: hash(b"source"),
        source: SourceIdentity::new("file:///app/source.cem", SourceId(1), b"source"),
        registry_hash: m.registry_hash.clone(),
    };
    for count in [64, 65] {
        m.operations = (0..count).map(|i| (hash(&[i]), source.clone())).collect();
        assert_eq!(inventory::shape(&m).is_err(), count > 64);
    }
    let mut staging = b;
    assert!(staging
        .blob(&hash(&vec![0; MAX_BYTES]), vec![0; MAX_BYTES])
        .is_err());
    let mut m = staging.manifest.clone();
    let keys = (0..64).map(|i| hash(&[i])).collect::<BTreeSet<_>>();
    let mut a = a;
    a.profiles.clear();
    a.leaves = keys;
    m.assemblies = (0..4).map(|i| (hash(&[i]), a.clone())).collect();
    assert!(inventory::shape(&m).is_ok());
    m.assemblies.insert(hash(b"fifth"), a);
    assert!(inventory::shape(&m).is_err());
}
