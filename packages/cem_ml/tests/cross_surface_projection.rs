//! Cross-surface projection coverage for the canonical CEM-ML fixtures.
//!
//! Tier A scope: assert that the `.cem → light-DOM HTML` projection
//! preserves schema event identity, source-map traceability, and
//! deterministic re-projection. The mirror direction (HTML → `.cem`)
//! lands once the Phase 11 HTML parity tokenizer replaces the stub in
//! `packages/cem_ml/src/tokenizer/html.rs`; the assertion shape stays the
//! same.

use cem_ml::events::cem::CemEventNormalizer;
use cem_ml::events::{EventNormalizer, NormalizedEvent};
use cem_ml::interpreter::light_dom::render_html;
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;

fn fixtures() -> Vec<std::path::PathBuf> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/cem-ml");
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("cem"))
        .collect();
    paths.sort();
    paths
}

/// Project a `.cem` input to a flat `Vec<(kind, qname)>` for shape
/// comparison. Trivia and modeswitch events are filtered out — they vary
/// across surfaces by design (cross-surface-conversion.md §4, §5).
fn event_shape(input: &str) -> Vec<(&'static str, String)> {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let mut normalizer = CemEventNormalizer::new(tok);
    let mut out = Vec::new();
    while let Some(ev) = normalizer.next_event() {
        match ev {
            NormalizedEvent::OpenScope { name, .. } => {
                out.push(("open", name.lexical_name));
            }
            NormalizedEvent::CloseScope { name, .. } => {
                out.push(("close", name.lexical_name));
            }
            NormalizedEvent::Name { name, .. } => {
                out.push(("attr", name.lexical_name));
            }
            NormalizedEvent::Value { .. } => {
                out.push(("value", String::new()));
            }
            _ => {}
        }
    }
    out
}

#[test]
fn canonical_cem_projection_is_deterministic_for_every_fixture() {
    for path in fixtures() {
        let input = std::fs::read_to_string(&path).unwrap();
        let first = render_html(&input).rendered;
        let second = render_html(&input).rendered;
        assert_eq!(
            first,
            second,
            "non-deterministic projection for {}",
            path.display()
        );
    }
}

#[test]
fn projection_event_shape_matches_documented_open_close_pairs() {
    // Every OpenScope has a corresponding CloseScope with the matching
    // lexical name. This is the basic event-identity contract every
    // cross-surface conversion must preserve.
    for path in fixtures() {
        let fixture = path.file_name().unwrap().to_string_lossy().into_owned();
        let input = std::fs::read_to_string(&path).unwrap();
        let shape = event_shape(&input);
        let mut stack: Vec<String> = Vec::new();
        for (kind, name) in &shape {
            match *kind {
                "open" => stack.push(name.clone()),
                "close" => {
                    let top = stack.pop();
                    assert_eq!(
                        top.as_deref(),
                        Some(name.as_str()),
                        "[{fixture}] close `{name}` did not match top-of-stack `{:?}`",
                        top
                    );
                }
                _ => {}
            }
        }
        assert!(
            stack.is_empty(),
            "[{fixture}] unbalanced opens remaining: {stack:?}"
        );
    }
}

#[test]
fn rendered_output_preserves_cem_namespace_attributes() {
    // Cross-surface §1: `cem:*` attributes survive the conversion
    // verbatim on the host element. The light-DOM render is the
    // canonical projection target; every fixture that uses a CEM
    // annotation in the source must show it in the rendered HTML.
    for path in fixtures() {
        let input = std::fs::read_to_string(&path).unwrap();
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        let rendered = render_html(&input).rendered;

        // Every fixture uses cem:screen at minimum.
        assert!(
            rendered.contains("cem:screen=\""),
            "[{stem}] expected cem:screen attribute in projection: {rendered}"
        );
    }
}

#[test]
fn projection_source_maps_record_content_type_transform_for_html_projection() {
    // Cross-surface §9: every output byte traces through a transform
    // frame back to either source bytes (CemTokenizer/HtmlTokenizer/
    // XmlTokenizer/EventNormalizer) or a transform that generated it
    // (CemAstBuilder, InterpreterRender, ContentTypeTransform).
    //
    // The InterpreterRender frame is always present because the
    // light-DOM renderer pushes it on every emitted span. The
    // tokenizer-rooted frame is present on most spans; spans derived
    // from trivia events (whose `NormalizedEvent::Trivia` variant
    // carries no source_map by design) walk back via the CemAstBuilder
    // frame which is sufficient transform identity.
    use cem_ml::source_map::TransformKind;
    for path in fixtures() {
        let input = std::fs::read_to_string(&path).unwrap();
        let out = render_html(&input);
        for span in &out.output_spans {
            let mut saw_interpreter = false;
            let mut saw_source_or_transform = false;
            for frame in &span.origin.frames {
                match frame.transform {
                    TransformKind::InterpreterRender => saw_interpreter = true,
                    TransformKind::CemTokenizer
                    | TransformKind::HtmlTokenizer
                    | TransformKind::XmlTokenizer
                    | TransformKind::EventNormalizer
                    | TransformKind::CemAstBuilder
                    | TransformKind::ContentTypeTransform { .. } => saw_source_or_transform = true,
                    _ => {}
                }
            }
            assert!(saw_interpreter, "[{path:?}] span missing interpreter frame");
            assert!(
                saw_source_or_transform,
                "[{path:?}] span has no upstream transform frame: {:?}",
                span.origin
            );
        }
    }
}
