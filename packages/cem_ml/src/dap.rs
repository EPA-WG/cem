//! Bounded Debug Adapter Protocol projection over [`OperationHandle`].
//!
//! The adapter owns protocol/session state only. Hosts own command launch,
//! attachment, and byte transport. DAP threads are logical engine tasks and all
//! stopped-state references are valid only for the current stop generation.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::debug_control::{
    BreakpointId, BreakpointResolution, DebugControlError, DebugSourceSelector, PauseSpec,
    StepMode, StepRequest, StopReason, StopToken, VariableFilter,
};
use crate::operation_control::{ExecutionScopeId, OperationId, TaskId};
use crate::operation_handle::{
    CancelRequest, EventSubscriptionOptions, EventSubscriptionPoll, OperationEventKind,
    OperationEventSubscription, OperationHandle, OperationHandleError, OperationSourceSelector,
};

pub const DAP_ADAPTER_VERSION: u16 = crate::capability::DEBUG_DAP_ADAPTER_VERSION;
pub const CEM_DEBUG_REQUEST_VERSION: u16 = crate::capability::DEBUG_REQUEST_VERSION;
pub const MAX_DAP_MESSAGE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DapRequest {
    pub seq: i32,
    #[serde(rename = "type")]
    pub message_type: String,
    pub command: String,
    #[serde(default)]
    pub arguments: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DapAdapterError {
    pub code: &'static str,
    pub message: String,
}

impl DapAdapterError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for DapAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for DapAdapterError {}

impl From<DebugControlError> for DapAdapterError {
    fn from(error: DebugControlError) -> Self {
        Self::new(error.code(), error.to_string())
    }
}

impl From<OperationHandleError> for DapAdapterError {
    fn from(error: OperationHandleError) -> Self {
        Self::new(error.code(), error.to_string())
    }
}

/// Native host boundary. The same adapter can be driven by the CLI, a test
/// fixture, or a future embedded editor host without moving engine semantics.
pub trait DapOperationHost<R: Send + Sync + 'static> {
    fn launch(&mut self, arguments: &Value) -> Result<OperationHandle<R>, DapAdapterError>;
    fn attach(&mut self, arguments: &Value) -> Result<OperationHandle<R>, DapAdapterError>;
    fn configuration_done(&mut self) -> Result<(), DapAdapterError> {
        Ok(())
    }
    fn disconnect(&mut self, _terminate: bool) -> Result<(), DapAdapterError> {
        Ok(())
    }
    fn supports_conditional_breakpoints(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperationOwnership {
    Launched,
    Attached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DapPathFormat {
    #[default]
    Path,
    Uri,
}

pub struct DapSession<R: Send + Sync + 'static> {
    next_sequence: i32,
    active: bool,
    initialized: bool,
    disconnected: bool,
    ownership: Option<OperationOwnership>,
    operation: Option<OperationHandle<R>>,
    events: Option<OperationEventSubscription<R>>,
    current_stop: Option<StopToken>,
    current_thread: Option<TaskId>,
    pending_continued_thread: Option<TaskId>,
    lines_start_at_one: bool,
    columns_start_at_one: bool,
    path_format: DapPathFormat,
    client_paths_by_uri: BTreeMap<String, String>,
    breakpoints_by_source: BTreeMap<String, Vec<BreakpointId>>,
    transient_breakpoints: BTreeSet<BreakpointId>,
}

impl<R: Send + Sync + 'static> fmt::Debug for DapSession<R> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DapSession")
            .field("active", &self.active)
            .field("initialized", &self.initialized)
            .field("disconnected", &self.disconnected)
            .field("ownership", &self.ownership)
            .field(
                "operation_id",
                &self.operation.as_ref().map(OperationHandle::operation_id),
            )
            .field("current_stop", &self.current_stop)
            .finish_non_exhaustive()
    }
}

impl<R: Send + Sync + 'static> DapSession<R> {
    pub fn new(active: bool) -> Self {
        Self {
            next_sequence: 1,
            active,
            initialized: false,
            disconnected: false,
            ownership: None,
            operation: None,
            events: None,
            current_stop: None,
            current_thread: None,
            pending_continued_thread: None,
            lines_start_at_one: true,
            columns_start_at_one: true,
            path_format: DapPathFormat::Path,
            client_paths_by_uri: BTreeMap::new(),
            breakpoints_by_source: BTreeMap::new(),
            transient_breakpoints: BTreeSet::new(),
        }
    }

    pub fn operation_id(&self) -> Option<OperationId> {
        self.operation.as_ref().map(OperationHandle::operation_id)
    }

    pub fn disconnected(&self) -> bool {
        self.disconnected
    }

    pub fn output_event(&mut self, category: &str, output: &str) -> Value {
        self.event("output", json!({ "category": category, "output": output }))
    }

    pub fn handle_request<H: DapOperationHost<R>>(
        &mut self,
        request: DapRequest,
        host: &mut H,
    ) -> Vec<Value> {
        if request.message_type != "request" {
            return vec![self.error_response(
                &request,
                DapAdapterError::new("cem.dap.message_type", "expected a DAP request"),
            )];
        }
        let mut after = Vec::new();
        let result = self.dispatch(&request, host, &mut after);
        let mut messages = vec![match result {
            Ok(body) => self.success_response(&request, body),
            Err(error) => self.error_response(&request, error),
        }];
        messages.append(&mut after);
        messages
    }

    pub fn poll_events(&mut self) -> Vec<Value> {
        let mut messages = Vec::new();
        while let Some(events) = self.events.as_mut() {
            let Ok((status, event)) = events.try_next() else {
                break;
            };
            match status {
                EventSubscriptionPoll::Pending | EventSubscriptionPoll::Closed => break,
                EventSubscriptionPoll::Event => {}
            }
            let Some(event) = event else {
                continue;
            };
            match event.kind {
                OperationEventKind::Stopped => {
                    if let Ok(stop) = serde_json::from_value::<crate::debug_control::StoppedEvent>(
                        event.payload.clone(),
                    ) {
                        self.current_stop = Some(stop.stop);
                        self.current_thread = stop.triggering_task;
                        let mut body = json!({
                            "reason": dap_stop_reason(stop.reason),
                            "allThreadsStopped": true,
                        });
                        insert_optional(
                            &mut body,
                            "threadId",
                            stop.triggering_task
                                .and_then(|task| dap_i32_id(task.get(), "thread").ok())
                                .map(|task| json!(task)),
                        );
                        insert_optional(
                            &mut body,
                            "hitBreakpointIds",
                            stop.breakpoint_id
                                .and_then(|id| dap_i32_id(id.get(), "breakpoint").ok())
                                .map(|id| json!([id])),
                        );
                        messages.push(self.event("stopped", body));
                    }
                }
                OperationEventKind::Continued => {
                    if let Ok(continued) = serde_json::from_value::<
                        crate::debug_control::ContinuedEvent,
                    >(event.payload.clone())
                    {
                        self.current_stop = None;
                        let thread = continued
                            .stepping_task
                            .or(self.pending_continued_thread.take())
                            .or(self.current_thread);
                        if let Some(thread) =
                            thread.and_then(|task| dap_i32_id(task.get(), "thread").ok())
                        {
                            messages.push(self.event(
                                "continued",
                                json!({
                                    "threadId": thread,
                                    "allThreadsContinued": continued.all_threads_continued,
                                }),
                            ));
                        }
                        self.current_thread = None;
                    }
                }
                OperationEventKind::BreakpointResolved => {
                    if let Ok(resolution) =
                        serde_json::from_value::<BreakpointResolution>(event.payload.clone())
                    {
                        if self
                            .transient_breakpoints
                            .contains(&resolution.breakpoint_id)
                        {
                            continue;
                        }
                        if let Ok(breakpoint) = self.project_breakpoint_resolution(resolution) {
                            messages.push(self.event(
                                "breakpoint",
                                json!({ "reason": "changed", "breakpoint": breakpoint }),
                            ));
                        }
                    }
                }
                OperationEventKind::Terminal => {
                    self.current_stop = None;
                    self.current_thread = None;
                    self.pending_continued_thread = None;
                    let exit_code = match event.payload.get("status").and_then(Value::as_str) {
                        Some("succeeded") => 0,
                        Some("cancelled") => 130,
                        _ => 1,
                    };
                    messages.push(self.event("exited", json!({ "exitCode": exit_code })));
                    messages.push(self.event("terminated", json!({})));
                }
                _ => {}
            }
        }
        messages
    }

    /// Apply the documented ownership default after unexpected transport loss.
    pub fn transport_lost(&mut self) {
        let terminate = self.ownership == Some(OperationOwnership::Launched);
        let _ = self.detach(terminate);
        self.disconnected = true;
    }

    fn dispatch<H: DapOperationHost<R>>(
        &mut self,
        request: &DapRequest,
        host: &mut H,
        after: &mut Vec<Value>,
    ) -> Result<Value, DapAdapterError> {
        match request.command.as_str() {
            "initialize" => {
                if self.initialized {
                    return Err(DapAdapterError::new(
                        "cem.dap.already_initialized",
                        "initialize may be requested only once per DAP session",
                    ));
                }
                self.lines_start_at_one = request
                    .arguments
                    .get("linesStartAt1")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                self.columns_start_at_one = request
                    .arguments
                    .get("columnsStartAt1")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                self.path_format = match request
                    .arguments
                    .get("pathFormat")
                    .and_then(Value::as_str)
                    .unwrap_or("path")
                {
                    "path" => DapPathFormat::Path,
                    "uri" => DapPathFormat::Uri,
                    other => {
                        return Err(DapAdapterError::new(
                            "cem.dap.path_format_unsupported",
                            format!("unsupported DAP pathFormat `{other}`"),
                        ));
                    }
                };
                self.initialized = true;
                let body = if self.active {
                    json!({
                        "supportsConfigurationDoneRequest": true,
                        "supportsTerminateRequest": true,
                        "supportsBreakpointLocationsRequest": true,
                        "supportsCancelRequest": true,
                        "supportsConditionalBreakpoints": host.supports_conditional_breakpoints(),
                        "supportsHitConditionalBreakpoints": true,
                        "supportTerminateDebuggee": true,
                        "cemDapAdapterVersion": DAP_ADAPTER_VERSION,
                        "cemDebugRequestVersion": CEM_DEBUG_REQUEST_VERSION,
                    })
                } else {
                    json!({})
                };
                Ok(body)
            }
            "launch" => {
                self.ensure_initialized_active()?;
                self.ensure_unbound()?;
                let operation = host.launch(&request.arguments)?;
                self.bind(operation, OperationOwnership::Launched)?;
                after.push(self.event("initialized", json!({})));
                Ok(json!({}))
            }
            "attach" => {
                self.ensure_initialized_active()?;
                self.ensure_unbound()?;
                let operation = host.attach(&request.arguments)?;
                self.bind(operation, OperationOwnership::Attached)?;
                after.push(self.event("initialized", json!({})));
                Ok(json!({}))
            }
            "configurationDone" => {
                self.operation()?;
                host.configuration_done()?;
                Ok(json!({}))
            }
            "setBreakpoints" => self.set_breakpoints(&request.arguments),
            "breakpointLocations" => self.breakpoint_locations(&request.arguments),
            "threads" => self.threads(),
            "stackTrace" => self.stack_trace(&request.arguments),
            "scopes" => self.scopes(&request.arguments),
            "variables" => self.variables(&request.arguments),
            "pause" => self.pause(&request.arguments),
            "continue" => self.continue_operation(&request.arguments),
            "next" => self.step(&request.arguments, StepMode::Next),
            "stepIn" => self.step(&request.arguments, StepMode::StepIn),
            "stepOut" => self.step(&request.arguments, StepMode::StepOut),
            "terminate" => {
                self.cancel_root("DAP terminate")?;
                Ok(json!({}))
            }
            // DAP cancel addresses an in-flight protocol request only. This
            // synchronous v1 adapter has no cancellable protocol work to abort.
            "cancel" => Ok(json!({})),
            "disconnect" => {
                let terminate = request
                    .arguments
                    .get("terminateDebuggee")
                    .and_then(Value::as_bool)
                    .unwrap_or(self.ownership == Some(OperationOwnership::Launched));
                self.detach(terminate)?;
                host.disconnect(terminate)?;
                self.disconnected = true;
                Ok(json!({}))
            }
            "cem/operation" => self.cem_operation(&request.arguments),
            "cem/executionScopes" => self.cem_execution_scopes(&request.arguments),
            "cem/cancel" => self.cem_cancel(&request.arguments),
            "cem/nativeValue" => self.cem_native_value(&request.arguments),
            "cem/workerTopology" => self.cem_worker_topology(&request.arguments),
            _ => Err(DapAdapterError::new(
                "cem.dap.command_unsupported",
                format!("unsupported DAP command `{}`", request.command),
            )),
        }
    }

    fn ensure_initialized_active(&self) -> Result<(), DapAdapterError> {
        if !self.initialized {
            return Err(DapAdapterError::new(
                "cem.dap.not_initialized",
                "initialize must precede launch or attach",
            ));
        }
        if !self.active {
            return Err(DapAdapterError::new(
                "cem.debug.inactive",
                "debug control is not active",
            ));
        }
        Ok(())
    }

    fn bind(
        &mut self,
        operation: OperationHandle<R>,
        ownership: OperationOwnership,
    ) -> Result<(), DapAdapterError> {
        self.ensure_unbound()?;
        if !operation.debug_control_active() {
            return Err(DapAdapterError::new(
                "cem.debug.inactive",
                "bound operation did not activate debug control",
            ));
        }
        let filters = BTreeSet::from([
            OperationEventKind::BreakpointResolved,
            OperationEventKind::Stopped,
            OperationEventKind::Continued,
            OperationEventKind::Terminal,
        ]);
        self.events = Some(operation.subscribe(EventSubscriptionOptions {
            from_sequence: Some(1),
            capacity: None,
            filters,
        })?);
        self.ownership = Some(ownership);
        self.operation = Some(operation);
        Ok(())
    }

    fn ensure_unbound(&self) -> Result<(), DapAdapterError> {
        if self.operation.is_some() {
            return Err(DapAdapterError::new(
                "cem.dap.operation_already_bound",
                "one DAP session may bind exactly one operation",
            ));
        }
        Ok(())
    }

    fn operation(&self) -> Result<&OperationHandle<R>, DapAdapterError> {
        self.operation.as_ref().ok_or_else(|| {
            DapAdapterError::new("cem.dap.operation_unbound", "launch or attach is required")
        })
    }

    fn stop(&self) -> Result<StopToken, DapAdapterError> {
        self.current_stop.ok_or_else(|| {
            DapAdapterError::new("cem.debug.not_stopped", "operation is not stopped")
        })
    }

    fn set_breakpoints(&mut self, arguments: &Value) -> Result<Value, DapAdapterError> {
        let source_uri = self.source_uri(arguments)?;
        self.operation()?;
        let prior = self
            .breakpoints_by_source
            .remove(&source_uri)
            .unwrap_or_default();
        for breakpoint in prior {
            self.operation()?.remove_pause_trigger(breakpoint)?;
        }
        let requested = arguments
            .get("breakpoints")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut installed = Vec::new();
        let mut projected = Vec::with_capacity(requested.len());
        for breakpoint in requested {
            let Some(client_line) = u32_field(&breakpoint, "line") else {
                projected.push(json!({ "verified": false, "message": "line is required" }));
                continue;
            };
            let line = match self.client_line(client_line) {
                Ok(line) => line,
                Err(error) => {
                    projected.push(json!({ "verified": false, "message": error.message }));
                    continue;
                }
            };
            let column = match u32_field(&breakpoint, "column")
                .map(|column| self.client_column(column))
                .transpose()
            {
                Ok(column) => column,
                Err(error) => {
                    projected.push(json!({ "verified": false, "message": error.message }));
                    continue;
                }
            };
            let mut spec = PauseSpec::source(DebugSourceSelector {
                source_uri: source_uri.clone(),
                line,
                column,
                end_line: None,
                end_column: None,
                byte_range: None,
                scope: None,
            });
            spec.condition = breakpoint
                .get("condition")
                .and_then(Value::as_str)
                .map(str::to_owned);
            spec.hit_condition = breakpoint
                .get("hitCondition")
                .and_then(Value::as_str)
                .map(str::to_owned);
            match self.operation()?.pause(spec) {
                Ok(trigger) => {
                    let id = trigger.breakpoint_id();
                    let dap_id = match dap_i32_id(id.get(), "breakpoint") {
                        Ok(id) => id,
                        Err(error) => {
                            let _ = self.operation()?.remove_pause_trigger(id);
                            projected.push(json!({
                                "verified": false,
                                "message": error.message,
                            }));
                            continue;
                        }
                    };
                    installed.push(id);
                    let location = self
                        .operation()?
                        .debug_executable_locations(&source_uri, Some(line), Some(line))?
                        .into_iter()
                        .find(|point| {
                            point
                                .location
                                .as_ref()
                                .is_some_and(|at| at.column == column || column.is_none())
                        })
                        .and_then(|point| point.location);
                    let mut response = json!({
                        "id": dap_id,
                        "verified": true,
                        "source": self.project_source(&source_uri),
                        "line": self.dap_line(location.as_ref().map(|at| at.line).unwrap_or(line)),
                    });
                    insert_optional(
                        &mut response,
                        "column",
                        location
                            .as_ref()
                            .and_then(|at| at.column)
                            .map(|column| json!(self.dap_column(column))),
                    );
                    projected.push(response);
                }
                Err(error) => projected.push(json!({
                    "verified": false,
                    "message": error.to_string(),
                })),
            }
        }
        self.breakpoints_by_source.insert(source_uri, installed);
        Ok(json!({ "breakpoints": projected }))
    }

    fn breakpoint_locations(&mut self, arguments: &Value) -> Result<Value, DapAdapterError> {
        let source_uri = self.source_uri(arguments)?;
        let start = Some(
            self.client_line(u32_field(arguments, "line").ok_or_else(|| {
                DapAdapterError::new("cem.dap.argument_required", "line is required")
            })?)?,
        );
        let end = u32_field(arguments, "endLine")
            .map(|line| self.client_line(line))
            .transpose()?
            .or(start);
        let start_column = u32_field(arguments, "column")
            .map(|column| self.client_column(column))
            .transpose()?;
        let end_column = u32_field(arguments, "endColumn")
            .map(|column| self.client_column(column))
            .transpose()?;
        let breakpoints = self
            .operation()?
            .debug_executable_locations(&source_uri, start, end)?
            .into_iter()
            .filter_map(|point| point.location)
            .filter(|location| {
                let after_start = start_column.is_none_or(|column| {
                    Some(location.line) != start || location.column.is_some_and(|at| at >= column)
                });
                let before_end = end_column.is_none_or(|column| {
                    Some(location.line) != end || location.column.is_some_and(|at| at <= column)
                });
                after_start && before_end
            })
            .map(|location| self.project_breakpoint_location(&location))
            .collect::<Vec<_>>();
        Ok(json!({ "breakpoints": breakpoints }))
    }

    fn threads(&self) -> Result<Value, DapAdapterError> {
        let operation = self.operation()?;
        let tree = operation.execution_scope_tree();
        let tasks = if let Some(stop) = self.current_stop {
            operation
                .suspended_snapshot(stop)?
                .threads
                .items
                .iter()
                .map(|thread| (thread.task, thread.owner))
                .collect::<Vec<_>>()
        } else {
            tree.tasks()
                .filter(|task| !task.completed)
                .map(|task| (task.id, task.owner))
                .collect::<Vec<_>>()
        };
        let threads = tasks
            .into_iter()
            .map(|(task, owner_scope)| {
                let owner = tree
                    .scope(owner_scope)
                    .map(|scope| scope.label.as_str())
                    .unwrap_or("task");
                Ok(json!({
                    "id": dap_i32_id(task.get(), "thread")?,
                    "name": format!("{owner} [{task}]")
                }))
            })
            .collect::<Result<Vec<_>, DapAdapterError>>()?;
        Ok(json!({ "threads": threads }))
    }

    fn stack_trace(&self, arguments: &Value) -> Result<Value, DapAdapterError> {
        let task = TaskId::from_raw(required_dap_id(arguments, "threadId")?);
        let start = u32_field(arguments, "startFrame").unwrap_or(0);
        let levels = u32_field(arguments, "levels");
        let page = self
            .operation()?
            .debug_stack_trace(self.stop()?, task, start, levels)?;
        let frames = page
            .items
            .into_iter()
            .map(|frame| {
                let mut projected = json!({
                    "id": dap_i32_id(frame.reference.get(), "stack frame")?,
                    "name": frame.name,
                    "line": frame.location.as_ref().map(|at| self.dap_line(at.line)).unwrap_or(0),
                    "column": frame.location.as_ref().and_then(|at| at.column).map(|column| self.dap_column(column)).unwrap_or(0),
                });
                insert_optional(
                    &mut projected,
                        "source",
                        frame
                            .location
                            .as_ref()
                            .map(|at| self.project_source(&at.source_uri)),
                );
                insert_optional(
                    &mut projected,
                    "endLine",
                    frame
                        .location
                        .as_ref()
                        .and_then(|at| at.end_line)
                        .map(|line| json!(self.dap_line(line))),
                );
                insert_optional(
                    &mut projected,
                    "endColumn",
                    frame
                        .location
                        .as_ref()
                        .and_then(|at| at.end_column)
                        .map(|column| json!(self.dap_column(column))),
                );
                Ok(projected)
            })
            .collect::<Result<Vec<_>, DapAdapterError>>()?;
        Ok(json!({ "stackFrames": frames, "totalFrames": page.total }))
    }

    fn scopes(&self, arguments: &Value) -> Result<Value, DapAdapterError> {
        let frame = crate::debug_control::SnapshotReferenceId::from_raw(required_dap_id(
            arguments, "frameId",
        )?);
        let scopes = self
            .operation()?
            .debug_frame_scopes(self.stop()?, frame)?
            .into_iter()
            .map(|scope| {
                Ok(json!({
                    "name": scope.name,
                    "variablesReference": dap_i32_id(scope.variables_reference.get(), "variables reference")?,
                    "namedVariables": scope.named_variables,
                    "expensive": scope.expensive,
                }))
            })
            .collect::<Result<Vec<_>, DapAdapterError>>()?;
        Ok(json!({ "scopes": scopes }))
    }

    fn variables(&self, arguments: &Value) -> Result<Value, DapAdapterError> {
        let reference = crate::debug_control::SnapshotReferenceId::from_raw(required_dap_id(
            arguments,
            "variablesReference",
        )?);
        let start = u32_field(arguments, "start").unwrap_or(0);
        let count = u32_field(arguments, "count");
        let filter = match arguments.get("filter").and_then(Value::as_str) {
            Some("indexed") => VariableFilter::Indexed,
            _ => VariableFilter::Named,
        };
        let page =
            self.operation()?
                .debug_variables(self.stop()?, reference, filter, start, count)?;
        let variables = page
            .items
            .into_iter()
            .map(|variable| {
                let variables_reference = variable
                    .value
                    .variables_reference
                    .map(|id| dap_i32_id(id.get(), "variables reference"))
                    .transpose()?
                    .unwrap_or(0);
                Ok(json!({
                    "name": variable.name,
                    "value": variable.value.preview,
                    "type": variable.value.type_name,
                    "variablesReference": variables_reference,
                    "namedVariables": variable.value.named_variables,
                    "indexedVariables": variable.value.indexed_variables,
                }))
            })
            .collect::<Result<Vec<_>, DapAdapterError>>()?;
        Ok(json!({ "variables": variables }))
    }

    fn pause(&mut self, arguments: &Value) -> Result<Value, DapAdapterError> {
        let preferred_task = Some(TaskId::from_raw(required_dap_id(arguments, "threadId")?));
        let mut pause = PauseSpec::next_safe_point(None);
        pause.preferred_task = preferred_task;
        let trigger = self.operation()?.pause(pause)?;
        self.transient_breakpoints.insert(trigger.breakpoint_id());
        Ok(json!({}))
    }

    fn continue_operation(&mut self, arguments: &Value) -> Result<Value, DapAdapterError> {
        let task = TaskId::from_raw(required_dap_id(arguments, "threadId")?);
        if self
            .operation()?
            .execution_scope_tree()
            .task(task)
            .is_none()
        {
            return Err(DapAdapterError::new(
                "cem.dap.thread_unknown",
                format!("thread {task} is not part of the bound operation"),
            ));
        }
        let continued = self.operation()?.resume(self.stop()?)?;
        self.pending_continued_thread = Some(task);
        self.current_stop = None;
        Ok(json!({ "allThreadsContinued": continued.all_threads_continued }))
    }

    fn step(&mut self, arguments: &Value, mode: StepMode) -> Result<Value, DapAdapterError> {
        let task = TaskId::from_raw(required_dap_id(arguments, "threadId")?);
        self.operation()?.step(StepRequest {
            stop: self.stop()?,
            task,
            mode,
        })?;
        self.pending_continued_thread = Some(task);
        self.current_stop = None;
        Ok(json!({}))
    }

    fn cancel_root(&self, reason: &str) -> Result<(), DapAdapterError> {
        self.operation()?.cancel(CancelRequest {
            reason: Some(reason.to_owned()),
            ..CancelRequest::default()
        })?;
        Ok(())
    }

    fn detach(&mut self, terminate: bool) -> Result<(), DapAdapterError> {
        if self.operation.is_none() {
            return Ok(());
        }
        if terminate {
            self.cancel_root("debug session disconnected")?;
        } else {
            if let Some(stop) = self.current_stop.take() {
                let _ = self.operation()?.resume(stop);
            }
            let mut breakpoints = self
                .breakpoints_by_source
                .values()
                .flatten()
                .copied()
                .collect::<Vec<_>>();
            breakpoints.extend(self.transient_breakpoints.iter().copied());
            breakpoints.sort();
            breakpoints.dedup();
            for breakpoint in breakpoints {
                let _ = self.operation()?.remove_pause_trigger(breakpoint);
            }
        }
        if let Some(events) = self.events.as_mut() {
            events.close();
        }
        self.current_thread = None;
        self.pending_continued_thread = None;
        self.client_paths_by_uri.clear();
        self.breakpoints_by_source.clear();
        self.transient_breakpoints.clear();
        Ok(())
    }

    fn cem_operation(&self, arguments: &Value) -> Result<Value, DapAdapterError> {
        require_custom_version(arguments)?;
        let operation = self.operation()?;
        Ok(json!({
            "version": CEM_DEBUG_REQUEST_VERSION,
            "operationId": operation.operation_id().get(),
            "rootScope": operation.execution_scope_tree().root().get(),
            "debugControlActive": operation.debug_control_active(),
            "stopped": self.current_stop,
            "terminal": operation.terminal_summary(),
        }))
    }

    fn cem_execution_scopes(&self, arguments: &Value) -> Result<Value, DapAdapterError> {
        require_custom_version(arguments)?;
        Ok(json!({
            "version": CEM_DEBUG_REQUEST_VERSION,
            "tree": self.operation()?.execution_scope_tree(),
        }))
    }

    fn cem_cancel(&self, arguments: &Value) -> Result<Value, DapAdapterError> {
        require_custom_version(arguments)?;
        let scope = arguments
            .get("scope")
            .and_then(Value::as_u64)
            .map(ExecutionScopeId::from_raw);
        let source_selector = arguments
            .get("source")
            .map(|source| OperationSourceSelector {
                source_uri: source
                    .get("sourceUri")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                line: u32_field(source, "line").unwrap_or(0),
                column: u32_field(source, "column"),
            });
        let acknowledgement = self.operation()?.cancel(CancelRequest {
            scope,
            reason: arguments
                .get("reason")
                .and_then(Value::as_str)
                .map(str::to_owned),
            source_selector,
        })?;
        Ok(json!({
            "version": CEM_DEBUG_REQUEST_VERSION,
            "acknowledgement": acknowledgement,
        }))
    }

    fn cem_native_value(&self, arguments: &Value) -> Result<Value, DapAdapterError> {
        require_custom_version(arguments)?;
        let reference = crate::debug_control::SnapshotReferenceId::from_raw(required_dap_id(
            arguments,
            "variablesReference",
        )?);
        let value = self
            .operation()?
            .debug_native_projection(self.stop()?, reference)?;
        Ok(json!({ "version": CEM_DEBUG_REQUEST_VERSION, "value": value }))
    }

    fn cem_worker_topology(&self, arguments: &Value) -> Result<Value, DapAdapterError> {
        require_custom_version(arguments)?;
        let operation = self.operation()?;
        let tree = operation.execution_scope_tree();
        let (logical_tasks, physical_workers) = if let Some(stop) = self.current_stop {
            let snapshot = operation.suspended_snapshot(stop)?;
            let workers = snapshot
                .threads
                .items
                .iter()
                .filter_map(|thread| thread.physical_worker)
                .collect::<BTreeSet<_>>();
            (snapshot.threads.original_count, workers)
        } else {
            (tree.tasks().len() as u64, BTreeSet::new())
        };
        Ok(json!({
            "version": CEM_DEBUG_REQUEST_VERSION,
            "topology": "native-thread-pool",
            "logicalTaskCount": logical_tasks,
            "effectiveMaxWorkers": tree.scope(tree.root()).map(|root| root.effective_policy.cpu_workers).unwrap_or(1),
            "observedPhysicalWorkers": physical_workers,
        }))
    }

    fn source_uri(&mut self, arguments: &Value) -> Result<String, DapAdapterError> {
        let path = arguments
            .get("source")
            .and_then(|source| source.get("path"))
            .and_then(Value::as_str)
            .filter(|path| !path.is_empty())
            .ok_or_else(|| {
                DapAdapterError::new("cem.dap.source_required", "source.path is required")
            })?;
        let uri = match self.path_format {
            DapPathFormat::Path => dap_path_to_uri(path)?,
            DapPathFormat::Uri => {
                if !crate::resolver::has_uri_scheme(path) {
                    return Err(DapAdapterError::new(
                        "cem.dap.source_uri_invalid",
                        "source.path must be an absolute URI when pathFormat is `uri`",
                    ));
                }
                path.to_owned()
            }
        };
        self.client_paths_by_uri
            .insert(uri.clone(), path.to_owned());
        Ok(uri)
    }

    fn client_source_path(&self, uri: &str) -> Option<String> {
        if let Some(path) = self.client_paths_by_uri.get(uri) {
            return Some(path.clone());
        }
        match self.path_format {
            DapPathFormat::Uri => Some(uri.to_owned()),
            DapPathFormat::Path => dap_uri_to_path(uri),
        }
    }

    fn project_source(&self, uri: &str) -> Value {
        let path = self.client_source_path(uri);
        let name = path
            .as_deref()
            .unwrap_or(uri)
            .rsplit(['/', '\\'])
            .find(|part| !part.is_empty())
            .unwrap_or(uri)
            .to_owned();
        let mut source = json!({ "name": name });
        insert_optional(&mut source, "path", path.map(Value::String));
        source
    }

    fn client_line(&self, line: u32) -> Result<u32, DapAdapterError> {
        client_position(line, self.lines_start_at_one, "line")
    }

    fn client_column(&self, column: u32) -> Result<u32, DapAdapterError> {
        client_position(column, self.columns_start_at_one, "column")
    }

    fn dap_line(&self, line: u32) -> u32 {
        dap_position(line, self.lines_start_at_one)
    }

    fn dap_column(&self, column: u32) -> u32 {
        dap_position(column, self.columns_start_at_one)
    }

    fn project_breakpoint_resolution(
        &self,
        resolution: BreakpointResolution,
    ) -> Result<Value, DapAdapterError> {
        let location = resolution
            .location
            .as_ref()
            .and_then(|point| point.location.as_ref());
        let mut projected = json!({
            "id": dap_i32_id(resolution.breakpoint_id.get(), "breakpoint")?,
            "verified": resolution.verified,
        });
        insert_optional(
            &mut projected,
            "message",
            resolution.error_code.map(Value::String),
        );
        insert_optional(
            &mut projected,
            "source",
            location.map(|at| self.project_source(&at.source_uri)),
        );
        insert_optional(
            &mut projected,
            "line",
            location.map(|at| json!(self.dap_line(at.line))),
        );
        insert_optional(
            &mut projected,
            "column",
            location
                .and_then(|at| at.column)
                .map(|column| json!(self.dap_column(column))),
        );
        insert_optional(
            &mut projected,
            "endLine",
            location
                .and_then(|at| at.end_line)
                .map(|line| json!(self.dap_line(line))),
        );
        insert_optional(
            &mut projected,
            "endColumn",
            location
                .and_then(|at| at.end_column)
                .map(|column| json!(self.dap_column(column))),
        );
        Ok(projected)
    }

    fn project_breakpoint_location(
        &self,
        location: &crate::operation_control::SourceLocation,
    ) -> Value {
        let mut projected = json!({ "line": self.dap_line(location.line) });
        insert_optional(
            &mut projected,
            "column",
            location.column.map(|column| json!(self.dap_column(column))),
        );
        insert_optional(
            &mut projected,
            "endLine",
            location.end_line.map(|line| json!(self.dap_line(line))),
        );
        insert_optional(
            &mut projected,
            "endColumn",
            location
                .end_column
                .map(|column| json!(self.dap_column(column))),
        );
        projected
    }

    fn success_response(&mut self, request: &DapRequest, body: Value) -> Value {
        json!({
            "seq": self.sequence(),
            "type": "response",
            "request_seq": request.seq,
            "success": true,
            "command": request.command,
            "body": body,
        })
    }

    fn error_response(&mut self, request: &DapRequest, error: DapAdapterError) -> Value {
        let formatted = format!("{}: {}", error.code, error.message);
        json!({
            "seq": self.sequence(),
            "type": "response",
            "request_seq": request.seq,
            "success": false,
            "command": request.command,
            "message": error.message,
            "body": { "error": { "id": dap_error_id(error.code), "format": formatted } },
        })
    }

    fn event(&mut self, event: &str, body: Value) -> Value {
        json!({
            "seq": self.sequence(),
            "type": "event",
            "event": event,
            "body": body,
        })
    }

    fn sequence(&mut self) -> i32 {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        sequence
    }
}

fn dap_stop_reason(reason: StopReason) -> &'static str {
    match reason {
        StopReason::Breakpoint => "breakpoint",
        StopReason::Pause => "pause",
        StopReason::Step => "step",
        StopReason::ControlFailure => "exception",
    }
}

fn insert_optional(target: &mut Value, field: &str, value: Option<Value>) {
    if let (Some(object), Some(value)) = (target.as_object_mut(), value) {
        object.insert(field.to_owned(), value);
    }
}

fn dap_i32_id(value: u64, label: &'static str) -> Result<i32, DapAdapterError> {
    i32::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            DapAdapterError::new(
                "cem.dap.identifier_out_of_range",
                format!("{label} identifier is outside the positive DAP int32 range"),
            )
        })
}

fn dap_error_id(code: &str) -> i32 {
    let hash = code.bytes().fold(2_166_136_261_u32, |hash, byte| {
        hash.wrapping_mul(16_777_619) ^ u32::from(byte)
    });
    i32::try_from(hash & i32::MAX as u32).unwrap_or(1).max(1)
}

fn client_position(
    position: u32,
    starts_at_one: bool,
    label: &'static str,
) -> Result<u32, DapAdapterError> {
    if starts_at_one {
        if position == 0 {
            return Err(DapAdapterError::new(
                "cem.dap.source_position_invalid",
                format!("{label} must be positive for a one-based DAP client"),
            ));
        }
        Ok(position)
    } else {
        position.checked_add(1).ok_or_else(|| {
            DapAdapterError::new(
                "cem.dap.source_position_invalid",
                format!("{label} exceeds the supported source range"),
            )
        })
    }
}

fn dap_position(position: u32, starts_at_one: bool) -> u32 {
    if starts_at_one {
        position
    } else {
        position.saturating_sub(1)
    }
}

fn dap_path_to_uri(path: &str) -> Result<String, DapAdapterError> {
    let normalized = path.replace('\\', "/");
    let prefix = if normalized.starts_with('/') {
        "file://"
    } else if crate::resolver::is_windows_drive_path(&normalized) {
        "file:///"
    } else {
        return Err(DapAdapterError::new(
            "cem.dap.source_path_invalid",
            "source.path must be absolute when pathFormat is `path`",
        ));
    };
    Ok(format!("{prefix}{}", percent_encode_uri_path(&normalized)))
}

fn dap_uri_to_path(uri: &str) -> Option<String> {
    let path = crate::resolver::local_file_uri_to_path(uri)?;
    let path = path.to_string_lossy().into_owned();
    #[cfg(target_os = "windows")]
    {
        let path = path.strip_prefix('/').unwrap_or(&path).replace('/', "\\");
        return Some(path);
    }
    #[cfg(not(target_os = "windows"))]
    Some(path)
}

fn percent_encode_uri_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
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

fn u32_field(value: &Value, field: &str) -> Option<u32> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

fn required_u64(value: &Value, field: &'static str) -> Result<u64, DapAdapterError> {
    value.get(field).and_then(Value::as_u64).ok_or_else(|| {
        DapAdapterError::new("cem.dap.argument_required", format!("{field} is required"))
    })
}

fn required_dap_id(value: &Value, field: &'static str) -> Result<u64, DapAdapterError> {
    let id = required_u64(value, field)?;
    dap_i32_id(id, field)?;
    Ok(id)
}

fn require_custom_version(arguments: &Value) -> Result<(), DapAdapterError> {
    let requested = arguments.get("version").and_then(Value::as_u64);
    if requested != Some(u64::from(CEM_DEBUG_REQUEST_VERSION)) {
        return Err(DapAdapterError::new(
            "cem.dap.custom_request_version",
            format!(
                "custom request version must be {CEM_DEBUG_REQUEST_VERSION}, received {requested:?}"
            ),
        ));
    }
    Ok(())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::thread;
    use std::time::{Duration, Instant};

    use crate::debug_control::{
        DebugSafePointCapture, DebugSafePointKind, DebugValueCapture, DebugVariableCapture,
        DebugVariableScopeCapture, LogicalFrameCapture,
    };
    use crate::operation_control::{
        ExecutionScopeKind, ExecutionScopeRegistration, OperationControl, SourceLocation,
    };
    use crate::operation_handle::{ArtifactDisposition, OperationOutcome};
    use crate::source::line_index::LineIndex;

    #[derive(Clone)]
    struct FixtureHost {
        operation: OperationHandle<()>,
        configured: bool,
    }

    impl DapOperationHost<()> for FixtureHost {
        fn launch(&mut self, _: &Value) -> Result<OperationHandle<()>, DapAdapterError> {
            Ok(self.operation.clone())
        }

        fn attach(&mut self, _: &Value) -> Result<OperationHandle<()>, DapAdapterError> {
            Ok(self.operation.clone())
        }

        fn configuration_done(&mut self) -> Result<(), DapAdapterError> {
            self.configured = true;
            Ok(())
        }
    }

    fn request(seq: i32, command: &str, arguments: Value) -> DapRequest {
        DapRequest {
            seq,
            message_type: "request".to_owned(),
            command: command.to_owned(),
            arguments,
        }
    }

    fn response_body(messages: &[Value]) -> &Value {
        assert_eq!(messages[0]["type"], "response");
        assert_eq!(messages[0]["success"], true, "{messages:?}");
        &messages[0]["body"]
    }

    fn initialized_session(operation: OperationHandle<()>) -> (DapSession<()>, FixtureHost) {
        let mut session = DapSession::new(true);
        let mut host = FixtureHost {
            operation,
            configured: false,
        };
        let initialized = session.handle_request(
            request(1, "initialize", json!({ "pathFormat": "uri" })),
            &mut host,
        );
        let capabilities = response_body(&initialized);
        assert_eq!(capabilities["supportsTerminateRequest"], true);
        assert_eq!(capabilities["supportTerminateDebuggee"], true);
        assert_eq!(capabilities["supportsBreakpointLocationsRequest"], true);
        assert_eq!(capabilities["supportsConditionalBreakpoints"], false);
        assert!(capabilities.get("supportsTerminateDebuggee").is_none());
        assert!(capabilities.get("supportSuspendDebuggee").is_none());
        assert!(capabilities.get("supportsSuspendDebuggee").is_none());
        assert!(capabilities.get("supportsSetVariable").is_none());
        assert_eq!(initialized.len(), 1);
        let launched = session.handle_request(
            request(2, "launch", json!({ "command": "fixture", "args": [] })),
            &mut host,
        );
        response_body(&launched);
        assert_eq!(launched[1]["event"], "initialized");
        (session, host)
    }

    #[test]
    fn standard_and_versioned_requests_project_one_bounded_stop_generation() {
        let control = OperationControl::default();
        let (operation, terminal) = OperationHandle::with_defaults(control.clone()).unwrap();
        operation.activate_debug_control(None).unwrap();
        let root = control.root_scope();
        let document = control
            .register_scope(
                root,
                ExecutionScopeRegistration::inherited(
                    ExecutionScopeKind::Document,
                    "fixture.cem",
                    control.scope_tree().scope(root).unwrap().effective_policy,
                ),
            )
            .unwrap();
        let task = control.register_task(document).unwrap();
        control.debug_task_started(task, Some(7)).unwrap();
        let non_bmp_source = "a😀<item/>";
        let dap_column = LineIndex::from_utf8(non_bmp_source)
            .project_host("a😀".len() as u64)
            .column;
        assert_eq!(dap_column, 4);
        let location = SourceLocation {
            source_uri: "file:///fixture.cem".to_owned(),
            line: 4,
            column: Some(dap_column),
            end_line: Some(4),
            end_column: Some(dap_column + 7),
            byte_range: None,
        };
        operation
            .register_debug_safe_point(
                document,
                DebugSafePointKind::Visible,
                "evaluate",
                Some(location.clone()),
            )
            .unwrap();

        let (mut session, mut host) = initialized_session(operation.clone());
        let locations = session.handle_request(
            request(
                3,
                "breakpointLocations",
                json!({
                    "source": { "path": "file:///fixture.cem" },
                    "line": 1,
                    "endLine": 10,
                }),
            ),
            &mut host,
        );
        assert_eq!(response_body(&locations)["breakpoints"][0]["line"], 4);
        assert_eq!(
            response_body(&locations)["breakpoints"][0]["column"],
            dap_column
        );
        let breakpoints = session.handle_request(
            request(
                4,
                "setBreakpoints",
                json!({
                    "source": { "path": "file:///fixture.cem" },
                    "breakpoints": [{ "line": 4, "column": dap_column }],
                }),
            ),
            &mut host,
        );
        assert_eq!(
            response_body(&breakpoints)["breakpoints"][0]["verified"],
            true
        );
        let first_breakpoint = response_body(&breakpoints)["breakpoints"][0]["id"]
            .as_u64()
            .unwrap();
        let resolved = session
            .poll_events()
            .into_iter()
            .find(|event| event["event"] == "breakpoint")
            .expect("breakpoint resolution event");
        assert_eq!(resolved["body"]["breakpoint"]["id"], first_breakpoint);
        assert_eq!(resolved["body"]["breakpoint"]["verified"], true);
        assert_eq!(
            resolved["body"]["breakpoint"]["source"]["path"],
            "file:///fixture.cem"
        );
        let replacement = session.handle_request(
            request(
                5,
                "setBreakpoints",
                json!({
                    "source": { "path": "file:///fixture.cem" },
                    "breakpoints": [{ "line": 4, "column": dap_column }],
                }),
            ),
            &mut host,
        );
        let replacement_breakpoint = response_body(&replacement)["breakpoints"][0]["id"]
            .as_u64()
            .unwrap();
        assert_ne!(replacement_breakpoint, first_breakpoint);
        let configured =
            session.handle_request(request(6, "configurationDone", json!({})), &mut host);
        response_body(&configured);
        assert!(host.configured);

        let worker_control = control.clone();
        let worker_location = location.clone();
        let worker = thread::spawn(move || {
            let value = DebugValueCapture::projected_native(
                "cem-node",
                "<item/>",
                vec![1_u8, 2, 3],
                json!({ "kind": "element", "name": "item" }),
            )
            .unwrap();
            let first = worker_control.debug_safe_point(
                task,
                DebugSafePointCapture::visible(
                    "evaluate",
                    Some(worker_location.clone()),
                    vec![LogicalFrameCapture {
                        name: "template item".to_owned(),
                        phase: "evaluate".to_owned(),
                        location: Some(worker_location.clone()),
                        execution_scope: document,
                        variable_scopes: vec![DebugVariableScopeCapture {
                            name: "lexical".to_owned(),
                            expensive: false,
                            variables: vec![DebugVariableCapture {
                                name: "node".to_owned(),
                                declaration: None,
                                value,
                            }],
                        }],
                    }],
                ),
            )?;
            assert_eq!(first, crate::debug_control::DebugSafePointOutcome::Resumed);
            let next_location = SourceLocation {
                line: 5,
                end_line: Some(5),
                ..worker_location.clone()
            };
            worker_control.debug_safe_point(
                task,
                DebugSafePointCapture::visible(
                    "evaluate",
                    Some(next_location.clone()),
                    vec![LogicalFrameCapture {
                        name: "template item".to_owned(),
                        phase: "evaluate".to_owned(),
                        location: Some(next_location),
                        execution_scope: document,
                        variable_scopes: Vec::new(),
                    }],
                ),
            )
        });

        let deadline = Instant::now() + Duration::from_secs(2);
        let stopped = loop {
            let events = session.poll_events();
            if let Some(stopped) = events.into_iter().find(|event| event["event"] == "stopped") {
                break stopped;
            }
            assert!(Instant::now() < deadline, "DAP stopped event timed out");
            thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(stopped["body"]["allThreadsStopped"], true);
        assert_eq!(stopped["body"]["threadId"], task.get());

        let threads = session.handle_request(request(7, "threads", json!({})), &mut host);
        assert_eq!(response_body(&threads)["threads"][0]["id"], task.get());
        let stack = session.handle_request(
            request(
                8,
                "stackTrace",
                json!({ "threadId": task.get(), "startFrame": 0, "levels": 10 }),
            ),
            &mut host,
        );
        let frame = response_body(&stack)["stackFrames"][0]["id"]
            .as_u64()
            .unwrap();
        assert_eq!(
            response_body(&stack)["stackFrames"][0]["column"],
            dap_column
        );
        let scopes =
            session.handle_request(request(9, "scopes", json!({ "frameId": frame })), &mut host);
        let variables_reference = response_body(&scopes)["scopes"][0]["variablesReference"]
            .as_u64()
            .unwrap();
        let variables = session.handle_request(
            request(
                10,
                "variables",
                json!({ "variablesReference": variables_reference, "start": 0, "count": 10 }),
            ),
            &mut host,
        );
        let native_reference = response_body(&variables)["variables"][0]["variablesReference"]
            .as_u64()
            .unwrap();
        let native = session.handle_request(
            request(
                11,
                "cem/nativeValue",
                json!({ "version": 1, "variablesReference": native_reference }),
            ),
            &mut host,
        );
        assert_eq!(response_body(&native)["value"]["name"], "item");

        for (seq, command) in [
            (12, "cem/operation"),
            (13, "cem/executionScopes"),
            (14, "cem/workerTopology"),
        ] {
            let messages =
                session.handle_request(request(seq, command, json!({ "version": 1 })), &mut host);
            assert_eq!(response_body(&messages)["version"], 1);
        }

        let incompatible = session.handle_request(
            request(15, "cem/operation", json!({ "version": 2 })),
            &mut host,
        );
        assert_eq!(incompatible[0]["success"], false);
        assert!(incompatible[0]["body"]["error"]["id"]
            .as_i64()
            .is_some_and(|id| id > 0));
        assert!(incompatible[0]["body"]["error"]["format"]
            .as_str()
            .is_some_and(|message| message.contains("cem.dap.custom_request_version")));

        let protocol_cancel =
            session.handle_request(request(16, "cancel", json!({ "requestId": 14 })), &mut host);
        response_body(&protocol_cancel);
        assert!(!control.is_cancelled());

        let stepped = session.handle_request(
            request(17, "next", json!({ "threadId": task.get() })),
            &mut host,
        );
        response_body(&stepped);
        let deadline = Instant::now() + Duration::from_secs(2);
        let stepped_stop = loop {
            let events = session.poll_events();
            if let Some(stopped) = events.into_iter().find(|event| event["event"] == "stopped") {
                break stopped;
            }
            assert!(Instant::now() < deadline, "DAP step event timed out");
            thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(stepped_stop["body"]["reason"], "step");

        let continued = session.handle_request(
            request(18, "continue", json!({ "threadId": task.get() })),
            &mut host,
        );
        assert_eq!(response_body(&continued)["allThreadsContinued"], true);
        let continued_event = session
            .poll_events()
            .into_iter()
            .find(|event| event["event"] == "continued")
            .expect("continued event");
        assert_eq!(continued_event["body"]["threadId"], task.get());
        assert_eq!(continued_event["body"]["allThreadsContinued"], true);
        assert_eq!(
            worker.join().unwrap().unwrap(),
            crate::debug_control::DebugSafePointOutcome::Resumed
        );
        control.complete_task(task).unwrap();

        let cancelled = session.handle_request(
            request(
                19,
                "cem/cancel",
                json!({ "version": 1, "scope": document.get(), "reason": "fixture" }),
            ),
            &mut host,
        );
        assert_eq!(
            response_body(&cancelled)["acknowledgement"]["selectedScope"],
            document.get()
        );
        let _ = terminal.settle(OperationOutcome::cancelled(
            Some("fixture".to_owned()),
            Vec::new(),
            ArtifactDisposition::default(),
        ));
    }

    #[test]
    fn initialize_projects_zero_based_coordinates_and_native_source_paths() {
        let control = OperationControl::default();
        let (operation, _) = OperationHandle::with_defaults(control.clone()).unwrap();
        operation.activate_debug_control(None).unwrap();
        let location = SourceLocation {
            source_uri: "file:///tmp/cem%20fixture.cem".to_owned(),
            line: 4,
            column: Some(4),
            end_line: Some(4),
            end_column: Some(8),
            byte_range: None,
        };
        operation
            .register_debug_safe_point(
                control.root_scope(),
                DebugSafePointKind::Visible,
                "evaluate",
                Some(location),
            )
            .unwrap();

        let mut session = DapSession::new(true);
        let mut host = FixtureHost {
            operation: operation.clone(),
            configured: false,
        };
        let initialized = session.handle_request(
            request(
                1,
                "initialize",
                json!({
                    "linesStartAt1": false,
                    "columnsStartAt1": false,
                    "pathFormat": "path",
                }),
            ),
            &mut host,
        );
        response_body(&initialized);
        response_body(&session.handle_request(
            request(2, "launch", json!({ "command": "fixture" })),
            &mut host,
        ));

        let locations = session.handle_request(
            request(
                3,
                "breakpointLocations",
                json!({
                    "source": { "path": "/tmp/cem fixture.cem" },
                    "line": 0,
                    "endLine": 9,
                }),
            ),
            &mut host,
        );
        assert_eq!(response_body(&locations)["breakpoints"][0]["line"], 3);
        assert_eq!(response_body(&locations)["breakpoints"][0]["column"], 3);

        let breakpoints = session.handle_request(
            request(
                4,
                "setBreakpoints",
                json!({
                    "source": { "path": "/tmp/cem fixture.cem" },
                    "breakpoints": [{ "line": 3, "column": 3 }],
                }),
            ),
            &mut host,
        );
        assert_eq!(
            response_body(&breakpoints)["breakpoints"][0]["source"]["path"],
            "/tmp/cem fixture.cem"
        );
        assert_eq!(response_body(&breakpoints)["breakpoints"][0]["line"], 3);
        assert_eq!(response_body(&breakpoints)["breakpoints"][0]["column"], 3);
        response_body(
            &session.handle_request(request(5, "configurationDone", json!({})), &mut host),
        );
        assert!(host.configured);
    }

    #[test]
    fn pause_targets_a_logical_thread_and_terminate_cancels_the_root() {
        let control = OperationControl::default();
        let (operation, _) = OperationHandle::with_defaults(control.clone()).unwrap();
        operation.activate_debug_control(None).unwrap();
        let task = control.register_task(control.root_scope()).unwrap();
        control.debug_task_started(task, Some(3)).unwrap();
        let location = SourceLocation {
            source_uri: "file:///pause.cem".to_owned(),
            line: 2,
            column: Some(1),
            end_line: Some(2),
            end_column: Some(4),
            byte_range: None,
        };
        operation
            .register_debug_safe_point(
                control.root_scope(),
                DebugSafePointKind::Visible,
                "evaluate",
                Some(location.clone()),
            )
            .unwrap();
        let (mut session, mut host) = initialized_session(operation);
        let running_threads = session.handle_request(request(3, "threads", json!({})), &mut host);
        assert_eq!(
            response_body(&running_threads)["threads"][0]["id"],
            task.get()
        );
        let paused = session.handle_request(
            request(4, "pause", json!({ "threadId": task.get() })),
            &mut host,
        );
        response_body(&paused);

        let worker_control = control.clone();
        let worker = thread::spawn(move || {
            worker_control.debug_safe_point(
                task,
                DebugSafePointCapture::visible("evaluate", Some(location), Vec::new()),
            )
        });
        let deadline = Instant::now() + Duration::from_secs(2);
        let stopped = loop {
            let events = session.poll_events();
            if let Some(stopped) = events.into_iter().find(|event| event["event"] == "stopped") {
                break stopped;
            }
            assert!(Instant::now() < deadline, "DAP pause event timed out");
            thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(stopped["body"]["threadId"], task.get());
        let continued = session.handle_request(
            request(5, "continue", json!({ "threadId": task.get() })),
            &mut host,
        );
        response_body(&continued);
        worker.join().unwrap().unwrap();

        let terminated = session.handle_request(request(6, "terminate", json!({})), &mut host);
        response_body(&terminated);
        assert!(control.is_cancelled());
    }

    #[test]
    fn disconnect_and_transport_loss_apply_launch_attach_ownership_rules() {
        let operation = |control: &OperationControl| {
            let (operation, _) = OperationHandle::with_defaults(control.clone()).unwrap();
            operation.activate_debug_control(None).unwrap();
            operation
        };

        let leave_control = OperationControl::default();
        let (mut leave, mut host) = initialized_session(operation(&leave_control));
        let disconnected = leave.handle_request(
            request(3, "disconnect", json!({ "terminateDebuggee": false })),
            &mut host,
        );
        response_body(&disconnected);
        assert!(!leave_control.is_cancelled());

        let launch_control = OperationControl::default();
        let (mut launched, _) = initialized_session(operation(&launch_control));
        launched.transport_lost();
        assert!(launch_control.is_cancelled());

        let attach_control = OperationControl::default();
        let attach_operation = operation(&attach_control);
        let mut attach_host = FixtureHost {
            operation: attach_operation,
            configured: false,
        };
        let mut attached = DapSession::new(true);
        attached.handle_request(request(1, "initialize", json!({})), &mut attach_host);
        let response = attached.handle_request(
            request(2, "attach", json!({ "operationId": 1 })),
            &mut attach_host,
        );
        response_body(&response);
        attached.transport_lost();
        assert!(!attach_control.is_cancelled());
    }
}
