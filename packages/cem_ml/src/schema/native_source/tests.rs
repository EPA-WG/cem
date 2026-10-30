use super::*;
use crate::{
    events::cem::CemEventNormalizer,
    schema::{machine::CemSchemaMachine, vocab::CompiledSchema},
    source::BytesSource,
    tokenizer::cem::CemTokenizer,
};
const TEXT: &str = "@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @name=test @namespace=urn:test | {body | {$ value == value }}}";
fn original() -> super::super::machine::LexicallyScopedDocument {
    CemSchemaMachine::new(
        CompiledSchema::cem_core(),
        CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
            SourceId(1),
            TEXT.as_bytes().to_vec(),
        ))),
    )
    .build_with_lexical_scopes()
}
fn identity() -> SourceIdentity {
    SourceIdentity::new(
        "file:///apps/validator/schema.cem",
        SourceId(1),
        TEXT.as_bytes(),
    )
}
fn snapshot() -> NativeSourceSnapshot {
    NativeSourceSnapshot::capture(&original(), &identity(), &mut || Ok(())).unwrap()
}
#[test]
fn original_text_names_and_occurrences_restore_against_new_allocations() {
    let original = original();
    let snap = NativeSourceSnapshot::capture(&original, &identity(), &mut || Ok(())).unwrap();
    let restored = snap.restore(&identity(), &mut || Ok(())).unwrap();
    assert!(!std::sync::Arc::ptr_eq(
        original.document(),
        restored.document()
    ));
    assert_eq!(
        original.document().nodes.len(),
        restored.document().nodes.len()
    );
    for node in &snap.nodes {
        if let Some(value) = &node.value {
            assert!(value.len() <= TEXT.len());
        }
    }
    let expression = original.occurrences().next().unwrap();
    assert!(restored.snapshot(restored.document(), expression).is_some());
    assert!(restored.snapshot(original.document(), expression).is_none());
}
#[test]
fn source_metadata_never_accepts_foreign_provenance_after_repinned_records() {
    let mut snapshot = snapshot();
    let binding = snapshot
        .lexical
        .namespace_bindings
        .values_mut()
        .next()
        .unwrap();
    binding.source_map.frames[0].source_id = SourceId(99);
    assert!(snapshot.restore(&identity(), &mut || Ok(())).is_err());
}
#[test]
fn owning_edges_names_indices_and_original_spans_are_bounded() {
    for mutation in 0..5 {
        let mut snapshot = snapshot();
        match mutation {
            0 => {
                let child = snapshot.nodes[0].children[0];
                snapshot.nodes[0].children.push(child);
            }
            1 => snapshot.nodes[0].children.push(u32::MAX),
            2 => snapshot.nodes[1].source.frames[0].source_id = SourceId(99),
            3 => snapshot.nodes[1].kind = "callback".into(),
            _ => snapshot
                .lexical
                .names
                .insert(
                    u32::MAX,
                    ExpandedName {
                        namespace_uri: String::new(),
                        local_name: "foreign".into(),
                        schema_id: None,
                    },
                )
                .map(|_| ())
                .unwrap_or(()),
        }
        assert!(snapshot.restore(&identity(), &mut || Ok(())).is_err());
    }
    assert!(snapshot()
        .restore(&identity(), &mut || Err("cancelled".into()))
        .is_err());
}
