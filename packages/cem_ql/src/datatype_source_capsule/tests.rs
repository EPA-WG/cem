use super::*;
use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::CemAstNode,
    schema::{machine::CemSchemaMachine, vocab::CompiledSchema},
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
fn capture(text: &str) -> Arc<LexicallyScopedDocument> {
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
}
fn sample() -> BuiltDatatypeSources {
    let text = r#"@ns schema = "https://cem.dev/ns/schema/1"
@default schema
{schema @name=s @namespace=urn:s | {types | {type @name=sample @kind=scalar}}}"#;
    let c = capture(text);
    let root = ids(&c)[0];
    export(
        &c,
        SourceIdentity::new("file:///app/types.cem", SourceId(1), text.as_bytes()),
        &hash(b"registry"),
        BTreeSet::from([root]),
        &OperationControl::default(),
    )
    .unwrap()
}
fn ids(c: &LexicallyScopedDocument) -> Vec<u32> {
    c.document()
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Element {
                expanded_name,
                node_id,
                ..
            } if expanded_name.local_name == "type" => Some(*node_id),
            _ => None,
        })
        .collect()
}
#[test]
fn correctly_repinned_invalid_graph_versions_and_source_ownership_still_refuse() {
    let b = sample();
    for case in 0..9 {
        let mut c: Capsule = rmp_serde::from_slice(&b.bytes[MAGIC.len()..]).unwrap();
        let id = *c.roots.first().unwrap();
        match case {
            0 => c.abi.push_str("future"),
            1 => c.ql_version.push_str("future"),
            2 => c.registry_hash = hash(b"foreign"),
            3 => c.roots.clear(),
            4 => c.types.get_mut(&id).unwrap().kind = "list".into(),
            5 => c.types.get_mut(&id).unwrap().name = "other".into(),
            6 => c.types.get_mut(&id).unwrap().dependencies.push(Dependency {
                field: id,
                role: "inherited-base".into(),
                target: id,
            }),
            7 => c.identity.uri = "file:///foreign.cem".into(),
            _ => {
                let other = c.types[&id].clone();
                c.types.insert(u32::MAX, other);
            }
        }
        let bytes = [MAGIC, &encode(&c).unwrap()].concat();
        let pin = DatatypeSourcesPin {
            hash: hash(&bytes),
            ..b.pin.clone()
        };
        assert!(
            verify(&bytes, &pin, &OperationControl::default()).is_err(),
            "case {case}"
        );
    }
}
#[test]
fn bounded_binary_preflight_rejects_trailing_unknown_deep_and_oversized_records() {
    let b = sample();
    for bytes in [
        [b.bytes.as_slice(), &[0]].concat(),
        [
            MAGIC,
            std::iter::repeat_n(0x91, 65).collect::<Vec<_>>().as_slice(),
            &[0xc0],
        ]
        .concat(),
        vec![0; MAX_BYTES + 1],
        [MAGIC, &[0xdf, 0xff, 0xff, 0xff, 0xff]].concat(),
    ] {
        let pin = DatatypeSourcesPin {
            hash: hash(&bytes),
            ..b.pin.clone()
        };
        assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
    }
    #[derive(Serialize)]
    struct Extra<'a> {
        #[serde(flatten)]
        capsule: &'a Capsule,
        extra: bool,
    }
    let c: Capsule = rmp_serde::from_slice(&b.bytes[MAGIC.len()..]).unwrap();
    let bytes = [
        MAGIC,
        &encode(&Extra {
            capsule: &c,
            extra: true,
        })
        .unwrap(),
    ]
    .concat();
    let pin = DatatypeSourcesPin {
        hash: hash(&bytes),
        ..b.pin
    };
    assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
}
#[test]
fn literal_graph_limits_accept_boundary_and_reject_type_root_and_depth_overflow() {
    let source = |types: String| {
        capture(&format!("@ns schema = \"https://cem.dev/ns/schema/1\"\n@default schema\n{{schema @name=s @namespace=urn:s | {{types | {types}}}}}"))
    };
    for (count, valid) in [(64, true), (65, false)] {
        let c = source(
            (0..count)
                .map(|i| format!("{{type @name=t{i} @kind=scalar}} "))
                .collect(),
        );
        assert_eq!(
            graph::inspect(
                &c,
                &BTreeSet::from([ids(&c)[0]]),
                &Work::new(&OperationControl::default())
            )
            .is_ok(),
            valid
        );
    }
    let c = source(
        (0..33)
            .map(|i| format!("{{type @name=t{i} @kind=scalar}} "))
            .collect(),
    );
    assert!(graph::inspect(
        &c,
        &ids(&c)[..32].iter().copied().collect(),
        &Work::new(&OperationControl::default())
    )
    .is_ok());
    assert!(graph::inspect(
        &c,
        &ids(&c).into_iter().collect(),
        &Work::new(&OperationControl::default())
    )
    .is_err());
    for (count, valid) in [(33, true), (34, false)] {
        let c = source(
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
        assert_eq!(
            graph::inspect(
                &c,
                &BTreeSet::from([*ids(&c).last().unwrap()]),
                &Work::new(&OperationControl::default())
            )
            .is_ok(),
            valid
        );
    }
}
