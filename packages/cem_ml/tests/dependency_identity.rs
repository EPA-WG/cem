use cem_ml::content_cache::ContentHash;
use cem_ml::dependency_identity::*;
use std::collections::BTreeMap;

fn content(value: &str) -> DependencyVersion {
    DependencyVersion::Content(ContentHash::from_blake3(value.as_bytes()))
}
fn snapshot() -> DependencySnapshot {
    DependencySnapshot {
        layer: DependencyLayer::TemplateCompilation,
        engine_version: "cem-ml/1;cem-ql/1".into(),
        format_version: "template-ir/1".into(),
        closure_state: DependencyClosureState::Ready,
        root_uri: "studio://project/root.cemt".into(),
        sources: BTreeMap::from([
            (
                "studio://project/root.cemt".into(),
                SourceDependency {
                    content_type: "application/cem".into(),
                    schema: Some("urn:cemt".into()),
                    version: content("root"),
                },
            ),
            (
                "studio://project/partial.cemt".into(),
                SourceDependency {
                    content_type: "application/cem".into(),
                    schema: Some("urn:cemt".into()),
                    version: content("partial"),
                },
            ),
        ]),
        imports: vec![ImportDependency {
            parent_uri: "studio://project/root.cemt".into(),
            specifier: "partial".into(),
            resolved_uri: "studio://project/partial.cemt".into(),
        }],
        options: BTreeMap::from([("entrypoint".into(), "body".into())]),
        policy: content("policy"),
        resolution: content("module-map"),
        scope: content("private scope"),
        inputs: BTreeMap::new(),
    }
}
#[test]
fn deterministic_complete_identity_and_each_native_axis_invalidates() {
    let before = snapshot();
    let key = before.identity().unwrap();
    assert_eq!(key, snapshot().identity().unwrap());
    let mut variants = Vec::new();
    macro_rules! change {
        ($field:ident, $value:expr) => {{
            let mut v = before.clone();
            v.$field = $value;
            variants.push(v);
        }};
    }
    change!(engine_version, "engine/2".into());
    change!(format_version, "ir/2".into());
    change!(layer, DependencyLayer::Stylesheet);
    change!(policy, content("new policy"));
    change!(resolution, content("new module map"));
    change!(scope, content("new scope"));
    for uri in before.sources.keys() {
        let mut v = before.clone();
        v.sources.get_mut(uri).unwrap().version = content("edited");
        variants.push(v);
        let mut v = before.clone();
        v.sources.get_mut(uri).unwrap().content_type = "text/cem-ml".into();
        variants.push(v);
        let mut v = before.clone();
        v.sources.get_mut(uri).unwrap().schema = Some("urn:cemt:2".into());
        variants.push(v);
    }
    let mut v = before.clone();
    v.options.insert("entrypoint".into(), "other".into());
    variants.push(v);
    let mut v = before.clone();
    v.imports[0].specifier = "another-alias".into();
    variants.push(v);
    let mut v = before.clone();
    let source = v.sources.remove("studio://project/partial.cemt").unwrap();
    v.sources
        .insert("studio://project/moved.cemt".into(), source);
    v.imports[0].resolved_uri = "studio://project/moved.cemt".into();
    variants.push(v);
    let mut v = before.clone();
    v.root_uri = "studio://project/partial.cemt".into();
    variants.push(v);
    for changed in variants {
        assert_ne!(key, changed.identity().unwrap());
    }
}
#[test]
fn render_requires_all_effective_inputs_and_each_revision_invalidates() {
    let mut value = snapshot();
    value.layer = DependencyLayer::RenderProjection;
    assert_eq!(value.identity(), Err(DependencyIdentityError::MissingInput));
    for kind in RENDER_DEPENDENCY_INPUTS {
        value.inputs.insert(
            *kind,
            DependencyVersion::Revision {
                owner: "instance-A".into(),
                generation: 1,
                revision: 1,
            },
        );
    }
    let original = value.identity().unwrap();
    for kind in RENDER_DEPENDENCY_INPUTS {
        let mut changed = value.clone();
        changed.inputs.insert(
            *kind,
            DependencyVersion::Revision {
                owner: "instance-A".into(),
                generation: 1,
                revision: 2,
            },
        );
        assert_ne!(original, changed.identity().unwrap());
        changed.inputs.insert(*kind, DependencyVersion::Opaque);
        assert_eq!(
            changed.identity(),
            Err(DependencyIdentityError::OpaqueDependency)
        );
    }
    let mut changed = value.clone();
    changed.inputs.insert(
        DependencyInput::Data,
        DependencyVersion::Revision {
            owner: "instance-B".into(),
            generation: 1,
            revision: 1,
        },
    );
    assert_ne!(original, changed.identity().unwrap());
    changed.inputs.insert(
        DependencyInput::Data,
        DependencyVersion::Revision {
            owner: "instance-A".into(),
            generation: 2,
            revision: 1,
        },
    );
    assert_ne!(original, changed.identity().unwrap());
}
#[test]
fn incomplete_opaque_ambiguous_and_oversized_snapshots_fail_closed() {
    let before = snapshot();
    for state in [
        DependencyClosureState::Pending,
        DependencyClosureState::Failed,
        DependencyClosureState::Released,
    ] {
        let mut v = before.clone();
        v.closure_state = state;
        assert_eq!(
            v.identity(),
            Err(DependencyIdentityError::IncompleteClosure)
        );
    }
    let mut v = before.clone();
    v.policy = DependencyVersion::Opaque;
    assert_eq!(v.identity(), Err(DependencyIdentityError::OpaqueDependency));
    let mut v = before.clone();
    v.sources.get_mut(&v.root_uri).unwrap().version = DependencyVersion::Opaque;
    assert_eq!(v.identity(), Err(DependencyIdentityError::OpaqueDependency));
    let mut v = before.clone();
    v.resolution = DependencyVersion::Revision {
        owner: String::new(),
        generation: 1,
        revision: 1,
    };
    assert_eq!(v.identity(), Err(DependencyIdentityError::InvalidContext));
    let mut v = before.clone();
    v.sources.remove("studio://project/partial.cemt");
    assert_eq!(v.identity(), Err(DependencyIdentityError::InvalidClosure));
    let mut v = before.clone();
    v.imports.push(v.imports[0].clone());
    assert_eq!(v.identity(), Err(DependencyIdentityError::InvalidClosure));
    let mut v = before.clone();
    v.options.insert(
        "oversized".into(),
        "x".repeat(MAX_DEPENDENCY_IDENTITY_BYTES + 1),
    );
    assert_eq!(v.identity(), Err(DependencyIdentityError::Bounds));
    let mut v = before.clone();
    v.policy = DependencyVersion::Content(ContentHash {
        scheme: "unknown".into(),
        hex: "a".repeat(64),
    });
    assert_eq!(v.identity(), Err(DependencyIdentityError::InvalidHash));
}
#[test]
fn dependency_order_is_canonical_and_late_runtime_or_owner_admission_is_rejected() {
    let mut before = snapshot();
    before.imports.push(ImportDependency {
        parent_uri: before.root_uri.clone(),
        specifier: "second".into(),
        resolved_uri: "studio://project/partial.cemt".into(),
    });
    let key = before.identity().unwrap();
    before.imports.reverse();
    assert_eq!(key, before.identity().unwrap());
    let captured = DependencyAdmission {
        owner: "owner-A".into(),
        runtime_generation: 1,
        revision: 3,
    };
    assert!(captured.is_current(&captured));
    assert!(!captured.is_current(&DependencyAdmission {
        runtime_generation: 2,
        ..captured.clone()
    }));
    assert!(!captured.is_current(&DependencyAdmission {
        revision: 4,
        ..captured.clone()
    }));
    assert!(!captured.is_current(&DependencyAdmission {
        owner: "owner-B".into(),
        ..captured.clone()
    }));
    assert!(!DependencyAdmission {
        owner: String::new(),
        ..captured.clone()
    }
    .is_current(&DependencyAdmission {
        owner: String::new(),
        ..captured
    }));
}

#[test]
fn limits_reject_source_import_counts_and_escaped_metadata_without_retaining_payloads() {
    let mut v = snapshot();
    for i in 0..MAX_DEPENDENCY_SOURCES {
        v.sources
            .insert(format!("source:{i}"), v.sources[&v.root_uri].clone());
    }
    assert_eq!(v.identity(), Err(DependencyIdentityError::Bounds));
    let mut v = snapshot();
    v.imports = vec![v.imports[0].clone(); MAX_DEPENDENCY_IMPORTS + 1];
    assert_eq!(v.identity(), Err(DependencyIdentityError::Bounds));
    let mut v = snapshot();
    v.options.insert("escaped".into(), "\0".repeat(200_000));
    assert_eq!(v.identity(), Err(DependencyIdentityError::Bounds));
    let mut v = snapshot();
    v.policy = DependencyVersion::Content(ContentHash {
        scheme: "cem-bin/1+blake3".into(),
        hex: "z".repeat(64),
    });
    assert_eq!(v.identity(), Err(DependencyIdentityError::InvalidHash));
    let mut v = snapshot();
    v.root_uri = "missing:root".into();
    assert_eq!(v.identity(), Err(DependencyIdentityError::InvalidClosure));
}
