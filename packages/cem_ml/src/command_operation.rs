//! Owned native operation preparation for admitted command-service requests.
//!
//! This layer consumes only inline, digest-verified request snapshots. It maps
//! normalized run-plan data into common engine/query request types without
//! invoking a host resolver, publishing output, or selecting a process policy.

use std::collections::BTreeMap;
use std::fmt;

use crate::capability::{CapabilityManifest, CapabilityOperation};
use crate::command_service::{
    validate_command_service_request_v1, CommandServiceError, CommandServiceLimitsV1,
    CommandServiceRequestV1, CommandTransformSourceV1, CommandVersionCapabilitiesResultV1,
    PortableOperationRequestV1, VirtualResourceV1,
};
use crate::diagnostics::Diagnostic;
use crate::engine::{
    self, CheckRequest, ConvertRequest, EngineContext, EngineInput, FormatIdentity, InspectRequest,
    ParseRequest, TemplateInput, TraceRequest, TransformExecutionPolicy, TransformGraphRequest,
    TransformRequest, TransformRuntimePhase, TransformSchedulerScopeIds, TransformTemplateKind,
    ValidateRequest,
};
use crate::query::{QueryExportFormat, QueryRunRequest, QuerySource};
use crate::run_config::{
    infer_content_type_from_path, NormalizedBudgets, NormalizedOutput, NormalizedRootScope,
    NormalizedRunPlan, ScopeConfig,
};
use crate::scheduler::OverflowPolicy;
use crate::transform_config::{parse_transform_graph_config, TransformGraphParseRequest};
use crate::transform_graph_request::{
    lower_transform_graph_request, ManifestTransformGraphResourceProvider,
    TransformGraphRequestError,
};

#[derive(Debug, Clone)]
pub struct PreparedCommandQueryV1 {
    pub request: QueryRunRequest,
    pub output: QueryExportFormat,
}

#[derive(Debug, Clone)]
pub struct PreparedCommandOutputV1<T> {
    pub output_id: Option<String>,
    pub destination: Option<String>,
    pub request: T,
}

#[derive(Debug, Clone)]
pub enum PreparedCommandTransformV1 {
    Direct(Vec<PreparedCommandOutputV1<TransformRequest>>),
    Graph(Box<PreparedCommandTransformGraphV1>),
}

#[derive(Debug, Clone)]
pub struct PreparedCommandTransformGraphV1 {
    pub config_uri: String,
    pub request: TransformGraphRequest,
}

#[derive(Debug, Clone)]
pub enum PreparedPortableOperationV1 {
    Parse(ParseRequest),
    Validate(ValidateRequest),
    Check(CheckRequest),
    Inspect(InspectRequest),
    Convert(Vec<PreparedCommandOutputV1<ConvertRequest>>),
    Query(Box<PreparedCommandQueryV1>),
    Transform(PreparedCommandTransformV1),
    Trace(TraceRequest),
    VersionCapabilities(CommandVersionCapabilitiesResultV1),
}

impl PreparedPortableOperationV1 {
    pub const fn operation(&self) -> CapabilityOperation {
        match self {
            Self::Parse(_) => CapabilityOperation::Parse,
            Self::Validate(_) => CapabilityOperation::Validate,
            Self::Check(_) => CapabilityOperation::Check,
            Self::Inspect(_) => CapabilityOperation::Inspect,
            Self::Convert(_) => CapabilityOperation::Convert,
            Self::Query(_) => CapabilityOperation::Query,
            Self::Transform(_) => CapabilityOperation::Transform,
            Self::Trace(_) => CapabilityOperation::Trace,
            Self::VersionCapabilities(_) => CapabilityOperation::VersionCapabilities,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandOperationPreparationError {
    Request(CommandServiceError),
    InlineResourceRequired {
        uri: String,
    },
    ResourceIdentityMismatch {
        uri: String,
    },
    InputMissing {
        input_id: String,
    },
    QueryIdentityMissing {
        uri: String,
    },
    TransformIdentity {
        uri: String,
        message: String,
    },
    TransformSurface {
        message: String,
    },
    TransformConfig {
        uri: String,
        code: String,
        message: String,
    },
    TransformConfigDiagnostics {
        uri: String,
        diagnostics: Vec<Diagnostic>,
    },
}

impl CommandOperationPreparationError {
    pub fn code(&self) -> &str {
        match self {
            Self::Request(error) => error.code(),
            Self::InlineResourceRequired { .. } => "cem.command_service.inline_resource_required",
            Self::ResourceIdentityMismatch { .. } => {
                "cem.command_service.resource_identity_mismatch"
            }
            Self::InputMissing { .. } => "cem.command_service.input_missing",
            Self::QueryIdentityMissing { .. } => "cem.command_service.query_identity_missing",
            Self::TransformIdentity { .. } => "cem.command_service.transform_identity",
            Self::TransformSurface { .. } => "cem.command_service.transform_surface",
            Self::TransformConfig { code, .. } => code,
            Self::TransformConfigDiagnostics { .. } => {
                "cem.command_service.transform_config_diagnostics"
            }
        }
    }
}

impl fmt::Display for CommandOperationPreparationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(error) => error.fmt(formatter),
            Self::InlineResourceRequired { uri } => write!(
                formatter,
                "command-service preparation requires inline bytes for `{uri}`"
            ),
            Self::ResourceIdentityMismatch { uri } => write!(
                formatter,
                "command-service resource identity for `{uri}` conflicts with the normalized run plan"
            ),
            Self::InputMissing { input_id } => write!(
                formatter,
                "command-service normalized input `{input_id}` is missing"
            ),
            Self::QueryIdentityMissing { uri } => write!(
                formatter,
                "command-service query resource `{uri}` requires an explicit or inferable identity"
            ),
            Self::TransformIdentity { uri, message } => write!(
                formatter,
                "command-service transform resource `{uri}` has an unsupported identity: {message}"
            ),
            Self::TransformSurface { message } => formatter.write_str(message),
            Self::TransformConfig {
                uri,
                code,
                message,
            } => write!(formatter, "{code}: transform config `{uri}`: {message}"),
            Self::TransformConfigDiagnostics { uri, diagnostics } => write!(
                formatter,
                "transform config `{uri}` produced {} diagnostic(s)",
                diagnostics.len()
            ),
        }
    }
}

impl std::error::Error for CommandOperationPreparationError {}

impl From<CommandServiceError> for CommandOperationPreparationError {
    fn from(error: CommandServiceError) -> Self {
        Self::Request(error)
    }
}

/// Validate and prepare one request using only the request's inline resources.
/// Missing inline bytes are intentionally reported before any host callback is
/// considered; host resolution belongs to the service-construction layer.
pub fn prepare_command_operation_v1(
    request: &CommandServiceRequestV1,
    limits: CommandServiceLimitsV1,
    base_context: &EngineContext,
    capability: &CapabilityManifest,
) -> Result<PreparedPortableOperationV1, CommandOperationPreparationError> {
    validate_command_service_request_v1(request, limits)?;

    if matches!(
        request.operation,
        PortableOperationRequestV1::VersionCapabilities
    ) {
        return Ok(PreparedPortableOperationV1::VersionCapabilities(
            CommandVersionCapabilitiesResultV1 {
                version: crate::capability::product_version(),
                capability: capability.clone(),
            },
        ));
    }

    let plan = request
        .run_plan
        .plan()
        .expect("validated non-capability request has a normalized run plan");
    let context = context_from_plan(request, plan, base_context)?;
    let fail_level = plan.diagnostics_mode.fail_level;

    match &request.operation {
        PortableOperationRequestV1::Parse {
            input_id,
            projection,
            preserve_source_offsets,
        } => Ok(PreparedPortableOperationV1::Parse(ParseRequest {
            input: engine_input(request, plan, input_id)?,
            projection: *projection,
            fail_level,
            preserve_source_offsets: *preserve_source_offsets,
            presentation_scope: None,
            context,
        })),
        PortableOperationRequestV1::Validate {
            input_ids,
            projection,
        } => Ok(PreparedPortableOperationV1::Validate(ValidateRequest {
            inputs: engine_inputs(request, plan, input_ids)?,
            projection: *projection,
            fail_level,
            context,
        })),
        PortableOperationRequestV1::Check {
            input_ids,
            projection,
            zero_hard_violations,
        } => Ok(PreparedPortableOperationV1::Check(CheckRequest {
            inputs: engine_inputs(request, plan, input_ids)?,
            projection: *projection,
            fail_level,
            zero_hard_violations: *zero_hard_violations,
            context,
        })),
        PortableOperationRequestV1::Inspect { input_id, show } => {
            Ok(PreparedPortableOperationV1::Inspect(InspectRequest {
                input: engine_input(request, plan, input_id)?,
                show: *show,
                presentation_scope: None,
                context,
            }))
        }
        PortableOperationRequestV1::Convert {
            input_id,
            to_format,
            preserve_source_offsets,
        } => {
            let input = engine_input(request, plan, input_id)?;
            let input_index = input_index(plan, input_id)?;
            let outputs = operation_outputs(plan, input_id);
            let requests = if outputs.is_empty() {
                vec![PreparedCommandOutputV1 {
                    output_id: None,
                    destination: None,
                    request: ConvertRequest {
                        input,
                        to_format: *to_format,
                        preserve_source_offsets: *preserve_source_offsets,
                        context,
                        target: None,
                        target_scope: ScopeConfig::default(),
                        scheduler_scope_id: input_index as u32,
                    },
                }]
            } else {
                outputs
                    .into_iter()
                    .map(|output| PreparedCommandOutputV1 {
                        output_id: Some(output.output_id.clone()),
                        destination: output
                            .resolved_destination
                            .clone()
                            .or_else(|| output.declared_destination.clone()),
                        request: ConvertRequest {
                            input: input.clone(),
                            to_format: *to_format,
                            preserve_source_offsets: *preserve_source_offsets,
                            context: context.clone(),
                            target: identity_option(&output.identity),
                            target_scope: scope_config(&output.root_scope),
                            scheduler_scope_id: input_index as u32,
                        },
                    })
                    .collect()
            };
            Ok(PreparedPortableOperationV1::Convert(requests))
        }
        PortableOperationRequestV1::Query {
            data_input_id,
            query_uri,
            output,
        } => {
            let resource = inline_resource(request, query_uri)?;
            let identity = inferred_resource_identity(query_uri, resource).ok_or_else(|| {
                CommandOperationPreparationError::QueryIdentityMissing {
                    uri: query_uri.clone(),
                }
            })?;
            Ok(PreparedPortableOperationV1::Query(Box::new(
                PreparedCommandQueryV1 {
                    request: QueryRunRequest {
                        data: engine_input(request, plan, data_input_id)?,
                        query: QuerySource {
                            uri: query_uri.clone(),
                            bytes: resource.bytes.clone(),
                            identity,
                        },
                        context,
                        context_item: None,
                        bindings: BTreeMap::new(),
                        limits: None,
                    },
                    output: *output,
                },
            )))
        }
        PortableOperationRequestV1::Transform {
            source,
            params,
            template_entrypoint,
            preserve_source_offsets,
        } => match source {
            CommandTransformSourceV1::Direct {
                data_input_id,
                template_uri,
            } => Ok(PreparedPortableOperationV1::Transform(
                PreparedCommandTransformV1::Direct(direct_transform_requests(
                    request,
                    plan,
                    data_input_id,
                    template_uri,
                    params,
                    template_entrypoint,
                    *preserve_source_offsets,
                    context,
                )?),
            )),
            CommandTransformSourceV1::Graph { config_uri } => {
                let resource = inline_resource(request, config_uri)?;
                let mut identity =
                    inferred_resource_identity(config_uri, resource).unwrap_or_default();
                if identity.content_type.is_none() {
                    identity.content_type = Some("application/cem+xml".to_owned());
                }
                if identity.schema.is_none() {
                    identity.schema =
                        Some(crate::transform_config::TRANSFORM_CONFIG_SCHEMA_URI.to_owned());
                }
                identity.base_uri = Some(config_uri.clone());
                let parsed = parse_transform_graph_config(TransformGraphParseRequest {
                    bytes: resource.bytes.clone(),
                    identity,
                    base_uri: Some(config_uri.clone()),
                })
                .map_err(|error| {
                    CommandOperationPreparationError::TransformConfig {
                        uri: config_uri.clone(),
                        code: error.code.to_owned(),
                        message: error.message,
                    }
                })?;
                if !parsed.diagnostics.is_empty() {
                    return Err(
                        CommandOperationPreparationError::TransformConfigDiagnostics {
                            uri: config_uri.clone(),
                            diagnostics: parsed.diagnostics,
                        },
                    );
                }
                let provider = ManifestTransformGraphResourceProvider::new(
                    config_uri,
                    &request.resources,
                );
                let graph_request = lower_transform_graph_request(
                    &context,
                    &parsed.graph,
                    &provider,
                    config_uri,
                    *preserve_source_offsets,
                )
                .map_err(|error| graph_lowering_error(config_uri, error))?;
                Ok(PreparedPortableOperationV1::Transform(
                    PreparedCommandTransformV1::Graph(Box::new(PreparedCommandTransformGraphV1 {
                        config_uri: config_uri.clone(),
                        request: graph_request,
                    })),
                ))
            }
        },
        PortableOperationRequestV1::Trace {
            input_id,
            projection,
        } => Ok(PreparedPortableOperationV1::Trace(TraceRequest {
            input: engine_input(request, plan, input_id)?,
            projection: *projection,
            context,
        })),
        PortableOperationRequestV1::VersionCapabilities => unreachable!(),
    }
}

fn graph_lowering_error(
    config_uri: &str,
    error: TransformGraphRequestError,
) -> CommandOperationPreparationError {
    match error {
        TransformGraphRequestError::Diagnostic(diagnostic) => {
            let diagnostic = *diagnostic;
            CommandOperationPreparationError::TransformConfig {
                uri: diagnostic.uri.unwrap_or_else(|| config_uri.to_owned()),
                code: diagnostic.code,
                message: diagnostic.message,
            }
        }
        TransformGraphRequestError::Engine(error) => {
            CommandOperationPreparationError::TransformConfig {
                uri: config_uri.to_owned(),
                code: "cem.command_service.transform_graph_lowering".to_owned(),
                message: error.to_string(),
            }
        }
    }
}

fn context_from_plan(
    request: &CommandServiceRequestV1,
    plan: &NormalizedRunPlan,
    base: &EngineContext,
) -> Result<EngineContext, CommandOperationPreparationError> {
    let mut context = base.clone();
    context.scheduler = plan.scheduler.clone();
    for package in &plan.schema_packages {
        let uri = package
            .resolved_uri
            .as_deref()
            .unwrap_or(&package.declared_uri);
        let resource = inline_resource(request, uri)?;
        let identity = merge_identity(uri, &package.identity, resource.identity.as_ref())?;
        let mut root_scope = scope_config(&package.root_scope);
        apply_identity_to_scope(&mut root_scope, &identity);
        context.schema_package_manifests.push(EngineInput {
            uri: uri.to_owned(),
            bytes: resource.bytes.clone(),
            from_format: None,
            identity: identity_option(&identity),
            root_scope,
        });
    }
    Ok(context)
}

fn engine_inputs(
    request: &CommandServiceRequestV1,
    plan: &NormalizedRunPlan,
    input_ids: &[String],
) -> Result<Vec<EngineInput>, CommandOperationPreparationError> {
    input_ids
        .iter()
        .map(|input_id| engine_input(request, plan, input_id))
        .collect()
}

fn engine_input(
    request: &CommandServiceRequestV1,
    plan: &NormalizedRunPlan,
    input_id: &str,
) -> Result<EngineInput, CommandOperationPreparationError> {
    let input = plan
        .inputs
        .iter()
        .find(|input| input.input_id == input_id)
        .ok_or_else(|| CommandOperationPreparationError::InputMissing {
            input_id: input_id.to_owned(),
        })?;
    let uri = input.resolved_uri.as_deref().unwrap_or(&input.declared_uri);
    let resource = inline_resource(request, uri)?;
    let identity = merge_identity(uri, &input.identity, resource.identity.as_ref())?;
    let mut root_scope = scope_config(&input.root_scope);
    apply_identity_to_scope(&mut root_scope, &identity);
    Ok(EngineInput {
        uri: uri.to_owned(),
        bytes: resource.bytes.clone(),
        from_format: input.from_format_hint,
        identity: identity_option(&identity),
        root_scope,
    })
}

#[allow(clippy::too_many_arguments)]
fn direct_transform_requests(
    request: &CommandServiceRequestV1,
    plan: &NormalizedRunPlan,
    data_input_id: &str,
    template_uri: &str,
    params: &BTreeMap<String, serde_json::Value>,
    template_entrypoint: &engine::TransformTemplateEntrypoint,
    preserve_source_offsets: bool,
    context: EngineContext,
) -> Result<Vec<PreparedCommandOutputV1<TransformRequest>>, CommandOperationPreparationError> {
    let data = engine_input(request, plan, data_input_id)?;
    let resource = inline_resource(request, template_uri)?;
    let identity = inferred_resource_identity(template_uri, resource).unwrap_or_default();
    let mut root_scope = ScopeConfig::default();
    apply_identity_to_scope(&mut root_scope, &identity);
    let template = TemplateInput {
        uri: template_uri.to_owned(),
        bytes: resource.bytes.clone(),
        identity: identity_option(&identity),
        root_scope,
    };
    let template_kind = engine::classify_transform_template_identity_with_registry(
        &identity,
        &context.template_adapter_registry,
    )
    .map_err(
        |error| CommandOperationPreparationError::TransformIdentity {
            uri: template_uri.to_owned(),
            message: error.to_string(),
        },
    )?;
    validate_transform_surface(template_kind, template_entrypoint, params)?;
    let execution_policy = transform_execution_policy(template_kind, template_entrypoint, params);
    let outputs = operation_outputs(plan, data_input_id);
    let targets: Vec<_> = if outputs.is_empty() {
        vec![(None, None, None, ScopeConfig::default())]
    } else {
        outputs
            .into_iter()
            .map(|output| {
                (
                    Some(output.output_id.clone()),
                    output
                        .resolved_destination
                        .clone()
                        .or_else(|| output.declared_destination.clone()),
                    identity_option(&output.identity),
                    scope_config(&output.root_scope),
                )
            })
            .collect()
    };
    Ok(targets
        .into_iter()
        .map(
            |(output_id, destination, target, target_scope)| PreparedCommandOutputV1 {
                output_id,
                destination,
                request: TransformRequest {
                    data: data.clone(),
                    template: template.clone(),
                    template_kind,
                    template_entrypoint: template_entrypoint.clone(),
                    params: params.clone(),
                    preserve_source_offsets,
                    context: context.clone(),
                    target,
                    target_scope,
                    scheduler_scope_ids: TransformSchedulerScopeIds {
                        data_load: 0,
                        template_load: 1,
                        execution: 2,
                        output: 3,
                    },
                    execution_policy,
                },
            },
        )
        .collect())
}

fn validate_transform_surface(
    template_kind: TransformTemplateKind,
    entrypoint: &engine::TransformTemplateEntrypoint,
    params: &BTreeMap<String, serde_json::Value>,
) -> Result<(), CommandOperationPreparationError> {
    let message =
        if template_kind == TransformTemplateKind::CemQlExpression && !entrypoint.is_implicit() {
            Some("CEM-QL expression transforms do not accept a named entrypoint")
        } else if template_kind == TransformTemplateKind::XPath
            && (!entrypoint.is_implicit() || !params.is_empty())
        {
            Some("XPath transforms require the implicit entrypoint and no params")
        } else if !matches!(
            template_kind,
            TransformTemplateKind::CemNative
                | TransformTemplateKind::Xslt
                | TransformTemplateKind::CemQlExpression
        ) && (!entrypoint.is_implicit() || !params.is_empty())
        {
            Some("this transform adapter does not support entrypoints or params")
        } else {
            None
        };
    if let Some(message) = message {
        return Err(CommandOperationPreparationError::TransformSurface {
            message: message.to_owned(),
        });
    }
    Ok(())
}

fn transform_execution_policy(
    template_kind: TransformTemplateKind,
    entrypoint: &engine::TransformTemplateEntrypoint,
    params: &BTreeMap<String, serde_json::Value>,
) -> TransformExecutionPolicy {
    TransformExecutionPolicy {
        runtime_phase: match template_kind {
            TransformTemplateKind::Xslt => TransformRuntimePhase::XsltParity,
            TransformTemplateKind::CemQlExpression => TransformRuntimePhase::CemQlExpression,
            TransformTemplateKind::CemNative if !entrypoint.is_implicit() || !params.is_empty() => {
                TransformRuntimePhase::CemNativeModules
            }
            TransformTemplateKind::XPath => TransformRuntimePhase::XPath,
            TransformTemplateKind::CemNative => TransformRuntimePhase::CemQlFragment,
        },
        ..TransformExecutionPolicy::default()
    }
}

fn operation_outputs<'a>(plan: &'a NormalizedRunPlan, input_id: &str) -> Vec<&'a NormalizedOutput> {
    plan.outputs
        .iter()
        .filter(|output| output.input_id.as_deref() == Some(input_id))
        .collect()
}

fn input_index(
    plan: &NormalizedRunPlan,
    input_id: &str,
) -> Result<usize, CommandOperationPreparationError> {
    plan.inputs
        .iter()
        .position(|input| input.input_id == input_id)
        .ok_or_else(|| CommandOperationPreparationError::InputMissing {
            input_id: input_id.to_owned(),
        })
}

fn inline_resource<'a>(
    request: &'a CommandServiceRequestV1,
    uri: &str,
) -> Result<&'a VirtualResourceV1, CommandOperationPreparationError> {
    request.resources.get(uri).ok_or_else(|| {
        CommandOperationPreparationError::InlineResourceRequired {
            uri: uri.to_owned(),
        }
    })
}

fn inferred_resource_identity(uri: &str, resource: &VirtualResourceV1) -> Option<FormatIdentity> {
    resource.identity.clone().or_else(|| {
        infer_content_type_from_path(uri).map(|content_type| FormatIdentity {
            content_type: Some(content_type),
            base_uri: Some(uri.to_owned()),
            ..FormatIdentity::default()
        })
    })
}

fn merge_identity(
    uri: &str,
    plan: &FormatIdentity,
    resource: Option<&FormatIdentity>,
) -> Result<FormatIdentity, CommandOperationPreparationError> {
    if identity_option(plan).is_some()
        && resource.and_then(identity_option).is_some()
        && resource != Some(plan)
    {
        return Err(CommandOperationPreparationError::ResourceIdentityMismatch {
            uri: uri.to_owned(),
        });
    }
    Ok(if identity_option(plan).is_some() {
        plan.clone()
    } else {
        resource.cloned().unwrap_or_default()
    })
}

fn identity_option(identity: &FormatIdentity) -> Option<FormatIdentity> {
    (identity.content_type.is_some()
        || identity.schema.is_some()
        || identity.default_namespace.is_some()
        || !identity.namespaces.is_empty()
        || identity.base_uri.is_some())
    .then(|| identity.clone())
}

fn apply_identity_to_scope(scope: &mut ScopeConfig, identity: &FormatIdentity) {
    scope.default_content_type = identity.content_type.clone();
    scope.schema = identity.schema.clone();
    scope.default_namespace = identity.default_namespace.clone();
    scope.namespaces = identity.namespaces.clone();
    scope.base_uri = identity.base_uri.clone();
}

fn scope_config(scope: &NormalizedRootScope) -> ScopeConfig {
    let mut config = ScopeConfig {
        default_content_type: scope.identity.content_type.clone(),
        schema: scope.identity.schema.clone(),
        version_pins: scope.version_pins.clone(),
        default_namespace: scope.default_namespace.clone(),
        namespaces: scope.namespaces.clone(),
        module_map: scope.module_map.as_ref().map(|module_map| {
            module_map
                .resolved_uri
                .clone()
                .unwrap_or_else(|| module_map.declared_uri.clone())
        }),
        base_uri: scope.base_uri.clone(),
        policy: scope.policy.policy_name.clone(),
        budgets: scope_budgets(scope),
        ..ScopeConfig::default()
    };
    if let Some(pipeline) = &scope.output_pipeline {
        config.output_color_type = pipeline.output_color_type.clone();
        config.cemt_formatter = pipeline.cemt_formatter.clone();
        config.cemt_formatter_profile = pipeline.cemt_formatter_profile.clone();
        config.cemt_formatter_options = pipeline.cemt_formatter_options.clone();
        config.cemt_colorizer = pipeline.cemt_colorizer.clone();
        config.cemt_color_profile = pipeline.cemt_color_profile.clone();
    }
    config
}

fn scope_budgets(scope: &NormalizedRootScope) -> BTreeMap<String, String> {
    let mut budgets = BTreeMap::from([
        (
            "cpuWorkers".to_owned(),
            scope.policy.cpu_workers.to_string(),
        ),
        ("queueSize".to_owned(), scope.policy.queue_size.to_string()),
        ("ioStreams".to_owned(), scope.policy.io_streams.to_string()),
        (
            "memoryBytes".to_owned(),
            scope.policy.memory_bytes.to_string(),
        ),
        (
            "stackDepth".to_owned(),
            scope.policy.stack_depth.to_string(),
        ),
        (
            "overflow".to_owned(),
            match scope.policy.overflow {
                OverflowPolicy::Block => "block",
                OverflowPolicy::Reject => "reject",
                OverflowPolicy::SpillToParent => "spill-to-parent",
            }
            .to_owned(),
        ),
    ]);
    insert_budget(&mut budgets, "timeoutMs", scope.policy.timeout_ms);
    insert_budget(&mut budgets, "pluginMs", scope.policy.plugin_time_budget_ms);
    append_normalized_budgets(&mut budgets, &scope.budgets);
    budgets
}

fn append_normalized_budgets(
    budgets: &mut BTreeMap<String, String>,
    normalized: &NormalizedBudgets,
) {
    insert_budget(budgets, "parseMs", normalized.parse_ms);
    insert_budget(budgets, "validateMs", normalized.validate_ms);
    insert_budget(budgets, "checkMs", normalized.check_ms);
    insert_budget(budgets, "convertMs", normalized.convert_ms);
    insert_budget(budgets, "traceMs", normalized.trace_ms);
    insert_budget(budgets, "inspectMs", normalized.inspect_ms);
    insert_budget(budgets, "benchMs", normalized.bench_ms);
    insert_budget(budgets, "fixtureValidateMs", normalized.fixture_validate_ms);
    insert_budget(
        budgets,
        "fixtureRoundtripMs",
        normalized.fixture_roundtrip_ms,
    );
    insert_budget(budgets, "observeMs", normalized.observe_ms);
    insert_budget(budgets, "pluginMs", normalized.plugin_ms);
    insert_budget(budgets, "memoryBytes", normalized.memory_bytes);
    insert_budget(budgets, "stackDepth", normalized.stack_depth);
    insert_budget(budgets, "timeoutMs", normalized.timeout_ms);
    insert_budget(budgets, "xpathItems", normalized.xpath_items);
    budgets.extend(
        normalized
            .unknown
            .iter()
            .map(|entry| (entry.name.clone(), entry.value.clone())),
    );
}

fn insert_budget<T: ToString>(
    budgets: &mut BTreeMap<String, String>,
    name: &str,
    value: Option<T>,
) {
    if let Some(value) = value {
        budgets.insert(name.to_owned(), value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::capability::{capability_manifest, CapabilityRequest, RuntimeKind};
    use crate::command_service::{
        sha256_hex, CommandPolicyStampV1, CommandProjectRevisionV1, CommandResourceVersionV1,
        CommandRunPlanV1, CommandUriMapV1, COMMAND_SERVICE_PROTOCOL_VERSION,
    };
    use crate::engine::{
        InspectView, LayerFormat, ParseProjection, TraceProjection, TransformTemplateEntrypoint,
        ValidateProjection,
    };
    use crate::query::QueryExportFormat;
    use crate::run_config::{
        parse_normalized_run_plan, NormalizedRunPlanRequest, RunConfigDefaults,
    };
    use crate::schema::registry::{
        CEM_TRANSFORM_CONTENT_TYPE, CEM_TRANSFORM_SCHEMA_URI, XPATH_CONTENT_TYPE, XPATH_SCHEMA_URI,
    };

    const DATA_URI: &str = "studio://catalog/data.cem";
    const SECOND_URI: &str = "studio://catalog/second.cem";
    const QUERY_URI: &str = "studio://catalog/query.xpath";
    const TEMPLATE_URI: &str = "studio://catalog/template.cemt";
    const GRAPH_URI: &str = "studio://catalog/graph.cem";

    fn plan() -> NormalizedRunPlan {
        parse_normalized_run_plan(NormalizedRunPlanRequest {
            input_records: vec![
                format!(
                    "uri={DATA_URI},contentType=application/cem+xml,budgets=parseMs:12|cpuWorkers:2"
                ),
                format!("uri={SECOND_URI},contentType=application/cem+xml"),
            ],
            output_records: vec![
                format!(
                    "input={DATA_URI},dest=studio://catalog/out-a.json,contentType=application/json"
                ),
                format!("input={DATA_URI},dest=studio://catalog/out-b.html,contentType=text/html"),
            ],
            defaults: RunConfigDefaults::default(),
            ..NormalizedRunPlanRequest::default()
        })
        .expect("normalized fixture plan")
    }

    fn capability() -> CapabilityManifest {
        capability_manifest(CapabilityRequest {
            runtime: RuntimeKind::Native,
            target_identity: "x86_64-unknown-linux-gnu".to_owned(),
            abi_identity: "rust:1".to_owned(),
            debug_control_active: false,
        })
        .expect("fixture capability")
    }

    fn identity(content_type: &str, schema: &str) -> FormatIdentity {
        FormatIdentity {
            content_type: Some(content_type.to_owned()),
            schema: Some(schema.to_owned()),
            ..FormatIdentity::default()
        }
    }

    fn insert_resource(
        request: &mut CommandServiceRequestV1,
        uri: &str,
        bytes: &[u8],
        identity: Option<FormatIdentity>,
    ) {
        request.resource_versions.insert(
            uri.to_owned(),
            CommandResourceVersionV1 {
                revision: 1,
                sha256: sha256_hex(bytes),
            },
        );
        request.resources.insert(
            uri.to_owned(),
            VirtualResourceV1 {
                bytes: bytes.to_vec(),
                identity,
            },
        );
    }

    fn request(operation: PortableOperationRequestV1) -> CommandServiceRequestV1 {
        let version = matches!(operation, PortableOperationRequestV1::VersionCapabilities);
        let mut request = CommandServiceRequestV1 {
            protocol_version: COMMAND_SERVICE_PROTOCOL_VERSION,
            request_id: "request:prepare".to_owned(),
            project: CommandProjectRevisionV1 {
                project_id: "catalog".to_owned(),
                revision: 1,
            },
            resource_versions: CommandUriMapV1::new(),
            operation,
            run_plan: if version {
                CommandRunPlanV1::Null(())
            } else {
                plan().into()
            },
            resources: CommandUriMapV1::new(),
            policy_stamp: CommandPolicyStampV1 {
                resolver: "resolver:1".to_owned(),
                safety: "safety:1".to_owned(),
                budget: "budget:1".to_owned(),
            },
        };
        if version {
            return request;
        }
        insert_resource(&mut request, DATA_URI, b"<catalog/>", None);
        insert_resource(&mut request, SECOND_URI, b"<second/>", None);
        match &request.operation {
            PortableOperationRequestV1::Query { .. } => insert_resource(
                &mut request,
                QUERY_URI,
                b"//item",
                Some(identity(XPATH_CONTENT_TYPE, XPATH_SCHEMA_URI)),
            ),
            PortableOperationRequestV1::Transform {
                source: CommandTransformSourceV1::Direct { .. },
                ..
            } => insert_resource(
                &mut request,
                TEMPLATE_URI,
                b"{@doc cem-ml 1}{template @name=main}",
                Some(identity(
                    CEM_TRANSFORM_CONTENT_TYPE,
                    CEM_TRANSFORM_SCHEMA_URI,
                )),
            ),
            PortableOperationRequestV1::Transform {
                source: CommandTransformSourceV1::Graph { .. },
                ..
            } => insert_resource(
                &mut request,
                GRAPH_URI,
                br#"{@doc cem-ml 1}{run | {import @id=data @src="studio://catalog/data.cem"}}"#,
                Some(identity(
                    "application/cem+xml",
                    crate::transform_config::TRANSFORM_CONFIG_SCHEMA_URI,
                )),
            ),
            _ => {}
        }
        request
    }

    fn prepare(
        request: &CommandServiceRequestV1,
    ) -> Result<PreparedPortableOperationV1, CommandOperationPreparationError> {
        prepare_command_operation_v1(
            request,
            CommandServiceLimitsV1::default(),
            &EngineContext::default(),
            &capability(),
        )
    }

    #[test]
    fn prepares_all_nine_portable_discriminators_as_owned_common_inputs() {
        let operations = vec![
            PortableOperationRequestV1::Parse {
                input_id: "input:0".to_owned(),
                projection: ParseProjection::DomJson,
                preserve_source_offsets: true,
            },
            PortableOperationRequestV1::Validate {
                input_ids: vec!["input:1".to_owned(), "input:0".to_owned()],
                projection: ValidateProjection::Json,
            },
            PortableOperationRequestV1::Check {
                input_ids: vec!["input:0".to_owned()],
                projection: ValidateProjection::Cem,
                zero_hard_violations: true,
            },
            PortableOperationRequestV1::Inspect {
                input_id: "input:0".to_owned(),
                show: InspectView::Summary,
            },
            PortableOperationRequestV1::Convert {
                input_id: "input:0".to_owned(),
                to_format: LayerFormat::DomJson,
                preserve_source_offsets: true,
            },
            PortableOperationRequestV1::Query {
                data_input_id: "input:0".to_owned(),
                query_uri: QUERY_URI.to_owned(),
                output: QueryExportFormat::Json,
            },
            PortableOperationRequestV1::Transform {
                source: CommandTransformSourceV1::Direct {
                    data_input_id: "input:0".to_owned(),
                    template_uri: TEMPLATE_URI.to_owned(),
                },
                params: BTreeMap::from([("title".to_owned(), json!("Catalog"))]),
                template_entrypoint: TransformTemplateEntrypoint::named("main"),
                preserve_source_offsets: true,
            },
            PortableOperationRequestV1::Trace {
                input_id: "input:0".to_owned(),
                projection: TraceProjection::Json,
            },
            PortableOperationRequestV1::VersionCapabilities,
        ];
        let expected = [
            CapabilityOperation::Parse,
            CapabilityOperation::Validate,
            CapabilityOperation::Check,
            CapabilityOperation::Inspect,
            CapabilityOperation::Convert,
            CapabilityOperation::Query,
            CapabilityOperation::Transform,
            CapabilityOperation::Trace,
            CapabilityOperation::VersionCapabilities,
        ];

        for (operation, expected) in operations.into_iter().zip(expected) {
            let prepared = prepare(&request(operation))
                .unwrap_or_else(|error| panic!("{expected:?}: {error}"));
            assert_eq!(prepared.operation(), expected);
        }
    }

    #[test]
    fn preparation_preserves_scope_budgets_input_order_and_output_fanout() {
        let parse = prepare(&request(PortableOperationRequestV1::Parse {
            input_id: "input:0".to_owned(),
            projection: ParseProjection::Json,
            preserve_source_offsets: true,
        }))
        .expect("parse prepares");
        let PreparedPortableOperationV1::Parse(parse) = parse else {
            panic!("parse preparation variant")
        };
        assert_eq!(parse.input.uri, DATA_URI);
        assert_eq!(
            parse
                .input
                .root_scope
                .budgets
                .get("parseMs")
                .map(String::as_str),
            Some("12")
        );
        assert_eq!(
            parse
                .input
                .root_scope
                .budgets
                .get("cpuWorkers")
                .map(String::as_str),
            Some("2")
        );
        assert!(parse.preserve_source_offsets);

        let validate = prepare(&request(PortableOperationRequestV1::Validate {
            input_ids: vec!["input:1".to_owned(), "input:0".to_owned()],
            projection: ValidateProjection::Json,
        }))
        .expect("validate prepares");
        let PreparedPortableOperationV1::Validate(validate) = validate else {
            panic!("validate preparation variant")
        };
        assert_eq!(
            validate
                .inputs
                .iter()
                .map(|input| input.uri.as_str())
                .collect::<Vec<_>>(),
            [SECOND_URI, DATA_URI]
        );

        let convert = prepare(&request(PortableOperationRequestV1::Convert {
            input_id: "input:0".to_owned(),
            to_format: LayerFormat::DomJson,
            preserve_source_offsets: false,
        }))
        .expect("convert prepares");
        let PreparedPortableOperationV1::Convert(convert) = convert else {
            panic!("convert preparation variant")
        };
        assert_eq!(convert.len(), 2);
        assert_eq!(
            convert[0]
                .request
                .target
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some("application/json")
        );
        assert_eq!(
            convert[0].destination.as_deref(),
            Some("studio://catalog/out-a.json")
        );
        assert_eq!(
            convert[1]
                .request
                .target
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some("text/html")
        );
    }

    #[test]
    fn preparation_owns_query_direct_transform_graph_and_capability_metadata() {
        let query = prepare(&request(PortableOperationRequestV1::Query {
            data_input_id: "input:0".to_owned(),
            query_uri: QUERY_URI.to_owned(),
            output: QueryExportFormat::Cem,
        }))
        .expect("query prepares");
        let PreparedPortableOperationV1::Query(query) = query else {
            panic!("query preparation variant")
        };
        assert_eq!(query.output, QueryExportFormat::Cem);
        assert_eq!(query.request.query.uri, QUERY_URI);
        assert_eq!(
            query.request.query.identity.schema.as_deref(),
            Some(XPATH_SCHEMA_URI)
        );

        let direct = prepare(&request(PortableOperationRequestV1::Transform {
            source: CommandTransformSourceV1::Direct {
                data_input_id: "input:0".to_owned(),
                template_uri: TEMPLATE_URI.to_owned(),
            },
            params: BTreeMap::from([("title".to_owned(), json!("Catalog"))]),
            template_entrypoint: TransformTemplateEntrypoint::named("main"),
            preserve_source_offsets: true,
        }))
        .expect("direct transform prepares");
        let PreparedPortableOperationV1::Transform(PreparedCommandTransformV1::Direct(direct)) =
            direct
        else {
            panic!("direct transform preparation variant")
        };
        assert_eq!(direct.len(), 2);
        assert_eq!(
            direct[0].request.execution_policy.runtime_phase,
            TransformRuntimePhase::CemNativeModules
        );
        assert!(direct[0].request.preserve_source_offsets);
        assert_eq!(
            direct[1].destination.as_deref(),
            Some("studio://catalog/out-b.html")
        );

        let graph = prepare(&request(PortableOperationRequestV1::Transform {
            source: CommandTransformSourceV1::Graph {
                config_uri: GRAPH_URI.to_owned(),
            },
            params: BTreeMap::new(),
            template_entrypoint: TransformTemplateEntrypoint::implicit(),
            preserve_source_offsets: true,
        }))
        .expect("graph transform prepares");
        let PreparedPortableOperationV1::Transform(PreparedCommandTransformV1::Graph(graph)) =
            graph
        else {
            panic!("graph transform preparation variant")
        };
        assert_eq!(graph.request.imports.len(), 1);
        assert_eq!(graph.request.imports[0].input.uri, DATA_URI);
        assert_eq!(graph.request.imports[0].input.bytes, b"<catalog/>");
        assert!(graph.request.preserve_source_offsets);

        let version = prepare(&request(PortableOperationRequestV1::VersionCapabilities))
            .expect("version capabilities prepares");
        let PreparedPortableOperationV1::VersionCapabilities(version) = version else {
            panic!("version capability preparation variant")
        };
        assert_eq!(version.version.common_version, crate::VERSION);
        assert_eq!(version.capability.runtime, RuntimeKind::Native);
    }

    #[test]
    fn preparation_rejects_top_level_graph_invocation_metadata() {
        let overrides = [
            (
                BTreeMap::from([("locale".to_owned(), json!("en"))]),
                TransformTemplateEntrypoint::implicit(),
                "operation.params",
            ),
            (
                BTreeMap::new(),
                TransformTemplateEntrypoint::named("main"),
                "operation.templateEntrypoint",
            ),
        ];

        for (params, template_entrypoint, field) in overrides {
            let error = prepare(&request(PortableOperationRequestV1::Transform {
                source: CommandTransformSourceV1::Graph {
                    config_uri: GRAPH_URI.to_owned(),
                },
                params,
                template_entrypoint,
                preserve_source_offsets: true,
            }))
            .expect_err("top-level graph invocation metadata is rejected");
            assert_eq!(
                error.code(),
                "cem.command_service.transform_graph_stage_local"
            );
            assert!(error.to_string().contains(field));
        }
    }

    #[test]
    fn preparation_requires_inline_bytes_and_rejects_plan_resource_identity_drift() {
        let mut missing = request(PortableOperationRequestV1::Parse {
            input_id: "input:0".to_owned(),
            projection: ParseProjection::Json,
            preserve_source_offsets: false,
        });
        missing.resources.remove(DATA_URI);
        let error = prepare(&missing).expect_err("missing inline bytes fail preparation");
        assert_eq!(error.code(), "cem.command_service.inline_resource_required");
        assert!(error.to_string().contains(DATA_URI));

        let mut mismatch = request(PortableOperationRequestV1::Parse {
            input_id: "input:0".to_owned(),
            projection: ParseProjection::Json,
            preserve_source_offsets: false,
        });
        mismatch.resources.get_mut(DATA_URI).unwrap().identity = Some(FormatIdentity {
            content_type: Some("text/html".to_owned()),
            ..FormatIdentity::default()
        });
        let error = prepare(&mismatch).expect_err("identity drift fails preparation");
        assert_eq!(
            error.code(),
            "cem.command_service.resource_identity_mismatch"
        );
        assert!(error.to_string().contains(DATA_URI));
    }
}
