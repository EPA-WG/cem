use super::*;
use crate::{
    ir::{IrId, IrNode},
    parser::UnaryOp,
    resolve::ModuleUri,
};
use cem_ml::{
    source::{ByteRange, SourceId},
    source_map::{FrameSpan, SourceMapFrame, TransformKind},
};

fn sample() -> Capsule {
    let body = "{accepted: str:length(value) > 0, diagnostics: ()}";
    let map = SourceMapStack {
        frames: vec![SourceMapFrame {
            source_id: SourceId(1),
            span: FrameSpan::Single(ByteRange::new(0, 100)),
            transform: TransformKind::CemAstBuilder,
        }],
    };
    let context = StandaloneExpressionContext::default()
        .with_binding(
            "value",
            StandaloneExpressionBinding::any(Default::default()),
        )
        .with_binding(
            "datatype",
            StandaloneExpressionBinding::any(Default::default()),
        )
        .with_binding(
            "candidate",
            StandaloneExpressionBinding::any(Default::default()),
        );
    let mut query = crate::api::compile_expression(body, &context)
        .unwrap()
        .query;
    for entry in &mut query.tree.source_maps {
        entry.frames.splice(0..0, map.frames.iter().cloned());
    }
    let mut bindings = query.policy_bindings.into_iter().collect::<Vec<_>>();
    bindings.sort_by_key(|p| p.0);
    let mut result = Capsule {
        abi: ABI.into(),
        ml_version: cem_ml::VERSION.into(),
        ql_version: crate::VERSION.into(),
        registry_hash: hash(b"registry"),
        source: SourceIdentity::new("https://example.test/schema.cem", SourceId(1), &[b' '; 100]),
        declaration: DeclarationBinding {
            occurrences: [1, 2, 3, 4],
            header_hash: hash(b"headers"),
            maps: vec![map; 4],
            namespace: cem_ml::schema::registry::CEM_SCHEMA_URI.into(),
            aliases: BTreeMap::new(),
            kind: "scalar".into(),
            value: "scalar:string".into(),
            candidate_required: false,
            result_namespace: cem_ml::schema::registry::CEM_SCHEMA_URI.into(),
            result_name: "datatype-validation-result".into(),
            body_hash: hash(body.as_bytes()),
            body_length: body.len(),
        },
        tree: query.tree,
        bindings,
        functions: vec![],
    };
    result.functions = inspect(&result, &Work::new(&OperationControl::default()))
        .unwrap()
        .functions;
    result
}
fn bytes_pin(value: &Capsule) -> (Vec<u8>, QueryCapsulePin) {
    let bytes = [MAGIC, &encode(value).unwrap()].concat();
    let pin = QueryCapsulePin {
        hash: hash(&bytes),
        source: value.source.clone(),
        registry_hash: value.registry_hash.clone(),
        declaration_hash: hash(&encode(&value.declaration).unwrap()),
    };
    (bytes, pin)
}
fn refuses(value: &Capsule) {
    let (bytes, pin) = bytes_pin(value);
    assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
}
#[test]
fn native_wire_roundtrip_contains_instructions_without_query_source() {
    let (bytes, pin) = bytes_pin(&sample());
    let verified = verify(&bytes, &pin, &OperationControl::default()).unwrap();
    assert!(verified.query.source.is_empty());
    assert_eq!(verified.function_count(), 1);
    assert!(!bytes.windows(9).any(|b| b == b"accepted:"));
}
#[test]
fn correctly_repinned_unsupported_instructions_and_dispatch_remain_refused() {
    let replacements = vec![
        IrNode::StateSlot(crate::resolve::StateSlotId(0)),
        IrNode::LeadingDot,
        IrNode::Parent,
        IrNode::Reference,
        IrNode::Call {
            callee: IrId(0),
            args: vec![],
        },
        IrNode::UnaryOp {
            operand: IrId(0),
            op: UnaryOp::Reference,
        },
        IrNode::StdlibCall {
            module: ModuleUri("cem:stdlib/native".into()),
            name: crate::parser::QName {
                prefix: Some("native".into()),
                local: "call".into(),
                range: ByteRange::new(0, 1),
            },
            args: vec![IrId(0)],
        },
    ];
    for node in replacements {
        let mut value = sample();
        value.tree.nodes[1] = node;
        refuses(&value);
    }
    for mutation in 0..6 {
        let mut value = sample();
        match mutation {
            0 => value.functions.clear(),
            1 => value.functions.push(value.functions[0].clone()),
            2 => value.functions[0].arity = 2,
            3 => value.functions[0].signature = "text->boolean".into(),
            4 => value.functions[0].implementation_abi.push_str("/other"),
            _ => {
                let mut unused = value.functions[0].clone();
                unused.name = "upper".into();
                unused.signature = "text->string".into();
                unused.implementation_abi = "cem.selected-validator/1/StringUpper".into();
                value.functions.push(unused);
            }
        }
        refuses(&value);
    }
}
#[test]
fn qualified_call_spans_cannot_claim_another_source_extent() {
    let mut value = sample();
    let call = value
        .tree
        .nodes
        .iter_mut()
        .find_map(|node| match node {
            IrNode::StdlibCall { name, .. } => Some(name),
            _ => None,
        })
        .unwrap();
    call.range = ByteRange::new(u64::MAX, 2);
    refuses(&value);
}
#[test]
fn graph_bindings_source_maps_and_signatures_are_checked_before_execution() {
    for mutation in 0..10 {
        let mut value = sample();
        match mutation {
            0 => value.tree.root = IrId(u32::MAX),
            1 => value.tree.types.pop().map(|_| ()).unwrap(),
            2 => value.tree.resolutions[0] = None,
            3 => value.bindings[0].1 = "foreign".into(),
            4 => value.bindings[1].0 = value.bindings[0].0,
            5 => value.tree.nodes[1] = IrNode::Sequence(vec![IrId(1)]),
            6 => value.tree.source_maps[0].frames[0].source_id = SourceId(99),
            7 => value.declaration.occurrences[0] = value.declaration.occurrences[1],
            8 => value.declaration.value = "callback".into(),
            _ => {
                value.tree.types[0] = Type::Resource {
                    content_type: crate::types::ContentType("text/plain".into()),
                    schema: None,
                }
            }
        }
        refuses(&value);
    }
}
#[test]
fn envelope_identity_canonical_shape_and_preallocation_bounds_fail_closed() {
    let (bytes, pin) = bytes_pin(&sample());
    for end in [0, 1, MAGIC.len(), bytes.len() / 2, bytes.len() - 1] {
        let mut trusted = pin.clone();
        trusted.hash = hash(&bytes[..end]);
        assert!(verify(&bytes[..end], &trusted, &OperationControl::default()).is_err());
    }
    for payload in [
        vec![0xdd, 0xff, 0xff, 0xff, 0xff],
        vec![0xdf, 0xff, 0xff, 0xff, 0xff],
        vec![0xdb, 0xff, 0xff, 0xff, 0xff],
        vec![0x91; 70],
    ] {
        let malformed = [MAGIC, &payload].concat();
        let mut trusted = pin.clone();
        trusted.hash = hash(&malformed);
        assert!(verify(&malformed, &trusted, &OperationControl::default()).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0xc0);
    let mut trusted = pin.clone();
    trusted.hash = hash(&trailing);
    assert!(verify(&trailing, &trusted, &OperationControl::default()).is_err());
    #[derive(Serialize)]
    struct Extended<'a> {
        #[serde(flatten)]
        capsule: &'a Capsule,
        ignored: u8,
    }
    let extended = [
        MAGIC,
        &encode(&Extended {
            capsule: &sample(),
            ignored: 1,
        })
        .unwrap(),
    ]
    .concat();
    trusted.hash = hash(&extended);
    assert!(verify(&extended, &trusted, &OperationControl::default()).is_err());
    for mutation in 0..4 {
        let mut value = sample();
        match mutation {
            0 => value.abi.push_str("/wrong"),
            1 => value.ml_version.push_str(".wrong"),
            2 => value.ql_version.push_str(".wrong"),
            _ => value.source.byte_length = 1024 * 1024 + 1,
        }
        refuses(&value);
    }
    let mut wrong = pin;
    wrong.registry_hash = hash(b"other");
    assert!(verify(&bytes, &wrong, &OperationControl::default()).is_err());
}
#[test]
fn expanded_graph_work_unreachable_nodes_depth_and_wire_size_are_bounded() {
    let mut value = sample();
    let map = value.tree.source_maps[0].clone();
    for _ in 0..25 {
        let prev = value.tree.root;
        value.tree.root = IrId(value.tree.nodes.len() as u32);
        value.tree.nodes.push(IrNode::Sequence(vec![prev, prev]));
        value.tree.types.push(Type::Any);
        value.tree.resolutions.push(None);
        value.tree.source_maps.push(map.clone());
    }
    refuses(&value);
    let mut value = sample();
    value.tree.root = IrId(0);
    refuses(&value);
    let mut value = sample();
    for _ in 0..65 {
        let prev = value.tree.root;
        value.tree.root = IrId(value.tree.nodes.len() as u32);
        value.tree.nodes.push(IrNode::Sequence(vec![prev]));
        value.tree.types.push(Type::Any);
        value.tree.resolutions.push(None);
        value.tree.source_maps.push(map.clone());
    }
    refuses(&value);
    let huge = vec![0; MAX_BYTES + 1];
    let mut pin = bytes_pin(&sample()).1;
    pin.hash = hash(&huge);
    assert!(verify(&huge, &pin, &OperationControl::default()).is_err());
}
#[test]
fn cancellation_refuses_even_a_previously_verified_program() {
    let (bytes, pin) = bytes_pin(&sample());
    let signal = cem_ml::scheduler::AbortSignal::new();
    let control = OperationControl::new(signal.clone());
    signal.abort();
    assert!(verify(&bytes, &pin, &control).is_err());
}
