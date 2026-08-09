use cem_ml::resolver::{
    ResolveDirection, ResolveListRequest, ResolvePolicyDenial, ResolvePolicyRequestKey,
    ResolvePurpose, ResolveRequest, ResolvedListEntry, ResolvedRead, ResolvedWrite,
    ResolverDiagnostic, ResolverPolicy, ResolverRegistry, ResourceResolver,
};
use cem_ml::scheduler::AbortSignal;
use cem_ml::schema::registry::{
    SchemaRegistry, CSS_CONTENT_TYPE, CSS_SCHEMA_URI, SCSS_CONTENT_TYPE,
};
use cem_ml::source::ByteRange;
use cem_ml::source_map::{FrameSpan, TransformKind};
use cem_ml::validation::scss::{
    evaluate_scss_to_css, evaluate_scss_to_css_with_policy, parse_scss_source_bytes,
    ScssEvaluationLimits, ScssEvaluationRequest, ScssOriginKind, ScssPolicyEvaluationRequest,
    ScssSafetyPolicy, ScssSourceRequest, ScssStatementKind,
};
use cem_ml::{
    conversion::{
        execute_css_document_output_pipeline_with_environment, ConversionOutputPipelineEnvironment,
        ConversionRegistry,
    },
    engine::{
        CemMlEngine, ConvertRequest, EngineContext, EngineInput, FormatIdentity, LayerFormat,
    },
    lifecycle::{LifecycleRegistry, LoadedInputAstStream},
    real::RealCemMlEngine,
    run_config::ScopeConfig,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
struct ScssMapResolver {
    entries: BTreeMap<String, Vec<u8>>,
}

impl ResourceResolver for ScssMapResolver {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        let canonical = if request.uri.contains("://") {
            request.uri.clone()
        } else {
            let base = request.base_uri.as_deref().unwrap_or_default();
            let directory = base
                .rsplit_once('/')
                .map_or(base, |(directory, _)| directory);
            format!("{directory}/{}", request.uri)
        };
        self.entries
            .get(&canonical)
            .cloned()
            .map(|bytes| ResolvedRead {
                uri: canonical.clone(),
                bytes,
                content_type: Some(SCSS_CONTENT_TYPE.to_owned()),
            })
            .ok_or_else(|| ResolverDiagnostic::Io {
                uri: canonical,
                message: "SCSS test resource not found".to_owned(),
            })
    }

    fn write(
        &self,
        request: &ResolveRequest,
        _bytes: &[u8],
    ) -> Result<ResolvedWrite, ResolverDiagnostic> {
        Err(ResolverDiagnostic::UnsupportedResolver {
            uri: request.uri.clone(),
            purpose: request.purpose,
            direction: ResolveDirection::Write,
        })
    }

    fn list(
        &self,
        request: &ResolveListRequest,
    ) -> Result<Vec<ResolvedListEntry>, ResolverDiagnostic> {
        Err(ResolverDiagnostic::UnsupportedResolver {
            uri: request.uri.clone(),
            purpose: request.purpose,
            direction: ResolveDirection::List,
        })
    }
}

fn scss_registry(entries: &[(&str, &str)]) -> ResolverRegistry {
    let mut registry = ResolverRegistry::new();
    registry.register(
        "cem+vfs",
        ResolvePurpose::Input,
        ResolveDirection::Read,
        ScssMapResolver {
            entries: entries
                .iter()
                .map(|(uri, source)| ((*uri).to_owned(), source.as_bytes().to_vec()))
                .collect(),
        },
    );
    registry
}

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

#[test]
fn lifecycle_registry_routes_scss_modules_through_the_engine_resolver_context() {
    let source = b"@use \"tokens\"; .card { @include tokens.inset(); }";
    let resolver_registry = scss_registry(&[(
        "cem+vfs://styles/_tokens.scss",
        "$space: 0.75rem; @mixin inset() { padding: $space; }",
    )]);
    let loaded = LifecycleRegistry::with_builtin_adapters().load(
        &EngineInput {
            uri: "cem+vfs://styles/main.scss".to_owned(),
            bytes: source.to_vec(),
            from_format: None,
            identity: None,
            root_scope: Default::default(),
        },
        &EngineContext {
            content_type: Some(SCSS_CONTENT_TYPE.to_owned()),
            schema: Some("https://cem.dev/ns/data/scss/1".to_owned()),
            resolver_registry,
            ..EngineContext::default()
        },
    );

    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    let document = match loaded.ast_stream.expect("resolved SCSS lifecycle stream") {
        LoadedInputAstStream::CssDocument(document) => document,
        other => panic!("unexpected SCSS lifecycle stream: {other:#?}"),
    };
    assert_eq!(
        document
            .events
            .iter()
            .map(|event| event.lexeme.as_str())
            .collect::<String>(),
        ".card {\n  padding: 0.75rem;\n}\n"
    );
}

#[test]
fn source_validation_is_passive_while_explicit_css_export_reuses_css_stages() {
    let source = EngineInput {
        uri: "file:///styles/main.scss".to_owned(),
        bytes: b"@use \"unavailable\"; .card { color: #036; }".to_vec(),
        from_format: None,
        identity: Some(FormatIdentity {
            content_type: Some(SCSS_CONTENT_TYPE.to_owned()),
            schema: Some("https://cem.dev/ns/data/scss/1".to_owned()),
            ..FormatIdentity::default()
        }),
        root_scope: Default::default(),
    };
    let loaded = LifecycleRegistry::with_builtin_adapters()
        .load_for_source_validation(&source, &EngineContext::default());

    assert_eq!(loaded.adapter_id, Some("scss"));
    assert!(loaded.ast_stream.is_none());
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);

    let input = EngineInput {
        uri: "file:///styles/card.scss".to_owned(),
        bytes: b"$accent: #036; .card { color: $accent; }".to_vec(),
        from_format: None,
        identity: Some(FormatIdentity {
            content_type: Some(SCSS_CONTENT_TYPE.to_owned()),
            schema: Some("https://cem.dev/ns/data/scss/1".to_owned()),
            ..FormatIdentity::default()
        }),
        root_scope: Default::default(),
    };
    let response = RealCemMlEngine::new()
        .convert(ConvertRequest {
            input,
            to_format: LayerFormat::Css,
            preserve_source_offsets: false,
            context: EngineContext::default(),
            target: Some(FormatIdentity {
                content_type: Some(CSS_CONTENT_TYPE.to_owned()),
                schema: Some(CSS_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            target_scope: ScopeConfig {
                cemt_formatter_profile: Some("tabular".to_owned()),
                ..ScopeConfig::default()
            },
            scheduler_scope_id: 0,
        })
        .expect("SCSS to CSS lifecycle conversion");

    assert!(
        response.diagnostics.is_empty(),
        "{:#?}",
        response.diagnostics
    );
    let bytes = response.primary_bytes.expect("browser-facing CSS bytes");
    assert_eq!(bytes.content_type, CSS_CONTENT_TYPE);
    assert_eq!(bytes.schema.as_deref(), Some(CSS_SCHEMA_URI));
    assert_eq!(
        String::from_utf8(bytes.bytes).expect("CSS output is UTF-8"),
        ".card {\n  color: #036;\n}\n"
    );
    let metadata = response.conversion.expect("CSS lifecycle metadata");
    assert_eq!(
        metadata.converter_id.as_deref(),
        Some("css-lifecycle-output")
    );
    assert_eq!(
        metadata.implementation.as_deref(),
        Some("css-ast-stream-to-css-output-pipeline")
    );
    let stages = metadata.output_pipeline.expect("CSS output stages").stages;
    assert_eq!(
        stages
            .iter()
            .map(|stage| stage.stage.as_str())
            .collect::<Vec<_>>(),
        ["formatter", "colorizer", "writer"]
    );
    assert_eq!(stages[0].profile.as_deref(), Some("tabular"));
    assert_eq!(stages[1].profile, None);
    assert!(stages.iter().all(|stage| {
        stage.content_type.as_deref() == Some(CSS_CONTENT_TYPE)
            && stage.schema.as_deref() == Some(CSS_SCHEMA_URI)
    }));
}

#[test]
fn scss_typed_handoff_reuses_css_formatter_and_colorizer_assets() {
    let loaded = LifecycleRegistry::with_builtin_adapters().load(
        &EngineInput {
            uri: "file:///styles/presentation.scss".to_owned(),
            bytes: b"$accent: #036; .card { color: $accent; }".to_vec(),
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
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    let document = match loaded.ast_stream.expect("SCSS CSS handoff") {
        LoadedInputAstStream::CssDocument(document) => document,
        other => panic!("unexpected SCSS lifecycle stream: {other:#?}"),
    };
    let schema_registry = SchemaRegistry::with_builtin_schemas();
    let conversion_registry = ConversionRegistry::with_builtin_converters();
    let execution = execute_css_document_output_pipeline_with_environment(
        &ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        },
        document,
        &ScopeConfig {
            cemt_formatter_profile: Some("tabular".to_owned()),
            cemt_color_profile: Some("html".to_owned()),
            ..ScopeConfig::default()
        },
        Some("file:///styles/presentation.scss"),
    );

    assert!(
        execution.diagnostics.is_empty(),
        "{:#?}",
        execution.diagnostics
    );
    assert_eq!(
        execution.formatted_cem_tree.as_ref().unwrap().value["contentType"],
        CSS_CONTENT_TYPE
    );
    assert_eq!(
        execution.formatted_cem_tree.as_ref().unwrap().value["formatterProfile"],
        "tabular"
    );
    assert_eq!(
        execution.colored_cem_tree.as_ref().unwrap().value["contentType"],
        CSS_CONTENT_TYPE
    );
    assert_eq!(
        execution.colored_cem_tree.as_ref().unwrap().value["colorProfile"],
        "html"
    );
}

#[test]
fn generated_css_uses_css_schema_validation_without_losing_scss_origins() {
    let loaded = LifecycleRegistry::with_builtin_adapters().load(
        &EngineInput {
            uri: "file:///styles/hero.scss".to_owned(),
            bytes: b".hero { background-image: url(\"images/hero.png\"); }".to_vec(),
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

    let diagnostic = loaded
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "cem.css.url_rejected")
        .expect("CSS package owns generated-value validation");
    assert_eq!(
        diagnostic.details.as_ref().unwrap()["schema"],
        CSS_SCHEMA_URI
    );
    assert!(diagnostic.source_map.as_ref().is_some_and(|source_map| {
        source_map.frames.iter().any(|frame| {
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

#[test]
fn scss_modules_use_explicit_resolver_policy_and_export_namespaced_members_once() {
    let stylesheet = parse(
        r#"@use "tokens";
@use "tokens";

.card {
  @include tokens.inset(tokens.$space);
  width: tokens.double(2px);
}
"#,
    );
    let registry = scss_registry(&[(
        "cem+vfs://styles/_tokens.scss",
        r#"$space: 0.5rem;
@mixin inset($amount) { padding: $amount; }
@function double($value) { @return $value * 2; }
.token-source { --loaded: once; }
"#,
    )]);
    let policy = ResolverPolicy::new();
    let policy_stamp = policy.cache_stamp();
    let safety_policy = ScssSafetyPolicy::default();
    let safety_policy_stamp = safety_policy.cache_stamp();
    let abort_signal = AbortSignal::new();

    let result = evaluate_scss_to_css_with_policy(ScssPolicyEvaluationRequest {
        stylesheet: &stylesheet,
        resolver_registry: &registry,
        resolver_policy: &policy,
        resolver_policy_stamp: &policy_stamp,
        safety_policy: &safety_policy,
        safety_policy_stamp: &safety_policy_stamp,
        abort_signal: &abort_signal,
        load_paths: &["cem+vfs://styles/load-path.scss".to_owned()],
        limits: ScssEvaluationLimits::default(),
    });

    assert!(result.diagnostics.is_empty(), "{:#?}", result.diagnostics);
    assert_eq!(result.module_resolutions.len(), 2);
    assert!(result.module_resolutions.iter().all(|resolution| {
        resolution.requested_uri == "_tokens.scss"
            && resolution.normalized_uri == "_tokens.scss"
            && resolution.effective_uri == "_tokens.scss"
            && resolution.canonical_uri == "cem+vfs://styles/_tokens.scss"
            && resolution.edge_kind == "use"
            && resolution.namespace.as_deref() == Some("tokens")
            && resolution.resolver_policy_stamp == policy_stamp
    }));
    let document = result.document.expect("resolved SCSS CSS handoff");
    let css = document
        .events
        .iter()
        .map(|event| event.lexeme.as_str())
        .collect::<String>();
    assert_eq!(css.matches(".token-source").count(), 1);
    assert!(css.contains("padding: 0.5rem;"), "{css}");
    assert!(css.contains("width: 4px;"), "{css}");
    let padding = document
        .events
        .iter()
        .find(|event| event.lexeme == "0.5rem")
        .expect("module variable expansion");
    assert!(padding.source_map.frames.iter().any(|frame| {
        matches!(
            &frame.transform,
            TransformKind::ScssOrigin {
                module_uri,
                origin_kind: ScssOriginKind::Definition,
                ..
            } if module_uri == "cem+vfs://styles/_tokens.scss"
        )
    }));
}

#[test]
fn scss_module_resolution_reports_policy_denials_and_complete_cycles() {
    let unsafe_stylesheet = parse("@use \"../tokens\";\n");
    let empty_registry = scss_registry(&[]);
    let allow_policy = ResolverPolicy::new();
    let allow_policy_stamp = allow_policy.cache_stamp();
    let safety_policy = ScssSafetyPolicy::default();
    let safety_policy_stamp = safety_policy.cache_stamp();
    let abort_signal = AbortSignal::new();
    let unsafe_result = evaluate_scss_to_css_with_policy(ScssPolicyEvaluationRequest {
        stylesheet: &unsafe_stylesheet,
        resolver_registry: &empty_registry,
        resolver_policy: &allow_policy,
        resolver_policy_stamp: &allow_policy_stamp,
        safety_policy: &safety_policy,
        safety_policy_stamp: &safety_policy_stamp,
        abort_signal: &abort_signal,
        load_paths: &[],
        limits: ScssEvaluationLimits::default(),
    });
    assert!(unsafe_result.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "cem.scss.resolver_denied"
            && diagnostic.message.contains("parent traversal is disabled")
    }));

    let denied_stylesheet = parse("@use \"tokens\";\n");
    let registry = scss_registry(&[]);
    let policy = ResolverPolicy::new().with_denial(
        ResolvePolicyRequestKey::new(
            "_tokens.scss",
            ResolvePurpose::Input,
            ResolveDirection::Read,
        )
        .with_base_uri("file:///styles/main.scss")
        .with_content_type_hint(SCSS_CONTENT_TYPE),
        ResolvePolicyDenial::new("fixture-denial"),
    );
    let policy_stamp = policy.cache_stamp();
    let denied = evaluate_scss_to_css_with_policy(ScssPolicyEvaluationRequest {
        stylesheet: &denied_stylesheet,
        resolver_registry: &registry,
        resolver_policy: &policy,
        resolver_policy_stamp: &policy_stamp,
        safety_policy: &safety_policy,
        safety_policy_stamp: &safety_policy_stamp,
        abort_signal: &abort_signal,
        load_paths: &[],
        limits: ScssEvaluationLimits::default(),
    });
    let denial = denied
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "cem.scss.resolver_denied")
        .expect("resolver policy denial");
    assert!(denial.message.contains("fixture-denial"));
    assert_eq!(
        denial.details.as_ref().unwrap()["resolverPolicyStamp"],
        policy_stamp
    );
    assert!(denied.document.is_none());

    let cycle_stylesheet = parse("@use \"a\";\n");
    let registry = scss_registry(&[
        ("cem+vfs://styles/_a.scss", "@use \"main\";\n"),
        ("cem+vfs://styles/main.scss", "@use \"a\";\n"),
    ]);
    let policy = ResolverPolicy::new();
    let policy_stamp = policy.cache_stamp();
    let cycle = evaluate_scss_to_css_with_policy(ScssPolicyEvaluationRequest {
        stylesheet: &cycle_stylesheet,
        resolver_registry: &registry,
        resolver_policy: &policy,
        resolver_policy_stamp: &policy_stamp,
        safety_policy: &safety_policy,
        safety_policy_stamp: &safety_policy_stamp,
        abort_signal: &abort_signal,
        load_paths: &["cem+vfs://styles/main.scss".to_owned()],
        limits: ScssEvaluationLimits::default(),
    });
    let diagnostic = cycle
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "cem.scss.module_cycle")
        .expect("module cycle diagnostic");
    assert!(diagnostic.message.contains("main.scss"));
    assert!(diagnostic.message.contains("_a.scss"));
    assert!(cycle.document.is_none());
}

#[test]
fn scss_forward_and_legacy_import_share_the_explicit_module_boundary() {
    let registry = scss_registry(&[(
        "cem+vfs://styles/_tokens.scss",
        "$space: 1rem; @mixin inset() { padding: $space; } .tokens { gap: $space; }",
    )]);
    let policy = ResolverPolicy::new();
    let policy_stamp = policy.cache_stamp();
    let safety_policy = ScssSafetyPolicy::default();
    let safety_policy_stamp = safety_policy.cache_stamp();
    let abort_signal = AbortSignal::new();
    let load_paths = ["cem+vfs://styles/main.scss".to_owned()];

    let forwarded_stylesheet = parse("@forward \"tokens\";\n");
    let forwarded = evaluate_scss_to_css_with_policy(ScssPolicyEvaluationRequest {
        stylesheet: &forwarded_stylesheet,
        resolver_registry: &registry,
        resolver_policy: &policy,
        resolver_policy_stamp: &policy_stamp,
        safety_policy: &safety_policy,
        safety_policy_stamp: &safety_policy_stamp,
        abort_signal: &abort_signal,
        load_paths: &load_paths,
        limits: ScssEvaluationLimits::default(),
    });
    assert!(
        forwarded.diagnostics.is_empty(),
        "{:#?}",
        forwarded.diagnostics
    );
    assert_eq!(forwarded.module_resolutions[0].edge_kind, "forward");
    assert!(forwarded
        .document
        .expect("forwarded module CSS")
        .events
        .iter()
        .any(|event| event.lexeme == ".tokens"));

    let imported_source = r#"@import "tokens";
.legacy { @include inset(); }
"#;
    let (imported_stylesheet, parse_diagnostics) = parse_scss_source_bytes(ScssSourceRequest {
        bytes: imported_source.as_bytes(),
        source_uri: "file:///styles/main.scss",
        content_type: Some(SCSS_CONTENT_TYPE),
    });
    assert!(parse_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.scss.import_deprecated"));
    let imported_stylesheet = imported_stylesheet.expect("legacy import stylesheet");
    let imported = evaluate_scss_to_css_with_policy(ScssPolicyEvaluationRequest {
        stylesheet: &imported_stylesheet,
        resolver_registry: &registry,
        resolver_policy: &policy,
        resolver_policy_stamp: &policy_stamp,
        safety_policy: &safety_policy,
        safety_policy_stamp: &safety_policy_stamp,
        abort_signal: &abort_signal,
        load_paths: &load_paths,
        limits: ScssEvaluationLimits::default(),
    });
    assert!(
        imported.diagnostics.is_empty(),
        "{:#?}",
        imported.diagnostics
    );
    assert_eq!(imported.module_resolutions[0].edge_kind, "import");
    assert!(imported
        .document
        .expect("legacy import CSS")
        .events
        .iter()
        .any(|event| event.lexeme == "1rem"));
}

#[test]
fn scss_evaluation_observes_cancellation_recursion_work_and_output_limits() {
    let registry = ResolverRegistry::new();
    let policy = ResolverPolicy::new();
    let policy_stamp = policy.cache_stamp();
    let safety_policy = ScssSafetyPolicy::default();
    let safety_policy_stamp = safety_policy.cache_stamp();

    let cancelled_stylesheet = parse(".card { color: red; }");
    let cancelled_signal = AbortSignal::new();
    cancelled_signal.abort();
    let cancelled = evaluate_scss_to_css_with_policy(ScssPolicyEvaluationRequest {
        stylesheet: &cancelled_stylesheet,
        resolver_registry: &registry,
        resolver_policy: &policy,
        resolver_policy_stamp: &policy_stamp,
        safety_policy: &safety_policy,
        safety_policy_stamp: &safety_policy_stamp,
        abort_signal: &cancelled_signal,
        load_paths: &[],
        limits: ScssEvaluationLimits::default(),
    });
    assert!(cancelled
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.scss.cancelled"));
    assert!(cancelled.document.is_none());

    let recursive_stylesheet = parse(
        r#"@function forever($value) { @return forever($value); }
.card { width: forever(1px); }
"#,
    );
    let recursive = evaluate_scss_to_css_with_policy(ScssPolicyEvaluationRequest {
        stylesheet: &recursive_stylesheet,
        resolver_registry: &registry,
        resolver_policy: &policy,
        resolver_policy_stamp: &policy_stamp,
        safety_policy: &safety_policy,
        safety_policy_stamp: &safety_policy_stamp,
        abort_signal: &AbortSignal::new(),
        load_paths: &[],
        limits: ScssEvaluationLimits {
            max_recursion_depth: 8,
            ..ScssEvaluationLimits::default()
        },
    });
    assert!(recursive.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "cem.scss.budget_exceeded"
            && diagnostic.message.contains("max-recursion-depth=8")
    }));

    let work_limited = evaluate_scss_to_css_with_policy(ScssPolicyEvaluationRequest {
        stylesheet: &cancelled_stylesheet,
        resolver_registry: &registry,
        resolver_policy: &policy,
        resolver_policy_stamp: &policy_stamp,
        safety_policy: &safety_policy,
        safety_policy_stamp: &safety_policy_stamp,
        abort_signal: &AbortSignal::new(),
        load_paths: &[],
        limits: ScssEvaluationLimits {
            max_work_units: 1,
            ..ScssEvaluationLimits::default()
        },
    });
    assert!(work_limited.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "cem.scss.budget_exceeded"
            && diagnostic.message.contains("max-work-units=1")
    }));

    let output_limited = evaluate_scss_to_css_with_policy(ScssPolicyEvaluationRequest {
        stylesheet: &cancelled_stylesheet,
        resolver_registry: &registry,
        resolver_policy: &policy,
        resolver_policy_stamp: &policy_stamp,
        safety_policy: &safety_policy,
        safety_policy_stamp: &safety_policy_stamp,
        abort_signal: &AbortSignal::new(),
        load_paths: &[],
        limits: ScssEvaluationLimits {
            max_output_nodes: 1,
            ..ScssEvaluationLimits::default()
        },
    });
    assert!(output_limited.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "cem.scss.budget_exceeded"
            && diagnostic.message.contains("max-output-nodes=1")
    }));
    assert!(output_limited.document.is_none());

    let byte_limited = evaluate_scss_to_css_with_policy(ScssPolicyEvaluationRequest {
        stylesheet: &cancelled_stylesheet,
        resolver_registry: &registry,
        resolver_policy: &policy,
        resolver_policy_stamp: &policy_stamp,
        safety_policy: &safety_policy,
        safety_policy_stamp: &safety_policy_stamp,
        abort_signal: &AbortSignal::new(),
        load_paths: &[],
        limits: ScssEvaluationLimits {
            max_output_bytes: 1,
            ..ScssEvaluationLimits::default()
        },
    });
    assert!(byte_limited.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "cem.scss.budget_exceeded"
            && diagnostic.message.contains("max-output-bytes=1")
    }));
    assert!(byte_limited.document.is_none());
}
