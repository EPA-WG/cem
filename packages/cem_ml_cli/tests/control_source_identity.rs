use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "cem-control-source-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn cases() -> [(&'static str, &'static str, &'static str); 4] {
    [
        (
            include_str!(
                "../../cem_ml/schema-packages/cli-command/v1/examples/parse-ast.command.json"
            ),
            "application/vnd.cem.cli-command+json",
            "https://cem.dev/ns/cli/command/1",
        ),
        (
            include_str!("../../../examples/projects/portable-config/authored/check.run.json"),
            "application/json",
            "https://cem.dev/ns/cli/run-config/1",
        ),
        (
            include_str!("../../../examples/projects/portable-config/project/card.project.json"),
            "application/vnd.cem.studio-project+json",
            "https://cem.dev/ns/studio/project/1",
        ),
        (
            include_str!("../../../examples/projects/portable-config/project/card.project.cem"),
            "application/vnd.cem.studio-project+cem",
            "https://cem.dev/ns/studio/project/1",
        ),
    ]
}
fn execute(
    dir: &Scratch,
    text: &str,
    mime: &str,
    schema: &str,
    operation: &str,
) -> (std::process::Output, Value) {
    execute_at_level(dir, text, mime, schema, operation, "validate")
}
fn execute_at_level(
    dir: &Scratch,
    text: &str,
    mime: &str,
    schema: &str,
    operation: &str,
    fail_level: &str,
) -> (std::process::Output, Value) {
    let path = dir.0.join("original.control");
    fs::write(&path, text).unwrap();
    let report = dir.0.join("report.json");
    if report.exists() {
        fs::remove_file(&report).unwrap();
    }
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cem-ml"));
    cmd.args([
        operation,
        path.to_str().unwrap(),
        "--content-type",
        mime,
        "--schema",
        schema,
        "--no-color",
        "--report-json",
        report.to_str().unwrap(),
    ]);
    match operation {
        "parse" => {
            cmd.args(["--format", "ast", "--fail-level", fail_level]);
        }
        "inspect" => {
            cmd.args(["--show", "ast", "--format", "json"]);
        }
        "convert" => {
            cmd.args([
                "--to-content-type",
                mime,
                "--to-schema",
                schema,
                "--output-color-type",
                "none",
            ]);
        }
        _ => {
            cmd.args(["--format", "json"]);
        }
    }
    let output = cmd.output().unwrap();
    let value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    (output, value)
}
#[test]
fn authored_control_sources_pass_parse_inspect_validate_check_and_format() {
    let dir = Scratch::new();
    for (text, mime, schema) in cases() {
        for operation in ["parse", "inspect", "validate", "check", "convert"] {
            let (output, report) = execute(&dir, text, mime, schema, operation);
            assert!(
                output.status.success(),
                "{mime} {operation}: {report}; {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                report["diagnostics"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|d| d["severity"] != "error" && d["severity"] != "fatal"),
                "{report}"
            );
            assert!(!output.stdout.is_empty());
            if operation == "convert" {
                let formatted = String::from_utf8(output.stdout).unwrap();
                assert!(execute(&dir, &formatted, mime, schema, "validate")
                    .0
                    .status
                    .success());
            }
        }
    }
}
#[test]
fn control_source_cli_refusals_keep_original_code_and_fail_status() {
    let dir = Scratch::new();
    let duplicate =
        include_str!("../../../examples/projects/portable-config/failures/duplicate.run.json");
    for (index, (text, mime, schema)) in cases().into_iter().enumerate() {
        let (broken, code) = match index {
            0 => (
                text.replace("\"schemaVersion\": 1", "\"schemaVersion\": 2"),
                "cem.cli_command.schema_version_unsupported",
            ),
            1 => (duplicate.to_owned(), "cem.run_config.input_uri_duplicate"),
            2 => (
                text.replace("\"schemaVersion\": 1", "\"schemaVersion\": 2"),
                "cem.studio_project.schema_version_unsupported",
            ),
            _ => (
                text.replace("@schema-version=1", "@schema-version=2"),
                "cem.studio_project.schema_version_unsupported",
            ),
        };
        assert_ne!(broken, text);
        for operation in ["parse", "inspect", "validate", "check", "convert"] {
            let (output, report) = execute(&dir, &broken, mime, schema, operation);
            assert_eq!(
                output.status.code(),
                Some(1),
                "{mime} {operation}: {report}"
            );
            let errors = report["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|d| d["severity"] == "error" || d["severity"] == "fatal")
                .collect::<Vec<_>>();
            assert!(
                errors.iter().any(|d| d["code"] == code
                    && d["uri"] == dir.0.join("original.control").to_str().unwrap()),
                "{report}"
            );
        }
    }
}

#[test]
fn duplicate_run_config_remains_a_preparation_refusal_before_resource_execution() {
    let dir = Scratch::new();
    let path = dir.0.join("duplicate.run.json");
    fs::write(
        &path,
        include_str!("../../../examples/projects/portable-config/failures/duplicate.run.json"),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args([
            "validate",
            "--config",
            path.to_str().unwrap(),
            "--config-content-type",
            "application/json",
            "--config-schema",
            "https://cem.dev/ns/cli/run-config/1",
            "--no-color",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cem.run_config.input_uri_duplicate"));
}

#[test]
fn parse_only_policy_preserves_semantic_diagnostics_without_a_source_tree() {
    let dir = Scratch::new();
    let (text, mime, schema) = cases()[0];
    let invalid = text.replace("\"schemaVersion\": 1", "\"schemaVersion\": 2");
    let (output, report) = execute_at_level(&dir, &invalid, mime, schema, "parse", "parse");
    assert!(
        output.status.success(),
        "parse-only policy remains distinct from validation"
    );
    let errors = report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["severity"] == "error")
        .collect::<Vec<_>>();
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0]["code"],
        "cem.cli_command.schema_version_unsupported"
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("cem-tree-inspection"));
}
