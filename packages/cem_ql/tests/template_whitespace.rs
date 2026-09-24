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

fn expect_render(source: &str, value: &str, expected: &str) {
    let result = render_template(source, &data(value));
    assert!(
        result.diagnostics.is_empty(),
        "{source:?}: {:?}",
        result.diagnostics
    );
    assert_eq!(result.rendered, expected, "{source:?}");
}

#[test]
fn layout_opt_in_removes_only_newline_trivia() {
    for value in ["", " \n\t ", "\u{00a0}\u{2003}", " 🍒e\u{0301}\t "] {
        for suffix in [
            "\n  ",
            "\r\n\t",
            "\r\t",
            "",
            "  ",
            "\n\u{00a0}",
            "\n\u{2003}",
        ] {
            let preserved = if suffix.contains(['\r', '\n']) && suffix.is_ascii() {
                ""
            } else {
                suffix
            };
            expect_render(
                &format!("{{textarea @cem:whitespace=layout | {{$text}}{suffix}}}"),
                value,
                &format!("<textarea>{value}{preserved}</textarea>"),
            );
        }
    }
    expect_render(
        "{p @cem:whitespace=layout |literal\n  }",
        "",
        "<p>literal\n  </p>",
    );
    expect_render(
        "{p @cem:whitespace=layout |```\n  ```{$text} ```\n  ```}",
        "A",
        "<p>\n  A \n  </p>",
    );
    expect_render(
        "{p @cem:whitespace=layout |{b | A} {i | B}\n  }",
        "",
        "<p><b>A</b> <i>B</i></p>",
    );
    expect_render(
        "{p @cem:whitespace=layout |{$text}\n\u{000b}}",
        "A",
        "<p>A\n\u{000b}</p>",
    );
}

#[test]
fn whitespace_modes_inherit_restore_and_override_on_control_nodes() {
    let source = "{section @cem:whitespace=layout |\n  {pre | {$text}\n  }\n  {pre @cem:whitespace=preserve |\n  {$text}\n  }\n  {pre | {$text}\n  }\n  }{pre | {$text}\n  }";
    expect_render(
        source,
        "A",
        "<section><pre>A</pre><pre>\n  A\n  </pre><pre>A</pre></section><pre>A\n  </pre>",
    );
    expect_render(
        "{pre @cem:whitespace=preserve |\n  {i @cem:whitespace=layout | {$text}\n  }\n  }",
        "A",
        "<pre>\n  <i>A</i>\n  </pre>",
    );
    expect_render(
        "{cem:if @cem:whitespace=preserve @test=true |\n  {$text}\n  }",
        "A",
        "\n  A\n  ",
    );
    expect_render(
        "{cem:for-each @cem:whitespace=layout @select='(1,2)' |\n  {b | {$item}\n  }\n  }",
        "",
        "<b>1</b><b>2</b>",
    );
    expect_render("{cem:choose @cem:whitespace=layout |{cem:when @test=true @cem:whitespace=preserve |\n  {$text}\n  }{cem:otherwise | unused}}", "A", "\n  A\n  ");
    expect_render("{cem:try @cem:whitespace=layout |{p | {$text}\n  }\n  {cem:catch @cem:whitespace=preserve | unused}}", "A", "<p>A</p>");
    expect_render(
        "{template @name=line | {$text}\n  }{p @cem:whitespace=layout |{call @template=line}\n  }",
        "A",
        "<p>A\n  </p>",
    );
}

#[test]
fn preserve_retains_opening_whitespace_and_never_emits_the_control_attribute() {
    for body in ["", " ", "\n\t", "\u{00a0}\u{2003}", "\n  literal\n  "] {
        expect_render(
            &format!("{{textarea @cem:whitespace=preserve |{body}}}"),
            "",
            &format!("<textarea>{body}</textarea>"),
        );
    }
    expect_render(
        "{p @title=ok @cem:whitespace=preserve @id=note | {$text} }",
        "A",
        "<p title=\"ok\" id=\"note\"> A </p>",
    );
}

#[test]
fn invalid_and_duplicate_whitespace_controls_report_authored_ranges() {
    for control in [
        "@cem:whitespace",
        "@cem:whitespace=unknown",
        "@cem:whitespace='{text}'",
        "@cem:whitespace=layout @cem:whitespace=preserve",
    ] {
        let source = format!("{{p {control} | {{$text}}}}");
        let artifact = compile_template(
            &source,
            &CompileTemplateOptions {
                host_bindings: vec!["text".into()],
                ..Default::default()
            },
        );
        let diagnostic = artifact
            .diagnostics
            .iter()
            .find(|d| d.code == "cem.ql.render.whitespace_policy_invalid")
            .expect("invalid static policy must be diagnosed");
        let offset = source.rfind("@cem:whitespace").unwrap() as u64;
        assert_eq!(diagnostic.byte_offset, Some(offset));
        assert!(!diagnostic.source_map.as_ref().unwrap().frames.is_empty());
    }
}

#[test]
fn portable_whitespace_policy_keeps_suppressed_source_provenance() {
    use cem_ql::template_artifact::{
        compile_template_artifact, CompiledTemplateArtifact, TemplateArtifactLoadContext,
        TemplateArtifactSourceMapMode,
    };
    let source = "{textarea @cem:whitespace=layout |\n  {$text}\n  }";
    let options = CompileTemplateOptions {
        host_bindings: vec!["text".into()],
        ..Default::default()
    };
    let direct = compile_template(source, &options);
    for mode in [
        TemplateArtifactSourceMapMode::Dev,
        TemplateArtifactSourceMapMode::Prod,
    ] {
        let compiled = compile_template_artifact(source, &options, mode);
        let reloaded = CompiledTemplateArtifact::from_bytes(compiled.bytes)
            .unwrap()
            .reload(&TemplateArtifactLoadContext {
                expected_source_hash: Some(compiled.identity.source_hash),
                host_bindings: options.host_bindings.clone(),
                source_map_mode: mode,
            })
            .unwrap();
        for value in ["", " \n🍒e\u{0301}\t "] {
            let plan = render_compiled_template(&reloaded, &data(value));
            assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
            assert_eq!(
                render_plan_to_html(&plan),
                format!("<textarea>{value}</textarea>")
            );
            if mode == TemplateArtifactSourceMapMode::Dev {
                assert_eq!(
                    plan.nodes,
                    render_compiled_template(&direct, &data(value)).nodes
                );
                let [RenderPlanNode::Element { children, .. }] = plan.nodes.as_slice() else {
                    panic!("textarea missing")
                };
                let suffix = children.last().unwrap();
                let RenderPlanNode::Text { text, source_map } = suffix else {
                    panic!("source evidence missing")
                };
                assert!(text.is_empty(), "layout emits no characters");
                assert_eq!(
                    source_map.origin().unwrap().span,
                    FrameSpan::Single(ByteRange::new(source.rfind('\n').unwrap() as u64, 3))
                );
            }
        }
    }
}
