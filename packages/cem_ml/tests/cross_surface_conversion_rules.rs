//! Phase 12 cross-surface conversion rule fixtures.
//!
//! These fixtures cover the exact construct list in
//! `docs/cem-ml-cli-plan.md` Phase 12 §6 and
//! `packages/cem_ml/docs/cross-surface-conversion.md`: namespace
//! bindings, default namespace changes, comments/whitespace,
//! doctypes/PIs/CDATA, anonymous typed scopes, rich content,
//! `$` expressions, attribute-value cem-ql spans, and source-map frames.

use cem_ml::engine::InputFormat;
use cem_ml::events::cem::CemEventNormalizer;
use cem_ml::events::{EventNormalizer, NormalizedEvent, TriviaKind};
use cem_ml::formatter;
use cem_ml::interpreter::light_dom::render_html;
use cem_ml::parser::builder::CemAstBuilder;
use cem_ml::parser::document::CemDocument;
use cem_ml::parser::CemAstNode;
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::source_map::TransformKind;
use cem_ml::tokenizer::cem::CemTokenizer;
use cem_ml::tokenizer::html::HtmlTokenizer;
use cem_ml::tokenizer::xml::XmlTokenizer;

fn fixture(name: &str) -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/cem-ml/cross-surface")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"))
}

fn parse_as(input: &str, format: InputFormat) -> CemDocument {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    match format {
        InputFormat::Cem => {
            CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(src))).build()
        }
        InputFormat::Html => {
            CemAstBuilder::new(CemEventNormalizer::new(HtmlTokenizer::from_source(src))).build()
        }
        InputFormat::Xml => {
            CemAstBuilder::new(CemEventNormalizer::new(XmlTokenizer::from_source(src))).build()
        }
    }
}

fn events_as(input: &str, format: InputFormat) -> Vec<NormalizedEvent> {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let mut events = Vec::new();
    match format {
        InputFormat::Cem => {
            let mut n = CemEventNormalizer::new(CemTokenizer::from_source(src));
            while let Some(event) = n.next_event() {
                events.push(event);
            }
        }
        InputFormat::Html => {
            let mut n = CemEventNormalizer::new(HtmlTokenizer::from_source(src));
            while let Some(event) = n.next_event() {
                events.push(event);
            }
        }
        InputFormat::Xml => {
            let mut n = CemEventNormalizer::new(XmlTokenizer::from_source(src));
            while let Some(event) = n.next_event() {
                events.push(event);
            }
        }
    }
    events
}

fn element_names(doc: &CemDocument) -> Vec<String> {
    doc.iter()
        .filter_map(|node| match node {
            CemAstNode::Element { expanded_name, .. } => Some(expanded_name.local_name.clone()),
            _ => None,
        })
        .collect()
}

fn text_values(doc: &CemDocument) -> Vec<String> {
    doc.iter()
        .filter_map(|node| match node {
            CemAstNode::Text { data, .. } => Some(data.clone()),
            _ => None,
        })
        .collect()
}

fn comment_values(doc: &CemDocument) -> Vec<String> {
    doc.iter()
        .filter_map(|node| match node {
            CemAstNode::Comment { data, .. } => Some(data.clone()),
            _ => None,
        })
        .collect()
}

fn pi_targets(doc: &CemDocument) -> Vec<String> {
    doc.iter()
        .filter_map(|node| match node {
            CemAstNode::ProcessingInstruction { target, .. } => Some(target.clone()),
            _ => None,
        })
        .collect()
}

fn has_attr_value(doc: &CemDocument, expected: &str) -> bool {
    doc.iter().any(|node| {
        matches!(
            node,
            CemAstNode::Attribute { value: Some(value), .. } if value == expected
        )
    })
}

fn has_transform(
    out: &cem_ml::interpreter::TransformOutput,
    predicate: impl Fn(&TransformKind) -> bool,
) -> bool {
    out.output_spans.iter().any(|span| {
        span.origin
            .frames
            .iter()
            .any(|frame| predicate(&frame.transform))
    })
}

#[test]
fn cem_fixture_preserves_anonymous_scope_expression_avt_comments_and_rich_content() {
    let input = fixture("conversion-rules.cem");
    let doc = parse_as(&input, InputFormat::Cem);

    let names = element_names(&doc);
    assert!(
        names.iter().any(|name| name.is_empty()),
        "anonymous scope missing: {names:?}"
    );
    assert!(
        names.iter().any(|name| name == "$"),
        "$ expression node missing: {names:?}"
    );
    assert!(comment_values(&doc)
        .iter()
        .any(|c| c.contains("conversion fixture")));
    assert!(text_values(&doc)
        .iter()
        .any(|text| text.contains(r#"{"name":"{$.user}","ok":true}"#)));
    assert!(has_attr_value(&doc, "{.busy}"));
    assert!(has_attr_value(&doc, "Hello {.name}"));

    let events = events_as(&input, InputFormat::Cem);
    assert!(events.iter().any(|event| matches!(
        event,
        NormalizedEvent::ModeSwitch { content_type, .. } if content_type == "application/json"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        NormalizedEvent::Trivia {
            kind: TriviaKind::Comment,
            data,
            ..
        } if data.contains("conversion fixture")
    )));
}

#[test]
fn xml_fixture_preserves_doctype_pi_cdata_namespace_rebinding_and_parity_wrappers() {
    let input = fixture("conversion-rules.xml");
    let doc = parse_as(&input, InputFormat::Xml);

    let targets = pi_targets(&doc);
    assert!(
        targets.iter().any(|target| target == "xml"),
        "xml PI missing: {targets:?}"
    );
    assert!(
        targets.iter().any(|target| target == "DOCTYPE"),
        "DOCTYPE PI missing: {targets:?}"
    );

    let names = element_names(&doc);
    assert!(
        names.iter().any(|name| name.is_empty()),
        "cem:scope did not lower to anonymous scope: {names:?}"
    );
    assert!(
        names.iter().any(|name| name == "$"),
        "cem:expr did not lower to $: {names:?}"
    );
    assert!(comment_values(&doc)
        .iter()
        .any(|c| c.contains("conversion fixture")));
    assert!(text_values(&doc)
        .iter()
        .any(|text| text.contains(r#"{"name":"{$.user}","ok":true}"#)));
    assert!(has_attr_value(&doc, "http://www.w3.org/2000/svg"));
    assert!(has_attr_value(&doc, "{.busy}"));
}

#[test]
fn html_fixture_preserves_doctype_comments_parity_wrappers_and_attribute_spans() {
    let input = fixture("conversion-rules.html");
    let doc = parse_as(&input, InputFormat::Html);

    let targets = pi_targets(&doc);
    assert!(
        targets.iter().any(|target| target == "DOCTYPE"),
        "DOCTYPE PI missing: {targets:?}"
    );

    let names = element_names(&doc);
    assert!(
        names.iter().any(|name| name.is_empty()),
        "cem:scope did not lower to anonymous scope: {names:?}"
    );
    assert!(
        names.iter().any(|name| name == "$"),
        "cem:expr did not lower to $: {names:?}"
    );
    assert!(comment_values(&doc)
        .iter()
        .any(|c| c.contains("conversion fixture")));
    assert!(has_attr_value(&doc, "{.busy}"));
    assert!(has_attr_value(&doc, "Hello {.name}"));
}

#[test]
fn reverse_conversion_outputs_are_byte_stable_and_source_mapped() {
    for (name, format, tokenizer, content_type) in [
        (
            "conversion-rules.cem",
            InputFormat::Cem,
            TransformKind::CemTokenizer,
            "application/cem",
        ),
        (
            "conversion-rules.xml",
            InputFormat::Xml,
            TransformKind::XmlTokenizer,
            "application/xml",
        ),
        (
            "conversion-rules.html",
            InputFormat::Html,
            TransformKind::HtmlTokenizer,
            "text/html",
        ),
    ] {
        let input = fixture(name);
        let once = formatter::format_source_as(&input, format);
        let twice = formatter::format_source_as(&once.rendered, InputFormat::Cem);
        assert_eq!(
            once.rendered, twice.rendered,
            "not byte-stable after canonicalization: {name}"
        );
        assert!(
            has_transform(&once, |t| *t == tokenizer),
            "{name} missing tokenizer frame"
        );
        assert!(
            has_transform(&once, |t| {
                matches!(t, TransformKind::ContentTypeTransform { content_type: actual } if actual == content_type)
            }),
            "{name} missing content-type transform frame"
        );
    }
}

#[test]
fn cem_to_light_dom_projection_preserves_source_map_frames() {
    let input = fixture("conversion-rules.cem");
    let out = render_html(&input);
    assert!(out.rendered.contains("<main"));
    assert!(out.output_spans.iter().all(|span| {
        span.origin
            .frames
            .iter()
            .any(|frame| matches!(frame.transform, TransformKind::InterpreterRender))
    }));
}
