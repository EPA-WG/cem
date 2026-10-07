use cem_ml::ast::reload::{ReferenceReloadBundle, ReloadError, ReloadLimits};
use cem_ql::api::reference_transport::RetainedReferenceSource;
use std::sync::Arc;

#[test]
fn later_metadata_and_verified_bytes_attach_without_replacing_the_arena() {
    let limits = ReloadLimits::default();
    let original = RetainedReferenceSource::parse(
        b"@ns p = urn:vendor\n{p:div}{#input}",
        "text/cem-ml",
        "memory:original.cem",
        limits,
    )
    .unwrap();
    let full = original.export_bundle(limits).unwrap();
    let mut partial = ReferenceReloadBundle::decode(&full, limits).unwrap();
    partial.lexical = None;
    for source in &mut partial.sources {
        source.bytes = None;
    }
    let mut retained =
        RetainedReferenceSource::reload(&partial.encode(limits).unwrap(), 1, limits).unwrap();
    let old = retained.clone();
    let reference = old
        .ingress()
        .source()
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            cem_ml::parser::CemAstNode::Reference { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .unwrap();
    assert_eq!(old.ingress().source().source_line_number(reference), None);
    assert!(retained.require_lexical().is_err());
    retained.attach_bundle(&full, limits).unwrap();
    assert!(Arc::ptr_eq(
        old.ingress().source().ast_owner(),
        retained.ingress().source().ast_owner()
    ));
    assert!(old.require_lexical().is_err());
    assert!(Arc::ptr_eq(
        retained.require_lexical().unwrap().document(),
        old.ingress().source().ast_owner()
    ));
    assert_eq!(
        retained
            .ingress()
            .reloaded()
            .source_text(cem_ml::source::SourceId(1))
            .unwrap(),
        Some("@ns p = urn:vendor\n{p:div}{#input}")
    );
    assert_eq!(
        retained.ingress().source().source_line_number(reference),
        original.ingress().source().source_line_number(reference)
    );
    assert!(retained
        .ingress()
        .source()
        .source_line_number(reference)
        .is_some());
    assert_eq!(old.ingress().source().source_line_number(reference), None);
    let capture = retained.require_lexical().unwrap().clone();
    retained.attach_bundle(&full, limits).unwrap();
    assert!(Arc::ptr_eq(&capture, retained.require_lexical().unwrap()));
}

#[test]
fn attachment_is_atomic_and_cannot_reinterpret_an_existing_capture() {
    let limits = ReloadLimits::default();
    let mut retained =
        RetainedReferenceSource::parse(b"{#input}", "text/cem-ml", "memory:original.cem", limits)
            .unwrap();
    let before = retained.export_bundle(limits).unwrap();
    let mut changed = ReferenceReloadBundle::decode(&before, limits).unwrap();
    changed.sources[0].uri = "memory:impostor.cem".into();
    assert!(matches!(
        retained.attach_bundle(&changed.encode(limits).unwrap(), limits),
        Err(ReloadError::InvalidSource)
    ));
    let other = RetainedReferenceSource::parse(
        b"{#different}",
        "text/cem-ml",
        "memory:original.cem",
        limits,
    )
    .unwrap();
    assert!(matches!(
        retained.attach_bundle(&other.export_bundle(limits).unwrap(), limits),
        Err(ReloadError::FingerprintMismatch)
    ));
    let mut invalid = ReferenceReloadBundle::decode(&before, limits).unwrap();
    invalid
        .lexical
        .as_mut()
        .unwrap()
        .diagnostics
        .push(Default::default());
    assert!(matches!(
        retained.attach_bundle(&invalid.encode(limits).unwrap(), limits),
        Err(ReloadError::InvalidMetadata)
    ));
    assert_eq!(retained.export_bundle(limits).unwrap(), before);
}
