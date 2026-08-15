//! Rust-owned command-service request lifecycle before engine execution.
//!
//! The host owns live ledger and resource-reader capabilities outside the wire
//! request. It rejects invalid, unavailable, cancelled, and stale work before
//! creating engine inputs, then returns one owned prepared invocation carrying
//! the operation handle and its single terminal-publication token.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::capability::{
    CapabilityAvailability, CapabilityManifest, CapabilityOperation, CAPABILITY_CONTRACT_VERSION,
};
use crate::command_execution::{
    execute_prepared_command_v1, execute_prepared_command_with_artifacts_v1,
    CommandServiceExecutionV1,
};
use crate::command_host::{
    hydrate_command_service_operation_v1, CommandResourceHydrationErrorV1, CommandResourceReaderV1,
    CommandServiceHydrationV1,
};
use crate::command_operation::{
    prepare_command_operation_v1, CommandOperationPreparationError, PreparedPortableOperationV1,
};
use crate::command_publication::{
    CommandPublicationHostFailureV1, CommandResourceWriterV1, CommandRevisionLedgerReaderV1,
    CommandRevisionLedgerRequestV1,
};
use crate::command_service::{
    admit_command_service_request_v1, validate_command_service_limits_v1,
    validate_command_service_request_v1, CommandServiceAdmissionV1, CommandServiceError,
    CommandServiceLimitsV1, CommandServiceRequestV1, CommandServiceResultV1,
    CommandServiceStatusV1, CommandStaleRevisionV1, PortableOperationRequestV1,
};
use crate::engine::{CemMlEngine, EngineContext};
use crate::operation_control::{ControlError, OperationControl, OperationId};
use crate::operation_handle::{
    validate_operation_host_limits, ControlAckDisposition, OperationHandle, OperationHandleError,
    OperationTerminalPublisher, TerminalClaim,
};
use crate::query::QueryResultExporterRegistry;

/// Coarse command-service lifecycle stages streamed by host bindings while the
/// common operation handle retains detailed engine and control semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum CommandServiceProgressStageV1 {
    Accepted,
    Prepared,
    Executing,
    Terminal,
}

/// Bounded, monotonic progress record for one active command-service request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandServiceProgressV1 {
    pub protocol_version: u16,
    pub request_id: String,
    pub operation_id: OperationId,
    pub sequence: u64,
    pub stage: CommandServiceProgressStageV1,
    pub completed: u8,
    pub total: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<CommandServiceStatusV1>,
}

impl CommandServiceProgressV1 {
    pub fn new(
        request_id: impl Into<String>,
        operation_id: OperationId,
        sequence: u64,
        stage: CommandServiceProgressStageV1,
        status: Option<CommandServiceStatusV1>,
    ) -> Self {
        let completed = match stage {
            CommandServiceProgressStageV1::Accepted => 0,
            CommandServiceProgressStageV1::Prepared => 1,
            CommandServiceProgressStageV1::Executing => 2,
            CommandServiceProgressStageV1::Terminal => 3,
        };
        Self {
            protocol_version: crate::command_service::COMMAND_SERVICE_PROTOCOL_VERSION,
            request_id: request_id.into(),
            operation_id,
            sequence,
            stage,
            completed,
            total: 3,
            status,
        }
    }
}

/// Stable acknowledgement returned by command-service host cancellation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandServiceControlAckV1 {
    pub protocol_version: u16,
    pub request_id: String,
    pub operation_id: OperationId,
    pub selected_scope: crate::operation_control::ExecutionScopeId,
    pub disposition: ControlAckDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandServiceOperationRegistryErrorV1 {
    AlreadyActive { request_id: String },
    NotActive { request_id: String },
    Control(ControlError),
}

impl CommandServiceOperationRegistryErrorV1 {
    pub fn code(&self) -> &str {
        match self {
            Self::AlreadyActive { .. } => "cem.command_service.request_active",
            Self::NotActive { .. } => "cem.command_service.request_inactive",
            Self::Control(error) => error.code(),
        }
    }
}

impl fmt::Display for CommandServiceOperationRegistryErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyActive { request_id } => {
                write!(
                    formatter,
                    "command-service request `{request_id}` is already active"
                )
            }
            Self::NotActive { request_id } => {
                write!(
                    formatter,
                    "command-service request `{request_id}` is not active"
                )
            }
            Self::Control(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for CommandServiceOperationRegistryErrorV1 {}

#[derive(Debug, Clone)]
struct ActiveCommandServiceOperationV1 {
    operation_id: OperationId,
    control: OperationControl,
}

/// Clone-shared registry used by host bindings to route cancellation by the
/// public request identity without exposing a Rust pointer or live control in
/// the serializable request.
#[derive(Debug, Clone, Default)]
pub struct CommandServiceOperationRegistryV1 {
    active: Arc<Mutex<BTreeMap<String, ActiveCommandServiceOperationV1>>>,
}

impl CommandServiceOperationRegistryV1 {
    pub fn register(
        &self,
        request_id: &str,
        control: OperationControl,
    ) -> Result<CommandServiceOperationRegistrationV1, CommandServiceOperationRegistryErrorV1> {
        let mut active = self
            .active
            .lock()
            .expect("poisoned command-service registry");
        if active.contains_key(request_id) {
            return Err(CommandServiceOperationRegistryErrorV1::AlreadyActive {
                request_id: request_id.to_owned(),
            });
        }
        let operation_id = control.operation_id();
        active.insert(
            request_id.to_owned(),
            ActiveCommandServiceOperationV1 {
                operation_id,
                control,
            },
        );
        Ok(CommandServiceOperationRegistrationV1 {
            registry: self.clone(),
            request_id: request_id.to_owned(),
            operation_id,
        })
    }

    pub fn cancel(
        &self,
        request_id: &str,
        reason: Option<String>,
    ) -> Result<CommandServiceControlAckV1, CommandServiceOperationRegistryErrorV1> {
        let active = self
            .active
            .lock()
            .expect("poisoned command-service registry");
        let operation = active.get(request_id).cloned().ok_or_else(|| {
            CommandServiceOperationRegistryErrorV1::NotActive {
                request_id: request_id.to_owned(),
            }
        })?;
        drop(active);
        let disposition = operation
            .control
            .cancel_root(reason, None)
            .map(ControlAckDisposition::from)
            .map_err(CommandServiceOperationRegistryErrorV1::Control)?;
        Ok(CommandServiceControlAckV1 {
            protocol_version: crate::command_service::COMMAND_SERVICE_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            operation_id: operation.operation_id,
            selected_scope: operation.control.root_scope(),
            disposition,
        })
    }

    fn remove(&self, request_id: &str, operation_id: OperationId) {
        let mut active = self
            .active
            .lock()
            .expect("poisoned command-service registry");
        if active
            .get(request_id)
            .is_some_and(|operation| operation.operation_id == operation_id)
        {
            active.remove(request_id);
        }
    }
}

/// Drop guard guaranteeing that every success, stale result, cancellation, and
/// early host failure releases its request identity exactly once.
#[derive(Debug)]
pub struct CommandServiceOperationRegistrationV1 {
    registry: CommandServiceOperationRegistryV1,
    request_id: String,
    operation_id: OperationId,
}

impl CommandServiceOperationRegistrationV1 {
    pub fn operation_id(&self) -> OperationId {
        self.operation_id
    }
}

impl Drop for CommandServiceOperationRegistrationV1 {
    fn drop(&mut self) {
        self.registry.remove(&self.request_id, self.operation_id);
    }
}

pub struct CommandExecutionServicesV1 {
    pub writer: Box<dyn CommandResourceWriterV1>,
    pub query_exporters: QueryResultExporterRegistry,
}

impl fmt::Debug for CommandExecutionServicesV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommandExecutionServicesV1")
            .field("writer", &"installed")
            .field("query_exporters", &self.query_exporters)
            .finish()
    }
}

pub struct CommandServiceHostV1 {
    ledger_reader: Box<dyn CommandRevisionLedgerReaderV1>,
    resource_reader: Box<dyn CommandResourceReaderV1>,
    execution: CommandExecutionServicesV1,
    limits: CommandServiceLimitsV1,
    base_context: EngineContext,
    capability: CapabilityManifest,
}

impl fmt::Debug for CommandServiceHostV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommandServiceHostV1")
            .field("limits", &self.limits)
            .field("capability", &self.capability)
            .field("execution", &self.execution)
            .finish_non_exhaustive()
    }
}

impl CommandServiceHostV1 {
    pub fn new(
        ledger_reader: Box<dyn CommandRevisionLedgerReaderV1>,
        resource_reader: Box<dyn CommandResourceReaderV1>,
        execution: CommandExecutionServicesV1,
        limits: CommandServiceLimitsV1,
        base_context: EngineContext,
        capability: CapabilityManifest,
    ) -> Result<Self, CommandServiceHostErrorV1> {
        validate_host_contract(limits, &capability)?;
        Ok(Self {
            ledger_reader,
            resource_reader,
            execution,
            limits,
            base_context,
            capability,
        })
    }

    pub fn limits(&self) -> CommandServiceLimitsV1 {
        self.limits
    }

    pub fn capability(&self) -> &CapabilityManifest {
        &self.capability
    }

    pub fn query_exporters(&self) -> &QueryResultExporterRegistry {
        &self.execution.query_exporters
    }

    pub fn resource_writer(&self) -> &dyn CommandResourceWriterV1 {
        self.execution.writer.as_ref()
    }

    /// Admit, hydrate, and prepare one command request. No operation handle is
    /// created for stale or unavailable work, and the root control is checked
    /// around every asynchronous host boundary.
    pub async fn prepare(
        &self,
        request: CommandServiceRequestV1,
        control: OperationControl,
    ) -> Result<CommandServicePreparationV1, CommandServiceHostErrorV1> {
        validate_command_service_request_v1(&request, self.limits)?;
        check_control(&control)?;

        let operation = request_operation(&request.operation);
        let availability = self.capability.availability(operation);
        if availability == CapabilityAvailability::Unavailable {
            return Err(CommandServiceHostErrorV1::OperationUnavailable {
                operation,
                availability,
            });
        }

        let ledger = self
            .ledger_reader
            .current(CommandRevisionLedgerRequestV1 {
                request_id: request.request_id.clone(),
                project: request.project.clone(),
            })
            .await
            .map_err(|source| CommandServiceHostErrorV1::LedgerRead { source })?;
        check_control(&control)?;

        match admit_command_service_request_v1(&request, &ledger, self.limits)? {
            CommandServiceAdmissionV1::Stale(stale) => {
                return Ok(CommandServicePreparationV1::Stale(stale));
            }
            CommandServiceAdmissionV1::Accepted => {}
        }

        let (operation_handle, terminal_publisher) =
            OperationHandle::new(control.clone(), self.limits.operation_host)?;
        let hydrated = hydrate_command_service_operation_v1(
            &request,
            &ledger,
            self.resource_reader.as_ref(),
            self.limits,
            &control,
        )
        .await?;
        let CommandServiceHydrationV1::Ready(hydrated_request) = hydrated else {
            unreachable!("an unchanged admitted ledger cannot become stale during hydration")
        };
        check_control(&control)?;
        let operation_context = self
            .base_context
            .clone()
            .with_operation_control(control.clone());
        let prepared = prepare_command_operation_v1(
            &hydrated_request,
            self.limits,
            &operation_context,
            &self.capability,
        )?;
        check_control(&control)?;

        Ok(CommandServicePreparationV1::Ready(Box::new(
            PreparedCommandServiceInvocationV1 {
                request: hydrated_request,
                prepared,
                operation: operation_handle,
                terminal_publisher,
            },
        )))
    }

    /// Execute one ready invocation through the common engine/query boundary,
    /// transactionally publish requested artifacts, and settle its terminal
    /// operation outcome exactly once.
    pub async fn execute<E: CemMlEngine + ?Sized>(
        &self,
        engine: &E,
        invocation: Box<PreparedCommandServiceInvocationV1>,
    ) -> Result<TerminalClaim<CommandServiceResultV1>, CommandServiceHostErrorV1> {
        execute_prepared_command_v1(
            engine,
            invocation,
            self.ledger_reader.as_ref(),
            self.execution.writer.as_ref(),
            &self.execution.query_exporters,
            &self.capability,
            self.limits,
        )
        .await
        .map_err(CommandServiceHostErrorV1::Operation)
    }

    /// Execute one ready invocation while returning the committed owned
    /// artifact bytes needed by an outer request-scoped host registry.
    pub async fn execute_with_artifacts<E: CemMlEngine + ?Sized>(
        &self,
        engine: &E,
        invocation: Box<PreparedCommandServiceInvocationV1>,
    ) -> Result<CommandServiceExecutionV1, CommandServiceHostErrorV1> {
        execute_prepared_command_with_artifacts_v1(
            engine,
            invocation,
            self.ledger_reader.as_ref(),
            self.execution.writer.as_ref(),
            &self.execution.query_exporters,
            &self.capability,
            self.limits,
        )
        .await
        .map_err(CommandServiceHostErrorV1::Operation)
    }
}

#[derive(Debug)]
pub enum CommandServicePreparationV1 {
    Ready(Box<PreparedCommandServiceInvocationV1>),
    Stale(CommandStaleRevisionV1),
}

#[derive(Debug)]
pub struct PreparedCommandServiceInvocationV1 {
    pub request: Box<CommandServiceRequestV1>,
    pub prepared: PreparedPortableOperationV1,
    pub operation: OperationHandle<CommandServiceResultV1>,
    pub terminal_publisher: OperationTerminalPublisher<CommandServiceResultV1>,
}

#[derive(Debug)]
pub enum CommandServiceHostErrorV1 {
    HostContract {
        field: &'static str,
        message: String,
    },
    Request(CommandServiceError),
    OperationUnavailable {
        operation: CapabilityOperation,
        availability: CapabilityAvailability,
    },
    LedgerRead {
        source: CommandPublicationHostFailureV1,
    },
    Hydration(CommandResourceHydrationErrorV1),
    Preparation(CommandOperationPreparationError),
    Operation(OperationHandleError),
}

impl CommandServiceHostErrorV1 {
    pub fn code(&self) -> &str {
        match self {
            Self::HostContract { .. } => "cem.command_service.host_contract",
            Self::Request(error) => error.code(),
            Self::OperationUnavailable { .. } => "cem.command_service.operation_unavailable",
            Self::LedgerRead { .. } => "cem.command_service.ledger_read",
            Self::Hydration(error) => error.code(),
            Self::Preparation(error) => error.code(),
            Self::Operation(error) => error.code(),
        }
    }
}

impl fmt::Display for CommandServiceHostErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HostContract { field, message } => {
                write!(
                    formatter,
                    "command-service host {field} is incompatible: {message}"
                )
            }
            Self::Request(error) => error.fmt(formatter),
            Self::OperationUnavailable {
                operation,
                availability,
            } => write!(
                formatter,
                "command-service operation {operation:?} is {availability:?} on this host"
            ),
            Self::LedgerRead { source } => {
                write!(
                    formatter,
                    "command-service revision-ledger read failed: {source}"
                )
            }
            Self::Hydration(error) => error.fmt(formatter),
            Self::Preparation(error) => error.fmt(formatter),
            Self::Operation(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for CommandServiceHostErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Request(error) => Some(error),
            Self::LedgerRead { source } => Some(source),
            Self::Hydration(error) => Some(error),
            Self::Preparation(error) => Some(error),
            Self::Operation(error) => Some(error),
            _ => None,
        }
    }
}

impl From<CommandServiceError> for CommandServiceHostErrorV1 {
    fn from(error: CommandServiceError) -> Self {
        Self::Request(error)
    }
}

impl From<CommandResourceHydrationErrorV1> for CommandServiceHostErrorV1 {
    fn from(error: CommandResourceHydrationErrorV1) -> Self {
        Self::Hydration(error)
    }
}

impl From<CommandOperationPreparationError> for CommandServiceHostErrorV1 {
    fn from(error: CommandOperationPreparationError) -> Self {
        Self::Preparation(error)
    }
}

impl From<OperationHandleError> for CommandServiceHostErrorV1 {
    fn from(error: OperationHandleError) -> Self {
        Self::Operation(error)
    }
}

fn validate_host_contract(
    limits: CommandServiceLimitsV1,
    capability: &CapabilityManifest,
) -> Result<(), CommandServiceHostErrorV1> {
    validate_command_service_limits_v1(limits)?;
    validate_operation_host_limits(limits.operation_host)?;
    if capability.contract_version != CAPABILITY_CONTRACT_VERSION {
        return Err(CommandServiceHostErrorV1::HostContract {
            field: "contractVersion",
            message: format!(
                "capability reports {}; expected {CAPABILITY_CONTRACT_VERSION}",
                capability.contract_version
            ),
        });
    }
    if capability.common_version != crate::VERSION {
        return Err(CommandServiceHostErrorV1::HostContract {
            field: "commonVersion",
            message: format!(
                "capability reports `{}`; expected `{}`",
                capability.common_version,
                crate::VERSION
            ),
        });
    }
    if capability.operation_limits != limits.operation_host {
        return Err(CommandServiceHostErrorV1::HostContract {
            field: "operationLimits",
            message: "capability and command-service operation limits differ".to_owned(),
        });
    }
    for operation in CapabilityOperation::ALL {
        if capability
            .operations
            .iter()
            .filter(|entry| entry.operation == operation)
            .count()
            != 1
        {
            return Err(CommandServiceHostErrorV1::HostContract {
                field: "operations",
                message: format!("capability operation {operation:?} must appear exactly once"),
            });
        }
    }
    Ok(())
}

fn check_control(control: &OperationControl) -> Result<(), CommandServiceHostErrorV1> {
    control
        .check_scope(control.root_scope())
        .map_err(OperationHandleError::Control)
        .map_err(CommandServiceHostErrorV1::Operation)
}

const fn request_operation(operation: &PortableOperationRequestV1) -> CapabilityOperation {
    match operation {
        PortableOperationRequestV1::Parse { .. } => CapabilityOperation::Parse,
        PortableOperationRequestV1::Validate { .. } => CapabilityOperation::Validate,
        PortableOperationRequestV1::Check { .. } => CapabilityOperation::Check,
        PortableOperationRequestV1::Inspect { .. } => CapabilityOperation::Inspect,
        PortableOperationRequestV1::Convert { .. } => CapabilityOperation::Convert,
        PortableOperationRequestV1::Query { .. } => CapabilityOperation::Query,
        PortableOperationRequestV1::Transform { .. } => CapabilityOperation::Transform,
        PortableOperationRequestV1::Trace { .. } => CapabilityOperation::Trace,
        PortableOperationRequestV1::VersionCapabilities => CapabilityOperation::VersionCapabilities,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::future::Future;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Wake, Waker};

    use super::*;
    use crate::capability::{
        capability_manifest, CapabilityRequest, OperationCapability, RuntimeKind,
    };
    use crate::command_host::{
        CommandHostFuture, CommandResolvedResourceV1, CommandResourceReadFailureV1,
        CommandResourceReadRequestV1,
    };
    use crate::command_publication::{
        CommandPreparedResourceWriteV1, CommandResourceWriteRequestV1,
    };
    use crate::command_service::{
        sha256_hex, CommandPolicyStampV1, CommandProjectRevisionV1, CommandResourceVersionV1,
        CommandRevisionLedgerV1, CommandRunPlanV1, CommandUriMapV1,
        COMMAND_SERVICE_PROTOCOL_VERSION,
    };
    use crate::engine::ParseProjection;
    use crate::query::{
        QueryEncodedOutput, QueryExportFormat, QueryExportRequest, QueryLanguage,
        QueryResultExporter,
    };
    use crate::run_config::{
        parse_normalized_run_plan, NormalizedRunPlanRequest, RunConfigDefaults,
    };
    use crate::scheduler::AbortSignal;

    const DATA_URI: &str = "studio://catalog/data.cem";

    #[derive(Default)]
    struct FixtureState {
        events: Mutex<Vec<String>>,
        abort_after_read: Mutex<Option<AbortSignal>>,
    }

    struct FixtureLedger {
        response: Result<CommandRevisionLedgerV1, CommandPublicationHostFailureV1>,
        state: Arc<FixtureState>,
        abort_after_read: Option<AbortSignal>,
    }

    impl CommandRevisionLedgerReaderV1 for FixtureLedger {
        fn current<'a>(
            &'a self,
            _request: CommandRevisionLedgerRequestV1,
        ) -> CommandHostFuture<'a, Result<CommandRevisionLedgerV1, CommandPublicationHostFailureV1>>
        {
            self.state.events.lock().unwrap().push("ledger".to_owned());
            if let Some(signal) = &self.abort_after_read {
                signal.abort();
            }
            Box::pin(std::future::ready(self.response.clone()))
        }
    }

    struct FixtureReader {
        response: Result<CommandResolvedResourceV1, CommandResourceReadFailureV1>,
        state: Arc<FixtureState>,
    }

    impl CommandResourceReaderV1 for FixtureReader {
        fn read<'a>(
            &'a self,
            request: CommandResourceReadRequestV1,
        ) -> CommandHostFuture<'a, Result<CommandResolvedResourceV1, CommandResourceReadFailureV1>>
        {
            self.state
                .events
                .lock()
                .unwrap()
                .push(format!("read:{}", request.uri));
            if let Some(signal) = self.state.abort_after_read.lock().unwrap().as_ref() {
                signal.abort();
            }
            Box::pin(std::future::ready(self.response.clone()))
        }
    }

    struct FixtureWriter;

    impl CommandResourceWriterV1 for FixtureWriter {
        fn prepare<'a>(
            &'a self,
            _request: CommandResourceWriteRequestV1,
            _bytes: &'a [u8],
        ) -> CommandHostFuture<
            'a,
            Result<Box<dyn CommandPreparedResourceWriteV1>, CommandPublicationHostFailureV1>,
        > {
            Box::pin(std::future::ready(Err(
                CommandPublicationHostFailureV1::new(
                    "fixture.unexpected_write",
                    "preparation fixtures must not write",
                ),
            )))
        }
    }

    struct FixtureQueryExporter;

    impl QueryResultExporter for FixtureQueryExporter {
        fn id(&self) -> &'static str {
            "fixture-query-exporter"
        }

        fn language(&self) -> QueryLanguage {
            QueryLanguage::XPath
        }

        fn format(&self) -> QueryExportFormat {
            QueryExportFormat::Json
        }

        fn export(&self, _request: QueryExportRequest<'_>) -> Result<QueryEncodedOutput, String> {
            Ok(QueryEncodedOutput {
                content_type: "application/json".to_owned(),
                bytes: b"[]".to_vec(),
            })
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

    fn request() -> CommandServiceRequestV1 {
        let bytes = b"{@doc cem-ml 1}";
        let version = CommandResourceVersionV1 {
            revision: 1,
            sha256: sha256_hex(bytes),
        };
        let plan = parse_normalized_run_plan(NormalizedRunPlanRequest {
            input_records: vec![format!("uri={DATA_URI},contentType=application/cem+xml")],
            defaults: RunConfigDefaults::default(),
            ..NormalizedRunPlanRequest::default()
        })
        .expect("command host fixture plan");
        CommandServiceRequestV1 {
            protocol_version: COMMAND_SERVICE_PROTOCOL_VERSION,
            request_id: "request:host".to_owned(),
            project: CommandProjectRevisionV1 {
                project_id: "catalog".to_owned(),
                revision: 7,
            },
            resource_versions: CommandUriMapV1::from(BTreeMap::from([(
                DATA_URI.to_owned(),
                version,
            )])),
            operation: PortableOperationRequestV1::Parse {
                input_id: "input:0".to_owned(),
                projection: ParseProjection::Json,
                preserve_source_offsets: true,
            },
            run_plan: CommandRunPlanV1::from(plan),
            resources: CommandUriMapV1::new(),
            policy_stamp: CommandPolicyStampV1 {
                resolver: "resolver:host".to_owned(),
                safety: "safety:host".to_owned(),
                budget: "budget:host".to_owned(),
            },
        }
    }

    fn ledger(request: &CommandServiceRequestV1) -> CommandRevisionLedgerV1 {
        CommandRevisionLedgerV1 {
            project: request.project.clone(),
            resource_versions: request.resource_versions.clone(),
        }
    }

    fn capability() -> CapabilityManifest {
        capability_manifest(CapabilityRequest {
            runtime: RuntimeKind::Native,
            target_identity: "native-test".to_owned(),
            abi_identity: "rust-test".to_owned(),
            debug_control_active: false,
        })
        .expect("command host fixture capability")
    }

    fn resource(request: &CommandServiceRequestV1) -> CommandResolvedResourceV1 {
        CommandResolvedResourceV1 {
            version: request.resource_versions.get(DATA_URI).unwrap().clone(),
            bytes: b"{@doc cem-ml 1}".to_vec(),
            identity: None,
        }
    }

    fn host(
        ledger_response: Result<CommandRevisionLedgerV1, CommandPublicationHostFailureV1>,
        reader_response: Result<CommandResolvedResourceV1, CommandResourceReadFailureV1>,
        state: Arc<FixtureState>,
        capability: CapabilityManifest,
        abort_after_ledger: Option<AbortSignal>,
    ) -> Result<CommandServiceHostV1, CommandServiceHostErrorV1> {
        let mut query_exporters = QueryResultExporterRegistry::new();
        query_exporters.register(FixtureQueryExporter);
        CommandServiceHostV1::new(
            Box::new(FixtureLedger {
                response: ledger_response,
                state: Arc::clone(&state),
                abort_after_read: abort_after_ledger,
            }),
            Box::new(FixtureReader {
                response: reader_response,
                state,
            }),
            CommandExecutionServicesV1 {
                writer: Box::new(FixtureWriter),
                query_exporters,
            },
            CommandServiceLimitsV1::default(),
            EngineContext::default(),
            capability,
        )
    }

    #[test]
    fn operation_registry_owns_duplicate_cancel_cleanup_and_progress_projection() {
        let registry = CommandServiceOperationRegistryV1::default();
        let control = OperationControl::new(AbortSignal::new());
        let operation_id = control.operation_id();
        let registration = registry
            .register("request:registry", control.clone())
            .expect("first registration succeeds");
        assert_eq!(registration.operation_id(), operation_id);

        let duplicate = registry
            .register("request:registry", OperationControl::default())
            .expect_err("duplicate request is rejected");
        assert_eq!(duplicate.code(), "cem.command_service.request_active");

        let accepted = registry
            .cancel("request:registry", Some("fixture cancellation".to_owned()))
            .expect("root cancellation is routed");
        assert_eq!(accepted.operation_id, operation_id);
        assert_eq!(accepted.disposition, ControlAckDisposition::Accepted);
        assert!(control.is_cancelled());
        assert_eq!(
            control.abort_signal().reason().as_deref(),
            Some("fixture cancellation")
        );
        assert_eq!(
            registry
                .cancel("request:registry", None)
                .expect("repeat cancellation is idempotent")
                .disposition,
            ControlAckDisposition::AlreadyRequested
        );

        let progress = [
            CommandServiceProgressStageV1::Accepted,
            CommandServiceProgressStageV1::Prepared,
            CommandServiceProgressStageV1::Executing,
            CommandServiceProgressStageV1::Terminal,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, stage)| {
            CommandServiceProgressV1::new(
                "request:registry",
                operation_id,
                index as u64 + 1,
                stage,
                (stage == CommandServiceProgressStageV1::Terminal)
                    .then_some(CommandServiceStatusV1::Cancelled),
            )
        })
        .collect::<Vec<_>>();
        assert_eq!(
            progress
                .iter()
                .map(|event| event.sequence)
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
        assert_eq!(
            serde_json::to_value(progress.last().unwrap()).unwrap(),
            serde_json::json!({
                "protocolVersion": 1,
                "requestId": "request:registry",
                "operationId": operation_id,
                "sequence": 4,
                "stage": "terminal",
                "completed": 3,
                "total": 3,
                "status": "cancelled"
            })
        );

        drop(registration);
        let reused = registry
            .register("request:registry", OperationControl::default())
            .expect("drop releases request identity for reuse");
        drop(reused);
        assert_eq!(
            registry
                .cancel("request:registry", None)
                .expect_err("completed request is no longer active")
                .code(),
            "cem.command_service.request_inactive"
        );
    }

    #[test]
    fn host_reads_ledger_then_missing_resource_and_returns_owned_preparation() {
        let request = request();
        let control = OperationControl::new(AbortSignal::new());
        let operation_id = control.operation_id();
        let state = Arc::new(FixtureState::default());
        let host = host(
            Ok(ledger(&request)),
            Ok(resource(&request)),
            Arc::clone(&state),
            capability(),
            None,
        )
        .unwrap();

        let outcome = block_on(host.prepare(request, control)).unwrap();
        let CommandServicePreparationV1::Ready(invocation) = outcome else {
            panic!("expected prepared invocation")
        };
        assert_eq!(invocation.operation.operation_id(), operation_id);
        assert!(invocation.request.resources.contains_key(DATA_URI));
        let PreparedPortableOperationV1::Parse(prepared) = invocation.prepared else {
            panic!("expected prepared parse operation")
        };
        assert_eq!(
            prepared.context.operation_control.operation_id(),
            operation_id
        );
        assert_eq!(
            state.events.lock().unwrap().as_slice(),
            ["ledger".to_owned(), format!("read:{DATA_URI}")]
        );
    }

    #[test]
    fn stale_and_unavailable_requests_stop_before_hydration() {
        let request = request();
        let state = Arc::new(FixtureState::default());
        let mut stale = ledger(&request);
        stale.project.revision += 1;
        let stale_host = host(
            Ok(stale),
            Ok(resource(&request)),
            Arc::clone(&state),
            capability(),
            None,
        )
        .unwrap();
        let outcome = block_on(
            stale_host.prepare(request.clone(), OperationControl::new(AbortSignal::new())),
        )
        .unwrap();
        assert!(matches!(outcome, CommandServicePreparationV1::Stale(_)));
        assert_eq!(state.events.lock().unwrap().as_slice(), ["ledger"]);

        state.events.lock().unwrap().clear();
        let mut unavailable = capability();
        unavailable.operations = unavailable
            .operations
            .into_iter()
            .map(|entry| {
                if entry.operation == CapabilityOperation::Parse {
                    OperationCapability {
                        availability: CapabilityAvailability::Unavailable,
                        ..entry
                    }
                } else {
                    entry
                }
            })
            .collect();
        let unavailable_host = host(
            Ok(ledger(&request)),
            Ok(resource(&request)),
            Arc::clone(&state),
            unavailable,
            None,
        )
        .unwrap();
        let error =
            block_on(unavailable_host.prepare(request, OperationControl::new(AbortSignal::new())))
                .unwrap_err();
        assert_eq!(error.code(), "cem.command_service.operation_unavailable");
        assert!(state.events.lock().unwrap().is_empty());
    }

    #[test]
    fn cancellation_and_host_failures_preserve_typed_boundary_errors() {
        let request = request();
        let state = Arc::new(FixtureState::default());
        let signal = AbortSignal::new();
        let cancelled = host(
            Ok(ledger(&request)),
            Ok(resource(&request)),
            Arc::clone(&state),
            capability(),
            Some(signal.clone()),
        )
        .unwrap();
        let error = block_on(cancelled.prepare(request.clone(), OperationControl::new(signal)))
            .unwrap_err();
        assert_eq!(error.code(), "host-cancellation");
        assert_eq!(state.events.lock().unwrap().as_slice(), ["ledger"]);

        state.events.lock().unwrap().clear();
        let read_signal = AbortSignal::new();
        *state.abort_after_read.lock().unwrap() = Some(read_signal.clone());
        let cancelled_after_read = host(
            Ok(ledger(&request)),
            Ok(resource(&request)),
            Arc::clone(&state),
            capability(),
            None,
        )
        .unwrap();
        let error = block_on(
            cancelled_after_read.prepare(request.clone(), OperationControl::new(read_signal)),
        )
        .unwrap_err();
        assert_eq!(error.code(), "host-cancellation");
        assert_eq!(
            state.events.lock().unwrap().as_slice(),
            ["ledger".to_owned(), format!("read:{DATA_URI}")]
        );
        *state.abort_after_read.lock().unwrap() = None;

        state.events.lock().unwrap().clear();
        let ledger_failure = host(
            Err(CommandPublicationHostFailureV1::new(
                "fixture.ledger",
                "ledger unavailable",
            )),
            Ok(resource(&request)),
            Arc::clone(&state),
            capability(),
            None,
        )
        .unwrap();
        assert_eq!(
            block_on(
                ledger_failure.prepare(request.clone(), OperationControl::new(AbortSignal::new()),)
            )
            .unwrap_err()
            .code(),
            "cem.command_service.ledger_read"
        );

        state.events.lock().unwrap().clear();
        let read_failure = host(
            Ok(ledger(&request)),
            Err(CommandResourceReadFailureV1::new(
                "fixture.read",
                "resource unavailable",
            )),
            Arc::clone(&state),
            capability(),
            None,
        )
        .unwrap();
        assert_eq!(
            block_on(read_failure.prepare(request, OperationControl::new(AbortSignal::new()),))
                .unwrap_err()
                .code(),
            "cem.command_service.host_read"
        );
        assert_eq!(
            state.events.lock().unwrap().as_slice(),
            ["ledger".to_owned(), format!("read:{DATA_URI}")]
        );
    }

    #[test]
    fn constructor_rejects_capability_and_limit_drift() {
        let request = request();
        let state = Arc::new(FixtureState::default());
        let installed = host(
            Ok(ledger(&request)),
            Ok(resource(&request)),
            Arc::clone(&state),
            capability(),
            None,
        )
        .unwrap();
        assert!(installed
            .query_exporters()
            .contains(QueryLanguage::XPath, QueryExportFormat::Json));
        let _writer = installed.resource_writer();

        let mut drifted = capability();
        drifted.operation_limits.max_retained_handles -= 1;
        let error = host(
            Ok(ledger(&request)),
            Ok(resource(&request)),
            state,
            drifted,
            None,
        )
        .unwrap_err();
        assert_eq!(error.code(), "cem.command_service.host_contract");
    }

    #[test]
    fn version_capabilities_prepares_without_resource_reads() {
        let mut request = request();
        request.operation = PortableOperationRequestV1::VersionCapabilities;
        request.run_plan = CommandRunPlanV1::Null(());
        request.resource_versions = CommandUriMapV1::new();
        let state = Arc::new(FixtureState::default());
        let host = host(
            Ok(ledger(&request)),
            Err(CommandResourceReadFailureV1::new(
                "fixture.unexpected",
                "reader must not be called",
            )),
            Arc::clone(&state),
            capability(),
            None,
        )
        .unwrap();
        let outcome =
            block_on(host.prepare(request, OperationControl::new(AbortSignal::new()))).unwrap();
        let CommandServicePreparationV1::Ready(invocation) = outcome else {
            panic!("expected prepared capability invocation")
        };
        assert!(matches!(
            invocation.prepared,
            PreparedPortableOperationV1::VersionCapabilities(_)
        ));
        assert_eq!(state.events.lock().unwrap().as_slice(), ["ledger"]);
    }
}
