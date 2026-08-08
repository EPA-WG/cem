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

const DEFERRED_CROSS_PACKAGE_CONVERTER_EDGES: &[(&str, &str, &str, &str)] = &[
    (
        "cem-dom-projection",
        "cem-dom-projection-to-html-cemt",
        "cem-dom-projection",
        "html",
    ),
    (
        "cem-dom-projection",
        "cem-dom-projection-to-html-rust",
        "cem-dom-projection",
        "html",
    ),
    (
        "cem-dom-projection",
        "cem-dom-projection-to-xml-cemt",
        "cem-dom-projection",
        "xml",
    ),
    (
        "cem-dom-projection",
        "cem-dom-projection-to-xml-rust",
        "cem-dom-projection",
        "xml",
    ),
    (
        "cem-ml",
        "cem-ml-to-ast-projection-rust",
        "cem-ml",
        "cem-ast-projection",
    ),
    (
        "cem-ml",
        "cem-ml-to-dom-projection-rust",
        "cem-ml",
        "cem-dom-projection",
    ),
    (
        "cem-ml",
        "cem-ml-to-events-projection-rust",
        "cem-ml",
        "cem-events-projection",
    ),
    (
        "html",
        "html-to-cem-dom-projection-rust",
        "html",
        "cem-dom-projection",
    ),
    (
        "xml",
        "xml-to-cem-dom-projection-rust",
        "xml",
        "cem-dom-projection",
    ),
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
    schema_uri: Option<String>,
    content_types: BTreeSet<String>,
    schema_source: Option<String>,
    example_paths: BTreeSet<String>,
    example_reference_keys: BTreeSet<String>,
    formatter_profiles: BTreeSet<String>,
    colorizer_profiles: BTreeSet<String>,
    cemt_artifact_paths: BTreeSet<String>,
    cemt_converter_templates: BTreeSet<String>,
    converter_endpoints: Vec<ManifestConverterEndpoint>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
struct ManifestConverterEndpoint {
    id: String,
    implementation: Option<String>,
    rust_symbol: Option<String>,
    from_content_type: Option<String>,
    from_schema: Option<String>,
    to_content_type: Option<String>,
    to_schema: Option<String>,
    template: Option<String>,
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
    schema_uri: Option<String>,
    content_types: BTreeSet<String>,
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
    converter_endpoints: Vec<ManifestConverterEndpoint>,
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
fn css_selector_package_is_schema_owned_nx_subproject() {
    let reports = audit_schema_package_structure();
    let css = reports
        .iter()
        .find(|report| report.package_id == "css")
        .expect("CSS stylesheet package report");
    let selector = reports
        .iter()
        .find(|report| report.package_id == "css-selector")
        .expect("CSS selector query package report");

    assert_eq!(
        css.schema_uri.as_deref(),
        Some("https://cem.dev/ns/data/css/1")
    );
    assert_eq!(css.content_types, BTreeSet::from(["text/css".to_owned()]));
    assert_eq!(
        selector.schema_uri.as_deref(),
        Some("https://cem.dev/ns/query/css-selector/1")
    );
    assert_eq!(
        selector.content_types,
        BTreeSet::from(["application/vnd.cem.query-expression+css-selector".to_owned()])
    );
    assert_eq!(
        selector.nx_project_name.as_deref(),
        Some("cem_ml_schema_package_css_selector_v1")
    );
    assert!(
        selector.missing_formatter_profiles.is_empty(),
        "{selector:#?}"
    );
    assert!(
        selector.missing_colorizer_profiles.is_empty(),
        "{selector:#?}"
    );
    assert!(
        selector
            .version_dir
            .join("tests/selectors-4-conformance.cem")
            .is_file(),
        "CSS selector package must own its Selectors Level 4 conformance matrix"
    );

    let cli_project = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../cem_ml_cli/project.json"),
    )
    .expect("cem_ml_cli project configuration");
    assert_eq!(
        cli_project
            .matches("\"cem_ml_schema_package_css_selector_v1\"")
            .count(),
        3,
        "CSS selector package must participate in CLI test, converter-parity, and e2e dependency gates"
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
    assert_eq!(csv.manifest_example_count, 16);
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
fn textual_schema_package_readmes_embed_examples_as_language_fences() {
    for report in audit_schema_package_structure() {
        let mut hard_errors = Vec::new();
        let manifest = parse_manifest(&report.version_dir.join("package.cem"), &mut hard_errors);
        assert!(
            hard_errors.is_empty(),
            "{} manifest must parse before checking README examples: {hard_errors:?}",
            report.package_id
        );

        let readme_path = report.version_dir.join("README.md");
        let readme = fs::read_to_string(&readme_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", readme_path.display()));
        let mut fenceable_count = 0usize;

        for example_path in &manifest.example_paths {
            let Some(language) = source_fence_language(example_path) else {
                continue;
            };
            let source_path = report.version_dir.join(example_path);
            let bytes = fs::read(&source_path).unwrap_or_else(|error| {
                panic!("failed to read {}: {error}", source_path.display())
            });
            let Ok(source) = std::str::from_utf8(&bytes) else {
                continue;
            };
            let source = source.strip_prefix('\u{feff}').unwrap_or(source);
            if source
                .chars()
                .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
            {
                continue;
            }

            fenceable_count += 1;
            let source = source.replace("\r\n", "\n").replace('\r', "\n");
            let source = source.trim_end();
            let delimiter = "`".repeat(markdown_fence_length(source));
            let expected = format!("{delimiter}{language}\n{source}\n{delimiter}");

            assert!(
                readme.contains(&expected),
                "{} README must embed `{example_path}` with an exact {language} source fence",
                report.package_id
            );
        }

        if fenceable_count == manifest.example_paths.len() {
            assert!(
                !readme.contains("![Preview of"),
                "{} README must not use SVG snapshots when every example supports a source fence",
                report.package_id
            );
        }
    }
}

fn source_fence_language(path: &str) -> Option<&'static str> {
    match Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase()
        .as_str()
    {
        "cem" | "cemt" => Some("cem"),
        "cemql" | "cem-ql" => Some("cemql"),
        "css" | "css-selector" => Some("css"),
        "csv" => Some("csv"),
        "html" | "htm" | "xhtml" => Some("html"),
        "json" => Some("json"),
        "md" | "markdown" => Some("markdown"),
        "mathml" | "mml" | "rng" | "xml" | "xsl" | "xslt" => Some("xml"),
        "rnc" => Some("rnc"),
        "svg" => Some("svg"),
        "xpath" => Some("xpath"),
        "yaml" | "yml" => Some("yaml"),
        _ => None,
    }
}

fn markdown_fence_length(source: &str) -> usize {
    let mut longest = 0usize;
    let mut current = 0usize;
    for byte in source.bytes() {
        if byte == b'`' {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    3.max(longest + 1)
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

#[test]
fn schema_package_converter_endpoint_checks_are_final_registry_pass() {
    let reports = audit_schema_package_structure();
    let report_text = format_audit_report(&reports);

    let deferred_edges = deferred_cross_package_converter_edges(&reports);
    let expected_edges = DEFERRED_CROSS_PACKAGE_CONVERTER_EDGES
        .iter()
        .map(|(package_id, converter_id, from_owner, to_owner)| {
            format!("{package_id}/{converter_id}: {from_owner} -> {to_owner}")
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        deferred_edges, expected_edges,
        "cross-package converter endpoint edges must be audited as final registry-pass work, not as package-local project dependencies\n{report_text}"
    );

    for report in &reports {
        let sibling_schema_package_dependencies =
            schema_package_verify_schema_project_dependencies(report);
        assert!(
            sibling_schema_package_dependencies.is_empty(),
            "{} verify target must not depend on sibling schema-package projects; cross-package converter endpoint checks belong to the final registry pass: {}",
            report.package_id,
            format_list(&sibling_schema_package_dependencies)
        );
    }

    for target_name in ["validate-converter-parity", "e2e"] {
        assert_cli_registry_target_depends_on_all_schema_package_verifies(target_name, &reports);
    }
}

#[test]
fn cem_ml_package_readme_tracks_manifest_contract() {
    let version_dir = schema_packages_root().join("cem-ml/v1");
    let readme_path = version_dir.join("README.md");
    let readme = fs::read_to_string(&readme_path)
        .unwrap_or_else(|error| panic!("{} is not readable: {error}", readme_path.display()));
    let mut hard_errors = Vec::new();
    let manifest = parse_manifest(&version_dir.join("package.cem"), &mut hard_errors);
    assert!(
        hard_errors.is_empty(),
        "cem-ml package manifest must parse before README contract checks: {hard_errors:?}"
    );

    assert_eq!(manifest.package_id.as_deref(), Some("cem-ml"));
    assert_eq!(
        manifest.schema_uri.as_deref(),
        Some("https://cem.dev/ns/cem-ml/1")
    );
    assert_eq!(
        manifest.schema_source.as_deref(),
        Some("schema/cem-ml-generic.cem")
    );
    assert_readme_mentions(&readme, "schema/cem-ml-generic.cem", "schema source");
    assert_readme_mentions(&readme, "bootstrap exception", "schema source exception");

    for content_type in &manifest.content_types {
        assert_readme_mentions(&readme, content_type, "content type");
    }

    for converter in &manifest.converter_endpoints {
        assert_readme_mentions(&readme, &converter.id, "converter id");
        for value in [
            converter.rust_symbol.as_deref(),
            converter.from_content_type.as_deref(),
            converter.from_schema.as_deref(),
            converter.to_content_type.as_deref(),
            converter.to_schema.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            assert_readme_mentions(&readme, value, &format!("converter `{}`", converter.id));
        }
    }

    for path in &manifest.cemt_artifact_paths {
        assert_readme_mentions(&readme, path, "CEMT artifact path");
    }

    for reference_key in &manifest.example_reference_keys {
        let parts = reference_key.split('|').collect::<Vec<_>>();
        assert_eq!(
            parts.len(),
            6,
            "example reference keys must keep id|path|content-type|schema|result|diagnostics shape"
        );
        let path = parts[1];
        let diagnostics = parts[5];
        assert_readme_mentions(&readme, path, "example path");
        for code in diagnostics.split_whitespace() {
            assert_readme_mentions(&readme, code, &format!("example `{path}` diagnostic"));
        }
    }
}

#[test]
fn cem_ml_readme_tracked_work_is_todo_or_waived() {
    let version_dir = schema_packages_root().join("cem-ml/v1");
    let readme_path = version_dir.join("README.md");
    let readme = fs::read_to_string(&readme_path)
        .unwrap_or_else(|error| panic!("{} is not readable: {error}", readme_path.display()));
    let todo_path = workspace_root().join("docs/todo.md");
    let todo = fs::read_to_string(&todo_path)
        .unwrap_or_else(|error| panic!("{} is not readable: {error}", todo_path.display()));

    let tracked_items = readme_tracked_but_not_complete_items(&readme);
    let open_todos = open_todo_checkitems(&todo);
    let waived_items = package_review_waiver_items(&readme);

    let missing_tracking = tracked_items
        .iter()
        .filter(|item| {
            let normalized = normalize_review_tracking_item(item);
            !open_todos.contains(&normalized) && !waived_items.contains(&normalized)
        })
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        missing_tracking.is_empty(),
        "CEM-ML README tracked-but-not-complete items must be represented by open docs/todo.md checkitems or package-local `package-review-waiver` metadata: {}",
        format_list(&missing_tracking)
    );

    let tracked_set = tracked_items
        .iter()
        .map(|item| normalize_review_tracking_item(item))
        .collect::<BTreeSet<_>>();
    let stale_waivers = waived_items
        .difference(&tracked_set)
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        stale_waivers.is_empty(),
        "CEM-ML package-review-waiver metadata must reference current README tracked-but-not-complete items: {}",
        format_list(&stale_waivers)
    );
}

#[test]
fn schema_definition_package_readme_tracks_manifest_contract() {
    let version_dir = schema_packages_root().join("schema/v1");
    let readme_path = version_dir.join("README.md");
    let readme = fs::read_to_string(&readme_path)
        .unwrap_or_else(|error| panic!("{} is not readable: {error}", readme_path.display()));
    let mut hard_errors = Vec::new();
    let manifest = parse_manifest(&version_dir.join("package.cem"), &mut hard_errors);
    assert!(
        hard_errors.is_empty(),
        "schema package manifest must parse before README contract checks: {hard_errors:?}"
    );

    assert_eq!(manifest.package_id.as_deref(), Some("schema"));
    assert_eq!(
        manifest.schema_uri.as_deref(),
        Some("https://cem.dev/ns/schema/1")
    );
    assert_eq!(
        manifest.schema_source.as_deref(),
        Some("schema/cem-schema.cem")
    );
    assert_readme_mentions(&readme, "schema/cem-schema.cem", "schema source");
    assert_readme_mentions(&readme, "bootstrap exception", "schema source exception");

    for content_type in &manifest.content_types {
        assert_readme_mentions(&readme, content_type, "content type");
    }

    assert!(
        manifest.converter_endpoints.is_empty(),
        "schema/v1 must not declare converter edges until the package owns an executable conversion contract"
    );
    assert_readme_mentions(&readme, "no converter edges", "converter edge status");

    assert!(
        manifest.cemt_artifact_paths.is_empty(),
        "schema/v1 must not declare formatter/colorizer artifacts until package-owned CEMT assets exist"
    );
    assert_readme_mentions(
        &readme,
        "package-owned formatter or colorizer CEMT artifacts",
        "CEMT output gap status",
    );

    for reference_key in &manifest.example_reference_keys {
        let parts = reference_key.split('|').collect::<Vec<_>>();
        assert_eq!(
            parts.len(),
            6,
            "example reference keys must keep id|path|content-type|schema|result|diagnostics shape"
        );
        let path = parts[1];
        let diagnostics = parts[5];
        assert_readme_mentions(&readme, path, "example path");
        for code in diagnostics.split_whitespace() {
            assert_readme_mentions(&readme, code, &format!("example `{path}` diagnostic"));
        }
    }
}

#[test]
fn schema_package_metadata_package_readme_tracks_manifest_contract() {
    let version_dir = schema_packages_root().join("schema-package/v1");
    let readme_path = version_dir.join("README.md");
    let readme = fs::read_to_string(&readme_path)
        .unwrap_or_else(|error| panic!("{} is not readable: {error}", readme_path.display()));
    let mut hard_errors = Vec::new();
    let manifest = parse_manifest(&version_dir.join("package.cem"), &mut hard_errors);
    assert!(
        hard_errors.is_empty(),
        "schema-package package manifest must parse before README contract checks: {hard_errors:?}"
    );

    assert_eq!(manifest.package_id.as_deref(), Some("schema-package"));
    assert_eq!(
        manifest.schema_uri.as_deref(),
        Some("https://cem.dev/ns/schema-package/1")
    );
    assert_eq!(
        manifest.schema_source.as_deref(),
        Some("schema/schema-package.cem")
    );
    assert_readme_mentions(&readme, "schema/schema-package.cem", "schema source");
    assert_readme_mentions(
        &readme,
        "cem_ml_schema_package_schema_package_v1",
        "Nx project name",
    );

    for content_type in &manifest.content_types {
        assert_readme_mentions(&readme, content_type, "content type");
    }

    assert!(
        manifest.converter_endpoints.is_empty(),
        "schema-package/v1 must not declare runtime converter edges until the package owns an executable conversion contract"
    );
    assert_readme_mentions(&readme, "no runtime converter", "converter edge status");

    assert!(
        manifest.cemt_artifact_paths.is_empty(),
        "schema-package/v1 must not declare formatter/colorizer artifacts until package-owned CEMT assets exist"
    );
    assert_readme_mentions(
        &readme,
        "no package-owned formatter or colorizer CEMT",
        "CEMT output gap status",
    );
    assert_readme_mentions(
        &readme,
        "validation fixtures, not registered package output",
        "example CEMT fixture status",
    );

    for reference_key in &manifest.example_reference_keys {
        let parts = reference_key.split('|').collect::<Vec<_>>();
        assert_eq!(
            parts.len(),
            6,
            "example reference keys must keep id|path|content-type|schema|result|diagnostics shape"
        );
        let path = parts[1];
        let diagnostics = parts[5];
        assert_readme_mentions(&readme, path, "example path");
        for code in diagnostics.split_whitespace() {
            assert_readme_mentions(&readme, code, &format!("example `{path}` diagnostic"));
        }
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
        schema_uri: manifest.schema_uri,
        content_types: manifest.content_types,
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
        converter_endpoints: manifest.converter_endpoints,
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

fn deferred_cross_package_converter_edges(
    reports: &[SchemaPackageStructureReport],
) -> BTreeSet<String> {
    let schema_owners = schema_uri_owners(reports);
    let content_type_owners = content_type_owners(reports);
    let mut edges = BTreeSet::new();

    for report in reports {
        for converter in &report.converter_endpoints {
            let from_owner = endpoint_owner(
                converter.from_schema.as_deref(),
                converter.from_content_type.as_deref(),
                &schema_owners,
                &content_type_owners,
            )
            .unwrap_or_else(|| "<unknown>".to_owned());
            let to_owner = endpoint_owner(
                converter.to_schema.as_deref(),
                converter.to_content_type.as_deref(),
                &schema_owners,
                &content_type_owners,
            )
            .unwrap_or_else(|| "<unknown>".to_owned());

            if from_owner != report.package_id || to_owner != report.package_id {
                edges.insert(format!(
                    "{}/{}: {} -> {}",
                    report.package_id, converter.id, from_owner, to_owner
                ));
            }
        }
    }

    edges
}

fn schema_uri_owners(reports: &[SchemaPackageStructureReport]) -> BTreeMap<String, String> {
    reports
        .iter()
        .filter_map(|report| {
            report
                .schema_uri
                .as_ref()
                .map(|schema_uri| (schema_uri.clone(), report.package_id.clone()))
        })
        .collect()
}

fn content_type_owners(reports: &[SchemaPackageStructureReport]) -> BTreeMap<String, String> {
    let mut owners = BTreeMap::new();
    for report in reports {
        for content_type in &report.content_types {
            owners.insert(content_type.clone(), report.package_id.clone());
        }
    }
    owners
}

fn endpoint_owner(
    schema: Option<&str>,
    content_type: Option<&str>,
    schema_owners: &BTreeMap<String, String>,
    content_type_owners: &BTreeMap<String, String>,
) -> Option<String> {
    let schema_owner = schema.and_then(|schema| schema_owners.get(schema)).cloned();
    let content_type_owner = content_type
        .map(normalize_content_type_identity)
        .and_then(|content_type| content_type_owners.get(&content_type).cloned());

    match (schema_owner, content_type_owner) {
        (Some(owner), _) | (None, Some(owner)) => Some(owner),
        (None, None) => None,
    }
}

fn schema_package_verify_schema_project_dependencies(
    report: &SchemaPackageStructureReport,
) -> Vec<String> {
    let project_json = read_json_file(&report.version_dir.join("project.json"));
    let verify = project_json
        .pointer("/targets/verify")
        .unwrap_or_else(|| panic!("{} verify target must exist", report.package_id));
    target_dependency_projects(verify, Some("verify"))
        .into_iter()
        .filter(|project| project.starts_with("cem_ml_schema_package_"))
        .collect()
}

fn assert_cli_registry_target_depends_on_all_schema_package_verifies(
    target_name: &str,
    reports: &[SchemaPackageStructureReport],
) {
    let project_json = read_json_file(&workspace_root().join("packages/cem_ml_cli/project.json"));
    let target = project_json
        .pointer(&format!("/targets/{target_name}"))
        .unwrap_or_else(|| panic!("cem_ml_cli target `{target_name}` must exist"));
    let actual_projects = target_dependency_projects(target, Some("verify"))
        .into_iter()
        .collect::<BTreeSet<_>>();
    let expected_projects = reports
        .iter()
        .map(|report| expected_schema_package_project_name(&report.package_id))
        .collect::<BTreeSet<_>>();

    assert_eq!(
        actual_projects, expected_projects,
        "cem_ml_cli:{target_name} must be the final registry pass after every schema-package verify target"
    );
}

fn target_dependency_projects(target: &Value, dependency_target: Option<&str>) -> Vec<String> {
    let Some(dependencies) = target.pointer("/dependsOn").and_then(Value::as_array) else {
        return Vec::new();
    };

    let mut projects = Vec::new();
    for dependency in dependencies {
        let matches_target = match dependency_target {
            Some(expected_target) => match dependency {
                Value::String(text) => text == expected_target,
                Value::Object(_) => json_string(dependency, "/target") == Some(expected_target),
                _ => false,
            },
            None => true,
        };
        if !matches_target {
            continue;
        }

        match dependency {
            Value::Object(_) => {
                if let Some(project_array) =
                    dependency.pointer("/projects").and_then(Value::as_array)
                {
                    projects.extend(
                        project_array
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned),
                    );
                }
            }
            Value::String(text) if dependency_target.is_none() => projects.push(text.clone()),
            _ => {}
        }
    }
    projects.sort();
    projects.dedup();
    projects
}

fn read_json_file(path: &Path) -> Value {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} is not readable: {error}", path.display()));
    serde_json::from_str(&source)
        .unwrap_or_else(|error| panic!("{} is not valid JSON: {error}", path.display()))
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
                if let Some(uri) = attrs.get("uri") {
                    summary.schema_uri = Some(uri.clone());
                }
                if let Some(source) = attrs.get("source") {
                    summary.schema_source = Some(normalize_manifest_path(source));
                }
            }
            "content-type" => {
                if let Some(content_type) = attrs.get("value") {
                    summary
                        .content_types
                        .insert(normalize_content_type_identity(content_type));
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
                let converter =
                    parse_manifest_converter(&document, child_id, child_index + 1, &attrs);
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
                summary.converter_endpoints.push(converter);
            }
            _ => {}
        }
    }

    summary
}

fn parse_manifest_converter(
    document: &CemDocument,
    converter_id: AstNodeId,
    converter_index: usize,
    attrs: &BTreeMap<String, String>,
) -> ManifestConverterEndpoint {
    let mut converter = ManifestConverterEndpoint {
        id: attrs
            .get("id")
            .cloned()
            .unwrap_or_else(|| format!("#{converter_index}")),
        implementation: attrs.get("implementation").cloned(),
        rust_symbol: attrs.get("rust-symbol").cloned(),
        template: attrs
            .get("template")
            .map(|path| normalize_manifest_path(path)),
        ..ManifestConverterEndpoint::default()
    };

    for endpoint_id in element_child_ids(document, converter_id) {
        let Some(local_name) = element_local_name(document, endpoint_id) else {
            continue;
        };
        if local_name != "from" && local_name != "to" {
            continue;
        }

        let endpoint_attrs = collect_attrs(document, endpoint_id);
        let content_type = endpoint_attrs
            .get("content-type")
            .map(|content_type| normalize_content_type_identity(content_type));
        let schema = endpoint_attrs.get("schema").cloned();
        match local_name {
            "from" => {
                converter.from_content_type = content_type;
                converter.from_schema = schema;
            }
            "to" => {
                converter.to_content_type = content_type;
                converter.to_schema = schema;
            }
            _ => {}
        }
    }

    converter
}

fn assert_readme_mentions(readme: &str, value: &str, context: &str) {
    assert!(
        readme.contains(value),
        "README must mention {context} `{value}`"
    );
}

fn readme_tracked_but_not_complete_items(readme: &str) -> Vec<String> {
    markdown_bullets_after_label(readme, "Tracked but not complete:")
}

fn package_review_waiver_items(readme: &str) -> BTreeSet<String> {
    readme
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let rest = trimmed.strip_prefix("<!-- package-review-waiver:")?;
            let item = rest.strip_suffix("-->").unwrap_or(rest).trim();
            (!item.is_empty()).then(|| normalize_review_tracking_item(item))
        })
        .collect()
}

fn open_todo_checkitems(todo: &str) -> BTreeSet<String> {
    let mut items = BTreeSet::new();
    let mut current: Option<String> = None;
    for line in todo.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
            if let Some(item) = current.take() {
                items.insert(normalize_review_tracking_item(&item));
            }
            current = Some(rest.to_owned());
            continue;
        }
        if trimmed.starts_with("- [x] ") || trimmed.starts_with("##") || trimmed.is_empty() {
            if let Some(item) = current.take() {
                items.insert(normalize_review_tracking_item(&item));
            }
            continue;
        }
        if let Some(item) = current.as_mut() {
            item.push(' ');
            item.push_str(trimmed);
        }
    }
    if let Some(item) = current {
        items.insert(normalize_review_tracking_item(&item));
    }
    items
}

fn markdown_bullets_after_label(markdown: &str, label: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current: Option<String> = None;
    let mut in_section = false;
    for line in markdown.lines() {
        let trimmed = line.trim();
        if !in_section {
            in_section = trimmed == label;
            continue;
        }
        if trimmed.starts_with("## ") {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("- ") {
            if let Some(item) = current.take() {
                items.push(item);
            }
            current = Some(rest.to_owned());
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with("<!-- package-review-waiver:") {
            continue;
        }
        if let Some(item) = current.as_mut() {
            if line.starts_with(' ') || line.starts_with('\t') {
                item.push(' ');
                item.push_str(trimmed);
            } else if let Some(item) = current.take() {
                items.push(item);
            }
        }
    }
    if let Some(item) = current {
        items.push(item);
    }
    items
}

fn normalize_review_tracking_item(item: &str) -> String {
    let item = item.trim().trim_end_matches(['.', ';']);
    item.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
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

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("cem-ml package is under the workspace packages directory")
        .to_path_buf()
}

fn normalize_manifest_path(path: &str) -> String {
    let mut normalized = path.trim().replace('\\', "/");
    while let Some(rest) = normalized.strip_prefix("./") {
        normalized = rest.to_owned();
    }
    normalized
}

fn normalize_content_type_identity(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
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
