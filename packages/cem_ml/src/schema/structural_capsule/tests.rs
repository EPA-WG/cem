use super::*;
use crate::{
    events::cem::CemEventNormalizer,
    schema::{machine::CemSchemaMachine, vocab::CompiledSchema},
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
const URI: &str = "https://app.test/schema/cart.cem";
const SCHEMA: &str = "https://app.test/schema/cart/1";
const SOURCE: &str = include_str!("../../../fixtures/structural-cart.cem");
fn capture(source: &str) -> LexicallyScopedDocument {
    CemSchemaMachine::new(
        CompiledSchema::cem_core(),
        CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
            SourceId(1),
            source.as_bytes().to_vec(),
        ))),
    )
    .build_with_lexical_scopes()
}
fn built(source: &str) -> Result<BuiltStructuralCapsule, String> {
    export(
        &capture(source),
        SourceIdentity::new(URI, SourceId(1), source.as_bytes()),
        SCHEMA,
        &hash(b"reviewed structural registry fixture"),
        &OperationControl::default(),
    )
}
#[test]
fn original_source_shape_and_passive_namespace_capture_survive_reload() {
    let original = capture(SOURCE);
    let flat = nodes(original.document()).unwrap();
    let identity = SourceIdentity::new(URI, SourceId(1), SOURCE.as_bytes());
    for (i, node) in flat.iter().enumerate() {
        assert!(
            i == 0 || !node.source.frames.is_empty(),
            "empty source map at {i}"
        );
        for frame in &node.source.frames {
            if frame.source_id != SourceId(1) {
                assert_eq!(frame.source_id, SourceId(0));
                assert!(
                    flat[0].children.contains(&(i as u32))
                        && matches!(node.kind.as_str(), "whitespace" | "text" | "comment")
                );
            }
            let ranges = match &frame.span {
                crate::source_map::FrameSpan::Single(r) => std::slice::from_ref(r),
                crate::source_map::FrameSpan::Multi(r) => r.as_slice(),
            };
            for r in ranges {
                assert!(
                    r.start + r.len as u64 <= SOURCE.len() as u64,
                    "range at {i}: {r:?}, source bytes {}",
                    SOURCE.len()
                );
            }
        }
    }
    let document = restore(&flat, &identity, &Work::new(&OperationControl::default())).unwrap();
    let lexical = LexicalReloadMetadata::export(
        &original,
        *blake3::hash(&encoded(&flat).unwrap()).as_bytes(),
    );
    lexical
        .validate(
            &document,
            *blake3::hash(&encoded(&flat).unwrap()).as_bytes(),
        )
        .unwrap();
    let owner = Arc::new(document);
    let reloaded = lexical.restore(owner.clone());
    for (id, node) in original.document().nodes.iter().enumerate() {
        if matches!(node, CemAstNode::Attribute { .. }) {
            if let Some(before) = original.attribute_namespaces(original.document(), id as u32) {
                assert_eq!(
                    encoded(before).unwrap(),
                    encoded(reloaded.attribute_namespaces(&owner, id as u32).unwrap()).unwrap()
                );
                assert!(reloaded
                    .attribute_namespaces(original.document(), id as u32)
                    .is_none());
            }
        }
    }
}
#[test]
fn structural_model_and_original_occurrences_reload_without_recompilation() {
    let original = capture(SOURCE);
    let build = built(SOURCE).unwrap();
    let loaded = verify(&build.bytes, &build.pin, &OperationControl::default()).unwrap();
    assert!(!Arc::ptr_eq(
        original.document(),
        loaded.source().ast_owner()
    ));
    assert_eq!(loaded.source().source_uri(), URI);
    let native = super::super::document_model::compile_schema_document_model(SCHEMA, SOURCE);
    assert!(native.is_ready_for_structural_validation());
    assert_eq!(loaded.model().attributes, native.attributes);
    assert_eq!(loaded.model().elements, native.elements);
    for text in [
        "{cart @title=Tea | {item @quantity=3}}",
        "{cart @title=T | {item @quantity=0}}",
        "{cart @title=Tea}",
        "{cart @title=Tea | {other}}",
        "{cart @title=Tea | {item @quantity=1}{item @quantity=2}{item @quantity=3}{item @quantity=4}}",
    ] {
        let input = capture(text);
        let expected = super::super::document_model::validate_document_model(input.document(), &native);
        let actual = super::super::document_model::validate_document_model(input.document(), loaded.model());
        assert_eq!(format!("{actual:?}"), format!("{expected:?}"), "{text}");
    }
    assert!(loaded.model().attribute_datatypes.is_empty());
    assert!(loaded.model().datatype_compilation.is_none());
    for (index, node) in original.document().nodes.iter().enumerate() {
        assert_eq!(
            format!("{node:?}"),
            format!("{:?}", loaded.source().ast().nodes[index])
        );
        assert_eq!(
            loaded.source().source_node_map(index as u32),
            Some(
                super::super::document_model::source_stack_for_node(node)
                    .clone()
                    .with_source_uri(URI)
            )
        );
    }
    assert_eq!(build.bytes, built(SOURCE).unwrap().bytes);
}

fn packet(capsule: &Capsule, original: &CapsulePin) -> (Vec<u8>, CapsulePin) {
    let bytes = [MAGIC, &encoded(capsule).unwrap()].concat();
    let mut pin = original.clone();
    pin.hash = hash(&bytes);
    (bytes, pin)
}
fn decode(build: &BuiltStructuralCapsule) -> Capsule {
    rmp_serde::from_slice(&build.bytes[MAGIC.len()..]).unwrap()
}
#[test]
fn export_refuses_unsupported_source_policies_without_dropping_them() {
    for source in [
        SOURCE.replace("@type=integer", "@type=custom"),
        SOURCE.replace("@type=integer", "@type=node"),
        SOURCE.replace("@type=integer", "@type=list"),
        SOURCE.replace("@type=integer", "@type=integer @default=3"),
        SOURCE.replace("@type=integer", "@type=integer @vendor-policy=ignored"),
        SOURCE.replace("@name=cart @required", "@name=cart @base=other @required"),
        SOURCE.replace("@children=item", "@children=cart"),
        SOURCE.replace("@children=item", "@children=missing"),
        SOURCE.replace("@children=item", "@children=*"),
        SOURCE.replace("@name=size", "@name=size @behavior=custom"),
        SOURCE.replace("@name=size", "@name=size @when-attribute=title"),
        SOURCE.replace("@min-total-children=1", "@min-total-children=5"),
        SOURCE.replace("{elements |", "{elements | {#dynamic}"),
        SOURCE.replace(
            "{elements |",
            "{uses | {use @name=x @src=https://foreign.test/schema}}{elements |",
        ),
        SOURCE.replace(
            "{elements |",
            "{types | {type @name=custom @kind=node}}{elements |",
        ),
        SOURCE.replace("{elements |", "{elements | {element @name=item}"),
        SOURCE.replace(
            "{attributes |",
            "{attributes | {attribute @name=unused @type=string}",
        ),
        SOURCE.replace("@type=string", "@type=string @type=integer"),
    ] {
        assert!(
            built(&source).is_err(),
            "accepted unsupported source: {source}"
        );
    }
}
#[test]
fn verifier_refuses_rehashed_unsupported_models_and_wrong_occurrences() {
    let build = built(SOURCE).unwrap();
    for mutation in 0..14 {
        let mut capsule = decode(&build);
        match mutation {
            0 => capsule.attributes[0].model.native_type_pending = true,
            1 => capsule.attributes[0].model.value_type = Some("node".into()),
            2 => capsule.attributes[0].model.default_value = Some("Tea".into()),
            3 => capsule.attributes[0]
                .model
                .string_prefixes
                .insert("T".into())
                .then_some(())
                .unwrap(),
            4 => capsule.attributes[0].occurrence = capsule.elements[0].occurrence,
            5 => capsule.elements[0].counts[0].occurrence = capsule.elements[1].occurrence,
            6 => {
                capsule.nodes[capsule.attributes[0].occurrence as usize]
                    .source
                    .frames[0]
                    .source_id = SourceId(9)
            }
            7 => capsule.attributes[0].model.source_map.frames.clear(),
            8 => capsule.abi = "cem.native-structural/2".into(),
            9 => capsule.engine_version = "other".into(),
            10 => capsule.registry_hash = hash(b"foreign registry"),
            11 => capsule.source.hash = hash(b"different original source"),
            12 => {
                capsule.nodes[capsule.attributes[0].occurrence as usize]
                    .source
                    .frames[0]
                    .source_id = SourceId(0)
            }
            _ => {
                capsule.nodes[capsule.attributes[0].occurrence as usize]
                    .source
                    .frames[0]
                    .transform = crate::source_map::TransformKind::SourceBoundary {
                    uri: "https://foreign.test/schema.cem".into(),
                }
            }
        }
        // Refresh dependency hashes as well: integrity alone grants no policy.
        if mutation < 4 {
            capsule.dependencies =
                dependencies(&capsule, &Work::new(&OperationControl::default())).unwrap();
        }
        let (bytes, pin) = packet(&capsule, &build.pin);
        assert!(
            verify(&bytes, &pin, &OperationControl::default()).is_err(),
            "mutation {mutation}"
        );
    }
    assert!(verify(&build.bytes, &build.pin, &OperationControl::default()).is_ok());
}
#[test]
fn exact_dependency_closure_refuses_missing_extra_duplicate_or_changed_edges() {
    let build = built(SOURCE).unwrap();
    for mutation in 0..4 {
        let mut capsule = decode(&build);
        match mutation {
            0 => {
                capsule.dependencies.pop();
            }
            1 => capsule.dependencies.push(capsule.dependencies[0].clone()),
            2 => capsule.dependencies[0].hash = hash(b"wrong contract"),
            _ => capsule.dependencies[0].to = u32::MAX,
        }
        let (bytes, pin) = packet(&capsule, &build.pin);
        assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
    }
}

#[test]
fn owning_source_edges_and_ranges_are_verified_even_with_new_integrity_pins() {
    let build = built(SOURCE).unwrap();
    for mutation in 0..7 {
        let mut capsule = decode(&build);
        match mutation {
            0 => {
                let child = capsule.nodes[0].children[0];
                capsule.nodes[0].children.push(child);
            }
            1 => capsule.nodes[0].children.push(u32::MAX),
            2 => {
                let orphan = capsule.nodes[capsule.attributes[0].occurrence as usize].clone();
                capsule.nodes.push(orphan);
            }
            3 => capsule.nodes[0].children.push(0),
            4 => capsule.nodes[0].kind = "reference".into(),
            5 => {
                capsule.nodes[capsule.attributes[0].occurrence as usize]
                    .source
                    .frames[0]
                    .span =
                    crate::source_map::FrameSpan::Single(crate::source::ByteRange::new(u64::MAX, 1))
            }
            _ => capsule.nodes[capsule.elements[0].occurrence as usize]
                .children
                .push(capsule.elements[0].occurrence),
        }
        capsule.lexical.payload_fingerprint =
            *blake3::hash(&encoded(&capsule.nodes).unwrap()).as_bytes();
        let (bytes, pin) = packet(&capsule, &build.pin);
        assert!(
            verify(&bytes, &pin, &OperationControl::default()).is_err(),
            "source mutation {mutation}"
        );
    }
}
#[test]
fn byte_collection_depth_integrity_and_cancellation_guards_fail_before_publication() {
    let build = built(SOURCE).unwrap();
    let bodies = [
        vec![0xdd, 0xff, 0xff, 0xff, 0xff],
        vec![0xdb, 0xff, 0xff, 0xff, 0xff],
        [vec![0x91; 65], vec![0xc0]].concat(),
        [vec![0xdc, 0x80, 0x01], vec![0xc0; 32769]].concat(),
        vec![0xc7, 0, 0],
    ];
    for body in bodies {
        let bytes = [MAGIC, &body].concat();
        let mut pin = build.pin.clone();
        pin.hash = hash(&bytes);
        assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
    }
    let mut bytes = build.bytes.clone();
    bytes.push(0);
    let mut pin = build.pin.clone();
    pin.hash = hash(&bytes);
    assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
    let bytes = vec![0; MAX_BYTES + 1];
    pin.hash = hash(&bytes);
    assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
    pin = build.pin.clone();
    pin.hash = hash(b"stale");
    assert!(verify(&build.bytes, &pin, &OperationControl::default()).is_err());
    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    assert!(verify(&build.bytes, &build.pin, &control).is_err());
    assert!(export(
        &capture(SOURCE),
        SourceIdentity::new(URI, SourceId(1), SOURCE.as_bytes()),
        SCHEMA,
        &build.pin.registry_hash,
        &control
    )
    .is_err());
}
#[test]
fn unknown_native_records_cannot_install_executables() {
    #[derive(Serialize)]
    struct Extension<'a> {
        #[serde(flatten)]
        capsule: &'a Capsule,
        executable: &'a str,
    }
    let build = built(SOURCE).unwrap();
    let capsule = decode(&build);
    let bytes = [
        MAGIC,
        &encoded(&Extension {
            capsule: &capsule,
            executable: "cem:validator/1",
        })
        .unwrap(),
    ]
    .concat();
    let mut pin = build.pin.clone();
    pin.hash = hash(&bytes);
    assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
}

#[test]
fn retained_budget_charges_shared_source_and_each_reachable_contract() {
    let build = built(SOURCE).unwrap();
    let mut capsule = decode(&build);
    capsule.attributes[0].model.allowed_values =
        (0..2500).map(|i| format!("choice-{i:04}")).collect();
    capsule.dependencies =
        dependencies(&capsule, &Work::new(&OperationControl::default())).unwrap();
    let (bytes, pin) = packet(&capsule, &build.pin);
    assert!(
        bytes.len() < MAX_BYTES,
        "fixture must test retained accounting, not the wire ceiling"
    );
    assert!(account(&capsule, &Work::new(&OperationControl::default())).is_err());
    assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
}
#[test]
fn acyclic_dependencies_still_have_one_shared_expanded_work_budget() {
    let mut declarations = String::new();
    for index in 0..30 {
        let children = if index < 28 {
            format!("@children=\"e{} e{}\"", index + 1, index + 2)
        } else {
            String::new()
        };
        declarations.push_str(&format!("{{element @name=e{index} {children}}}"));
    }
    let source =
        format!("{{schema @name=cart @namespace={SCHEMA} | {{elements | {declarations}}}}}");
    assert!(built(&source).is_err());
}
