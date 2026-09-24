//! Characterize current shared whitespace behavior before selecting a new policy.
use cem_ml::source::{ByteRange, BytesSource, SourceId};
use cem_ml::source_map::FrameSpan;
use cem_ml::tokenizer::cem::CemTokenizer;
use cem_ml::tokenizer::{SchemaTokenKind, SchemaTokenizer};
use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{
    compile_template, render_compiled_template, render_plan_to_html, render_template,
    CompileTemplateOptions, RenderPlanNode, TemplateData,
};

fn data(value: &str) -> TemplateData {
    TemplateData::default().with_binding(
        "text",
        ItemStream::once(Item::Atomic(AtomValue::String(value.into()))),
    )
}

#[test]
fn tokenizer_distinguishes_layout_trivia_from_text_and_rich_content() {
    let source = "{textarea |\n  {$text}\n  ``` \n ```tail \n }";
    let mut tokenizer =
        CemTokenizer::from_source(BytesSource::new(SourceId(1), source.as_bytes().to_vec()));
    let mut runs = Vec::new();
    while let Some(token) = tokenizer.next_token() {
        let (kind, value) = match token.kind {
            SchemaTokenKind::Trivia(value) => ("trivia", value),
            SchemaTokenKind::Text(value) => ("text", value),
            SchemaTokenKind::RichContent { data } => ("rich", data),
            _ => continue,
        };
        let authored = &source[token.byte_range.start as usize..token.byte_range.end() as usize];
        assert_eq!(
            authored,
            if kind == "rich" {
                format!("```{value}```")
            } else {
                value.clone()
            }
        );
        runs.push((kind, value));
    }
    assert!(tokenizer.take_diagnostics().is_empty());
    assert_eq!(
        runs,
        [
            ("trivia", "|"),
            ("trivia", "\n  "),
            ("trivia", "\n  "),
            ("rich", " \n "),
            ("text", "tail \n "),
        ]
        .map(|(kind, value)| (kind, value.to_owned()))
    );
}

#[test]
fn body_boundary_whitespace_is_asymmetric_across_element_names() {
    // This is current behavior, not approval of a new trimming policy.
    for tag in ["textarea", "pre", "p", "code"] {
        for (body, expected) in [
            ("\n  {$text}\n  ", "A\n  "),
            ("\r\n\t{$text}\r\n\t", "A\r\n\t"),
            ("  {$text}  ", "A  "),
            (
                "\u{00a0}\u{2003}{$text}\u{00a0}\u{2003}",
                "A\u{00a0}\u{2003}",
            ),
            ("\n  literal\n  ", "literal\n  "),
            ("\n  ```\n  literal\n  ```", "\n  literal\n  "),
            ("\n  ", ""),
            ("{span | A} {span | B}", "<span>A</span> <span>B</span>"),
            (
                "{span | A}\n  {span | B}\n  ",
                "<span>A</span>\n  <span>B</span>\n  ",
            ),
            ("{$text} tail\n  ", "A tail\n  "),
        ] {
            let source = format!("{{{tag} |{body}}}");
            let result = render_template(&source, &data("A"));
            assert!(
                result.diagnostics.is_empty(),
                "{source:?}: {:?}",
                result.diagnostics
            );
            assert_eq!(
                result.rendered,
                format!("<{tag}>{expected}</{tag}>"),
                "{source:?}"
            );
        }
    }
}

#[test]
fn binding_and_rich_content_whitespace_remain_exact() {
    for value in ["", "  ", "\n\t", "\u{00a0}\u{2003}", " \n🍒e\u{0301}\t "] {
        for (source, suffix) in [
            ("{textarea | {$text}}", ""),
            ("{textarea | {$text}\n  }", "\n  "),
            ("{textarea | {$text}```\n  ```}", "\n  "),
        ] {
            let result = render_template(source, &data(value));
            assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
            assert_eq!(
                result.rendered,
                format!("<textarea>{value}{suffix}</textarea>")
            );
        }
        let source = format!("{{textarea | ```{value}```}}");
        let result = render_template(&source, &TemplateData::default());
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.rendered, format!("<textarea>{value}</textarea>"));
    }
}

#[test]
fn binding_and_authored_suffix_keep_separate_source_ranges() {
    let source = "{textarea |\n  {$text}\n  }";
    let artifact = compile_template(
        source,
        &CompileTemplateOptions {
            host_bindings: vec!["text".into()],
            ..Default::default()
        },
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    for value in ["", " \n🍒e\u{0301}\t "] {
        let plan = render_compiled_template(&artifact, &data(value));
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        let [RenderPlanNode::Element { children, .. }] = plan.nodes.as_slice() else {
            panic!("expected one textarea");
        };
        let [RenderPlanNode::Text {
            text,
            source_map: binding_source,
        }, RenderPlanNode::Text {
            text: suffix,
            source_map,
        }] = children.as_slice()
        else {
            panic!("expected binding text and authored suffix: {children:?}");
        };
        assert_eq!(text, value);
        assert_eq!(suffix, "\n  ");
        assert_ne!(binding_source, source_map);
        let suffix_start = source.rfind('\n').unwrap();
        assert!(source_map.frames.iter().any(|frame| {
            frame.source_id == SourceId(1)
                && frame.span == FrameSpan::Single(ByteRange::new(suffix_start as u64, 3))
        }));
        assert_eq!(
            render_plan_to_html(&plan),
            format!("<textarea>{value}\n  </textarea>")
        );
    }
}

#[test]
fn xml_space_is_pass_through_in_cem_templates() {
    // The XSLT compiler's xml:space contract is separate from CEM template bodies.
    for policy in ["default", "preserve"] {
        let source = format!("{{textarea @xml:space={policy} |\n  {{$text}}\n  }}");
        let result = render_template(&source, &data("A"));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.rendered,
            format!("<textarea xml:space=\"{policy}\">A\n  </textarea>")
        );
    }
}
