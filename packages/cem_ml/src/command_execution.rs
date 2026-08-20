//! Common execution and terminal settlement for prepared command-service work.
//!
//! Prepared requests are already admitted, hydrated, and lowered. This layer
//! invokes only common engine/query boundaries, constructs a bounded wire
//! result, transactionally publishes requested artifacts, and consumes the
//! invocation's single terminal publisher exactly once.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::capability::{CapabilityManifest, CapabilityOperation};
use crate::command_artifact::CommandServiceRetainedArtifactV1;
use crate::command_operation::{
    PreparedCommandOutputV1, PreparedCommandTransformV1, PreparedPortableOperationV1,
};
use crate::command_publication::{
    publish_command_artifacts_v1, CommandPublicationErrorV1, CommandPublicationItemV1,
    CommandPublicationV1, CommandResourceWriterV1, CommandRevisionLedgerReaderV1,
};
use crate::command_runtime::PreparedCommandServiceInvocationV1;
use crate::command_service::{
    validate_command_service_result_v1, CommandArtifactKindV1, CommandExecutionIdentityV1,
    CommandFanoutResultV1, CommandOutputResultV1, CommandPayloadV1, CommandQueryResultV1,
    CommandServiceLimitsV1, CommandServiceRequestV1, CommandServiceResultV1,
    CommandServiceStatusV1, CommandSourceMapOwnerV1, CommandSourceMapReferenceV1,
    CommandTransformResultV1, PortableOperationResultV1, COMMAND_SERVICE_PROTOCOL_VERSION,
};
use crate::diagnostics::{Diagnostic, Severity};
use crate::engine::{
    CemMlEngine, EngineError, FailLevel, FormatIdentity, PrimaryBytes, TransformGraphResponse,
};
use crate::operation_control::{
    ControlCause, ControlError, ControlFailure, ROOT_EXECUTION_SCOPE_ID,
};
use crate::operation_handle::{
    ArtifactDisposition, BoundedList, OperationHandle, OperationHandleError, OperationOutcome,
    RetainedHandleKind, RetainedHandleMetadata, TerminalClaim,
};
use crate::query::{run_query, QueryExportRequest, QueryResultExporterRegistry, QueryRunError};
use crate::report::{Report, ReportOptionsSnapshot};
use crate::report_projection::project_report_v1;
use crate::resolver::ResolvePurpose;
use crate::run_config::{NormalizedOutput, NormalizedRunPlan};
use crate::source_map::SourceMapStack;

const EXIT_OK: u8 = 0;
const EXIT_HARD_FAILURE: u8 = 1;
const EXIT_USAGE: u8 = 2;
const EXIT_SCHEMA: u8 = 3;
const EXIT_IO: u8 = 6;
const EXIT_INTERNAL: u8 = 7;
const EXIT_CANCELLED: u8 = 130;

struct ExecutionProduct {
    operation_result: PortableOperationResultV1,
    diagnostics: Vec<Diagnostic>,
    report: Option<Report>,
    publication_items: Vec<CommandPublicationItemV1>,
    source_maps: Vec<CommandSourceMapReferenceV1>,
    force_hard_failure: bool,
}

/// One settled command terminal plus the committed artifact bytes that a host
/// may install into its request-scoped artifact registry.
#[derive(Debug)]
pub struct CommandServiceExecutionV1 {
    pub terminal: TerminalClaim<CommandServiceResultV1>,
    pub artifacts: Vec<CommandServiceRetainedArtifactV1>,
}

enum ExecutionFailure {
    Command {
        status: CommandServiceStatusV1,
        exit_code: u8,
        diagnostics: Vec<Diagnostic>,
    },
    Control(ControlFailure, Vec<Diagnostic>),
    Cancelled(Option<SourceMapStack>),
}

/// Execute and settle one fully prepared command-service invocation.
pub async fn execute_prepared_command_v1<E: CemMlEngine + ?Sized>(
    engine: &E,
    invocation: Box<PreparedCommandServiceInvocationV1>,
    ledger_reader: &dyn CommandRevisionLedgerReaderV1,
    writer: &dyn CommandResourceWriterV1,
    query_exporters: &QueryResultExporterRegistry,
    capability: &CapabilityManifest,
    limits: CommandServiceLimitsV1,
) -> Result<TerminalClaim<CommandServiceResultV1>, OperationHandleError> {
    Ok(execute_prepared_command_with_artifacts_v1(
        engine,
        invocation,
        ledger_reader,
        writer,
        query_exporters,
        capability,
        limits,
    )
    .await?
    .terminal)
}

/// Execute and settle one prepared command while preserving owned copies of
/// successfully published artifact bytes for an outer host registry. Failure,
/// cancellation, stale publication, and terminal conflicts return no bytes.
pub async fn execute_prepared_command_with_artifacts_v1<E: CemMlEngine + ?Sized>(
    engine: &E,
    invocation: Box<PreparedCommandServiceInvocationV1>,
    ledger_reader: &dyn CommandRevisionLedgerReaderV1,
    writer: &dyn CommandResourceWriterV1,
    query_exporters: &QueryResultExporterRegistry,
    capability: &CapabilityManifest,
    limits: CommandServiceLimitsV1,
) -> Result<CommandServiceExecutionV1, OperationHandleError> {
    let PreparedCommandServiceInvocationV1 {
        request,
        prepared,
        operation,
        terminal_publisher,
    } = *invocation;
    let operation_kind = prepared.operation();

    let resolution = ensure_active(&operation)
        .and_then(|()| execute_operation(engine, &request, prepared, query_exporters, limits));

    let mut retained_artifacts = Vec::new();
    let (outcome, validate_result) = match resolution {
        Err(ExecutionFailure::Control(failure, diagnostics)) => (
            OperationOutcome::from_control_failure(
                failure,
                diagnostics,
                ArtifactDisposition::default(),
            ),
            None,
        ),
        Err(ExecutionFailure::Cancelled(source_map)) => {
            let failure = ControlFailure {
                operation_id: operation.operation_id(),
                affected_scope: ROOT_EXECUTION_SCOPE_ID,
                cause: ControlCause::HostCancellation { reason: None },
                source_map,
            };
            (
                OperationOutcome::from_control_failure(
                    failure,
                    vec![diagnostic(
                        "cem.operation.cancelled",
                        Severity::Error,
                        "operation cancelled by host",
                    )],
                    ArtifactDisposition::default(),
                ),
                None,
            )
        }
        Err(ExecutionFailure::Command {
            status,
            exit_code,
            diagnostics,
        }) => {
            let result = result_envelope(
                &request,
                operation_kind,
                status,
                Some(exit_code),
                None,
                diagnostics,
                None,
                Vec::new(),
                Vec::new(),
                capability,
                limits,
            );
            (success_outcome(result, Vec::new(), &operation), Some(()))
        }
        Ok(product) => {
            let semantic_failure = product.force_hard_failure
                || diagnostics_fail(
                    request
                        .run_plan
                        .plan()
                        .map(|plan| plan.diagnostics_mode.fail_level)
                        .unwrap_or(FailLevel::Validate),
                    &product.diagnostics,
                );
            let mut result = result_envelope(
                &request,
                operation_kind,
                if semantic_failure {
                    CommandServiceStatusV1::Failed
                } else {
                    CommandServiceStatusV1::Succeeded
                },
                Some(if semantic_failure {
                    EXIT_HARD_FAILURE
                } else {
                    EXIT_OK
                }),
                Some(product.operation_result),
                product.diagnostics,
                product.report,
                Vec::new(),
                product.source_maps,
                capability,
                limits,
            );

            if let Err(error) = validate_command_service_result_v1(&result, limits) {
                let failure = internal_failure(
                    &operation,
                    "cem.command_service.result_invalid",
                    error.to_string(),
                );
                (
                    OperationOutcome::from_control_failure(
                        failure.0,
                        failure.1,
                        ArtifactDisposition::default(),
                    ),
                    None,
                )
            } else if semantic_failure || product.publication_items.is_empty() {
                (success_outcome(result, Vec::new(), &operation), Some(()))
            } else {
                let mut sorted_items = product.publication_items;
                sorted_items.sort_by(|left, right| left.uri.cmp(&right.uri));
                match publish_command_artifacts_v1(
                    &request,
                    sorted_items.clone(),
                    ledger_reader,
                    writer,
                    &operation,
                    limits,
                )
                .await
                {
                    Ok(CommandPublicationV1::Published(artifacts)) => {
                        let retained = artifacts
                            .iter()
                            .zip(&sorted_items)
                            .map(|(artifact, item)| RetainedHandleMetadata {
                                operation_id: operation.operation_id(),
                                handle_id: artifact.handle_id,
                                kind: RetainedHandleKind::Artifact,
                                label: item.label.clone(),
                            })
                            .collect::<Vec<_>>();
                        retained_artifacts = artifacts
                            .iter()
                            .cloned()
                            .zip(sorted_items)
                            .map(|(handle, item)| CommandServiceRetainedArtifactV1 {
                                handle,
                                bytes: item.bytes,
                            })
                            .collect();
                        result.artifacts =
                            bounded(artifacts, limits.operation_host.max_artifact_references);
                        debug_assert!(validate_command_service_result_v1(&result, limits).is_ok());
                        (success_outcome_with_disposition(result, retained), Some(()))
                    }
                    Ok(CommandPublicationV1::Stale(stale)) => {
                        result.status = CommandServiceStatusV1::Stale;
                        result.exit_code = None;
                        result.result = None;
                        result.report = None;
                        result.source_maps = BoundedList::default();
                        result.stale = Some(stale);
                        (success_outcome(result, Vec::new(), &operation), Some(()))
                    }
                    Err(error) => match publication_failure(&operation, error) {
                        ExecutionFailure::Control(failure, diagnostics) => (
                            OperationOutcome::from_control_failure(
                                failure,
                                diagnostics,
                                ArtifactDisposition::default(),
                            ),
                            None,
                        ),
                        ExecutionFailure::Command {
                            status,
                            exit_code,
                            diagnostics,
                        } => {
                            let failure_result = result_envelope(
                                &request,
                                operation_kind,
                                status,
                                Some(exit_code),
                                None,
                                diagnostics,
                                None,
                                Vec::new(),
                                Vec::new(),
                                capability,
                                limits,
                            );
                            (
                                success_outcome(failure_result, Vec::new(), &operation),
                                Some(()),
                            )
                        }
                        ExecutionFailure::Cancelled(source_map) => {
                            let failure = ControlFailure {
                                operation_id: operation.operation_id(),
                                affected_scope: ROOT_EXECUTION_SCOPE_ID,
                                cause: ControlCause::HostCancellation { reason: None },
                                source_map,
                            };
                            (
                                OperationOutcome::from_control_failure(
                                    failure,
                                    vec![diagnostic(
                                        "cem.operation.cancelled",
                                        Severity::Error,
                                        "operation cancelled by host",
                                    )],
                                    ArtifactDisposition::default(),
                                ),
                                None,
                            )
                        }
                    },
                }
            }
        }
    };

    if validate_result.is_some() {
        if let OperationOutcome::Succeeded { result, .. } = &outcome {
            if let Err(error) = validate_command_service_result_v1(result, limits) {
                let (failure, diagnostics) = internal_failure(
                    &operation,
                    "cem.command_service.result_invalid",
                    error.to_string(),
                );
                let terminal =
                    terminal_publisher.settle(OperationOutcome::from_control_failure(
                        failure,
                        diagnostics,
                        ArtifactDisposition::default(),
                    ))?;
                return Ok(CommandServiceExecutionV1 {
                    terminal,
                    artifacts: Vec::new(),
                });
            }
        }
    }
    let terminal = terminal_publisher.settle(outcome)?;
    let retains_published_artifacts = matches!(
        terminal.outcome().as_ref(),
        OperationOutcome::Succeeded { result, .. }
            if result.status == CommandServiceStatusV1::Succeeded
                && result.artifacts.items.len() == retained_artifacts.len()
    );
    if !retains_published_artifacts {
        retained_artifacts.clear();
    }
    Ok(CommandServiceExecutionV1 {
        terminal,
        artifacts: retained_artifacts,
    })
}

/// Project the operation-handle terminal into the command-service wire result.
///
/// Successful command execution already owns a complete result envelope. A
/// control failure, cancellation, or fatal host outcome is retained by the
/// operation handle instead, so host adapters use this common projection rather
/// than recreating status and exit policy in JavaScript.
pub fn project_command_service_terminal_v1(
    request: &CommandServiceRequestV1,
    claim: &TerminalClaim<CommandServiceResultV1>,
    capability: &CapabilityManifest,
    limits: CommandServiceLimitsV1,
) -> CommandServiceResultV1 {
    match claim.outcome().as_ref() {
        OperationOutcome::Succeeded { result, .. } => result.clone(),
        OperationOutcome::Failed { diagnostics, .. } => result_envelope(
            request,
            request.operation.operation(),
            CommandServiceStatusV1::Failed,
            Some(EXIT_HARD_FAILURE),
            None,
            diagnostics.items.clone(),
            None,
            Vec::new(),
            Vec::new(),
            capability,
            limits,
        ),
        OperationOutcome::Cancelled { diagnostics, .. } => result_envelope(
            request,
            request.operation.operation(),
            CommandServiceStatusV1::Cancelled,
            Some(EXIT_CANCELLED),
            None,
            diagnostics.items.clone(),
            None,
            Vec::new(),
            Vec::new(),
            capability,
            limits,
        ),
        OperationOutcome::Fatal { diagnostics, .. } => result_envelope(
            request,
            request.operation.operation(),
            CommandServiceStatusV1::Fatal,
            Some(EXIT_INTERNAL),
            None,
            diagnostics.items.clone(),
            None,
            Vec::new(),
            Vec::new(),
            capability,
            limits,
        ),
    }
}

/// Construct the canonical stale terminal returned when admission observes a
/// newer project or resource snapshot before an operation handle exists.
pub fn stale_command_service_result_v1(
    request: &CommandServiceRequestV1,
    stale: crate::command_service::CommandStaleRevisionV1,
    capability: &CapabilityManifest,
    limits: CommandServiceLimitsV1,
) -> CommandServiceResultV1 {
    let mut result = result_envelope(
        request,
        request.operation.operation(),
        CommandServiceStatusV1::Stale,
        None,
        None,
        Vec::new(),
        None,
        Vec::new(),
        Vec::new(),
        capability,
        limits,
    );
    result.stale = Some(stale);
    result
}

/// Construct the canonical cancelled terminal when cooperative cancellation is
/// observed during an asynchronous host boundary before operation preparation
/// has produced an operation handle.
pub fn cancelled_command_service_result_v1(
    request: &CommandServiceRequestV1,
    reason: Option<&str>,
    capability: &CapabilityManifest,
    limits: CommandServiceLimitsV1,
) -> CommandServiceResultV1 {
    let message = reason
        .map(|reason| format!("operation cancelled by host: {reason}"))
        .unwrap_or_else(|| "operation cancelled by host".to_owned());
    result_envelope(
        request,
        request.operation.operation(),
        CommandServiceStatusV1::Cancelled,
        Some(EXIT_CANCELLED),
        None,
        vec![diagnostic(
            "cem.operation.cancelled",
            Severity::Error,
            message,
        )],
        None,
        Vec::new(),
        Vec::new(),
        capability,
        limits,
    )
}

fn execute_operation<E: CemMlEngine + ?Sized>(
    engine: &E,
    request: &CommandServiceRequestV1,
    prepared: PreparedPortableOperationV1,
    query_exporters: &QueryResultExporterRegistry,
    limits: CommandServiceLimitsV1,
) -> Result<ExecutionProduct, ExecutionFailure> {
    let plan = request.run_plan.plan();
    match prepared {
        PreparedPortableOperationV1::Parse(engine_request) => {
            let response = engine.parse(engine_request).map_err(engine_failure)?;
            let diagnostics = response.diagnostics.clone();
            let items = output_items(
                plan,
                response.primary_bytes.as_ref(),
                &response.primary,
                "parse-output",
            )?;
            Ok(product(
                PortableOperationResultV1::Parse(response),
                diagnostics,
                plan,
                items,
                Vec::new(),
            )?)
        }
        PreparedPortableOperationV1::Validate(engine_request) => {
            let response = engine.validate(engine_request).map_err(engine_failure)?;
            let diagnostics = response.report.diagnostics.clone();
            let items = report_output_items(plan, &response.report)?;
            Ok(product_with_report(
                PortableOperationResultV1::Validate(response.clone()),
                diagnostics,
                plan,
                items,
                response.report,
                Vec::new(),
                false,
            )?)
        }
        PreparedPortableOperationV1::Check(engine_request) => {
            let zero_hard_violations = engine_request.zero_hard_violations;
            let response = engine.check(engine_request).map_err(engine_failure)?;
            let diagnostics = response.report.diagnostics.clone();
            let items = report_output_items(plan, &response.report)?;
            let force_hard_failure = zero_hard_violations && response.hard_violation_count > 0;
            Ok(product_with_report(
                PortableOperationResultV1::Check(response.clone()),
                diagnostics,
                plan,
                items,
                response.report,
                Vec::new(),
                force_hard_failure,
            )?)
        }
        PreparedPortableOperationV1::Inspect(engine_request) => {
            let response = engine.inspect(engine_request).map_err(engine_failure)?;
            let items = output_items(
                plan,
                response.primary_bytes.as_ref(),
                &response.body,
                "inspect-output",
            )?;
            Ok(product(
                PortableOperationResultV1::Inspect(response),
                Vec::new(),
                plan,
                items,
                Vec::new(),
            )?)
        }
        PreparedPortableOperationV1::Convert(outputs) => {
            let mut results = Vec::with_capacity(outputs.len());
            let mut diagnostics = Vec::new();
            let mut items = Vec::new();
            for output in outputs {
                let PreparedCommandOutputV1 {
                    output_id,
                    destination,
                    request: engine_request,
                } = output;
                let target = engine_request.target.clone();
                let response = engine.convert(engine_request).map_err(engine_failure)?;
                diagnostics.extend(response.diagnostics.clone());
                if let Some(uri) = destination.as_deref() {
                    items.push(publication_item(
                        output_id.as_deref().unwrap_or("convert-output"),
                        uri,
                        CommandArtifactKindV1::Output,
                        ResolvePurpose::Output,
                        response.primary_bytes.as_ref(),
                        target.as_ref(),
                        &response.primary,
                    )?);
                }
                results.push(CommandOutputResultV1 {
                    output_id,
                    destination,
                    response,
                });
            }
            Ok(product(
                PortableOperationResultV1::Convert(CommandFanoutResultV1 {
                    outputs: bounded(results, limits.operation_host.max_artifact_references),
                }),
                diagnostics,
                plan,
                items,
                Vec::new(),
            )?)
        }
        PreparedPortableOperationV1::Query(query) => {
            let response = run_query(query.request).map_err(query_failure)?;
            let encoded = query_exporters
                .export(
                    query.output,
                    QueryExportRequest {
                        result: &response.result,
                        no_color: plan
                            .map(|plan| plan.diagnostics_mode.no_color)
                            .unwrap_or(false),
                    },
                )
                .map_err(|message| ExecutionFailure::Command {
                    status: CommandServiceStatusV1::Failed,
                    exit_code: EXIT_HARD_FAILURE,
                    diagnostics: vec![diagnostic(
                        "cem.query.exporter_unavailable",
                        Severity::Error,
                        message,
                    )],
                })?;
            let output = if encoded.content_type.contains("json") {
                serde_json::from_slice(&encoded.bytes).unwrap_or_else(|_| {
                    Value::String(String::from_utf8_lossy(&encoded.bytes).into_owned())
                })
            } else {
                Value::String(String::from_utf8_lossy(&encoded.bytes).into_owned())
            };
            let items = encoded_output_items(plan, &encoded.content_type, &encoded.bytes);
            let source_maps = vec![source_map_reference(
                "query-source-map",
                response.result.source_map.clone(),
            )];
            Ok(product(
                PortableOperationResultV1::Query(CommandQueryResultV1 {
                    language: response.language,
                    inputs: response.inputs,
                    output,
                }),
                response.diagnostics,
                plan,
                items,
                source_maps,
            )?)
        }
        PreparedPortableOperationV1::Transform(PreparedCommandTransformV1::Direct(outputs)) => {
            let mut results = Vec::with_capacity(outputs.len());
            let mut diagnostics = Vec::new();
            let mut items = Vec::new();
            let mut source_maps = Vec::new();
            for (index, output) in outputs.into_iter().enumerate() {
                let PreparedCommandOutputV1 {
                    output_id,
                    destination,
                    request: engine_request,
                } = output;
                let target = engine_request.target.clone();
                let response = engine.transform(engine_request).map_err(engine_failure)?;
                diagnostics.extend(response.diagnostics.clone());
                if let Some(source_map) = response.source_map.clone() {
                    source_maps.push(source_map_reference(
                        &format!("transform-source-map-{index}"),
                        source_map,
                    ));
                }
                if let Some(uri) = destination.as_deref() {
                    items.push(publication_item(
                        output_id.as_deref().unwrap_or("transform-output"),
                        uri,
                        CommandArtifactKindV1::Output,
                        ResolvePurpose::Output,
                        None,
                        target.as_ref(),
                        &response.primary,
                    )?);
                }
                results.push(CommandOutputResultV1 {
                    output_id,
                    destination,
                    response,
                });
            }
            Ok(product(
                PortableOperationResultV1::Transform(CommandTransformResultV1::Direct(
                    CommandFanoutResultV1 {
                        outputs: bounded(results, limits.operation_host.max_artifact_references),
                    },
                )),
                diagnostics,
                plan,
                items,
                source_maps,
            )?)
        }
        PreparedPortableOperationV1::Transform(PreparedCommandTransformV1::Graph(graph)) => {
            let response = engine
                .transform_graph(graph.request)
                .map_err(engine_failure)?;
            graph_product(response, plan)
        }
        PreparedPortableOperationV1::Trace(engine_request) => {
            let response = engine.trace(engine_request).map_err(engine_failure)?;
            let items = output_items(plan, None, &response.body, "trace-output")?;
            Ok(product(
                PortableOperationResultV1::Trace(response),
                Vec::new(),
                plan,
                items,
                Vec::new(),
            )?)
        }
        PreparedPortableOperationV1::VersionCapabilities(response) => Ok(ExecutionProduct {
            operation_result: PortableOperationResultV1::VersionCapabilities(response),
            diagnostics: Vec::new(),
            report: None,
            publication_items: Vec::new(),
            source_maps: Vec::new(),
            force_hard_failure: false,
        }),
    }
}

fn graph_product(
    response: TransformGraphResponse,
    plan: Option<&NormalizedRunPlan>,
) -> Result<ExecutionProduct, ExecutionFailure> {
    let mut items = Vec::new();
    let mut source_maps = Vec::new();
    for (index, artifact) in response.artifacts.iter().enumerate() {
        if let Some(source_map) = artifact.source_map.clone() {
            source_maps.push(source_map_reference(
                &format!("graph-source-map-{index}"),
                source_map,
            ));
        }
        if let Some(uri) = artifact.destination.as_deref() {
            items.push(publication_item(
                &artifact.export_id,
                uri,
                CommandArtifactKindV1::Output,
                ResolvePurpose::Output,
                None,
                artifact.identity.as_ref(),
                &artifact.primary,
            )?);
        }
    }
    let diagnostics = response.diagnostics.clone();
    product(
        PortableOperationResultV1::Transform(CommandTransformResultV1::Graph(response)),
        diagnostics,
        plan,
        items,
        source_maps,
    )
}

fn product(
    operation_result: PortableOperationResultV1,
    diagnostics: Vec<Diagnostic>,
    plan: Option<&NormalizedRunPlan>,
    mut publication_items: Vec<CommandPublicationItemV1>,
    source_maps: Vec<CommandSourceMapReferenceV1>,
) -> Result<ExecutionProduct, ExecutionFailure> {
    let report = plan.map(|plan| report_for(plan, diagnostics.clone()));
    if let (Some(plan), Some(report)) = (plan, report.as_ref()) {
        publication_items.extend(report_items(plan, report)?);
    }
    Ok(ExecutionProduct {
        operation_result,
        diagnostics,
        report,
        publication_items,
        source_maps,
        force_hard_failure: false,
    })
}

#[allow(clippy::too_many_arguments)]
fn product_with_report(
    operation_result: PortableOperationResultV1,
    diagnostics: Vec<Diagnostic>,
    plan: Option<&NormalizedRunPlan>,
    mut publication_items: Vec<CommandPublicationItemV1>,
    report: Report,
    source_maps: Vec<CommandSourceMapReferenceV1>,
    force_hard_failure: bool,
) -> Result<ExecutionProduct, ExecutionFailure> {
    if let Some(plan) = plan {
        publication_items.extend(report_items(plan, &report)?);
    }
    Ok(ExecutionProduct {
        operation_result,
        diagnostics,
        report: Some(report),
        publication_items,
        source_maps,
        force_hard_failure,
    })
}

fn output_items(
    plan: Option<&NormalizedRunPlan>,
    primary_bytes: Option<&PrimaryBytes>,
    value: &Value,
    fallback_label: &str,
) -> Result<Vec<CommandPublicationItemV1>, ExecutionFailure> {
    let Some(plan) = plan else {
        return Ok(Vec::new());
    };
    plan.outputs
        .iter()
        .filter_map(|output| output_destination(output).map(|uri| (output, uri)))
        .map(|(output, uri)| {
            publication_item(
                if output.output_id.is_empty() {
                    fallback_label
                } else {
                    &output.output_id
                },
                uri,
                CommandArtifactKindV1::Output,
                ResolvePurpose::Output,
                primary_bytes,
                Some(&output.identity),
                value,
            )
        })
        .collect()
}

fn encoded_output_items(
    plan: Option<&NormalizedRunPlan>,
    content_type: &str,
    bytes: &[u8],
) -> Vec<CommandPublicationItemV1> {
    plan.into_iter()
        .flat_map(|plan| &plan.outputs)
        .filter_map(|output| output_destination(output).map(|uri| (output, uri)))
        .map(|(output, uri)| CommandPublicationItemV1 {
            label: output.output_id.clone(),
            uri: uri.to_owned(),
            kind: CommandArtifactKindV1::Output,
            purpose: ResolvePurpose::Output,
            content_type: content_type.to_owned(),
            bytes: bytes.to_vec(),
            source_map_id: None,
        })
        .collect()
}

fn report_output_items(
    plan: Option<&NormalizedRunPlan>,
    report: &Report,
) -> Result<Vec<CommandPublicationItemV1>, ExecutionFailure> {
    let Some(plan) = plan else {
        return Ok(Vec::new());
    };
    let projected = project_report_v1(report, plan.diagnostics_mode.report_projection)
        .map_err(serialization_failure)?;
    Ok(plan
        .outputs
        .iter()
        .filter_map(|output| output_destination(output).map(|uri| (output, uri)))
        .map(|(output, uri)| CommandPublicationItemV1 {
            label: output.output_id.clone(),
            uri: uri.to_owned(),
            kind: CommandArtifactKindV1::Output,
            purpose: ResolvePurpose::Output,
            content_type: projected.content_type.to_owned(),
            bytes: projected.bytes.clone(),
            source_map_id: None,
        })
        .collect())
}

fn report_items(
    plan: &NormalizedRunPlan,
    report: &Report,
) -> Result<Vec<CommandPublicationItemV1>, ExecutionFailure> {
    let projected = project_report_v1(report, plan.diagnostics_mode.report_projection)
        .map_err(serialization_failure)?;
    Ok(plan
        .diagnostics_mode
        .report_destinations
        .iter()
        .enumerate()
        .map(|(index, uri)| CommandPublicationItemV1 {
            label: format!("report-{index}"),
            uri: uri.clone(),
            kind: CommandArtifactKindV1::Report,
            purpose: ResolvePurpose::Report,
            content_type: projected.content_type.to_owned(),
            bytes: projected.bytes.clone(),
            source_map_id: None,
        })
        .collect())
}

#[allow(clippy::too_many_arguments)]
fn publication_item(
    label: &str,
    uri: &str,
    kind: CommandArtifactKindV1,
    purpose: ResolvePurpose,
    primary_bytes: Option<&PrimaryBytes>,
    identity: Option<&FormatIdentity>,
    value: &Value,
) -> Result<CommandPublicationItemV1, ExecutionFailure> {
    let content_type = primary_bytes
        .map(|primary| primary.content_type.clone())
        .or_else(|| identity.and_then(|identity| identity.content_type.clone()))
        .unwrap_or_else(|| "application/json".to_owned());
    let bytes = if let Some(primary) = primary_bytes {
        primary.bytes.clone()
    } else if !content_type.contains("json") {
        match value.as_str() {
            Some(value) => value.as_bytes().to_vec(),
            None => serde_json::to_vec(value).map_err(serialization_failure)?,
        }
    } else {
        serde_json::to_vec(value).map_err(serialization_failure)?
    };
    Ok(CommandPublicationItemV1 {
        label: label.to_owned(),
        uri: uri.to_owned(),
        kind,
        purpose,
        content_type,
        bytes,
        source_map_id: None,
    })
}

fn output_destination(output: &NormalizedOutput) -> Option<&str> {
    output
        .resolved_destination
        .as_deref()
        .or(output.declared_destination.as_deref())
}

fn report_for(plan: &NormalizedRunPlan, diagnostics: Vec<Diagnostic>) -> Report {
    let identity = plan.inputs.first().map(|input| &input.identity);
    Report::deterministic(
        plan.inputs
            .iter()
            .map(|input| {
                input
                    .resolved_uri
                    .clone()
                    .unwrap_or_else(|| input.declared_uri.clone())
            })
            .collect(),
        diagnostics,
        ReportOptionsSnapshot {
            fail_level: plan.diagnostics_mode.fail_level,
            schema: identity.and_then(|identity| identity.schema.clone()),
            content_type: identity.and_then(|identity| identity.content_type.clone()),
            base_uri: identity.and_then(|identity| identity.base_uri.clone()),
        },
    )
}

fn source_map_reference(id: &str, source_map: SourceMapStack) -> CommandSourceMapReferenceV1 {
    CommandSourceMapReferenceV1 {
        source_map_id: id.to_owned(),
        owner: CommandSourceMapOwnerV1::Result,
        source_map: CommandPayloadV1::Inline { value: source_map },
    }
}

#[allow(clippy::too_many_arguments)]
fn result_envelope(
    request: &CommandServiceRequestV1,
    operation: CapabilityOperation,
    status: CommandServiceStatusV1,
    exit_code: Option<u8>,
    result: Option<PortableOperationResultV1>,
    diagnostics: Vec<Diagnostic>,
    report: Option<Report>,
    artifacts: Vec<crate::command_service::CommandArtifactHandleV1>,
    source_maps: Vec<CommandSourceMapReferenceV1>,
    capability: &CapabilityManifest,
    limits: CommandServiceLimitsV1,
) -> CommandServiceResultV1 {
    CommandServiceResultV1 {
        protocol_version: COMMAND_SERVICE_PROTOCOL_VERSION,
        request_id: request.request_id.clone(),
        project: request.project.clone(),
        resource_versions: request.resource_versions.clone(),
        operation,
        status,
        exit_code,
        result: result.map(|value| CommandPayloadV1::Inline { value }),
        diagnostics: bounded(diagnostics, limits.operation_host.max_terminal_diagnostics),
        report: report.map(|value| CommandPayloadV1::Inline { value }),
        artifacts: bounded(artifacts, limits.operation_host.max_artifact_references),
        source_maps: bounded(source_maps, limits.operation_host.max_artifact_references),
        identity: CommandExecutionIdentityV1 {
            common_version: capability.common_version.clone(),
            runtime: capability.runtime,
            target_identity: capability.target_identity.clone(),
            abi_identity: capability.abi_identity.clone(),
            schema_package_versions: schema_package_versions(request.run_plan.plan()),
            resolver_policy_stamp: request.policy_stamp.resolver.clone(),
            safety_policy_stamp: request.policy_stamp.safety.clone(),
            budget_policy_stamp: request.policy_stamp.budget.clone(),
        },
        stale: None,
    }
}

fn schema_package_versions(plan: Option<&NormalizedRunPlan>) -> BTreeMap<String, String> {
    plan.into_iter()
        .flat_map(|plan| &plan.schema_packages)
        .filter_map(|package| {
            package
                .root_scope
                .version_pins
                .get(&package.schema_package_id)
                .or_else(|| package.root_scope.version_pins.values().next())
                .map(|version| (package.schema_package_id.clone(), version.clone()))
        })
        .collect()
}

fn diagnostics_fail(level: FailLevel, diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|diagnostic| match level {
        FailLevel::Strict => true,
        FailLevel::Validate => diagnostic.severity.is_hard_violation(),
        FailLevel::Parse => diagnostic.severity == Severity::Fatal,
    })
}

fn ensure_active<R: Send + Sync + 'static>(
    operation: &OperationHandle<R>,
) -> Result<(), ExecutionFailure> {
    operation
        .ensure_active()
        .map_err(|error| operation_failure(operation, error))
}

#[allow(unreachable_patterns)]
fn engine_failure(error: EngineError) -> ExecutionFailure {
    match error {
        EngineError::Control(failure) => ExecutionFailure::Control(
            failure.clone(),
            vec![diagnostic(
                failure.code(),
                Severity::Fatal,
                failure.to_string(),
            )],
        ),
        EngineError::Cancelled { source_map } => ExecutionFailure::Cancelled(source_map),
        EngineError::Io { .. } => ExecutionFailure::Command {
            status: CommandServiceStatusV1::Failed,
            exit_code: EXIT_IO,
            diagnostics: vec![diagnostic(
                "cem.engine.io",
                Severity::Error,
                error.to_string(),
            )],
        },
        EngineError::SchemaResolution(_) => ExecutionFailure::Command {
            status: CommandServiceStatusV1::Failed,
            exit_code: EXIT_SCHEMA,
            diagnostics: vec![diagnostic(
                "cem.engine.schema_resolution",
                Severity::Error,
                error.to_string(),
            )],
        },
        EngineError::Internal(_) | EngineError::NotImplemented => ExecutionFailure::Command {
            status: CommandServiceStatusV1::Fatal,
            exit_code: EXIT_INTERNAL,
            diagnostics: vec![diagnostic(
                "cem.engine.internal",
                Severity::Fatal,
                error.to_string(),
            )],
        },
        _ => ExecutionFailure::Command {
            status: CommandServiceStatusV1::Fatal,
            exit_code: EXIT_INTERNAL,
            diagnostics: vec![diagnostic(
                "cem.engine.internal",
                Severity::Fatal,
                error.to_string(),
            )],
        },
    }
}

fn query_failure(error: QueryRunError) -> ExecutionFailure {
    match error {
        QueryRunError::Contract(error) => ExecutionFailure::Command {
            status: CommandServiceStatusV1::Failed,
            exit_code: EXIT_USAGE,
            diagnostics: vec![diagnostic(
                "cem.query.contract",
                Severity::Error,
                error.to_string(),
            )],
        },
        QueryRunError::Execution(failure) => ExecutionFailure::Command {
            status: CommandServiceStatusV1::Failed,
            exit_code: EXIT_HARD_FAILURE,
            diagnostics: failure.diagnostics,
        },
    }
}

fn publication_failure<R: Send + Sync + 'static>(
    operation: &OperationHandle<R>,
    error: CommandPublicationErrorV1,
) -> ExecutionFailure {
    if let CommandPublicationErrorV1::Operation(operation_error) = error {
        return operation_failure(operation, operation_error);
    }
    ExecutionFailure::Command {
        status: CommandServiceStatusV1::Failed,
        exit_code: EXIT_IO,
        diagnostics: vec![diagnostic(error.code(), Severity::Error, error.to_string())],
    }
}

fn operation_failure<R: Send + Sync + 'static>(
    operation: &OperationHandle<R>,
    error: OperationHandleError,
) -> ExecutionFailure {
    match error {
        OperationHandleError::Control(ControlError::Triggered(failure))
        | OperationHandleError::TerminalControlConflict(failure) => ExecutionFailure::Control(
            failure.clone(),
            vec![diagnostic(
                failure.code(),
                Severity::Error,
                failure.to_string(),
            )],
        ),
        error => {
            let (failure, diagnostics) = internal_failure(
                operation,
                "cem.command_service.operation_failure",
                error.to_string(),
            );
            ExecutionFailure::Control(failure, diagnostics)
        }
    }
}

fn internal_failure<R: Send + Sync + 'static>(
    operation: &OperationHandle<R>,
    code: &str,
    message: String,
) -> (ControlFailure, Vec<Diagnostic>) {
    (
        ControlFailure {
            operation_id: operation.operation_id(),
            affected_scope: ROOT_EXECUTION_SCOPE_ID,
            cause: ControlCause::InternalFailure {
                diagnostic_code: code.to_owned(),
            },
            source_map: None,
        },
        vec![diagnostic(code, Severity::Fatal, message)],
    )
}

fn serialization_failure(error: impl std::fmt::Display) -> ExecutionFailure {
    ExecutionFailure::Command {
        status: CommandServiceStatusV1::Fatal,
        exit_code: EXIT_INTERNAL,
        diagnostics: vec![diagnostic(
            "cem.command_service.serialization",
            Severity::Fatal,
            error.to_string(),
        )],
    }
}

fn diagnostic(code: &str, severity: Severity, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        code: code.to_owned(),
        severity,
        message: message.into(),
        ..Diagnostic::default()
    }
}

fn bounded<T>(mut items: Vec<T>, maximum: u32) -> BoundedList<T> {
    let original_count = items.len().try_into().unwrap_or(u32::MAX);
    items.truncate(maximum as usize);
    BoundedList {
        items,
        original_count,
    }
}

fn success_outcome(
    result: CommandServiceResultV1,
    retained: Vec<RetainedHandleMetadata>,
    _operation: &OperationHandle<CommandServiceResultV1>,
) -> OperationOutcome<CommandServiceResultV1> {
    success_outcome_with_disposition(result, retained)
}

fn success_outcome_with_disposition(
    result: CommandServiceResultV1,
    retained: Vec<RetainedHandleMetadata>,
) -> OperationOutcome<CommandServiceResultV1> {
    OperationOutcome::succeeded(
        result,
        Vec::new(),
        ArtifactDisposition::new(retained, Vec::new()),
    )
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::io;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Wake, Waker};

    use serde_json::json;

    use super::*;
    use crate::capability::{capability_manifest, product_version, CapabilityRequest, RuntimeKind};
    use crate::command_host::CommandHostFuture;
    use crate::command_operation::{PreparedCommandQueryV1, PreparedCommandTransformGraphV1};
    use crate::command_publication::{
        CommandPreparedResourceWriteV1, CommandPublicationHostFailureV1, CommandResolvedWriteV1,
        CommandResourceWriteRequestV1, CommandRevisionLedgerRequestV1,
    };
    use crate::command_service::{
        sha256_hex, CommandChangedResourceV1, CommandPolicyStampV1, CommandProjectRevisionV1,
        CommandResourceVersionV1, CommandRevisionLedgerV1, CommandRunPlanV1,
        CommandStaleRevisionV1, CommandUriMapV1, CommandVersionCapabilitiesResultV1,
        PortableOperationRequestV1, VirtualResourceV1,
    };
    use crate::engine::{
        BenchRequest, BenchResponse, CheckRequest, CheckResponse, ConvertRequest, ConvertResponse,
        EngineContext, EngineInput, EngineResult, FixtureRoundtripRequest,
        FixtureRoundtripResponse, FixtureValidateRequest, FixtureValidateResponse, InspectRequest,
        InspectResponse, InspectView, LayerFormat, ParseProjection, ParseRequest, ParseResponse,
        TemplateInput, TraceProjection, TraceRequest, TraceResponse, TransformExecutionPolicy,
        TransformGraphArtifact, TransformGraphRequest, TransformRequest, TransformResponse,
        TransformSchedulerScopeIds, TransformTemplateEntrypoint, TransformTemplateKind,
        ValidateProjection, ValidateRequest, ValidateResponse,
    };
    use crate::operation_control::OperationControl;
    use crate::operation_handle::{
        EventSubscriptionOptions, EventSubscriptionPoll, OperationEventKind,
    };
    use crate::query::{QueryExportFormat, QueryRunRequest, QuerySource};
    use crate::report::SchedulerTraceReport;
    use crate::run_config::{parse_normalized_run_plan, NormalizedRunPlanRequest, ScopeConfig};
    use crate::scheduler::AbortSignal;
    use crate::schema::registry::{
        CSS_SELECTOR_CONTENT_TYPE, CSS_SELECTOR_SCHEMA_URI, HTML_CONTENT_TYPE, HTML_SCHEMA_URI,
    };
    use crate::validation::css_selector::register_css_selector_query_exporters;

    const DATA_URI: &str = "studio://execution/data.html";
    const QUERY_URI: &str = "studio://execution/query.css";
    const OUTPUT_URI: &str = "studio://execution/output.json";

    #[derive(Default)]
    struct EngineState {
        calls: Vec<&'static str>,
        fail_parse_io: bool,
    }

    #[derive(Clone, Default)]
    struct FixtureEngine {
        state: Arc<Mutex<EngineState>>,
    }

    impl FixtureEngine {
        fn record(&self, call: &'static str) {
            self.state.lock().unwrap().calls.push(call);
        }
    }

    fn fixture_report(inputs: Vec<String>, context: &EngineContext) -> Report {
        Report::deterministic(
            inputs,
            Vec::new(),
            ReportOptionsSnapshot {
                fail_level: FailLevel::Validate,
                schema: context.schema.clone(),
                content_type: context.content_type.clone(),
                base_uri: context.base_uri.clone(),
            },
        )
    }

    impl CemMlEngine for FixtureEngine {
        fn parse(&self, request: ParseRequest) -> EngineResult<ParseResponse> {
            self.record("parse");
            if self.state.lock().unwrap().fail_parse_io {
                return Err(EngineError::Io {
                    path: PathBuf::from("fixture.cem"),
                    source: io::Error::other("fixture I/O failure"),
                });
            }
            Ok(ParseResponse {
                primary: json!({"input": request.input.uri}),
                primary_bytes: None,
                diagnostics: Vec::new(),
            })
        }

        fn validate(&self, request: ValidateRequest) -> EngineResult<ValidateResponse> {
            self.record("validate");
            Ok(ValidateResponse {
                report: fixture_report(
                    request.inputs.into_iter().map(|input| input.uri).collect(),
                    &request.context,
                ),
            })
        }

        fn check(&self, request: CheckRequest) -> EngineResult<CheckResponse> {
            self.record("check");
            Ok(CheckResponse {
                report: fixture_report(
                    request.inputs.into_iter().map(|input| input.uri).collect(),
                    &request.context,
                ),
                hard_violation_count: 0,
            })
        }

        fn inspect(&self, request: InspectRequest) -> EngineResult<InspectResponse> {
            self.record("inspect");
            Ok(InspectResponse {
                view: request.show,
                body: json!({"input": request.input.uri}),
                primary_bytes: None,
            })
        }

        fn convert(&self, request: ConvertRequest) -> EngineResult<ConvertResponse> {
            self.record("convert");
            Ok(ConvertResponse {
                primary: json!({"input": request.input.uri}),
                primary_bytes: None,
                conversion: None,
                diagnostics: Vec::new(),
                scheduler_trace: SchedulerTraceReport::default(),
            })
        }

        fn transform(&self, request: TransformRequest) -> EngineResult<TransformResponse> {
            self.record("transform");
            Ok(TransformResponse {
                primary: json!({"input": request.data.uri}),
                source_map: None,
                output_spans: Vec::new(),
                diagnostics: Vec::new(),
                scheduler_trace: SchedulerTraceReport::default(),
            })
        }

        fn transform_graph(
            &self,
            _request: TransformGraphRequest,
        ) -> EngineResult<TransformGraphResponse> {
            self.record("transform-graph");
            Ok(TransformGraphResponse {
                artifacts: vec![TransformGraphArtifact {
                    export_id: "graph-output".to_owned(),
                    input: "data".to_owned(),
                    destination: None,
                    identity: Some(json_identity()),
                    primary: json!({"graph": true}),
                    source_map: None,
                    output_spans: Vec::new(),
                }],
                diagnostics: Vec::new(),
                scheduler_trace: SchedulerTraceReport::default(),
            })
        }

        fn trace(&self, request: TraceRequest) -> EngineResult<TraceResponse> {
            self.record("trace");
            Ok(TraceResponse {
                body: json!({"input": request.input.uri}),
            })
        }

        fn bench(&self, _request: BenchRequest) -> EngineResult<BenchResponse> {
            unreachable!("command-service v1 does not prepare bench")
        }

        fn fixture_validate(
            &self,
            _request: FixtureValidateRequest,
        ) -> EngineResult<FixtureValidateResponse> {
            unreachable!("command-service v1 does not prepare fixture validation")
        }

        fn fixture_roundtrip(
            &self,
            _request: FixtureRoundtripRequest,
        ) -> EngineResult<FixtureRoundtripResponse> {
            unreachable!("command-service v1 does not prepare fixture roundtrip")
        }
    }

    #[derive(Default)]
    struct WriterState {
        events: Vec<String>,
        staged: BTreeMap<String, Vec<u8>>,
        committed: BTreeMap<String, Vec<u8>>,
        fail_commit: bool,
    }

    #[derive(Clone, Default)]
    struct FixtureWriter {
        state: Arc<Mutex<WriterState>>,
    }

    impl CommandResourceWriterV1 for FixtureWriter {
        fn prepare<'a>(
            &'a self,
            request: CommandResourceWriteRequestV1,
            bytes: &'a [u8],
        ) -> CommandHostFuture<
            'a,
            Result<Box<dyn CommandPreparedResourceWriteV1>, CommandPublicationHostFailureV1>,
        > {
            let mut state = self.state.lock().unwrap();
            state.events.push(format!("prepare:{}", request.uri));
            state.staged.insert(request.uri.clone(), bytes.to_vec());
            drop(state);
            let prepared: Box<dyn CommandPreparedResourceWriteV1> =
                Box::new(FixturePreparedWrite {
                    uri: request.uri,
                    state: Arc::clone(&self.state),
                });
            Box::pin(std::future::ready(Ok(prepared)))
        }
    }

    struct FixturePreparedWrite {
        uri: String,
        state: Arc<Mutex<WriterState>>,
    }

    impl CommandPreparedResourceWriteV1 for FixturePreparedWrite {
        fn commit<'a>(
            &'a mut self,
        ) -> CommandHostFuture<'a, Result<CommandResolvedWriteV1, CommandPublicationHostFailureV1>>
        {
            let mut state = self.state.lock().unwrap();
            state.events.push(format!("commit:{}", self.uri));
            if state.fail_commit {
                return Box::pin(std::future::ready(Err(
                    CommandPublicationHostFailureV1::new(
                        "fixture.commit",
                        "fixture commit failure",
                    ),
                )));
            }
            let bytes = state.staged.remove(&self.uri).unwrap();
            state.committed.insert(self.uri.clone(), bytes);
            Box::pin(std::future::ready(Ok(CommandResolvedWriteV1 {
                uri: self.uri.clone(),
            })))
        }

        fn rollback<'a>(
            &'a mut self,
        ) -> CommandHostFuture<'a, Result<(), CommandPublicationHostFailureV1>> {
            let mut state = self.state.lock().unwrap();
            state.events.push(format!("rollback:{}", self.uri));
            state.staged.remove(&self.uri);
            state.committed.remove(&self.uri);
            Box::pin(std::future::ready(Ok(())))
        }
    }

    #[derive(Clone)]
    struct FixtureLedger {
        value: CommandRevisionLedgerV1,
    }

    impl CommandRevisionLedgerReaderV1 for FixtureLedger {
        fn current<'a>(
            &'a self,
            _request: CommandRevisionLedgerRequestV1,
        ) -> CommandHostFuture<'a, Result<CommandRevisionLedgerV1, CommandPublicationHostFailureV1>>
        {
            Box::pin(std::future::ready(Ok(self.value.clone())))
        }
    }

    struct NoopWake;

    impl Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let waker = Waker::from(Arc::new(NoopWake));
        let mut context = Context::from_waker(&waker);
        let mut future = std::pin::pin!(future);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    fn capability() -> CapabilityManifest {
        capability_manifest(CapabilityRequest {
            runtime: RuntimeKind::Native,
            target_identity: "native-fixture".to_owned(),
            abi_identity: "rust-fixture".to_owned(),
            debug_control_active: false,
        })
        .unwrap()
    }

    fn json_identity() -> FormatIdentity {
        FormatIdentity {
            content_type: Some("application/json".to_owned()),
            ..FormatIdentity::default()
        }
    }

    fn input() -> EngineInput {
        EngineInput {
            uri: DATA_URI.to_owned(),
            bytes: b"<html><body><div id=\"fixture\"></div></body></html>".to_vec(),
            from_format: None,
            identity: Some(FormatIdentity {
                content_type: Some(HTML_CONTENT_TYPE.to_owned()),
                schema: Some(HTML_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        }
    }

    fn fixture_request(with_output: bool) -> CommandServiceRequestV1 {
        let bytes = input().bytes;
        let plan = parse_normalized_run_plan(NormalizedRunPlanRequest {
            input_records: vec![format!(
                "uri={DATA_URI},contentType={HTML_CONTENT_TYPE},schema={HTML_SCHEMA_URI}"
            )],
            output_records: if with_output {
                vec![format!(
                    "input={DATA_URI},dest={OUTPUT_URI},contentType=application/json"
                )]
            } else {
                Vec::new()
            },
            ..NormalizedRunPlanRequest::default()
        })
        .unwrap();
        CommandServiceRequestV1 {
            protocol_version: COMMAND_SERVICE_PROTOCOL_VERSION,
            request_id: "request:execution".to_owned(),
            project: CommandProjectRevisionV1 {
                project_id: "execution".to_owned(),
                revision: 1,
            },
            resource_versions: CommandUriMapV1::from(BTreeMap::from([(
                DATA_URI.to_owned(),
                CommandResourceVersionV1 {
                    revision: 1,
                    sha256: sha256_hex(&bytes),
                },
            )])),
            operation: PortableOperationRequestV1::Parse {
                input_id: "input:0".to_owned(),
                projection: ParseProjection::Json,
                preserve_source_offsets: false,
            },
            run_plan: CommandRunPlanV1::from(plan),
            resources: CommandUriMapV1::from(BTreeMap::from([(
                DATA_URI.to_owned(),
                VirtualResourceV1 {
                    bytes,
                    identity: None,
                },
            )])),
            policy_stamp: CommandPolicyStampV1 {
                resolver: "resolver:fixture".to_owned(),
                safety: "safety:fixture".to_owned(),
                budget: "budget:fixture".to_owned(),
            },
        }
    }

    fn ledger(request: &CommandServiceRequestV1) -> FixtureLedger {
        FixtureLedger {
            value: CommandRevisionLedgerV1 {
                project: request.project.clone(),
                resource_versions: request.resource_versions.clone(),
            },
        }
    }

    fn fixture_context(control: &OperationControl) -> EngineContext {
        EngineContext::default().with_operation_control(control.clone())
    }

    fn prepared_variants(
        control: &OperationControl,
        capability: &CapabilityManifest,
    ) -> Vec<PreparedPortableOperationV1> {
        let context = fixture_context(control);
        let input = input();
        let template = TemplateInput {
            uri: "studio://execution/template.cemt".to_owned(),
            bytes: b"template".to_vec(),
            identity: None,
            root_scope: ScopeConfig::default(),
        };
        vec![
            PreparedPortableOperationV1::Parse(ParseRequest {
                input: input.clone(),
                projection: ParseProjection::Json,
                fail_level: FailLevel::Validate,
                preserve_source_offsets: false,
                presentation_scope: None,
                context: context.clone(),
            }),
            PreparedPortableOperationV1::Validate(ValidateRequest {
                inputs: vec![input.clone()],
                projection: ValidateProjection::Json,
                fail_level: FailLevel::Validate,
                context: context.clone(),
            }),
            PreparedPortableOperationV1::Check(CheckRequest {
                inputs: vec![input.clone()],
                projection: ValidateProjection::Json,
                fail_level: FailLevel::Validate,
                zero_hard_violations: true,
                context: context.clone(),
            }),
            PreparedPortableOperationV1::Inspect(InspectRequest {
                input: input.clone(),
                show: InspectView::Summary,
                presentation_scope: None,
                context: context.clone(),
            }),
            PreparedPortableOperationV1::Convert(vec![PreparedCommandOutputV1 {
                output_id: Some("convert-output".to_owned()),
                destination: None,
                request: ConvertRequest {
                    input: input.clone(),
                    to_format: LayerFormat::Json,
                    preserve_source_offsets: false,
                    context: context.clone(),
                    target: Some(json_identity()),
                    target_scope: ScopeConfig::default(),
                    scheduler_scope_id: 0,
                },
            }]),
            PreparedPortableOperationV1::Query(Box::new(PreparedCommandQueryV1 {
                request: QueryRunRequest {
                    data: input.clone(),
                    query: QuerySource {
                        uri: QUERY_URI.to_owned(),
                        bytes: b"div".to_vec(),
                        identity: FormatIdentity {
                            content_type: Some(CSS_SELECTOR_CONTENT_TYPE.to_owned()),
                            schema: Some(CSS_SELECTOR_SCHEMA_URI.to_owned()),
                            ..FormatIdentity::default()
                        },
                    },
                    context: context.clone(),
                    context_item: None,
                    bindings: BTreeMap::new(),
                    limits: None,
                },
                output: QueryExportFormat::Json,
            })),
            PreparedPortableOperationV1::Transform(PreparedCommandTransformV1::Direct(vec![
                PreparedCommandOutputV1 {
                    output_id: Some("transform-output".to_owned()),
                    destination: None,
                    request: TransformRequest {
                        data: input.clone(),
                        template: template.clone(),
                        template_kind: TransformTemplateKind::CemNative,
                        template_entrypoint: TransformTemplateEntrypoint::implicit(),
                        params: BTreeMap::new(),
                        preserve_source_offsets: false,
                        context: context.clone(),
                        target: Some(json_identity()),
                        target_scope: ScopeConfig::default(),
                        scheduler_scope_ids: TransformSchedulerScopeIds::default(),
                        execution_policy: TransformExecutionPolicy::default(),
                    },
                },
            ])),
            PreparedPortableOperationV1::Transform(PreparedCommandTransformV1::Graph(Box::new(
                PreparedCommandTransformGraphV1 {
                    config_uri: "studio://execution/graph.cem".to_owned(),
                    request: TransformGraphRequest {
                        imports: Vec::new(),
                        joins: Vec::new(),
                        conversions: Vec::new(),
                        stages: Vec::new(),
                        importmap_rewrites: Vec::new(),
                        exports: Vec::new(),
                        edges: Vec::new(),
                        preserve_source_offsets: false,
                        context: context.clone(),
                        execution_policy: TransformExecutionPolicy::default(),
                    },
                },
            ))),
            PreparedPortableOperationV1::Trace(TraceRequest {
                input,
                projection: TraceProjection::Json,
                context,
            }),
            PreparedPortableOperationV1::VersionCapabilities(CommandVersionCapabilitiesResultV1 {
                version: product_version(),
                capability: capability.clone(),
            }),
        ]
    }

    fn fixture_invocation(
        request: CommandServiceRequestV1,
        prepared: PreparedPortableOperationV1,
        control: OperationControl,
    ) -> (
        Box<PreparedCommandServiceInvocationV1>,
        OperationHandle<CommandServiceResultV1>,
        crate::operation_handle::OperationEventSubscription<CommandServiceResultV1>,
    ) {
        let (operation, terminal_publisher) =
            OperationHandle::new(control, crate::capability::OperationHostLimits::default())
                .unwrap();
        let subscription = operation
            .subscribe(EventSubscriptionOptions::default())
            .unwrap();
        (
            Box::new(PreparedCommandServiceInvocationV1 {
                request: Box::new(request),
                prepared,
                operation: operation.clone(),
                terminal_publisher,
            }),
            operation,
            subscription,
        )
    }

    fn terminal_count(
        subscription: &mut crate::operation_handle::OperationEventSubscription<
            CommandServiceResultV1,
        >,
    ) -> usize {
        let mut count = 0;
        loop {
            let (status, event) = subscription.try_next().unwrap();
            if event.is_some_and(|event| event.kind == OperationEventKind::Terminal) {
                count += 1;
            }
            if status == EventSubscriptionPoll::Closed {
                return count;
            }
        }
    }

    fn succeeded_result(claim: &TerminalClaim<CommandServiceResultV1>) -> &CommandServiceResultV1 {
        let OperationOutcome::Succeeded { result, .. } = claim.outcome().as_ref() else {
            panic!("expected command result terminal outcome")
        };
        result
    }

    #[test]
    fn all_prepared_operations_use_common_boundaries_and_one_terminal() {
        let engine = FixtureEngine::default();
        let capability = capability();
        let writer = FixtureWriter::default();
        let mut exporters = QueryResultExporterRegistry::new();
        register_css_selector_query_exporters(&mut exporters);
        let mut result_operations = Vec::new();

        for operation_index in 0..10 {
            let signal = AbortSignal::new();
            let control = OperationControl::new(signal);
            let prepared = prepared_variants(&control, &capability).remove(operation_index);
            let request = fixture_request(false);
            let projection_request = request.clone();
            let fixture_ledger = ledger(&request);
            let (invocation, _operation, mut subscription) =
                fixture_invocation(request, prepared, control);
            let claim = block_on(execute_prepared_command_v1(
                &engine,
                invocation,
                &fixture_ledger,
                &writer,
                &exporters,
                &capability,
                CommandServiceLimitsV1::default(),
            ))
            .unwrap();
            assert!(claim.published());
            let result = succeeded_result(&claim);
            assert_eq!(result.status, CommandServiceStatusV1::Succeeded);
            assert_eq!(
                project_command_service_terminal_v1(
                    &projection_request,
                    &claim,
                    &capability,
                    CommandServiceLimitsV1::default(),
                )
                .request_id,
                projection_request.request_id
            );
            result_operations.push(result.operation);
            assert_eq!(terminal_count(&mut subscription), 1);
        }

        assert_eq!(
            result_operations,
            [
                CapabilityOperation::Parse,
                CapabilityOperation::Validate,
                CapabilityOperation::Check,
                CapabilityOperation::Inspect,
                CapabilityOperation::Convert,
                CapabilityOperation::Query,
                CapabilityOperation::Transform,
                CapabilityOperation::Transform,
                CapabilityOperation::Trace,
                CapabilityOperation::VersionCapabilities,
            ]
        );
        assert_eq!(
            engine.state.lock().unwrap().calls,
            [
                "parse",
                "validate",
                "check",
                "inspect",
                "convert",
                "transform",
                "transform-graph",
                "trace",
            ]
        );
    }

    #[test]
    fn fanout_results_preserve_output_identity_and_destination() {
        let engine = FixtureEngine::default();
        let capability = capability();
        let control = OperationControl::new(AbortSignal::new());
        let context = fixture_context(&control);
        let outputs = ["a", "b"]
            .into_iter()
            .map(|id| PreparedCommandOutputV1 {
                output_id: Some(id.to_owned()),
                destination: None,
                request: ConvertRequest {
                    input: input(),
                    to_format: LayerFormat::Json,
                    preserve_source_offsets: false,
                    context: context.clone(),
                    target: Some(json_identity()),
                    target_scope: ScopeConfig::default(),
                    scheduler_scope_id: 0,
                },
            })
            .collect();
        let request = fixture_request(false);
        let fixture_ledger = ledger(&request);
        let writer = FixtureWriter::default();
        let (invocation, _operation, mut subscription) = fixture_invocation(
            request,
            PreparedPortableOperationV1::Convert(outputs),
            control,
        );
        let claim = block_on(execute_prepared_command_v1(
            &engine,
            invocation,
            &fixture_ledger,
            &writer,
            &QueryResultExporterRegistry::new(),
            &capability,
            CommandServiceLimitsV1::default(),
        ))
        .unwrap();
        let Some(CommandPayloadV1::Inline {
            value: PortableOperationResultV1::Convert(result),
        }) = succeeded_result(&claim).result.as_ref()
        else {
            panic!("expected inline convert fan-out result")
        };
        assert_eq!(result.outputs.original_count, 2);
        assert_eq!(
            result
                .outputs
                .items
                .iter()
                .map(|output| output.output_id.as_deref())
                .collect::<Vec<_>>(),
            [Some("a"), Some("b")]
        );
        assert_eq!(terminal_count(&mut subscription), 1);
    }

    #[test]
    fn cancellation_and_engine_failure_map_without_duplicate_terminal() {
        let capability = capability();
        let engine = FixtureEngine::default();
        let signal = AbortSignal::new();
        let control = OperationControl::new(signal.clone());
        let prepared = prepared_variants(&control, &capability).remove(0);
        signal.abort();
        let request = fixture_request(false);
        let projection_request = request.clone();
        let fixture_ledger = ledger(&request);
        let writer = FixtureWriter::default();
        let (invocation, _operation, mut subscription) =
            fixture_invocation(request, prepared, control);
        let claim = block_on(execute_prepared_command_v1(
            &engine,
            invocation,
            &fixture_ledger,
            &writer,
            &QueryResultExporterRegistry::new(),
            &capability,
            CommandServiceLimitsV1::default(),
        ))
        .unwrap();
        assert!(matches!(
            claim.outcome().as_ref(),
            OperationOutcome::Cancelled { .. }
        ));
        let projected = project_command_service_terminal_v1(
            &projection_request,
            &claim,
            &capability,
            CommandServiceLimitsV1::default(),
        );
        assert_eq!(projected.status, CommandServiceStatusV1::Cancelled);
        assert_eq!(projected.exit_code, Some(EXIT_CANCELLED));
        assert_eq!(terminal_count(&mut subscription), 1);

        let failing_engine = FixtureEngine::default();
        failing_engine.state.lock().unwrap().fail_parse_io = true;
        let control = OperationControl::new(AbortSignal::new());
        let prepared = prepared_variants(&control, &capability).remove(0);
        let request = fixture_request(false);
        let fixture_ledger = ledger(&request);
        let (invocation, _operation, mut subscription) =
            fixture_invocation(request, prepared, control);
        let claim = block_on(execute_prepared_command_v1(
            &failing_engine,
            invocation,
            &fixture_ledger,
            &writer,
            &QueryResultExporterRegistry::new(),
            &capability,
            CommandServiceLimitsV1::default(),
        ))
        .unwrap();
        let result = succeeded_result(&claim);
        assert_eq!(result.status, CommandServiceStatusV1::Failed);
        assert_eq!(result.exit_code, Some(EXIT_IO));
        assert_eq!(terminal_count(&mut subscription), 1);
    }

    #[test]
    fn stale_terminal_projection_retains_current_revision_without_an_exit_code() {
        let request = fixture_request(false);
        let stale = CommandStaleRevisionV1 {
            current_project_revision: request.project.revision + 1,
            changed_resources: vec![CommandChangedResourceV1 {
                uri: DATA_URI.to_owned(),
                revision: 2,
                sha256: sha256_hex(b"changed"),
            }],
        };
        let result = stale_command_service_result_v1(
            &request,
            stale.clone(),
            &capability(),
            CommandServiceLimitsV1::default(),
        );
        assert_eq!(result.status, CommandServiceStatusV1::Stale);
        assert_eq!(result.exit_code, None);
        assert_eq!(result.stale, Some(stale));
        assert!(result.result.is_none());
        assert!(result.artifacts.items.is_empty());

        let cancelled = cancelled_command_service_result_v1(
            &request,
            Some("fixture cancellation"),
            &capability(),
            CommandServiceLimitsV1::default(),
        );
        assert_eq!(cancelled.status, CommandServiceStatusV1::Cancelled);
        assert_eq!(cancelled.exit_code, Some(EXIT_CANCELLED));
        assert_eq!(cancelled.diagnostics.items.len(), 1);
        assert!(cancelled.diagnostics.items[0]
            .message
            .contains("fixture cancellation"));
    }

    #[test]
    fn query_contract_failure_maps_to_usage_without_publication() {
        let capability = capability();
        let engine = FixtureEngine::default();
        let control = OperationControl::new(AbortSignal::new());
        let prepared = PreparedPortableOperationV1::Query(Box::new(PreparedCommandQueryV1 {
            request: QueryRunRequest {
                data: input(),
                query: QuerySource {
                    uri: QUERY_URI.to_owned(),
                    bytes: b"div".to_vec(),
                    identity: FormatIdentity::default(),
                },
                context: fixture_context(&control),
                context_item: None,
                bindings: BTreeMap::new(),
                limits: None,
            },
            output: QueryExportFormat::Json,
        }));
        let request = fixture_request(false);
        let fixture_ledger = ledger(&request);
        let writer = FixtureWriter::default();
        let (invocation, _operation, mut subscription) =
            fixture_invocation(request, prepared, control);
        let claim = block_on(execute_prepared_command_v1(
            &engine,
            invocation,
            &fixture_ledger,
            &writer,
            &QueryResultExporterRegistry::new(),
            &capability,
            CommandServiceLimitsV1::default(),
        ))
        .unwrap();
        let result = succeeded_result(&claim);
        assert_eq!(result.status, CommandServiceStatusV1::Failed);
        assert_eq!(result.exit_code, Some(EXIT_USAGE));
        assert_eq!(result.diagnostics.items[0].code, "cem.query.contract");
        assert!(writer.state.lock().unwrap().events.is_empty());
        assert_eq!(terminal_count(&mut subscription), 1);
    }

    #[test]
    fn publication_commits_or_rolls_back_before_one_terminal_result() {
        let capability = capability();
        let engine = FixtureEngine::default();
        let control = OperationControl::new(AbortSignal::new());
        let context = fixture_context(&control);
        let prepared = PreparedPortableOperationV1::Convert(vec![PreparedCommandOutputV1 {
            output_id: Some("published-output".to_owned()),
            destination: Some(OUTPUT_URI.to_owned()),
            request: ConvertRequest {
                input: input(),
                to_format: LayerFormat::Json,
                preserve_source_offsets: false,
                context,
                target: Some(json_identity()),
                target_scope: ScopeConfig::default(),
                scheduler_scope_id: 0,
            },
        }]);
        let request = fixture_request(true);
        let fixture_ledger = ledger(&request);
        let writer = FixtureWriter::default();
        let state = Arc::clone(&writer.state);
        let (invocation, _operation, mut subscription) =
            fixture_invocation(request, prepared, control);
        let execution = block_on(execute_prepared_command_with_artifacts_v1(
            &engine,
            invocation,
            &fixture_ledger,
            &writer,
            &QueryResultExporterRegistry::new(),
            &capability,
            CommandServiceLimitsV1::default(),
        ))
        .unwrap();
        let CommandServiceExecutionV1 {
            terminal: claim,
            artifacts,
        } = execution;
        let result = succeeded_result(&claim);
        assert_eq!(result.status, CommandServiceStatusV1::Succeeded);
        assert_eq!(result.artifacts.items.len(), 1);
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].handle, result.artifacts.items[0]);
        assert_eq!(
            artifacts[0].bytes.len() as u64,
            artifacts[0].handle.byte_length
        );
        assert_eq!(sha256_hex(&artifacts[0].bytes), artifacts[0].handle.sha256);
        assert!(state.lock().unwrap().committed.contains_key(OUTPUT_URI));
        assert_eq!(terminal_count(&mut subscription), 1);

        let control = OperationControl::new(AbortSignal::new());
        let context = fixture_context(&control);
        let prepared = PreparedPortableOperationV1::Convert(vec![PreparedCommandOutputV1 {
            output_id: Some("failed-output".to_owned()),
            destination: Some(OUTPUT_URI.to_owned()),
            request: ConvertRequest {
                input: input(),
                to_format: LayerFormat::Json,
                preserve_source_offsets: false,
                context,
                target: Some(json_identity()),
                target_scope: ScopeConfig::default(),
                scheduler_scope_id: 0,
            },
        }]);
        let request = fixture_request(true);
        let fixture_ledger = ledger(&request);
        let writer = FixtureWriter::default();
        writer.state.lock().unwrap().fail_commit = true;
        let state = Arc::clone(&writer.state);
        let (invocation, _operation, mut subscription) =
            fixture_invocation(request, prepared, control);
        let claim = block_on(execute_prepared_command_v1(
            &engine,
            invocation,
            &fixture_ledger,
            &writer,
            &QueryResultExporterRegistry::new(),
            &capability,
            CommandServiceLimitsV1::default(),
        ))
        .unwrap();
        let result = succeeded_result(&claim);
        assert_eq!(result.status, CommandServiceStatusV1::Failed);
        assert_eq!(result.exit_code, Some(EXIT_IO));
        let state = state.lock().unwrap();
        assert!(state.staged.is_empty());
        assert!(state.committed.is_empty());
        assert!(state
            .events
            .iter()
            .any(|event| event.starts_with("rollback:")));
        drop(state);
        assert_eq!(terminal_count(&mut subscription), 1);
    }

    #[test]
    fn stale_publication_rolls_back_and_settles_stale_once() {
        let capability = capability();
        let engine = FixtureEngine::default();
        let control = OperationControl::new(AbortSignal::new());
        let prepared = PreparedPortableOperationV1::Convert(vec![PreparedCommandOutputV1 {
            output_id: Some("stale-output".to_owned()),
            destination: Some(OUTPUT_URI.to_owned()),
            request: ConvertRequest {
                input: input(),
                to_format: LayerFormat::Json,
                preserve_source_offsets: false,
                context: fixture_context(&control),
                target: Some(json_identity()),
                target_scope: ScopeConfig::default(),
                scheduler_scope_id: 0,
            },
        }]);
        let request = fixture_request(true);
        let mut fixture_ledger = ledger(&request);
        fixture_ledger.value.project.revision += 1;
        let writer = FixtureWriter::default();
        let state = Arc::clone(&writer.state);
        let (invocation, _operation, mut subscription) =
            fixture_invocation(request, prepared, control);
        let claim = block_on(execute_prepared_command_v1(
            &engine,
            invocation,
            &fixture_ledger,
            &writer,
            &QueryResultExporterRegistry::new(),
            &capability,
            CommandServiceLimitsV1::default(),
        ))
        .unwrap();
        let result = succeeded_result(&claim);
        assert_eq!(result.status, CommandServiceStatusV1::Stale);
        assert_eq!(result.exit_code, None);
        assert!(result.stale.is_some());
        let state = state.lock().unwrap();
        assert!(state.staged.is_empty());
        assert!(state.committed.is_empty());
        assert!(state
            .events
            .iter()
            .any(|event| event.starts_with("rollback:")));
        drop(state);
        assert_eq!(terminal_count(&mut subscription), 1);
    }
}
