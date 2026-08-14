//! Async JavaScript bridge for the Rust-owned command-service v1 lifecycle.
//!
//! The exported function keeps the request/result model in common Rust while
//! accepting only constructor-style host capabilities as JavaScript callbacks.
//! Callback arguments and responses are canonical JSON strings; publication
//! bytes are transferred separately as a `Uint8Array`.

use std::fmt;

use js_sys::{Function, Promise, Uint8Array};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::capability::{capability_manifest, CapabilityRequest};
use crate::command_execution::{
    project_command_service_terminal_v1, stale_command_service_result_v1,
};
use crate::command_host::{
    CommandHostFuture, CommandResolvedResourceV1, CommandResourceReadFailureV1,
    CommandResourceReadRequestV1, CommandResourceReaderV1,
};
use crate::command_publication::{
    CommandPreparedResourceWriteV1, CommandPublicationHostFailureV1, CommandResolvedWriteV1,
    CommandResourceWriteRequestV1, CommandResourceWriterV1, CommandRevisionLedgerReaderV1,
    CommandRevisionLedgerRequestV1,
};
use crate::command_runtime::{
    CommandExecutionServicesV1, CommandServiceHostErrorV1, CommandServiceHostV1,
    CommandServicePreparationV1,
};
use crate::command_service::{
    decode_command_service_request_v1, CommandRevisionLedgerV1, CommandServiceLimitsV1,
    CommandServiceResultV1,
};
use crate::engine::EngineContext;
use crate::operation_control::OperationControl;
use crate::query::QueryResultExporterRegistry;
use crate::real::RealCemMlEngine;
use crate::validation::css_selector::register_css_selector_query_exporters;
use crate::validation::xpath::register_xpath_query_exporters;

/// Execute one command-service v1 request against JavaScript-owned async host
/// capabilities. The returned promise always resolves to a JSON string holding
/// either the canonical `CommandServiceResultV1` or `{ "error": ... }` for a
/// pre-terminal host/admission failure.
#[wasm_bindgen(js_name = "executeCommandServiceV1")]
pub async fn execute_command_service_v1(
    request_json: String,
    capability_request_json: String,
    current_revision: Function,
    read_resource: Function,
    prepare_write: Function,
    commit_write: Function,
    rollback_write: Function,
) -> String {
    command_service_response(
        execute_command_service(
            &request_json,
            &capability_request_json,
            JsCommandServiceCallbacks {
                current_revision,
                read_resource,
                prepare_write,
                commit_write,
                rollback_write,
            },
        )
        .await,
    )
}

async fn execute_command_service(
    request_json: &str,
    capability_request_json: &str,
    callbacks: JsCommandServiceCallbacks,
) -> Result<CommandServiceResultV1, WasmCommandServiceError> {
    let request = decode_command_service_request_v1(request_json.as_bytes())
        .map_err(|error| WasmCommandServiceError::new(error.code(), error.to_string()))?;
    let projection_request = request.clone();
    let capability_request = serde_json::from_str::<CapabilityRequest>(capability_request_json)
        .map_err(|error| {
            WasmCommandServiceError::new(
                "cem.capability.invalid_request",
                format!("command-service capability request is invalid: {error}"),
            )
        })?;
    let capability = capability_manifest(capability_request)
        .map_err(|error| WasmCommandServiceError::new(error.code, error.message))?;
    let limits = CommandServiceLimitsV1::default();
    let mut query_exporters = QueryResultExporterRegistry::new();
    register_css_selector_query_exporters(&mut query_exporters);
    register_xpath_query_exporters(&mut query_exporters);
    let host = CommandServiceHostV1::new(
        Box::new(JsCommandRevisionLedger {
            callback: callbacks.current_revision.clone(),
        }),
        Box::new(JsCommandResourceReader {
            callback: callbacks.read_resource.clone(),
        }),
        CommandExecutionServicesV1 {
            writer: Box::new(JsCommandResourceWriter {
                prepare: callbacks.prepare_write,
                commit: callbacks.commit_write,
                rollback: callbacks.rollback_write,
            }),
            query_exporters,
        },
        limits,
        EngineContext::default(),
        capability.clone(),
    )
    .map_err(host_error)?;

    match host
        .prepare(request, OperationControl::default())
        .await
        .map_err(host_error)?
    {
        CommandServicePreparationV1::Stale(stale) => Ok(stale_command_service_result_v1(
            &projection_request,
            stale,
            &capability,
            limits,
        )),
        CommandServicePreparationV1::Ready(invocation) => {
            let terminal = host
                .execute(&RealCemMlEngine::new(), invocation)
                .await
                .map_err(host_error)?;
            Ok(project_command_service_terminal_v1(
                &projection_request,
                &terminal,
                &capability,
                limits,
            ))
        }
    }
}

struct JsCommandServiceCallbacks {
    current_revision: Function,
    read_resource: Function,
    prepare_write: Function,
    commit_write: Function,
    rollback_write: Function,
}

struct JsCommandRevisionLedger {
    callback: Function,
}

impl CommandRevisionLedgerReaderV1 for JsCommandRevisionLedger {
    fn current<'a>(
        &'a self,
        request: CommandRevisionLedgerRequestV1,
    ) -> CommandHostFuture<'a, Result<CommandRevisionLedgerV1, CommandPublicationHostFailureV1>>
    {
        let callback = self.callback.clone();
        Box::pin(async move {
            let request = serde_json::to_string(&request).map_err(publication_serialize_error)?;
            let response = invoke_one(&callback, JsValue::from_str(&request))
                .await
                .map_err(publication_callback_error)?;
            decode_callback(response, "currentRevision").map_err(publication_callback_error)
        })
    }
}

struct JsCommandResourceReader {
    callback: Function,
}

impl CommandResourceReaderV1 for JsCommandResourceReader {
    fn read<'a>(
        &'a self,
        request: CommandResourceReadRequestV1,
    ) -> CommandHostFuture<'a, Result<CommandResolvedResourceV1, CommandResourceReadFailureV1>>
    {
        let callback = self.callback.clone();
        Box::pin(async move {
            let request = serde_json::to_string(&request).map_err(resource_serialize_error)?;
            let response = invoke_one(&callback, JsValue::from_str(&request))
                .await
                .map_err(resource_callback_error)?;
            decode_callback(response, "readResource").map_err(resource_callback_error)
        })
    }
}

struct JsCommandResourceWriter {
    prepare: Function,
    commit: Function,
    rollback: Function,
}

impl CommandResourceWriterV1 for JsCommandResourceWriter {
    fn prepare<'a>(
        &'a self,
        request: CommandResourceWriteRequestV1,
        bytes: &'a [u8],
    ) -> CommandHostFuture<
        'a,
        Result<Box<dyn CommandPreparedResourceWriteV1>, CommandPublicationHostFailureV1>,
    > {
        let prepare = self.prepare.clone();
        let commit = self.commit.clone();
        let rollback = self.rollback.clone();
        let bytes = Uint8Array::from(bytes);
        Box::pin(async move {
            let request = serde_json::to_string(&request).map_err(publication_serialize_error)?;
            let response = invoke_two(&prepare, JsValue::from_str(&request), bytes.into())
                .await
                .map_err(publication_callback_error)?;
            let prepared = decode_callback::<PreparedWriteToken>(response, "prepareWrite")
                .map_err(publication_callback_error)?;
            if prepared.token.trim().is_empty() {
                return Err(CommandPublicationHostFailureV1::new(
                    "cem.command_service.host_callback",
                    "prepareWrite returned an empty transaction token",
                ));
            }
            Ok(Box::new(JsPreparedCommandWrite {
                token: prepared.token,
                commit,
                rollback,
            }) as Box<dyn CommandPreparedResourceWriteV1>)
        })
    }
}

struct JsPreparedCommandWrite {
    token: String,
    commit: Function,
    rollback: Function,
}

impl CommandPreparedResourceWriteV1 for JsPreparedCommandWrite {
    fn commit<'a>(
        &'a mut self,
    ) -> CommandHostFuture<'a, Result<CommandResolvedWriteV1, CommandPublicationHostFailureV1>>
    {
        let callback = self.commit.clone();
        let token = self.token.clone();
        Box::pin(async move {
            let response = invoke_one(&callback, JsValue::from_str(&token))
                .await
                .map_err(publication_callback_error)?;
            decode_callback(response, "commitWrite").map_err(publication_callback_error)
        })
    }

    fn rollback<'a>(
        &'a mut self,
    ) -> CommandHostFuture<'a, Result<(), CommandPublicationHostFailureV1>> {
        let callback = self.rollback.clone();
        let token = self.token.clone();
        Box::pin(async move {
            let response = invoke_one(&callback, JsValue::from_str(&token))
                .await
                .map_err(publication_callback_error)?;
            decode_ack(response, "rollbackWrite").map_err(publication_callback_error)
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreparedWriteToken {
    token: String,
}

#[derive(Debug)]
struct CallbackFailure {
    code: String,
    message: String,
}

async fn invoke_one(callback: &Function, value: JsValue) -> Result<JsValue, CallbackFailure> {
    let response = callback
        .call1(&JsValue::NULL, &value)
        .map_err(|error| rejected_callback("callback", error))?;
    JsFuture::from(Promise::resolve(&response))
        .await
        .map_err(|error| rejected_callback("callback promise", error))
}

async fn invoke_two(
    callback: &Function,
    first: JsValue,
    second: JsValue,
) -> Result<JsValue, CallbackFailure> {
    let response = callback
        .call2(&JsValue::NULL, &first, &second)
        .map_err(|error| rejected_callback("callback", error))?;
    JsFuture::from(Promise::resolve(&response))
        .await
        .map_err(|error| rejected_callback("callback promise", error))
}

fn decode_callback<T: DeserializeOwned>(
    response: JsValue,
    boundary: &str,
) -> Result<T, CallbackFailure> {
    let json = response.as_string().ok_or_else(|| CallbackFailure {
        code: "cem.command_service.host_callback".to_owned(),
        message: format!("{boundary} must resolve to a JSON string"),
    })?;
    let value = serde_json::from_str::<Value>(&json).map_err(|error| CallbackFailure {
        code: "cem.command_service.host_callback".to_owned(),
        message: format!("{boundary} returned invalid JSON: {error}"),
    })?;
    if let Some(error) = callback_failure(&value, boundary) {
        return Err(error);
    }
    serde_json::from_value(value).map_err(|error| CallbackFailure {
        code: "cem.command_service.host_callback".to_owned(),
        message: format!("{boundary} returned an invalid response: {error}"),
    })
}

fn decode_ack(response: JsValue, boundary: &str) -> Result<(), CallbackFailure> {
    if response.is_null() || response.is_undefined() {
        return Ok(());
    }
    let Some(json) = response.as_string() else {
        return Err(CallbackFailure {
            code: "cem.command_service.host_callback".to_owned(),
            message: format!("{boundary} must resolve to undefined, null, or a JSON string"),
        });
    };
    if json.trim().is_empty() {
        return Ok(());
    }
    let value = serde_json::from_str::<Value>(&json).map_err(|error| CallbackFailure {
        code: "cem.command_service.host_callback".to_owned(),
        message: format!("{boundary} returned invalid JSON: {error}"),
    })?;
    if let Some(error) = callback_failure(&value, boundary) {
        return Err(error);
    }
    Ok(())
}

fn callback_failure(value: &Value, boundary: &str) -> Option<CallbackFailure> {
    let error = value.get("error")?.as_object()?;
    let code = error
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or("cem.command_service.host_callback");
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("host callback failed");
    Some(CallbackFailure {
        code: code.to_owned(),
        message: format!("{boundary}: {message}"),
    })
}

fn rejected_callback(boundary: &str, error: JsValue) -> CallbackFailure {
    CallbackFailure {
        code: "cem.command_service.host_callback".to_owned(),
        message: format!("{boundary} rejected: {}", js_error_message(&error)),
    }
}

fn js_error_message(value: &JsValue) -> String {
    value
        .as_string()
        .or_else(|| js_sys::Error::from(value.clone()).message().as_string())
        .unwrap_or_else(|| "unknown JavaScript error".to_owned())
}

fn publication_serialize_error(error: serde_json::Error) -> CommandPublicationHostFailureV1 {
    CommandPublicationHostFailureV1::new(
        "cem.command_service.host_callback",
        format!("host callback request serialization failed: {error}"),
    )
}

fn resource_serialize_error(error: serde_json::Error) -> CommandResourceReadFailureV1 {
    CommandResourceReadFailureV1::new(
        "cem.command_service.host_callback",
        format!("host callback request serialization failed: {error}"),
    )
}

fn publication_callback_error(error: CallbackFailure) -> CommandPublicationHostFailureV1 {
    CommandPublicationHostFailureV1::new(error.code, error.message)
}

fn resource_callback_error(error: CallbackFailure) -> CommandResourceReadFailureV1 {
    CommandResourceReadFailureV1::new(error.code, error.message)
}

fn host_error(error: CommandServiceHostErrorV1) -> WasmCommandServiceError {
    WasmCommandServiceError::new(error.code(), error.to_string())
}

#[derive(Debug)]
struct WasmCommandServiceError {
    code: String,
    message: String,
}

impl WasmCommandServiceError {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for WasmCommandServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

fn command_service_response(
    result: Result<CommandServiceResultV1, WasmCommandServiceError>,
) -> String {
    match result {
        Ok(result) => serde_json::to_string(&result).unwrap_or_else(|error| {
            serde_json::json!({
                "error": {
                    "code": "cem.command_service.serialization_failed",
                    "message": error.to_string(),
                }
            })
            .to_string()
        }),
        Err(error) => serde_json::json!({
            "error": {
                "code": error.code,
                "message": error.message,
            }
        })
        .to_string(),
    }
}
