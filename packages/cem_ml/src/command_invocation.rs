//! Rust-owned lowering from the generated universal command parser into the
//! command-service request and post-terminal presentation boundaries.
//!
//! Hosts provide only an environment and immutable resource snapshots. The
//! option identifiers consumed here are emitted from the native Clap graph;
//! Node never owns a parallel operation, run-plan, report, or exit-code map.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::command_service::{
    sha256_hex, CommandPolicyStampV1, CommandProjectRevisionV1,
    CommandResourceVersionV1, CommandRunPlanV1, CommandServiceRequestV1,
    CommandServiceResultV1, CommandTransformResultV1, CommandTransformSourceV1,
    CommandUriMapV1, PortableOperationRequestV1, PortableOperationResultV1,
    VirtualResourceV1, COMMAND_SERVICE_PROTOCOL_VERSION,
};
use crate::diagnostics::Severity;
use crate::engine::{
    FailLevel, FormatIdentity, InputFormat, ParseProjection, TransformTemplateEntrypoint,
};
use crate::report_projection::project_report_v1;
use crate::resolver::{has_uri_scheme, is_windows_drive_path, ResolvePurpose};
use crate::run_config::{
    self, InputSpec, NormalizedDiagnosticsMode, NormalizedPrimaryKind,
    NormalizedReportProjection, NormalizedRunPlan, NormalizedRunPlanRequest, ResolverSpec,
    RunConfig, RunConfigDefaults, RunConfigParseRequest, ScopeConfig,
};
use crate::transform_config::{
    parse_transform_graph_config, TransformGraphNodeKind, TransformGraphParseRequest,
    TRANSFORM_CONFIG_SCHEMA_URI,
};
use crate::transform_graph_request::{
    resolve_transform_graph_reference, transform_graph_reference_matches,
};

pub const COMMAND_SCHEMA_VERSION: u16 = 1;
pub const STDOUT_RESOURCE_URI: &str = "cem-stdio://stdout";
pub const INLINE_QUERY_URI_PREFIX: &str = "cem+inline://query/";
pub const INLINE_TEMPLATE_URI: &str = "cem+inline://transform/template.cem-ql";
pub const COPYRIGHT_NOTICE: &str =
    "Copyright (c) 2026 Sasha Firsov <https://github.com/sashafirsov>";

const EXIT_USAGE: u8 = 2;
const EXIT_SCHEMA: u8 = 3;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(untagged)]
pub enum ParsedCommandValueV1 {
    String(String),
    Boolean(bool),
    Number(u64),
    Strings(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ParsedCommandInvocationV1 {
    pub schema_version: u16,
    pub common_version: String,
    pub command_path: Vec<String>,
    #[serde(default)]
    pub global_options: BTreeMap<String, ParsedCommandValueV1>,
    #[serde(default)]
    pub options: BTreeMap<String, ParsedCommandValueV1>,
    #[serde(default)]
    pub positionals: BTreeMap<String, ParsedCommandValueV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandInvocationEnvironmentV1 {
    pub request_id: String,
    pub project_id: String,
    pub project_revision: u64,
    #[serde(default)]
    pub resource_revision: u64,
    pub cwd: String,
    pub resolver_policy_stamp: String,
    pub safety_policy_stamp: String,
    pub budget_policy_stamp: String,
    #[serde(default)]
    pub stdout_is_terminal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum CommandInvocationResourceRequirementKindV1 {
    Read,
    Glob,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandInvocationResourceRequirementV1 {
    pub kind: CommandInvocationResourceRequirementKindV1,
    pub uri: String,
    pub purpose: ResolvePurpose,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<FormatIdentity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum CommandPresentationTargetKindV1 {
    Stdout,
    Stderr,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandPresentationRouteV1 {
    pub target: CommandPresentationTargetKindV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    pub projection: NormalizedReportProjection,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandPresentationPlanV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_artifact_uri: Option<String>,
    #[serde(default)]
    pub report_routes: Vec<CommandPresentationRouteV1>,
    #[serde(default)]
    pub diagnostics_to_stderr: bool,
    #[serde(default)]
    pub graph_outputs_to_stdout: bool,
    #[serde(default)]
    pub version_to_stdout: bool,
    #[serde(default)]
    pub source_map_summary: bool,
    #[serde(default)]
    pub quiet: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandInvocationV1 {
    pub request: CommandServiceRequestV1,
    pub presentation: CommandPresentationPlanV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandInvocationErrorV1 {
    pub code: String,
    pub message: String,
    pub exit_code: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(
    tag = "state",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum CommandInvocationBuildResponseV1 {
    NeedsResources {
        requirements: Vec<CommandInvocationResourceRequirementV1>,
    },
    Ready {
        invocation: Box<CommandInvocationV1>,
    },
    Error {
        error: CommandInvocationErrorV1,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandPresentationWriteV1 {
    pub target: CommandPresentationTargetKindV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandPresentationV1 {
    #[serde(default)]
    pub writes: Vec<CommandPresentationWriteV1>,
}

pub fn build_command_invocation_v1(
    parsed: ParsedCommandInvocationV1,
    environment: CommandInvocationEnvironmentV1,
    mut resources: CommandUriMapV1<VirtualResourceV1>,
) -> CommandInvocationBuildResponseV1 {
    match build_command_invocation(&parsed, &environment, &mut resources) {
        Ok(BuildOutcome::Needs(requirements)) => {
            CommandInvocationBuildResponseV1::NeedsResources { requirements }
        }
        Ok(BuildOutcome::Ready(invocation)) => CommandInvocationBuildResponseV1::Ready {
            invocation,
        },
        Err(error) => CommandInvocationBuildResponseV1::Error { error },
    }
}

enum BuildOutcome {
    Needs(Vec<CommandInvocationResourceRequirementV1>),
    Ready(Box<CommandInvocationV1>),
}

fn build_command_invocation(
    parsed: &ParsedCommandInvocationV1,
    environment: &CommandInvocationEnvironmentV1,
    resources: &mut CommandUriMapV1<VirtualResourceV1>,
) -> Result<BuildOutcome, CommandInvocationErrorV1> {
    validate_parsed_command(parsed)?;
    let command = parsed
        .command_path
        .first()
        .map(String::as_str)
        .ok_or_else(|| usage("cem.command.required_command", "a CEM-ML command is required"))?;
    if command == "version" {
        let request = command_request(
            environment,
            resources,
            PortableOperationRequestV1::VersionCapabilities,
            CommandRunPlanV1::Null(()),
        );
        return Ok(BuildOutcome::Ready(Box::new(CommandInvocationV1 {
            request,
            presentation: CommandPresentationPlanV1 {
                stdout_artifact_uri: None,
                report_routes: Vec::new(),
                diagnostics_to_stderr: false,
                graph_outputs_to_stdout: false,
                version_to_stdout: true,
                source_map_summary: false,
                quiet: false,
            },
        })));
    }

    let run_config_uri = option_string(parsed, "config")
        .filter(|_| command != "transform")
        .map(|uri| resolve_host_uri(&environment.cwd, &uri));
    if let Some(uri) = run_config_uri.as_deref() {
        if !resources.contains_key(uri) {
            return Ok(BuildOutcome::Needs(vec![requirement(
                CommandInvocationResourceRequirementKindV1::Read,
                uri,
                ResolvePurpose::Config,
                Some(run_config_identity(parsed, uri)),
            )]));
        }
    }

    let (mut plan, presentation) = command_run_plan(parsed, environment, resources, command)?;
    let operation = command_operation(parsed, environment, resources, command, &plan)?;
    let mut requirements = operation_resource_requirements(&operation, &plan, resources)?;
    requirements.extend(scope_resource_requirements(&plan, resources));
    requirements.sort_by(|left, right| left.uri.cmp(&right.uri));
    requirements.dedup_by(|left, right| left.kind == right.kind && left.uri == right.uri);
    if !requirements.is_empty() {
        return Ok(BuildOutcome::Needs(requirements));
    }

    // Resource snapshots are immutable request input. Inline query/template
    // sources are inserted while lowering and are hashed by the same path.
    plan.diagnostics_mode.report_destinations.clear();
    let request = command_request(environment, resources, operation, plan.into());
    Ok(BuildOutcome::Ready(Box::new(CommandInvocationV1 {
        request,
        presentation,
    })))
}

fn validate_parsed_command(
    parsed: &ParsedCommandInvocationV1,
) -> Result<(), CommandInvocationErrorV1> {
    if parsed.schema_version != COMMAND_SCHEMA_VERSION {
        return Err(usage(
            "cem.command.schema_version",
            format!(
                "command schema {} is unsupported; expected {COMMAND_SCHEMA_VERSION}",
                parsed.schema_version
            ),
        ));
    }
    if parsed.common_version != crate::VERSION {
        return Err(CommandInvocationErrorV1 {
            code: "cem.command.common_version".to_owned(),
            message: format!(
                "parsed command common version `{}` does not match runtime `{}`",
                parsed.common_version,
                crate::VERSION
            ),
            exit_code: EXIT_SCHEMA,
        });
    }
    if parsed.meta_action.is_some() {
        return Err(usage(
            "cem.command.meta_action",
            "help/version meta actions must be rendered before command lowering",
        ));
    }
    Ok(())
}

fn command_run_plan(
    parsed: &ParsedCommandInvocationV1,
    environment: &CommandInvocationEnvironmentV1,
    resources: &mut CommandUriMapV1<VirtualResourceV1>,
    command: &str,
) -> Result<(NormalizedRunPlan, CommandPresentationPlanV1), CommandInvocationErrorV1> {
    let input_scope = input_scope(parsed)?;
    let output_scope = output_scope(parsed, command, environment.stdout_is_terminal)?;
    let from_format_hint = option_string(parsed, "from_format")
        .map(|value| enum_value::<InputFormat>("from_format", &value))
        .transpose()?;
    let to_format_fallback = (command == "convert")
        .then(|| option_string(parsed, "to_format").unwrap_or_else(|| "dom-json".to_owned()));
    let mut config = load_run_config(parsed, environment, resources, command)?;
    append_context_config(parsed, &mut config)?;

    let input_uris = command_input_uris(parsed, environment, command)?;
    let mut input_records = option_strings(parsed, "input_specs");
    input_records.extend(
        input_uris
            .iter()
            .map(|uri| format!("uri={}", record_value(uri))),
    );

    let mut output_records = if supports_run_options(command) {
        option_strings(parsed, "output_specs")
    } else {
        Vec::new()
    };
    let primary_output = primary_output_destination(parsed, command);
    let mut stdout_artifact_uri = None;
    let graph_transform = command == "transform" && option_string(parsed, "config").is_some();
    if command_has_primary_output(command) && !graph_transform {
        let destination = primary_output.unwrap_or_else(|| STDOUT_RESOURCE_URI.to_owned());
        if destination == STDOUT_RESOURCE_URI {
            stdout_artifact_uri = Some(destination.clone());
        }
        let input_ref = input_uris.first().map(String::as_str);
        output_records.push(output_record(input_ref, &destination));
    } else if matches!(command, "validate" | "check") {
        output_records.push(output_record(None, STDOUT_RESOURCE_URI));
        stdout_artifact_uri = Some(STDOUT_RESOURCE_URI.to_owned());
    }

    let diagnostics = diagnostics_mode(parsed, command)?;
    let config_bytes = (!config.inputs.is_empty()
        || !config.outputs.is_empty()
        || !config.schema_packages.is_empty()
        || !config.resolvers.is_empty()
        || config.scheduler != Default::default())
    .then(|| serde_json::to_vec(&config))
    .transpose()
    .map_err(|error| internal_error("cem.command.run_config_serialize", error.to_string()))?;
    let config_base_uri = option_string(parsed, "config")
        .filter(|_| command != "transform")
        .map(|uri| resolve_host_uri(&environment.cwd, &uri));
    let plan = run_config::parse_normalized_run_plan(NormalizedRunPlanRequest {
        config_bytes,
        config_identity: FormatIdentity {
            content_type: Some("application/json".to_owned()),
            schema: Some(run_config::RUN_CONFIG_SCHEMA_URI.to_owned()),
            base_uri: config_base_uri.clone(),
            ..FormatIdentity::default()
        },
        config_base_uri,
        defaults: RunConfigDefaults {
            input_scope,
            output_scope,
            from_format_hint,
            to_format_fallback,
        },
        input_records,
        output_records,
        diagnostics_mode: diagnostics,
        command_profile: Some(command.to_owned()),
    })
    .map_err(|error| CommandInvocationErrorV1 {
        code: error.code.to_owned(),
        message: error.message,
        exit_code: EXIT_USAGE,
    })?;
    if let Some(diagnostic) = plan
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Err(CommandInvocationErrorV1 {
            code: diagnostic.code.clone(),
            message: diagnostic.message.clone(),
            exit_code: EXIT_USAGE,
        });
    }

    let report_routes = report_routes(parsed, environment, command)?;
    Ok((
        plan,
        CommandPresentationPlanV1 {
            stdout_artifact_uri,
            diagnostics_to_stderr: report_routes.is_empty() && !global_bool(parsed, "quiet"),
            report_routes,
            graph_outputs_to_stdout: command == "transform"
                && option_string(parsed, "config").is_some(),
            version_to_stdout: false,
            source_map_summary: option_bool(parsed, "source_map_summary"),
            quiet: global_bool(parsed, "quiet"),
        },
    ))
}

fn load_run_config(
    parsed: &ParsedCommandInvocationV1,
    environment: &CommandInvocationEnvironmentV1,
    resources: &CommandUriMapV1<VirtualResourceV1>,
    command: &str,
) -> Result<RunConfig, CommandInvocationErrorV1> {
    let Some(raw_uri) = option_string(parsed, "config").filter(|_| command != "transform") else {
        return Ok(RunConfig::default());
    };
    let uri = resolve_host_uri(&environment.cwd, &raw_uri);
    let resource = resources
        .get(&uri)
        .ok_or_else(|| usage("cem.command.resource_missing", format!("resource `{uri}` is missing")))?;
    let parsed_config = run_config::parse_run_config(RunConfigParseRequest {
        bytes: resource.bytes.clone(),
        identity: run_config_identity(parsed, &uri),
        base_uri: Some(uri),
    })
    .map_err(|error| CommandInvocationErrorV1 {
        code: error.code.to_owned(),
        message: error.message,
        exit_code: EXIT_SCHEMA,
    })?;
    Ok(parsed_config.config)
}

fn append_context_config(
    parsed: &ParsedCommandInvocationV1,
    config: &mut RunConfig,
) -> Result<(), CommandInvocationErrorV1> {
    for uri in option_strings(parsed, "schema_packages") {
        config.schema_packages.push(InputSpec {
            uri,
            root_scope: ScopeConfig::default(),
        });
    }
    for (field, read, write) in [
        ("resolver_read_maps", true, false),
        ("resolver_write_maps", false, true),
    ] {
        for mapping in option_strings(parsed, field) {
            let (uri_prefix, local_root) = split_pair(field, &mapping)?;
            config.resolvers.push(ResolverSpec {
                uri_prefix,
                local_root,
                read,
                write,
            });
        }
    }
    Ok(())
}

fn command_input_uris(
    parsed: &ParsedCommandInvocationV1,
    environment: &CommandInvocationEnvironmentV1,
    command: &str,
) -> Result<Vec<String>, CommandInvocationErrorV1> {
    let raw = match command {
        "validate" | "check" => positional_strings(parsed, "inputs"),
        "query" | "transform" => positional_string(parsed, "data").into_iter().collect(),
        "parse" | "inspect" | "convert" | "trace" => {
            positional_string(parsed, "input").into_iter().collect()
        }
        _ => Vec::new(),
    };
    Ok(raw
        .into_iter()
        .map(|uri| resolve_host_uri(&environment.cwd, &uri))
        .collect())
}

fn command_operation(
    parsed: &ParsedCommandInvocationV1,
    environment: &CommandInvocationEnvironmentV1,
    resources: &mut CommandUriMapV1<VirtualResourceV1>,
    command: &str,
    plan: &NormalizedRunPlan,
) -> Result<PortableOperationRequestV1, CommandInvocationErrorV1> {
    let single_input = || {
        if plan.inputs.len() != 1 {
            Err(usage(
                "cem.command.input_count",
                format!("{command} requires exactly one input; found {}", plan.inputs.len()),
            ))
        } else {
            Ok(plan.inputs[0].input_id.clone())
        }
    };
    let all_inputs = || {
        if plan.inputs.is_empty() {
            Err(usage(
                "cem.command.input_required",
                format!("{command} requires at least one input"),
            ))
        } else {
            Ok(plan
                .inputs
                .iter()
                .map(|input| input.input_id.clone())
                .collect::<Vec<_>>())
        }
    };
    match command {
        "parse" => {
            let raw = option_string(parsed, "format").unwrap_or_else(|| "ast".to_owned());
            let projection = match raw.as_str() {
                "ast-json" => ParseProjection::Ast,
                "events-json" => ParseProjection::Events,
                _ => enum_value("format", &raw)?,
            };
            Ok(PortableOperationRequestV1::Parse {
                input_id: single_input()?,
                projection,
                preserve_source_offsets: option_bool(parsed, "preserve_source_offsets"),
            })
        }
        "validate" => Ok(PortableOperationRequestV1::Validate {
            input_ids: all_inputs()?,
            projection: enum_value(
                "format",
                &option_string(parsed, "format").unwrap_or_else(|| "text".to_owned()),
            )?,
        }),
        "check" => Ok(PortableOperationRequestV1::Check {
            input_ids: all_inputs()?,
            projection: enum_value(
                "format",
                &option_string(parsed, "format").unwrap_or_else(|| "text".to_owned()),
            )?,
            zero_hard_violations: option_bool(parsed, "zero_hard_violations"),
        }),
        "inspect" => Ok(PortableOperationRequestV1::Inspect {
            input_id: single_input()?,
            show: enum_value(
                "show",
                &option_string(parsed, "show").unwrap_or_else(|| "summary".to_owned()),
            )?,
        }),
        "convert" => Ok(PortableOperationRequestV1::Convert {
            input_id: single_input()?,
            to_format: enum_value(
                "to_format",
                &option_string(parsed, "to_format").unwrap_or_else(|| "dom-json".to_owned()),
            )?,
            preserve_source_offsets: option_bool(parsed, "preserve_source_offsets"),
        }),
        "query" => {
            let content_type = required_option(parsed, "query_content_type")?;
            let schema = option_string(parsed, "query_schema");
            let query_uri = if let Some(source) = option_string(parsed, "query") {
                let suffix = query_suffix(&content_type);
                let uri = format!("{INLINE_QUERY_URI_PREFIX}{suffix}");
                resources.insert(
                    uri.clone(),
                    VirtualResourceV1 {
                        bytes: source.into_bytes(),
                        identity: Some(FormatIdentity {
                            content_type: Some(content_type.clone()),
                            schema,
                            base_uri: Some(uri.clone()),
                            ..FormatIdentity::default()
                        }),
                    },
                );
                uri
            } else {
                let raw = required_option(parsed, "query_file")?;
                let uri = resolve_host_uri(&environment.cwd, &raw);
                if let Some(resource) = resources.get_mut(&uri) {
                    resource.identity = Some(FormatIdentity {
                        content_type: Some(content_type.clone()),
                        schema,
                        base_uri: Some(uri.clone()),
                        ..FormatIdentity::default()
                    });
                }
                uri
            };
            Ok(PortableOperationRequestV1::Query {
                data_input_id: single_input()?,
                query_uri,
                output: enum_value(
                    "output",
                    &option_string(parsed, "output").unwrap_or_else(|| "terminal".to_owned()),
                )?,
            })
        }
        "transform" => {
            if let Some(raw) = option_string(parsed, "config") {
                return Ok(PortableOperationRequestV1::Transform {
                    source: CommandTransformSourceV1::Graph {
                        config_uri: resolve_host_uri(&environment.cwd, &raw),
                    },
                    params: BTreeMap::new(),
                    template_entrypoint: TransformTemplateEntrypoint::implicit(),
                    preserve_source_offsets: false,
                });
            }
            let template_uri = if let Some(raw) = option_string(parsed, "template") {
                let uri = resolve_host_uri(&environment.cwd, &raw);
                if let Some(resource) = resources.get_mut(&uri) {
                    resource.identity = Some(FormatIdentity {
                        content_type: option_string(parsed, "template_content_type")
                            .or_else(|| run_config::infer_content_type_from_path(&uri)),
                        schema: option_string(parsed, "template_schema"),
                        base_uri: Some(uri.clone()),
                        ..FormatIdentity::default()
                    });
                }
                uri
            } else {
                let expression = required_option(parsed, "template_expression")?;
                resources.insert(
                    INLINE_TEMPLATE_URI.to_owned(),
                    VirtualResourceV1 {
                        bytes: expression.into_bytes(),
                        identity: Some(FormatIdentity {
                            content_type: Some(option_string(parsed, "template_content_type").unwrap_or_else(
                                || crate::schema::registry::CEM_QL_EXPRESSION_CONTENT_TYPE.to_owned(),
                            )),
                            schema: Some(option_string(parsed, "template_schema").unwrap_or_else(|| {
                                crate::schema::registry::CEM_QL_EXPRESSION_SCHEMA_URI.to_owned()
                            })),
                            base_uri: Some(INLINE_TEMPLATE_URI.to_owned()),
                            ..FormatIdentity::default()
                        }),
                    },
                );
                INLINE_TEMPLATE_URI.to_owned()
            };
            Ok(PortableOperationRequestV1::Transform {
                source: CommandTransformSourceV1::Direct {
                    data_input_id: single_input()?,
                    template_uri,
                },
                params: parse_params(&option_strings(parsed, "params"))?,
                template_entrypoint: option_string(parsed, "template_entrypoint")
                    .map(TransformTemplateEntrypoint::named)
                    .unwrap_or_else(TransformTemplateEntrypoint::implicit),
                preserve_source_offsets: false,
            })
        }
        "trace" => Ok(PortableOperationRequestV1::Trace {
            input_id: single_input()?,
            projection: enum_value(
                "format",
                &option_string(parsed, "format").unwrap_or_else(|| "json".to_owned()),
            )?,
        }),
        _ => Err(usage(
            "cem.command.unavailable",
            format!("command `{command}` is unavailable in the portable runtime"),
        )),
    }
}

fn operation_resource_requirements(
    operation: &PortableOperationRequestV1,
    plan: &NormalizedRunPlan,
    resources: &CommandUriMapV1<VirtualResourceV1>,
) -> Result<Vec<CommandInvocationResourceRequirementV1>, CommandInvocationErrorV1> {
    let mut requirements = Vec::new();
    let mut add_input = |input_id: &str| {
        if let Some(input) = plan.inputs.iter().find(|input| input.input_id == input_id) {
            let uri = input
                .resolved_uri
                .as_deref()
                .unwrap_or(&input.declared_uri);
            add_read_requirement(
                &mut requirements,
                resources,
                uri,
                ResolvePurpose::Input,
                Some(input.identity.clone()),
            );
        }
    };
    match operation {
        PortableOperationRequestV1::Parse { input_id, .. }
        | PortableOperationRequestV1::Inspect { input_id, .. }
        | PortableOperationRequestV1::Convert { input_id, .. }
        | PortableOperationRequestV1::Trace { input_id, .. } => add_input(input_id),
        PortableOperationRequestV1::Validate { input_ids, .. }
        | PortableOperationRequestV1::Check { input_ids, .. } => {
            for input_id in input_ids {
                add_input(input_id);
            }
        }
        PortableOperationRequestV1::Query {
            data_input_id,
            query_uri,
            ..
        } => {
            add_input(data_input_id);
            add_read_requirement(
                &mut requirements,
                resources,
                query_uri,
                ResolvePurpose::Query,
                None,
            );
        }
        PortableOperationRequestV1::Transform { source, .. } => match source {
            CommandTransformSourceV1::Direct {
                data_input_id,
                template_uri,
            } => {
                add_input(data_input_id);
                add_read_requirement(
                    &mut requirements,
                    resources,
                    template_uri,
                    ResolvePurpose::Template,
                    None,
                );
            }
            CommandTransformSourceV1::Graph { config_uri } => {
                if !resources.contains_key(config_uri) {
                    add_read_requirement(
                        &mut requirements,
                        resources,
                        config_uri,
                        ResolvePurpose::Config,
                        Some(FormatIdentity {
                            content_type: Some("application/cem+xml".to_owned()),
                            schema: Some(TRANSFORM_CONFIG_SCHEMA_URI.to_owned()),
                            base_uri: Some(config_uri.clone()),
                            ..FormatIdentity::default()
                        }),
                    );
                } else {
                    requirements.extend(graph_requirements(config_uri, resources)?);
                }
            }
        },
        PortableOperationRequestV1::VersionCapabilities => {}
    }
    Ok(requirements)
}

fn graph_requirements(
    config_uri: &str,
    resources: &CommandUriMapV1<VirtualResourceV1>,
) -> Result<Vec<CommandInvocationResourceRequirementV1>, CommandInvocationErrorV1> {
    let resource = resources
        .get(config_uri)
        .expect("graph config presence checked before discovery");
    let response = parse_transform_graph_config(TransformGraphParseRequest {
        bytes: resource.bytes.clone(),
        identity: resource.identity.clone().unwrap_or_else(|| FormatIdentity {
            content_type: Some("application/cem+xml".to_owned()),
            schema: Some(TRANSFORM_CONFIG_SCHEMA_URI.to_owned()),
            base_uri: Some(config_uri.to_owned()),
            ..FormatIdentity::default()
        }),
        base_uri: Some(config_uri.to_owned()),
    })
    .map_err(|error| CommandInvocationErrorV1 {
        code: error.code.to_owned(),
        message: error.message,
        exit_code: EXIT_SCHEMA,
    })?;
    if let Some(diagnostic) = response.diagnostics.first() {
        return Err(CommandInvocationErrorV1 {
            code: diagnostic.code.clone(),
            message: diagnostic.message.clone(),
            exit_code: EXIT_SCHEMA,
        });
    }
    let mut requirements = Vec::new();
    for node in response.graph.nodes {
        let references = match node.kind {
            TransformGraphNodeKind::Import => vec![(node.src, ResolvePurpose::Input, node.content_type)],
            TransformGraphNodeKind::Transform => {
                vec![(node.src, ResolvePurpose::Template, node.template_content_type)]
            }
            TransformGraphNodeKind::ImportMapRewrite => vec![
                (node.source_map, ResolvePurpose::ModuleMap, Some("application/json".to_owned())),
                (node.target_map, ResolvePurpose::ModuleMap, Some("application/json".to_owned())),
            ],
            TransformGraphNodeKind::Join | TransformGraphNodeKind::Export => Vec::new(),
        };
        for (reference, purpose, content_type) in references {
            let Some(reference) = reference else { continue };
            let uri = resolve_transform_graph_reference(config_uri, &reference);
            let is_pattern = reference.contains('*') || reference.contains('{');
            let satisfied = if is_pattern {
                resources.keys().any(|candidate| {
                    transform_graph_reference_matches(&uri, candidate).unwrap_or(false)
                })
            } else {
                resources.contains_key(&uri)
            };
            if !satisfied {
                requirements.push(requirement(
                    if is_pattern {
                        CommandInvocationResourceRequirementKindV1::Glob
                    } else {
                        CommandInvocationResourceRequirementKindV1::Read
                    },
                    &uri,
                    purpose,
                    content_type.map(|content_type| FormatIdentity {
                        content_type: Some(content_type),
                        base_uri: Some(uri.clone()),
                        ..FormatIdentity::default()
                    }),
                ));
            }
        }
    }
    Ok(requirements)
}

fn scope_resource_requirements(
    plan: &NormalizedRunPlan,
    resources: &CommandUriMapV1<VirtualResourceV1>,
) -> Vec<CommandInvocationResourceRequirementV1> {
    let mut requirements = Vec::new();
    for package in &plan.schema_packages {
        let uri = package
            .resolved_uri
            .as_deref()
            .unwrap_or(&package.declared_uri);
        add_read_requirement(
            &mut requirements,
            resources,
            uri,
            ResolvePurpose::Input,
            Some(package.identity.clone()),
        );
    }
    for scope in plan
        .inputs
        .iter()
        .map(|input| &input.root_scope)
        .chain(plan.schema_packages.iter().map(|package| &package.root_scope))
    {
        if let Some(module_map) = scope.module_map.as_ref() {
            let uri = module_map
                .resolved_uri
                .as_deref()
                .unwrap_or(&module_map.declared_uri);
            add_read_requirement(
                &mut requirements,
                resources,
                uri,
                ResolvePurpose::ModuleMap,
                Some(FormatIdentity {
                    content_type: module_map.content_type.clone(),
                    base_uri: module_map.base_uri.clone(),
                    ..FormatIdentity::default()
                }),
            );
        }
    }
    requirements
}

fn command_request(
    environment: &CommandInvocationEnvironmentV1,
    resources: &CommandUriMapV1<VirtualResourceV1>,
    operation: PortableOperationRequestV1,
    run_plan: CommandRunPlanV1,
) -> CommandServiceRequestV1 {
    let versions = resources
        .iter()
        .map(|(uri, resource)| {
            (
                uri.clone(),
                CommandResourceVersionV1 {
                    revision: environment.resource_revision,
                    sha256: sha256_hex(&resource.bytes),
                },
            )
        })
        .collect::<CommandUriMapV1<_>>();
    CommandServiceRequestV1 {
        protocol_version: COMMAND_SERVICE_PROTOCOL_VERSION,
        request_id: environment.request_id.clone(),
        project: CommandProjectRevisionV1 {
            project_id: environment.project_id.clone(),
            revision: environment.project_revision,
        },
        resource_versions: versions,
        operation,
        run_plan,
        resources: resources.clone(),
        policy_stamp: CommandPolicyStampV1 {
            resolver: environment.resolver_policy_stamp.clone(),
            safety: environment.safety_policy_stamp.clone(),
            budget: environment.budget_policy_stamp.clone(),
        },
    }
}

pub fn project_command_presentation_v1(
    plan: &CommandPresentationPlanV1,
    result: &CommandServiceResultV1,
) -> Result<CommandPresentationV1, CommandInvocationErrorV1> {
    let mut writes = Vec::new();
    if plan.diagnostics_to_stderr && !plan.quiet && !result.diagnostics.items.is_empty() {
        let mut body = String::new();
        for diagnostic in &result.diagnostics.items {
            let severity = match diagnostic.severity {
                Severity::Info => "info",
                Severity::Warning => "warning",
                Severity::Error => "error",
                Severity::Fatal => "fatal",
            };
            body.push_str(&format!(
                "{}:{}:{}: {severity}: {} [{}]\n",
                diagnostic.uri.as_deref().unwrap_or("<unknown>"),
                diagnostic.line.unwrap_or(0),
                diagnostic.column.unwrap_or(0),
                diagnostic.message,
                diagnostic.code,
            ));
        }
        writes.push(CommandPresentationWriteV1 {
            target: CommandPresentationTargetKindV1::Stderr,
            uri: None,
            bytes: body.into_bytes(),
        });
    }
    if plan.version_to_stdout {
        writes.push(CommandPresentationWriteV1 {
            target: CommandPresentationTargetKindV1::Stdout,
            uri: None,
            bytes: format!("cem-ml {}\n{COPYRIGHT_NOTICE}\n", crate::VERSION).into_bytes(),
        });
    }
    if !plan.report_routes.is_empty() {
        let report = result
            .report
            .as_ref()
            .and_then(|payload| match payload {
                crate::command_service::CommandPayloadV1::Inline { value } => Some(value),
                crate::command_service::CommandPayloadV1::Artifact { .. } => None,
            })
            .ok_or_else(|| {
                internal_error(
                    "cem.command.presentation_report_missing",
                    "terminal result does not contain an inline report",
                )
            })?;
        for route in &plan.report_routes {
            let projected = project_report_v1(report, route.projection).map_err(|message| {
                internal_error("cem.command.presentation_report", message.to_string())
            })?;
            writes.push(CommandPresentationWriteV1 {
                target: route.target,
                uri: route.uri.clone(),
                bytes: projected.bytes,
            });
        }
    }
    if plan.graph_outputs_to_stdout {
        if let Some(crate::command_service::CommandPayloadV1::Inline {
            value: PortableOperationResultV1::Transform(CommandTransformResultV1::Graph(graph)),
        }) = result.result.as_ref()
        {
            for artifact in graph
                .artifacts
                .iter()
                .filter(|artifact| artifact.destination.is_none())
            {
                let mut bytes = document_value_bytes(&artifact.primary)?;
                if !bytes.ends_with(b"\n") {
                    bytes.push(b'\n');
                }
                writes.push(CommandPresentationWriteV1 {
                    target: CommandPresentationTargetKindV1::Stdout,
                    uri: None,
                    bytes,
                });
            }
        }
    }
    Ok(CommandPresentationV1 { writes })
}

fn document_value_bytes(value: &Value) -> Result<Vec<u8>, CommandInvocationErrorV1> {
    if let Some(content) = value.as_str().or_else(|| value.get("content").and_then(Value::as_str)) {
        return Ok(content.as_bytes().to_vec());
    }
    serde_json::to_vec_pretty(value)
        .map_err(|error| internal_error("cem.command.presentation_serialize", error.to_string()))
}

fn diagnostics_mode(
    parsed: &ParsedCommandInvocationV1,
    command: &str,
) -> Result<NormalizedDiagnosticsMode, CommandInvocationErrorV1> {
    let fail_level = option_string(parsed, "fail_level")
        .map(|value| enum_value::<FailLevel>("fail_level", &value))
        .transpose()?
        .unwrap_or(match command {
            "parse" => FailLevel::Parse,
            _ => FailLevel::Validate,
        });
    let projection = match command {
        "validate" | "check" => report_projection(
            &option_string(parsed, "format").unwrap_or_else(|| "text".to_owned()),
        )?,
        _ => NormalizedReportProjection::Cem,
    };
    Ok(NormalizedDiagnosticsMode {
        fail_level,
        primary_kind: if matches!(command, "validate" | "check") {
            NormalizedPrimaryKind::Report
        } else {
            NormalizedPrimaryKind::Content
        },
        report_projection: projection,
        report_destinations: Vec::new(),
        observe_events_destination: global_string(parsed, "observe_events"),
        quiet: global_bool(parsed, "quiet"),
        verbose: global_bool(parsed, "verbose"),
        no_color: global_bool(parsed, "no_color"),
    })
}

fn input_scope(parsed: &ParsedCommandInvocationV1) -> Result<ScopeConfig, CommandInvocationErrorV1> {
    let transform_data = parsed.command_path.first().is_some_and(|command| command == "transform");
    Ok(ScopeConfig {
        default_content_type: if transform_data {
            option_string(parsed, "data_content_type")
        } else {
            option_string(parsed, "content_type")
        },
        schema: if transform_data {
            option_string(parsed, "data_schema")
        } else {
            option_string(parsed, "schema")
        },
        default_namespace: option_string(parsed, "default_namespace"),
        namespaces: parse_pairs("namespaces", &option_strings(parsed, "namespaces"))?,
        module_map: option_string(parsed, "module_map"),
        version_pins: parse_pairs("version_pins", &option_strings(parsed, "version_pins"))?,
        base_uri: option_string(parsed, "base_uri"),
        policy: option_string(parsed, "scope_policy"),
        budgets: parse_pairs("scope_budgets", &option_strings(parsed, "scope_budgets"))?,
        ..ScopeConfig::default()
    })
}

fn output_scope(
    parsed: &ParsedCommandInvocationV1,
    command: &str,
    stdout_is_terminal: bool,
) -> Result<ScopeConfig, CommandInvocationErrorV1> {
    let mut scope = ScopeConfig::default();
    if command == "convert" {
        scope.default_content_type = option_string(parsed, "to_content_type");
        scope.schema = option_string(parsed, "to_schema");
        scope.output_color_type = option_string(parsed, "output_color_type");
        scope.cemt_formatter = option_string(parsed, "cemt_formatter");
        scope.cemt_formatter_profile = option_string(parsed, "cemt_formatter_profile");
        scope.cemt_formatter_options =
            parse_pairs("cemt_formatter_options", &option_strings(parsed, "cemt_formatter_options"))?;
        scope.cemt_colorizer = option_string(parsed, "cemt_colorizer");
        scope.cemt_color_profile = option_string(parsed, "cemt_color_profile");
        if option_bool(parsed, "tabular") {
            scope.output_color_type.get_or_insert_with(|| "ansi-256".to_owned());
            scope
                .cemt_formatter_profile
                .get_or_insert_with(|| "tabular".to_owned());
            scope
                .cemt_color_profile
                .get_or_insert_with(|| "terminal".to_owned());
        }
    } else if command == "transform" {
        scope.default_content_type = option_string(parsed, "to_content_type");
        scope.schema = option_string(parsed, "to_schema");
        scope.output_color_type = option_string(parsed, "output_color_type");
    } else if command == "parse" || command == "inspect" {
        scope.cemt_formatter_profile = Some("tabular".to_owned());
        if stdout_is_terminal && !global_bool(parsed, "no_color") {
            scope.cemt_color_profile = Some("terminal".to_owned());
            scope.output_color_type = Some("ansi-256".to_owned());
        } else {
            scope.output_color_type = Some("none".to_owned());
        }
    } else if command == "trace" {
        let format = option_string(parsed, "format").unwrap_or_else(|| "json".to_owned());
        scope.default_content_type = Some(match format.as_str() {
            "xml" => "application/xml",
            "cem" => "application/cem+xml",
            "html" => "text/html",
            "text" => "text/plain",
            _ => "application/json",
        }
        .to_owned());
    }
    Ok(scope)
}

fn report_routes(
    parsed: &ParsedCommandInvocationV1,
    environment: &CommandInvocationEnvironmentV1,
    command: &str,
) -> Result<Vec<CommandPresentationRouteV1>, CommandInvocationErrorV1> {
    let basename = match command {
        "convert" => "cem-ml.convert.report",
        "transform" => "cem-ml.transform.report",
        "query" => "cem-ml.query.report",
        _ => "cem-ml.report",
    };
    let mut routes = Vec::new();
    if let Some(path) = option_string(parsed, "report") {
        let projection = report_projection(
            &option_string(parsed, "report_format").unwrap_or_else(|| "cem".to_owned()),
        )?;
        routes.push(file_report_route(
            environment,
            &path,
            basename,
            projection,
        ));
    }
    if let Some(path) = option_string(parsed, "report_json") {
        routes.push(file_report_route(
            environment,
            &path,
            basename,
            NormalizedReportProjection::Json,
        ));
    }
    if let Some(path) = option_string(parsed, "report_md") {
        routes.push(file_report_route(
            environment,
            &path,
            basename,
            NormalizedReportProjection::Markdown,
        ));
    }
    Ok(routes)
}

fn file_report_route(
    environment: &CommandInvocationEnvironmentV1,
    raw: &str,
    basename: &str,
    projection: NormalizedReportProjection,
) -> CommandPresentationRouteV1 {
    let extension = match projection {
        NormalizedReportProjection::Json => "json",
        NormalizedReportProjection::Markdown => "md",
        NormalizedReportProjection::Xml => "xml",
        NormalizedReportProjection::Html => "html",
        NormalizedReportProjection::Text => "txt",
        NormalizedReportProjection::Cem => "cem",
    };
    let path = Path::new(raw);
    let target = if path.extension().is_some() {
        raw.to_owned()
    } else {
        path.join(format!("{basename}.{extension}"))
            .to_string_lossy()
            .into_owned()
    };
    CommandPresentationRouteV1 {
        target: CommandPresentationTargetKindV1::File,
        uri: Some(resolve_host_uri(&environment.cwd, &target)),
        projection,
    }
}

fn primary_output_destination(parsed: &ParsedCommandInvocationV1, command: &str) -> Option<String> {
    if command_has_primary_output(command) {
        option_string(parsed, "out")
    } else {
        None
    }
}

fn command_has_primary_output(command: &str) -> bool {
    matches!(command, "parse" | "inspect" | "convert" | "query" | "trace")
        || command == "transform"
}

fn supports_run_options(command: &str) -> bool {
    matches!(command, "parse" | "validate" | "check" | "inspect" | "convert" | "trace")
}

fn output_record(input: Option<&str>, destination: &str) -> String {
    let mut fields = Vec::new();
    if let Some(input) = input {
        fields.push(format!("input={}", record_value(input)));
    }
    fields.push(format!("destination={}", record_value(destination)));
    fields.join(",")
}

fn record_value(value: &str) -> String {
    if value.contains([',', '"', '\\']) {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        value.to_owned()
    }
}

fn run_config_identity(parsed: &ParsedCommandInvocationV1, uri: &str) -> FormatIdentity {
    FormatIdentity {
        content_type: option_string(parsed, "config_content_type")
            .or_else(|| run_config::infer_content_type_from_path(uri))
            .or_else(|| Some("application/json".to_owned())),
        schema: Some(
            option_string(parsed, "config_schema")
                .unwrap_or_else(|| run_config::RUN_CONFIG_SCHEMA_URI.to_owned()),
        ),
        base_uri: Some(uri.to_owned()),
        ..FormatIdentity::default()
    }
}

fn report_projection(value: &str) -> Result<NormalizedReportProjection, CommandInvocationErrorV1> {
    let normalized = if value == "md" { "markdown" } else { value };
    enum_value("report projection", normalized)
}

fn query_suffix(content_type: &str) -> &'static str {
    if content_type.contains("xpath") {
        "query.xpath"
    } else if content_type.contains("css") {
        "query.css"
    } else {
        "query.cem-ql"
    }
}

fn parse_params(values: &[String]) -> Result<BTreeMap<String, Value>, CommandInvocationErrorV1> {
    let mut params = BTreeMap::new();
    for value in values {
        let (name, field_value) = split_pair("params", value)?;
        if params
            .insert(name.clone(), Value::String(field_value))
            .is_some()
        {
            return Err(usage(
                "cem.command.param_duplicate",
                format!("transform --param `{name}` is declared more than once"),
            ));
        }
    }
    Ok(params)
}

fn parse_pairs(
    field: &str,
    values: &[String],
) -> Result<BTreeMap<String, String>, CommandInvocationErrorV1> {
    let mut pairs = BTreeMap::new();
    for value in values {
        let (key, field_value) = split_pair(field, value)?;
        if pairs.insert(key.clone(), field_value).is_some() {
            return Err(usage(
                "cem.command.key_duplicate",
                format!("{field} key `{key}` is declared more than once"),
            ));
        }
    }
    Ok(pairs)
}

fn split_pair(field: &str, value: &str) -> Result<(String, String), CommandInvocationErrorV1> {
    let Some((key, field_value)) = value.split_once('=') else {
        return Err(usage(
            "cem.command.key_value",
            format!("{field} value `{value}` must use NAME=VALUE"),
        ));
    };
    let key = key.trim();
    if key.is_empty() || field_value.is_empty() {
        return Err(usage(
            "cem.command.key_value",
            format!("{field} value `{value}` must contain a non-empty name and value"),
        ));
    }
    Ok((key.to_owned(), field_value.to_owned()))
}

fn enum_value<T>(field: &str, value: &str) -> Result<T, CommandInvocationErrorV1>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(Value::String(value.to_owned())).map_err(|_| {
        usage(
            "cem.command.invalid_value",
            format!("invalid {field} value `{value}`"),
        )
    })
}

fn add_read_requirement(
    requirements: &mut Vec<CommandInvocationResourceRequirementV1>,
    resources: &CommandUriMapV1<VirtualResourceV1>,
    uri: &str,
    purpose: ResolvePurpose,
    identity: Option<FormatIdentity>,
) {
    if !resources.contains_key(uri) {
        requirements.push(requirement(
            CommandInvocationResourceRequirementKindV1::Read,
            uri,
            purpose,
            identity,
        ));
    }
}

fn requirement(
    kind: CommandInvocationResourceRequirementKindV1,
    uri: &str,
    purpose: ResolvePurpose,
    identity: Option<FormatIdentity>,
) -> CommandInvocationResourceRequirementV1 {
    CommandInvocationResourceRequirementV1 {
        kind,
        uri: uri.to_owned(),
        purpose,
        identity,
    }
}

fn resolve_host_uri(cwd: &str, raw: &str) -> String {
    if has_uri_scheme(raw) && !is_windows_drive_path(raw) {
        return raw.to_owned();
    }
    let path = if Path::new(raw).is_absolute() || is_windows_drive_path(raw) {
        PathBuf::from(raw)
    } else {
        Path::new(cwd).join(raw)
    };
    path_to_file_uri(&normalize_host_path(path))
}

fn normalize_host_path(path: PathBuf) -> String {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized.to_string_lossy().replace('\\', "/")
}

fn path_to_file_uri(path: &str) -> String {
    let prefix = if path.starts_with('/') {
        "file://"
    } else {
        "file:///"
    };
    let mut encoded = String::with_capacity(prefix.len() + path.len());
    encoded.push_str(prefix);
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b':' | b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

fn option_value<'a>(
    parsed: &'a ParsedCommandInvocationV1,
    field: &str,
) -> Option<&'a ParsedCommandValueV1> {
    parsed.options.get(field)
}

fn option_string(parsed: &ParsedCommandInvocationV1, field: &str) -> Option<String> {
    match option_value(parsed, field) {
        Some(ParsedCommandValueV1::String(value)) if !value.is_empty() => Some(value.clone()),
        _ => None,
    }
}

fn required_option(
    parsed: &ParsedCommandInvocationV1,
    field: &str,
) -> Result<String, CommandInvocationErrorV1> {
    option_string(parsed, field).ok_or_else(|| {
        usage(
            "cem.command.required_option",
            format!("required command option `{field}` is missing"),
        )
    })
}

fn option_strings(parsed: &ParsedCommandInvocationV1, field: &str) -> Vec<String> {
    match option_value(parsed, field) {
        Some(ParsedCommandValueV1::Strings(values)) => values.clone(),
        Some(ParsedCommandValueV1::String(value)) if !value.is_empty() => vec![value.clone()],
        _ => Vec::new(),
    }
}

fn option_bool(parsed: &ParsedCommandInvocationV1, field: &str) -> bool {
    matches!(option_value(parsed, field), Some(ParsedCommandValueV1::Boolean(true)))
}

fn global_string(parsed: &ParsedCommandInvocationV1, field: &str) -> Option<String> {
    match parsed.global_options.get(field) {
        Some(ParsedCommandValueV1::String(value)) if !value.is_empty() => Some(value.clone()),
        _ => None,
    }
}

fn global_bool(parsed: &ParsedCommandInvocationV1, field: &str) -> bool {
    matches!(
        parsed.global_options.get(field),
        Some(ParsedCommandValueV1::Boolean(true))
    )
}

fn positional_string(parsed: &ParsedCommandInvocationV1, field: &str) -> Option<String> {
    match parsed.positionals.get(field) {
        Some(ParsedCommandValueV1::String(value)) if !value.is_empty() => Some(value.clone()),
        _ => None,
    }
}

fn positional_strings(parsed: &ParsedCommandInvocationV1, field: &str) -> Vec<String> {
    match parsed.positionals.get(field) {
        Some(ParsedCommandValueV1::Strings(values)) => values.clone(),
        Some(ParsedCommandValueV1::String(value)) if !value.is_empty() => vec![value.clone()],
        _ => Vec::new(),
    }
}

fn usage(code: &str, message: impl Into<String>) -> CommandInvocationErrorV1 {
    CommandInvocationErrorV1 {
        code: code.to_owned(),
        message: message.into(),
        exit_code: EXIT_USAGE,
    }
}

fn internal_error(code: &str, message: impl Into<String>) -> CommandInvocationErrorV1 {
    CommandInvocationErrorV1 {
        code: code.to_owned(),
        message: message.into(),
        exit_code: 7,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(command: &str) -> ParsedCommandInvocationV1 {
        ParsedCommandInvocationV1 {
            schema_version: COMMAND_SCHEMA_VERSION,
            common_version: crate::VERSION.to_owned(),
            command_path: vec![command.to_owned()],
            global_options: BTreeMap::from([
                ("quiet".to_owned(), ParsedCommandValueV1::Boolean(false)),
                ("verbose".to_owned(), ParsedCommandValueV1::Boolean(false)),
                ("no_color".to_owned(), ParsedCommandValueV1::Boolean(true)),
            ]),
            options: BTreeMap::new(),
            positionals: BTreeMap::new(),
            meta_action: None,
        }
    }

    fn environment() -> CommandInvocationEnvironmentV1 {
        CommandInvocationEnvironmentV1 {
            request_id: "request-1".to_owned(),
            project_id: "project-1".to_owned(),
            project_revision: 1,
            resource_revision: 1,
            cwd: "/workspace".to_owned(),
            resolver_policy_stamp: "node-file-https-v1".to_owned(),
            safety_policy_stamp: "portable-v1".to_owned(),
            budget_policy_stamp: "common-default-v1".to_owned(),
            stdout_is_terminal: false,
        }
    }

    #[test]
    fn parse_lowering_requests_then_owns_the_canonical_snapshot() {
        let mut command = parsed("parse");
        command.positionals.insert(
            "input".to_owned(),
            ParsedCommandValueV1::String("input.cem".to_owned()),
        );
        command.options.insert(
            "format".to_owned(),
            ParsedCommandValueV1::String("ast".to_owned()),
        );
        let first = build_command_invocation_v1(
            command.clone(),
            environment(),
            CommandUriMapV1::new(),
        );
        let CommandInvocationBuildResponseV1::NeedsResources { requirements } = first else {
            panic!("expected a resource read");
        };
        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].uri, "file:///workspace/input.cem");

        let resources = CommandUriMapV1::from(BTreeMap::from([(
            "file:///workspace/input.cem".to_owned(),
            VirtualResourceV1 {
                bytes: b"{root}".to_vec(),
                identity: None,
            },
        )]));
        let ready = build_command_invocation_v1(command, environment(), resources);
        let CommandInvocationBuildResponseV1::Ready { invocation } = ready else {
            panic!("expected a ready invocation");
        };
        assert_eq!(invocation.request.request_id, "request-1");
        assert_eq!(
            invocation.presentation.stdout_artifact_uri.as_deref(),
            Some(STDOUT_RESOURCE_URI)
        );
        assert!(matches!(
            invocation.request.operation,
            PortableOperationRequestV1::Parse { .. }
        ));
        assert_eq!(invocation.request.resource_versions.len(), 1);
    }

    #[test]
    fn graph_lowering_exposes_rust_owned_glob_requirements() {
        let mut command = parsed("transform");
        command.options.insert(
            "config".to_owned(),
            ParsedCommandValueV1::String("graph.cem".to_owned()),
        );
        let resources = CommandUriMapV1::from(BTreeMap::from([(
            "file:///workspace/graph.cem".to_owned(),
            VirtualResourceV1 {
                bytes: b"{run | {import @id=docs @src=\"docs/*.cem\"} {export @input=docs @out=\"out.json\"}}".to_vec(),
                identity: Some(FormatIdentity {
                    content_type: Some("application/cem+xml".to_owned()),
                    schema: Some(TRANSFORM_CONFIG_SCHEMA_URI.to_owned()),
                    base_uri: Some("file:///workspace/graph.cem".to_owned()),
                    ..FormatIdentity::default()
                }),
            },
        )]));
        let response = build_command_invocation_v1(command, environment(), resources);
        let CommandInvocationBuildResponseV1::NeedsResources { requirements } = response else {
            panic!("expected graph resources");
        };
        assert!(requirements.iter().any(|requirement| {
            requirement.kind == CommandInvocationResourceRequirementKindV1::Glob
                && requirement.uri == "file:///workspace/docs/*.cem"
        }));
    }

    #[test]
    fn report_routes_project_multiple_native_compatible_formats() {
        let report = crate::report::Report::deterministic(
            vec!["input.cem".to_owned()],
            Vec::new(),
            crate::report::ReportOptionsSnapshot {
                fail_level: FailLevel::Validate,
                schema: None,
                content_type: None,
                base_uri: None,
            },
        );
        let mut result = serde_json::from_value::<CommandServiceResultV1>(serde_json::json!({
            "protocolVersion": 1,
            "requestId": "request-1",
            "project": { "projectId": "project-1", "revision": 1 },
            "resourceVersions": {},
            "operation": "validate",
            "status": "succeeded",
            "exitCode": 0,
            "diagnostics": { "items": [], "originalCount": 0 },
            "artifacts": { "items": [], "originalCount": 0 },
            "sourceMaps": { "items": [], "originalCount": 0 },
            "identity": {
                "commonVersion": crate::VERSION,
                "runtime": "wasm-node",
                "targetIdentity": "wasm32-unknown-unknown:nodejs",
                "abiIdentity": "fixture",
                "schemaPackageVersions": {},
                "resolverPolicyStamp": "resolver",
                "safetyPolicyStamp": "safety",
                "budgetPolicyStamp": "budget"
            }
        }))
        .expect("result fixture");
        result.report = Some(crate::command_service::CommandPayloadV1::Inline { value: report });
        let plan = CommandPresentationPlanV1 {
            stdout_artifact_uri: None,
            report_routes: vec![
                CommandPresentationRouteV1 {
                    target: CommandPresentationTargetKindV1::File,
                    uri: Some("report.json".to_owned()),
                    projection: NormalizedReportProjection::Json,
                },
                CommandPresentationRouteV1 {
                    target: CommandPresentationTargetKindV1::File,
                    uri: Some("report.md".to_owned()),
                    projection: NormalizedReportProjection::Markdown,
                },
            ],
            diagnostics_to_stderr: false,
            graph_outputs_to_stdout: false,
            version_to_stdout: false,
            source_map_summary: false,
            quiet: false,
        };
        let projected = project_command_presentation_v1(&plan, &result).expect("projection");
        assert_eq!(projected.writes.len(), 2);
        assert!(projected.writes[0].bytes.starts_with(b"{"));
        assert!(projected.writes[1].bytes.starts_with(b"# cem-ml report"));
    }
}
