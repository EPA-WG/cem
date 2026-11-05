use cem_ml::schema::document_model::compile_schema_document_model;
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
fn studio_project_schema_definition_compiles_without_diagnostics() {
    let model =
        compile_schema_document_model(STUDIO_PROJECT_SCHEMA_URI, STUDIO_PROJECT_CEM_SCHEMA_SOURCE);
    assert!(
        model.compile_diagnostics.is_empty(),
        "{:#?}",
        model.compile_diagnostics
    );
    assert!(model.is_ready_for_validation());
}

fn studio_project_schema_diagnostics(source: &str) -> Vec<cem_ml::diagnostics::Diagnostic> {
    use cem_ml::{
        events::cem::CemEventNormalizer,
        parser::builder::CemAstBuilder,
        schema::document_model::validate_document_model,
        source::{BytesSource, SourceId},
        tokenizer::cem::CemTokenizer,
    };
    let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), source.as_bytes().to_vec()),
    )))
    .build();
    assert!(
        document.diagnostics.is_empty(),
        "{:?}",
        document.diagnostics
    );
    let model =
        compile_schema_document_model(STUDIO_PROJECT_SCHEMA_URI, STUDIO_PROJECT_CEM_SCHEMA_SOURCE);
    validate_document_model(&document, &model)
}

#[test]
fn studio_project_schema_stable_ids_match_the_portable_codec() {
    let max_id = "a".repeat(64);
    let overlong = "a".repeat(65);
    for (id, valid) in [
        ("0", true),
        ("9-project", true),
        ("a-", true),
        (&max_id, true),
        ("", false),
        ("-a", false),
        ("Upper", false),
        ("a_b", false),
        ("été", false),
        ("a b", false),
        (&overlong, false),
    ] {
        let mut json: serde_json::Value = serde_json::from_slice(VALID_JSON).unwrap();
        json["id"] = id.into();
        json["rootUri"] = format!("studio://{id}/").into();
        let cem = std::str::from_utf8(VALID_CEM)
            .unwrap()
            .replacen(
                "@id=\"feature-tour\"",
                &format!("@id={}", serde_json::to_string(id).unwrap()),
                1,
            )
            .replace("studio://feature-tour/", &format!("studio://{id}/"));
        let diagnostics = studio_project_schema_diagnostics(&cem);
        if valid {
            assert!(diagnostics.is_empty(), "{id:?}: {diagnostics:?}");
        } else {
            assert!(
                diagnostics.iter().any(|d| d.code
                    == "cem.schema_model.invalid_attribute_datatype_param"
                    && d.details
                        .as_ref()
                        .is_some_and(|details| details["attribute"] == "id"
                            && details["datatypeParam"] == "pattern")),
                "{id:?}: {diagnostics:?}"
            );
        }
        for (bytes, content_type) in [
            (
                serde_json::to_vec(&json).unwrap(),
                STUDIO_PROJECT_JSON_CONTENT_TYPE,
            ),
            (cem.into_bytes(), STUDIO_PROJECT_CEM_CONTENT_TYPE),
        ] {
            let result = parse_studio_project(&bytes, content_type, STUDIO_PROJECT_SCHEMA_URI);
            if valid {
                assert!(result.is_ok(), "{id:?}: {result:?}");
            } else {
                let code = if id.is_empty() && content_type == STUDIO_PROJECT_CEM_CONTENT_TYPE {
                    "cem.studio_project.field_required"
                } else {
                    "cem.studio_project.id_invalid"
                };
                assert_eq!(result.unwrap_err().code, code, "{content_type}: {id:?}");
            }
        }
    }
}

#[test]
fn studio_project_schema_paths_keep_containment_and_reserved_directory_checks() {
    for (json_field, cem_field) in [("path", "path"), ("referencePath", "reference-path")] {
        for (path, check) in [
            ("/outside.cem", "type:path"),
            ("../outside.cem", "type:path"),
            ("data/../outside.cem", "type:path"),
            ("data/./source.cem", "type:path"),
            ("data//source.cem", "type:path"),
            ("data/", "type:path"),
            ("data\\source.cem", "type:path"),
            (
                ".cem-studio/private.cem",
                "datatype-param:pathForbiddenDirectoryNames",
            ),
            (
                "data/.cem-studio/private.cem",
                "datatype-param:pathForbiddenDirectoryNames",
            ),
        ] {
            let mut json: serde_json::Value = serde_json::from_slice(VALID_JSON).unwrap();
            json["resources"][0][json_field] = path.into();
            let replacement = format!("@{cem_field}={}", serde_json::to_string(path).unwrap());
            let replacement = if cem_field == "path" {
                replacement
            } else {
                format!("@path=\"data/tour.cem\" {replacement}")
            };
            let cem = std::str::from_utf8(VALID_CEM).unwrap().replacen(
                "@path=\"data/tour.cem\"",
                &replacement,
                1,
            );
            let diagnostics = studio_project_schema_diagnostics(&cem);
            assert!(
                diagnostics.iter().any(|d| d
                    .details
                    .as_ref()
                    .is_some_and(|details| details["attribute"] == cem_field
                        && details["checkKind"] == check)),
                "{cem_field}={path:?}: {diagnostics:?}"
            );
            for (bytes, content_type) in [
                (
                    serde_json::to_vec(&json).unwrap(),
                    STUDIO_PROJECT_JSON_CONTENT_TYPE,
                ),
                (cem.into_bytes(), STUDIO_PROJECT_CEM_CONTENT_TYPE),
            ] {
                let error = parse_studio_project(&bytes, content_type, STUDIO_PROJECT_SCHEMA_URI)
                    .unwrap_err();
                assert_eq!(
                    error.code, "cem.studio_project.resource_path_invalid",
                    "{json_field}={path:?}"
                );
                assert_eq!(error.field_path, Some(format!("resources[0].{json_field}")));
            }
        }
    }
}

#[test]
fn studio_project_entry_artifact_round_trips_in_both_projections() {
    for path in [
        "generated/index.html",
        "généré/été notes.html",
        "generated/.hidden/index.html",
    ] {
        let mut value: serde_json::Value = serde_json::from_slice(VALID_JSON).unwrap();
        value["entries"][1]["kind"] = "transformation-graph".into();
        value["entries"][1]["entryArtifact"] = path.into();
        let project = parse_studio_project(
            &serde_json::to_vec(&value).unwrap(),
            STUDIO_PROJECT_JSON_CONTENT_TYPE,
            STUDIO_PROJECT_SCHEMA_URI,
        )
        .unwrap();
        for projection in [StudioProjectProjection::Cem, StudioProjectProjection::Json] {
            let bytes = project.serialize(projection).unwrap();
            let restored = parse_studio_project(
                bytes.as_bytes(),
                projection.content_type(),
                STUDIO_PROJECT_SCHEMA_URI,
            )
            .unwrap();
            assert_eq!(restored, project);
            assert_eq!(restored.serialize(projection).unwrap(), bytes);
            let json: serde_json::Value =
                serde_json::from_str(&restored.serialize(StudioProjectProjection::Json).unwrap())
                    .unwrap();
            assert_eq!(json["entries"][1]["entryArtifact"], path);
        }
    }
}

#[test]
fn studio_project_entry_artifact_rejects_noncanonical_paths_and_non_build_kinds() {
    for path in [
        "",
        "/index.html",
        "../index.html",
        "generated/../index.html",
        "./index.html",
        "generated//index.html",
        "generated/",
        "C:/site.html",
        "https://site.test/index.html",
        "generated\\index.html",
        "index.html?x=1",
        "index.html#top",
        "index\n.html",
        "index\0.html",
    ] {
        let mut value: serde_json::Value = serde_json::from_slice(VALID_JSON).unwrap();
        value["entries"][1]["kind"] = "transformation-graph".into();
        value["entries"][1]["entryArtifact"] = path.into();
        let error = parse_studio_project(
            &serde_json::to_vec(&value).unwrap(),
            STUDIO_PROJECT_JSON_CONTENT_TYPE,
            STUDIO_PROJECT_SCHEMA_URI,
        )
        .unwrap_err();
        assert_eq!(
            error.code, "cem.studio_project.entry_artifact_invalid",
            "{path:?}: {error}"
        );
        assert_eq!(
            error.field_path.as_deref(),
            Some("entries[1].entryArtifact")
        );
    }
    for kind in [
        "validation",
        "inspection",
        "trace",
        "subproject",
        "data-set",
    ] {
        let mut value: serde_json::Value = serde_json::from_slice(VALID_JSON).unwrap();
        value["entries"][1]["kind"] = kind.into();
        value["entries"][1]["entryArtifact"] = "generated/index.html".into();
        if ["subproject", "data-set"].contains(&kind) {
            value["entries"][1]
                .as_object_mut()
                .unwrap()
                .remove("runConfigResourceId");
        }
        let error = parse_studio_project(
            &serde_json::to_vec(&value).unwrap(),
            STUDIO_PROJECT_JSON_CONTENT_TYPE,
            STUDIO_PROJECT_SCHEMA_URI,
        )
        .unwrap_err();
        assert_eq!(
            error.code, "cem.studio_project.entry_artifact_forbidden",
            "{kind}: {error}"
        );
    }
}

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

#[test]
fn resource_reference_survives_rename_and_round_trip() {
    let mut project = parse_studio_project(
        VALID_JSON,
        STUDIO_PROJECT_JSON_CONTENT_TYPE,
        STUDIO_PROJECT_SCHEMA_URI,
    )
    .unwrap();
    let before = studio_project_resource_uri(&project, "tour-source").unwrap();
    project
        .resources
        .iter_mut()
        .find(|r| r.id == "tour-source")
        .unwrap()
        .path = "moved/renamed.cem".into();
    assert_eq!(
        studio_project_resource_uri(&project, "tour-source").unwrap(),
        before
    );
    let bytes = serde_json::to_vec(&project).unwrap();
    let restored = parse_studio_project(
        &bytes,
        STUDIO_PROJECT_JSON_CONTENT_TYPE,
        STUDIO_PROJECT_SCHEMA_URI,
    )
    .unwrap();
    assert_eq!(
        studio_project_resource_uri(&restored, "tour-source").unwrap(),
        before
    );
}

#[test]
fn resource_reference_rejects_duplicate_and_escaping_addresses() {
    let project = parse_studio_project(
        VALID_JSON,
        STUDIO_PROJECT_JSON_CONTENT_TYPE,
        STUDIO_PROJECT_SCHEMA_URI,
    )
    .unwrap();
    for reference in ["../escape", "data/tour.cem"] {
        let mut invalid = project.clone();
        invalid.resources[1].reference_path = Some(reference.into());
        let bytes = serde_json::to_vec(&invalid).unwrap();
        assert!(parse_studio_project(
            &bytes,
            STUDIO_PROJECT_JSON_CONTENT_TYPE,
            STUDIO_PROJECT_SCHEMA_URI
        )
        .is_err());
    }
}

#[test]
fn resource_id_address_survives_both_native_projections() {
    let mut project = parse_studio_project(
        VALID_JSON,
        STUDIO_PROJECT_JSON_CONTENT_TYPE,
        STUDIO_PROJECT_SCHEMA_URI,
    )
    .unwrap();
    project.resources[0].reference_path = Some(".resources/tour-source".into());
    project.resources[0].path = "new-name.cem".into();
    for projection in [StudioProjectProjection::Cem, StudioProjectProjection::Json] {
        let text = project.serialize(projection).unwrap();
        let restored = parse_studio_project(
            text.as_bytes(),
            projection.content_type(),
            STUDIO_PROJECT_SCHEMA_URI,
        )
        .unwrap();
        assert_eq!(
            studio_project_resource_uri(&restored, "tour-source").unwrap(),
            "studio://feature-tour/.resources/tour-source"
        );
    }
}
