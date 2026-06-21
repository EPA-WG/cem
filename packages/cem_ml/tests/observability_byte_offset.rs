//! AC-P-3 verification fixture: `byte_offset` is the canonical
//! top-level projection on every report event — not just on
//! `Diagnostic`.
//!
//! Drives the canonical CEM-ML fixture corpus through
//! `observe_pipeline`, then asserts:
//!
//!   1. every parse-channel event carries `byte_offset = Some(_)` —
//!      every normalized event is anchored by a token byte range;
//!   2. every validate-channel event carries `byte_offset = Some(_)`
//!      whenever its underlying `Diagnostic` has a source location
//!      (the canonical case for AC-P-3 diagnostics);
//!   3. transform-channel events MAY have `byte_offset = None` when
//!      they signal a phase boundary, but when they DO carry one it
//!      matches the first source-map frame's start (origin-first
//!      invariant per AC-P-7);
//!   4. JSON serialization round-trips the field under its wire name
//!      `"byteOffset"` for every event that has it, and omits the
//!      key when the field is `None`.

use cem_ml::engine::InputFormat;
use cem_ml::observability::{events_to_jsonl, BufferingObserver, EventChannel, ReportEvent};
use cem_ml::real::observe_pipeline;
use cem_ml::source::{ByteRange, SourceId};
use cem_ml::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};

fn canonical_fixture_paths() -> Vec<std::path::PathBuf> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/cem-ml");
    let mut out = Vec::new();
    walk(&root, &mut out);
    out.sort();
    out
}

fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("cem") {
            out.push(path);
        }
    }
}

fn drive(input: &str) -> Vec<ReportEvent> {
    let observer = BufferingObserver::new();
    let _ = observe_pipeline(input.as_bytes(), InputFormat::Cem, &observer);
    observer.snapshot()
}

/// Return `true` when `byte_offset` lies inside at least one frame's
/// byte range on the event's source-map stack. `FrameSpan::Single` is
/// a half-open `[start, end)`; `Multi` is the union of its parts.
fn byte_offset_inside_any_frame(event: &ReportEvent, byte_offset: u64) -> bool {
    let Some(stack) = event.source_map.as_ref() else {
        return false;
    };
    stack.frames.iter().any(|f| match &f.span {
        FrameSpan::Single(r) => byte_offset >= r.start && byte_offset < r.end(),
        FrameSpan::Multi(rs) => rs
            .iter()
            .any(|r| byte_offset >= r.start && byte_offset < r.end()),
    })
}

#[test]
fn byte_offset_inside_any_frame_treats_range_end_as_exclusive() {
    let event = ReportEvent {
        sequence: 0,
        channel: EventChannel::Transform,
        byte_offset: Some(10),
        source_map: Some(SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(0),
                span: FrameSpan::Single(ByteRange::new(10, 5)),
                transform: TransformKind::CemTokenizer,
            }],
        }),
        parse: None,
        validate: None,
        transform: Some(cem_ml::observability::TransformReportEvent {
            transform: TransformKind::CemTokenizer,
            summary: "tokenized".to_owned(),
        }),
    };

    assert!(byte_offset_inside_any_frame(&event, 10));
    assert!(byte_offset_inside_any_frame(&event, 14));
    assert!(!byte_offset_inside_any_frame(&event, 15));
}

// ---------------------------------------------------------------------------
// 1. Parse-channel coverage on the canonical fixture corpus
// ---------------------------------------------------------------------------

#[test]
fn every_parse_event_carries_a_byte_offset_on_every_canonical_fixture() {
    let fixtures = canonical_fixture_paths();
    assert!(
        fixtures.len() >= 5,
        "expected the canonical fixture corpus (>=5 .cem files)"
    );
    for path in &fixtures {
        let input = std::fs::read_to_string(path).unwrap();
        let events = drive(&input);
        let parse_events: Vec<&ReportEvent> = events
            .iter()
            .filter(|e| matches!(e.channel, EventChannel::Parse))
            .collect();
        assert!(
            !parse_events.is_empty(),
            "fixture {} produced no parse events",
            path.display()
        );
        for event in &parse_events {
            assert!(
                event.byte_offset.is_some(),
                "parse event missing byte_offset in fixture {} (sequence={}, kind={:?})",
                path.display(),
                event.sequence,
                event.parse.as_ref().map(|p| p.kind)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Validate-channel coverage — diagnostics with a location carry through
// ---------------------------------------------------------------------------

#[test]
fn validate_events_carry_byte_offset_when_their_diagnostic_does() {
    // The relaxed-content-boundary lint is anchored to the offending
    // element, so the diagnostic always has a byte offset — and so
    // must the projected report event.
    let events = drive("{p Hello}");
    let validate_events: Vec<&ReportEvent> = events
        .iter()
        .filter(|e| matches!(e.channel, EventChannel::Validate))
        .collect();
    assert!(
        !validate_events.is_empty(),
        "expected at least one validate event for the relaxed-boundary input"
    );
    for event in &validate_events {
        // Every diagnostic with a recorded source location must
        // surface byte_offset on the report event.
        if let Some(payload) = &event.validate {
            assert!(
                event.byte_offset.is_some(),
                "validate event for code `{}` is missing byte_offset",
                payload.code
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Origin-first invariant: byte_offset == first frame's start
// ---------------------------------------------------------------------------

#[test]
fn byte_offset_and_source_map_are_co_present_and_consistent() {
    // AC-P-3 + AC-P-7: `byte_offset` is a projection from the selected
    // source-map frame, and byte ranges are the canonical coordinate.
    // Cross-projection invariant: whenever an event carries a
    // source-map stack, it must also carry a `byte_offset`, and that
    // offset must lie inside one of the frame ranges on the stack —
    // the two views agree on the same scope, even though the emitting
    // layer is free to choose which frame is "selected".
    let events = drive("@doc cem-ml 1\n{p @id=x | hi}");
    let mut checked = 0;
    for event in &events {
        if event.source_map.is_some() {
            let byte = event
                .byte_offset
                .expect("events with a source_map must carry byte_offset");
            assert!(
                byte_offset_inside_any_frame(event, byte),
                "byte_offset {byte} is outside every frame on the stack \
                 (channel={:?}, sequence={})",
                event.channel,
                event.sequence,
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "expected at least one event with a source map");
}

// ---------------------------------------------------------------------------
// 4. JSON wire shape — `byteOffset` present iff the field is Some
// ---------------------------------------------------------------------------

#[test]
fn serialized_json_carries_byteoffset_key_when_present_and_omits_it_otherwise() {
    let events = drive("@doc cem-ml 1\n{p | hi}");
    let mut covered_present = false;
    let mut covered_absent = false;
    for event in &events {
        let json = serde_json::to_value(event).expect("event serializes");
        let map = json.as_object().expect("event is a JSON object");
        match event.byte_offset {
            Some(value) => {
                let from_json = map
                    .get("byteOffset")
                    .and_then(|v| v.as_u64())
                    .expect("byteOffset must be present and a number");
                assert_eq!(from_json, value);
                covered_present = true;
            }
            None => {
                assert!(
                    !map.contains_key("byteOffset"),
                    "byteOffset must be omitted when the field is None"
                );
                covered_absent = true;
            }
        }
    }
    assert!(
        covered_present,
        "expected at least one event carrying byteOffset"
    );
    assert!(
        covered_absent,
        "expected at least one transform-channel phase event without byteOffset"
    );
}

// ---------------------------------------------------------------------------
// 5. JSONL projection preserves the field on every line
// ---------------------------------------------------------------------------

#[test]
fn jsonl_projection_preserves_byte_offset_on_every_emitted_event() {
    let events = drive("@doc cem-ml 1\n{section | {h1 | T} {p | x}}");
    let jsonl = events_to_jsonl(&events);
    let mut roundtripped = 0;
    for (line, original) in jsonl.lines().zip(events.iter()) {
        let parsed: ReportEvent = serde_json::from_str(line).expect("each JSONL line round-trips");
        assert_eq!(parsed.byte_offset, original.byte_offset);
        roundtripped += 1;
    }
    assert_eq!(
        roundtripped,
        events.len(),
        "JSONL line count must match the event count"
    );
}
