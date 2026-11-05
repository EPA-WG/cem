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
use cem_ml::events::{EventNormalizer, NormalizedEvent, ScalarValue, TriviaKind};
use cem_ml::formatter;
use cem_ml::interpreter::light_dom::render_html;
use cem_ml::parser::builder::CemAstBuilder;
use cem_ml::parser::document::CemDocument;
use cem_ml::parser::CemAstNode;
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::source_map::{FrameSpan, TransformKind};
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

fn button_attribute<'a>(doc: &'a CemDocument, name: &str) -> &'a CemAstNode {
    let attributes = doc
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                expanded_name,
                attributes,
                ..
            } if expanded_name.local_name == "button" => Some(attributes),
            _ => None,
        })
        .expect("fixture button missing");
    let matching: Vec<_> = attributes
        .iter()
        .filter_map(|id| doc.get(*id))
        .filter(|node| matches!(node, CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name == name))
        .collect();
    assert_eq!(matching.len(), 1, "expected one button @{name}");
    matching[0]
}

fn assert_literal_button_attribute(doc: &CemDocument, name: &str, expected: &str) {
    let CemAstNode::Attribute {
        value, value_nodes, ..
    } = button_attribute(doc, name)
    else {
        unreachable!()
    };
    assert_eq!(value.as_deref(), Some(expected));
    assert!(
        value_nodes.is_empty(),
        "literal @{name} became a native slot"
    );
}

fn assert_native_disabled_expression(doc: &CemDocument, input: &str) {
    let CemAstNode::Attribute {
        value, value_nodes, ..
    } = button_attribute(doc, "disabled")
    else {
        unreachable!()
    };
    assert!(value.is_none(), "native expression became a literal value");
    let [expression_id] = value_nodes.as_slice() else {
        panic!("disabled must own exactly one native expression: {value_nodes:?}")
    };
    let Some(CemAstNode::Element {
        expanded_name,
        attributes,
        children,
        source,
        ..
    }) = doc.get(*expression_id)
    else {
        panic!("disabled lost its native expression node")
    };
    assert_eq!(expanded_name.local_name, "$");
    assert!(attributes.is_empty());
    let [body_id] = children.as_slice() else {
        panic!("disabled expression must own exactly one body")
    };
    assert!(matches!(doc.get(*body_id), Some(CemAstNode::Text { data, .. }) if data == ".busy"));
    assert!(doc.iter().all(|node| !matches!(node, CemAstNode::Element { children, .. } if children.contains(expression_id))),
        "attribute expression leaked into element content");
    let frame = source.current().expect("expression source frame missing");
    assert_eq!(frame.source_id, SourceId(1));
    assert!(matches!(
        frame.transform,
        TransformKind::ExpressionEmbedding { .. }
    ));
    let FrameSpan::Single(range) = frame.span else {
        panic!("expression source must retain its original byte range")
    };
    assert_eq!(&input[range.start as usize..range.end() as usize], ".busy");
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
    assert_native_disabled_expression(&doc, &input);
    assert_literal_button_attribute(&doc, "aria-label", "Hello {.name}");

    let events = events_as(&input, InputFormat::Cem);
    assert!(events.windows(2).any(|pair| matches!(pair,
        [NormalizedEvent::Name { name, .. }, NormalizedEvent::Value { value: ScalarValue::Expression(value), byte_range }]
            if name.lexical_name == "disabled" && value == "{.busy}"
                && &input[byte_range.start as usize..byte_range.end() as usize] == "{.busy}"
    )), "disabled event lost its exact braced expression");
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
fn quoted_cem_attribute_braces_remain_literal_through_canonical_conversion() {
    let input =
        fixture("conversion-rules.cem").replace("@disabled={.busy}", r#"@disabled="{.busy}""#);
    let doc = parse_as(&input, InputFormat::Cem);
    assert_literal_button_attribute(&doc, "disabled", "{.busy}");
    let formatted = formatter::format_source_as(&input, InputFormat::Cem);
    let reparsed = parse_as(&formatted.rendered, InputFormat::Cem);
    assert_literal_button_attribute(&reparsed, "disabled", "{.busy}");
    assert_literal_button_attribute(&reparsed, "aria-label", "Hello {.name}");
    assert_eq!(formatter::format(&reparsed), formatted.rendered);
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
    assert_literal_button_attribute(&doc, "disabled", "{.busy}");
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
    assert_literal_button_attribute(&doc, "disabled", "{.busy}");
    assert_literal_button_attribute(&doc, "aria-label", "Hello {.name}");
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
        let reparsed = parse_as(&once.rendered, InputFormat::Cem);
        if matches!(format, InputFormat::Cem) {
            assert_native_disabled_expression(&reparsed, &once.rendered);
        } else {
            assert_literal_button_attribute(&reparsed, "disabled", "{.busy}");
        }
        assert_literal_button_attribute(&reparsed, "aria-label", "Hello {.name}");
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
