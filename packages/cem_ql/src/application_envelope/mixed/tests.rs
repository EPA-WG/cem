use super::*;
use crate as cem_ql;
use cem_ml::{
    parser::CemAstNode,
    schema::{structural_capsule::SourceIdentity, vocab::CompiledSchema},
    source::SourceId,
};
#[allow(dead_code)]
#[path = "../../../fixtures/component-application/named_receiver_types.rs"]
mod scalar;
fn application() -> Arc<super::super::v3::VerifiedApplicationEnvelope> {
    use crate::datatype_module_capsule::{self as modules, v2 as assembly};
    let control = OperationControl::default();
    let mut b = super::super::v3::EnvelopeBuilder::new(super::super::tests::built()).unwrap();
    let text = "@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=value @kind=scalar}}}";
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
    let module = modules::ModuleSource {
        identity: SourceIdentity::new("file:///app/types.cem", SourceId(1), text.as_bytes()),
        capture,
        roots: BTreeSet::from([root]),
        bindings: BTreeMap::new(),
    };
    let graph = assembly::export(
        "app",
        &BTreeMap::from([("app".into(), assembly::ModuleSource::new(module))]),
        &[],
        &[],
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    let id = b.add_assembly(graph, vec![], vec![]).unwrap();
    let b = b
        .build(
            BTreeMap::from([(
                "root".into(),
                super::super::v3::AssemblyRoot {
                    assembly: id,
                    datatype: root,
                },
            )]),
            &control,
        )
        .unwrap();
    Arc::new(super::super::v3::verify(&b.bytes, &b.pin, &control).unwrap())
}
fn built(app: Arc<super::super::v3::VerifiedApplicationEnvelope>) -> BuiltMixedReceiverDeclaration {
    let d = crate::declaration_artifact::build_declaration_artifact(
        "<cem-element tag=\"cem-mixed\"><template id=\"cem-mixed\" type=\"text/cem-ml\">{attribute @name=count @type=cart-count}{p | Hello}</template></cem-element>",
        "file:///app/mixed.xhtml",crate::template_artifact::TemplateArtifactSourceMapMode::Dev,None,
    ).unwrap();
    build(
        d,
        app,
        BTreeMap::from([(
            "cart-count".into(),
            ReceiverContract::Scalar(scalar::types().remove(0)),
        )]),
        &OperationControl::default(),
    )
    .unwrap()
}
fn raw(b: &BuiltMixedReceiverDeclaration) -> Envelope {
    rmp_serde::from_slice(&b.bytes[MAGIC.len()..]).unwrap()
}
fn pack(
    e: &Envelope,
    original: &MixedReceiverPin,
    manifest: bool,
) -> BuiltMixedReceiverDeclaration {
    let bytes = [MAGIC, &encode(e).unwrap()].concat();
    BuiltMixedReceiverDeclaration {
        pin: MixedReceiverPin {
            hash: hash(&bytes),
            manifest_hash: if manifest {
                hash(&encode(&e.manifest).unwrap())
            } else {
                original.manifest_hash.clone()
            },
            application_hash: original.application_hash.clone(),
            registry_hash: original.registry_hash.clone(),
        },
        bytes,
    }
}
#[test]
fn mixed_repinned_incompatible_manifest_and_contract_records_still_refuse() {
    let app = application();
    let b = built(app.clone());
    for case in 0..20 {
        let mut e = raw(&b);
        match case {
            0 => e.manifest.abi.push_str("future"),
            1 => e.manifest.ml_version.push_str("future"),
            2 => e.manifest.ql_version.push_str("future"),
            3 => e.manifest.application.manifest_hash = hash(b"other manifest"),
            4 => e.manifest.application.registry_hash = hash(b"other registry"),
            5 => e.metadata.0.push(b' '),
            6 => e.template.0.push(0),
            7 => e.manifest.metadata_hash = hash(b"other metadata"),
            8 => e.manifest.template_hash = hash(b"other template"),
            9 => e.manifest.requirements_hash = hash(b"other requirements"),
            10 => e.manifest.receivers.clear(),
            11 => {
                let ReceiverContract::Scalar(v) =
                    e.manifest.receivers.get_mut("cart-count").unwrap()
                else {
                    unreachable!()
                };
                v.value.model.native_type_pending = true;
            }
            12 => {
                let ReceiverContract::Scalar(v) =
                    e.manifest.receivers.get_mut("cart-count").unwrap()
                else {
                    unreachable!()
                };
                v.value.content_type = Some("application/json".into());
            }
            13 => {
                let ReceiverContract::Scalar(v) =
                    e.manifest.receivers.get_mut("cart-count").unwrap()
                else {
                    unreachable!()
                };
                v.value.model.value_type = Some("node".into());
            }
            14 => {
                let ReceiverContract::Scalar(v) =
                    e.manifest.receivers.get_mut("cart-count").unwrap()
                else {
                    unreachable!()
                };
                v.source_hash = "not a pin".into();
            }
            15 => {
                let ReceiverContract::Scalar(v) =
                    e.manifest.receivers.get_mut("cart-count").unwrap()
                else {
                    unreachable!()
                };
                v.name = "other".into();
            }
            16 => {
                let mut v = e.manifest.receivers.remove("cart-count").unwrap();
                if let ReceiverContract::Scalar(s) = &mut v {
                    s.name = "string".into();
                }
                e.manifest.receivers.insert("string".into(), v);
            }
            17 => {
                let mut v = e.manifest.receivers["cart-count"].clone();
                if let ReceiverContract::Scalar(s) = &mut v {
                    s.name = "unused".into();
                }
                e.manifest.receivers.insert("unused".into(), v);
            }
            18 => {
                let ReceiverContract::Scalar(v) =
                    e.manifest.receivers.get_mut("cart-count").unwrap()
                else {
                    unreachable!()
                };
                let mut r = v.value.model.clone();
                r.native_type_pending = true;
                v.value.restrictions.push(r);
            }
            _ => {
                let ReceiverContract::Scalar(v) =
                    e.manifest.receivers.get_mut("cart-count").unwrap()
                else {
                    unreachable!()
                };
                v.source_uri = "file:///app/types.cem".into();
            }
        }
        let changed = pack(&e, &b.pin, true);
        assert!(
            verify(
                &changed.bytes,
                &changed.pin,
                app.clone(),
                &OperationControl::default()
            )
            .is_err(),
            "case {case}"
        );
    }
}
#[test]
fn mixed_manifest_dependency_and_registry_pins_remain_independent() {
    let app = application();
    let b = built(app.clone());
    let mut e = raw(&b);
    let ReceiverContract::Scalar(v) = e.manifest.receivers.get_mut("cart-count").unwrap() else {
        unreachable!()
    };
    v.value.model.max_inclusive = Some("6".into());
    let changed = pack(&e, &b.pin, false);
    assert!(verify(
        &changed.bytes,
        &changed.pin,
        app.clone(),
        &OperationControl::default()
    )
    .is_err());
    let mut pin = b.pin.clone();
    pin.registry_hash = hash(b"foreign");
    assert!(verify(&b.bytes, &pin, app.clone(), &OperationControl::default()).is_err());
    pin = b.pin.clone();
    pin.application_hash = hash(b"foreign parent");
    assert!(verify(&b.bytes, &pin, app, &OperationControl::default()).is_err());
}
#[test]
fn mixed_unknown_trailing_array_truncated_and_deep_records_are_bounded() {
    let app = application();
    let b = built(app.clone());
    let e = raw(&b);
    #[derive(Serialize)]
    struct Unknown<'a> {
        manifest: &'a Manifest,
        metadata: &'a Blob,
        template: &'a Blob,
        unknown: bool,
    }
    #[derive(Serialize)]
    struct Arrays<'a> {
        manifest: &'a Manifest,
        metadata: &'a Vec<u8>,
        template: &'a Vec<u8>,
    }
    let cases = vec![
        [b.bytes.as_slice(), &[0]].concat(),
        [MAGIC, &vec![0x91; 65], &[0xc0]].concat(),
        [MAGIC, &[0xdf, 0xff, 0xff, 0xff, 0xff]].concat(),
        [MAGIC, &[0xc6, 0, 0x10, 0, 0]].concat(),
        [
            MAGIC,
            &encode(&Unknown {
                manifest: &e.manifest,
                metadata: &e.metadata,
                template: &e.template,
                unknown: true,
            })
            .unwrap(),
        ]
        .concat(),
        [
            MAGIC,
            &encode(&Arrays {
                manifest: &e.manifest,
                metadata: &e.metadata.0,
                template: &e.template.0,
            })
            .unwrap(),
        ]
        .concat(),
    ];
    for bytes in cases {
        let pin = MixedReceiverPin {
            hash: hash(&bytes),
            ..b.pin.clone()
        };
        assert!(verify(&bytes, &pin, app.clone(), &OperationControl::default()).is_err());
    }
}
#[test]
fn mixed_scalar_projection_metadata_and_total_byte_limits_refuse_overflow() {
    let app = application();
    let b = built(app.clone());
    let mut e = raw(&b);
    let ReceiverContract::Scalar(v) = e.manifest.receivers.get_mut("cart-count").unwrap() else {
        unreachable!()
    };
    v.value.model.pattern = Some("x".repeat(64 * 1024 + 1));
    let changed = pack(&e, &b.pin, true);
    assert!(verify(
        &changed.bytes,
        &changed.pin,
        app.clone(),
        &OperationControl::default()
    )
    .is_err());
    let mut e = raw(&b);
    e.metadata.0.extend(vec![b' '; 64 * 1024 + 1]);
    e.manifest.metadata_hash = hash(&e.metadata.0);
    let changed = pack(&e, &b.pin, true);
    assert!(verify(
        &changed.bytes,
        &changed.pin,
        app.clone(),
        &OperationControl::default()
    )
    .is_err());
    let bytes = vec![0; MAX_BYTES + 1];
    let pin = MixedReceiverPin {
        hash: hash(&bytes),
        ..b.pin
    };
    assert!(verify(&bytes, &pin, app, &OperationControl::default()).is_err());
}
