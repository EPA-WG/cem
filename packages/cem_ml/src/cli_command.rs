//! Portable authored CEM-ML CLI command v1 resource.
//!
//! This is the durable command-side configuration boundary used by Studio and
//! other hosts. It records versioned literal argv and deliberately excludes
//! lowered requests, effective configuration, resolver snapshots, and run
//! plans. Command grammar validation remains owned by the generated CLI
//! command schema in the host that will execute the resource.

use serde::{Deserialize, Serialize};

pub const CLI_COMMAND_CONTENT_TYPE: &str = "application/vnd.cem.cli-command+json";
pub const CLI_COMMAND_SCHEMA_URI: &str = "https://cem.dev/ns/cli/command/1";
pub const CLI_COMMAND_JSON_SCHEMA_URI: &str = "https://cem.dev/schema/cli/command.schema.json";
pub const CLI_COMMAND_SCHEMA_VERSION: u32 = 1;
pub const CLI_COMMAND_GRAMMAR_SCHEMA_VERSION: u32 = 1;
pub const CLI_COMMAND_BINARY_NAME: &str = "cem-ml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliCommandProjection {
    Json,
}

impl CliCommandProjection {
    pub const fn content_type(self) -> &'static str {
        match self {
            Self::Json => CLI_COMMAND_CONTENT_TYPE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CliCommandResource {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub schema_version: u32,
    pub command_schema_version: u32,
    pub common_version: String,
    pub binary_name: String,
    pub argv: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliCommandError {
    pub code: &'static str,
    pub message: String,
    pub field_path: Option<String>,
}

impl std::fmt::Display for CliCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(field_path) = &self.field_path {
            write!(formatter, "{} at {field_path}: {}", self.code, self.message)
        } else {
            write!(formatter, "{}: {}", self.code, self.message)
        }
    }
}

impl std::error::Error for CliCommandError {}

impl CliCommandResource {
    pub fn serialize(&self, projection: CliCommandProjection) -> Result<String, CliCommandError> {
        validate_cli_command(self)?;
        match projection {
            CliCommandProjection::Json => {
                let mut output = serde_json::to_string_pretty(self).map_err(|error| {
                    cli_command_error(
                        "cem.cli_command.json_serialization_failed",
                        format!("CLI command JSON serialization failed: {error}"),
                        None,
                    )
                })?;
                output.push('\n');
                Ok(output)
            }
        }
    }
}

pub fn parse_cli_command(
    bytes: &[u8],
    content_type: &str,
    schema: &str,
) -> Result<CliCommandResource, CliCommandError> {
    if schema.trim() != CLI_COMMAND_SCHEMA_URI {
        return Err(cli_command_error(
            "cem.cli_command.schema_identity_unsupported",
            format!(
                "CLI command schema `{}` is unsupported; expected `{CLI_COMMAND_SCHEMA_URI}`",
                schema.trim()
            ),
            Some("$schema"),
        ));
    }
    if content_type_essence(content_type) != CLI_COMMAND_CONTENT_TYPE {
        return Err(cli_command_error(
            "cem.cli_command.content_type_unsupported",
            format!(
                "CLI command content type `{content_type}` is unsupported; expected `{CLI_COMMAND_CONTENT_TYPE}`"
            ),
            None,
        ));
    }
    let command = serde_json::from_slice::<CliCommandResource>(bytes).map_err(|error| {
        cli_command_error(
            "cem.cli_command.invalid_json",
            format!("CLI command JSON could not be parsed: {error}"),
            None,
        )
    })?;
    validate_cli_command(&command)?;
    Ok(command)
}

pub fn validate_cli_command(command: &CliCommandResource) -> Result<(), CliCommandError> {
    if command.schema != CLI_COMMAND_SCHEMA_URI {
        return Err(cli_command_error(
            "cem.cli_command.schema_identity_unsupported",
            format!(
                "CLI command JSON $schema `{}` is unsupported; expected `{CLI_COMMAND_SCHEMA_URI}`",
                command.schema
            ),
            Some("$schema"),
        ));
    }
    if command.schema_version != CLI_COMMAND_SCHEMA_VERSION {
        return Err(cli_command_error(
            "cem.cli_command.schema_version_unsupported",
            format!(
                "CLI command schemaVersion {} is unsupported; expected {CLI_COMMAND_SCHEMA_VERSION}",
                command.schema_version
            ),
            Some("schemaVersion"),
        ));
    }
    if command.command_schema_version != CLI_COMMAND_GRAMMAR_SCHEMA_VERSION {
        return Err(cli_command_error(
            "cem.cli_command.command_schema_version_unsupported",
            format!(
                "CLI command commandSchemaVersion {} is unsupported; expected {CLI_COMMAND_GRAMMAR_SCHEMA_VERSION}",
                command.command_schema_version
            ),
            Some("commandSchemaVersion"),
        ));
    }
    if !is_semver(&command.common_version) {
        return Err(cli_command_error(
            "cem.cli_command.common_version_invalid",
            format!(
                "CLI command commonVersion `{}` is not canonical SemVer",
                command.common_version
            ),
            Some("commonVersion"),
        ));
    }
    if command.binary_name != CLI_COMMAND_BINARY_NAME {
        return Err(cli_command_error(
            "cem.cli_command.binary_name_invalid",
            format!(
                "CLI command binaryName `{}` is unsupported; expected `{CLI_COMMAND_BINARY_NAME}`",
                command.binary_name
            ),
            Some("binaryName"),
        ));
    }
    if command.argv.is_empty() {
        return Err(cli_command_error(
            "cem.cli_command.argv_empty",
            "CLI command argv must contain at least one argument",
            Some("argv"),
        ));
    }
    for (index, argument) in command.argv.iter().enumerate() {
        if argument.chars().any(is_unsupported_control) {
            return Err(cli_command_error(
                "cem.cli_command.argument_control",
                format!("CLI command argv[{index}] contains an unsupported control character"),
                Some(&format!("argv[{index}]")),
            ));
        }
    }
    Ok(())
}

fn is_unsupported_control(character: char) -> bool {
    let code = character as u32;
    (code < 0x20 && !matches!(character, '\t' | '\n' | '\r')) || code == 0x7f
}

fn is_semver(value: &str) -> bool {
    let (without_build, build) = value
        .split_once('+')
        .map_or((value, None), |(core, tail)| (core, Some(tail)));
    if build.is_some_and(|tail| !identifier_sequence_is_valid(tail, false)) {
        return false;
    }
    let (core, prerelease) = without_build
        .split_once('-')
        .map_or((without_build, None), |(core, tail)| (core, Some(tail)));
    if prerelease.is_some_and(|tail| !identifier_sequence_is_valid(tail, true)) {
        return false;
    }
    let mut parts = core.split('.');
    let valid = [parts.next(), parts.next(), parts.next()]
        .into_iter()
        .all(|part| part.is_some_and(numeric_identifier_is_valid));
    valid && parts.next().is_none()
}

fn numeric_identifier_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value == "0" || !value.starts_with('0'))
}

fn identifier_sequence_is_valid(value: &str, reject_numeric_leading_zero: bool) -> bool {
    !value.is_empty()
        && value.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && (!reject_numeric_leading_zero
                    || !part.bytes().all(|byte| byte.is_ascii_digit())
                    || numeric_identifier_is_valid(part))
        })
}

fn content_type_essence(value: &str) -> &str {
    value.split(';').next().unwrap_or(value).trim()
}

fn cli_command_error(
    code: &'static str,
    message: impl Into<String>,
    field_path: Option<&str>,
) -> CliCommandError {
    CliCommandError {
        code,
        message: message.into(),
        field_path: field_path.map(str::to_owned),
    }
}
