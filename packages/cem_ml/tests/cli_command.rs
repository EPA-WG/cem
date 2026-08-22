use cem_ml::cli_command::{
    parse_cli_command, CliCommandProjection, CLI_COMMAND_CONTENT_TYPE, CLI_COMMAND_JSON_SCHEMA_URI,
    CLI_COMMAND_SCHEMA_URI,
};
use cem_ml::schema::package_sources::{
    builtin_schema_package_artifact_source, builtin_schema_package_source,
};
use cem_ml::schema::registry::{
    schema_package_examples_from_package_sources, SchemaPackageExampleExpectedResult,
};

const VALID_JSON: &[u8] =
    include_bytes!("../schema-packages/cli-command/v1/examples/parse-ast.command.json");
const CLI_COMMAND_RUST_SOURCE: &str = include_str!("../src/cli_command.rs");
const CLI_COMMAND_CEM_SCHEMA_SOURCE: &str =
    include_str!("../schema-packages/cli-command/v1/schema/cli-command.cem");

#[test]
fn cli_command_v1_identities_are_fixed() {
    assert_eq!(
        CLI_COMMAND_CONTENT_TYPE,
        "application/vnd.cem.cli-command+json"
    );
    assert_eq!(CLI_COMMAND_SCHEMA_URI, "https://cem.dev/ns/cli/command/1");
    assert_eq!(
        CLI_COMMAND_JSON_SCHEMA_URI,
        "https://cem.dev/schema/cli/command.schema.json"
    );
}

#[test]
fn cli_command_package_examples_and_json_schema_are_manifest_indexed() {
    let package =
        builtin_schema_package_source("cli-command").expect("built-in CLI command schema package");
    assert_eq!(
        package.schema_path,
        "schema-packages/cli-command/v1/schema/cli-command.cem"
    );

    let examples =
        schema_package_examples_from_package_sources(package).expect("CLI command examples");
    assert_eq!(examples.len(), 7);
    assert!(examples
        .iter()
        .all(|example| example.schema == CLI_COMMAND_SCHEMA_URI));
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
        "cli-command",
        "schema-packages/cli-command/v1/schema/cli-command.schema.json",
    )
    .expect("embedded CLI command JSON Schema");
    let schema: serde_json::Value =
        serde_json::from_str(artifact.source).expect("valid CLI command JSON Schema");
    assert_eq!(
        schema.get("$id").and_then(serde_json::Value::as_str),
        Some(CLI_COMMAND_JSON_SCHEMA_URI)
    );
}

#[test]
fn cli_command_schema_owns_every_native_diagnostic_code() {
    let native_codes = cli_command_diagnostic_codes(CLI_COMMAND_RUST_SOURCE);
    let schema_codes = cli_command_diagnostic_codes(CLI_COMMAND_CEM_SCHEMA_SOURCE);
    assert_eq!(native_codes, schema_codes);
}

fn cli_command_diagnostic_codes(source: &str) -> std::collections::BTreeSet<&str> {
    source
        .split('"')
        .filter(|value| value.starts_with("cem.cli_command."))
        .collect()
}

#[test]
fn cli_command_json_round_trips_deterministically() {
    let command = parse_cli_command(VALID_JSON, CLI_COMMAND_CONTENT_TYPE, CLI_COMMAND_SCHEMA_URI)
        .expect("valid authored CLI command");
    assert_eq!(command.schema_version, 1);
    assert_eq!(command.command_schema_version, 1);
    assert_eq!(command.common_version, "0.1.0");
    assert_eq!(command.binary_name, "cem-ml");
    assert_eq!(command.argv[0], "parse");
    assert!(!command.argv.iter().any(|argument| argument == "cem-ml"));

    let first = command
        .serialize(CliCommandProjection::Json)
        .expect("CLI command JSON projection");
    let reparsed = parse_cli_command(
        first.as_bytes(),
        CLI_COMMAND_CONTENT_TYPE,
        CLI_COMMAND_SCHEMA_URI,
    )
    .expect("serialized CLI command parses");
    let second = reparsed
        .serialize(CliCommandProjection::Json)
        .expect("re-serialized CLI command projection");
    assert_eq!(command, reparsed);
    assert_eq!(first, second);
}

#[test]
fn cli_command_rejects_forward_versions_wrong_binary_and_controls() {
    for (path, code) in [
        (
            "schema-packages/cli-command/v1/examples/invalid-forward-version.command.json",
            "cem.cli_command.schema_version_unsupported",
        ),
        (
            "schema-packages/cli-command/v1/examples/invalid-binary.command.json",
            "cem.cli_command.binary_name_invalid",
        ),
        (
            "schema-packages/cli-command/v1/examples/invalid-command-schema-version.command.json",
            "cem.cli_command.command_schema_version_unsupported",
        ),
        (
            "schema-packages/cli-command/v1/examples/invalid-common-version.command.json",
            "cem.cli_command.common_version_invalid",
        ),
        (
            "schema-packages/cli-command/v1/examples/invalid-control.command.json",
            "cem.cli_command.argument_control",
        ),
    ] {
        let bytes = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
            .expect("CLI command rejection fixture");
        let error = parse_cli_command(&bytes, CLI_COMMAND_CONTENT_TYPE, CLI_COMMAND_SCHEMA_URI)
            .expect_err("CLI command rejection fixture must fail");
        assert_eq!(error.code, code, "fixture {path}: {error}");
    }
}

#[test]
fn cli_command_rejects_unknown_schema_and_content_type() {
    let schema_error = parse_cli_command(
        VALID_JSON,
        CLI_COMMAND_CONTENT_TYPE,
        "https://cem.dev/ns/cli/command/2",
    )
    .expect_err("forward schema identity must fail");
    assert_eq!(
        schema_error.code,
        "cem.cli_command.schema_identity_unsupported"
    );

    let content_error = parse_cli_command(VALID_JSON, "application/json", CLI_COMMAND_SCHEMA_URI)
        .expect_err("generic JSON must not silently select the CLI command model");
    assert_eq!(
        content_error.code,
        "cem.cli_command.content_type_unsupported"
    );
}
