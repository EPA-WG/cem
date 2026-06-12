//! End-to-end pipeline coverage for the Tier A canonical fixtures.
//!
//! For each `examples/cem-ml/*.cem` fixture this test drives every layer
//! independently and asserts they all complete without hard violations:
//!
//! 1. `BytesSource` → `Utf8Decoder`: bytes decode into scalars with no
//!    `cem.byte.*` errors.
//! 2. `CemTokenizer`: every fixture tokenizes with no
//!    `cem.tokenizer.*` errors.
//! 3. `CemEventNormalizer`: at least one `OpenScope`, one `CloseScope`,
//!    and balanced depth.
//! 4. `CemSchemaMachine`: zero schema-layer hard violations.
//! 5. `CemAstBuilder`: produces a non-empty `CemDocument` with a
//!    `Document` root node.
//! 6. `validation::run`: end-to-end Tier A validation reports zero hard
//!    violations.
//! 7. `LightDomInterpreter`: renders a non-empty HTML string, every
//!    output byte covered by at least one source-map span, and every
//!    span's origin walks back to the originating tokenizer/AST frame.

use cem_ml::diagnostics::Severity;
use cem_ml::events::{cem::CemEventNormalizer, EventNormalizer, NormalizedEvent};
use cem_ml::interpreter::light_dom::{render_html, LightDomInterpreter};
use cem_ml::parser::builder::CemAstBuilder;
use cem_ml::parser::CemAstNode;
use cem_ml::schema::machine::CemSchemaMachine;
use cem_ml::schema::vocab::CompiledSchema;
use cem_ml::source::decode::{DecodeConfig, Utf8Decoder};
use cem_ml::source::{BytesSource, EncodingDecoder, SourceId};
use cem_ml::source_map::TransformKind;
use cem_ml::tokenizer::cem::CemTokenizer;
use cem_ml::tokenizer::{SchemaToken, SchemaTokenizer};
use cem_ml::validation;

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

#[test]
fn every_canonical_fixture_runs_through_every_layer() {
    let paths = fixtures();
    assert!(paths.len() >= 5, "expected >= 5 canonical fixtures");

    for path in &paths {
        let fixture_name = path.file_name().unwrap().to_string_lossy().into_owned();
        let bytes = std::fs::read(path).unwrap();

        // 1. Decoder.
        {
            let src = BytesSource::new(SourceId(1), bytes.clone());
            let mut d = Utf8Decoder::with_config(src, DecodeConfig::default());
            while d.decode_next().is_some() {}
            let diags = d.take_diagnostics();
            let hard: Vec<_> = diags
                .iter()
                .filter(|d| {
                    matches!(d.severity, Severity::Error | Severity::Fatal)
                        && d.code.starts_with("cem.byte.")
                })
                .collect();
            assert!(
                hard.is_empty(),
                "[{fixture_name}] decoder hard diags: {hard:?}"
            );
        }

        // 2. Tokenizer.
        let mut tokens: Vec<SchemaToken> = Vec::new();
        {
            let src = BytesSource::new(SourceId(1), bytes.clone());
            let mut tok = CemTokenizer::from_source(src);
            while let Some(t) = tok.next_token() {
                tokens.push(t);
            }
            let diags = tok.take_diagnostics();
            let hard: Vec<_> = diags
                .iter()
                .filter(|d| {
                    matches!(d.severity, Severity::Error | Severity::Fatal)
                        && d.code.starts_with("cem.tokenizer.")
                })
                .collect();
            assert!(
                hard.is_empty(),
                "[{fixture_name}] tokenizer hard diags: {hard:?}"
            );
            assert!(
                !tokens.is_empty(),
                "[{fixture_name}] tokenizer emitted no tokens"
            );
        }

        // 3. Event normalizer: open/close balance.
        let mut open_count = 0usize;
        let mut close_count = 0usize;
        {
            let src = BytesSource::new(SourceId(1), bytes.clone());
            let tok = CemTokenizer::from_source(src);
            let mut n = CemEventNormalizer::new(tok);
            while let Some(ev) = n.next_event() {
                match ev {
                    NormalizedEvent::OpenScope { .. } => open_count += 1,
                    NormalizedEvent::CloseScope { .. } => close_count += 1,
                    _ => {}
                }
            }
            assert_eq!(
                open_count, close_count,
                "[{fixture_name}] unbalanced open/close: {open_count} vs {close_count}"
            );
            assert!(open_count > 0, "[{fixture_name}] no OpenScope events");
        }

        // 4. Schema machine.
        {
            let src = BytesSource::new(SourceId(1), bytes.clone());
            let tok = CemTokenizer::from_source(src);
            let n = CemEventNormalizer::new(tok);
            let outcome = CemSchemaMachine::new(CompiledSchema::cem_core(), n).run();
            let hard: Vec<_> = outcome
                .diagnostics
                .iter()
                .filter(|d| {
                    matches!(d.severity, Severity::Error | Severity::Fatal)
                        && (d.code.starts_with("cem.schema.") || d.code.starts_with("cem.handoff."))
                })
                .collect();
            assert!(
                hard.is_empty(),
                "[{fixture_name}] schema layer hard diags: {hard:?}"
            );
        }

        // 5. AST builder.
        let document = {
            let src = BytesSource::new(SourceId(1), bytes.clone());
            let tok = CemTokenizer::from_source(src);
            let n = CemEventNormalizer::new(tok);
            CemAstBuilder::new(n).build()
        };
        assert!(!document.nodes.is_empty(), "[{fixture_name}] AST is empty");
        assert!(
            matches!(document.nodes[0], CemAstNode::Document { .. }),
            "[{fixture_name}] node[0] is not Document"
        );

        // 6. End-to-end validation.
        let input = std::str::from_utf8(&bytes).unwrap();
        let report = validation::run(input);
        let hard = report.hard_violations();
        assert_eq!(
            hard,
            0,
            "[{fixture_name}] validation::run produced {hard} hard violation(s): {:?}",
            report
                .diagnostics
                .iter()
                .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
                .collect::<Vec<_>>()
        );

        // 7. Transform + per-byte source-map coverage.
        let output = LightDomInterpreter::new().render(&document);
        assert!(
            !output.rendered.is_empty(),
            "[{fixture_name}] rendered output is empty"
        );
        let mut covered = vec![false; output.rendered.len()];
        for span in &output.output_spans {
            for i in span.output_range.start as usize
                ..(span.output_range.start + span.output_range.len as u64) as usize
            {
                if i < covered.len() {
                    covered[i] = true;
                }
            }
            assert!(
                !span.origin.frames.is_empty(),
                "[{fixture_name}] output span has empty source-map stack"
            );
        }
        let uncovered: Vec<usize> = covered
            .iter()
            .enumerate()
            .filter_map(|(i, c)| (!*c).then_some(i))
            .collect();
        assert!(
            uncovered.is_empty(),
            "[{fixture_name}] {} output bytes not covered by any source-map span",
            uncovered.len()
        );
    }
}

#[test]
fn every_output_span_traces_to_source_or_to_a_transform_frame() {
    // Item 3 of `docs/todo.md` §Verification: every generated node traces
    // back to original source bytes OR to the transform that generated it.
    // The light-DOM renderer emits an `InterpreterRender` frame on top of
    // the originating AST node's stack, which itself was rooted by the
    // tokenizer's `CemTokenizer` frame. Walk every span and confirm the
    // origin chain reaches either a tokenizer frame (real source bytes)
    // or a CemAstBuilder/InterpreterRender frame (transform-generated).
    for path in fixtures() {
        let fixture_name = path.file_name().unwrap().to_string_lossy().into_owned();
        let input = std::fs::read_to_string(&path).unwrap();
        let output = render_html(&input);
        for span in &output.output_spans {
            let traces_to_source_or_transform = span.origin.frames.iter().any(|f| {
                matches!(
                    f.transform,
                    TransformKind::CemTokenizer
                        | TransformKind::HtmlTokenizer
                        | TransformKind::XmlTokenizer
                        | TransformKind::EventNormalizer
                        | TransformKind::CemAstBuilder
                        | TransformKind::InterpreterRender
                )
            });
            assert!(
                traces_to_source_or_transform,
                "[{fixture_name}] output span has no recognized origin frame: {:?}",
                span.origin
            );
        }
    }
}

#[test]
fn pipeline_is_deterministic_across_re_runs() {
    // Determinism check across the full pipeline: same input → same
    // rendered output + identical diagnostic codes in identical order.
    for path in fixtures() {
        let input = std::fs::read_to_string(&path).unwrap();
        let a = render_html(&input);
        let b = render_html(&input);
        assert_eq!(
            a.rendered, b.rendered,
            "non-deterministic render for {path:?}"
        );
        let codes_a: Vec<&str> = a.diagnostics.iter().map(|d| d.code.as_str()).collect();
        let codes_b: Vec<&str> = b.diagnostics.iter().map(|d| d.code.as_str()).collect();
        assert_eq!(
            codes_a, codes_b,
            "non-deterministic diagnostic stream for {path:?}"
        );
    }
}
