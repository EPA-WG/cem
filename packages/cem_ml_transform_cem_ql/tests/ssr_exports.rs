#![cfg(any(target_os = "linux", windows))]
use cem_ml::{
    engine::{
        CemMlEngine, EngineInput, FormatIdentity, TemplateInput, TransformExecutionPolicy,
        TransformRequest, TransformSchedulerScopeIds, TransformTemplateEntrypoint,
        TransformTemplateKind,
    },
    operation_control::OperationControl,
    real::RealCemMlEngine,
    resolver::{
        ResolveDirection, ResolvePolicyDenial, ResolvePolicyRequestKey, ResolvePurpose,
        ResolveRequest, ResolvedRead, ResolvedWrite, ResolverDiagnostic, ResourceResolver,
        SsrReadBudget,
    },
    run_config::ScopeConfig,
};
use cem_ml_transform_cem_ql::engine_context_with_cem_ql_template_adapter;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
const TEMPLATE: &str = include_str!("../templates/ssr-page.cemt");
fn request(target: &str) -> TransformRequest {
    let identity = |t: &str| FormatIdentity {
        content_type: Some(t.into()),
        ..Default::default()
    };
    TransformRequest {
        data: EngineInput { uri: "file:///source/page.html".into(), bytes: br#"<!doctype html><html><head><title>Ready</title></head><body><cem-element tag="x-parent" version="1.2.3"><template id="x-parent" type="text/cem-ml">{section | {x-child}}</template></cem-element><cem-element tag="x-child" version="0.2.3"><template id="x-child" type="text/cem-ml">{b | Ready &amp; safe}</template></cem-element><x-parent></x-parent></body></html>"#.to_vec(), from_format: None, identity: Some(identity("text/html")), root_scope: ScopeConfig::default() },
        template: TemplateInput { uri: "file:///builtin/ssr-page.cemt".into(), bytes: TEMPLATE.as_bytes().to_vec(), identity: Some(identity("text/cem-ml")), root_scope: ScopeConfig::default() },
        template_kind: TransformTemplateKind::CemNative,
        template_entrypoint: TransformTemplateEntrypoint::named("ssr:render-page"),
        params: BTreeMap::from([("page-url".into(), json!("https://ssr.test/guide/")), ("consumer-versions".into(), json!({"x-parent":"1.9.0", "x-child":"0.2.9"})), ("max-depth".into(),json!(8)), ("max-instances".into(),json!(8)), ("max-render-passes".into(),json!(8)), ("max-resources".into(),json!(16)), ("max-bytes".into(),json!(1048576)), ("wall-ms".into(),json!(60000)), ("cpu-ms".into(),json!(10000))]),
        preserve_source_offsets: true,
        context: engine_context_with_cem_ql_template_adapter(),
        target: Some(identity(target)), target_scope: ScopeConfig::default(),
        scheduler_scope_ids: TransformSchedulerScopeIds {data_load:10,template_load:11,execution:12,output:13},
        execution_policy: TransformExecutionPolicy { runtime_phase: cem_ml::engine::TransformRuntimePhase::CemNativeModules, ..Default::default() },
    }
}
#[test]
fn named_html_export_renders_nested_native_participants_and_retains_islands() {
    let response = RealCemMlEngine::new()
        .transform(request("text/html"))
        .unwrap();
    assert!(
        !response
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        response.diagnostics
    );
    let html = response.primary.as_str().unwrap();
    assert!(html.contains("data-cem-island="), "{html}");
    assert!(
        html.contains("data-cem-ssr-profile=\"cem-native-inline/1\""),
        "{html}"
    );
    assert!(html.contains("<section>"), "{html}");
    assert!(html.contains("<b>Ready"), "{html}");
    assert!(!html.contains('\u{1b}'));
    assert!(!html.contains("cem-syntax"));
    assert!(html.starts_with("<!doctype html>"), "{html}");
    assert!(html.contains("<cem-island:context-root"), "{html}");
    assert!(!html.contains("<http:"));
}

#[test]
fn authored_base_keeps_the_ordinary_html_source_policy() {
    use cem_ml::validation::html::{validate_html_source_bytes, HtmlSourceValidationRequest};

    let diagnostics = validate_html_source_bytes(HtmlSourceValidationRequest {
        bytes: br#"<!doctype html><html><head><base href="/assets/"></head><body></body></html>"#,
        source_uri: "file:///source/page.html",
        content_type: Some("text/html"),
    });
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "cem.html.external_resource_rejected"
            && diagnostic.severity.is_hard_violation()
    }));
}

#[test]
fn named_page_profiles_render_from_authored_sources() {
    use cem_ml::source_map::{FrameSpan, TransformKind};

    let declaration = r#"{cem-element @tag=x-parent @version="1.2.3" | {template @id=x-parent @type="text/cem-ml" |```{article | Published}```}}{x-parent}"#;
    let cem = format!(
        r#"@doc cem-ml 1
@ns svg = "http://www.w3.org/2000/svg"
{{html | {{head | {{base @href="/assets/"}}{{title | CEM page}}}}{{body |
  {{svg:svg @viewBox="0 0 24 24" | {{svg:title | Icon}}{{svg:circle @r=8}}}}
  {{p | From source}}{declaration}
}}}}"#
    );
    let markdown = format!("# Native page\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n\n```cem-ml\n{{p | From source}}{declaration}\n```\n\n```\n{{p | Literal}}\n```\n");
    for (suffix, content_type, schema, source) in [
        (
            "cem",
            cem_ml::ssr_route::SSR_CEM_PAGE_CONTENT_TYPE,
            cem_ml::ssr_route::SSR_CEM_PAGE_SCHEMA,
            &cem,
        ),
        (
            "cem.md",
            cem_ml::ssr_route::SSR_MARKDOWN_PAGE_CONTENT_TYPE,
            cem_ml::ssr_route::SSR_MARKDOWN_PAGE_SCHEMA,
            &markdown,
        ),
    ] {
        let mut r = request("text/html");
        r.params
            .insert("consumer-versions".into(), json!({"x-parent": "1.9.0"}));
        let uri = format!("file:///source/page.{suffix}");
        r.data.uri = uri.clone();
        r.data.bytes = source.as_bytes().to_vec();
        r.data.identity = Some(FormatIdentity {
            content_type: Some(content_type.into()),
            schema: Some(schema.into()),
            ..Default::default()
        });
        let response = RealCemMlEngine::new().transform(r).unwrap();
        let html = response
            .primary
            .as_str()
            .unwrap_or_else(|| panic!("{suffix}: {:?}", response.diagnostics));
        assert!(
            html.contains("<p>From source</p>") && html.contains("<article>Published</article>"),
            "{html}"
        );
        assert!(html.starts_with("<!doctype html>"), "{html}");
        let text_offset = html.find("From source").unwrap() as u64;
        let origin = response
            .output_spans
            .iter()
            .find(|span| {
                span.output_range.start <= text_offset && span.output_range.end() > text_offset
            })
            .expect("written text retains its source range");
        let offset = source.find("From source").unwrap() as u64;
        let includes = |span: &FrameSpan| {
            let ranges = match span {
                FrameSpan::Single(range) => std::slice::from_ref(range),
                FrameSpan::Multi(ranges) => ranges,
            };
            ranges
                .iter()
                .any(|range| range.start <= offset && range.end() > offset)
        };
        assert!(origin.origin.frames.iter().any(|frame| {
            matches!(&frame.transform, TransformKind::SourceBoundary { uri: origin } if *origin == uri)
                && includes(&frame.span)
        }), "{:?}", origin.origin);
        if suffix == "cem" {
            assert!(
                html.contains(r#"<base href="/assets/">"#)
                    && html.contains(r#"<svg viewBox="0 0 24 24">"#),
                "{html}"
            );
        }
        if suffix == "cem.md" {
            assert!(
                html.contains("<pre><code>{p | Literal}")
                    && html.contains("<th>A</th>")
                    && html.contains("<td>2</td>"),
                "{html}"
            );
            assert!(origin.origin.frames.iter().any(|frame| {
                matches!(&frame.transform, TransformKind::SourceBoundary { uri: origin } if origin.starts_with(&format!("{uri}#cem-ml-fence-")))
            }));
        }
    }
}

#[test]
fn markdown_page_cpu_exhaustion_precedes_route_and_unsafe_markup_is_refused() {
    let mut r = request("text/html");
    r.data.uri = "file:///source/page.cem.md".into();
    r.data.identity = Some(FormatIdentity {
        content_type: Some(cem_ml::ssr_route::SSR_MARKDOWN_PAGE_CONTENT_TYPE.into()),
        schema: Some(cem_ml::ssr_route::SSR_MARKDOWN_PAGE_SCHEMA.into()),
        ..Default::default()
    });
    let mut expensive = r.clone();
    expensive.data.bytes = "# Repeated paragraph\n\n".repeat(35_000).into_bytes();
    expensive.params.insert("cpu-ms".into(), json!(1));
    expensive
        .params
        .insert("page-url".into(), json!("invalid route"));
    let refused = RealCemMlEngine::new().transform(expensive).unwrap();
    assert!(refused.primary.is_null());
    assert!(
        refused.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.ssr.page_ingress"
                && diagnostic.message.contains("CPU budget exhausted")
        }),
        "{:?}",
        refused.diagnostics
    );

    for source in [
        "<script>throw 1</script>\n",
        "```cem-ml\n{iframe @srcdoc='unsafe'}\n```\n",
        "[unsafe](javascript:alert)\n",
    ] {
        let mut r = r.clone();
        r.context.operation_control = OperationControl::default();
        r.data.bytes = source.as_bytes().to_vec();
        let output = RealCemMlEngine::new().transform(r).unwrap();
        assert!(output.primary.is_null());
        assert!(
            output.diagnostics.iter().any(|diagnostic| {
                matches!(
                    diagnostic.code.as_str(),
                    "cem.ssr.page_ingress"
                        | "cem.markdown.embedded_html_rejected"
                        | "cem.unsafe.srcdoc"
                )
            }),
            "{:?}",
            output.diagnostics
        );
    }
}

#[test]
fn deeply_nested_cem_page_refuses_before_recursive_projection() {
    let mut r = request("text/html");
    r.data.uri = "file:///source/deep.cem".into();
    r.data.identity = Some(FormatIdentity {
        content_type: Some(cem_ml::ssr_route::SSR_CEM_PAGE_CONTENT_TYPE.into()),
        schema: Some(cem_ml::ssr_route::SSR_CEM_PAGE_SCHEMA.into()),
        ..Default::default()
    });
    r.data.bytes = format!(
        "{{html | {{head}}{{body | {}{} }}}}",
        "{div | ".repeat(80),
        "}".repeat(80)
    )
    .into_bytes();
    let response = RealCemMlEngine::new().transform(r).unwrap();
    assert!(response.primary.is_null());
    assert!(
        response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("native document nesting limit")),
        "{:?}",
        response.diagnostics
    );
}

#[test]
fn html_parser_can_exhaust_the_original_cpu_budget_before_route_preflight() {
    let mut r = request("text/html");
    let html = format!(
        "<!doctype html><html><head><title>CPU</title></head><body>{}</body></html>",
        "<p>x</p>".repeat(80_000),
    );
    r.data.bytes = html.into_bytes();
    r.params.insert("cpu-ms".into(), json!(1));
    r.params.insert("page-url".into(), json!("invalid route"));
    let response = RealCemMlEngine::new().transform(r).unwrap();
    assert!(response.primary.is_null());
    assert!(
        response.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.ssr.page_ingress"
                && diagnostic.message.contains("CPU budget exhausted")
        }),
        "{:?}",
        response.diagnostics
    );
    assert!(!response
        .diagnostics
        .iter()
        .any(|diagnostic| { diagnostic.message.contains("cem.ssr.page_url_invalid") }));
}

#[test]
fn immutable_builtin_and_no_transport_profile_cannot_be_bypassed() {
    let mut edited = request("text/html");
    edited.template.bytes.push(b'\n');
    let response = RealCemMlEngine::new().transform(edited).unwrap();
    assert!(response
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation()));
    let mut external = request("text/html");
    external.data.bytes = String::from_utf8(external.data.bytes)
        .unwrap()
        .replace(
            "{section | {x-child}}",
            "{style |```@import 'external.css';```}{section | {x-child}}",
        )
        .into_bytes();
    let response = RealCemMlEngine::new().transform(external).unwrap();
    assert!(response
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation()));
}

struct MeasuredStylesheet {
    original: OperationControl,
    burn_ms: u64,
    reads: Arc<AtomicU64>,
    completed_cpu_ms: Arc<AtomicU64>,
}

impl ResourceResolver for MeasuredStylesheet {
    fn read(&self, _: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        panic!("SSR must use the bounded resolver entrypoint");
    }
    fn read_ssr_resource(
        &self,
        request: &ResolveRequest,
        budget: &SsrReadBudget<'_>,
    ) -> Result<ResolvedRead, ResolverDiagnostic> {
        assert!(self.original.same_operation(budget.control));
        assert_eq!(request.uri, "https://ssr.test/guide/external.css");
        assert_eq!(request.base_uri.as_deref(), Some("https://ssr.test/guide/"));
        assert!(budget.max_bytes < 1_048_576);
        self.reads.fetch_add(1, Ordering::Relaxed);
        let target = budget.cpu.usage_ms(budget.control).unwrap() + self.burn_ms;
        while budget
            .cpu
            .usage_ms(budget.control)
            .is_ok_and(|usage| usage < target)
        {
            std::hint::spin_loop();
        }
        budget.check(request)?;
        let bytes = budget.read_from(request, b"section { color: red }".as_slice())?;
        self.completed_cpu_ms.store(
            budget.cpu.usage_ms(budget.control).unwrap(),
            Ordering::Relaxed,
        );
        Ok(ResolvedRead {
            uri: request.uri.clone(),
            bytes,
            content_type: Some("text/css".into()),
        })
    }
    fn write(&self, _: &ResolveRequest, _: &[u8]) -> Result<ResolvedWrite, ResolverDiagnostic> {
        panic!("SSR stylesheet acquisition must not write");
    }
}

#[test]
fn stylesheet_acquisition_cpu_is_in_the_original_report_and_can_exhaust_it() {
    for (burn, limit, denied) in [(12, 10_000, false), (150, 100, false), (12, 10_000, true)] {
        let mut r = request("application/vnd.cem.ssr-report+json");
        r.data.bytes = String::from_utf8(r.data.bytes)
            .unwrap()
            .replace(
                "{section | {x-child}}",
                "{style |```@import 'external.css';```}{section | {x-child}}",
            )
            .into_bytes();
        r.params.insert("cpu-ms".into(), json!(limit));
        let reads = Arc::new(AtomicU64::new(0));
        let completed = Arc::new(AtomicU64::new(0));
        r.context.resolver_registry.register(
            "https",
            ResolvePurpose::SsrResource,
            ResolveDirection::Read,
            MeasuredStylesheet {
                original: r.context.operation_control.clone(),
                burn_ms: burn,
                reads: reads.clone(),
                completed_cpu_ms: completed.clone(),
            },
        );
        if denied {
            r.context.resolver_policy.register_denial(
                ResolvePolicyRequestKey::new(
                    "https://ssr.test/guide/external.css",
                    ResolvePurpose::SsrResource,
                    ResolveDirection::Read,
                ),
                ResolvePolicyDenial::new("fixture denies this stylesheet"),
            );
        }
        let output = RealCemMlEngine::new().transform(r).unwrap();
        let diagnostics = output
            .diagnostics
            .iter()
            .map(|d| d.message.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        if denied {
            assert_eq!(reads.load(Ordering::Relaxed), 0);
            assert!(
                diagnostics.contains("fixture denies this stylesheet"),
                "{diagnostics}"
            );
        } else {
            assert_eq!(reads.load(Ordering::Relaxed), 1, "{diagnostics}");
            let completed = completed.load(Ordering::Relaxed);
            if burn > limit {
                assert_eq!(completed, 0);
                assert!(diagnostics.contains("CPU"), "{diagnostics}");
            } else {
                assert!(completed >= burn);
                assert!(output.primary["usage"]["cpuTimeMs"].as_u64().unwrap() >= completed);
            }
        }
    }
}

#[test]
fn inline_css_and_inert_native_payloads_export_through_closed_source_leases() {
    let mut r = request("text/html");
    r.data.bytes = String::from_utf8(r.data.bytes)
        .unwrap()
        .replace(
            "{b | Ready &amp; safe}",
            "{style |```b {color:red}```}{b | Ready &amp; safe}",
        )
        .replace(
            "<x-parent></x-parent>",
            "<x-parent><template type='cem-ml'>{item | {$ 1 div 0}}</template></x-parent>",
        )
        .into_bytes();
    let response = RealCemMlEngine::new().transform(r).unwrap();
    assert!(
        !response
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        response.diagnostics
    );
    let html = response.primary.as_str().unwrap();
    assert!(html.contains("color:red"), "{html}");
    assert!(html.contains("1 div 0"), "{html}");
    assert!(html.contains("data-cem-island="));
}

#[test]
fn html_export_preserves_literal_script_svg_and_textarea_without_source_coloring() {
    let mut r = request("text/html");
    r.data.bytes = String::from_utf8(r.data.bytes)
        .unwrap()
        .replace(
            "</head>",
            "<script type=\"text/plain\">if (a < b) { c = '&'; }</script></head>",
        )
        .replace(
            "</body>",
            "<svg><use xlink:href='#shape'/></svg><textarea>&lt;literal&gt;</textarea></body>",
        )
        .into_bytes();
    let response = RealCemMlEngine::new().transform(r).unwrap();
    assert!(
        !response
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        response.diagnostics
    );
    let html = response.primary.as_str().unwrap();
    assert!(
        html.contains("<script type=\"text/plain\">if (a < b) { c = '&'; }</script>"),
        "{html}"
    );
    assert!(html.contains("<svg><use xlink:href=\"#shape\""), "{html}");
    assert!(
        html.contains("<textarea>&lt;literal&gt;</textarea>"),
        "{html}"
    );
}
#[test]
fn named_report_export_reports_fresh_native_readiness_without_hydration_authority() {
    let response = RealCemMlEngine::new()
        .transform(request("application/vnd.cem.ssr-report+json"))
        .unwrap();
    assert!(
        !response
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        response.diagnostics
    );
    let report: Value = response.primary;
    assert_eq!(report["readiness"], "native-ready");
    assert_eq!(report["hydrationReady"], false);
    assert_eq!(report["participants"].as_array().unwrap().len(), 2);
    assert_eq!(report["route"]["pageUrl"], "https://ssr.test/guide/");
    assert_eq!(report["route"]["sourceUri"], "file:///source/page.html");
}
#[test]
fn readiness_and_output_controls_refuse_before_encoding() {
    for (key, value) in [
        ("page-url", json!("relative")),
        ("consumer-versions", json!({"x-parent":"2.0.0"})),
        ("max-instances", json!(1)),
        ("max-bytes", json!(1)),
        ("cpu-ms", json!(0)),
    ] {
        let mut r = request("text/html");
        r.params.insert(key.into(), value);
        let result = RealCemMlEngine::new().transform(r);
        assert!(
            result.is_err()
                || result
                    .unwrap()
                    .diagnostics
                    .iter()
                    .any(|d| d.severity.is_hard_violation()),
            "accepted {key}"
        );
    }
    let result = RealCemMlEngine::new().transform(request("application/json"));
    assert!(
        result.is_err()
            || result
                .unwrap()
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation())
    );
}
