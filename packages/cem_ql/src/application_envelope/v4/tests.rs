use super::*;
use cem_ml::{
    parser::CemAstNode,
    schema::{structural_capsule::SourceIdentity, vocab::CompiledSchema},
    source::SourceId,
};
fn built() -> BuiltApplicationEnvelope {
    let control = OperationControl::default();
    let mut base = super::super::EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    let declaration = |tag: &str, body: &str| {
        crate::declaration_artifact::build_declaration_artifact(&format!("<cem-element tag=\"{tag}\"><template id=\"{tag}\" type=\"text/cem-ml\">{body}</template></cem-element>"),&format!("file:///app/{tag}.xhtml"),crate::template_artifact::TemplateArtifactSourceMapMode::Dev,None).unwrap()
    };
    base.add_component(declaration("cem-app", "{p | Hello}"), BTreeMap::new())
        .unwrap();
    let base = base
        .build(
            BTreeSet::from(["cem-app".into()]),
            BTreeSet::new(),
            &control,
        )
        .unwrap();
    let text="@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=value @kind=scalar}}}";
    let capture = cem_ml::import::import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "file:///app/types.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
    .captured;
    let id = capture
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
        roots: BTreeSet::from([id]),
        bindings: BTreeMap::new(),
    };
    let graph = crate::datatype_module_capsule::v2::export(
        "app",
        &BTreeMap::from([(
            "app".into(),
            crate::datatype_module_capsule::v2::ModuleSource::new(module),
        )]),
        &[],
        &[],
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    let mut parent = v3::EnvelopeBuilder::new(base).unwrap();
    let graph = parent.add_assembly(graph, vec![], vec![]).unwrap();
    let parent = parent
        .build(
            BTreeMap::from([(
                "value".into(),
                v3::AssemblyRoot {
                    assembly: graph,
                    datatype: id,
                },
            )]),
            &control,
        )
        .unwrap();
    let owner = Arc::new(v3::verify(&parent.bytes, &parent.pin, &control).unwrap());
    let d = mixed::build(
        declaration(
            "cem-value",
            "{attribute @name=value @type=app-value}{cem-app}",
        ),
        owner.clone(),
        BTreeMap::from([(
            "app-value".into(),
            mixed::ReceiverContract::Executable {
                root: "value".into(),
                target: owner.roots()["value"].clone(),
            },
        )]),
        &control,
    )
    .unwrap();
    build(
        parent,
        vec![d],
        BTreeSet::from(["cem-value".into()]),
        &control,
    )
    .unwrap()
}
fn raw(b: &BuiltApplicationEnvelope) -> Envelope {
    rmp_serde::from_slice(&b.bytes[MAGIC.len()..]).unwrap()
}
fn pack(e: &impl Serialize, manifest: &Manifest) -> BuiltApplicationEnvelope {
    let bytes = [MAGIC, &encode(e).unwrap()].concat();
    BuiltApplicationEnvelope {
        pin: ApplicationEnvelopePin {
            hash: hash(&bytes),
            manifest_hash: hash(&encode(manifest).unwrap()),
            registry_hash: manifest.registry_hash.clone(),
        },
        bytes,
    }
}
#[test]
fn publication_repinned_manifest_blobs_and_nested_pins_refuse_incompatible_inputs() {
    let b = built();
    for case in 0..18 {
        let mut e = raw(&b);
        match case {
            0 => e.manifest.abi.push_str("future"),
            1 => e.manifest.ml_version.push_str("future"),
            2 => e.manifest.ql_version.push_str("future"),
            3 => e.manifest.parent.hash = hash(b"missing"),
            4 => e.manifest.parent.manifest_hash = hash(b"wrong manifest"),
            5 => e.manifest.parent.registry_hash = hash(b"wrong registry"),
            6 => {
                e.manifest
                    .mixed
                    .get_mut("cem-value")
                    .unwrap()
                    .application_hash = hash(b"other parent")
            }
            7 => {
                e.manifest.mixed.get_mut("cem-value").unwrap().registry_hash =
                    hash(b"other registry")
            }
            8 => {
                let p = e.manifest.mixed.remove("cem-value").unwrap();
                e.manifest.mixed.insert("cem-other".into(), p);
            }
            9 => {
                e.blobs.remove(&e.manifest.parent.hash);
            }
            10 => {
                e.blobs.insert(hash(b"extra"), Blob(b"extra".to_vec()));
            }
            11 => e.manifest.requirements_hash = hash(b"changed requirements"),
            12 => e.manifest.registry_demand_hash = hash(b"changed demand"),
            13 => {
                e.manifest.roots.insert("cem-missing".into());
            }
            14 => e.manifest.roots.clear(),
            15 => e.manifest.mixed.clear(),
            16 => e.blobs.get_mut(&e.manifest.parent.hash).unwrap().0.push(0),
            _ => {
                let id = e.manifest.mixed["cem-value"].hash.clone();
                e.blobs.get_mut(&id).unwrap().0.push(0);
            }
        }
        let candidate = pack(&e, &e.manifest);
        assert!(
            verify(
                &candidate.bytes,
                &candidate.pin,
                &OperationControl::default()
            )
            .is_err(),
            "case {case}"
        );
    }
}
#[test]
fn publication_rejects_unknown_fields_alternate_blobs_trailing_bytes_and_limits() {
    let b = built();
    let e = raw(&b);
    #[derive(Serialize)]
    struct Future<'a> {
        manifest: &'a Manifest,
        blobs: &'a BTreeMap<String, Blob>,
        future: u8,
    }
    let x = pack(
        &Future {
            manifest: &e.manifest,
            blobs: &e.blobs,
            future: 1,
        },
        &e.manifest,
    );
    assert!(verify(&x.bytes, &x.pin, &OperationControl::default()).is_err());
    #[derive(Serialize)]
    struct Arrays<'a> {
        manifest: &'a Manifest,
        blobs: BTreeMap<String, Vec<u8>>,
    }
    let x = pack(
        &Arrays {
            manifest: &e.manifest,
            blobs: e
                .blobs
                .iter()
                .map(|(k, v)| (k.clone(), v.0.clone()))
                .collect(),
        },
        &e.manifest,
    );
    assert!(verify(&x.bytes, &x.pin, &OperationControl::default()).is_err());
    let mut bytes = b.bytes.clone();
    bytes.push(0);
    let pin = ApplicationEnvelopePin {
        hash: hash(&bytes),
        ..b.pin.clone()
    };
    assert!(verify(&bytes, &pin, &OperationControl::default()).is_err());
    let bytes = vec![0; MAX_BYTES + 1];
    assert!(verify(&bytes, &b.pin, &OperationControl::default()).is_err());
    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    assert!(verify(&b.bytes, &b.pin, &control).is_err());
}
#[test]
fn publication_preserves_deterministic_bytes_and_fresh_original_arenas() {
    let b = built();
    assert_eq!(b.bytes, built().bytes);
    let a = verify(&b.bytes, &b.pin, &OperationControl::default()).unwrap();
    let c = verify(&b.bytes, &b.pin, &OperationControl::default()).unwrap();
    assert!(!Arc::ptr_eq(a.parent(), c.parent()));
    let id = &a.parent().roots()["value"].assembly;
    assert!(!Arc::ptr_eq(
        a.parent().assembly_owner(id).unwrap(),
        c.parent().assembly_owner(id).unwrap()
    ));
    assert!(a.registry_demand().registry().entries().next().is_none());
    assert!(!a.registry_demand().execution_units().is_empty());
}
fn batch(count: usize, fields: usize) -> Result<BuiltApplicationEnvelope, String> {
    let b = built();
    let e = raw(&b);
    let control = OperationControl::default();
    let parent = v3::BuiltApplicationEnvelope {
        bytes: e.blobs[&e.manifest.parent.hash].0.clone(),
        pin: e.manifest.parent,
    };
    let owner = Arc::new(v3::verify(&parent.bytes, &parent.pin, &control).unwrap());
    let declarations=(0..count).map(|n|{
  let tag=format!("cem-batch-{n}");let fields=(0..fields).map(|i|format!("{{attribute @name=value-{i} @type=app-value}}")).collect::<String>();let d=crate::declaration_artifact::build_declaration_artifact(&format!("<cem-element tag=\"{tag}\"><template id=\"{tag}\" type=\"text/cem-ml\">{fields}{{cem-app}}</template></cem-element>"),&format!("file:///app/{tag}.xhtml"),crate::template_artifact::TemplateArtifactSourceMapMode::Dev,None).unwrap();
  mixed::build(d,owner.clone(),BTreeMap::from([("app-value".into(),mixed::ReceiverContract::Executable{root:"value".into(),target:owner.roots()["value"].clone()})]),&control).unwrap()
 }).collect();
    build(
        parent,
        declarations,
        (0..count).map(|n| format!("cem-batch-{n}")).collect(),
        &control,
    )
}
#[test]
fn publication_bounds_composed_declaration_count_without_raising_nested_limits() {
    assert!(batch(32, 1).is_ok());
    assert!(batch(33, 1).is_err());
}
#[test]
fn publication_bounds_aggregate_original_receiver_occurrences() {
    assert!(batch(4, 64).is_ok());
    assert!(batch(5, 64).is_err());
}
