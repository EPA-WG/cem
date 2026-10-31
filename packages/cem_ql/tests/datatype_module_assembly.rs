use cem_ml::{
    content_cache::ContentHash,
    events::cem::CemEventNormalizer,
    operation_control::OperationControl,
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
    datatype_module_capsule::{self as modules, v2 as assembly, ModuleSource, ModuleTarget},
    native_registrations::{
        dependency_sources, profiles::v2 as profiles, profiles::OperationRole,
        ImplementationIdentity, OperationMode,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[path = "support/datatype_module_sources.rs"]
mod module_sources;
use module_sources::*;
fn node(c: &LexicallyScopedDocument, local: &str, n: usize) -> u32 {
    c.document()
        .nodes
        .iter()
        .filter_map(|x| match x {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == local => Some(*node_id),
            _ => None,
        })
        .nth(n)
        .unwrap()
}
fn reference(c: &LexicallyScopedDocument, text: &str) -> u32 {
    c.document()
        .nodes
        .iter()
        .find_map(|x| match x {
            CemAstNode::Reference {
                node_id,
                expression,
                ..
            } if expression == text => Some(*node_id),
            _ => None,
        })
        .unwrap()
}
fn wrapped(uri: &str, text: &str, root: Option<&str>) -> assembly::ModuleSource {
    let c = capture(text);
    let roots = root
        .map(|n| BTreeSet::from([datatype(&c, n)]))
        .unwrap_or_default();
    assembly::ModuleSource::new(ModuleSource {
        identity: SourceIdentity::new(uri, SourceId(1), text.as_bytes()),
        capture: c,
        roots,
        bindings: BTreeMap::new(),
    })
}
fn pack(m: &assembly::ModuleSource, name: &str) -> assembly::ProfileArtifact {
    let c = &m.types.capture;
    let schema = SchemaDeclarationNode::new(c.document().clone(), node(c, "schema", 0)).unwrap();
    let declaration = SchemaDeclarationNode::new(c.document().clone(), datatype(c, name)).unwrap();
    let mut registry = DatatypeRegistry::default();
    registry.insert(schema.clone(), declaration).unwrap();
    let source = registry.source(&schema, name).unwrap();
    let control = OperationControl::default();
    let a = dependency_sources::export_operation(
        c,
        &source,
        OperationMode::Constant,
        m.types.identity.clone(),
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
    let p = profiles::OperationProfile {
        source: m.types.identity.clone(),
        scope: schema.node_id(),
        declaration: source.declaration().node_id(),
        kind: cem_ml::schema::datatype_registry::DatatypeKind::Scalar,
        representation: cem_ml::schema::datatype_validation::ValueRepresentation::Scalar(
            ScalarRepresentation::String,
        ),
        dependencies: BTreeMap::from([(OperationRole::Constant, a.pin.clone())]),
    };
    let sources = vec![a];
    let b = profiles::export(
        &BTreeMap::from([("constant".into(), p)]),
        &sources,
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    assembly::ProfileArtifact {
        bytes: b.bytes,
        pin: b.pin,
        sources,
    }
}
fn fixture() -> (
    BTreeMap<String, assembly::ModuleSource>,
    Vec<assembly::ProfileArtifact>,
) {
    let core = assembly::ModuleSource::new(leaf());
    let palette=wrapped("file:///app/palette.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:palette | {constants | {constant @value=blue} {constant @value=red}}}",None);
    let mut app=wrapped("file:///app/app.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=choice @base={#base} | {#colors} {constant @value=local}}}}",Some("choice"));
    let c = app.types.capture.clone();
    app.types.bindings.insert(
        field(&c, "choice", "base"),
        ModuleTarget::ModuleDatatype {
            module: "core".into(),
            root: *core.types.roots.first().unwrap(),
        },
    );
    app.imports = BTreeSet::from(["core".into(), "palette".into()]);
    let mut palette = palette;
    let blue = node(&palette.types.capture, "constant", 0);
    let red = node(&palette.types.capture, "constant", 1);
    palette.constant_exports = BTreeSet::from([blue, red]);
    let target = |id| assembly::ModuleNodeId {
        module: "palette".into(),
        node: id,
    };
    app.constants.insert(
        reference(&c, "#colors"),
        vec![target(blue), target(red), target(blue)],
    );
    let artifact = pack(&app, "choice");
    app.profiles.insert(
        datatype(&c, "choice"),
        assembly::ProfileSelection {
            pin: artifact.pin.clone(),
            id: "constant".into(),
        },
    );
    (
        BTreeMap::from([
            ("core".into(), core),
            ("palette".into(), palette),
            ("app".into(), app),
        ]),
        vec![artifact],
    )
}
#[test]
fn versioned_module_assembly_keeps_original_profiles_constant_order_duplicates_and_inert_owners() {
    let (m, p) = fixture();
    let c = OperationControl::default();
    let b = assembly::export("app", &m, &[], &p, &hash(b"registry"), &c).unwrap();
    assert!(b.bytes.starts_with(assembly::MAGIC));
    let v = assembly::verify(&b.bytes, &b.pin, &[], &p, &c).unwrap();
    let copy = assembly::verify(&b.bytes, &b.pin, &[], &p, &c).unwrap();
    assert!(!v.selections_verified());
    assert!(!v.executable_closure_verified());
    assert_eq!(v.module_order(), &["core", "palette", "app"]);
    let app = v.module("app").unwrap();
    let id = datatype(&m["app"].types.capture, "choice");
    let profile = &app.profiles()[&id];
    assert!(Arc::ptr_eq(
        profile.source().declaration().document(),
        app.source().ast_owner()
    ));
    for req in profile.requirements().values() {
        assert!(Arc::ptr_eq(
            req.capture().document(),
            app.source().ast_owner()
        ));
    }
    let sites = app.constant_sites();
    assert_eq!(sites.len(), 2);
    assert_eq!(sites[0].targets.len(), 3);
    assert_eq!(
        sites[0].targets[0].identity(),
        sites[0].targets[2].identity()
    );
    assert!(sites[0].targets.iter().all(|n| Arc::ptr_eq(
        n.document(),
        v.module("palette").unwrap().source().ast_owner()
    )));
    assert!(Arc::ptr_eq(
        sites[1].targets[0].document(),
        app.source().ast_owner()
    ));
    assert!(!Arc::ptr_eq(
        app.source().ast_owner(),
        copy.module("app").unwrap().source().ast_owner()
    ));
    for (name, original) in &m {
        let verified = v.module(name).unwrap();
        for occurrence in original.types.capture.occurrences() {
            assert_eq!(
                rmp_serde::to_vec_named(
                    &original
                        .types
                        .capture
                        .snapshot(original.types.capture.document(), occurrence)
                )
                .unwrap(),
                rmp_serde::to_vec_named(
                    &verified
                        .capture()
                        .snapshot(verified.capture().document(), occurrence)
                )
                .unwrap()
            );
        }
    }
    assert_eq!(
        b.bytes,
        assembly::export("app", &m, &[], &p, &hash(b"registry"), &c)
            .unwrap()
            .bytes
    );
}
#[test]
fn versioned_module_assembly_requires_exact_imports_exports_and_used_constant_bindings() {
    for case in 0..6 {
        let (mut m, p) = fixture();
        match case {
            0 => {
                m.get_mut("app").unwrap().imports.remove("palette");
            }
            1 => {
                m.get_mut("app").unwrap().imports.insert("unused".into());
            }
            2 => m.get_mut("palette").unwrap().constant_exports.clear(),
            3 => m.get_mut("app").unwrap().constants.clear(),
            4 => {
                m.get_mut("app").unwrap().constants.insert(u32::MAX, vec![]);
            }
            _ => {
                let app = m.get_mut("app").unwrap();
                app.constants.values_mut().next().unwrap()[0].node = 0;
            }
        }
        assert!(
            assembly::export(
                "app",
                &m,
                &[],
                &p,
                &hash(b"registry"),
                &OperationControl::default()
            )
            .is_err(),
            "{case}"
        );
    }
}
#[test]
fn versioned_module_assembly_refuses_missing_unused_foreign_and_disagreeing_profiles() {
    let (mut m, mut p) = fixture();
    let c = OperationControl::default();
    assert!(assembly::export("app", &m, &[], &[], &hash(b"registry"), &c).is_err());
    let selection = m.get_mut("app").unwrap().profiles.pop_first().unwrap();
    assert!(assembly::export("app", &m, &[], &p, &hash(b"registry"), &c).is_err());
    m.get_mut("app")
        .unwrap()
        .profiles
        .insert(selection.0, selection.1);
    let a = pack(&m["core"], "scalar");
    p.push(a);
    assert!(assembly::export("app", &m, &[], &p, &hash(b"registry"), &c).is_err());
    p.pop();
    let old = m.get_mut("app").unwrap().types.identity.hash.clone();
    m.get_mut("app").unwrap().types.identity.hash = hash(b"wrong-original-source");
    assert!(assembly::export("app", &m, &[], &p, &hash(b"registry"), &c).is_err());
    m.get_mut("app").unwrap().types.identity.hash = old;
    let app = m.get_mut("app").unwrap();
    let capture = app.types.capture.clone();
    app.profiles.insert(
        node(&capture, "constant", 0),
        app.profiles.values().next().unwrap().clone(),
    );
    assert!(assembly::export("app", &m, &[], &p, &hash(b"registry"), &c).is_err());
}

#[test]
fn versioned_module_assembly_flattens_reference_chains_in_order_without_interpreting_values() {
    let (mut m, p) = fixture();
    let mut palette=wrapped("file:///app/palette.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:palette | {constants | {#more} {constant @value=blue} {constant @value=red}}}",None);
    let r = reference(&palette.types.capture, "#more");
    let blue = node(&palette.types.capture, "constant", 0);
    let red = node(&palette.types.capture, "constant", 1);
    palette.constant_exports.insert(r);
    palette.constants.insert(
        r,
        vec![
            assembly::ModuleNodeId {
                module: "palette".into(),
                node: red,
            },
            assembly::ModuleNodeId {
                module: "palette".into(),
                node: blue,
            },
        ],
    );
    m.get_mut("app")
        .unwrap()
        .constants
        .values_mut()
        .next()
        .unwrap()
        .clear();
    m.get_mut("app")
        .unwrap()
        .constants
        .values_mut()
        .next()
        .unwrap()
        .extend([
            assembly::ModuleNodeId {
                module: "palette".into(),
                node: r,
            },
            assembly::ModuleNodeId {
                module: "palette".into(),
                node: r,
            },
        ]);
    m.insert("palette".into(), palette);
    let c = OperationControl::default();
    let b = assembly::export("app", &m, &[], &p, &hash(b"registry"), &c).unwrap();
    let v = assembly::verify(&b.bytes, &b.pin, &[], &p, &c).unwrap();
    let site = &v.module("app").unwrap().constant_sites()[0];
    assert_eq!(
        site.targets.iter().map(|n| n.node_id()).collect::<Vec<_>>(),
        vec![red, blue, red, blue]
    );
    assert!(!v.selections_verified());
}
#[test]
fn versioned_module_assembly_reports_original_constant_and_module_cycle_witnesses() {
    let (mut m, p) = fixture();
    let app = m.get_mut("app").unwrap();
    let r = reference(&app.types.capture, "#colors");
    app.constants.insert(
        r,
        vec![assembly::ModuleNodeId {
            module: "app".into(),
            node: r,
        }],
    );
    let err = assembly::export(
        "app",
        &m,
        &[],
        &p,
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .err()
    .unwrap();
    assert_eq!(err.code, "datatype-module-constant-cycle");
    assert!(err.related.iter().all(|n| n.node_id() == r));
    let (mut m, p) = fixture();
    let app_root = *m["app"].types.roots.first().unwrap();
    let mut core=wrapped("file:///app/types/core.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:core | {types | {type @name=scalar @kind=scalar @base={#app}}}}",Some("scalar"));
    core.types.bindings.insert(
        field(&core.types.capture, "scalar", "base"),
        ModuleTarget::ModuleDatatype {
            module: "app".into(),
            root: app_root,
        },
    );
    core.imports.insert("app".into());
    let root = *core.types.roots.first().unwrap();
    m.insert("core".into(), core);
    *m.get_mut("app")
        .unwrap()
        .types
        .bindings
        .values_mut()
        .next()
        .unwrap() = ModuleTarget::ModuleDatatype {
        module: "core".into(),
        root,
    };
    let err = assembly::export(
        "app",
        &m,
        &[],
        &p,
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .err()
    .unwrap();
    assert_eq!(err.code, "datatype-module-cycle");
    assert_eq!(err.related.len(), 2);
    assert_eq!(err.cycle_modules.first(), err.cycle_modules.last());
}
#[test]
fn versioned_module_assembly_bounds_reference_depth_and_slot_expansion() {
    for (length, valid) in [(30, true), (32, false)] {
        let (mut m, p) = fixture();
        let refs = (0..length)
            .map(|i| format!("{{#r{i}}}"))
            .collect::<String>();
        let text=format!("@ns s = https://cem.dev/ns/schema/1\n@default s\n{{schema @namespace=urn:palette | {{constants | {refs} {{constant @value=blue}}}}}}");
        let mut palette = wrapped("file:///app/palette.cem", &text, None);
        let terminal = node(&palette.types.capture, "constant", 0);
        for i in 0..length {
            let r = reference(&palette.types.capture, &format!("#r{i}"));
            let target = if i + 1 == length {
                terminal
            } else {
                reference(&palette.types.capture, &format!("#r{}", i + 1))
            };
            palette.constants.insert(
                r,
                vec![assembly::ModuleNodeId {
                    module: "palette".into(),
                    node: target,
                }],
            );
        }
        let r = reference(&palette.types.capture, "#r0");
        palette.constant_exports.insert(r);
        m.insert("palette".into(), palette);
        *m.get_mut("app")
            .unwrap()
            .constants
            .values_mut()
            .next()
            .unwrap() = vec![assembly::ModuleNodeId {
            module: "palette".into(),
            node: r,
        }];
        let result = assembly::export(
            "app",
            &m,
            &[],
            &p,
            &hash(b"registry"),
            &OperationControl::default(),
        );
        assert_eq!(
            result.is_ok(),
            valid,
            "length={length} {:?}",
            result.err().map(|e| e.code)
        );
    }
    let (mut m, p) = fixture();
    let app = m.get_mut("app").unwrap();
    let target = app.constants.values().next().unwrap()[0].clone();
    app.constants
        .values_mut()
        .next()
        .unwrap()
        .resize(129, target);
    assert!(assembly::export(
        "app",
        &m,
        &[],
        &p,
        &hash(b"registry"),
        &OperationControl::default()
    )
    .is_err());
    // A short input graph can still expand beyond the per-selection allowance.
    let (mut m, p) = fixture();
    let mut palette=wrapped("file:///app/palette.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:palette | {constants | {#more} {constant @value=blue}}}",None);
    let r = reference(&palette.types.capture, "#more");
    let leaf = node(&palette.types.capture, "constant", 0);
    palette.constant_exports.insert(r);
    palette.constants.insert(
        r,
        vec![
            assembly::ModuleNodeId {
                module: "palette".into(),
                node: leaf
            };
            65
        ],
    );
    m.insert("palette".into(), palette);
    *m.get_mut("app")
        .unwrap()
        .constants
        .values_mut()
        .next()
        .unwrap() = vec![
        assembly::ModuleNodeId {
            module: "palette".into(),
            node: r
        };
        2
    ];
    let err = assembly::export(
        "app",
        &m,
        &[],
        &p,
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .err()
    .unwrap();
    assert_eq!(err.code, "datatype-module-constant-limit");
}
#[test]
fn versioned_module_assembly_preserves_legacy_graphs_and_leaf_contracts_without_reader_aliasing() {
    let old = chain();
    let mut m = BTreeMap::new();
    for (id, types) in old {
        let imports = types
            .bindings
            .values()
            .filter_map(|t| match t {
                ModuleTarget::ModuleDatatype { module, .. } => Some(module.clone()),
                _ => None,
            })
            .collect();
        let mut module = assembly::ModuleSource::new(types);
        module.imports = imports;
        m.insert(id, module);
    }
    let c = OperationControl::default();
    let b = assembly::export("app", &m, &[], &[], &hash(b"registry"), &c).unwrap();
    let pin = modules::DatatypeModulesPin {
        hash: b.pin.hash.clone(),
        registry_hash: b.pin.registry_hash.clone(),
        root: b.pin.root.clone(),
    };
    assert!(modules::verify(&b.bytes, &pin, &[], &c).is_err());
    let old = modules::export("app", &chain(), &[], &hash(b"registry"), &c).unwrap();
    let pin = assembly::DatatypeModulesPin {
        hash: old.pin.hash.clone(),
        registry_hash: old.pin.registry_hash.clone(),
        root: old.pin.root.clone(),
    };
    assert!(assembly::verify(&old.bytes, &pin, &[], &[], &c).is_err());
    let leaf = leaf();
    let root = *leaf.roots.first().unwrap();
    let dt = cem_ql::datatype_source_capsule::export(
        &leaf.capture,
        leaf.identity.clone(),
        &hash(b"registry"),
        leaf.roots,
        &c,
    )
    .unwrap();
    let mut app=wrapped("file:///app/import.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=selected @base={#leaf}}}}",Some("selected"));
    app.types.bindings.insert(
        field(&app.types.capture, "selected", "base"),
        ModuleTarget::Datatype {
            pin: dt.pin.clone(),
            root,
        },
    );
    let artifacts = [cem_ql::datatype_dependency_capsule::DependencyArtifact::datatype(dt)];
    let b = assembly::export(
        "app",
        &BTreeMap::from([("app".into(), app)]),
        &artifacts,
        &[],
        &hash(b"registry"),
        &c,
    )
    .unwrap();
    assert_eq!(
        assembly::verify(&b.bytes, &b.pin, &artifacts, &[], &c)
            .unwrap()
            .leaf_ids()
            .count(),
        1
    );
}
#[test]
fn versioned_module_assembly_checks_complete_profile_sources_and_nested_profile_limits() {
    let (mut m, p) = fixture();
    let original = m["app"].types.capture.clone();
    let changed="@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:zzz | {types | {type @name=choice @base={#base} | {#colors} {constant @value=local}}}}";
    m.get_mut("app").unwrap().types.capture = capture(changed);
    assert!(assembly::export(
        "app",
        &m,
        &[],
        &p,
        &hash(b"registry"),
        &OperationControl::default()
    )
    .is_err());
    m.get_mut("app").unwrap().types.capture = original;
    for case in 0..3 {
        let (m, mut p) = fixture();
        match case {
            0 => p[0].bytes.push(0),
            1 => p[0].sources[0].bytes.push(0),
            _ => p[0].sources[0].bytes = vec![0; 64 * 1024 + 1],
        }
        assert!(assembly::export(
            "app",
            &m,
            &[],
            &p,
            &hash(b"registry"),
            &OperationControl::default()
        )
        .is_err());
    }
}

#[test]
fn versioned_module_assembly_refuses_repinned_malformed_versions_bounds_and_cancellation() {
    use cem_ml::schema::native_source::dependencies::NativeDependencySourceSnapshot;
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Types {
        identity: SourceIdentity,
        roots: BTreeSet<AstNodeId>,
        bindings: BTreeMap<AstNodeId, ModuleTarget>,
        source: NativeDependencySourceSnapshot,
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Module {
        types: Types,
        imports: BTreeSet<String>,
        constant_exports: BTreeSet<AstNodeId>,
        constants: BTreeMap<AstNodeId, Vec<assembly::ModuleNodeId>>,
        profiles: BTreeMap<AstNodeId, assembly::ProfileSelection>,
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Wire {
        abi: String,
        ml_version: String,
        ql_version: String,
        registry_hash: String,
        root: String,
        modules: BTreeMap<String, Module>,
    }
    let (m, p) = fixture();
    let control = OperationControl::default();
    let b = assembly::export("app", &m, &[], &p, &hash(b"registry"), &control).unwrap();
    let raw = &b.bytes[assembly::MAGIC.len()..];
    let body: Wire = rmp_serde::from_slice(raw).unwrap();
    assert_eq!(rmp_serde::to_vec_named(&body).unwrap(), raw);
    for case in 0..10 {
        let mut w: Wire = rmp_serde::from_slice(raw).unwrap();
        match case {
            0 => w.abi = "unsupported".into(),
            1 => w.ml_version = "unsupported".into(),
            2 => w.ql_version = "unsupported".into(),
            3 => w.registry_hash = hash(b"other"),
            4 => w.root = "unknown".into(),
            5 => w.modules.clear(),
            6 => {
                w.modules.get_mut("app").unwrap().types.roots.clear();
            }
            7 => {
                w.modules
                    .get_mut("app")
                    .unwrap()
                    .profiles
                    .values_mut()
                    .next()
                    .unwrap()
                    .id = "missing".into();
            }
            8 => {
                w.modules.get_mut("app").unwrap().types.identity.hash = hash(b"other-source");
            }
            _ => {
                let node = *w.modules["app"].constants.keys().next().unwrap();
                let target = w.modules["app"].constants[&node][0].clone();
                w.modules
                    .get_mut("app")
                    .unwrap()
                    .constants
                    .get_mut(&node)
                    .unwrap()
                    .resize(129, target);
            }
        }
        let bytes = [assembly::MAGIC, &rmp_serde::to_vec_named(&w).unwrap()].concat();
        let mut pin = b.pin.clone();
        pin.hash = hash(&bytes);
        assert!(
            assembly::verify(&bytes, &pin, &[], &p, &control).is_err(),
            "{case}"
        );
    }
    #[derive(serde::Serialize)]
    struct Extra {
        #[serde(flatten)]
        capsule: Wire,
        unknown: bool,
    }
    let extra = [
        assembly::MAGIC,
        &rmp_serde::to_vec_named(&Extra {
            capsule: body,
            unknown: true,
        })
        .unwrap(),
    ]
    .concat();
    for bytes in [
        extra,
        [b.bytes.as_slice(), &[0]].concat(),
        [b"CEMDM3\n", raw].concat(),
        [assembly::MAGIC, &vec![0x91; 80], &[0xc0]].concat(),
        vec![0; 4 * 1024 * 1024 + 1],
    ] {
        let mut pin = b.pin.clone();
        pin.hash = hash(&bytes);
        assert!(assembly::verify(&bytes, &pin, &[], &p, &control).is_err());
    }
    let mut oversized = fixture().1;
    oversized[0].sources[0].bytes = vec![0; 4 * 1024 * 1024];
    assert!(assembly::verify(&b.bytes, &b.pin, &[], &oversized, &control).is_err());
    let aggregate = (0..32)
        .map(|_| assembly::ProfileArtifact {
            bytes: vec![0; 64 * 1024],
            pin: p[0].pin.clone(),
            sources: vec![dependency_sources::BuiltOperationSource {
                bytes: vec![0; 64 * 1024],
                pin: p[0].sources[0].pin.clone(),
            }],
        })
        .collect::<Vec<_>>();
    assert!(assembly::verify(&b.bytes, &b.pin, &[], &aggregate, &control).is_err());
    let too_many = (0..33)
        .map(|i| (format!("m{i}"), m["core"].clone()))
        .collect();
    assert!(assembly::export("m0", &too_many, &[], &[], &hash(b"registry"), &control).is_err());
    let mut module_too_large = m.clone();
    let text=format!("@ns s = https://cem.dev/ns/schema/1\n@default s\n{{schema @namespace=urn:palette | {{constants | {{constant @value=\"{}\"}}}}}}","a".repeat(64*1024));
    let mut palette = wrapped("file:///app/palette.cem", &text, None);
    palette
        .constant_exports
        .insert(node(&palette.types.capture, "constant", 0));
    module_too_large.insert("palette".into(), palette);
    assert!(assembly::export(
        "app",
        &module_too_large,
        &[],
        &p,
        &hash(b"registry"),
        &control
    )
    .is_err());
    let stopped = OperationControl::default();
    stopped.cancel_root(None, None).unwrap();
    assert!(assembly::verify(&b.bytes, &b.pin, &[], &p, &stopped).is_err());
    assert!(assembly::export("app", &m, &[], &p, &hash(b"registry"), &stopped).is_err());
}
