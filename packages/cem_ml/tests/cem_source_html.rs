use cem_ml::{engine::*, real::RealCemMlEngine, run_config::ScopeConfig};

const SOURCE: &str = "@doc cem-ml 1\n{main @title=\"café & <🍒>\" |\n {p | <script>alert(1)</script> & café 🍒}\n}\n";
fn convert(profile: &str, color: Option<&str>, legacy: bool) -> ConvertResponse {
    let mut scope = ScopeConfig {
        cemt_formatter_profile: Some(profile.into()),
        ..Default::default()
    };
    if legacy {
        scope.cemt_color_profile = Some("html".into());
    }
    scope.output_color_type = color.map(Into::into);
    convert_scope(scope)
}
fn convert_scope(scope: ScopeConfig) -> ConvertResponse {
    RealCemMlEngine::new()
        .convert(ConvertRequest {
            input: EngineInput {
                uri: "studio://login/data/source.cem".into(),
                bytes: SOURCE.as_bytes().to_vec(),
                identity: Some(FormatIdentity {
                    content_type: Some("application/cem".into()),
                    schema: Some("https://cem.dev/ns/cem-ml/1".into()),
                    ..Default::default()
                }),
                from_format: None,
                root_scope: Default::default(),
            },
            target: None,
            to_format: LayerFormat::Cem,
            preserve_source_offsets: true,
            target_scope: scope,
            context: EngineContext::default(),
            scheduler_scope_id: 0,
        })
        .unwrap()
}
fn visible_text(html: &str) -> String {
    let body = html
        .split_once('>')
        .unwrap()
        .1
        .rsplit_once("</pre>")
        .unwrap()
        .0;
    let mut visible = String::new();
    let mut tag = false;
    for ch in body.chars() {
        match ch {
            '<' => tag = true,
            '>' => tag = false,
            _ if !tag => visible.push(ch),
            _ => {}
        }
    }
    visible
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}
#[test]
fn html_source_colors_preserve_layout_escape_markup_and_report_html_identity() {
    for profile in ["pretty", "tabular"] {
        let plain = convert(profile, Some("none"), false).primary_bytes.unwrap();
        assert_eq!(plain.content_type, "application/cem");
        let plain = String::from_utf8(plain.bytes).unwrap();
        for selector in ["html", "html-css-vars", "html-classes", "html-inline-style"] {
            let response = convert(profile, Some(selector), false);
            assert!(
                !response
                    .diagnostics
                    .iter()
                    .any(|d| d.severity.is_hard_violation()),
                "{:?}",
                response.diagnostics
            );
            let output = response.primary_bytes.unwrap();
            let html = String::from_utf8(output.bytes).unwrap();
            assert!(html.starts_with("<pre"), "{selector}: {html}");
            assert!(html.contains("data-role="));
            assert!(!html.contains("<script>"));
            assert!(html.contains("&lt;script&gt;"));
            assert!(!html.contains('\u{1b}'));
            assert_eq!(visible_text(&html), plain, "{profile}/{selector}");
            assert_eq!(output.content_type, "text/html");
            assert_eq!(
                output.schema.as_deref(),
                Some("https://cem.dev/ns/data/html/1")
            );
            let writer = response
                .conversion
                .unwrap()
                .output_pipeline
                .unwrap()
                .stages
                .pop()
                .unwrap();
            assert_eq!(writer.content_type.as_deref(), Some("text/html"));
            for span in response.primary["outputSpans"].as_array().unwrap() {
                let span: cem_ml::interpreter::OutputSpan =
                    serde_json::from_value(span.clone()).unwrap();
                let start = span.output_range.start as usize;
                let end = span.output_range.end() as usize;
                assert!(html.get(start..end).is_some());
            }
        }
    }
}
#[test]
fn legacy_html_profile_and_terminal_or_plain_selection_remain_explicit() {
    let legacy = convert("tabular", None, true).primary_bytes.unwrap();
    assert!(String::from_utf8(legacy.bytes).unwrap().starts_with("<pre"));
    let plain = convert("tabular", Some("none"), true)
        .primary_bytes
        .unwrap();
    assert!(!String::from_utf8(plain.bytes).unwrap().contains("<span"));
    let terminal = convert("tabular", Some("ansi-256"), false)
        .primary_bytes
        .unwrap();
    assert!(String::from_utf8(terminal.bytes)
        .unwrap()
        .contains('\u{1b}'));
}

#[test]
fn html_source_colors_retain_formatter_options_and_original_source_frames() {
    let mut scope = ScopeConfig {
        cemt_formatter_profile: Some("tabular".into()),
        cemt_formatter_options: [
            ("indent".into(), "\t".into()),
            ("tabSize".into(), "6".into()),
            ("lineEnding".into(), "crlf".into()),
        ]
        .into(),
        output_color_type: Some("none".into()),
        ..Default::default()
    };
    let plain_response = convert_scope(scope.clone());
    let plain = String::from_utf8(plain_response.primary_bytes.unwrap().bytes).unwrap();
    scope.output_color_type = Some("html-css-vars".into());
    let response = convert_scope(scope);
    let html = String::from_utf8(response.primary_bytes.unwrap().bytes).unwrap();
    assert!(html.contains("tab-size: 6"), "{html}");
    assert_eq!(visible_text(&html), plain);
    let spans = response.primary["outputSpans"].as_array().unwrap();
    assert!(!spans.is_empty());
    let mut unicode_origin = false;
    for value in spans {
        let span: cem_ml::interpreter::OutputSpan = serde_json::from_value(value.clone()).unwrap();
        let output = &html[span.output_range.start as usize..span.output_range.end() as usize];
        let source_frames: Vec<_> = span
            .origin
            .frames
            .iter()
            .filter(|frame| {
                !matches!(
                    frame.transform,
                    cem_ml::source_map::TransformKind::InterpreterRender
                )
            })
            .collect();
        assert!(!source_frames.is_empty());
        assert!(plain_response.primary["outputSpans"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| {
                let plain_span: cem_ml::interpreter::OutputSpan =
                    serde_json::from_value(value.clone()).unwrap();
                plain_span
                    .origin
                    .frames
                    .iter()
                    .filter(|frame| {
                        !matches!(
                            frame.transform,
                            cem_ml::source_map::TransformKind::InterpreterRender
                        )
                    })
                    .collect::<Vec<_>>()
                    == source_frames
            }));
        if output.contains("🍒") {
            let frame = source_frames
                .iter()
                .rev()
                .find(|frame| {
                    matches!(
                        frame.transform,
                        cem_ml::source_map::TransformKind::CemAstBuilder
                    )
                })
                .unwrap();
            let cem_ml::source_map::FrameSpan::Single(range) = frame.span else {
                panic!("expected a retained source range")
            };
            let authored = &SOURCE[range.start as usize..range.end() as usize];
            // Attribute output retains its attribute-name owner; text retains
            // its original text range, as in the uncolored typed writer.
            if output.contains("@title=") {
                assert_eq!(authored, "title");
            } else {
                assert!(authored.contains("🍒"));
                unicode_origin = true;
            }
        }
    }
    assert!(unicode_origin);
}

#[test]
fn package_color_profiles_preserve_cem_encoding_and_original_writer_spans() {
    let plain = convert("tabular", Some("none"), false);
    let plain_bytes = plain.primary_bytes.unwrap().bytes;
    for profile in ["classes", "inline-style", "css-custom-properties"] {
        let result = convert_scope(ScopeConfig {
            cemt_formatter_profile: Some("tabular".into()),
            cemt_color_profile: Some(profile.into()),
            ..Default::default()
        });
        assert!(
            result.diagnostics.is_empty(),
            "{profile}: {:?}",
            result.diagnostics
        );
        let output = result.primary_bytes.unwrap();
        assert_eq!(output.content_type, "application/cem", "{profile}");
        assert_eq!(
            output.schema.as_deref(),
            Some("https://cem.dev/ns/cem-ml/1")
        );
        assert_eq!(
            output.bytes, plain_bytes,
            "{profile} must not request an HTML preview"
        );
        assert_eq!(
            result.primary["outputSpans"], plain.primary["outputSpans"],
            "{profile}: source writer ownership must remain intact"
        );
        let stages = result.conversion.unwrap().output_pipeline.unwrap().stages;
        let colorizer = stages
            .iter()
            .find(|stage| stage.stage == "colorizer")
            .unwrap();
        assert_eq!(colorizer.profile.as_deref(), Some(profile));
        assert_eq!(
            stages.last().unwrap().content_type.as_deref(),
            Some("application/cem")
        );
    }
}

#[test]
fn explicit_writer_encoding_overrides_package_profile_and_html_aliases_are_exact() {
    let plain = convert("tabular", Some("none"), false)
        .primary_bytes
        .unwrap()
        .bytes;
    for profile in ["classes", "inline-style", "css-custom-properties", "html"] {
        let output = convert_scope(ScopeConfig {
            cemt_formatter_profile: Some("tabular".into()),
            cemt_color_profile: Some(profile.into()),
            output_color_type: Some("none".into()),
            ..Default::default()
        });
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let output = output.primary_bytes.unwrap();
        assert_eq!(output.content_type, "application/cem");
        assert_eq!(
            output.bytes, plain,
            "{profile}: explicit none has precedence"
        );
    }
    for selector in [
        "html-classes",
        "classes",
        "class-based",
        "html.classes",
        "html-css-vars",
        "css-vars",
        "css-custom-properties",
        "html-inline-style",
        "inline-style",
    ] {
        let response = convert("tabular", Some(selector), false);
        assert!(
            response.diagnostics.is_empty(),
            "{selector}: {:?}",
            response.diagnostics
        );
        let output = response.primary_bytes.unwrap();
        assert_eq!(output.content_type, "text/html", "{selector}");
        let html = String::from_utf8(output.bytes).unwrap();
        assert!(html.starts_with("<pre"), "{selector}: {html}");
        assert!(
            html.ends_with("</pre>"),
            "{selector}: no source-only newline outside the preview: {html:?}"
        );
        assert_eq!(visible_text(&html).as_bytes(), plain, "{selector}");
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }
}
