//! Attributed export outcomes, separate from expression and reference resolution.
use super::*;
use cem_ml::{
    operation_control::{ControlCause, ControlError, ExecutionScopeId},
    source_map::SourceMapStack,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeExportFailureKind {
    UnsupportedSourceReference,
    UnsupportedGraph,
    InvalidValue,
    Limit,
    Cancelled,
    AccessDenied,
    ExecutionFailure,
}
#[derive(Debug, Clone)]
pub struct NativeExportFailure {
    pub kind: NativeExportFailureKind,
    pub message: String,
    pub source: Option<SourceMapStack>,
    pub control: Option<ControlError>,
}
impl std::fmt::Display for NativeExportFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for NativeExportFailure {}
impl From<String> for NativeExportFailure {
    fn from(message: String) -> Self {
        let kind = if message == "Cyclic native CEM value graph" {
            NativeExportFailureKind::UnsupportedGraph
        } else if message.contains("limit exceeded") {
            NativeExportFailureKind::Limit
        } else if message.contains("access denied") {
            NativeExportFailureKind::AccessDenied
        } else {
            NativeExportFailureKind::InvalidValue
        };
        Self {
            kind,
            message,
            source: None,
            control: None,
        }
    }
}
impl From<&str> for NativeExportFailure {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}
impl From<ControlError> for NativeExportFailure {
    fn from(error: ControlError) -> Self {
        let kind = match &error {
            ControlError::Triggered(failure) => match &failure.cause {
                ControlCause::HostCancellation { .. } | ControlCause::Superseded { .. } => {
                    NativeExportFailureKind::Cancelled
                }
                ControlCause::MemoryExceeded { .. }
                | ControlCause::StackDepthExceeded { .. }
                | ControlCause::TimeoutExceeded { .. }
                | ControlCause::QueueCapacityExceeded { .. } => NativeExportFailureKind::Limit,
                ControlCause::InternalFailure { diagnostic_code }
                    if diagnostic_code == "cem.value.limit" =>
                {
                    NativeExportFailureKind::Limit
                }
                _ => NativeExportFailureKind::ExecutionFailure,
            },
            _ => NativeExportFailureKind::ExecutionFailure,
        };
        let source = match &error {
            ControlError::Triggered(f) => f.source_map.clone(),
            _ => None,
        };
        Self {
            kind,
            message: error.to_string(),
            source,
            control: Some(error),
        }
    }
}
impl NativeExportFailure {
    pub fn diagnostic_code(&self) -> &'static str {
        match self.kind {
            NativeExportFailureKind::UnsupportedSourceReference => {
                "cem.value.unsupported_source_reference"
            }
            NativeExportFailureKind::UnsupportedGraph => "cem.value.unsupported_graph",
            NativeExportFailureKind::Limit => "cem.value.limit",
            _ => "cem.value.artifact",
        }
    }
    pub(super) fn into_control(
        self,
        control: &OperationControl,
        scope: ExecutionScopeId,
    ) -> ControlError {
        if let Some(error) = self.control {
            return error;
        }
        let cause = ControlCause::InternalFailure {
            diagnostic_code: self.diagnostic_code().into(),
        };
        match control.fail_scope(scope, cause, self.source) {
            Ok(failure) => ControlError::Triggered(failure),
            Err(error) => error,
        }
    }
}

pub fn export_values(
    values: &ItemStream,
    limits: &CemValueArtifactLimits,
) -> Result<Vec<u8>, NativeExportFailure> {
    export_values_with_control(
        values,
        limits,
        QueryContextScope(0),
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
    )
}

pub fn export_values_with_control(
    values: &ItemStream,
    limits: &CemValueArtifactLimits,
    query_scope: QueryContextScope,
    control: &OperationControl,
    scope: ExecutionScopeId,
) -> Result<Vec<u8>, NativeExportFailure> {
    let mut budget = value_control::ValueControl::new(control, scope, query_scope, *limits)?;
    if let Some(message) = &values.error {
        return Err(NativeExportFailure {
            kind: NativeExportFailureKind::InvalidValue,
            message: format!("Cannot export failed query result: {message:?}"),
            source: values.items.first().and_then(Item::source_map),
            control: None,
        });
    }
    let limits = budget.limits;
    let mut failure = None;
    let graph = encode_graph_checked(values, &limits, true, query_scope, &mut || {
        budget.charge(0, 0).map_err(|error| {
            let message = error.to_string();
            failure = Some(error);
            message
        })
    });
    if let Some(error) = failure {
        return Err(error.into());
    }
    let (graph, _) = graph.map_err(|mut error| {
        if error.source.is_none() {
            error.source = values.items.first().and_then(Item::source_map);
        }
        error
    })?;
    budget.charge(graph.accounted_bytes(), 0)?;
    let bytes = graph.encode_with_check(&limits, &mut || {
        control.check_scope(scope).map_err(|error| {
            let message = error.to_string();
            failure = Some(error);
            message
        })
    });
    if let Some(error) = failure {
        return Err(error.into());
    }
    let bytes = bytes.map_err(|message| {
        let mut error = NativeExportFailure::from(message);
        error.source = values.items.first().and_then(Item::source_map);
        error
    })?;
    budget.charge(bytes.len(), 0)?;
    control.check_scope(scope)?;
    Ok(bytes)
}
