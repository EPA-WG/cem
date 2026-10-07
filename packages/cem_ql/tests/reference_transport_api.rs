use cem_ml::{
    ast::reload::{ReloadDependency, ReloadLimits},
    value::artifact::CemValueArtifactLimits,
};
use cem_ql::{
    api::{reference_transport::RetainedReferenceSource, StandaloneExpressionContext},
    eval::{portable::NativeExportFailureKind, retained_cem_node},
};
use std::sync::Arc;

#[test]
fn executable_bundle_queries_keep_native_owners_and_guard_materialized_export() {
    let source = RetainedReferenceSource::parse(
        b"{item}{#input}",
        "text/cem-ml",
        "memory:source.cem",
        ReloadLimits::default(),
    )
    .unwrap();
    let bytes = source.export_bundle(ReloadLimits::default()).unwrap();
    let loaded = RetainedReferenceSource::reload(&bytes, 1, ReloadLimits::default()).unwrap();
    assert!(loaded.require_lexical().is_ok());
    assert!(!Arc::ptr_eq(
        source.ingress().source().ast_owner(),
        loaded.ingress().source().ast_owner()
    ));
    for _ in 0..2 {
        let result = loaded
            .evaluate("input", &StandaloneExpressionContext::default())
            .unwrap();
        assert!(Arc::ptr_eq(
            retained_cem_node(&result.result.items[0]).unwrap().owner(),
            loaded.ingress().source()
        ));
        let failure = cem_ql::eval::portable::export_values(
            &result.result,
            &CemValueArtifactLimits::default(),
        )
        .unwrap_err();
        assert_eq!(
            failure.kind,
            NativeExportFailureKind::UnsupportedSourceReference
        );
        assert!(failure.source.is_some());
        let empty = loaded
            .evaluate("#()", &StandaloneExpressionContext::default())
            .unwrap();
        assert!(cem_ql::eval::portable::export_values(
            &empty.result,
            &CemValueArtifactLimits::default()
        )
        .is_ok());
    }
}

#[test]
fn ast_inspection_does_not_claim_capture_readiness_or_admit_cemv_as_source() {
    let source = RetainedReferenceSource::parse(
        b"{#input}",
        "text/cem-ml",
        "memory:source.cem",
        ReloadLimits::default(),
    )
    .unwrap();
    let bytes = source.export_bundle(ReloadLimits::default()).unwrap();
    let mut bundle =
        cem_ml::ast::reload::ReferenceReloadBundle::decode(&bytes, ReloadLimits::default())
            .unwrap();
    bundle.lexical = None;
    let loaded = RetainedReferenceSource::reload(
        &bundle.encode(ReloadLimits::default()).unwrap(),
        1,
        ReloadLimits::default(),
    )
    .unwrap();
    assert_eq!(
        loaded.require_lexical().unwrap_err(),
        ReloadDependency::MissingLexicalMetadata
    );
    assert!(loaded
        .evaluate(
            "input.children.kind",
            &StandaloneExpressionContext::default()
        )
        .is_ok());
    let values = loaded
        .evaluate("#()", &StandaloneExpressionContext::default())
        .unwrap();
    let cemv =
        cem_ql::eval::portable::export_values(&values.result, &CemValueArtifactLimits::default())
            .unwrap();
    assert!(RetainedReferenceSource::reload(&cemv, 1, ReloadLimits::default()).is_err());
}

#[test]
fn execution_bindings_are_fresh_and_do_not_rewrite_saved_source() {
    use cem_ql::{
        api::StandaloneExpressionBinding,
        eval::{AtomValue, Item, ItemStream},
    };
    let source = RetainedReferenceSource::parse(
        b"{#datadom}",
        "text/cem-ml",
        "memory:source.cem",
        ReloadLimits::default(),
    )
    .unwrap();
    let before = source.export_bundle(ReloadLimits::default()).unwrap();
    for label in ["one", "two"] {
        let context = StandaloneExpressionContext::default().with_binding(
            "datadom",
            StandaloneExpressionBinding::any(ItemStream::once(Item::Atomic(AtomValue::String(
                label.into(),
            )))),
        );
        let evaluated = source
            .evaluate(
                "(datadom, input.children.expression, input.children.targets_available)",
                &context,
            )
            .unwrap();
        assert_eq!(
            evaluated.result.items[0].atom(),
            Some(AtomValue::String(label.into()))
        );
        assert_eq!(
            evaluated.result.items[1].atom(),
            Some(AtomValue::String("#datadom".into()))
        );
        assert_eq!(
            evaluated.result.items[2].atom(),
            Some(AtomValue::Boolean(false))
        );
    }
    assert_eq!(
        source.export_bundle(ReloadLimits::default()).unwrap(),
        before
    );
}

#[test]
fn reload_source_attribution_does_not_borrow_primary_coordinates() {
    use cem_ml::{
        ast::reload::{ReferenceReloadBundle, ReloadIngress, ReloadSource},
        parser::CemAstNode,
        source::SourceId,
    };
    let source = RetainedReferenceSource::parse(
        b"{div}\n{#input}",
        "text/cem-ml",
        "memory:source.cem",
        ReloadLimits::default(),
    )
    .unwrap();
    let bytes = source.export_bundle(ReloadLimits::default()).unwrap();
    let mut bundle = ReferenceReloadBundle::decode(&bytes, ReloadLimits::default()).unwrap();
    bundle.lexical = None;
    let mut reloaded = bundle.reload(ReloadLimits::default()).unwrap();
    let document = Arc::get_mut(&mut reloaded.document).unwrap();
    let id = document
        .nodes
        .iter_mut()
        .find_map(|node| match node {
            CemAstNode::Reference {
                source, node_id, ..
            } => {
                source.frames[0].source_id = SourceId(2);
                source.frames[0].span =
                    cem_ml::source_map::FrameSpan::Single(cem_ml::source::ByteRange::new(6, 8));
                Some(*node_id)
            }
            _ => None,
        })
        .unwrap();
    reloaded.sources.push(ReloadSource::new(
        SourceId(2),
        "memory:vendor.cem",
        b"{div}\n{#input}",
        false,
    ));
    let ingress = ReloadIngress::new(Arc::new(reloaded), SourceId(1)).unwrap();
    let range = ingress.source().source_node_range(id).unwrap();
    assert_eq!((range.line, range.column), (0, 0));
    assert!(range.offset > 0);
    let node = cem_ql::eval::RetainedCemNode::new(ingress.source().clone(), id)
        .unwrap()
        .query_item();
    let provenance = node.view().unwrap().provenance().unwrap();
    assert_eq!(provenance.source_uri.as_deref(), Some("memory:vendor.cem"));
    assert_eq!(provenance.line_number, None);
}

#[test]
fn cem_and_xml_reload_keep_expression_slots_and_literal_attribute_boundaries() {
    for (media, text, refs) in [
        ("text/cem-ml", "{div @target={#input} | {#input}}", 2),
        ("application/xml", "<div xmlns:r='https://cem.dev/ns/cem-ml/1' target='{#input}'><r:expr>#input</r:expr></div>", 1),
    ] {
        let source = RetainedReferenceSource::parse(text.as_bytes(), media, "memory:input", ReloadLimits::default()).unwrap();
        let bytes = source.export_bundle(ReloadLimits::default()).unwrap();
        let loaded = RetainedReferenceSource::reload(&bytes, 1, ReloadLimits::default()).unwrap();
        let occurrences: Vec<_> = loaded.require_lexical().unwrap().occurrences().collect();
        assert_eq!(occurrences.len(), refs);
        for id in occurrences {
            assert!(matches!(loaded.ingress().source().ast().get(id), Some(cem_ml::parser::CemAstNode::Reference { expression, targets: None, .. }) if expression == "#input"));
        }
        if media == "application/xml" {
            assert!(loaded.ingress().source().ast().nodes.iter().any(|node| matches!(node, cem_ml::parser::CemAstNode::Attribute { value: Some(value), value_nodes, .. } if value == "{#input}" && value_nodes.is_empty())));
        }
    }
}
