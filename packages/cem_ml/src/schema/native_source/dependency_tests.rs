use super::dependencies::NativeDependencySourceSnapshot;
use super::*;
use crate::{
    events::cem::CemEventNormalizer,
    schema::{machine::CemSchemaMachine, vocab::CompiledSchema},
    source::BytesSource,
    tokenizer::cem::CemTokenizer,
};
const TEXT: &str = "@ns s = https://cem.dev/ns/schema/1\n@ns v = urn:vendor\n@default s\n{schema @namespace=urn:test | {types | {type @name=local @base={#library#base} @rule={#library#check}}} {body | {#other}}}";
fn captured() -> super::super::machine::LexicallyScopedDocument {
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
    SourceIdentity::new("file:///app/types.cem", SourceId(1), TEXT.as_bytes())
}
#[test]
fn dependency_source_retains_original_reference_contexts_fields_and_lexical_owners() {
    let original = captured();
    assert!(NativeSourceSnapshot::capture(&original, &identity(), &mut || Ok(())).is_err());
    let snapshot =
        NativeDependencySourceSnapshot::capture(&original, &identity(), &mut || Ok(())).unwrap();
    let restored = snapshot.restore(&identity(), &mut || Ok(())).unwrap();
    assert!(!std::sync::Arc::ptr_eq(
        original.document(),
        restored.document()
    ));
    assert_eq!(
        format!("{:?}", original.document().nodes),
        format!("{:?}", restored.document().nodes)
    );
    let references = original
        .document()
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Reference { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(references.len(), 3);
    for id in references {
        assert!(restored.snapshot(restored.document(), id).is_some());
        assert!(restored.snapshot(original.document(), id).is_none());
    }
    assert!(NativeSourceSnapshot::capture(&restored, &identity(), &mut || Ok(())).is_err());
}
#[test]
fn dependency_source_rejects_forged_reference_contexts_targets_and_partial_snapshots() {
    let snapshot = || {
        NativeDependencySourceSnapshot::capture(&captured(), &identity(), &mut || Ok(())).unwrap()
    };
    for case in 0..7 {
        let mut s = snapshot();
        match case {
            0 => {
                *s.contexts.values_mut().next().unwrap() = u32::MAX;
            }
            1 => {
                s.contexts.pop_first();
            }
            2 => {
                s.contexts.insert(u32::MAX, 0);
            }
            3 => {
                s.lexical.occurrences.clear();
            }
            4 => {
                s.nodes[1].source.frames[0].source_id = SourceId(99);
            }
            5 => {
                *s.contexts.values_mut().next().unwrap() = 0;
            }
            _ => {
                s.version = 2;
            }
        }
        if matches!(case, 0 | 5) {
            // Check container ownership independently of lexical integrity pins.
            assert!(super::restore_profile(
                &s.nodes,
                &identity(),
                true,
                Some(&s.contexts),
                &mut || Ok(())
            )
            .is_err());
        }
        assert!(
            s.restore(&identity(), &mut || Ok(())).is_err(),
            "case {case}"
        );
    }
    let original = captured();
    // A new owner is needed to mutate an otherwise immutable native source.
    let mut document = crate::parser::document::CemDocument::default();
    document.nodes = original.document().nodes.clone();
    if let Some(CemAstNode::Reference { targets, .. }) = document
        .nodes
        .iter_mut()
        .find(|n| matches!(n, CemAstNode::Reference { .. }))
    {
        *targets = Some(vec![]);
    }
    assert!(super::dependencies::capture_nodes(&document).is_err());
    assert!(snapshot()
        .restore(&identity(), &mut || Err("cancelled".into()))
        .is_err());
}
