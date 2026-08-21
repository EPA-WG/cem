use cem_ml::schema::package_sources::{
    builtin_schema_package_artifact_source, builtin_schema_package_source,
};
use cem_ml::schema::registry::{
    schema_package_examples_from_package_sources, SchemaPackageExampleExpectedResult,
};
use cem_ml::studio_project::{
    parse_studio_project, studio_project_resource_uri, StudioProjectProjection,
    STUDIO_PROJECT_CEM_CONTENT_TYPE, STUDIO_PROJECT_JSON_CONTENT_TYPE,
    STUDIO_PROJECT_JSON_SCHEMA_URI, STUDIO_PROJECT_SCHEMA_URI,
};

const VALID_CEM: &[u8] =
    include_bytes!("../schema-packages/studio-project/v1/examples/feature-tour.project.cem");
const VALID_JSON: &[u8] =
    include_bytes!("../schema-packages/studio-project/v1/examples/feature-tour.project.json");
const STUDIO_PROJECT_RUST_SOURCE: &str = include_str!("../src/studio_project.rs");
const STUDIO_PROJECT_CEM_SCHEMA_SOURCE: &str =
    include_str!("../schema-packages/studio-project/v1/schema/studio-project.cem");

#[test]
fn studio_project_v1_identities_are_fixed() {
    assert_eq!(
        STUDIO_PROJECT_CEM_CONTENT_TYPE,
        "application/vnd.cem.studio-project+cem"
    );
    assert_eq!(
        STUDIO_PROJECT_JSON_CONTENT_TYPE,
        "application/vnd.cem.studio-project+json"
    );
    assert_eq!(
        STUDIO_PROJECT_SCHEMA_URI,
        "https://cem.dev/ns/studio/project/1"
    );
    assert_eq!(
        STUDIO_PROJECT_JSON_SCHEMA_URI,
        "https://cem.dev/schema/studio/project.schema.json"
    );
}

#[test]
fn studio_project_package_examples_and_json_schema_are_manifest_indexed() {
    let package = builtin_schema_package_source("studio-project")
        .expect("built-in Studio project schema package");
    assert_eq!(
        package.schema_path,
        "schema-packages/studio-project/v1/schema/studio-project.cem"
    );

    let examples =
        schema_package_examples_from_package_sources(package).expect("Studio project examples");
    assert_eq!(examples.len(), 6);
    assert!(examples
        .iter()
        .all(|example| example.schema == STUDIO_PROJECT_SCHEMA_URI));
    assert_eq!(
        examples
            .iter()
            .filter(|example| {
                example.expected_result == SchemaPackageExampleExpectedResult::Pass
            })
            .count(),
        2
    );

    let artifact = builtin_schema_package_artifact_source(
        "studio-project",
        "schema-packages/studio-project/v1/schema/studio-project.schema.json",
    )
    .expect("embedded Studio project JSON Schema");
    let schema: serde_json::Value =
        serde_json::from_str(artifact.source).expect("valid Studio project JSON Schema");
    assert_eq!(
        schema.get("$id").and_then(serde_json::Value::as_str),
        Some(STUDIO_PROJECT_JSON_SCHEMA_URI)
    );
}

#[test]
fn studio_project_schema_owns_every_native_diagnostic_code() {
    let native_codes = studio_project_diagnostic_codes(STUDIO_PROJECT_RUST_SOURCE);
    let schema_codes = studio_project_diagnostic_codes(STUDIO_PROJECT_CEM_SCHEMA_SOURCE);
    assert_eq!(native_codes, schema_codes);
}

fn studio_project_diagnostic_codes(source: &str) -> std::collections::BTreeSet<&str> {
    source
        .split('"')
        .filter(|value| value.starts_with("cem.studio_project."))
        .collect()
}

#[test]
fn studio_project_cem_and_json_fixtures_normalize_to_one_model() {
    let cem = parse_studio_project(
        VALID_CEM,
        STUDIO_PROJECT_CEM_CONTENT_TYPE,
        STUDIO_PROJECT_SCHEMA_URI,
    )
    .expect("valid CEM Studio project");
    let json = parse_studio_project(
        VALID_JSON,
        STUDIO_PROJECT_JSON_CONTENT_TYPE,
        STUDIO_PROJECT_SCHEMA_URI,
    )
    .expect("valid JSON Studio project");

    assert_eq!(cem, json);
    assert_eq!(cem.schema_version, 1);
    assert_eq!(cem.id, "feature-tour");
    assert_eq!(cem.entries.len(), 2);
    assert_eq!(cem.resources.len(), 2);
}

#[test]
fn studio_project_cem_requires_the_registered_namespace_as_default() {
    for source in [
        String::from_utf8(VALID_CEM.to_vec())
            .expect("UTF-8 fixture")
            .replace(STUDIO_PROJECT_SCHEMA_URI, "https://example.invalid/studio"),
        String::from_utf8(VALID_CEM.to_vec())
            .expect("UTF-8 fixture")
            .replace("@default studio", "@default other"),
    ] {
        let error = parse_studio_project(
            source.as_bytes(),
            STUDIO_PROJECT_CEM_CONTENT_TYPE,
            STUDIO_PROJECT_SCHEMA_URI,
        )
        .expect_err("CEM project namespace identity must be explicit and canonical");
        assert_eq!(error.code, "cem.studio_project.schema_identity_unsupported");
    }
}

#[test]
fn studio_project_json_requires_normalized_collection_fields() {
    let mut project: serde_json::Value =
        serde_json::from_slice(VALID_JSON).expect("valid JSON fixture");
    project
        .as_object_mut()
        .expect("project object")
        .remove("entries");
    let bytes = serde_json::to_vec(&project).expect("JSON bytes");

    let error = parse_studio_project(
        &bytes,
        STUDIO_PROJECT_JSON_CONTENT_TYPE,
        STUDIO_PROJECT_SCHEMA_URI,
    )
    .expect_err("normalized JSON project must declare entries and resources");
    assert_eq!(error.code, "cem.studio_project.invalid_json");
}

#[test]
fn studio_project_projections_round_trip_deterministically() {
    let project = parse_studio_project(
        VALID_CEM,
        STUDIO_PROJECT_CEM_CONTENT_TYPE,
        STUDIO_PROJECT_SCHEMA_URI,
    )
    .expect("valid CEM Studio project");

    for projection in [StudioProjectProjection::Cem, StudioProjectProjection::Json] {
        let first = project
            .serialize(projection)
            .expect("Studio project projection");
        let reparsed = parse_studio_project(
            first.as_bytes(),
            projection.content_type(),
            STUDIO_PROJECT_SCHEMA_URI,
        )
        .expect("serialized Studio project parses");
        let second = reparsed
            .serialize(projection)
            .expect("re-serialized Studio project projection");
        assert_eq!(project, reparsed);
        assert_eq!(first, second);
    }
}

#[test]
fn studio_project_logical_uri_is_derived_from_root_and_safe_path() {
    let project = parse_studio_project(
        VALID_JSON,
        STUDIO_PROJECT_JSON_CONTENT_TYPE,
        STUDIO_PROJECT_SCHEMA_URI,
    )
    .expect("valid JSON Studio project");

    assert_eq!(
        studio_project_resource_uri(&project, "tour-source").as_deref(),
        Ok("studio://feature-tour/data/tour.cem")
    );
}

#[test]
fn studio_project_rejects_forward_versions_escaping_paths_and_forbidden_state() {
    for (path, content_type, code) in [
        (
            "schema-packages/studio-project/v1/examples/invalid-forward-version.project.json",
            STUDIO_PROJECT_JSON_CONTENT_TYPE,
            "cem.studio_project.schema_version_unsupported",
        ),
        (
            "schema-packages/studio-project/v1/examples/invalid-escaping-path.project.json",
            STUDIO_PROJECT_JSON_CONTENT_TYPE,
            "cem.studio_project.resource_path_invalid",
        ),
        (
            "schema-packages/studio-project/v1/examples/invalid-forbidden-state.project.json",
            STUDIO_PROJECT_JSON_CONTENT_TYPE,
            "cem.studio_project.invalid_json",
        ),
        (
            "schema-packages/studio-project/v1/examples/invalid-duplicate-id.project.cem",
            STUDIO_PROJECT_CEM_CONTENT_TYPE,
            "cem.studio_project.id_duplicate",
        ),
    ] {
        let bytes = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
            .expect("Studio project rejection fixture");
        let error = parse_studio_project(&bytes, content_type, STUDIO_PROJECT_SCHEMA_URI)
            .expect_err("Studio project rejection fixture must fail");
        assert_eq!(error.code, code, "fixture {path}: {error}");
    }
}

#[test]
fn studio_project_rejects_unknown_schema_and_content_type() {
    let schema_error = parse_studio_project(
        VALID_JSON,
        STUDIO_PROJECT_JSON_CONTENT_TYPE,
        "https://cem.dev/ns/studio/project/2",
    )
    .expect_err("forward schema identity must fail");
    assert_eq!(
        schema_error.code,
        "cem.studio_project.schema_identity_unsupported"
    );

    let content_error =
        parse_studio_project(VALID_JSON, "application/json", STUDIO_PROJECT_SCHEMA_URI)
            .expect_err("generic JSON must not silently select the Studio project model");
    assert_eq!(
        content_error.code,
        "cem.studio_project.content_type_unsupported"
    );
}
