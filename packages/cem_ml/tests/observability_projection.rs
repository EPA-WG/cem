//! Observability API projection tests (AC-O-1, AC-O-3).
//!
//! Asserts that `observe_pipeline` emits a stable event stream across
//! every canonical CEM-ML fixture:
//!
//! - the channel vocabulary is exactly `parse` / `validate` / `transform`,
//! - every parse-channel event carries a [`ParseEventKind`] drawn from
//!   the stable enum,
//! - sequence numbers are monotonic and dense (`0..n`),
//! - parse-channel scope opens and closes are balanced,
//! - every event survives a `serde_json` round-trip and a JSONL
//!   projection round-trip.

use cem_ml::engine::InputFormat;
use cem_ml::observability::{
    events_to_jsonl, BufferingObserver, EventChannel, ParseEventKind, ReportEvent,
};
use cem_ml::real::observe_pipeline;

fn canonical_fixture_paths() -> Vec<std::path::PathBuf> {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/cem-ml");
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

#[test]
fn every_fixture_emits_a_well_formed_event_stream() {
    let fixtures = canonical_fixture_paths();
    assert!(
        fixtures.len() >= 5,
        "expected the canonical fixture corpus"
    );
    for path in &fixtures {
        let input = std::fs::read_to_string(path).unwrap();
        let observer = BufferingObserver::new();
        let _ = observe_pipeline(input.as_bytes(), InputFormat::Cem, &observer);
        let events = observer.snapshot();
        assert!(
            !events.is_empty(),
            "fixture {} produced no events",
            path.display()
        );

        // Sequence numbers form 0..events.len() exactly.
        for (idx, ev) in events.iter().enumerate() {
            assert_eq!(
                ev.sequence as usize, idx,
                "sequence gap in fixture {}",
                path.display()
            );
        }

        // Channel vocabulary is exactly the stable set.
        for ev in &events {
            match ev.channel {
                EventChannel::Parse => {
                    assert!(
                        ev.parse.is_some(),
                        "parse-channel event must carry a parse payload in {}",
                        path.display()
                    );
                }
                EventChannel::Validate => {
                    assert!(
                        ev.validate.is_some(),
                        "validate-channel event must carry a validate payload in {}",
                        path.display()
                    );
                }
                EventChannel::Transform => {
                    assert!(
                        ev.transform.is_some(),
                        "transform-channel event must carry a transform payload in {}",
                        path.display()
                    );
                }
            }
        }

        // Parse-channel scope opens and closes balance.
        let opens = events
            .iter()
            .filter(|e| matches!(e.parse.as_ref().map(|p| p.kind), Some(ParseEventKind::OpenScope)))
            .count();
        let closes = events
            .iter()
            .filter(|e| matches!(e.parse.as_ref().map(|p| p.kind), Some(ParseEventKind::CloseScope)))
            .count();
        assert_eq!(
            opens,
            closes,
            "OpenScope / CloseScope counts must balance in {}",
            path.display()
        );

        // At least one transform event (tokenizer/normalizer/AST builder
        // are always emitted) and the channel summary covers parse,
        // transform — validate is fixture-dependent.
        let channels: std::collections::HashSet<EventChannel> =
            events.iter().map(|e| e.channel).collect();
        assert!(
            channels.contains(&EventChannel::Parse),
            "parse channel missing in {}",
            path.display()
        );
        assert!(
            channels.contains(&EventChannel::Transform),
            "transform channel missing in {}",
            path.display()
        );

        // Round-trip every event through serde to confirm the payload
        // shape is wire-stable.
        for ev in &events {
            let json = serde_json::to_value(ev).unwrap_or_else(|e| {
                panic!("event serialization failed in {}: {e}", path.display())
            });
            let _: ReportEvent = serde_json::from_value(json).unwrap_or_else(|e| {
                panic!("event deserialization failed in {}: {e}", path.display())
            });
        }

        // JSONL projection: exactly one line per event, every line
        // parses back into a ReportEvent.
        let jsonl = events_to_jsonl(&events);
        let line_count = jsonl.lines().count();
        assert_eq!(
            line_count,
            events.len(),
            "JSONL line count must match event count in {}",
            path.display()
        );
        for line in jsonl.lines() {
            let _: ReportEvent = serde_json::from_str(line).unwrap_or_else(|e| {
                panic!("JSONL line failed to round-trip in {}: {e}", path.display())
            });
        }
    }
}

#[test]
fn validate_channel_carries_diagnostic_codes() {
    // The relaxed-boundary fixture (handcrafted here) is guaranteed to
    // emit a `cem.lint.relaxed_content_boundary` diagnostic, which
    // routes through the validate channel.
    let input = "{p Hello}";
    let observer = BufferingObserver::new();
    let _ = observe_pipeline(input.as_bytes(), InputFormat::Cem, &observer);
    let codes: Vec<String> = observer
        .snapshot()
        .into_iter()
        .filter(|e| matches!(e.channel, EventChannel::Validate))
        .filter_map(|e| e.validate.map(|v| v.code))
        .collect();
    assert!(
        codes.iter().any(|c| c == "cem.lint.relaxed_content_boundary"),
        "expected the relaxed-boundary lint on the validate channel, got {codes:?}"
    );
}

#[test]
fn parse_channel_kinds_are_drawn_from_the_stable_set() {
    let input = "@doc cem-ml 1\n{p @id=x | hi}";
    let observer = BufferingObserver::new();
    let _ = observe_pipeline(input.as_bytes(), InputFormat::Cem, &observer);
    let kinds: std::collections::HashSet<ParseEventKind> = observer
        .snapshot()
        .into_iter()
        .filter_map(|e| e.parse.map(|p| p.kind))
        .collect();
    // We expect at least OpenScope, CloseScope, Name, Value, Separator
    // for this fixture; new kinds are additive only.
    for k in [
        ParseEventKind::OpenScope,
        ParseEventKind::CloseScope,
        ParseEventKind::Name,
        ParseEventKind::Value,
        ParseEventKind::Separator,
    ] {
        assert!(kinds.contains(&k), "missing ParseEventKind::{k:?}");
    }
}
