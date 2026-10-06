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
