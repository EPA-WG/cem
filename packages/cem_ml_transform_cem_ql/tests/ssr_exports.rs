#![cfg(any(target_os = "linux", windows))]
use cem_ml::{
    engine::{
        CemMlEngine, EngineInput, FormatIdentity, TemplateInput, TransformExecutionPolicy,
        TransformRequest, TransformSchedulerScopeIds, TransformTemplateEntrypoint,
        TransformTemplateKind,
    },
    real::RealCemMlEngine,
    run_config::ScopeConfig,
};
use cem_ml_transform_cem_ql::engine_context_with_cem_ql_template_adapter;
use serde_json::{json, Value};
use std::collections::BTreeMap;
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
