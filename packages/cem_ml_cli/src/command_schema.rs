//! Versioned command grammar projected from the canonical native Clap graph.

use crate::cli::Cli;
use cem_ml::capability::{
    capability_manifest, CapabilityAvailability, CapabilityManifest, CapabilityOperation,
    CapabilityRequest, RuntimeKind,
};
use clap::{Arg, ArgAction, Command, CommandFactory};
use serde::Serialize;

pub const COMMAND_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedCommandSchema {
    pub schema_version: u16,
    pub common_version: String,
    pub binary_name: String,
    pub root_arguments: Vec<CommandArgument>,
    pub global_arguments: Vec<CommandArgument>,
    pub commands: Vec<CommandDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandDescriptor {
    pub name: String,
    pub about: Option<String>,
    pub long_about: Option<String>,
    pub capability_operation: Option<CapabilityOperation>,
    pub availability: RuntimeAvailability,
    pub arguments: Vec<CommandArgument>,
    pub groups: Vec<CommandArgumentGroup>,
    pub subcommands: Vec<CommandDescriptor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeAvailability {
    pub native: CapabilityAvailability,
    pub wasm_node: CapabilityAvailability,
    pub wasm_browser_worker: CapabilityAvailability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandArgument {
    pub id: String,
    pub long: Option<String>,
    pub short: Option<char>,
    pub positional_index: Option<usize>,
    pub value_names: Vec<String>,
    pub action: String,
    pub min_values: usize,
    pub max_values: Option<usize>,
    pub required: bool,
    pub global: bool,
    pub hidden: bool,
    pub allow_hyphen_values: bool,
    pub value_delimiter: Option<char>,
    pub default_values: Vec<String>,
    pub possible_values: Vec<String>,
    pub conflicts_with: Vec<String>,
    pub help: Option<String>,
    pub long_help: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandArgumentGroup {
    pub id: String,
    pub arguments: Vec<String>,
    pub required: bool,
    pub multiple: bool,
}

struct CapabilityMatrix {
    native: CapabilityManifest,
    wasm_node: CapabilityManifest,
    wasm_browser_worker: CapabilityManifest,
}

pub fn shared_command_schema() -> SharedCommandSchema {
    let mut root = Cli::command();
    root.build();
    let capabilities = CapabilityMatrix::new();
    SharedCommandSchema {
        schema_version: COMMAND_SCHEMA_VERSION,
        common_version: cem_ml::VERSION.to_owned(),
        binary_name: root.get_bin_name().unwrap_or(root.get_name()).to_owned(),
        root_arguments: root
            .get_arguments()
            .filter(|argument| !argument.is_global_set())
            .map(|argument| command_argument(&root, argument))
            .collect(),
        global_arguments: root
            .get_arguments()
            .filter(|argument| argument.is_global_set())
            .map(|argument| command_argument(&root, argument))
            .collect(),
        commands: root
            .get_subcommands()
            .map(|command| command_descriptor(command, command.get_name(), &capabilities))
            .collect(),
    }
}

impl CapabilityMatrix {
    fn new() -> Self {
        Self {
            native: manifest(RuntimeKind::Native),
            wasm_node: manifest(RuntimeKind::WasmNode),
            wasm_browser_worker: manifest(RuntimeKind::WasmBrowserWorker),
        }
    }

    fn availability(&self, operation: CapabilityOperation) -> RuntimeAvailability {
        RuntimeAvailability {
            native: self.native.availability(operation),
            wasm_node: self.wasm_node.availability(operation),
            wasm_browser_worker: self.wasm_browser_worker.availability(operation),
        }
    }
}

fn manifest(runtime: RuntimeKind) -> CapabilityManifest {
    capability_manifest(CapabilityRequest {
        runtime,
        target_identity: "command-schema".to_owned(),
        abi_identity: format!("command-schema-v{COMMAND_SCHEMA_VERSION}"),
        debug_control_active: false,
    })
    .expect("fixed command-schema identities satisfy the common capability bounds")
}

fn command_descriptor(
    command: &Command,
    top_level_name: &str,
    capabilities: &CapabilityMatrix,
) -> CommandDescriptor {
    let operation = capability_operation(top_level_name);
    CommandDescriptor {
        name: command.get_name().to_owned(),
        about: styled(command.get_about()),
        long_about: styled(command.get_long_about()),
        capability_operation: operation,
        availability: operation
            .map(|operation| capabilities.availability(operation))
            .unwrap_or_else(|| host_command_availability(top_level_name)),
        arguments: command
            .get_arguments()
            .filter(|argument| !argument.is_global_set())
            .map(|argument| command_argument(command, argument))
            .collect(),
        groups: command
            .get_groups()
            .filter_map(command_argument_group)
            .collect(),
        subcommands: command
            .get_subcommands()
            .map(|child| command_descriptor(child, top_level_name, capabilities))
            .collect(),
    }
}

fn capability_operation(name: &str) -> Option<CapabilityOperation> {
    match name {
        "parse" => Some(CapabilityOperation::Parse),
        "validate" => Some(CapabilityOperation::Validate),
        "check" => Some(CapabilityOperation::Check),
        "inspect" => Some(CapabilityOperation::Inspect),
        "convert" => Some(CapabilityOperation::Convert),
        "query" => Some(CapabilityOperation::Query),
        "transform" => Some(CapabilityOperation::Transform),
        "trace" => Some(CapabilityOperation::Trace),
        "version" => Some(CapabilityOperation::VersionCapabilities),
        "bench" => Some(CapabilityOperation::Bench),
        "fixture" => Some(CapabilityOperation::Fixture),
        "schema" => Some(CapabilityOperation::SchemaMutation),
        "plugin" => Some(CapabilityOperation::PluginMutation),
        _ => None,
    }
}

fn host_command_availability(name: &str) -> RuntimeAvailability {
    use CapabilityAvailability::{Available, Unavailable};
    match name {
        "help" => RuntimeAvailability {
            native: Available,
            wasm_node: Available,
            wasm_browser_worker: Available,
        },
        _ => RuntimeAvailability {
            native: Available,
            wasm_node: Unavailable,
            wasm_browser_worker: Unavailable,
        },
    }
}

fn command_argument(command: &Command, argument: &Arg) -> CommandArgument {
    let range = argument
        .get_num_args()
        .unwrap_or_else(|| match argument.get_action() {
            ArgAction::Set | ArgAction::Append => 1.into(),
            _ => 0.into(),
        });
    let maximum = range.max_values();
    CommandArgument {
        id: argument.get_id().to_string(),
        long: argument.get_long().map(str::to_owned),
        short: argument.get_short(),
        positional_index: argument.get_index(),
        value_names: argument
            .get_value_names()
            .into_iter()
            .flatten()
            .map(ToString::to_string)
            .collect(),
        action: action_name(argument.get_action()).to_owned(),
        min_values: range.min_values(),
        max_values: (maximum != usize::MAX).then_some(maximum),
        required: argument.is_required_set(),
        global: argument.is_global_set(),
        hidden: argument.is_hide_set(),
        allow_hyphen_values: argument.is_allow_hyphen_values_set(),
        value_delimiter: argument.get_value_delimiter(),
        default_values: argument
            .get_default_values()
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect(),
        possible_values: argument
            .get_possible_values()
            .into_iter()
            .filter(|value| !value.is_hide_set())
            .flat_map(|value| {
                value
                    .get_name_and_aliases()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .collect(),
        conflicts_with: command
            .get_arg_conflicts_with(argument)
            .into_iter()
            .map(|conflict| conflict.get_id().to_string())
            .collect(),
        help: styled(argument.get_help()),
        long_help: styled(argument.get_long_help()),
    }
}

fn command_argument_group(group: &clap::ArgGroup) -> Option<CommandArgumentGroup> {
    let arguments: Vec<_> = group.get_args().map(ToString::to_string).collect();
    if arguments.is_empty() {
        return None;
    }
    let mut group = group.clone();
    Some(CommandArgumentGroup {
        id: group.get_id().to_string(),
        arguments,
        required: group.is_required_set(),
        multiple: group.is_multiple(),
    })
}

fn action_name(action: &ArgAction) -> &'static str {
    match action {
        ArgAction::Set => "set",
        ArgAction::Append => "append",
        ArgAction::SetTrue => "set-true",
        ArgAction::SetFalse => "set-false",
        ArgAction::Count => "count",
        ArgAction::Help => "help",
        ArgAction::HelpShort => "help-short",
        ArgAction::HelpLong => "help-long",
        ArgAction::Version => "version",
        _ => "unknown",
    }
}

fn styled(value: Option<&clap::builder::StyledStr>) -> Option<String> {
    value.map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn schema_projects_portable_commands_groups_and_common_availability() {
        let schema = shared_command_schema();
        assert_eq!(schema.schema_version, COMMAND_SCHEMA_VERSION);
        assert_eq!(schema.common_version, cem_ml::VERSION);
        assert_eq!(schema.binary_name, "cem-ml");

        for name in [
            "parse",
            "validate",
            "check",
            "inspect",
            "convert",
            "query",
            "transform",
            "trace",
            "version",
        ] {
            let command = schema
                .commands
                .iter()
                .find(|command| command.name == name)
                .unwrap_or_else(|| panic!("missing shared command `{name}`"));
            assert_eq!(
                command.availability.wasm_node,
                CapabilityAvailability::Available
            );
            assert_eq!(
                command.availability.wasm_browser_worker,
                CapabilityAvailability::Available
            );
        }

        let query = schema
            .commands
            .iter()
            .find(|command| command.name == "query")
            .unwrap();
        assert!(query.groups.iter().any(|group| {
            group.id == "query_source"
                && group.required
                && !group.multiple
                && group.arguments == ["query", "query_file"]
        }));
        let output = query
            .arguments
            .iter()
            .find(|argument| argument.id == "output")
            .unwrap();
        assert_eq!(output.default_values, ["terminal"]);
        assert_eq!(output.possible_values, ["terminal", "cem", "json"]);
    }

    #[test]
    fn serialized_schema_uses_stable_versioned_field_names() {
        let value = serde_json::to_value(shared_command_schema()).unwrap();
        assert_eq!(value["schemaVersion"], COMMAND_SCHEMA_VERSION);
        assert_eq!(value["commonVersion"], cem_ml::VERSION);
        assert_eq!(value["binaryName"], "cem-ml");
        assert!(value["rootArguments"].is_array());
        assert!(value["globalArguments"].is_array());
        assert!(value["commands"].is_array());
    }

    #[test]
    fn npm_roundtrip_fixture_arguments_are_accepted_by_native_clap() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../cem-ml-cli-npm/tests/command-roundtrip.fixture.json"
        ))
        .unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let arguments = case["argv"].as_array().unwrap();
            let arguments: Vec<_> = std::iter::once("cem-ml")
                .chain(arguments.iter().map(|value| value.as_str().unwrap()))
                .collect();
            Cli::try_parse_from(arguments)
                .unwrap_or_else(|error| panic!("native fixture `{name}` failed: {error}"));
        }
        for case in fixture["invalidCases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let arguments = case["argv"].as_array().unwrap();
            let arguments: Vec<_> = std::iter::once("cem-ml")
                .chain(arguments.iter().map(|value| value.as_str().unwrap()))
                .collect();
            assert!(
                Cli::try_parse_from(arguments).is_err(),
                "native invalid fixture `{name}` was unexpectedly accepted"
            );
        }
    }
}
