use super::*;
use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::CemAstNode,
    schema::{machine::CemSchemaMachine, vocab::CompiledSchema},
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
fn sample() -> BuiltDatatypeDependencies {
    let text="@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:test | {types | {type @name=base @kind=scalar} {type @name=child @base={#base}}}}";
    let c = CemSchemaMachine::new(
        CompiledSchema::cem_core(),
        CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
            SourceId(1),
            text.as_bytes().to_vec(),
        ))),
    )
    .build_with_lexical_scopes();
    let types = c
        .document()
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "type" => Some(*node_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    let field = c
        .document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Attribute {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "base" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    export(
        &c,
        SourceIdentity::new("file:///app/types.cem", SourceId(1), text.as_bytes()),
        &hash(b"registry"),
        BTreeSet::from([types[1]]),
        BTreeMap::from([(field, DependencyTarget::LocalDatatype(types[0]))]),
        &[],
        &OperationControl::default(),
    )
    .unwrap()
}
#[test]
fn repinned_versions_roots_fields_and_target_mutations_refuse() {
    let b = sample();
    for case in 0..9 {
        let mut c: Capsule = rmp_serde::from_slice(&b.bytes[MAGIC.len()..]).unwrap();
        match case {
            0 => c.abi.push_str("future"),
            1 => c.ml_version.push('!'),
            2 => c.ql_version.push('!'),
            3 => c.roots.clear(),
            4 => {
                c.roots.insert(u32::MAX);
            }
            5 => {
                c.bindings.clear();
            }
            6 => {
                c.bindings
                    .insert(u32::MAX, DependencyTarget::LocalDatatype(0));
            }
            7 => {
                *c.bindings.values_mut().next().unwrap() =
                    DependencyTarget::LocalDatatype(*c.roots.first().unwrap());
            }
            _ => c.identity.uri = "file:///other.cem".into(),
        }
        let bytes = [MAGIC, &encode(&c).unwrap()].concat();
        let pin = DatatypeDependenciesPin {
            hash: hash(&bytes),
            ..b.pin.clone()
        };
        assert!(
            verify(&bytes, &pin, &[], &OperationControl::default()).is_err(),
            "case {case}"
        );
    }
}
#[test]
fn noncanonical_unknown_trailing_deep_and_oversized_binary_refuse() {
    let b = sample();
    #[derive(Serialize)]
    struct Extra<'a> {
        #[serde(flatten)]
        capsule: &'a Capsule,
        unknown: bool,
    }
    let c: Capsule = rmp_serde::from_slice(&b.bytes[MAGIC.len()..]).unwrap();
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
        [
            MAGIC,
            &std::iter::repeat_n(0x91, 65).collect::<Vec<_>>(),
            &[0xc0],
        ]
        .concat(),
        vec![0; MAX_BYTES + 1],
        [MAGIC, &[0xdf, 0xff, 0xff, 0xff, 0xff]].concat(),
    ] {
        let pin = DatatypeDependenciesPin {
            hash: hash(&bytes),
            ..b.pin.clone()
        };
        assert!(verify(&bytes, &pin, &[], &OperationControl::default()).is_err());
    }
}
#[test]
fn local_graph_root_type_and_depth_boundaries_are_independent_of_wire_size() {
    let capture = |types: String| {
        let text=format!("@ns s = https://cem.dev/ns/schema/1\n@default s\n{{schema @namespace=urn:test | {{types | {types}}}}}");
        Arc::new(
            CemSchemaMachine::new(
                CompiledSchema::cem_core(),
                CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                    SourceId(1),
                    text.as_bytes().to_vec(),
                ))),
            )
            .build_with_lexical_scopes(),
        )
    };
    let ids = |c: &LexicallyScopedDocument| {
        c.document()
            .nodes
            .iter()
            .filter_map(|n| match n {
                CemAstNode::Element {
                    node_id,
                    expanded_name,
                    ..
                } if expanded_name.local_name == "type" => Some(*node_id),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    for (count, valid) in [(64, true), (65, false)] {
        let c = capture(
            (0..count)
                .map(|i| format!("{{type @name=t{i} @kind=scalar}} "))
                .collect(),
        );
        assert_eq!(
            graph::inspect(
                &c,
                &BTreeSet::from([ids(&c)[0]]),
                &BTreeMap::new(),
                &BTreeMap::new(),
                &Work::new(&OperationControl::default())
            )
            .is_ok(),
            valid
        );
    }
    let c = capture(
        (0..33)
            .map(|i| format!("{{type @name=t{i} @kind=scalar}} "))
            .collect(),
    );
    for (count, valid) in [(32, true), (33, false)] {
        assert_eq!(
            graph::inspect(
                &c,
                &ids(&c)[..count].iter().copied().collect(),
                &BTreeMap::new(),
                &BTreeMap::new(),
                &Work::new(&OperationControl::default())
            )
            .is_ok(),
            valid
        );
    }
    for (count, valid) in [(33, true), (34, false)] {
        let c = capture(
            (0..count)
                .map(|i| {
                    if i == 0 {
                        "{type @name=t0 @kind=scalar} ".into()
                    } else {
                        format!("{{type @name=t{i} @base=t{}}} ", i - 1)
                    }
                })
                .collect(),
        );
        let types = ids(&c);
        let mut bindings = BTreeMap::new();
        for (i, id) in types.iter().enumerate().skip(1) {
            let CemAstNode::Element { attributes, .. } = c.document().get(*id).unwrap() else {
                unreachable!()
            };
            let field=*attributes.iter().find(|id|matches!(c.document().get(**id),Some(CemAstNode::Attribute{expanded_name,..}) if expanded_name.local_name=="base")).unwrap();
            bindings.insert(field, DependencyTarget::LocalDatatype(types[i - 1]));
        }
        assert_eq!(
            graph::inspect(
                &c,
                &BTreeSet::from([*types.last().unwrap()]),
                &bindings,
                &BTreeMap::new(),
                &Work::new(&OperationControl::default())
            )
            .is_ok(),
            valid
        );
    }
}
