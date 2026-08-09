use cem_ml::real::RealCemMlEngine;
use cem_ml::schema::package_sources::builtin_schema_package_source;
use cem_ml::schema::registry::{
    schema_package_examples_from_package_sources, SchemaPackageExampleExpectedResult,
    SCSS_CONTENT_TYPE, SCSS_SCHEMA_URI,
};
use cem_ml_cli::{cli, dispatch};
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};

const EXIT_OK: u8 = 0;
const EXIT_HARD_FAILURE: u8 = 1;
const CEM_SCHEMA_PACKAGE_URI: &str = "https://cem.dev/ns/schema-package/1";
const CEM_SCHEMA_PACKAGE_CONTENT_TYPE: &str = "application/vnd.cem.schema-package+cem";

#[derive(Debug)]
struct CliOutput {
    exit_code: u8,
    stdout: String,
    stderr: String,
}

fn run(engine: &RealCemMlEngine, args: &[String]) -> CliOutput {
    let parsed =
        cli::Cli::try_parse_from(std::iter::once("cem-ml").chain(args.iter().map(String::as_str)))
            .unwrap_or_else(|err| panic!("parse in-process cem-ml args {args:?}: {err}"));
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut streams = dispatch::Streams {
        stdout: &mut stdout,
        stderr: &mut stderr,
        quiet: parsed.quiet,
        no_color: parsed.no_color,
    };
    let outcome = dispatch::dispatch(engine, parsed, &mut streams);
    CliOutput {
        exit_code: outcome.exit_code,
        stdout: String::from_utf8(stdout).expect("SCSS CLI stdout is UTF-8"),
        stderr: String::from_utf8(stderr).expect("SCSS CLI stderr is UTF-8"),
    }
}

fn workspace_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn write_source(name: &str, source: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("cem-ml-cli-scss-{}", std::process::id()));
    fs::create_dir_all(&root).expect("create SCSS test directory");
    let path = root.join(name);
    fs::write(&path, source).expect("write SCSS test source");
    path
}

fn validate_source(path: &Path, content_type: &str) -> CliOutput {
    run(
        &RealCemMlEngine::new(),
        &[
            "validate".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            "--content-type".to_owned(),
            content_type.to_owned(),
            "--schema".to_owned(),
            SCSS_SCHEMA_URI.to_owned(),
            path.to_str().expect("SCSS test path is UTF-8").to_owned(),
        ],
    )
}

fn report(output: &CliOutput) -> serde_json::Value {
    serde_json::from_str(output.stdout.trim()).unwrap_or_else(|err| {
        panic!(
            "SCSS CLI stdout is validation JSON: {err}\nstdout:\n{}\nstderr:\n{}",
            output.stdout, output.stderr
        )
    })
}

fn has_diagnostic(report: &serde_json::Value, code: &str) -> bool {
    report["diagnostics"].as_array().is_some_and(|diagnostics| {
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic["code"] == code)
    })
}

#[test]
fn validate_scss_source_uses_native_parser() {
    let path = write_source(
        "native-parser.scss",
        "$space: 0.5rem;\n@mixin inset($amount: $space) { padding: $amount; }\n.card { @include inset(); }\n",
    );
    let output = validate_source(&path, SCSS_CONTENT_TYPE);
    let report = report(&output);

    assert_eq!(output.exit_code, EXIT_OK, "{}", output.stderr);
    assert_eq!(report["summary"]["hardViolationCount"], 0);
    assert!(report["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn validate_scss_source_reports_parse_error() {
    let path = write_source("indented-syntax.scss", ".card\n  color: red\n");
    let output = validate_source(&path, SCSS_CONTENT_TYPE);
    let report = report(&output);

    assert_eq!(output.exit_code, EXIT_HARD_FAILURE, "{}", output.stderr);
    assert!(has_diagnostic(&report, "cem.scss.parse_error"));
}

#[test]
fn validate_scss_source_reports_deprecated_import() {
    let path = write_source(
        "deprecated-import.scss",
        "@import \"tokens\";\n.card { color: red; }\n",
    );
    let output = validate_source(&path, SCSS_CONTENT_TYPE);
    let report = report(&output);

    assert_eq!(output.exit_code, EXIT_OK, "{}", output.stderr);
    assert!(has_diagnostic(&report, "cem.scss.import_deprecated"));
}

#[test]
fn schema_owned_scss_examples_validate_through_cli() {
    let package = builtin_schema_package_source("scss").expect("SCSS package is built in");
    let examples =
        schema_package_examples_from_package_sources(package).expect("SCSS package examples parse");
    assert!(!examples.is_empty(), "SCSS package declares examples");

    let engine = RealCemMlEngine::new();
    for example in examples {
        let relative = if example.path.starts_with("schema-packages/") {
            format!("packages/cem_ml/{}", example.path)
        } else {
            example.path.clone()
        };
        let path = workspace_path(&relative);
        assert!(path.exists(), "SCSS example exists: {}", path.display());
        let output = run(
            &engine,
            &[
                "validate".to_owned(),
                "--format".to_owned(),
                "json".to_owned(),
                "--content-type".to_owned(),
                example.content_type.clone(),
                "--schema".to_owned(),
                example.schema.clone(),
                path.to_str()
                    .expect("SCSS example path is UTF-8")
                    .to_owned(),
            ],
        );
        let report = report(&output);
        let expected_exit = match example.expected_result {
            SchemaPackageExampleExpectedResult::Pass => EXIT_OK,
            SchemaPackageExampleExpectedResult::Fail => EXIT_HARD_FAILURE,
        };
        assert_eq!(
            output.exit_code, expected_exit,
            "SCSS example `{}` stderr:\n{}",
            example.id, output.stderr
        );
        for code in example.expected_diagnostic_codes {
            assert!(
                has_diagnostic(&report, &code),
                "SCSS example `{}` expected diagnostic `{code}`:\n{}",
                example.id,
                output.stdout
            );
        }
    }
}

#[test]
fn builtin_scss_manifest_validates_through_cli() {
    let path = workspace_path("packages/cem_ml/schema-packages/scss/v1/package.cem");
    let output = run(
        &RealCemMlEngine::new(),
        &[
            "validate".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            "--content-type".to_owned(),
            CEM_SCHEMA_PACKAGE_CONTENT_TYPE.to_owned(),
            "--schema".to_owned(),
            CEM_SCHEMA_PACKAGE_URI.to_owned(),
            path.to_str()
                .expect("SCSS manifest path is UTF-8")
                .to_owned(),
        ],
    );
    let report = report(&output);

    assert_eq!(output.exit_code, EXIT_OK, "{}", output.stderr);
    assert_eq!(report["summary"]["hardViolationCount"], 0);
    assert!(report["diagnostics"].as_array().unwrap().is_empty());
}
