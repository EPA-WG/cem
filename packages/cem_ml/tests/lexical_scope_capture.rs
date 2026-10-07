use cem_ml::{
    events::{cem::CemEventNormalizer, EventNormalizer, NormalizedEvent, ScalarValue},
    parser::{builder::CemAstBuilder, CemAstNode},
    schema::{
        machine::{CemSchemaMachine, LexicalScopeSnapshot},
        scoping::SchemaSource,
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    source_map::FrameSpan,
    tokenizer::cem::CemTokenizer,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

struct CountingEvents<E> {
    inner: E,
    reads: Arc<AtomicUsize>,
}
impl<E: EventNormalizer> EventNormalizer for CountingEvents<E> {
    fn next_event(&mut self) -> Option<NormalizedEvent> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.inner.next_event()
    }
}

#[test]
fn one_stream_retains_occurrence_bindings_without_evaluation_or_retroactive_rebinding() {
    let source = "@ns v = urn:first\n@default v\n{before @target={#missing} | {#missing}}\n@ns v = urn:second\n@default v\n{after | {#missing}}";
    let observations = Arc::new(Mutex::new(Vec::new()));
    let saved = observations.clone();
    let normalizer = CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
        SourceId(7),
        source.as_bytes().to_vec(),
    )));
    let events = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer).track_lexical_scope(
        move |event, machine| {
            if let Some(event) = event {
                let range = match event {
                    NormalizedEvent::OpenScope {
                        name, byte_range, ..
                    } if name.lexical_name == "$" => Some(*byte_range),
                    NormalizedEvent::Value {
                        value: ScalarValue::Expression(_),
                        byte_range,
                    } => Some(*byte_range),
                    _ => None,
                };
                if let Some(range) = range {
                    saved
                        .lock()
                        .unwrap()
                        .push((range, machine.lexical_snapshot()));
                }
            }
        },
    );
    let doc = CemAstBuilder::new(events).build();
    assert!(doc.diagnostics.is_empty(), "{:?}", doc.diagnostics);
    let saved = observations.lock().unwrap();
    assert_eq!(saved.len(), 3);
    for (index, (range, snapshot)) in saved.iter().enumerate() {
        let expected = if index < 2 { "urn:first" } else { "urn:second" };
        assert_eq!(
            snapshot.namespaces.binding("").unwrap().namespace_uri,
            expected
        );
        assert_eq!(
            snapshot.namespaces.binding("v").unwrap().namespace_uri,
            expected
        );
        assert!(snapshot.namespaces.binding("v").unwrap().declared_at.start < range.start);
        assert!(doc.nodes.iter().any(|node| matches!(node, CemAstNode::Reference {targets: None, source, ..} if source.frames.iter().any(|frame| matches!(frame.span, FrameSpan::Single(span) if span == *range || (span.start >= range.start && span.end() <= range.end()))))));
    }
    assert_eq!(
        doc.nodes
            .iter()
            .filter(|node| matches!(node, CemAstNode::Reference { targets: None, .. }))
            .count(),
        3
    );
}

#[test]
fn schema_snapshots_keep_host_wrapping_sibling_sources_and_restore_parent() {
    let source = "@schema src=outer\n{section @cem:schema-src=host | {#a} {cem:schema @select='missing()' | {#b}} {#c} {cem:schema @src=sibling} {#d}} {#e}";
    let snapshots = Arc::new(Mutex::new(Vec::<LexicalScopeSnapshot>::new()));
    let saved = snapshots.clone();
    let normalizer = CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
        SourceId(2),
        source.as_bytes().to_vec(),
    )));
    let events = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer)
        .track_lexical_scope(move |event, machine| {
            if matches!(event, Some(NormalizedEvent::OpenScope {name, ..}) if name.lexical_name == "$") {
                saved.lock().unwrap().push(machine.lexical_snapshot());
            }
        });
    let doc = CemAstBuilder::new(events).build();
    let snapshots = snapshots.lock().unwrap();
    assert_eq!(
        snapshots
            .iter()
            .map(|snapshot| snapshot.schema.active.clone())
            .collect::<Vec<_>>(),
        vec![
            SchemaSource::Uri("host".into()),
            SchemaSource::Select("missing()".into()),
            SchemaSource::Uri("host".into()),
            SchemaSource::Uri("sibling".into()),
            SchemaSource::Uri("outer".into()),
        ]
    );
    assert_eq!(
        doc.nodes
            .iter()
            .filter(|node| matches!(node, CemAstNode::Reference { targets: None, .. }))
            .count(),
        5
    );
}

#[test]
fn tracking_completion_reports_final_diagnostics_once_and_does_not_consume_extra_events() {
    let completions = Arc::new(Mutex::new(Vec::new()));
    let saved = completions.clone();
    let normalizer = CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
        SourceId(3),
        b"{section".to_vec(),
    )));
    let reads = Arc::new(AtomicUsize::new(0));
    let normalizer = CountingEvents {
        inner: normalizer,
        reads: reads.clone(),
    };
    let mut events = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer)
        .track_lexical_scope(move |event, machine| {
            if event.is_none() {
                saved.lock().unwrap().push(machine.diagnostics().to_vec());
            }
        });
    while events.next_event().is_some() {}
    let reads_at_eof = reads.load(Ordering::Relaxed);
    assert!(events.next_event().is_none());
    assert!(events.next_event().is_none());
    assert_eq!(reads.load(Ordering::Relaxed), reads_at_eof);
    let completions = completions.lock().unwrap();
    assert_eq!(completions.len(), 1);
    assert!(completions[0]
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.schema.unclosed_scope"));
}

fn scoped_document(source: &str) -> cem_ml::schema::machine::LexicallyScopedDocument {
    let normalizer = CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
        SourceId(19),
        source.as_bytes().to_vec(),
    )));
    CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer).build_with_lexical_scopes()
}

#[test]
fn completed_associations_use_original_owner_and_builder_ids_for_all_expression_slots() {
    let source = "@ns v = urn:first\n{before @target={#missing} @general={items} | {#missing}}\n@ns v = urn:second\n{after | {#missing}}";
    let scoped = scoped_document(source);
    let owner = scoped.document().clone();
    let foreign = scoped_document(source);
    let occurrences = scoped.occurrences().collect::<Vec<_>>();
    assert_eq!(occurrences.len(), 4);
    for (index, node) in occurrences.iter().copied().enumerate() {
        let snapshot = scoped.snapshot(&owner, node).unwrap();
        assert_eq!(
            snapshot.namespaces.binding("v").unwrap().namespace_uri,
            if index < 3 { "urn:first" } else { "urn:second" }
        );
        assert!(scoped.snapshot(foreign.document(), node).is_none());
        assert!(matches!(
            owner.get(node),
            Some(CemAstNode::Reference { targets: None, .. }) | Some(CemAstNode::Element { .. })
        ));
    }
    // General expressions keep their native payload node, never the attribute or
    // temporary text children removed while folding standalone references.
    let general = occurrences[1];
    assert!(
        matches!(owner.get(general), Some(CemAstNode::Element { expanded_name, .. })
        if expanded_name.local_name == "$")
    );
    assert!(scoped.snapshot(&owner, 0).is_none());
    assert!(scoped.snapshot(&owner, u32::MAX).is_none());
    for (id, node) in owner.nodes.iter().enumerate() {
        if matches!(node, CemAstNode::Text { .. } | CemAstNode::Attribute { .. }) {
            assert!(scoped.snapshot(&owner, id as u32).is_none());
        }
    }
}

#[test]
fn completed_associations_restore_schema_and_namespace_defaults_and_keep_pending_diagnostics() {
    let source = "@ns v = urn:outer\n@schema src=outer\n{section @cem:schema-src=host | {#a} {cem:schema @select='missing()' @xmlns:v=urn:inner | {#b}} {#c} {cem:schema @src=sibling} {#d}} {#e}";
    let scoped = scoped_document(source);
    let associations = scoped
        .occurrences()
        .map(|node| {
            let snapshot = scoped.snapshot(scoped.document(), node).unwrap();
            (
                snapshot.schema.active.clone(),
                snapshot
                    .namespaces
                    .binding("v")
                    .unwrap()
                    .namespace_uri
                    .clone(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        associations,
        vec![
            (SchemaSource::Uri("host".into()), "urn:outer".into()),
            (SchemaSource::Select("missing()".into()), "urn:inner".into()),
            (SchemaSource::Uri("host".into()), "urn:outer".into()),
            (SchemaSource::Uri("sibling".into()), "urn:outer".into()),
            (SchemaSource::Uri("outer".into()), "urn:outer".into()),
        ]
    );
    assert_eq!(
        scoped
            .document()
            .nodes
            .iter()
            .filter(|node| matches!(node, CemAstNode::Reference { targets: None, .. }))
            .count(),
        5
    );
    let incomplete = scoped_document("{section");
    assert!(incomplete
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.schema.unclosed_scope"));
    assert_eq!(incomplete.occurrences().count(), 0);
}

#[test]
fn captured_names_keep_source_position_bindings_and_original_owner() {
    let source = "@ns s = https://cem.dev/ns/schema/1\n@ns c = https://cem.dev/ns/core/1\n@default s\n{c:schema @c:name=inline @c:flag @c:target={#later} | {s:schema}}\n@ns s = urn:other\n@default s\n{schema @xmlns:s=urn:inner | {s:schema} }\n{s:schema} {missing:schema} {#later}";
    let normalizer = CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
        SourceId(1),
        source.as_bytes().to_vec(),
    )));
    let captured =
        CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer).build_with_lexical_scopes();
    let owner = captured.document();
    let wrapper = owner
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.namespace_uri == "c" && expanded_name.local_name == "schema" => {
                cem_ml::schema::declaration_references::SchemaDeclarationNode::new(
                    owner.clone(),
                    *node_id,
                )
            }
            _ => None,
        })
        .unwrap();
    let admitted =
        cem_ml::schema::scope_references::admit_schema_scope_target(wrapper.clone(), |node| {
            captured
                .expanded_name(node.document(), node.node_id())
                .cloned()
        })
        .unwrap();
    assert!(Arc::ptr_eq(admitted.declaration.document(), owner));
    let CemAstNode::Element { attributes, .. } = wrapper.node() else {
        unreachable!()
    };
    assert_eq!(attributes.len(), 3);
    for id in attributes {
        assert_eq!(
            captured.expanded_name(owner, *id).unwrap().namespace_uri,
            "https://cem.dev/ns/core/1"
        );
    }
    let schema_names: Vec<_> = owner
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "schema" => Some(
                captured
                    .expanded_name(owner, *node_id)
                    .map(|name| name.namespace_uri.as_str()),
            ),
            _ => None,
        })
        .collect();
    assert_eq!(
        schema_names,
        vec![
            Some("https://cem.dev/ns/core/1"),
            Some("https://cem.dev/ns/schema/1"),
            Some("urn:other"),
            Some("urn:inner"),
            Some("urn:other"),
            None
        ]
    );
    let other_owner = Arc::new(
        CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
            BytesSource::new(SourceId(1), source.as_bytes().to_vec()),
        )))
        .build(),
    );
    assert!(captured
        .expanded_name(&other_owner, wrapper.node_id())
        .is_none());
    for id in captured.occurrences() {
        if matches!(owner.get(id), Some(CemAstNode::Reference { .. })) {
            assert!(captured.expanded_name(owner, id).is_none());
        }
    }
}

#[test]
fn intrinsic_namespace_attribute_names_do_not_require_an_xmlns_binding() {
    let text = "{item @xmlns:v=urn:inner @missing:flag=yes | {v:child}} {xmlns:item}";
    let normalizer = CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
        SourceId(1),
        text.as_bytes().to_vec(),
    )));
    let captured =
        CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer).build_with_lexical_scopes();
    let owner = captured.document();
    for node in &owner.nodes {
        match node {
            CemAstNode::Attribute {
                node_id,
                expanded_name,
                ..
            } if expanded_name.namespace_uri == "xmlns" => {
                let name = captured
                    .expanded_name(owner, *node_id)
                    .expect("intrinsic namespace declaration name");
                assert_eq!(name.namespace_uri, "http://www.w3.org/2000/xmlns/");
                assert_eq!(name.local_name, "v");
            }
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.namespace_uri == "xmlns" => {
                assert!(captured.expanded_name(owner, *node_id).is_none());
            }
            CemAstNode::Attribute {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "flag" => {
                assert!(captured.expanded_name(owner, *node_id).is_none());
            }
            _ => {}
        }
    }
}
