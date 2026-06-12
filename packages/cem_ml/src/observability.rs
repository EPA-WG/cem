//! Public observability surface (AC-O-1, AC-O-3).
//!
//! Tier A exposes three named event channels — `onParseEvent`,
//! `onValidate`, `onTransform` — through the [`EngineObserver`] trait.
//! Payload categories and channel names are stable so editor tooling,
//! WASM consumers, and CLI projections can rely on them across crate
//! versions. Implementations MAY surface observers as callbacks or as
//! async streams; the canonical storage form remains the
//! [`ReportEvent`] discriminated union.
//!
//! Each event carries the cross-cutting metadata mandated by AC-O-3:
//!
//! - a monotonic [`sequence`](ReportEvent::sequence) number,
//! - the originating [`byte_offset`](ReportEvent::byte_offset),
//! - the [`source_map`](ReportEvent::source_map) stack as it exists at
//!   emission time.
//!
//! Implementations append channel-specific payload through one of the
//! three [`ReportEvent`] variants.
//!
//! The observability surface is *parallel* to the normal
//! parse/validate/transform calls — it never blocks the canonical
//! result computation. Buffering observers (see [`BufferingObserver`])
//! record events into a `Vec<ReportEvent>` for tests and the CLI
//! report projection.

use crate::diagnostics::Diagnostic;
use crate::source_map::{SourceMapStack, TransformKind};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Public name of the three observable channels exposed by the engine.
/// The string form is stable per AC-O-1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventChannel {
    Parse,
    Validate,
    Transform,
}

impl EventChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            EventChannel::Parse => "parse",
            EventChannel::Validate => "validate",
            EventChannel::Transform => "transform",
        }
    }
}

/// Stable categorical kind emitted on the `parse` channel. Mirrors the
/// shared [`crate::events::NormalizedEvent`] variants so every consumer
/// gets the same vocabulary regardless of the input profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseEventKind {
    OpenScope,
    CloseScope,
    Name,
    Value,
    Trivia,
    Separator,
    ModeSwitch,
    ProcessingInstruction,
    Error,
}

impl ParseEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ParseEventKind::OpenScope => "open_scope",
            ParseEventKind::CloseScope => "close_scope",
            ParseEventKind::Name => "name",
            ParseEventKind::Value => "value",
            ParseEventKind::Trivia => "trivia",
            ParseEventKind::Separator => "separator",
            ParseEventKind::ModeSwitch => "mode_switch",
            ParseEventKind::ProcessingInstruction => "processing_instruction",
            ParseEventKind::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseReportEvent {
    pub kind: ParseEventKind,
    /// Lexical name attached to the event, when one exists (scope
    /// names, attribute names, mode-switch content types, processing-
    /// instruction targets, diagnostic codes).
    pub name: Option<String>,
    /// Scalar value associated with the event (attribute or text data,
    /// trivia/separator literal). The serialized form is verbatim from
    /// the normalizer.
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateReportEvent {
    /// Stable rule/diagnostic code (`cem.ref.unresolved_reference`,
    /// `cem.lint.relaxed_content_boundary`, …). Channel consumers
    /// route on this field.
    pub code: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformReportEvent {
    /// Originating transform layer (`CemTokenizer`, `CemAstBuilder`,
    /// `HandoffBoundary`, `ContentTypeTransform`, …). Mirrors
    /// [`TransformKind`] so consumers can correlate with the source-map
    /// stack frames they already index.
    pub transform: TransformKind,
    /// Free-form summary intended for human-readable trace projections;
    /// the canonical signal is [`transform`](Self::transform).
    pub summary: String,
}

/// A single observable engine event. The `channel` field discriminates
/// the payload; the cross-cutting `sequence` / `byte_offset` /
/// `source_map` fields are shared so consumers can sort or correlate
/// across channels with one comparator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportEvent {
    pub sequence: u64,
    pub channel: EventChannel,
    #[serde(rename = "byteOffset", skip_serializing_if = "Option::is_none")]
    pub byte_offset: Option<u64>,
    #[serde(rename = "sourceMap", skip_serializing_if = "Option::is_none")]
    pub source_map: Option<SourceMapStack>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse: Option<ParseReportEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate: Option<ValidateReportEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform: Option<TransformReportEvent>,
}

impl ReportEvent {
    pub fn channel(&self) -> EventChannel {
        self.channel
    }

    /// Stable categorical descriptor for routing or filtering. The
    /// shape is `"<channel>.<kind>"`, e.g. `"parse.open_scope"`,
    /// `"validate.diagnostic"`, `"transform.applied"`.
    pub fn kind(&self) -> String {
        match self.channel {
            EventChannel::Parse => {
                let k = self
                    .parse
                    .as_ref()
                    .map(|p| p.kind.as_str())
                    .unwrap_or("unknown");
                format!("parse.{k}")
            }
            EventChannel::Validate => "validate.diagnostic".to_owned(),
            EventChannel::Transform => "transform.applied".to_owned(),
        }
    }
}

/// Observer trait surfaced to public consumers (Rust callers, WASM
/// bindings layered on top, CLI projection). Each method is the named
/// public entry point per AC-O-1.
///
/// All methods receive a borrowed event. Concrete observers MAY clone
/// the event into an owned buffer (see [`BufferingObserver`]) or
/// forward it to an async stream.
pub trait EngineObserver: Send + Sync {
    fn on_parse_event(&self, event: &ReportEvent);
    fn on_validate(&self, event: &ReportEvent);
    fn on_transform(&self, event: &ReportEvent);
}

/// `EngineObserver` adapter that records every emitted event into a
/// `Vec<ReportEvent>` in arrival order. Intended for tests, CLI report
/// projection, and any embedding that prefers a "drain at the end"
/// model over per-event callbacks.
#[derive(Debug, Clone, Default)]
pub struct BufferingObserver {
    events: Arc<Mutex<Vec<ReportEvent>>>,
}

impl BufferingObserver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the snapshot of events captured so far (cheap clone of
    /// the underlying buffer).
    pub fn snapshot(&self) -> Vec<ReportEvent> {
        self.events.lock().expect("poisoned observer mutex").clone()
    }

    pub fn drain(&self) -> Vec<ReportEvent> {
        std::mem::take(&mut *self.events.lock().expect("poisoned observer mutex"))
    }

    pub fn len(&self) -> usize {
        self.events.lock().expect("poisoned observer mutex").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn record(&self, event: &ReportEvent) {
        self.events
            .lock()
            .expect("poisoned observer mutex")
            .push(event.clone());
    }
}

impl EngineObserver for BufferingObserver {
    fn on_parse_event(&self, event: &ReportEvent) {
        self.record(event);
    }
    fn on_validate(&self, event: &ReportEvent) {
        self.record(event);
    }
    fn on_transform(&self, event: &ReportEvent) {
        self.record(event);
    }
}

/// Internal sequence-number generator used by [`EventEmitter`] so
/// every emitted event carries a monotonic id even when multiple
/// channels interleave.
#[derive(Debug, Default)]
pub struct EventSequencer {
    next: u64,
}

impl EventSequencer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_sequence(&mut self) -> u64 {
        let id = self.next;
        self.next += 1;
        id
    }
}

/// Thin builder bound to a single observer + sequencer so the
/// engine can emit events without re-stating the cross-cutting
/// fields at every callsite.
pub struct EventEmitter<'a> {
    observer: &'a dyn EngineObserver,
    sequencer: &'a mut EventSequencer,
}

impl<'a> EventEmitter<'a> {
    pub fn new(observer: &'a dyn EngineObserver, sequencer: &'a mut EventSequencer) -> Self {
        Self {
            observer,
            sequencer,
        }
    }

    pub fn parse(
        &mut self,
        kind: ParseEventKind,
        name: Option<String>,
        value: Option<String>,
        byte_offset: Option<u64>,
        source_map: Option<SourceMapStack>,
    ) {
        let event = ReportEvent {
            sequence: self.sequencer.next_sequence(),
            channel: EventChannel::Parse,
            byte_offset,
            source_map,
            parse: Some(ParseReportEvent { kind, name, value }),
            validate: None,
            transform: None,
        };
        self.observer.on_parse_event(&event);
    }

    pub fn validate(&mut self, diag: &Diagnostic) {
        let event = ReportEvent {
            sequence: self.sequencer.next_sequence(),
            channel: EventChannel::Validate,
            byte_offset: diag.byte_offset,
            source_map: diag.source_map.clone(),
            parse: None,
            validate: Some(ValidateReportEvent {
                code: diag.code.clone(),
                severity: severity_label(diag.severity).to_owned(),
                message: diag.message.clone(),
            }),
            transform: None,
        };
        self.observer.on_validate(&event);
    }

    pub fn transform(
        &mut self,
        transform: TransformKind,
        summary: impl Into<String>,
        byte_offset: Option<u64>,
        source_map: Option<SourceMapStack>,
    ) {
        let event = ReportEvent {
            sequence: self.sequencer.next_sequence(),
            channel: EventChannel::Transform,
            byte_offset,
            source_map,
            parse: None,
            validate: None,
            transform: Some(TransformReportEvent {
                transform,
                summary: summary.into(),
            }),
        };
        self.observer.on_transform(&event);
    }
}

fn severity_label(severity: crate::diagnostics::Severity) -> &'static str {
    use crate::diagnostics::Severity;
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
        Severity::Fatal => "fatal",
    }
}

/// Project a list of [`ReportEvent`]s into newline-delimited JSON.
/// One event per line; well-suited for CLI piping or file output.
pub fn events_to_jsonl(events: &[ReportEvent]) -> String {
    let mut out = String::new();
    for event in events {
        if let Ok(line) = serde_json::to_string(event) {
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Severity;

    fn sample_parse(seq: u64) -> ReportEvent {
        ReportEvent {
            sequence: seq,
            channel: EventChannel::Parse,
            byte_offset: Some(7),
            source_map: None,
            parse: Some(ParseReportEvent {
                kind: ParseEventKind::OpenScope,
                name: Some("button".to_owned()),
                value: None,
            }),
            validate: None,
            transform: None,
        }
    }

    fn sample_validate(seq: u64) -> ReportEvent {
        ReportEvent {
            sequence: seq,
            channel: EventChannel::Validate,
            byte_offset: Some(13),
            source_map: None,
            parse: None,
            validate: Some(ValidateReportEvent {
                code: "cem.ref.unresolved_reference".to_owned(),
                severity: "warning".to_owned(),
                message: "missing".to_owned(),
            }),
            transform: None,
        }
    }

    fn sample_transform(seq: u64) -> ReportEvent {
        ReportEvent {
            sequence: seq,
            channel: EventChannel::Transform,
            byte_offset: None,
            source_map: None,
            parse: None,
            validate: None,
            transform: Some(TransformReportEvent {
                transform: TransformKind::CemTokenizer,
                summary: "tokenized".to_owned(),
            }),
        }
    }

    #[test]
    fn channel_strings_are_stable() {
        assert_eq!(EventChannel::Parse.as_str(), "parse");
        assert_eq!(EventChannel::Validate.as_str(), "validate");
        assert_eq!(EventChannel::Transform.as_str(), "transform");
    }

    #[test]
    fn parse_event_kind_strings_are_stable() {
        let pairs = [
            (ParseEventKind::OpenScope, "open_scope"),
            (ParseEventKind::CloseScope, "close_scope"),
            (ParseEventKind::Name, "name"),
            (ParseEventKind::Value, "value"),
            (ParseEventKind::Trivia, "trivia"),
            (ParseEventKind::Separator, "separator"),
            (ParseEventKind::ModeSwitch, "mode_switch"),
            (ParseEventKind::ProcessingInstruction, "processing_instruction"),
            (ParseEventKind::Error, "error"),
        ];
        for (kind, expected) in pairs {
            assert_eq!(kind.as_str(), expected);
        }
    }

    #[test]
    fn report_event_kind_returns_channel_dot_kind() {
        assert_eq!(sample_parse(0).kind(), "parse.open_scope");
        assert_eq!(sample_validate(1).kind(), "validate.diagnostic");
        assert_eq!(sample_transform(2).kind(), "transform.applied");
    }

    #[test]
    fn buffering_observer_records_each_channel_once() {
        let observer = BufferingObserver::new();
        observer.on_parse_event(&sample_parse(0));
        observer.on_validate(&sample_validate(1));
        observer.on_transform(&sample_transform(2));
        let captured = observer.snapshot();
        assert_eq!(captured.len(), 3);
        assert_eq!(captured[0].channel, EventChannel::Parse);
        assert_eq!(captured[1].channel, EventChannel::Validate);
        assert_eq!(captured[2].channel, EventChannel::Transform);
    }

    #[test]
    fn event_emitter_assigns_monotonic_sequence_numbers() {
        let observer = BufferingObserver::new();
        let mut seq = EventSequencer::new();
        {
            let mut emit = EventEmitter::new(&observer, &mut seq);
            emit.parse(ParseEventKind::OpenScope, Some("p".into()), None, Some(0), None);
            emit.validate(&crate::diagnostics::Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: Some(4),
                code: "cem.test.diag".into(),
                severity: Severity::Warning,
                message: "hello".into(),
                node: None,
                source_map: None,
            });
            emit.transform(TransformKind::CemTokenizer, "tokenized", None, None);
        }
        let captured = observer.snapshot();
        assert_eq!(captured.iter().map(|e| e.sequence).collect::<Vec<_>>(), vec![0, 1, 2]);
    }

    #[test]
    fn event_emitter_sets_only_the_matching_channel_payload() {
        let observer = BufferingObserver::new();
        let mut seq = EventSequencer::new();
        {
            let mut emit = EventEmitter::new(&observer, &mut seq);
            emit.parse(ParseEventKind::OpenScope, None, None, Some(0), None);
            emit.validate(&crate::diagnostics::Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: Some(4),
                code: "cem.schema.scoping.exclusive_src_select".into(),
                severity: Severity::Error,
                message: "exclusive selectors conflict".into(),
                node: None,
                source_map: None,
            });
            emit.transform(
                TransformKind::SchemaValidation { schema_id: 7 },
                "validated",
                None,
                None,
            );
        }

        let captured = observer.snapshot();
        assert_eq!(captured.len(), 3);

        let parse = &captured[0];
        assert_eq!(parse.channel, EventChannel::Parse);
        assert!(parse.parse.is_some());
        assert!(parse.validate.is_none());
        assert!(parse.transform.is_none());

        let validate = &captured[1];
        assert_eq!(validate.channel, EventChannel::Validate);
        assert!(validate.parse.is_none());
        assert!(validate.validate.is_some());
        assert!(validate.transform.is_none());

        let transform = &captured[2];
        assert_eq!(transform.channel, EventChannel::Transform);
        assert!(transform.parse.is_none());
        assert!(transform.validate.is_none());
        assert!(transform.transform.is_some());
    }

    #[test]
    fn events_round_trip_through_serde_json() {
        let originals = vec![sample_parse(0), sample_validate(1), sample_transform(2)];
        for original in originals {
            let json = serde_json::to_value(&original).unwrap();
            // Channel must serialize as a lowercase string per AC-O-1.
            assert!(matches!(
                json.get("channel").and_then(|v| v.as_str()),
                Some("parse") | Some("validate") | Some("transform")
            ));
            let round: ReportEvent = serde_json::from_value(json).unwrap();
            assert_eq!(round.sequence, original.sequence);
            assert_eq!(round.channel, original.channel);
        }
    }

    #[test]
    fn serde_wire_form_keeps_nullable_parse_fields_and_transform_objects() {
        let parse_json = serde_json::to_value(ReportEvent {
            sequence: 0,
            channel: EventChannel::Parse,
            byte_offset: Some(1),
            source_map: None,
            parse: Some(ParseReportEvent {
                kind: ParseEventKind::Trivia,
                name: None,
                value: None,
            }),
            validate: None,
            transform: None,
        })
        .unwrap();
        assert!(parse_json
            .pointer("/parse/name")
            .is_some_and(|v| v.is_null()));
        assert!(parse_json
            .pointer("/parse/value")
            .is_some_and(|v| v.is_null()));

        let transform_json = serde_json::to_value(ReportEvent {
            sequence: 1,
            channel: EventChannel::Transform,
            byte_offset: None,
            source_map: None,
            parse: None,
            validate: None,
            transform: Some(TransformReportEvent {
                transform: TransformKind::SchemaValidation { schema_id: 42 },
                summary: "validated".to_owned(),
            }),
        })
        .unwrap();
        assert_eq!(
            transform_json.pointer("/transform/transform/kind"),
            Some(&serde_json::Value::String("SchemaValidation".to_owned()))
        );
        assert_eq!(
            transform_json
                .pointer("/transform/transform/schema_id")
                .and_then(|v| v.as_u64()),
            Some(42)
        );
    }

    #[test]
    fn events_to_jsonl_emits_one_line_per_event() {
        let events = vec![sample_parse(0), sample_validate(1), sample_transform(2)];
        let text = events_to_jsonl(&events);
        assert_eq!(text.lines().count(), 3);
        for line in text.lines() {
            let _: ReportEvent = serde_json::from_str(line).unwrap();
        }
    }
}
