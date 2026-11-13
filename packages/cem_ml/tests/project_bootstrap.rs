use base64::Engine;
use cem_ml::project_bootstrap::project_bootstrap_json;
use cem_ml::studio_project::{
    parse_studio_project, StudioProjectProjection, STUDIO_PROJECT_CEM_CONTENT_TYPE,
    STUDIO_PROJECT_SCHEMA_URI,
};
use serde_json::{json, Value};

fn call(request: Value) -> Value {
    serde_json::from_str(&project_bootstrap_json(&request.to_string())).unwrap()
}
fn configuration(preset: &str, id: &str) -> Value {
    let response = call(json!({"action":"preset", "preset":preset, "projectId":id,
        "name":"New project", "createdAt":"2026-11-13T00:00:00Z"}));
    assert!(response.get("error").is_none(), "{response}");
    response["configuration"].clone()
}
fn generated(config: Value) -> Value {
    let response = call(json!({"action":"generate", "configuration":config}));
    assert!(response.get("error").is_none(), "{response}");
    response
}

#[test]
fn all_presets_produce_roundtrippable_projects_with_exact_bytes() {
    let catalog = call(json!({"action":"catalog"}));
    assert_eq!(catalog["presets"].as_array().unwrap().len(), 5);
    assert_eq!(
        catalog["configurationSchema"]["properties"]["schemaVersion"]["const"],
        1
    );
    for preset in catalog["presets"].as_array().unwrap() {
        let config = configuration(preset["id"].as_str().unwrap(), "new-project");
        let result = generated(config.clone());
        let bundle = &result["bundle"];
        assert_eq!(bundle["archiveSchemaVersion"], 1);
        let project = parse_studio_project(
            result["manifest"].as_str().unwrap().as_bytes(),
            STUDIO_PROJECT_CEM_CONTENT_TYPE,
            STUDIO_PROJECT_SCHEMA_URI,
        )
        .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(
                &project.serialize(StudioProjectProjection::Json).unwrap()
            )
            .unwrap(),
            bundle["project"]
        );
        for (resource, content) in project
            .resources
            .iter()
            .zip(bundle["contents"].as_array().unwrap())
        {
            assert_eq!(content["resourceId"], resource.id);
            assert_eq!(
                resource.reference_path.as_deref(),
                Some(format!(".resources/{}", resource.id).as_str())
            );
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(content["base64"].as_str().unwrap())
                .unwrap();
            let original = config["resources"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["id"] == resource.id)
                .unwrap();
            assert_eq!(bytes, original["text"].as_str().unwrap().as_bytes());
            assert_eq!(resource.sha256, cem_ml::command_service::sha256_hex(&bytes));
        }
    }
}

#[test]
fn generation_is_deterministic_and_new_project_ids_have_independent_roots() {
    let config = configuration("transformation-pipeline", "first");
    let first = generated(config.clone());
    assert_eq!(first, generated(config.clone()));
    let mut second_config = config;
    second_config["projectId"] = "second".into();
    for r in second_config["resources"].as_array_mut().unwrap() {
        r["path"] = format!("renamed/{}.source", r["id"].as_str().unwrap()).into();
    }
    let second = generated(second_config);
    assert_ne!(
        first["bundle"]["project"]["rootUri"],
        second["bundle"]["project"]["rootUri"]
    );
    assert_eq!(first["bundle"]["contents"], second["bundle"]["contents"]);
}

#[test]
fn invalid_configuration_never_returns_a_partial_bundle() {
    for (field, value) in [
        ("schemaVersion", json!(2)),
        ("commonVersion", json!("999.0.0")),
        ("projectId", json!("../outside")),
        ("createdAt", json!("yesterday")),
        ("unexpected", json!(true)),
    ] {
        let mut config = configuration("simple-conversion", "invalid");
        config[field] = value;
        let result = call(json!({"action":"generate", "configuration":config}));
        assert!(result.get("error").is_some(), "{field}: {result}");
        assert!(result.get("bundle").is_none());
    }
    for path in [
        "../escape",
        "project.cem",
        "data/../escape",
        "data\\bad",
        "data//bad",
    ] {
        let mut config = configuration("simple-conversion", "invalid");
        config["resources"][0]["path"] = path.into();
        assert!(
            call(json!({"action":"generate", "configuration":config}))
                .get("error")
                .is_some(),
            "{path}"
        );
    }
}

#[test]
fn graph_dependencies_and_output_paths_are_preflighted_by_native_contracts() {
    for (from, to) in [
        ("@src=source", "@src=missing"),
        ("@src=source", "@src=https://example.org/source"),
        ("../generated/result.html", "../project.cem"),
        ("@id=view", "@id=view @input=missing"),
        ("@id=view", "@id=view @input=page"),
    ] {
        let mut config = configuration("transformation-pipeline", "invalid-graph");
        let graph = config["resources"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["role"] == "graph")
            .unwrap();
        let source = graph["text"].as_str().unwrap();
        assert!(source.contains(from), "{from}: {source}");
        graph["text"] = source.replace(from, to).into();
        let result = call(json!({"action":"generate", "configuration":config}));
        assert!(result.get("error").is_some(), "{from} -> {to}: {result}");
        assert!(result.get("bundle").is_none());
    }
}

#[test]
fn graph_failures_retain_native_diagnostics_and_resource_identity() {
    let mut config = configuration("transformation-pipeline", "invalid-graph");
    let graph = config["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["role"] == "graph")
        .unwrap();
    graph["text"] = graph["text"]
        .as_str()
        .unwrap()
        .replace("@id=view", "@id=view @input=missing")
        .into();
    let response = call(json!({"action":"generate", "configuration":config}));
    assert_eq!(
        response["error"]["code"],
        "cem.project_bootstrap.graph_invalid"
    );
    assert_eq!(response["error"]["fieldPath"], "resources[2].text");
    assert!(response["error"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["code"] == "cem.transform_config.ref_unknown"
            && d["uri"] == "studio://invalid-graph/.resources/pipeline"));
}

#[test]
fn duplicate_hierarchy_and_forward_or_oversized_requests_are_rejected() {
    for field in ["id", "path"] {
        let mut config = configuration("simple-conversion", "duplicate");
        config["resources"][1][field] = config["resources"][0][field].clone();
        assert!(call(json!({"action":"generate", "configuration":config}))
            .get("error")
            .is_some());
    }
    let mut config = configuration("simple-conversion", "duplicate");
    config["resources"][1]["path"] = "data/source.yaml/child".into();
    assert!(call(json!({"action":"generate", "configuration":config}))
        .get("error")
        .is_some());
    assert!(call(json!({"action":"catalog","unexpected":true}))
        .get("error")
        .is_some());
    assert!(call(json!({"action":"future"})).get("error").is_some());
    let response: Value = serde_json::from_str(&project_bootstrap_json(
        &" ".repeat(cem_ml::project_bootstrap::PROJECT_BOOTSTRAP_MAX_REQUEST_BYTES + 1),
    ))
    .unwrap();
    assert_eq!(
        response["error"]["code"],
        "cem.project_bootstrap.request_limit"
    );
}
