//! Reverse cross-surface conversion coverage.
//!
//! HTML and XML parity inputs lower through their tokenizer profiles,
//! build the shared AST, and serialize to canonical CEM-ML with a
//! `ContentTypeTransform` source-map boundary.

use cem_ml::engine::InputFormat;
use cem_ml::formatter;
use cem_ml::source_map::TransformKind;

fn has_transform(
    out: &cem_ml::interpreter::TransformOutput,
    expected: fn(&TransformKind) -> bool,
) -> bool {
    out.output_spans.iter().any(|span| {
        span.origin
            .frames
            .iter()
            .any(|frame| expected(&frame.transform))
    })
}

#[test]
fn every_html_parity_fixture_serializes_to_canonical_cem_ml() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/semantic");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("html") {
            continue;
        }
        let input = std::fs::read_to_string(&path).unwrap();
        let once = formatter::format_source_as(&input, InputFormat::Html);
        let twice = formatter::format_source_as(&once.rendered, InputFormat::Cem);
        assert_eq!(
            once.rendered,
            twice.rendered,
            "HTML reverse conversion is not byte-stable for `{}`",
            path.display()
        );
        assert!(once.rendered.contains("{main") || once.rendered.contains("{html"));
        assert!(has_transform(&once, |t| matches!(
            t,
            TransformKind::HtmlTokenizer
        )));
        assert!(has_transform(&once, |t| {
            matches!(t, TransformKind::ContentTypeTransform { content_type } if content_type == "text/html")
        }));
        checked += 1;
    }
    assert!(checked >= 5);
}

#[test]
fn namespace_rebinding_xml_fixture_serializes_to_canonical_cem_ml() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/cem-ml/namespace-rebinding/default-html-svg-html.xml");
    let input = std::fs::read_to_string(&path).unwrap();
    let once = formatter::format_source_as(&input, InputFormat::Xml);
    let twice = formatter::format_source_as(&once.rendered, InputFormat::Cem);
    assert_eq!(once.rendered, twice.rendered);
    assert!(once.rendered.contains("{main"));
    assert!(once.rendered.contains("{svg"));
    assert!(once
        .rendered
        .contains("@xmlns=http://www.w3.org/1999/xhtml"));
    assert!(once.rendered.contains("@xmlns=http://www.w3.org/2000/svg"));
    assert!(has_transform(&once, |t| matches!(
        t,
        TransformKind::XmlTokenizer
    )));
    assert!(has_transform(&once, |t| {
        matches!(t, TransformKind::ContentTypeTransform { content_type } if content_type == "application/xml")
    }));
}
