//! WASM-callable observer surface (AC-O-1 / AC-C-1).
//!
//! Exposes the named event channels — `onParseEvent`, `onValidate`,
//! `onTransform` — to JS callers through `wasm-bindgen`. Each
//! registration replaces the previously-installed callback for that
//! channel; the matching `off*` function clears it. Callbacks receive
//! the event as a JSON string (the canonical wire form documented in
//! `cem-ml-stack-design-impl.md` §3.12.1 and modelled by
//! `packages/cem_ml/schema/observability/report-event.schema.json`).
//! Resolver callbacks follow the same registration pattern through
//! `onResolveRead` and `onResolveWrite`; [`JsResourceResolver`] adapts
//! those callbacks to the shared `ResourceResolver` trait for Rust-side
//! WASM entrypoints.
//!
//! ```js
//! import init, { onParseEvent, offParseEvent } from "@epa-wg/cem-ml/wasm";
//!
//! await init();
//! onParseEvent((json) => {
//!   const event = JSON.parse(json);
//!   console.log(event.channel, event.sequence, event.byteOffset);
//! });
//! onResolveRead((json) => {
//!   const request = JSON.parse(json);
//!   return { uri: request.uri, bytes: new TextEncoder().encode("<cem />") };
//! });
//! // later:
//! offParseEvent();
//! offResolveRead();
//! ```
//!
//! The same adapter also exposes the bounded legacy custom-element
//! HTML+XSLT lowering path as JSON so JS hosts can call the CEM-owned
//! converter before routing the canonical CEM-ML through the render engine.
//!
//! [`JsObserver`] is the `EngineObserver` adapter that dispatches each
//! event through whichever JS callback is currently registered.
//! Embedders running the pipeline from Rust-side WASM code pass it as
//! the observer to `observe_pipeline`.

use std::cell::RefCell;
use std::collections::BTreeMap;

use js_sys::{Array, ArrayBuffer, Function, Reflect, Uint8Array};
use wasm_bindgen::{prelude::*, JsCast};

use crate::capability::{
    capability_manifest, CapabilityAvailability, CapabilityManifest, CapabilityRequest,
    ControlCapabilityKind, ControlCoverage, ExecutorTopology, RuntimeKind,
};
use crate::observability::{EngineObserver, ReportEvent};
use crate::operation_control::OperationId;
use crate::operation_handle::OPERATION_PROTOCOL_VERSION;
use crate::resolver::{
    ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedRead, ResolvedWrite,
    ResolverDiagnostic, ResolverRegistry, ResourceResolver,
};
use crate::resumable_operation::{
    execute_operation_work, ResumableOperationError, ResumableOperationHost, ResumableRunRequest,
};
use crate::worker_control::{
    OperationWorkPacket, OperationWorkResult, WorkerCoordinatorLimits, WorkerSlotId,
    WORKER_PROTOCOL_VERSION,
};

mod command_service;
#[cfg(feature = "debug-control")]
use crate::worker_control::{
    WorkerAddress, WorkerGeneration, WorkerStopDisposition, WorkerStopGeneration,
};
pub use command_service::{
    cancel_command_service_v1, dispose_command_artifact_v1, dispose_command_artifacts_v1,
    execute_command_service_v1, read_command_artifact_v1,
};

thread_local! {
    static PARSE_OBSERVER: RefCell<Option<Function>> = const { RefCell::new(None) };
    static VALIDATE_OBSERVER: RefCell<Option<Function>> = const { RefCell::new(None) };
    static TRANSFORM_OBSERVER: RefCell<Option<Function>> = const { RefCell::new(None) };
    static RESOLVER_READ: RefCell<Option<Function>> = const { RefCell::new(None) };
    static RESOLVER_WRITE: RefCell<Option<Function>> = const { RefCell::new(None) };
    static RESUMABLE_OPERATION_HOSTS: RefCell<BTreeMap<u32, ResumableOperationHost>> = const { RefCell::new(BTreeMap::new()) };
    static NEXT_RESUMABLE_OPERATION_HOST_ID: RefCell<u32> = const { RefCell::new(1) };
}

/// Returns the common `cem_ml` Cargo version embedded in this WASM build.
#[wasm_bindgen(js_name = "version")]
pub fn version() -> String {
    crate::VERSION.to_owned()
}

/// Projects the common capability contract for a host-provided runtime
/// request. Both the browser and Node npm loaders call this same export so
/// deployment metadata cannot drift from the engine's Rust-owned semantics.
#[wasm_bindgen(js_name = "capabilityManifest")]
pub fn capability_manifest_json(request_json: &str) -> String {
    let request = match parse_capability_request(request_json) {
        Ok(request) => request,
        Err(error) => return error,
    };

    match capability_manifest(request) {
        Ok(manifest) => serde_json::to_string(&manifest).unwrap_or_else(capability_serialize_error),
        Err(error) => serde_json::json!({
            "error": {
                "code": error.code,
                "field": error.field,
                "message": error.message
            }
        })
        .to_string(),
    }
}

/// Projects a Node worker-pool capability through common Rust semantics. A
/// worker remains sequential internally; this host projection advertises the
/// bounded physical pool coordinating those isolated runtime instances.
#[wasm_bindgen(js_name = "nodeWorkerCapabilityManifest")]
pub fn node_worker_capability_manifest_json(
    request_json: &str,
    effective_max_workers: u32,
) -> String {
    let request = match parse_capability_request(request_json) {
        Ok(request) => request,
        Err(error) => return error,
    };
    if request.runtime != RuntimeKind::WasmNode {
        return capability_error(
            "cem.capability.runtime_mismatch",
            "runtime",
            "Node worker capability projection requires runtime `wasm-node`",
        );
    }
    let maximum = u32::from(WorkerCoordinatorLimits::default().max_workers);
    if effective_max_workers == 0 || effective_max_workers > maximum {
        return capability_error(
            "cem.capability.worker_count",
            "effectiveMaxWorkers",
            &format!("effective worker count {effective_max_workers} is outside 1..={maximum}"),
        );
    }

    match capability_manifest(request) {
        Ok(mut manifest) => {
            manifest.executor_topology = ExecutorTopology::NodeWorkerPool;
            manifest.effective_max_workers = effective_max_workers;
            enable_worker_hard_cancel(&mut manifest);
            serde_json::to_string(&manifest).unwrap_or_else(capability_serialize_error)
        }
        Err(error) => capability_error(error.code, error.field, &error.message),
    }
}

/// Projects a browser dedicated-worker pool capability through common Rust
/// semantics. Each worker remains sequential internally; the browser host
/// coordinates these isolated runtime instances through message passing.
#[wasm_bindgen(js_name = "browserWorkerCapabilityManifest")]
pub fn browser_worker_capability_manifest_json(
    request_json: &str,
    effective_max_workers: u32,
) -> String {
    let request = match parse_capability_request(request_json) {
        Ok(request) => request,
        Err(error) => return error,
    };
    if request.runtime != RuntimeKind::WasmBrowserWorker {
        return capability_error(
            "cem.capability.runtime_mismatch",
            "runtime",
            "browser worker capability projection requires runtime `wasm-browser-worker`",
        );
    }
    let maximum = u32::from(WorkerCoordinatorLimits::default().max_workers);
    if effective_max_workers == 0 || effective_max_workers > maximum {
        return capability_error(
            "cem.capability.worker_count",
            "effectiveMaxWorkers",
            &format!("effective worker count {effective_max_workers} is outside 1..={maximum}"),
        );
    }

    match capability_manifest(request) {
        Ok(mut manifest) => {
            manifest.executor_topology = ExecutorTopology::BrowserWorkerPool;
            manifest.effective_max_workers = effective_max_workers;
            enable_worker_hard_cancel(&mut manifest);
            serde_json::to_string(&manifest).unwrap_or_else(capability_serialize_error)
        }
        Err(error) => capability_error(error.code, error.field, &error.message),
    }
}

/// Describes the common worker/operation protocol versions and hard transfer
/// bounds consumed by Node worker threads and browser dedicated workers.
#[wasm_bindgen(js_name = "workerProtocolDescriptor")]
pub fn worker_protocol_descriptor_json() -> String {
    let limits = WorkerCoordinatorLimits::default();
    serde_json::json!({
        "workerProtocolVersion": WORKER_PROTOCOL_VERSION,
        "operationProtocolVersion": OPERATION_PROTOCOL_VERSION,
        "limits": {
            "maxWorkers": limits.max_workers,
            "maxTransferBuffersPerMessage": limits.max_transfer_buffers_per_message,
            "maxTransferBytesPerMessage": limits.max_transfer_bytes_per_message,
        }
    })
    .to_string()
}

/// Create one coordinator-owned resumable operation host for a physical pool.
/// The returned host identity scopes operation IDs and permits multiple pools
/// to coexist in the same JavaScript runtime without sharing routes.
#[wasm_bindgen(js_name = "initializeResumableOperationHost")]
pub fn initialize_resumable_operation_host(worker_count: u16) -> String {
    resumable_response((|| {
        let host = ResumableOperationHost::new(worker_count)?;
        let workers = host.workers().to_vec();
        let host_id = NEXT_RESUMABLE_OPERATION_HOST_ID.with(|cell| {
            let mut next = cell.borrow_mut();
            let host_id = *next;
            *next = next.checked_add(1).ok_or_else(|| {
                ResumableOperationError::new(
                    "cem.operation.host_identity_exhausted",
                    "resumable operation host identity space is exhausted",
                )
            })?;
            Ok::<u32, ResumableOperationError>(host_id)
        })?;
        RESUMABLE_OPERATION_HOSTS.with(|cell| cell.borrow_mut().insert(host_id, host));
        Ok(serde_json::json!({
            "hostId": host_id,
            "workers": workers,
        }))
    })())
}

#[wasm_bindgen(js_name = "disposeResumableOperationHost")]
pub fn dispose_resumable_operation_host(host_id: u32) -> String {
    let disposed = RESUMABLE_OPERATION_HOSTS.with(|cell| cell.borrow_mut().remove(&host_id));
    serde_json::json!({ "hostId": host_id, "disposed": disposed.is_some() }).to_string()
}

#[wasm_bindgen(js_name = "startResumableOperation")]
pub fn start_resumable_operation(host_id: u32, request_json: &str) -> String {
    resumable_response((|| {
        let request = serde_json::from_str::<ResumableRunRequest>(request_json)
            .map_err(resumable_deserialization_error)?;
        with_resumable_host_mut(host_id, |host| host.start(request))
    })())
}

#[wasm_bindgen(js_name = "pollResumableOperation")]
pub fn poll_resumable_operation(host_id: u32, operation_id: &str, max_packets: u32) -> String {
    resumable_response((|| {
        let operation = parse_resumable_operation_id(operation_id)?;
        with_resumable_host_mut(host_id, |host| host.poll(operation, max_packets))
    })())
}

#[wasm_bindgen(js_name = "acceptResumableOperationResult")]
pub fn accept_resumable_operation_result(host_id: u32, result_json: &str) -> String {
    resumable_response((|| {
        let result = serde_json::from_str::<OperationWorkResult>(result_json)
            .map_err(resumable_deserialization_error)?;
        with_resumable_host_mut(host_id, |host| host.accept_result(result))
    })())
}

#[wasm_bindgen(js_name = "cancelResumableOperation")]
pub fn cancel_resumable_operation(
    host_id: u32,
    operation_id: &str,
    reason: Option<String>,
) -> String {
    resumable_response((|| {
        let operation = parse_resumable_operation_id(operation_id)?;
        with_resumable_host_mut(host_id, |host| host.cancel(operation, reason))
    })())
}

#[wasm_bindgen(js_name = "replaceResumableOperationWorker")]
pub fn replace_resumable_operation_worker(host_id: u32, slot: u32) -> String {
    resumable_response((|| {
        if slot == 0 {
            return Err(ResumableOperationError::new(
                "cem.worker.slot_invalid",
                "worker slot must be non-zero",
            ));
        }
        with_resumable_host_mut(host_id, |host| {
            host.replace_worker(WorkerSlotId::from_raw(u64::from(slot)))
        })
    })())
}

/// Stateless helper-runtime entrypoint used by Node and dedicated workers.
#[wasm_bindgen(js_name = "executeOperationWork")]
pub fn execute_operation_work_json(packet_json: &str) -> String {
    resumable_response((|| {
        let packet = serde_json::from_str::<OperationWorkPacket>(packet_json)
            .map_err(resumable_deserialization_error)?;
        execute_operation_work(packet)
    })())
}

#[cfg(feature = "debug-control")]
#[wasm_bindgen(js_name = "pauseResumableOperation")]
pub fn pause_resumable_operation(host_id: u32, operation_id: &str, generation: u32) -> String {
    resumable_response((|| {
        let operation = parse_resumable_operation_id(operation_id)?;
        with_resumable_host_mut(host_id, |host| {
            host.pause(
                operation,
                WorkerStopGeneration::from_raw(u64::from(generation)),
            )
        })
    })())
}

#[cfg(feature = "debug-control")]
#[wasm_bindgen(js_name = "acknowledgeResumableOperationStop")]
pub fn acknowledge_resumable_operation_stop(
    host_id: u32,
    operation_id: &str,
    stop_generation: u32,
    worker_slot: u32,
    worker_generation: u32,
    external_wait: bool,
) -> String {
    resumable_response((|| {
        let operation = parse_resumable_operation_id(operation_id)?;
        let worker = WorkerAddress::new(
            WorkerSlotId::from_raw(u64::from(worker_slot)),
            WorkerGeneration::from_raw(u64::from(worker_generation)),
        );
        let disposition = if external_wait {
            WorkerStopDisposition::ExternalWait
        } else {
            WorkerStopDisposition::Parked
        };
        with_resumable_host_mut(host_id, |host| {
            host.acknowledge_stop(
                operation,
                WorkerStopGeneration::from_raw(u64::from(stop_generation)),
                worker,
                disposition,
            )
        })
    })())
}

#[cfg(feature = "debug-control")]
#[wasm_bindgen(js_name = "continueResumableOperation")]
pub fn continue_resumable_operation(
    host_id: u32,
    operation_id: &str,
    stop_generation: u32,
) -> String {
    resumable_response((|| {
        let operation = parse_resumable_operation_id(operation_id)?;
        with_resumable_host_mut(host_id, |host| {
            host.continue_operation(
                operation,
                WorkerStopGeneration::from_raw(u64::from(stop_generation)),
            )
        })
    })())
}

#[cfg(feature = "debug-control")]
#[wasm_bindgen(js_name = "stepResumableOperation")]
pub fn step_resumable_operation(
    host_id: u32,
    operation_id: &str,
    current_stop_generation: u32,
    next_stop_generation: u32,
) -> String {
    resumable_response((|| {
        let operation = parse_resumable_operation_id(operation_id)?;
        with_resumable_host_mut(host_id, |host| {
            host.step(
                operation,
                WorkerStopGeneration::from_raw(u64::from(current_stop_generation)),
                WorkerStopGeneration::from_raw(u64::from(next_stop_generation)),
            )
        })
    })())
}

fn with_resumable_host_mut<T>(
    host_id: u32,
    action: impl FnOnce(&mut ResumableOperationHost) -> Result<T, ResumableOperationError>,
) -> Result<T, ResumableOperationError> {
    RESUMABLE_OPERATION_HOSTS.with(|cell| {
        let mut hosts = cell.borrow_mut();
        let host = hosts.get_mut(&host_id).ok_or_else(|| {
            ResumableOperationError::new(
                "cem.operation.host_unknown",
                format!("unknown resumable operation host {host_id}"),
            )
        })?;
        action(host)
    })
}

fn parse_resumable_operation_id(value: &str) -> Result<OperationId, ResumableOperationError> {
    let raw = value.parse::<u64>().map_err(|_| {
        ResumableOperationError::new(
            "cem.operation.identity_invalid",
            format!("operation identity `{value}` is not a non-zero u64"),
        )
    })?;
    if raw == 0 {
        return Err(ResumableOperationError::new(
            "cem.operation.identity_invalid",
            "operation identity must be non-zero",
        ));
    }
    Ok(OperationId::from_raw(raw))
}

fn resumable_response<T: serde::Serialize>(result: Result<T, ResumableOperationError>) -> String {
    match result {
        Ok(value) => serde_json::to_string(&value).unwrap_or_else(|error| {
            serde_json::json!({
                "error": {
                    "code": "cem.operation.serialize",
                    "message": error.to_string(),
                }
            })
            .to_string()
        }),
        Err(error) => serde_json::json!({ "error": error }).to_string(),
    }
}

fn resumable_deserialization_error(error: serde_json::Error) -> ResumableOperationError {
    ResumableOperationError::new("cem.operation.deserialize", error.to_string())
}

fn enable_worker_hard_cancel(manifest: &mut CapabilityManifest) {
    let hard_cancel = manifest
        .controls
        .iter_mut()
        .find(|entry| entry.control == ControlCapabilityKind::HardCancel)
        .expect("common capability manifest includes hard-cancel control");
    hard_cancel.availability = CapabilityAvailability::Available;
    hard_cancel.coverage = ControlCoverage::WorkerTermination;
}

fn parse_capability_request(request_json: &str) -> Result<CapabilityRequest, String> {
    serde_json::from_str::<CapabilityRequest>(request_json).map_err(|error| {
        capability_error(
            "cem.capability.invalid_request",
            "request",
            &error.to_string(),
        )
    })
}

fn capability_error(code: &str, field: &str, message: &str) -> String {
    serde_json::json!({
        "error": {
            "code": code,
            "field": field,
            "message": message
        }
    })
    .to_string()
}

fn capability_serialize_error(error: serde_json::Error) -> String {
    serde_json::json!({
        "error": {
            "code": "cem.capability.serialize_failed",
            "message": error.to_string()
        }
    })
    .to_string()
}

#[wasm_bindgen(js_name = "onParseEvent")]
pub fn on_parse_event(callback: Function) {
    PARSE_OBSERVER.with(|cell| *cell.borrow_mut() = Some(callback));
}

#[wasm_bindgen(js_name = "offParseEvent")]
pub fn off_parse_event() {
    PARSE_OBSERVER.with(|cell| *cell.borrow_mut() = None);
}

#[wasm_bindgen(js_name = "onValidate")]
pub fn on_validate(callback: Function) {
    VALIDATE_OBSERVER.with(|cell| *cell.borrow_mut() = Some(callback));
}

#[wasm_bindgen(js_name = "offValidate")]
pub fn off_validate() {
    VALIDATE_OBSERVER.with(|cell| *cell.borrow_mut() = None);
}

#[wasm_bindgen(js_name = "onTransform")]
pub fn on_transform(callback: Function) {
    TRANSFORM_OBSERVER.with(|cell| *cell.borrow_mut() = Some(callback));
}

#[wasm_bindgen(js_name = "offTransform")]
pub fn off_transform() {
    TRANSFORM_OBSERVER.with(|cell| *cell.borrow_mut() = None);
}

#[wasm_bindgen(js_name = "onResolveRead")]
pub fn on_resolve_read(callback: Function) {
    RESOLVER_READ.with(|cell| *cell.borrow_mut() = Some(callback));
}

#[wasm_bindgen(js_name = "offResolveRead")]
pub fn off_resolve_read() {
    RESOLVER_READ.with(|cell| *cell.borrow_mut() = None);
}

#[wasm_bindgen(js_name = "onResolveWrite")]
pub fn on_resolve_write(callback: Function) {
    RESOLVER_WRITE.with(|cell| *cell.borrow_mut() = Some(callback));
}

#[wasm_bindgen(js_name = "offResolveWrite")]
pub fn off_resolve_write() {
    RESOLVER_WRITE.with(|cell| *cell.borrow_mut() = None);
}

#[wasm_bindgen(js_name = "convertLegacyCustomElementTemplate")]
pub fn convert_legacy_custom_element_template(source: &str) -> String {
    let result = crate::legacy_custom_element::convert_template_source(source);
    serde_json::to_string(&result).unwrap_or_else(|error| {
        serde_json::json!({
            "source": "",
            "diagnostics": [{
                "code": "legacy_xslt.wasm.serialize_failed",
                "message": format!("legacy conversion result could not be serialized: {error}")
            }]
        })
        .to_string()
    })
}

#[wasm_bindgen(js_name = "normalizeRunConfig")]
pub fn normalize_run_config(json: &str) -> String {
    let request = crate::run_config::RunConfigParseRequest {
        bytes: json.as_bytes().to_vec(),
        identity: crate::engine::FormatIdentity {
            content_type: Some("application/json".to_owned()),
            schema: Some(crate::run_config::RUN_CONFIG_SCHEMA_URI.to_owned()),
            ..crate::engine::FormatIdentity::default()
        },
        base_uri: None,
    };
    match crate::run_config::parse_run_config(request) {
        Ok(response) => {
            let response = crate::run_config::normalize_run_config(
                response.config,
                crate::run_config::RunConfigDefaults::default(),
                None,
            );
            serde_json::to_string(&response).unwrap_or_else(wasm_serialize_error)
        }
        Err(error) => serde_json::json!({
            "error": {
                "code": error.code,
                "message": error.to_string()
            }
        })
        .to_string(),
    }
}

/// Parse and normalize the exact run-plan request consumed by command-service
/// v1. Host adapters use this Rust-owned entrypoint instead of reproducing
/// input/output identity, resolver, scope, budget, or destination semantics.
#[wasm_bindgen(js_name = "normalizeCommandRunPlanV1")]
pub fn normalize_command_run_plan_v1(json: &str) -> String {
    let request = match serde_json::from_str::<crate::run_config::NormalizedRunPlanRequest>(json) {
        Ok(request) => request,
        Err(error) => {
            return serde_json::json!({
                "error": {
                    "code": "cem.command_service.run_plan_decode",
                    "message": error.to_string(),
                }
            })
            .to_string()
        }
    };
    match crate::run_config::parse_normalized_run_plan(request) {
        Ok(plan) => serde_json::to_string(&plan).unwrap_or_else(wasm_serialize_error),
        Err(error) => serde_json::json!({
            "error": {
                "code": error.code,
                "message": error.to_string(),
            }
        })
        .to_string(),
    }
}

#[wasm_bindgen(js_name = "parseInputSpecRecord")]
pub fn parse_input_spec_record(record: &str) -> String {
    match crate::run_config::parse_input_spec_record(record) {
        Ok(spec) => serde_json::to_string(&spec).unwrap_or_else(wasm_serialize_error),
        Err(error) => serde_json::json!({
            "error": {
                "code": "cem.run_config.invalid_input_spec",
                "message": error.to_string()
            }
        })
        .to_string(),
    }
}

#[wasm_bindgen(js_name = "parseOutputSpecRecord")]
pub fn parse_output_spec_record(record: &str) -> String {
    match crate::run_config::parse_output_spec_record(record) {
        Ok(spec) => serde_json::to_string(&spec).unwrap_or_else(wasm_serialize_error),
        Err(error) => serde_json::json!({
            "error": {
                "code": "cem.run_config.invalid_output_spec",
                "message": error.to_string()
            }
        })
        .to_string(),
    }
}

fn wasm_serialize_error(error: serde_json::Error) -> String {
    serde_json::json!({
        "error": {
            "code": "cem.run_config.serialize_failed",
            "message": error.to_string()
        }
    })
    .to_string()
}

/// `ResourceResolver` adapter that forwards read/write requests to
/// JavaScript callbacks registered with `onResolveRead` and
/// `onResolveWrite`.
///
/// Rust-side WASM entrypoints can install this resolver into an
/// `EngineContext` using [`resolver_registry_for_schemes`]. The JS read
/// callback receives a request JSON string and returns either a string
/// body or an object with `uri`, `bytes`, and optional `contentType`.
/// `bytes` may be a string, `Uint8Array`, `ArrayBuffer`, or numeric array.
/// The JS write callback receives request JSON plus a `Uint8Array` payload
/// and returns nothing, a URI string, or an object with an optional `uri`.
#[derive(Debug, Clone)]
pub struct JsResourceResolver;

impl ResourceResolver for JsResourceResolver {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        let Some(callback) = RESOLVER_READ.with(|cell| cell.borrow().clone()) else {
            return Err(unsupported(request));
        };
        let request_json = request_json(request)?;
        let response = callback
            .call1(&JsValue::NULL, &JsValue::from_str(&request_json))
            .map_err(|error| js_io(request, error_message(&error)))?;
        if response.is_null() || response.is_undefined() {
            return Err(unsupported(request));
        }
        read_response(request, response)
    }

    fn write(
        &self,
        request: &ResolveRequest,
        bytes: &[u8],
    ) -> Result<ResolvedWrite, ResolverDiagnostic> {
        let Some(callback) = RESOLVER_WRITE.with(|cell| cell.borrow().clone()) else {
            return Err(unsupported(request));
        };
        let request_json = request_json(request)?;
        let payload = Uint8Array::from(bytes);
        let response = callback
            .call2(
                &JsValue::NULL,
                &JsValue::from_str(&request_json),
                payload.as_ref(),
            )
            .map_err(|error| js_io(request, error_message(&error)))?;
        write_response(request, response)
    }
}

pub fn resolver_registry_for_schemes(schemes: &[&str]) -> ResolverRegistry {
    let mut registry = ResolverRegistry::new();
    for scheme in schemes {
        for purpose in RESOLVE_PURPOSES {
            registry.register(*scheme, purpose, ResolveDirection::Read, JsResourceResolver);
            registry.register(
                *scheme,
                purpose,
                ResolveDirection::Write,
                JsResourceResolver,
            );
        }
    }
    registry
}

pub fn context_with_resolver_schemes(schemes: &[&str]) -> crate::engine::EngineContext {
    crate::engine::EngineContext {
        resolver_registry: resolver_registry_for_schemes(schemes),
        ..crate::engine::EngineContext::default()
    }
}

pub fn schema_package_manifest_input(uri: &str) -> crate::engine::EngineInput {
    crate::run_config::schema_package_manifest_input(uri, crate::run_config::ScopeConfig::default())
}

pub fn context_with_resolver_schemes_and_schema_packages(
    schemes: &[&str],
    schema_package_uris: &[&str],
) -> crate::engine::EngineContext {
    let mut context = context_with_resolver_schemes(schemes);
    context.schema_package_manifests.extend(
        schema_package_uris
            .iter()
            .map(|uri| schema_package_manifest_input(uri)),
    );
    context
}

const RESOLVE_PURPOSES: [ResolvePurpose; 8] = [
    ResolvePurpose::Config,
    ResolvePurpose::Input,
    ResolvePurpose::Query,
    ResolvePurpose::Template,
    ResolvePurpose::ModuleMap,
    ResolvePurpose::Output,
    ResolvePurpose::Report,
    ResolvePurpose::ObserveEvents,
];

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmResolveRequest<'a> {
    uri: &'a str,
    base_uri: Option<&'a str>,
    purpose: &'static str,
    direction: &'static str,
    content_type_hint: Option<&'a str>,
}

fn request_json(request: &ResolveRequest) -> Result<String, ResolverDiagnostic> {
    serde_json::to_string(&WasmResolveRequest {
        uri: &request.uri,
        base_uri: request.base_uri.as_deref(),
        purpose: request.purpose.as_str(),
        direction: request.direction.as_str(),
        content_type_hint: request.content_type_hint.as_deref(),
    })
    .map_err(|error| js_io(request, error.to_string()))
}

fn read_response(
    request: &ResolveRequest,
    response: JsValue,
) -> Result<ResolvedRead, ResolverDiagnostic> {
    if let Some(text) = response.as_string() {
        return Ok(ResolvedRead {
            uri: request.uri.clone(),
            bytes: text.into_bytes(),
            content_type: None,
        });
    }
    if let Some(message) = response_error(&response) {
        return Err(js_io(request, message));
    }

    let uri =
        optional_string_property(request, &response, "uri")?.unwrap_or_else(|| request.uri.clone());
    let bytes_value = property(request, &response, "bytes")?;
    if bytes_value.is_null() || bytes_value.is_undefined() {
        return Err(js_io(
            request,
            "resolver read response must include bytes".to_owned(),
        ));
    }
    let bytes = bytes_from_js(request, &bytes_value)?;
    let content_type = optional_string_property(request, &response, "contentType")?;
    Ok(ResolvedRead {
        uri,
        bytes,
        content_type,
    })
}

fn write_response(
    request: &ResolveRequest,
    response: JsValue,
) -> Result<ResolvedWrite, ResolverDiagnostic> {
    if response.is_null() || response.is_undefined() {
        return Ok(ResolvedWrite {
            uri: request.uri.clone(),
        });
    }
    if let Some(uri) = response.as_string() {
        return Ok(ResolvedWrite { uri });
    }
    if let Some(message) = response_error(&response) {
        return Err(js_io(request, message));
    }
    Ok(ResolvedWrite {
        uri: optional_string_property(request, &response, "uri")?
            .unwrap_or_else(|| request.uri.clone()),
    })
}

fn bytes_from_js(request: &ResolveRequest, value: &JsValue) -> Result<Vec<u8>, ResolverDiagnostic> {
    if let Some(text) = value.as_string() {
        return Ok(text.into_bytes());
    }
    if value.is_instance_of::<Uint8Array>()
        || value.is_instance_of::<ArrayBuffer>()
        || Array::is_array(value)
    {
        let array = Uint8Array::new(value);
        let mut bytes = vec![0; array.length() as usize];
        array.copy_to(&mut bytes);
        return Ok(bytes);
    }
    Err(js_io(
        request,
        "resolver read response bytes must be a string, Uint8Array, ArrayBuffer, or number array"
            .to_owned(),
    ))
}

fn property(
    request: &ResolveRequest,
    value: &JsValue,
    name: &str,
) -> Result<JsValue, ResolverDiagnostic> {
    Reflect::get(value, &JsValue::from_str(name)).map_err(|error| {
        js_io(
            request,
            format!(
                "resolver response property `{name}` could not be read: {}",
                error_message(&error)
            ),
        )
    })
}

fn optional_string_property(
    request: &ResolveRequest,
    value: &JsValue,
    name: &str,
) -> Result<Option<String>, ResolverDiagnostic> {
    let property = property(request, value, name)?;
    if property.is_null() || property.is_undefined() {
        return Ok(None);
    }
    property.as_string().map(Some).ok_or_else(|| {
        js_io(
            request,
            format!("resolver response property `{name}` must be a string"),
        )
    })
}

fn response_error(value: &JsValue) -> Option<String> {
    let error = Reflect::get(value, &JsValue::from_str("error")).ok()?;
    if error.is_null() || error.is_undefined() {
        return None;
    }
    Some(error_message(&error))
}

fn unsupported(request: &ResolveRequest) -> ResolverDiagnostic {
    ResolverDiagnostic::UnsupportedResolver {
        uri: request.uri.clone(),
        purpose: request.purpose,
        direction: request.direction,
    }
}

fn js_io(request: &ResolveRequest, message: String) -> ResolverDiagnostic {
    ResolverDiagnostic::Io {
        uri: request.uri.clone(),
        message,
    }
}

fn error_message(error: &JsValue) -> String {
    if let Some(message) = error.as_string() {
        return message;
    }
    js_sys::JSON::stringify(error)
        .ok()
        .and_then(|value| value.as_string())
        .unwrap_or_else(|| "JavaScript resolver callback failed".to_owned())
}

/// `EngineObserver` adapter that forwards every event to whichever
/// JS callback is currently registered through `onParseEvent` /
/// `onValidate` / `onTransform`. Embedders pass `&JsObserver` to
/// `observe_pipeline` so JS code sees the events on the registered
/// channels.
pub struct JsObserver;

impl EngineObserver for JsObserver {
    fn on_parse_event(&self, event: &ReportEvent) {
        dispatch_parse(event);
    }
    fn on_validate(&self, event: &ReportEvent) {
        dispatch_validate(event);
    }
    fn on_transform(&self, event: &ReportEvent) {
        dispatch_transform(event);
    }
}

fn dispatch_parse(event: &ReportEvent) {
    let callback = PARSE_OBSERVER.with(|cell| cell.borrow().clone());
    invoke(callback.as_ref(), event);
}

fn dispatch_validate(event: &ReportEvent) {
    let callback = VALIDATE_OBSERVER.with(|cell| cell.borrow().clone());
    invoke(callback.as_ref(), event);
}

fn dispatch_transform(event: &ReportEvent) {
    let callback = TRANSFORM_OBSERVER.with(|cell| cell.borrow().clone());
    invoke(callback.as_ref(), event);
}

fn invoke(callback: Option<&Function>, event: &ReportEvent) {
    let Some(callback) = callback else { return };
    let json = match serde_json::to_string(event) {
        Ok(s) => s,
        Err(_) => return,
    };
    // Ignore the JS-side return value and any thrown error — observer
    // callbacks are fire-and-forget per AC-O-1; engine work must not
    // abort because a JS observer threw.
    let _ = callback.call1(&JsValue::NULL, &JsValue::from_str(&json));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_input_spec_record_preserves_namespace_scope() {
        let json = parse_input_spec_record(
            "uri=src/a.data,defaultNs=http://www.w3.org/1999/xhtml,namespaces=svg:http://www.w3.org/2000/svg",
        );
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["uri"], "src/a.data");
        assert_eq!(
            value["rootScope"]["defaultNamespace"],
            "http://www.w3.org/1999/xhtml"
        );
        assert_eq!(
            value["rootScope"]["namespaces"]["svg"],
            "http://www.w3.org/2000/svg"
        );
    }

    #[test]
    fn parse_output_spec_record_preserves_namespace_target_scope() {
        let json = parse_output_spec_record(
            "dest=dist/a.out,defaultNs=https://cem.dev/ns/core/1,namespaces=html:http://www.w3.org/1999/xhtml",
        );
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["destination"], "dist/a.out");
        assert_eq!(
            value["rootScope"]["defaultNamespace"],
            "https://cem.dev/ns/core/1"
        );
        assert_eq!(
            value["rootScope"]["namespaces"]["html"],
            "http://www.w3.org/1999/xhtml"
        );
    }

    #[test]
    fn normalize_run_config_preserves_input_and_output_namespace_scopes() {
        let json = normalize_run_config(
            r#"{
                "inputs": [{
                    "uri": "src/a.data",
                    "rootScope": {
                        "defaultNamespace": "http://www.w3.org/1999/xhtml"
                    }
                }],
                "outputs": [{
                    "destination": "dist/a.out",
                    "rootScope": {
                        "namespaces": {
                            "svg": "http://www.w3.org/2000/svg"
                        }
                    }
                }]
            }"#,
        );
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(
            value["config"]["inputs"][0]["rootScope"]["defaultNamespace"],
            "http://www.w3.org/1999/xhtml"
        );
        assert_eq!(
            value["config"]["outputs"][0]["rootScope"]["namespaces"]["svg"],
            "http://www.w3.org/2000/svg"
        );
        assert_eq!(value["diagnostics"], serde_json::json!([]));
    }

    #[test]
    fn normalize_run_config_preserves_uri_shaped_paths_at_api_boundary() {
        let json = normalize_run_config(
            r#"{
                "inputs": [{
                    "uri": "https://example.test/src/a.cem",
                    "rootScope": {
                        "moduleMap": "cem+vfs://workspace/cem.modules.json"
                    }
                }],
                "outputs": [{
                    "inputRef": "https://example.test/src/a.cem",
                    "destination": "file://example.test/dist/a.json",
                    "rootScope": {
                        "moduleMap": "https://example.test/out.modules.json"
                    }
                }]
            }"#,
        );
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(
            value["config"]["inputs"][0]["uri"],
            "https://example.test/src/a.cem"
        );
        assert_eq!(
            value["config"]["inputs"][0]["rootScope"]["moduleMap"],
            "cem+vfs://workspace/cem.modules.json"
        );
        assert_eq!(
            value["config"]["outputs"][0]["destination"],
            "file://example.test/dist/a.json"
        );
        assert_eq!(
            value["config"]["outputs"][0]["rootScope"]["moduleMap"],
            "https://example.test/out.modules.json"
        );
        assert_eq!(value["diagnostics"], serde_json::json!([]));
    }
}
