use super::*;
use cem_ml::{
    events::cem::CemEventNormalizer,
    schema::{machine::CemSchemaMachine, vocab::CompiledSchema},
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
fn sample() -> BuiltDatatypeModules {
    let text="@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:core | {types | {type @name=scalar @kind=scalar}}}";
    let capture = Arc::new(
        CemSchemaMachine::new(
            CompiledSchema::cem_core(),
            CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                SourceId(1),
                text.as_bytes().to_vec(),
            ))),
        )
        .build_with_lexical_scopes(),
    );
    let root = capture
        .document()
        .nodes
        .iter()
        .find_map(|n| match n {
            cem_ml::parser::CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "type" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let module = ModuleSource {
        identity: SourceIdentity::new("file:///app/core.cem", SourceId(1), text.as_bytes()),
        capture,
        roots: BTreeSet::from([root]),
        bindings: BTreeMap::new(),
    };
    export(
        "core",
        &BTreeMap::from([("core".into(), module)]),
        &[],
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap()
}
fn repin(c: &Capsule, old: &DatatypeModulesPin) -> (Vec<u8>, DatatypeModulesPin) {
    let bytes = [MAGIC, &encode(c).unwrap()].concat();
    let pin = DatatypeModulesPin {
        hash: hash(&bytes),
        ..old.clone()
    };
    (bytes, pin)
}
#[test]
fn repinned_versions_roots_fields_and_source_hash_changes_cannot_admit_a_module_graph() {
    let b = sample();
    for case in 0..12 {
        let mut c: Capsule = rmp_serde::from_slice(&b.bytes[MAGIC.len()..]).unwrap();
        match case {
            0 => c.abi.push_str("future"),
            1 => c.ml_version.push('!'),
            2 => c.ql_version.push('!'),
            3 => c.registry_hash = hash(b"other"),
            4 => c.root = "missing".into(),
            5 => {
                c.modules.get_mut("core").unwrap().roots.clear();
            }
            6 => {
                c.modules.get_mut("core").unwrap().roots = BTreeSet::from([u32::MAX]);
            }
            7 => {
                c.modules
                    .get_mut("core")
                    .unwrap()
                    .bindings
                    .insert(u32::MAX, ModuleTarget::LocalDatatype(0));
            }
            8 => c.modules.get_mut("core").unwrap().identity.byte_length = 0,
            9 => {
                let m = c.modules.remove("core").unwrap();
                c.root = "../bad".into();
                c.modules.insert(c.root.clone(), m);
            }
            10 => c.modules.clear(),
            _ => {
                for i in 0..513 {
                    c.modules
                        .get_mut("core")
                        .unwrap()
                        .bindings
                        .insert(i, ModuleTarget::LocalDatatype(0));
                }
            }
        }
        let (bytes, pin) = repin(&c, &b.pin);
        assert!(
            verify(&bytes, &pin, &[], &OperationControl::default()).is_err(),
            "case {case}"
        );
    }
}
#[test]
fn unknown_noncanonical_trailing_deep_and_oversized_wire_are_refused_before_admission() {
    let b = sample();
    let c: Capsule = rmp_serde::from_slice(&b.bytes[MAGIC.len()..]).unwrap();
    #[derive(Serialize)]
    struct Extra<'a> {
        #[serde(flatten)]
        capsule: &'a Capsule,
        unknown: bool,
    }
    for bytes in [
        [b.bytes.as_slice(), &[0]].concat(),
        [
            MAGIC,
            &encode(&Extra {
                capsule: &c,
                unknown: true,
            })
            .unwrap(),
        ]
        .concat(),
        [MAGIC, &rmp_serde::to_vec(&c).unwrap()].concat(),
        [MAGIC, &vec![0x91; 65], &[0xc0]].concat(),
        vec![0; MAX_BYTES + 1],
        [MAGIC, &[0xdf, 0xff, 0xff, 0xff, 0xff]].concat(),
    ] {
        let pin = DatatypeModulesPin {
            hash: hash(&bytes),
            ..b.pin.clone()
        };
        assert!(verify(&bytes, &pin, &[], &OperationControl::default()).is_err());
    }
    let legacy = dependencies_pin(&b);
    assert!(crate::datatype_dependency_capsule::verify(
        &b.bytes,
        &legacy,
        &[],
        &OperationControl::default()
    )
    .is_err());
}
fn dependencies_pin(
    b: &BuiltDatatypeModules,
) -> crate::datatype_dependency_capsule::DatatypeDependenciesPin {
    let c: Capsule = rmp_serde::from_slice(&b.bytes[MAGIC.len()..]).unwrap();
    crate::datatype_dependency_capsule::DatatypeDependenciesPin {
        hash: b.pin.hash.clone(),
        source: c.modules["core"].identity.clone(),
        registry_hash: b.pin.registry_hash.clone(),
    }
}
#[test]
fn module_count_and_per_record_byte_limits_apply_independently() {
    let b = sample();
    let mut c: Capsule = rmp_serde::from_slice(&b.bytes[MAGIC.len()..]).unwrap();
    for i in 0..33 {
        let original: Capsule = rmp_serde::from_slice(&b.bytes[MAGIC.len()..]).unwrap();
        c.modules.insert(
            format!("m{i}"),
            original.modules.into_values().next().unwrap(),
        );
    }
    let (bytes, pin) = repin(&c, &b.pin);
    assert!(verify(&bytes, &pin, &[], &OperationControl::default()).is_err());
    let mut c: Capsule = rmp_serde::from_slice(&b.bytes[MAGIC.len()..]).unwrap();
    c.modules.get_mut("core").unwrap().identity.uri = "x".repeat(MAX_MODULE_BYTES);
    let (bytes, pin) = repin(&c, &b.pin);
    assert_eq!(
        verify(&bytes, &pin, &[], &OperationControl::default())
            .err()
            .unwrap()
            .code,
        "datatype-module-graph-limit"
    );
}
