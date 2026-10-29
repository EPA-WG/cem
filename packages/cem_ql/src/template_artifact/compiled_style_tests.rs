use super::*;
fn context(compiled: &CompiledTemplateArtifact) -> PinnedTemplateArtifactContext {
    PinnedTemplateArtifactContext {
        content_hash: compiled.content_hash.header_value(),
        source_hash: compiled.identity.source_hash.header_value(),
        cem_ml_version: cem_ml::VERSION.into(),
        cem_ql_version: crate::VERSION.into(),
        host_bindings: vec![],
        source_map_mode: compiled.identity.source_map_mode,
    }
}
#[test]
fn compiled_style_template_round_trips_with_identical_native_tree_and_legacy_v1_support() {
    let source =
        "{style | ``` :scope { background:url('../icon.svg'); color:red; } ```}{p | Hello}";
    for mode in [
        TemplateArtifactSourceMapMode::Dev,
        TemplateArtifactSourceMapMode::Prod,
    ] {
        let v1 = compile_template_artifact(source, &Default::default(), mode);
        let v2 = compile_template_artifact_with_compiled_styles(source, &Default::default(), mode)
            .unwrap();
        assert_eq!(v1.identity.artifact_version, CEM_TEMPLATE_ARTIFACT_VERSION);
        assert_eq!(
            v2.identity.artifact_version,
            CEM_COMPILED_STYLE_ARTIFACT_VERSION
        );
        assert!(v2.bytes.starts_with(COMPILED_ARTIFACT_MAGIC));
        let a = v1.reload_pinned(&context(&v1)).unwrap();
        let b = CompiledTemplateArtifact::from_bytes(v2.bytes.clone())
            .unwrap()
            .reload_pinned(&context(&v2))
            .unwrap();
        assert_eq!(a.stylesheets, b.stylesheets);
        let left = a.stylesheets[0].css_tree();
        let right = b.stylesheets[0].css_tree();
        assert!(!std::sync::Arc::ptr_eq(left, right));
        assert_eq!(left.source_key(0), right.source_key(0));
        for id in 0..left.ast().nodes.len() {
            assert_eq!(
                format!("{:?}", left.node(id as u32)),
                format!("{:?}", right.node(id as u32))
            );
        }
        assert_eq!(
            v2.bytes,
            compile_template_artifact_with_compiled_styles(source, &Default::default(), mode)
                .unwrap()
                .bytes
        );
    }
}
#[test]
fn v2_refuses_missing_corrupt_or_source_only_styles_without_css_fallback() {
    let source = "{style | ``` :scope { color:red; } ```}{p | Hello}";
    let v2 = compile_template_artifact_with_compiled_styles(
        source,
        &Default::default(),
        TemplateArtifactSourceMapMode::Prod,
    )
    .unwrap();
    let (_, body) = deserialize_artifact_bytes(&v2.bytes).unwrap();
    for which in 0..3 {
        let mut payload: CompiledStylePayload = rmp_serde::from_slice(&body).unwrap();
        match which {
            0 => payload.stylesheets[0].bytes[0] ^= 1,
            1 => payload.stylesheets[0].hash = String::new(),
            _ => payload.stylesheets[0].bytes = Vec::new(),
        }
        let bytes =
            serialize_artifact_bytes(&v2.identity, &rmp_serde::to_vec_named(&payload).unwrap());
        let altered = CompiledTemplateArtifact::from_bytes(bytes).unwrap();
        assert!(altered.reload_pinned(&context(&altered)).is_err());
    }
    // A v2 envelope cannot carry the old source-CSS payload, even with its own valid pin.
    let v1 = compile_template_artifact(
        source,
        &Default::default(),
        TemplateArtifactSourceMapMode::Prod,
    );
    let (_, old_body) = deserialize_artifact_bytes(&v1.bytes).unwrap();
    let bytes = serialize_artifact_bytes(&v2.identity, &old_body);
    let altered = CompiledTemplateArtifact::from_bytes(bytes).unwrap();
    assert!(altered.reload_pinned(&context(&altered)).is_err());
    for tail in [0, 0xc1, 0xdc] {
        let mut trailing = body.clone(); trailing.push(tail);
        let altered = CompiledTemplateArtifact::from_bytes(serialize_artifact_bytes(&v2.identity, &trailing)).unwrap();
        assert!(altered.reload_pinned(&context(&altered)).is_err());
    }
    let mut bytes = v2.bytes.clone();
    bytes[..ARTIFACT_MAGIC.len()].copy_from_slice(ARTIFACT_MAGIC);
    assert!(CompiledTemplateArtifact::from_bytes(bytes).is_err());
}
