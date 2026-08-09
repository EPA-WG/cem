use cem_ml::schema::registry::{CSS_CONTENT_TYPE, CSS_SCHEMA_URI, SCSS_CONTENT_TYPE};
use cem_ml::source::ByteRange;
use cem_ml::source_map::{FrameSpan, TransformKind};
use cem_ml::validation::scss::{
    evaluate_scss_to_css, parse_scss_source_bytes, ScssEvaluationRequest, ScssOriginKind,
    ScssSourceRequest, ScssStatementKind,
};
use cem_ml::{
    engine::{EngineContext, EngineInput},
    lifecycle::{LifecycleRegistry, LoadedInputAstStream},
};

fn parse(source: &str) -> cem_ml::validation::scss::ScssStylesheetAst {
    let (stylesheet, diagnostics) = parse_scss_source_bytes(ScssSourceRequest {
        bytes: source.as_bytes(),
        source_uri: "file:///styles/main.scss",
        content_type: Some(SCSS_CONTENT_TYPE),
    });
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    stylesheet.expect("typed SCSS stylesheet")
}

#[test]
fn scss_parser_is_lossless_and_retains_native_statement_ranges() {
    let source = r#"// component source
$component: "card";
$accent: #036;

@mixin tone($color) {
  color: $color;
}

.#{$component} {
  @include tone($accent);

  &__title {
    font-weight: 700;
  }
}
"#;
    let stylesheet = parse(source);

    assert_eq!(
        stylesheet
            .tokens
            .iter()
            .map(|token| token.lexeme.as_str())
            .collect::<String>(),
        source
    );
    assert_eq!(stylesheet.source.media_type, SCSS_CONTENT_TYPE);
    assert_eq!(stylesheet.source.syntax, "scss");
    assert_eq!(stylesheet.source.encoding, "utf-8");
    assert!(stylesheet
        .statements
        .iter()
        .any(|statement| statement.kind() == ScssStatementKind::VariableDeclaration));
    assert!(stylesheet
        .statements
        .iter()
        .any(|statement| statement.kind() == ScssStatementKind::MixinDeclaration));
    let rule = stylesheet
        .statements
        .iter()
        .find(|statement| statement.kind() == ScssStatementKind::Rule)
        .expect("top-level SCSS rule");
    assert!(rule.source_range().byte_length > 0);
    assert_eq!(rule.source_range().start.line, 9);
}

#[test]
fn scss_evaluator_lowers_directly_to_css_ast_with_exact_expansion_origins() {
    let source = r#"$component: "card";
$accent: #036;

@mixin tone($color) {
  color: $color;
}

.#{$component} {
  @include tone($accent);

  &__title {
    font-weight: 700;
  }
}
"#;
    let stylesheet = parse(source);
    let result = evaluate_scss_to_css(ScssEvaluationRequest {
        stylesheet: &stylesheet,
    });
    assert!(result.diagnostics.is_empty(), "{:#?}", result.diagnostics);
    let document = result.document.expect("typed CSS handoff");

    assert_eq!(document.source.media_type, CSS_CONTENT_TYPE);
    assert_eq!(result.target_schema, CSS_SCHEMA_URI);
    assert_eq!(
        document
            .events
            .iter()
            .map(|event| event.lexeme.as_str())
            .collect::<String>(),
        ".card {\n  color: #036;\n}\n.card__title {\n  font-weight: 700;\n}\n"
    );
    let mut generated_offset = 0u64;
    for event in &document.events {
        assert_eq!(event.source_range.start.byte_offset, generated_offset);
        assert_eq!(event.source_range.byte_length, event.lexeme.len() as u64);
        generated_offset += event.lexeme.len() as u64;
    }
    assert_eq!(generated_offset, document.source.byte_length as u64);

    let selector = document
        .events
        .iter()
        .find(|event| event.lexeme == ".card")
        .expect("interpolated selector event");
    assert_eq!(selector.source_range.start.line, 1);
    assert_eq!(selector.source_range.start.column, 1);
    assert_eq!(selector.source_range.start.byte_offset, 0);
    assert!(selector.source_map.frames.iter().any(|frame| {
        matches!(
            &frame.transform,
            TransformKind::ScssOrigin {
                origin_kind: ScssOriginKind::Interpolation,
                ..
            }
        )
    }));
    assert!(selector.source_map.frames.iter().any(|frame| {
        matches!(
            &frame.transform,
            TransformKind::ScssOrigin {
                origin_kind: ScssOriginKind::Definition,
                name: Some(name),
                ..
            } if name == "component"
        )
    }));
    let component_definition = selector
        .source_map
        .frames
        .iter()
        .find(|frame| {
            matches!(
                &frame.transform,
                TransformKind::ScssOrigin {
                    origin_kind: ScssOriginKind::Definition,
                    name: Some(name),
                    module_uri,
                } if name == "component" && module_uri == "file:///styles/main.scss"
            )
        })
        .expect("component definition origin");
    let component_source = "$component: \"card\";";
    assert_eq!(
        component_definition.span,
        FrameSpan::Single(ByteRange::new(
            source.find(component_source).unwrap() as u64,
            component_source.len() as u32,
        ))
    );

    let expanded_value = document
        .events
        .iter()
        .find(|event| event.lexeme == "#036")
        .expect("mixin-expanded variable value");
    assert_eq!(
        expanded_value.source_range.start.byte_offset,
        ".card {\n  color: ".len() as u64
    );
    for expected in [
        ScssOriginKind::Module,
        ScssOriginKind::Definition,
        ScssOriginKind::CallSite,
    ] {
        assert!(expanded_value.source_map.frames.iter().any(|frame| {
            matches!(
                &frame.transform,
                TransformKind::ScssOrigin { origin_kind, .. } if *origin_kind == expected
            )
        }));
    }
}

#[test]
fn scss_evaluator_handles_functions_and_control_flow_without_css_reparse() {
    let source = r#"@function double($value) {
  @return $value * 2;
}

$enabled: true;

@if $enabled {
  .meter {
    width: double(4px);
  }
}
"#;
    let stylesheet = parse(source);
    let result = evaluate_scss_to_css(ScssEvaluationRequest {
        stylesheet: &stylesheet,
    });
    assert!(result.diagnostics.is_empty(), "{:#?}", result.diagnostics);
    let document = result.document.expect("typed CSS handoff");
    assert_eq!(
        document
            .events
            .iter()
            .map(|event| event.lexeme.as_str())
            .collect::<String>(),
        ".meter {\n  width: 8px;\n}\n"
    );
    assert_eq!(document.recovery_count, 0);
    assert!(document.events.iter().all(|event| !event.recovered));
}

#[test]
fn scss_parser_rejects_indented_sass_and_invalid_utf8_with_owned_diagnostics() {
    let (stylesheet, diagnostics) = parse_scss_source_bytes(ScssSourceRequest {
        bytes: b".card\n  color: red\n",
        source_uri: "file:///styles/main.sass",
        content_type: Some(SCSS_CONTENT_TYPE),
    });
    assert!(stylesheet.is_none());
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.scss.parse_error"));

    let (stylesheet, diagnostics) = parse_scss_source_bytes(ScssSourceRequest {
        bytes: &[b'.', 0xff, b'{', b'}'],
        source_uri: "file:///styles/main.scss",
        content_type: Some("text/x-scss; charset=UTF-8"),
    });
    assert!(stylesheet.is_none());
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.scss.unsupported_encoding"));
}

#[test]
fn lifecycle_registry_loads_scss_as_a_generated_css_ast_without_claiming_css_input() {
    let source = b"$accent: #036; .card { color: $accent; }";
    let loaded = LifecycleRegistry::with_builtin_adapters().load(
        &EngineInput {
            uri: "file:///styles/main.scss".to_owned(),
            bytes: source.to_vec(),
            from_format: None,
            identity: None,
            root_scope: Default::default(),
        },
        &EngineContext {
            content_type: Some(SCSS_CONTENT_TYPE.to_owned()),
            schema: Some("https://cem.dev/ns/data/scss/1".to_owned()),
            ..EngineContext::default()
        },
    );

    assert_eq!(loaded.adapter_id, Some("scss"));
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    let document = match loaded.ast_stream.expect("SCSS typed CSS handoff") {
        LoadedInputAstStream::CssDocument(document) => document,
        other => panic!("unexpected SCSS lifecycle stream: {other:#?}"),
    };
    assert_eq!(document.source.media_type, CSS_CONTENT_TYPE);
    assert_eq!(
        document
            .events
            .iter()
            .map(|event| event.lexeme.as_str())
            .collect::<String>(),
        ".card {\n  color: #036;\n}\n"
    );
    assert!(document.events.iter().all(|event| {
        event.source_map.frames.iter().any(|frame| {
            matches!(
                frame.transform,
                TransformKind::ScssOrigin {
                    origin_kind: ScssOriginKind::Module,
                    ..
                }
            )
        })
    }));
}
