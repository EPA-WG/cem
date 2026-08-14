//! Request-scoped command-service artifact retention and bounded retrieval.
//!
//! Published artifact bytes remain owned by common Rust behind opaque wire
//! identities. Reads always return an owned byte copy, and disposal retains
//! bounded tombstones so repeated cleanup is idempotent without confusing an
//! unknown handle with one that was already released.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::capability::MAX_IDENTITY_BYTES;
use crate::command_service::{
    sha256_hex, validate_command_artifact_handle_v1, CommandArtifactHandleV1,
    CommandServiceLimitsV1, COMMAND_SERVICE_PROTOCOL_VERSION,
};
use crate::operation_handle::RetainedHandleId;

/// One published artifact and the owned bytes retained for later host reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandServiceRetainedArtifactV1 {
    pub handle: CommandArtifactHandleV1,
    pub bytes: Vec<u8>,
}

/// Metadata accompanying one copied artifact byte range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandServiceArtifactReadV1 {
    pub protocol_version: u16,
    pub request_id: String,
    pub handle: CommandArtifactHandleV1,
    pub offset: u64,
    pub byte_length: u64,
    pub eof: bool,
}

/// Common read result. `bytes` is always an owned copy rather than a view into
/// retained storage or WASM linear memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandServiceArtifactChunkV1 {
    pub metadata: CommandServiceArtifactReadV1,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommandServiceArtifactDisposeDispositionV1 {
    Disposed,
    AlreadyDisposed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandServiceArtifactDisposeAckV1 {
    pub protocol_version: u16,
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle_id: Option<RetainedHandleId>,
    pub disposition: CommandServiceArtifactDisposeDispositionV1,
    pub disposed_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandServiceArtifactRegistryErrorV1 {
    InvalidRequestId,
    RequestUnknown {
        request_id: String,
    },
    ArtifactCount {
        requested: usize,
        maximum: u32,
    },
    DuplicateHandle {
        handle_id: RetainedHandleId,
    },
    ArtifactContract {
        handle_id: RetainedHandleId,
        message: String,
    },
    HandleUnknown {
        request_id: String,
        handle_id: RetainedHandleId,
    },
    HandleForeign {
        request_id: String,
        handle_id: RetainedHandleId,
    },
    HandleDisposed {
        request_id: String,
        handle_id: RetainedHandleId,
    },
    ReadRange {
        offset: u64,
        byte_length: u64,
    },
    ReadLimit {
        requested: u64,
        maximum: u64,
    },
}

impl CommandServiceArtifactRegistryErrorV1 {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequestId => "cem.command_service.artifact_request_invalid",
            Self::RequestUnknown { .. } => "cem.command_service.artifact_request_unknown",
            Self::ArtifactCount { .. } => "cem.command_service.artifact_count",
            Self::DuplicateHandle { .. } => "cem.command_service.artifact_handle_duplicate",
            Self::ArtifactContract { .. } => "cem.command_service.artifact_contract",
            Self::HandleUnknown { .. } => "cem.command_service.artifact_handle_unknown",
            Self::HandleForeign { .. } => "cem.command_service.artifact_handle_foreign",
            Self::HandleDisposed { .. } => "cem.command_service.artifact_handle_disposed",
            Self::ReadRange { .. } => "cem.command_service.artifact_read_range",
            Self::ReadLimit { .. } => "cem.command_service.artifact_read_limit",
        }
    }
}

impl fmt::Display for CommandServiceArtifactRegistryErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequestId => formatter.write_str(
                "artifact request identity is empty, over-bound, or contains control characters",
            ),
            Self::RequestUnknown { request_id } => {
                write!(formatter, "artifact request `{request_id}` is not registered")
            }
            Self::ArtifactCount { requested, maximum } => write!(
                formatter,
                "artifact request contains {requested} handles, exceeding the {maximum}-handle limit"
            ),
            Self::DuplicateHandle { handle_id } => {
                write!(formatter, "artifact handle {handle_id} is duplicated")
            }
            Self::ArtifactContract { handle_id, message } => {
                write!(formatter, "artifact handle {handle_id} is invalid: {message}")
            }
            Self::HandleUnknown {
                request_id,
                handle_id,
            } => write!(
                formatter,
                "artifact handle {handle_id} is unknown for request `{request_id}`"
            ),
            Self::HandleForeign {
                request_id,
                handle_id,
            } => write!(
                formatter,
                "artifact handle {handle_id} is not owned by request `{request_id}`"
            ),
            Self::HandleDisposed {
                request_id,
                handle_id,
            } => write!(
                formatter,
                "artifact handle {handle_id} for request `{request_id}` is disposed"
            ),
            Self::ReadRange {
                offset,
                byte_length,
            } => write!(
                formatter,
                "artifact read offset {offset} exceeds byte length {byte_length}"
            ),
            Self::ReadLimit { requested, maximum } => write!(
                formatter,
                "artifact read size {requested} is outside 1..={maximum} bytes"
            ),
        }
    }
}

impl std::error::Error for CommandServiceArtifactRegistryErrorV1 {}

#[derive(Debug, Clone)]
struct RetainedArtifactEntryV1 {
    handle: CommandArtifactHandleV1,
    bytes: Arc<[u8]>,
}

#[derive(Debug, Default)]
struct RequestArtifactStateV1 {
    retained: BTreeMap<RetainedHandleId, RetainedArtifactEntryV1>,
    disposed: BTreeSet<RetainedHandleId>,
}

/// Clone-shared, request-scoped retained-artifact registry for host bindings.
#[derive(Debug, Clone)]
pub struct CommandServiceArtifactRegistryV1 {
    requests: Arc<Mutex<BTreeMap<String, RequestArtifactStateV1>>>,
    max_artifacts: u32,
    max_read_bytes: u64,
}

impl Default for CommandServiceArtifactRegistryV1 {
    fn default() -> Self {
        Self::with_limits(CommandServiceLimitsV1::default())
    }
}

impl CommandServiceArtifactRegistryV1 {
    pub fn with_limits(limits: CommandServiceLimitsV1) -> Self {
        Self {
            requests: Arc::new(Mutex::new(BTreeMap::new())),
            max_artifacts: limits.operation_host.max_artifact_references,
            max_read_bytes: limits.worker.max_transfer_bytes_per_message,
        }
    }

    /// Begin a new generation for a request identity. Any prior retained bytes
    /// are released before the new operation performs host I/O.
    pub fn begin_request(
        &self,
        request_id: &str,
    ) -> Result<(), CommandServiceArtifactRegistryErrorV1> {
        validate_request_id(request_id)?;
        let mut requests = self
            .requests
            .lock()
            .expect("poisoned command-service artifact registry");
        if let Some(state) = requests.get_mut(request_id) {
            state.disposed.extend(state.retained.keys().copied());
            state.retained.clear();
        }
        Ok(())
    }

    /// Atomically publish the retained bytes for one successfully committed
    /// request. Metadata and digests are checked before the registry mutates.
    pub fn publish(
        &self,
        request_id: &str,
        artifacts: Vec<CommandServiceRetainedArtifactV1>,
    ) -> Result<(), CommandServiceArtifactRegistryErrorV1> {
        validate_request_id(request_id)?;
        if artifacts.len() > self.max_artifacts as usize {
            return Err(CommandServiceArtifactRegistryErrorV1::ArtifactCount {
                requested: artifacts.len(),
                maximum: self.max_artifacts,
            });
        }

        let mut retained = BTreeMap::new();
        for artifact in artifacts {
            let handle_id = artifact.handle.handle_id;
            validate_command_artifact_handle_v1(&artifact.handle).map_err(|error| {
                CommandServiceArtifactRegistryErrorV1::ArtifactContract {
                    handle_id,
                    message: error.to_string(),
                }
            })?;
            if artifact.handle.byte_length != artifact.bytes.len() as u64 {
                return Err(CommandServiceArtifactRegistryErrorV1::ArtifactContract {
                    handle_id,
                    message: "declared byte length does not match retained bytes".to_owned(),
                });
            }
            if sha256_hex(&artifact.bytes) != artifact.handle.sha256 {
                return Err(CommandServiceArtifactRegistryErrorV1::ArtifactContract {
                    handle_id,
                    message: "declared digest does not match retained bytes".to_owned(),
                });
            }
            if retained
                .insert(
                    handle_id,
                    RetainedArtifactEntryV1 {
                        handle: artifact.handle,
                        bytes: Arc::from(artifact.bytes),
                    },
                )
                .is_some()
            {
                return Err(CommandServiceArtifactRegistryErrorV1::DuplicateHandle { handle_id });
            }
        }

        let mut requests = self
            .requests
            .lock()
            .expect("poisoned command-service artifact registry");
        let state = requests.entry(request_id.to_owned()).or_default();
        state.disposed.extend(state.retained.keys().copied());
        for handle_id in retained.keys() {
            state.disposed.remove(handle_id);
        }
        state.retained = retained;
        Ok(())
    }

    pub fn read(
        &self,
        request_id: &str,
        handle_id: RetainedHandleId,
        offset: u64,
        max_bytes: u64,
    ) -> Result<CommandServiceArtifactChunkV1, CommandServiceArtifactRegistryErrorV1> {
        validate_request_id(request_id)?;
        if max_bytes == 0 || max_bytes > self.max_read_bytes {
            return Err(CommandServiceArtifactRegistryErrorV1::ReadLimit {
                requested: max_bytes,
                maximum: self.max_read_bytes,
            });
        }
        let requests = self
            .requests
            .lock()
            .expect("poisoned command-service artifact registry");
        let state = match requests.get(request_id) {
            Some(state) => state,
            None if requests
                .values()
                .any(|state| state.retained.contains_key(&handle_id)) =>
            {
                return Err(CommandServiceArtifactRegistryErrorV1::HandleForeign {
                    request_id: request_id.to_owned(),
                    handle_id,
                });
            }
            None => {
                return Err(CommandServiceArtifactRegistryErrorV1::RequestUnknown {
                    request_id: request_id.to_owned(),
                });
            }
        };
        let Some(entry) = state.retained.get(&handle_id) else {
            return Err(missing_handle_error(
                &requests, request_id, handle_id, state,
            ));
        };
        let byte_length = entry.bytes.len() as u64;
        if offset > byte_length {
            return Err(CommandServiceArtifactRegistryErrorV1::ReadRange {
                offset,
                byte_length,
            });
        }
        let end = offset.saturating_add(max_bytes).min(byte_length);
        let start = usize::try_from(offset).expect("validated artifact offset fits usize");
        let end_index = usize::try_from(end).expect("validated artifact end fits usize");
        let bytes = entry.bytes[start..end_index].to_vec();
        Ok(CommandServiceArtifactChunkV1 {
            metadata: CommandServiceArtifactReadV1 {
                protocol_version: COMMAND_SERVICE_PROTOCOL_VERSION,
                request_id: request_id.to_owned(),
                handle: entry.handle.clone(),
                offset,
                byte_length: end - offset,
                eof: end == byte_length,
            },
            bytes,
        })
    }

    pub fn dispose(
        &self,
        request_id: &str,
        handle_id: RetainedHandleId,
    ) -> Result<CommandServiceArtifactDisposeAckV1, CommandServiceArtifactRegistryErrorV1> {
        validate_request_id(request_id)?;
        let mut requests = self
            .requests
            .lock()
            .expect("poisoned command-service artifact registry");
        let disposition = {
            let state = match requests.get(request_id) {
                Some(state) => state,
                None if requests
                    .values()
                    .any(|state| state.retained.contains_key(&handle_id)) =>
                {
                    return Err(CommandServiceArtifactRegistryErrorV1::HandleForeign {
                        request_id: request_id.to_owned(),
                        handle_id,
                    });
                }
                None => {
                    return Err(CommandServiceArtifactRegistryErrorV1::RequestUnknown {
                        request_id: request_id.to_owned(),
                    });
                }
            };
            if state.retained.contains_key(&handle_id) {
                CommandServiceArtifactDisposeDispositionV1::Disposed
            } else if state.disposed.contains(&handle_id) {
                CommandServiceArtifactDisposeDispositionV1::AlreadyDisposed
            } else {
                return Err(missing_handle_error(
                    &requests, request_id, handle_id, state,
                ));
            }
        };
        if disposition == CommandServiceArtifactDisposeDispositionV1::Disposed {
            let state = requests
                .get_mut(request_id)
                .expect("validated artifact request remains registered");
            state.retained.remove(&handle_id);
            state.disposed.insert(handle_id);
        }
        Ok(CommandServiceArtifactDisposeAckV1 {
            protocol_version: COMMAND_SERVICE_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            handle_id: Some(handle_id),
            disposition,
            disposed_count: u32::from(
                disposition == CommandServiceArtifactDisposeDispositionV1::Disposed,
            ),
        })
    }

    pub fn dispose_request(
        &self,
        request_id: &str,
    ) -> Result<CommandServiceArtifactDisposeAckV1, CommandServiceArtifactRegistryErrorV1> {
        validate_request_id(request_id)?;
        let mut requests = self
            .requests
            .lock()
            .expect("poisoned command-service artifact registry");
        let state = requests.get_mut(request_id).ok_or_else(|| {
            CommandServiceArtifactRegistryErrorV1::RequestUnknown {
                request_id: request_id.to_owned(),
            }
        })?;
        let disposed_count = state.retained.len().try_into().unwrap_or(u32::MAX);
        state.disposed.extend(state.retained.keys().copied());
        state.retained.clear();
        Ok(CommandServiceArtifactDisposeAckV1 {
            protocol_version: COMMAND_SERVICE_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            handle_id: None,
            disposition: if disposed_count == 0 {
                CommandServiceArtifactDisposeDispositionV1::AlreadyDisposed
            } else {
                CommandServiceArtifactDisposeDispositionV1::Disposed
            },
            disposed_count,
        })
    }
}

fn validate_request_id(request_id: &str) -> Result<(), CommandServiceArtifactRegistryErrorV1> {
    if request_id.is_empty()
        || request_id.len() > MAX_IDENTITY_BYTES
        || request_id.chars().any(char::is_control)
    {
        return Err(CommandServiceArtifactRegistryErrorV1::InvalidRequestId);
    }
    Ok(())
}

fn missing_handle_error(
    requests: &BTreeMap<String, RequestArtifactStateV1>,
    request_id: &str,
    handle_id: RetainedHandleId,
    state: &RequestArtifactStateV1,
) -> CommandServiceArtifactRegistryErrorV1 {
    if state.disposed.contains(&handle_id) {
        CommandServiceArtifactRegistryErrorV1::HandleDisposed {
            request_id: request_id.to_owned(),
            handle_id,
        }
    } else if requests.iter().any(|(other_request, other_state)| {
        other_request != request_id && other_state.retained.contains_key(&handle_id)
    }) {
        CommandServiceArtifactRegistryErrorV1::HandleForeign {
            request_id: request_id.to_owned(),
            handle_id,
        }
    } else {
        CommandServiceArtifactRegistryErrorV1::HandleUnknown {
            request_id: request_id.to_owned(),
            handle_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_service::CommandArtifactKindV1;

    fn artifact(handle_id: u64, bytes: &[u8]) -> CommandServiceRetainedArtifactV1 {
        CommandServiceRetainedArtifactV1 {
            handle: CommandArtifactHandleV1 {
                handle_id: RetainedHandleId::from_raw(handle_id),
                kind: CommandArtifactKindV1::Output,
                uri: Some(format!("memory:artifact-{handle_id}")),
                content_type: "application/octet-stream".to_owned(),
                byte_length: bytes.len() as u64,
                sha256: sha256_hex(bytes),
                source_map_id: None,
            },
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn registry_owns_chunk_integrity_scope_disposal_and_request_reuse() {
        let mut limits = CommandServiceLimitsV1::default();
        limits.worker.max_transfer_bytes_per_message = 4;
        let registry = CommandServiceArtifactRegistryV1::with_limits(limits);
        registry.begin_request("request:a").unwrap();
        registry
            .publish("request:a", vec![artifact(1, b"abcdef")])
            .unwrap();

        let first = registry
            .read("request:a", RetainedHandleId::from_raw(1), 0, 4)
            .unwrap();
        assert_eq!(first.bytes, b"abcd");
        assert!(!first.metadata.eof);
        assert_eq!(
            serde_json::to_value(&first.metadata).unwrap(),
            serde_json::json!({
                "protocolVersion": 1,
                "requestId": "request:a",
                "handle": {
                    "handleId": 1,
                    "kind": "output",
                    "uri": "memory:artifact-1",
                    "contentType": "application/octet-stream",
                    "byteLength": 6,
                    "sha256": sha256_hex(b"abcdef")
                },
                "offset": 0,
                "byteLength": 4,
                "eof": false
            })
        );
        let second = registry
            .read("request:a", RetainedHandleId::from_raw(1), 4, 4)
            .unwrap();
        assert_eq!(second.bytes, b"ef");
        assert!(second.metadata.eof);
        assert_eq!(
            registry
                .read("request:a", RetainedHandleId::from_raw(1), 0, 5)
                .unwrap_err()
                .code(),
            "cem.command_service.artifact_read_limit"
        );

        registry.begin_request("request:b").unwrap();
        registry
            .publish("request:b", vec![artifact(2, b"other")])
            .unwrap();
        assert_eq!(
            registry
                .read("request:a", RetainedHandleId::from_raw(2), 0, 1)
                .unwrap_err()
                .code(),
            "cem.command_service.artifact_handle_foreign"
        );
        assert_eq!(
            registry
                .read("request:a", RetainedHandleId::from_raw(99), 0, 1)
                .unwrap_err()
                .code(),
            "cem.command_service.artifact_handle_unknown"
        );

        registry.begin_request("request:b").unwrap();
        assert_eq!(
            registry
                .read("request:b", RetainedHandleId::from_raw(2), 0, 1)
                .unwrap_err()
                .code(),
            "cem.command_service.artifact_handle_disposed"
        );
        registry
            .publish("request:b", vec![artifact(2, b"new generation")])
            .unwrap();
        assert_eq!(
            registry
                .read("request:b", RetainedHandleId::from_raw(2), 0, 4)
                .unwrap()
                .bytes,
            b"new "
        );

        let disposed = registry
            .dispose("request:a", RetainedHandleId::from_raw(1))
            .unwrap();
        assert_eq!(
            disposed.disposition,
            CommandServiceArtifactDisposeDispositionV1::Disposed
        );
        assert_eq!(disposed.disposed_count, 1);
        assert_eq!(
            registry
                .dispose("request:a", RetainedHandleId::from_raw(1))
                .unwrap()
                .disposition,
            CommandServiceArtifactDisposeDispositionV1::AlreadyDisposed
        );
        assert_eq!(
            registry
                .read("request:a", RetainedHandleId::from_raw(1), 0, 1)
                .unwrap_err()
                .code(),
            "cem.command_service.artifact_handle_disposed"
        );

        registry.begin_request("request:a").unwrap();
        registry
            .publish("request:a", vec![artifact(1, b"new")])
            .unwrap();
        assert_eq!(
            registry
                .read("request:a", RetainedHandleId::from_raw(1), 0, 4)
                .unwrap()
                .bytes,
            b"new"
        );
        assert_eq!(
            registry
                .dispose_request("request:a")
                .unwrap()
                .disposed_count,
            1
        );
        assert_eq!(
            registry.dispose_request("request:a").unwrap().disposition,
            CommandServiceArtifactDisposeDispositionV1::AlreadyDisposed
        );
    }

    #[test]
    fn publication_validation_is_atomic() {
        let registry = CommandServiceArtifactRegistryV1::default();
        registry.begin_request("request:atomic").unwrap();
        registry
            .publish("request:atomic", vec![artifact(2, b"retained")])
            .unwrap();
        let mut invalid = artifact(1, b"fixture");
        invalid.handle.byte_length += 1;
        assert_eq!(
            registry
                .publish("request:atomic", vec![invalid])
                .unwrap_err()
                .code(),
            "cem.command_service.artifact_contract"
        );
        assert_eq!(
            registry
                .read("request:atomic", RetainedHandleId::from_raw(2), 0, 8)
                .unwrap()
                .bytes,
            b"retained"
        );
    }
}
