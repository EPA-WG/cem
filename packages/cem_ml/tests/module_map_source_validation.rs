use cem_ml::{
    engine::*,
    real::RealCemMlEngine,
    schema::registry::{
        MODULE_MAP_CONTENT_TYPE, MODULE_MAP_SCHEMA_URI, MODULE_MAP_V2_SCHEMA_URI,
        MODULE_MAP_V3_SCHEMA_URI,
    },
};
use serde_json::{json, Value};
const URI: &str = "studio://maps/source.module-map.json";
fn input(text: &str, schema: &str) -> EngineInput {
    EngineInput {
        uri: URI.into(),
        bytes: text.as_bytes().into(),
        from_format: None,
        identity: Some(FormatIdentity {
            content_type: Some(MODULE_MAP_CONTENT_TYPE.into()),
            schema: Some(schema.into()),
            ..Default::default()
        }),
        root_scope: Default::default(),
    }
}
fn diagnostics(source: EngineInput) -> Vec<cem_ml::diagnostics::Diagnostic> {
    RealCemMlEngine::new()
        .validate(ValidateRequest {
            inputs: vec![source],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: EngineContext::default(),
        })
        .unwrap()
        .report
        .diagnostics
}
fn valid(schema: &str) -> Value {
    if schema == MODULE_MAP_V3_SCHEMA_URI {
        json!({"$schema":schema,"imports":{
        "@demo/app":{"path":"never://not-readable/app.js","contentType":"text/javascript","moduleImports":{"@demo/data":"@demo/data"}},
        "@demo/data":{"path":"../data.json","contentType":"application/json"}},
        "resources":{"@demo/view":{"path":"../view.xhtml","contentType":"application/xhtml+xml"}}})
    } else {
        let mut value =
            json!({"$schema":schema,"imports":{"@demo/app":"never://not-readable/app.js"}});
        if schema == MODULE_MAP_V2_SCHEMA_URI {
            value["resources"] =
                json!({"@demo/style":{"path":"../style.css","contentType":"text/css"}});
        }
        value
    }
}
#[test]
fn module_map_selected_versions_refuse_incompatible_schemas_and_repair() {
    for schema in [
        MODULE_MAP_SCHEMA_URI,
        MODULE_MAP_V2_SCHEMA_URI,
        MODULE_MAP_V3_SCHEMA_URI,
    ] {
        let original = valid(schema);
        assert!(diagnostics(input(&original.to_string(), schema)).is_empty());
        for embedded in [
            "https://cem.dev/ns/data/module-map/4",
            if schema == MODULE_MAP_SCHEMA_URI {
                MODULE_MAP_V2_SCHEMA_URI
            } else {
                MODULE_MAP_SCHEMA_URI
            },
        ] {
            let mut broken = original.clone();
            broken["$schema"] = embedded.into();
            let d = diagnostics(input(&broken.to_string(), schema));
            assert!(
                d.iter().any(|d| d.severity.is_hard_violation()
                    && d.code.starts_with("cem.module_map.schema_")),
                "{schema}: {d:?}"
            );
        }
        let mut missing = original.clone();
        missing.as_object_mut().unwrap().remove("$schema");
        assert!(diagnostics(input(&missing.to_string(), schema))
            .iter()
            .any(|d| d.severity.is_hard_violation()));
        assert!(diagnostics(input(&original.to_string(), schema)).is_empty());
    }
}
#[test]
fn module_map_version_specific_types_keys_and_edges_are_semantic_source_errors() {
    let mut cases = Vec::new();
    let mut v = valid(MODULE_MAP_SCHEMA_URI);
    v["imports"] = json!({"./relative":"../app.js"});
    cases.push((v, MODULE_MAP_SCHEMA_URI, "cem.module_map.specifier_invalid"));
    let mut v = valid(MODULE_MAP_SCHEMA_URI);
    v["resources"] = json!({});
    cases.push((
        v,
        MODULE_MAP_SCHEMA_URI,
        "cem.module_map.resources_unsupported",
    ));
    let mut v = valid(MODULE_MAP_SCHEMA_URI);
    v["imports"]["@demo/app"] = "../bad.css".into();
    cases.push((
        v,
        MODULE_MAP_SCHEMA_URI,
        "cem.module_map.asset_type_unsupported",
    ));
    let mut v = valid(MODULE_MAP_V2_SCHEMA_URI);
    v["resources"]["@demo/style"]["contentType"] = "application/xhtml+xml".into();
    cases.push((
        v,
        MODULE_MAP_V2_SCHEMA_URI,
        "cem.module_map.resource_type_unsupported",
    ));
    let mut v = valid(MODULE_MAP_V2_SCHEMA_URI);
    v["resources"]["@demo/style"]["path"] = "../not-css.js".into();
    cases.push((
        v,
        MODULE_MAP_V2_SCHEMA_URI,
        "cem.module_map.resource_type_mismatch",
    ));
    let mut v = valid(MODULE_MAP_V2_SCHEMA_URI);
    v["resources"]["@demo/style"]["extra"] = true.into();
    cases.push((
        v,
        MODULE_MAP_V2_SCHEMA_URI,
        "cem.module_map.resource_entry_invalid",
    ));
    let mut v = valid(MODULE_MAP_V3_SCHEMA_URI);
    v["imports"]["@demo/app"] = "../app.js".into();
    cases.push((
        v,
        MODULE_MAP_V3_SCHEMA_URI,
        "cem.module_map.import_entry_invalid",
    ));
    let mut v = valid(MODULE_MAP_V3_SCHEMA_URI);
    v["imports"]["@demo/app"]["contentType"] = "text/css".into();
    cases.push((
        v,
        MODULE_MAP_V3_SCHEMA_URI,
        "cem.module_map.import_type_unsupported",
    ));
    let mut v = valid(MODULE_MAP_V3_SCHEMA_URI);
    v["imports"]["@demo/app"]["moduleImports"]["@demo/data"] = "@missing/data".into();
    cases.push((
        v,
        MODULE_MAP_V3_SCHEMA_URI,
        "cem.module_map.module_import_target_missing",
    ));
    let mut v = valid(MODULE_MAP_V3_SCHEMA_URI);
    v["imports"]["@demo/app"]["moduleImports"]["@demo/data"] = "@demo/view".into();
    cases.push((
        v,
        MODULE_MAP_V3_SCHEMA_URI,
        "cem.module_map.module_import_target_invalid",
    ));
    let mut v = valid(MODULE_MAP_V3_SCHEMA_URI);
    v["resources"]["@demo/view"]["moduleImports"] = json!({"@demo/data":"@demo/data"});
    cases.push((
        v,
        MODULE_MAP_V3_SCHEMA_URI,
        "cem.module_map.module_import_target_invalid",
    ));
    let mut v = valid(MODULE_MAP_V2_SCHEMA_URI);
    v["resources"]["@demo/app"] = json!({"path":"../duplicate.js","contentType":"text/javascript"});
    cases.push((
        v,
        MODULE_MAP_V2_SCHEMA_URI,
        "cem.module_map.asset_identity_duplicate",
    ));
    for (value, schema, code) in cases {
        let d = diagnostics(input(&value.to_string(), schema));
        assert!(
            d.iter().any(|d| d.code == code
                && d.severity.is_hard_violation()
                && d.uri.as_deref() == Some(URI)),
            "{schema} {code}: {d:?}"
        );
    }
}
#[test]
fn module_map_semantic_diagnostics_keep_original_utf8_key_ranges_and_cross_operation_refusals() {
    let text=format!("{{\r\n \"$schema\":\"{MODULE_MAP_SCHEMA_URI}\",\r\n \"imports\":{{\"@demo/café\":\"../café.js\",\"./bad\":\"../bad.js\"}}\r\n}}");
    let source = input(&text, MODULE_MAP_SCHEMA_URI);
    let d = diagnostics(source.clone());
    let error = d
        .iter()
        .find(|d| d.code == "cem.module_map.specifier_invalid")
        .expect("native key refusal");
    assert_eq!(
        error.byte_offset,
        Some(text.find("\"./bad\"").unwrap() as u64)
    );
    assert_eq!(error.line, Some(3));
    assert!(error
        .source_map
        .as_ref()
        .is_some_and(|m| !m.frames.is_empty()));
    let checked = RealCemMlEngine::new()
        .check(CheckRequest {
            inputs: vec![source.clone()],
            projection: ValidateProjection::Json,
            zero_hard_violations: true,
            fail_level: FailLevel::Validate,
            context: EngineContext::default(),
        })
        .unwrap();
    assert_eq!(checked.hard_violation_count, 1);
    let parsed = RealCemMlEngine::new()
        .parse(ParseRequest {
            input: source.clone(),
            projection: ParseProjection::Ast,
            fail_level: FailLevel::Validate,
            preserve_source_offsets: true,
            presentation_scope: Some(Default::default()),
            context: EngineContext::default(),
        })
        .unwrap();
    assert!(parsed.primary_bytes.is_none());
    assert!(parsed
        .diagnostics
        .iter()
        .any(|d| d.code == error.code && d.byte_offset == error.byte_offset));
    let inspected = RealCemMlEngine::new()
        .inspect(InspectRequest {
            input: source.clone(),
            show: InspectView::Ast,
            presentation_scope: Some(Default::default()),
            context: EngineContext::default(),
        })
        .unwrap();
    assert!(inspected.body.is_null());
    assert!(inspected.diagnostics.iter().any(|d| d.code == error.code));
    let formatted = RealCemMlEngine::new()
        .convert(ConvertRequest {
            target: source.identity.clone(),
            input: source,
            to_format: LayerFormat::Json,
            preserve_source_offsets: true,
            context: EngineContext::default(),
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        })
        .unwrap();
    assert!(formatted.primary_bytes.is_none());
    assert!(formatted.diagnostics.iter().any(|d| d.code == error.code));
}
#[test]
fn map_identity_selection_does_not_elevate_generic_json_or_read_declared_assets() {
    let mut source = input(
        &valid(MODULE_MAP_V3_SCHEMA_URI).to_string(),
        MODULE_MAP_V3_SCHEMA_URI,
    );
    assert!(
        diagnostics(source.clone()).is_empty(),
        "unreadable never:// and relative assets are inert source values"
    );
    source.identity.as_mut().unwrap().schema = None;
    assert!(
        diagnostics(source.clone())
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "MIME-only selects v1, not embedded v3"
    );
    source.identity.as_mut().unwrap().content_type = None;
    source.identity.as_mut().unwrap().schema = Some(MODULE_MAP_V3_SCHEMA_URI.into());
    assert!(diagnostics(source.clone()).is_empty());
    source.identity = Some(FormatIdentity {
        content_type: Some("application/json".into()),
        schema: Some("https://cem.dev/ns/data/json/1".into()),
        ..Default::default()
    });
    source.bytes =
        br#"{"$schema":"https://cem.dev/ns/data/module-map/4","imports":{"./bad":false}}"#.to_vec();
    assert!(
        diagnostics(source).is_empty(),
        "generic JSON remains literal data"
    );
}
