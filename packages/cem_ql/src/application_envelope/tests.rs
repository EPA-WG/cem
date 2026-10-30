use super::*;
fn built() -> BuiltApplicationEnvelope {
    let source = r#"<cem-element tag="cem-app"><template id="cem-app" type="text/cem-ml">{p | Hello}</template></cem-element>"#;
    let declaration = crate::declaration_artifact::build_declaration_artifact(
        source,
        "file:///apps/example/app.xhtml",
        crate::template_artifact::TemplateArtifactSourceMapMode::Dev,
        None,
    )
    .unwrap();
    let mut builder = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    builder.add_component(declaration, BTreeMap::new()).unwrap();
    builder
        .build(
            BTreeSet::from(["cem-app".into()]),
            BTreeSet::new(),
            &OperationControl::default(),
        )
        .unwrap()
}
fn pack(
    envelope: &Envelope,
    pin: &ApplicationEnvelopePin,
    manifest: bool,
) -> (Vec<u8>, ApplicationEnvelopePin) {
    let bytes = [MAGIC, &encode(envelope).unwrap()].concat();
    let pin = ApplicationEnvelopePin {
        hash: hash(&bytes),
        manifest_hash: if manifest {
            hash(&encode(&envelope.manifest).unwrap())
        } else {
            pin.manifest_hash.clone()
        },
        registry_hash: pin.registry_hash.clone(),
    };
    (bytes, pin)
}
#[test]
fn independently_pinned_manifest_cannot_change_when_only_package_bytes_are_repinned() {
    let built = built();
    let mut e: Envelope = rmp_serde::from_slice(&built.bytes[MAGIC.len()..]).unwrap();
    e.manifest.roots.clear();
    let (bytes, pin) = pack(&e, &built.pin, false);
    assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
}
#[test]
fn correctly_repinned_incompatible_manifest_graph_and_blob_records_still_refuse() {
    let built = built();
    for case in 0..12 {
        let mut e: Envelope = rmp_serde::from_slice(&built.bytes[MAGIC.len()..]).unwrap();
        match case {
            0 => e.manifest.abi.push_str("future"),
            1 => e.manifest.ml_version.push_str("future"),
            2 => e.manifest.registry_hash = hash(b"other"),
            3 => e.manifest.roots.clear(),
            4 => {
                e.manifest
                    .components
                    .get_mut("cem-app")
                    .unwrap()
                    .dependencies
                    .insert("cem-missing".into());
            }
            5 => e.manifest.requirements_hash = hash(b"other requirements"),
            6 => e.manifest.registry_plan_hash = hash(b"other recipe"),
            7 => {
                e.blobs.insert(hash(b"orphan"), Blob(b"orphan".to_vec()));
            }
            8 => {
                e.manifest
                    .components
                    .get_mut("cem-app")
                    .unwrap()
                    .receivers
                    .insert(
                        "unused".into(),
                        NodeReceiverRequirement {
                            contract: hash(b"fake"),
                            schema_uri: "urn:fake".into(),
                        },
                    );
            }
            9 => {
                let c = e.manifest.components.get_mut("cem-app").unwrap();
                c.template = c.metadata.clone();
            }
            10 => {
                let bytes = b"CEMNR1\ninvalid".to_vec();
                let id = hash(&bytes);
                e.manifest.contracts.insert(
                    id.clone(),
                    ContractPin::Native(NativeRequirementPin {
                        hash: id.clone(),
                        source: cem_ml::schema::structural_capsule::SourceIdentity::new(
                            "file:///app/type.cem",
                            cem_ml::source::SourceId(1),
                            b"source",
                        ),
                        registry_hash: e.manifest.registry_hash.clone(),
                    }),
                );
                e.manifest.dependencies.insert(id.clone(), BTreeSet::new());
                e.manifest.contract_roots.insert(id.clone());
                e.blobs.insert(id, Blob(bytes));
            }
            _ => {
                let id = e.manifest.components["cem-app"].metadata.clone();
                e.blobs.get_mut(&id).unwrap().0.push(b' ');
            }
        }
        let (bytes, pin) = pack(&e, &built.pin, true);
        assert!(
            verify(&bytes, &pin, &OperationControl::default()).is_err(),
            "case {case}"
        );
    }
}
#[test]
fn noncanonical_unknown_trailing_deep_and_oversized_messages_are_bounded() {
    let built = built();
    let mut bytes = built.bytes.clone();
    bytes.push(0);
    let mut pin = built.pin.clone();
    pin.hash = hash(&bytes);
    assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
    let mut bytes = MAGIC.to_vec();
    bytes.extend(std::iter::repeat_n(0x91, 65));
    bytes.push(0xc0);
    pin.hash = hash(&bytes);
    assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
    let bytes = vec![0; MAX_BYTES + 1];
    pin.hash = hash(&bytes);
    assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
    let mut bytes = MAGIC.to_vec();
    bytes.extend([0xdf, 0xff, 0xff, 0xff, 0xff]);
    pin.hash = hash(&bytes);
    assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
    #[derive(Serialize)]
    struct Unknown<'a> {
        manifest: &'a Manifest,
        blobs: &'a BTreeMap<String, Blob>,
        unknown: bool,
    }
    let e: Envelope = rmp_serde::from_slice(&built.bytes[MAGIC.len()..]).unwrap();
    let bytes = [
        MAGIC,
        &encode(&Unknown {
            manifest: &e.manifest,
            blobs: &e.blobs,
            unknown: true,
        })
        .unwrap(),
    ]
    .concat();
    pin.hash = hash(&bytes);
    assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
}
#[test]
fn per_byte_array_encoding_cannot_replace_opaque_native_binary_blobs() {
    #[derive(Serialize)]
    struct ArrayEnvelope<'a> {
        manifest: &'a Manifest,
        blobs: BTreeMap<String, Vec<u8>>,
    }
    let built = built();
    let envelope: Envelope = rmp_serde::from_slice(&built.bytes[MAGIC.len()..]).unwrap();
    let arrays = ArrayEnvelope {
        manifest: &envelope.manifest,
        blobs: envelope
            .blobs
            .iter()
            .map(|(id, blob)| (id.clone(), blob.0.clone()))
            .collect(),
    };
    let bytes = [MAGIC, &encode(&arrays).unwrap()].concat();
    let pin = ApplicationEnvelopePin {
        hash: hash(&bytes),
        ..built.pin
    };
    assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
}
#[test]
fn component_limit_and_failed_blob_staging_leave_existing_builder_usable() {
    let source = |tag: &str| {
        crate::declaration_artifact::build_declaration_artifact(&format!("<cem-element tag=\"{tag}\"><template id=\"{tag}\" type=\"text/cem-ml\">{{p | Hello}}</template></cem-element>"),"file:///app/test.xhtml",crate::template_artifact::TemplateArtifactSourceMapMode::Dev,None).unwrap()
    };
    let mut b = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    let roots = (0..32)
        .map(|i| format!("cem-c{i}"))
        .collect::<BTreeSet<_>>();
    for tag in &roots {
        b.add_component(source(tag), BTreeMap::new()).unwrap();
    }
    assert!(b
        .add_component(source("cem-overflow"), BTreeMap::new())
        .is_err());
    assert!(b
        .build(roots, BTreeSet::new(), &OperationControl::default())
        .is_ok());
    let mut b = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    b.add_component(source("cem-live"), BTreeMap::new())
        .unwrap();
    // Almost-full staging simulates opaque contract content; do not allocate an
    // unbounded count or poison a real published runtime to exercise refusal.
    let filler = vec![0; MAX_BYTES - 4096];
    let filler_id = b.blob(filler).unwrap();
    let before = b.blobs.len();
    let large_source = source("cem-fail");
    let mut large = large_source;
    large.metadata_json.push_str(&" ".repeat(5000));
    large.metadata_hash = hash(large.metadata_json.as_bytes());
    assert!(b.add_component(large, BTreeMap::new()).is_err());
    assert_eq!(b.blobs.len(), before);
    b.blobs.remove(&filler_id);
    assert!(b
        .build(
            BTreeSet::from(["cem-live".into()]),
            BTreeSet::new(),
            &OperationControl::default()
        )
        .is_ok());
}
