use cem_ml::{
    ast::reload::{ReferenceReloadBundle, ReloadDependency, ReloadLimits, ReloadSource},
    import::import_bytes_with_lexical_scopes,
    parser::CemAstNode,
    schema::{machine::SchemaElementForm, vocab::CompiledSchema},
    source::SourceId,
};
use std::sync::Arc;

fn source(text: &str) -> cem_ml::import::ScopedCemImport {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "source.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
}
fn bundle(text: &str, include_bytes: bool) -> ReferenceReloadBundle {
    let source = source(text);
    ReferenceReloadBundle::export(
        &source.captured,
        vec![
            ReloadSource::new(SourceId(0), "source.cem", text.as_bytes(), include_bytes),
            ReloadSource::new(SourceId(1), "source.cem", text.as_bytes(), include_bytes),
        ],
        ReloadLimits::default(),
    )
    .unwrap()
}

#[test]
fn reload_rebinds_passive_snapshots_to_a_fresh_owner() {
    let text = "@ns cem = https://cem.dev/ns/core/1\n@ns p = urn:outer\n{cem:schema @src=first.cem | {#p:item}}\n{cem:schema @src=next.cem}\n{host @xmlns:p={#library} |\n@default p\n{p:item} {pending} {#p:item}}";
    let original = source(text);
    let bundle = ReferenceReloadBundle::export(
        &original.captured,
        vec![
            ReloadSource::new(SourceId(0), "source.cem", text.as_bytes(), true),
            ReloadSource::new(SourceId(1), "source.cem", text.as_bytes(), true),
        ],
        ReloadLimits::default(),
    )
    .unwrap();
    let bytes = bundle.encode(ReloadLimits::default()).unwrap();
    let reload = ReferenceReloadBundle::decode(&bytes, ReloadLimits::default())
        .unwrap()
        .reload(ReloadLimits::default())
        .unwrap();
    assert!(!Arc::ptr_eq(&reload.document, original.captured.document()));
    let capture = reload.require_lexical().unwrap();
    assert!(Arc::ptr_eq(capture.document(), &reload.document));
    assert_eq!(reload.source_text(SourceId(1)).unwrap(), Some(text));
    for id in original.captured.occurrences() {
        assert!(original.captured.snapshot(&reload.document, id).is_none());
        assert!(capture.snapshot(&reload.document, id).is_some());
        assert_eq!(
            capture
                .snapshot(&reload.document, id)
                .unwrap()
                .schema
                .active,
            original
                .captured
                .snapshot(original.captured.document(), id)
                .unwrap()
                .schema
                .active
        );
        assert_eq!(
            capture
                .pending_namespace_bindings(&reload.document, id)
                .map(|p| p.len()),
            original
                .captured
                .pending_namespace_bindings(original.captured.document(), id)
                .map(|p| p.len())
        );
    }
    for node in &reload.document.nodes {
        if let CemAstNode::Element { node_id, .. } | CemAstNode::Attribute { node_id, .. } = node {
            if let Some(expected) = original
                .captured
                .expanded_name(original.captured.document(), *node_id)
            {
                let actual = capture.expanded_name(&reload.document, *node_id).unwrap();
                assert_eq!(actual.namespace_uri, expected.namespace_uri);
                assert_eq!(actual.local_name, expected.local_name);
            }
        }
    }
    let mut forms = vec![];
    for node in &reload.document.nodes {
        if let CemAstNode::Element {
            node_id,
            expanded_name,
            ..
        } = node
        {
            if expanded_name.local_name == "schema" {
                forms.push(
                    capture
                        .schema_element_form(&reload.document, *node_id)
                        .unwrap(),
                );
            }
            if let Some(name) = original
                .captured
                .pending_namespace_name(original.captured.document(), *node_id)
            {
                assert_eq!(
                    capture
                        .pending_namespace_name(&reload.document, *node_id)
                        .unwrap()
                        .declaration,
                    name.declaration
                );
            }
        }
    }
    assert!(forms.contains(&SchemaElementForm::Wrapping));
    assert!(forms.contains(&SchemaElementForm::Following));
}

#[test]
fn ast_only_and_missing_source_bytes_are_explicit_not_invalid() {
    let mut bundle = bundle("{#input}", false);
    let reload = bundle.reload(ReloadLimits::default()).unwrap();
    assert_eq!(reload.source_text(SourceId(1)).unwrap(), None);
    bundle.lexical = None;
    let reload = bundle.reload(ReloadLimits::default()).unwrap();
    assert_eq!(
        reload.require_lexical().unwrap_err(),
        ReloadDependency::MissingLexicalMetadata
    );
    assert!(matches!(
        reload.document.nodes[1],
        CemAstNode::Reference { targets: None, .. }
    ));
}

#[test]
fn invalid_fingerprints_versions_kinds_and_dependencies_reject_the_handoff() {
    let original = bundle("{host @xmlns:p={#library} | {p:item} {#p:item}}", true);
    let mut changed = original.clone();
    changed.payload_fingerprint[0] ^= 1;
    assert!(changed.reload(ReloadLimits::default()).is_err());
    let mut changed = original.clone();
    changed.version += 1;
    assert!(changed.reload(ReloadLimits::default()).is_err());
    let mut changed = original.clone();
    changed.sources[0].bytes.as_mut().unwrap().push(b'!');
    assert!(changed.reload(ReloadLimits::default()).is_err());
    let mut changed = original.clone();
    let sidecar = changed.lexical.as_mut().unwrap();
    let (_, snapshot) = sidecar.occurrences.pop_first().unwrap();
    sidecar.occurrences.insert(0, snapshot);
    assert!(changed.reload(ReloadLimits::default()).is_err());
    let mut changed = original.clone();
    changed
        .lexical
        .as_mut()
        .unwrap()
        .pending_namespace_names
        .values_mut()
        .next()
        .unwrap()
        .declaration = u32::MAX;
    assert!(changed.reload(ReloadLimits::default()).is_err());
    let mut changed = original.clone();
    changed.lexical.as_mut().unwrap().payload_fingerprint[0] ^= 1;
    assert!(changed.reload(ReloadLimits::default()).is_err());
    let mut changed = original.clone();
    changed.sources.clear();
    assert!(changed.reload(ReloadLimits::default()).is_err());
    let mut changed = bundle("@ns p = urn:first\n{plain} {#input}", true);
    let sidecar = changed.lexical.as_mut().unwrap();
    let plain = *sidecar
        .names
        .iter()
        .find(|(_, name)| name.local_name == "plain")
        .unwrap()
        .0;
    let (_, binding) = sidecar.namespace_bindings.pop_first().unwrap();
    sidecar.namespace_bindings.insert(plain, binding);
    assert!(changed.reload(ReloadLimits::default()).is_err());
}

#[test]
fn reload_limits_and_verified_late_source_handoff_are_enforced() {
    let mut bundle = bundle("{#input}", false);
    assert!(bundle
        .reload(ReloadLimits {
            max_bytes: 8,
            ..Default::default()
        })
        .is_err());
    assert!(bundle
        .reload(ReloadLimits {
            max_nodes: 1,
            ..Default::default()
        })
        .is_err());
    assert!(bundle.sources[0].supply_bytes(b"wrong".to_vec()).is_err());
    bundle.sources[0]
        .supply_bytes(b"{#input}".to_vec())
        .unwrap();
    assert_eq!(
        bundle
            .reload(ReloadLimits::default())
            .unwrap()
            .source_text(SourceId(0))
            .unwrap(),
        Some("{#input}")
    );
    let mut bytes = bundle.payload.clone();
    bytes[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    let hash_offset = bytes.len() - 8;
    let hash = bytes[..hash_offset]
        .iter()
        .fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ *b as u64).wrapping_mul(0x100000001b3)
        });
    bytes[hash_offset..].copy_from_slice(&hash.to_le_bytes());
    assert!(matches!(
        cem_ml::ast::DebugBinaryDecoder::new().decode_bounded(&bytes, 100),
        Err(cem_ml::ast::decode::DecodeError::CountLimitExceeded)
    ));
}
