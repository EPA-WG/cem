use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use cem_ml::events::cem::CemEventNormalizer;
use cem_ml::parser::builder::CemAstBuilder;
use cem_ml::parser::document::CemDocument;
use cem_ml::parser::{AstNodeId, CemAstNode};
use cem_ml::schema::package_sources::builtin_schema_package_sources;
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;
use serde_json::Value;

const BASELINE_FORMATTER_PROFILES: &[&str] = &["compact", "pretty", "tabular"];
const BASELINE_COLORIZER_PROFILES: &[&str] = &["terminal", "html", "md"];
const SCHEMA_PACKAGE_PROJECT_INPUTS: &[&str] = &[
    "{projectRoot}/package.cem",
    "{projectRoot}/README.md",
    "{projectRoot}/schema/**/*.cem",
    "{projectRoot}/formatters/**/*.cemt",
    "{projectRoot}/colorizers/**/*.cemt",
    "{projectRoot}/converters/**/*.cemt",
    "{projectRoot}/examples/**/*",
];

const SCHEMA_SOURCE_FILENAME_EXCEPTIONS: &[SchemaSourceFilenameException] = &[
    SchemaSourceFilenameException {
        package_id: "cem-ml",
        source: "schema/cem-ml-generic.cem",
        canonical_source: "schema/cem-ml.cem",
        reason: "bootstrap generic CEM-ML schema identity is embedded by the runtime catalog",
    },
    SchemaSourceFilenameException {
        package_id: "schema",
        source: "schema/cem-schema.cem",
        canonical_source: "schema/schema.cem",
        reason: "bootstrap schema-definition identity is embedded by the runtime catalog",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SchemaSourceFilenameException {
    package_id: &'static str,
    source: &'static str,
    canonical_source: &'static str,
    reason: &'static str,
}

#[derive(Debug, Clone, Default)]
struct ManifestSummary {
    package_id: Option<String>,
    version: Option<String>,
    schema_source: Option<String>,
    example_paths: BTreeSet<String>,
    example_reference_keys: BTreeSet<String>,
    formatter_profiles: BTreeSet<String>,
    colorizer_profiles: BTreeSet<String>,
    cemt_artifact_paths: BTreeSet<String>,
    cemt_converter_templates: BTreeSet<String>,
}

#[derive(Debug, Clone, Default)]
struct NxProjectSummary {
    exists: bool,
    name: Option<String>,
    verify_target_exists: bool,
    missing_verify_inputs: Vec<String>,
}

#[derive(Debug, Clone)]
struct SchemaPackageStructureReport {
    package_id: String,
    version_dir: PathBuf,
    project_json_exists: bool,
    nx_project_name: Option<String>,
    nx_verify_target_exists: bool,
    missing_nx_verify_inputs: Vec<String>,
    manifest_exists: bool,
    readme_exists: bool,
    examples_dir_exists: bool,
    example_file_count: usize,
    manifest_example_count: usize,
    sidecar_example_reference_count: usize,
    schema_source: Option<String>,
    schema_source_exists: bool,
    schema_source_exception: Option<SchemaSourceFilenameException>,
    cemt_artifact_paths: BTreeSet<String>,
    scanned_cemt_assets: BTreeSet<String>,
    unregistered_cemt_assets: Vec<String>,
    formatter_profiles: BTreeSet<String>,
    missing_formatter_profiles: Vec<String>,
    colorizer_profiles: BTreeSet<String>,
    missing_colorizer_profiles: Vec<String>,
    cemt_converter_templates: BTreeSet<String>,
    missing_cemt_converter_templates: Vec<String>,
    hard_errors: Vec<String>,
    alignment_gaps: Vec<String>,
}

#[test]
fn built_in_schema_package_structure_audit_reports_folder_contract() {
    let reports = audit_schema_package_structure();
    let report_text = format_audit_report(&reports);
    println!("{report_text}");

    let audited_package_ids = reports
        .iter()
        .map(|report| report.package_id.as_str())
        .collect::<BTreeSet<_>>();
    let embedded_package_ids = builtin_schema_package_sources()
        .iter()
        .map(|source| source.package_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        audited_package_ids, embedded_package_ids,
        "schema-package folders and embedded package source catalog must stay aligned\n{report_text}"
    );

    let hard_errors = reports
        .iter()
        .flat_map(|report| {
            report
                .hard_errors
                .iter()
                .map(|error| format!("{}: {error}", report.package_id))
        })
        .collect::<Vec<_>>();
    assert!(
        hard_errors.is_empty(),
        "schema-package structure audit found hard errors:\n{}\n\n{report_text}",
        hard_errors.join("\n")
    );
}

#[test]
fn schema_package_structure_audit_covers_profiles_artifacts_and_cemt_converters() {
    let reports = audit_schema_package_structure();
    let by_package = reports
        .iter()
        .map(|report| (report.package_id.as_str(), report))
        .collect::<BTreeMap<_, _>>();

    let csv = by_package.get("csv").expect("csv package report");
    assert!(csv.missing_formatter_profiles.is_empty(), "{csv:#?}");
    assert!(csv.missing_colorizer_profiles.is_empty(), "{csv:#?}");
    assert!(csv.cemt_artifact_paths.contains("formatters/compact.cemt"));
    assert!(csv.cemt_artifact_paths.contains("formatters/pretty.cemt"));
    assert!(csv.cemt_artifact_paths.contains("formatters/tabular.cemt"));
    assert!(csv.cemt_artifact_paths.contains("colorizers/terminal.cemt"));
    assert!(csv.cemt_artifact_paths.contains("colorizers/html.cemt"));
    assert!(csv.cemt_artifact_paths.contains("colorizers/md.cemt"));

    let dom_projection = by_package
        .get("cem-dom-projection")
        .expect("cem-dom-projection package report");
    assert!(dom_projection
        .cemt_converter_templates
        .contains("converters/dom-to-html.cemt"));
    assert!(dom_projection
        .cemt_converter_templates
        .contains("converters/dom-to-xml.cemt"));
    assert!(
        dom_projection.missing_cemt_converter_templates.is_empty(),
        "{dom_projection:#?}"
    );
}

#[test]
fn schema_package_examples_use_manifest_owned_reference_records() {
    let reports = audit_schema_package_structure();
    for report in &reports {
        assert!(
            report.manifest_example_count > 0,
            "{} must declare package-owned example references in package.cem",
            report.package_id
        );
        assert!(
            report.hard_errors.iter().all(|error| {
                !error.contains("manifest example")
                    && !error.contains("manifest declares no examples")
            }),
            "{} has incomplete manifest-owned example references: {report:#?}",
            report.package_id
        );
    }

    let csv = reports
        .iter()
        .find(|report| report.package_id == "csv")
        .expect("csv package report");
    assert_eq!(csv.manifest_example_count, 8);
}

#[test]
fn schema_package_schema_filename_exceptions_are_documented_and_explicit() {
    let reports = audit_schema_package_structure();
    let package_contract_readme = fs::read_to_string(schema_packages_root().join("README.md"))
        .expect("schema-package README");
    let exception_sources = reports
        .iter()
        .filter_map(|report| {
            report
                .schema_source_exception
                .map(|exception| (report.package_id.as_str(), exception.source))
        })
        .collect::<Vec<_>>();

    assert_eq!(
        exception_sources,
        vec![
            ("cem-ml", "schema/cem-ml-generic.cem"),
            ("schema", "schema/cem-schema.cem"),
        ]
    );

    for exception in SCHEMA_SOURCE_FILENAME_EXCEPTIONS {
        assert!(
            !exception.reason.trim().is_empty(),
            "{exception:?} must document why it is not renamed to {}",
            exception.canonical_source
        );
        assert!(
            package_contract_readme.contains(exception.source),
            "{exception:?} must be named in the schema-package README"
        );

        let report = reports
            .iter()
            .find(|report| report.package_id == exception.package_id)
            .expect("schema source exception package is audited");
        assert_eq!(report.schema_source.as_deref(), Some(exception.source));
        assert!(report.schema_source_exists, "{report:#?}");
    }

    for report in &reports {
        assert!(
            report
                .hard_errors
                .iter()
                .all(|error| !error.contains("not a documented schema-source filename exception")),
            "{} has an undocumented schema-source filename drift: {report:#?}",
            report.package_id
        );
    }
}

#[test]
fn schema_package_folders_are_nx_owned_libraries_with_cemt_inputs() {
    let reports = audit_schema_package_structure();
    for report in &reports {
        assert!(
            report.project_json_exists,
            "{} must include project.json so Nx owns package inputs",
            report.package_id
        );
        assert_eq!(
            report.nx_project_name.as_deref(),
            Some(expected_schema_package_project_name(&report.package_id).as_str()),
            "{} must use the schema-package Nx project naming convention",
            report.package_id
        );
        assert!(
            report.nx_verify_target_exists,
            "{} must expose a cached verify target",
            report.package_id
        );
        assert!(
            report.missing_nx_verify_inputs.is_empty(),
            "{} verify target must track schema, example, formatter, colorizer, and converter inputs: {report:#?}",
            report.package_id
        );
        assert!(
            report
                .hard_errors
                .iter()
                .all(|error| !error.contains("Nx project")),
            "{} has incomplete Nx schema-package ownership: {report:#?}",
            report.package_id
        );
    }
}

fn audit_schema_package_structure() -> Vec<SchemaPackageStructureReport> {
    let root = schema_packages_root();
    let mut reports = fs::read_dir(&root)
        .expect("schema-packages directory")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()))
        .filter_map(|entry| {
            let package_id = entry.file_name().to_string_lossy().into_owned();
            let version_dir = entry.path().join("v1");
            version_dir
                .is_dir()
                .then(|| audit_package_version_dir(package_id, version_dir))
        })
        .collect::<Vec<_>>();
    reports.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    reports
}

fn audit_package_version_dir(
    package_id: String,
    version_dir: PathBuf,
) -> SchemaPackageStructureReport {
    let manifest_path = version_dir.join("package.cem");
    let manifest_exists = manifest_path.is_file();
    let readme_exists = version_dir.join("README.md").is_file();
    let examples_dir = version_dir.join("examples");
    let examples_dir_exists = examples_dir.is_dir();
    let example_file_count = count_files_recursively(&examples_dir);
    let sidecar_example_reference_count = count_example_reference_sidecars(&examples_dir);
    let scanned_cemt_assets = scan_cemt_assets(&version_dir);

    let mut hard_errors = Vec::new();
    let nx_project = audit_schema_package_project_json(&package_id, &version_dir, &mut hard_errors);
    let manifest = if manifest_exists {
        parse_manifest(&manifest_path, &mut hard_errors)
    } else {
        hard_errors.push("missing package.cem".to_owned());
        ManifestSummary::default()
    };

    if !readme_exists {
        hard_errors.push("missing README.md".to_owned());
    }
    if !examples_dir_exists {
        hard_errors.push("missing examples/ directory".to_owned());
    }
    if example_file_count == 0 {
        hard_errors.push("examples/ contains no package-owned fixtures".to_owned());
    }

    if manifest
        .package_id
        .as_deref()
        .is_some_and(|id| id != package_id)
    {
        hard_errors.push(format!(
            "manifest package id `{}` does not match folder id `{package_id}`",
            manifest.package_id.as_deref().unwrap_or_default()
        ));
    }
    if manifest
        .version
        .as_deref()
        .is_some_and(|version| version != "1.0.0")
    {
        hard_errors.push(format!(
            "manifest version `{}` does not match v1 folder",
            manifest.version.as_deref().unwrap_or_default()
        ));
    }

    let schema_source_exists = manifest
        .schema_source
        .as_deref()
        .is_some_and(|schema_source| version_dir.join(schema_source).is_file());
    match manifest.schema_source.as_deref() {
        Some(_) if schema_source_exists => {}
        Some(schema_source) => hard_errors.push(format!(
            "manifest schema source `{schema_source}` is not readable"
        )),
        None => hard_errors.push("manifest does not declare schema @source".to_owned()),
    }

    let mut missing_declared_cemt = Vec::new();
    for path in manifest
        .cemt_artifact_paths
        .iter()
        .chain(manifest.cemt_converter_templates.iter())
    {
        if !version_dir.join(path).is_file() {
            missing_declared_cemt.push(path.clone());
        }
    }
    if !missing_declared_cemt.is_empty() {
        hard_errors.push(format!(
            "manifest declares unreadable CEMT assets: {}",
            missing_declared_cemt.join(", ")
        ));
    }

    let missing_cemt_converter_templates = manifest
        .cemt_converter_templates
        .iter()
        .filter(|path| !version_dir.join(path).is_file())
        .cloned()
        .collect::<Vec<_>>();

    for example_path in &manifest.example_paths {
        if !version_dir.join(example_path).is_file() {
            hard_errors.push(format!("manifest example `{example_path}` is not readable"));
        }
    }

    let declared_cemt = manifest
        .cemt_artifact_paths
        .iter()
        .chain(manifest.cemt_converter_templates.iter())
        .cloned()
        .collect::<BTreeSet<_>>();
    let unregistered_cemt_assets = scanned_cemt_assets
        .difference(&declared_cemt)
        .cloned()
        .collect::<Vec<_>>();

    let missing_formatter_profiles =
        missing_profiles(&manifest.formatter_profiles, BASELINE_FORMATTER_PROFILES);
    let missing_colorizer_profiles =
        missing_profiles(&manifest.colorizer_profiles, BASELINE_COLORIZER_PROFILES);

    let schema_source_exception = manifest
        .schema_source
        .as_deref()
        .and_then(|schema_source| schema_source_filename_exception(&package_id, schema_source));

    let mut alignment_gaps = Vec::new();
    if let Some(schema_source) = &manifest.schema_source {
        let canonical_schema_source = format!("schema/{package_id}.cem");
        if schema_source != &canonical_schema_source && schema_source_exception.is_none() {
            hard_errors.push(format!(
                "manifest schema source `{schema_source}` differs from canonical `{canonical_schema_source}` and is not a documented schema-source filename exception"
            ));
        }
    }
    if manifest.example_paths.is_empty() {
        alignment_gaps.push("manifest declares no examples".to_owned());
    }
    if !missing_formatter_profiles.is_empty() {
        alignment_gaps.push(format!(
            "missing baseline formatter profiles: {}",
            missing_formatter_profiles.join(", ")
        ));
    }
    if !missing_colorizer_profiles.is_empty() {
        alignment_gaps.push(format!(
            "missing baseline colorizer profiles: {}",
            missing_colorizer_profiles.join(", ")
        ));
    }
    if !unregistered_cemt_assets.is_empty() {
        alignment_gaps.push(format!(
            "CEMT assets exist outside manifest artifact/converter declarations: {}",
            unregistered_cemt_assets.join(", ")
        ));
    }

    SchemaPackageStructureReport {
        package_id,
        version_dir,
        project_json_exists: nx_project.exists,
        nx_project_name: nx_project.name,
        nx_verify_target_exists: nx_project.verify_target_exists,
        missing_nx_verify_inputs: nx_project.missing_verify_inputs,
        manifest_exists,
        readme_exists,
        examples_dir_exists,
        example_file_count,
        manifest_example_count: manifest.example_reference_keys.len(),
        sidecar_example_reference_count,
        schema_source: manifest.schema_source,
        schema_source_exists,
        schema_source_exception,
        cemt_artifact_paths: manifest.cemt_artifact_paths,
        scanned_cemt_assets,
        unregistered_cemt_assets,
        formatter_profiles: manifest.formatter_profiles,
        missing_formatter_profiles,
        colorizer_profiles: manifest.colorizer_profiles,
        missing_colorizer_profiles,
        cemt_converter_templates: manifest.cemt_converter_templates,
        missing_cemt_converter_templates,
        hard_errors,
        alignment_gaps,
    }
}

fn audit_schema_package_project_json(
    package_id: &str,
    version_dir: &Path,
    hard_errors: &mut Vec<String>,
) -> NxProjectSummary {
    let project_json_path = version_dir.join("project.json");
    if !project_json_path.is_file() {
        hard_errors.push("missing Nx project.json".to_owned());
        return NxProjectSummary::default();
    }

    let source = match fs::read_to_string(&project_json_path) {
        Ok(source) => source,
        Err(error) => {
            hard_errors.push(format!("Nx project.json is not readable: {error}"));
            return NxProjectSummary {
                exists: true,
                ..NxProjectSummary::default()
            };
        }
    };
    let project_json = match serde_json::from_str::<Value>(&source) {
        Ok(project_json) => project_json,
        Err(error) => {
            hard_errors.push(format!("Nx project.json is not valid JSON: {error}"));
            return NxProjectSummary {
                exists: true,
                ..NxProjectSummary::default()
            };
        }
    };

    let name = json_string(&project_json, "/name").map(str::to_owned);
    let expected_name = expected_schema_package_project_name(package_id);
    if name.as_deref() != Some(expected_name.as_str()) {
        hard_errors.push(format!(
            "Nx project name `{}` does not match expected `{expected_name}`",
            name.as_deref().unwrap_or("<missing>")
        ));
    }

    if json_string(&project_json, "/projectType") != Some("library") {
        hard_errors.push("Nx projectType must be `library`".to_owned());
    }

    let expected_source_root = format!("packages/cem_ml/schema-packages/{package_id}/v1");
    if json_string(&project_json, "/sourceRoot") != Some(expected_source_root.as_str()) {
        hard_errors.push(format!(
            "Nx sourceRoot `{}` does not match expected `{expected_source_root}`",
            json_string(&project_json, "/sourceRoot").unwrap_or("<missing>")
        ));
    }

    let verify = project_json.pointer("/targets/verify");
    let verify_target_exists = verify.is_some();
    let Some(verify) = verify else {
        hard_errors.push("Nx project must expose a `verify` target".to_owned());
        return NxProjectSummary {
            exists: true,
            name,
            ..NxProjectSummary::default()
        };
    };

    if verify.pointer("/cache").and_then(Value::as_bool) != Some(true) {
        hard_errors.push("Nx project verify target must be cacheable".to_owned());
    }
    if json_string(verify, "/executor") != Some("nx:run-commands") {
        hard_errors.push("Nx project verify target must use nx:run-commands".to_owned());
    }

    if !verify_depends_on_cli_build(verify) {
        hard_errors.push("Nx project verify target must depend on cem_ml_cli:build".to_owned());
    }
    if !verify_uses_parse_fail_level(verify) {
        hard_errors.push(
            "Nx project verify target must run CLI validation with --fail-level parse".to_owned(),
        );
    }

    let inputs = verify
        .pointer("/inputs")
        .and_then(Value::as_array)
        .map(|inputs| {
            inputs
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let missing_verify_inputs = SCHEMA_PACKAGE_PROJECT_INPUTS
        .iter()
        .filter(|input| !inputs.contains(**input))
        .map(|input| (*input).to_owned())
        .collect::<Vec<_>>();
    if !missing_verify_inputs.is_empty() {
        hard_errors.push(format!(
            "Nx project verify target is missing inputs: {}",
            missing_verify_inputs.join(", ")
        ));
    }

    if !verify_outputs_report(verify) {
        hard_errors.push(
            "Nx project verify target must declare {projectRoot}/dist/cem-ml.report.json output"
                .to_owned(),
        );
    }

    NxProjectSummary {
        exists: true,
        name,
        verify_target_exists,
        missing_verify_inputs,
    }
}

fn expected_schema_package_project_name(package_id: &str) -> String {
    format!("cem_ml_schema_package_{}_v1", package_id.replace('-', "_"))
}

fn json_string<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

fn verify_depends_on_cli_build(verify: &Value) -> bool {
    verify
        .pointer("/dependsOn")
        .and_then(Value::as_array)
        .is_some_and(|depends_on| {
            depends_on.iter().any(|dependency| {
                json_string(dependency, "/target") == Some("build")
                    && dependency
                        .pointer("/projects")
                        .and_then(Value::as_array)
                        .is_some_and(|projects| {
                            projects
                                .iter()
                                .any(|project| project.as_str() == Some("cem_ml_cli"))
                        })
            })
        })
}

fn verify_uses_parse_fail_level(verify: &Value) -> bool {
    verify
        .pointer("/options/commands")
        .and_then(Value::as_array)
        .is_some_and(|commands| {
            commands.iter().filter_map(Value::as_str).any(|command| {
                command.contains(" validate ") && command.contains(" --fail-level parse ")
            })
        })
}

fn verify_outputs_report(verify: &Value) -> bool {
    verify
        .pointer("/outputs")
        .and_then(Value::as_array)
        .is_some_and(|outputs| {
            outputs
                .iter()
                .any(|output| output.as_str() == Some("{projectRoot}/dist/cem-ml.report.json"))
        })
}

fn parse_manifest(manifest_path: &Path, hard_errors: &mut Vec<String>) -> ManifestSummary {
    let source = match fs::read_to_string(manifest_path) {
        Ok(source) => source,
        Err(error) => {
            hard_errors.push(format!(
                "package.cem could not be read at `{}`: {error}",
                manifest_path.display()
            ));
            return ManifestSummary::default();
        }
    };
    let document = parse_cem_document(&source);
    if !document.diagnostics.is_empty() {
        hard_errors.extend(document.diagnostics.iter().map(|diagnostic| {
            format!(
                "package.cem parse diagnostic {}: {}",
                diagnostic.code, diagnostic.message
            )
        }));
    }

    let Some(package_id) = first_element_by_local_name(&document, "package") else {
        hard_errors.push("package.cem does not contain a package element".to_owned());
        return ManifestSummary::default();
    };
    let package_attrs = collect_attrs(&document, package_id);
    let mut summary = ManifestSummary {
        package_id: package_attrs.get("id").cloned(),
        version: package_attrs.get("version").cloned(),
        ..ManifestSummary::default()
    };
    let mut example_ids = BTreeSet::new();

    for (child_index, child_id) in element_child_ids(&document, package_id)
        .into_iter()
        .enumerate()
    {
        let Some(local_name) = element_local_name(&document, child_id) else {
            continue;
        };
        let attrs = collect_attrs(&document, child_id);
        match local_name {
            "schema" => {
                if let Some(source) = attrs.get("source") {
                    summary.schema_source = Some(normalize_manifest_path(source));
                }
            }
            "example" => {
                let example_index = child_index + 1;
                let label = manifest_example_label(&attrs, example_index);
                let id = required_manifest_example_attr(&attrs, "id", &label, hard_errors);
                let path = required_manifest_example_attr(&attrs, "path", &label, hard_errors)
                    .map(|path| normalize_manifest_path(&path));
                let content_type =
                    required_manifest_example_attr(&attrs, "content-type", &label, hard_errors);
                let schema = required_manifest_example_attr(&attrs, "schema", &label, hard_errors);
                let expected_result =
                    required_manifest_example_attr(&attrs, "expected-result", &label, hard_errors);
                let expected_diagnostics = attrs
                    .get("expected-diagnostics")
                    .map(|value| {
                        value
                            .split_whitespace()
                            .filter(|code| !code.trim().is_empty())
                            .map(str::to_owned)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                if let Some(id) = &id {
                    if !example_ids.insert(id.clone()) {
                        hard_errors
                            .push(format!("manifest example `{label}` duplicates id `{id}`"));
                    }
                }
                if let Some(path) = &path {
                    if !summary.example_paths.insert(path.clone()) {
                        hard_errors.push(format!(
                            "manifest example `{label}` duplicates path `{path}`"
                        ));
                    }
                }
                match expected_result.as_deref() {
                    Some("pass") => {}
                    Some("fail") => {
                        if expected_diagnostics.is_empty() {
                            hard_errors.push(format!(
                                "manifest example `{label}` expects failure without expected-diagnostics"
                            ));
                        }
                    }
                    Some(result) => hard_errors.push(format!(
                        "manifest example `{label}` has invalid expected-result `{result}`"
                    )),
                    None => {}
                }

                if let (Some(id), Some(path), Some(content_type), Some(schema), Some(result)) =
                    (id, path, content_type, schema, expected_result)
                {
                    summary.example_reference_keys.insert(format!(
                        "{id}|{path}|{content_type}|{schema}|{result}|{}",
                        expected_diagnostics.join(" ")
                    ));
                }
            }
            "artifact" => {
                let kind = attrs.get("kind").map(String::as_str);
                if let Some(path) = attrs.get("path").map(|path| normalize_manifest_path(path)) {
                    if path.ends_with(".cemt") {
                        summary.cemt_artifact_paths.insert(path);
                    }
                }
                match kind {
                    Some("formatter") => {
                        if let Some(profile) = attrs.get("formatter-profile") {
                            summary.formatter_profiles.insert(profile.clone());
                        }
                    }
                    Some("colorizer") => {
                        if let Some(profile) = attrs.get("color-profile") {
                            summary.colorizer_profiles.insert(profile.clone());
                        }
                    }
                    _ => {}
                }
            }
            "converter" => {
                if attrs
                    .get("implementation")
                    .is_some_and(|implementation| implementation == "cemt")
                {
                    if let Some(template) = attrs.get("template") {
                        summary
                            .cemt_converter_templates
                            .insert(normalize_manifest_path(template));
                    }
                }
            }
            _ => {}
        }
    }

    summary
}

fn parse_cem_document(source: &str) -> CemDocument {
    let source = BytesSource::new(SourceId(1), source.as_bytes().to_vec());
    let tokenizer = CemTokenizer::from_source(source);
    let normalizer = CemEventNormalizer::new(tokenizer);
    CemAstBuilder::new(normalizer).build()
}

fn first_element_by_local_name(document: &CemDocument, local_name: &str) -> Option<AstNodeId> {
    document.iter().find_map(|node| match node {
        CemAstNode::Element {
            node_id,
            expanded_name,
            ..
        } if expanded_name.local_name == local_name => Some(*node_id),
        _ => None,
    })
}

fn element_child_ids(document: &CemDocument, node_id: AstNodeId) -> Vec<AstNodeId> {
    match document.get(node_id) {
        Some(CemAstNode::Element { children, .. }) => children
            .iter()
            .copied()
            .filter(|child_id| matches!(document.get(*child_id), Some(CemAstNode::Element { .. })))
            .collect(),
        _ => Vec::new(),
    }
}

fn element_local_name(document: &CemDocument, node_id: AstNodeId) -> Option<&str> {
    match document.get(node_id) {
        Some(CemAstNode::Element { expanded_name, .. }) => Some(expanded_name.local_name.as_str()),
        _ => None,
    }
}

fn collect_attrs(document: &CemDocument, node_id: AstNodeId) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    let Some(CemAstNode::Element { attributes, .. }) = document.get(node_id) else {
        return attrs;
    };
    for attr_id in attributes {
        let Some(CemAstNode::Attribute {
            expanded_name,
            value,
            ..
        }) = document.get(*attr_id)
        else {
            continue;
        };
        if let Some(value) = value {
            attrs.insert(expanded_name.local_name.clone(), value.clone());
        }
    }
    attrs
}

fn manifest_example_label(attrs: &BTreeMap<String, String>, index: usize) -> String {
    attrs
        .get("id")
        .or_else(|| attrs.get("path"))
        .cloned()
        .unwrap_or_else(|| format!("#{index}"))
}

fn required_manifest_example_attr(
    attrs: &BTreeMap<String, String>,
    name: &str,
    label: &str,
    hard_errors: &mut Vec<String>,
) -> Option<String> {
    let Some(value) = attrs.get(name) else {
        hard_errors.push(format!(
            "manifest example `{label}` is missing required @{name}"
        ));
        return None;
    };
    if value.trim().is_empty() {
        hard_errors.push(format!(
            "manifest example `{label}` has empty required @{name}"
        ));
        return None;
    }
    Some(value.clone())
}

fn schema_source_filename_exception(
    package_id: &str,
    schema_source: &str,
) -> Option<SchemaSourceFilenameException> {
    SCHEMA_SOURCE_FILENAME_EXCEPTIONS
        .iter()
        .copied()
        .find(|exception| exception.package_id == package_id && exception.source == schema_source)
}

fn schema_packages_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schema-packages")
}

fn normalize_manifest_path(path: &str) -> String {
    let mut normalized = path.trim().replace('\\', "/");
    while let Some(rest) = normalized.strip_prefix("./") {
        normalized = rest.to_owned();
    }
    normalized
}

fn missing_profiles(actual: &BTreeSet<String>, expected: &[&str]) -> Vec<String> {
    expected
        .iter()
        .filter(|profile| !actual.contains(**profile))
        .map(|profile| (*profile).to_owned())
        .collect()
}

fn count_files_recursively(root: &Path) -> usize {
    if !root.is_dir() {
        return 0;
    }
    let mut count = 0;
    let Ok(entries) = fs::read_dir(root) else {
        return 0;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            count += count_files_recursively(&path);
        } else if path.is_file() {
            count += 1;
        }
    }
    count
}

fn count_example_reference_sidecars(root: &Path) -> usize {
    if !root.is_dir() {
        return 0;
    }
    let mut count = 0;
    let Ok(entries) = fs::read_dir(root) else {
        return 0;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            count += count_example_reference_sidecars(&path);
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".example.cem"))
        {
            count += 1;
        }
    }
    count
}

fn scan_cemt_assets(version_dir: &Path) -> BTreeSet<String> {
    let mut assets = BTreeSet::new();
    for folder in ["formatters", "colorizers", "converters"] {
        collect_cemt_assets(version_dir, &version_dir.join(folder), &mut assets);
    }
    assets
}

fn collect_cemt_assets(version_dir: &Path, dir: &Path, assets: &mut BTreeSet<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_cemt_assets(version_dir, &path, assets);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("cemt") {
            let relative_path = path
                .strip_prefix(version_dir)
                .expect("CEMT asset is under version dir")
                .to_string_lossy()
                .replace('\\', "/");
            assets.insert(relative_path);
        }
    }
}

fn format_audit_report(reports: &[SchemaPackageStructureReport]) -> String {
    let mut output = String::from("Schema package structure audit:\n");
    for report in reports {
        output.push_str(&format!(
            "- {}/v1: dir={}, roots(project={}, package={}, readme={}, examples={} files={}), nx_name={}, nx_verify={}, missing_nx_inputs={}, schema={} exists={}, schema_exception={}, manifest_examples={}, example_sidecars={}, declared_cemt_assets={}, scanned_cemt_assets={}, formatter_profiles={}, missing_formatter={}, colorizer_profiles={}, missing_colorizer={}, cemt_converters={}, missing_cemt_converters={}, gaps={}, hard_errors={}\n",
            report.package_id,
            format_report_path(&report.version_dir),
            report.project_json_exists,
            report.manifest_exists,
            report.readme_exists,
            report.examples_dir_exists,
            report.example_file_count,
            report.nx_project_name.as_deref().unwrap_or("<missing>"),
            report.nx_verify_target_exists,
            format_list(&report.missing_nx_verify_inputs),
            report.schema_source.as_deref().unwrap_or("<missing>"),
            report.schema_source_exists,
            format_schema_source_exception(report.schema_source_exception),
            report.manifest_example_count,
            report.sidecar_example_reference_count,
            format_set(&report.cemt_artifact_paths),
            format_set(&report.scanned_cemt_assets),
            format_set(&report.formatter_profiles),
            format_list(&report.missing_formatter_profiles),
            format_set(&report.colorizer_profiles),
            format_list(&report.missing_colorizer_profiles),
            format_set(&report.cemt_converter_templates),
            format_list(&report.missing_cemt_converter_templates),
            format_list(&report.alignment_gaps),
            format_list(&report.hard_errors),
        ));
        if !report.unregistered_cemt_assets.is_empty() {
            output.push_str(&format!(
                "  unregistered_cemt_assets={}\n",
                format_list(&report.unregistered_cemt_assets)
            ));
        }
    }
    output
}

fn format_schema_source_exception(exception: Option<SchemaSourceFilenameException>) -> String {
    exception
        .map(|exception| {
            format!(
                "{} instead of {} ({})",
                exception.source, exception.canonical_source, exception.reason
            )
        })
        .unwrap_or_else(|| "none".to_owned())
}

fn format_set(values: &BTreeSet<String>) -> String {
    if values.is_empty() {
        "[]".to_owned()
    } else {
        format!(
            "[{}]",
            values.iter().cloned().collect::<Vec<_>>().join(", ")
        )
    }
}

fn format_report_path(path: &Path) -> String {
    path.strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")))
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"))
}

fn format_list(values: &[String]) -> String {
    if values.is_empty() {
        "[]".to_owned()
    } else {
        format!("[{}]", values.join("; "))
    }
}
