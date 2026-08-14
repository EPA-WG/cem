//! Transactional command-service artifact publication.
//!
//! Hosts install asynchronous ledger and writer capabilities when constructing
//! the service. The wire request carries only identities. This layer stages a
//! deterministic batch, reserves operation-owned artifact bytes, refreshes the
//! revision ledger immediately before commit, and exposes handles only after
//! every participant commits. Any stale, cancelled, or failed path rolls every
//! participant back, including participants that already committed.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::capability::MAX_IDENTITY_BYTES;
use crate::command_host::CommandHostFuture;
use crate::command_service::{
    admit_command_service_request_v1, sha256_hex, validate_command_service_request_v1,
    CommandArtifactHandleV1, CommandArtifactKindV1, CommandProjectRevisionV1,
    CommandRevisionLedgerV1, CommandServiceAdmissionV1, CommandServiceError,
    CommandServiceLimitsV1, CommandServiceRequestV1, CommandStaleRevisionV1,
};
use crate::operation_control::MAX_SOURCE_URI_BYTES;
use crate::operation_handle::{OperationHandle, OperationHandleError, RetainedHandleMetadata};
use crate::resolver::{uri_scheme, ResolvePurpose};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPublicationItemV1 {
    pub label: String,
    pub uri: String,
    pub kind: CommandArtifactKindV1,
    pub purpose: ResolvePurpose,
    pub content_type: String,
    pub bytes: Vec<u8>,
    pub source_map_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandRevisionLedgerRequestV1 {
    pub request_id: String,
    pub project: CommandProjectRevisionV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResourceWriteRequestV1 {
    pub request_id: String,
    pub project: CommandProjectRevisionV1,
    pub label: String,
    pub uri: String,
    pub kind: CommandArtifactKindV1,
    pub purpose: ResolvePurpose,
    pub content_type: String,
    pub byte_length: u64,
    pub sha256: String,
    pub source_map_id: Option<String>,
    pub resolver_policy_stamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResolvedWriteV1 {
    pub uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPublicationHostFailureV1 {
    pub code: String,
    pub message: String,
}

impl CommandPublicationHostFailureV1 {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for CommandPublicationHostFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CommandPublicationHostFailureV1 {}

/// Constructor-owned provider for the authoritative current revision ledger.
pub trait CommandRevisionLedgerReaderV1 {
    fn current<'a>(
        &'a self,
        request: CommandRevisionLedgerRequestV1,
    ) -> CommandHostFuture<'a, Result<CommandRevisionLedgerV1, CommandPublicationHostFailureV1>>;
}

/// One asynchronously staged host write.
///
/// `rollback` must restore the pre-prepare destination state both before and
/// after a successful `commit`. The command service relies on that guarantee
/// to undo earlier participants when a later commit fails.
pub trait CommandPreparedResourceWriteV1 {
    fn commit<'a>(
        &'a mut self,
    ) -> CommandHostFuture<'a, Result<CommandResolvedWriteV1, CommandPublicationHostFailureV1>>;

    fn rollback<'a>(
        &'a mut self,
    ) -> CommandHostFuture<'a, Result<(), CommandPublicationHostFailureV1>>;
}

/// Constructor-owned transactional writer. `prepare` must finish staging the
/// supplied bytes before its future resolves; the returned participant owns
/// all state needed for later commit or rollback.
pub trait CommandResourceWriterV1 {
    fn prepare<'a>(
        &'a self,
        request: CommandResourceWriteRequestV1,
        bytes: &'a [u8],
    ) -> CommandHostFuture<
        'a,
        Result<Box<dyn CommandPreparedResourceWriteV1>, CommandPublicationHostFailureV1>,
    >;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPublicationParticipantFailureV1 {
    pub uri: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandPublicationV1 {
    Published(Vec<CommandArtifactHandleV1>),
    Stale(CommandStaleRevisionV1),
}

#[derive(Debug)]
pub enum CommandPublicationErrorV1 {
    Request(CommandServiceError),
    InvalidItem {
        field: &'static str,
        message: String,
    },
    ArtifactCount {
        requested: usize,
        maximum: u32,
    },
    DuplicateDestination {
        uri: String,
    },
    LedgerRead {
        source: CommandPublicationHostFailureV1,
    },
    Prepare {
        uri: String,
        source: CommandPublicationHostFailureV1,
    },
    Commit {
        uri: String,
        source: CommandPublicationHostFailureV1,
    },
    ResolvedDestination {
        requested: String,
        actual: String,
    },
    Operation(OperationHandleError),
    Rollback {
        primary_code: String,
        primary_message: String,
        failures: Vec<CommandPublicationParticipantFailureV1>,
    },
}

impl CommandPublicationErrorV1 {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Request(error) => error.code(),
            Self::InvalidItem { .. } => "cem.command_service.publication_item_invalid",
            Self::ArtifactCount { .. } => "cem.command_service.publication_artifact_count",
            Self::DuplicateDestination { .. } => {
                "cem.command_service.publication_destination_duplicate"
            }
            Self::LedgerRead { .. } => "cem.command_service.publication_ledger_read",
            Self::Prepare { .. } => "cem.command_service.publication_prepare",
            Self::Commit { .. } => "cem.command_service.publication_commit",
            Self::ResolvedDestination { .. } => {
                "cem.command_service.publication_resolved_destination"
            }
            Self::Operation(error) => error.code(),
            Self::Rollback { .. } => "cem.command_service.publication_rollback",
        }
    }
}

impl fmt::Display for CommandPublicationErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(error) => error.fmt(formatter),
            Self::InvalidItem { field, message } => {
                write!(formatter, "command publication item {field} is invalid: {message}")
            }
            Self::ArtifactCount { requested, maximum } => write!(
                formatter,
                "command publication has {requested} artifacts, exceeding {maximum}"
            ),
            Self::DuplicateDestination { uri } => {
                write!(formatter, "command publication destination `{uri}` is duplicated")
            }
            Self::LedgerRead { source } => {
                write!(formatter, "prepublication revision-ledger read failed: {source}")
            }
            Self::Prepare { uri, source } => {
                write!(formatter, "preparing publication destination `{uri}` failed: {source}")
            }
            Self::Commit { uri, source } => {
                write!(formatter, "committing publication destination `{uri}` failed: {source}")
            }
            Self::ResolvedDestination { requested, actual } => write!(
                formatter,
                "publication destination `{requested}` resolved to invalid or duplicate URI `{actual}`"
            ),
            Self::Operation(error) => error.fmt(formatter),
            Self::Rollback {
                primary_message,
                failures,
                ..
            } => write!(
                formatter,
                "{primary_message}; {} publication rollback participant(s) also failed",
                failures.len()
            ),
        }
    }
}

impl std::error::Error for CommandPublicationErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Request(error) => Some(error),
            Self::LedgerRead { source }
            | Self::Prepare { source, .. }
            | Self::Commit { source, .. } => Some(source),
            Self::Operation(error) => Some(error),
            _ => None,
        }
    }
}

impl From<CommandServiceError> for CommandPublicationErrorV1 {
    fn from(error: CommandServiceError) -> Self {
        Self::Request(error)
    }
}

impl From<OperationHandleError> for CommandPublicationErrorV1 {
    fn from(error: OperationHandleError) -> Self {
        Self::Operation(error)
    }
}

struct PreparedParticipant {
    request: CommandResourceWriteRequestV1,
    bytes: Vec<u8>,
    prepared: Box<dyn CommandPreparedResourceWriteV1>,
    resolved_uri: Option<String>,
}

/// Transactionally publish a URI-sorted artifact batch.
///
/// The current ledger is fetched only after every destination is staged and
/// artifact capacity is reserved, and immediately before the first commit.
pub async fn publish_command_artifacts_v1<R: Send + Sync + 'static>(
    request: &CommandServiceRequestV1,
    mut items: Vec<CommandPublicationItemV1>,
    ledger_reader: &dyn CommandRevisionLedgerReaderV1,
    writer: &dyn CommandResourceWriterV1,
    operation: &OperationHandle<R>,
    limits: CommandServiceLimitsV1,
) -> Result<CommandPublicationV1, CommandPublicationErrorV1> {
    validate_command_service_request_v1(request, limits)?;
    operation.ensure_active()?;
    validate_items(&items, operation, limits)?;
    items.sort_by(|left, right| left.uri.cmp(&right.uri));
    if items.is_empty() {
        return Ok(CommandPublicationV1::Published(Vec::new()));
    }

    let mut participants = Vec::with_capacity(items.len());
    for item in items {
        if let Err(error) = operation.ensure_active() {
            return Err(rollback_error(error.into(), &mut participants).await);
        }
        let byte_length = u64::try_from(item.bytes.len()).map_err(|_| {
            CommandPublicationErrorV1::InvalidItem {
                field: "bytes",
                message: "byte length does not fit in an unsigned 64-bit integer".to_owned(),
            }
        })?;
        let write_request = CommandResourceWriteRequestV1 {
            request_id: request.request_id.clone(),
            project: request.project.clone(),
            label: item.label,
            uri: item.uri,
            kind: item.kind,
            purpose: item.purpose,
            content_type: item.content_type,
            byte_length,
            sha256: sha256_hex(&item.bytes),
            source_map_id: item.source_map_id,
            resolver_policy_stamp: request.policy_stamp.resolver.clone(),
        };
        let prepared = match writer.prepare(write_request.clone(), &item.bytes).await {
            Ok(prepared) => prepared,
            Err(source) => {
                let primary = CommandPublicationErrorV1::Prepare {
                    uri: write_request.uri,
                    source,
                };
                return Err(rollback_error(primary, &mut participants).await);
            }
        };
        participants.push(PreparedParticipant {
            request: write_request,
            bytes: item.bytes,
            prepared,
            resolved_uri: None,
        });
    }

    if let Err(error) = operation.ensure_active() {
        return Err(rollback_error(error.into(), &mut participants).await);
    }
    let reservation = match operation.reserve_artifact_batch(
        participants
            .iter_mut()
            .map(|participant| {
                (
                    participant.request.label.clone(),
                    std::mem::take(&mut participant.bytes),
                )
            })
            .collect(),
    ) {
        Ok(reservation) => reservation,
        Err(error) => return Err(rollback_error(error.into(), &mut participants).await),
    };

    let current = match ledger_reader
        .current(CommandRevisionLedgerRequestV1 {
            request_id: request.request_id.clone(),
            project: request.project.clone(),
        })
        .await
    {
        Ok(current) => current,
        Err(source) => {
            let primary = CommandPublicationErrorV1::LedgerRead { source };
            return Err(rollback_error(primary, &mut participants).await);
        }
    };
    if let Err(error) = operation.ensure_active() {
        return Err(rollback_error(error.into(), &mut participants).await);
    }
    match admit_command_service_request_v1(request, &current, limits) {
        Ok(CommandServiceAdmissionV1::Accepted) => {}
        Ok(CommandServiceAdmissionV1::Stale(stale)) => {
            let failures = rollback_all(&mut participants).await;
            if failures.is_empty() {
                return Ok(CommandPublicationV1::Stale(stale));
            }
            return Err(CommandPublicationErrorV1::Rollback {
                primary_code: "cem.command_service.stale".to_owned(),
                primary_message: "request became stale before publication".to_owned(),
                failures,
            });
        }
        Err(error) => return Err(rollback_error(error.into(), &mut participants).await),
    }

    let mut resolved_destinations = BTreeSet::new();
    for index in 0..participants.len() {
        if let Err(error) = operation.ensure_active() {
            return Err(rollback_error(error.into(), &mut participants).await);
        }
        let requested = participants[index].request.uri.clone();
        let resolved = match participants[index].prepared.commit().await {
            Ok(resolved) => resolved,
            Err(source) => {
                let primary = CommandPublicationErrorV1::Commit {
                    uri: requested,
                    source,
                };
                return Err(rollback_error(primary, &mut participants).await);
            }
        };
        if !valid_uri(&resolved.uri) || !resolved_destinations.insert(resolved.uri.clone()) {
            let primary = CommandPublicationErrorV1::ResolvedDestination {
                requested,
                actual: resolved.uri,
            };
            return Err(rollback_error(primary, &mut participants).await);
        }
        participants[index].resolved_uri = Some(resolved.uri);
        if let Err(error) = operation.ensure_active() {
            return Err(rollback_error(error.into(), &mut participants).await);
        }
    }

    let metadata = reservation.finalize();
    let artifacts = participants
        .into_iter()
        .zip(metadata)
        .map(|(participant, metadata)| artifact_handle(participant, metadata))
        .collect();
    Ok(CommandPublicationV1::Published(artifacts))
}

fn validate_items<R: Send + Sync + 'static>(
    items: &[CommandPublicationItemV1],
    operation: &OperationHandle<R>,
    limits: CommandServiceLimitsV1,
) -> Result<(), CommandPublicationErrorV1> {
    if operation.limits() != limits.operation_host {
        return Err(CommandPublicationErrorV1::InvalidItem {
            field: "limits",
            message: "operation-handle limits differ from command-service limits".to_owned(),
        });
    }
    if items.len() > limits.operation_host.max_artifact_references as usize {
        return Err(CommandPublicationErrorV1::ArtifactCount {
            requested: items.len(),
            maximum: limits.operation_host.max_artifact_references,
        });
    }
    let mut destinations = BTreeSet::new();
    for item in items {
        validate_identity("label", &item.label)?;
        if !valid_uri(&item.uri) {
            return Err(CommandPublicationErrorV1::InvalidItem {
                field: "uri",
                message: format!("`{}` is not a bounded absolute URI", item.uri),
            });
        }
        if !destinations.insert(item.uri.clone()) {
            return Err(CommandPublicationErrorV1::DuplicateDestination {
                uri: item.uri.clone(),
            });
        }
        validate_identity("contentType", &item.content_type)?;
        if let Some(source_map_id) = item.source_map_id.as_deref() {
            validate_identity("sourceMapId", source_map_id)?;
        }
    }
    Ok(())
}

fn validate_identity(field: &'static str, value: &str) -> Result<(), CommandPublicationErrorV1> {
    if value.is_empty() || value.len() > MAX_IDENTITY_BYTES || value.chars().any(char::is_control) {
        return Err(CommandPublicationErrorV1::InvalidItem {
            field,
            message: "value is empty, over-bound, or contains control characters".to_owned(),
        });
    }
    Ok(())
}

fn valid_uri(uri: &str) -> bool {
    !uri.is_empty()
        && uri.len() <= MAX_SOURCE_URI_BYTES
        && !uri.chars().any(char::is_control)
        && uri_scheme(uri).is_some()
}

fn artifact_handle(
    participant: PreparedParticipant,
    metadata: RetainedHandleMetadata,
) -> CommandArtifactHandleV1 {
    CommandArtifactHandleV1 {
        handle_id: metadata.handle_id,
        kind: participant.request.kind,
        uri: participant.resolved_uri,
        content_type: participant.request.content_type,
        byte_length: participant.request.byte_length,
        sha256: participant.request.sha256,
        source_map_id: participant.request.source_map_id,
    }
}

async fn rollback_error(
    primary: CommandPublicationErrorV1,
    participants: &mut [PreparedParticipant],
) -> CommandPublicationErrorV1 {
    let failures = rollback_all(participants).await;
    if failures.is_empty() {
        primary
    } else {
        CommandPublicationErrorV1::Rollback {
            primary_code: primary.code().to_owned(),
            primary_message: primary.to_string(),
            failures,
        }
    }
}

async fn rollback_all(
    participants: &mut [PreparedParticipant],
) -> Vec<CommandPublicationParticipantFailureV1> {
    let mut failures = Vec::new();
    for participant in participants.iter_mut().rev() {
        if let Err(error) = participant.prepared.rollback().await {
            failures.push(CommandPublicationParticipantFailureV1 {
                uri: participant.request.uri.clone(),
                code: error.code,
                message: error.message,
            });
        }
    }
    failures
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::future::Future;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Wake, Waker};

    use super::*;
    use crate::capability::OperationHostLimits;
    use crate::command_service::{
        CommandPolicyStampV1, CommandRunPlanV1, CommandUriMapV1, PortableOperationRequestV1,
        COMMAND_SERVICE_PROTOCOL_VERSION,
    };
    use crate::operation_control::OperationControl;
    use crate::operation_handle::{RetainedHandleKind, RetainedHandleMetadata};
    use crate::scheduler::AbortSignal;

    const FIRST_URI: &str = "studio://output/a.txt";
    const SECOND_URI: &str = "studio://output/b.txt";

    #[derive(Default)]
    struct FixtureWriterState {
        events: Vec<String>,
        staged: BTreeMap<String, Vec<u8>>,
        committed: BTreeMap<String, Vec<u8>>,
    }

    struct FixtureWriter {
        state: Arc<Mutex<FixtureWriterState>>,
        prepare_failure: Option<String>,
        commit_failure: Option<String>,
        rollback_failure: Option<String>,
    }

    impl FixtureWriter {
        fn new(state: Arc<Mutex<FixtureWriterState>>) -> Self {
            Self {
                state,
                prepare_failure: None,
                commit_failure: None,
                rollback_failure: None,
            }
        }

        fn failing_prepare(mut self, uri: &str) -> Self {
            self.prepare_failure = Some(uri.to_owned());
            self
        }

        fn failing_commit(mut self, uri: &str) -> Self {
            self.commit_failure = Some(uri.to_owned());
            self
        }

        fn failing_rollback(mut self, uri: &str) -> Self {
            self.rollback_failure = Some(uri.to_owned());
            self
        }
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
            let result = {
                let mut state = self.state.lock().expect("fixture writer mutex");
                state.events.push(format!("prepare:{}", request.uri));
                if self.prepare_failure.as_deref() == Some(request.uri.as_str()) {
                    Err(CommandPublicationHostFailureV1::new(
                        "fixture.prepare",
                        "fixture prepare failure",
                    ))
                } else {
                    state.staged.insert(request.uri.clone(), bytes.to_vec());
                    Ok(Box::new(FixturePreparedWrite {
                        uri: request.uri,
                        state: Arc::clone(&self.state),
                        commit_failure: self.commit_failure.clone(),
                        rollback_failure: self.rollback_failure.clone(),
                    })
                        as Box<dyn CommandPreparedResourceWriteV1>)
                }
            };
            Box::pin(std::future::ready(result))
        }
    }

    struct FixturePreparedWrite {
        uri: String,
        state: Arc<Mutex<FixtureWriterState>>,
        commit_failure: Option<String>,
        rollback_failure: Option<String>,
    }

    impl CommandPreparedResourceWriteV1 for FixturePreparedWrite {
        fn commit<'a>(
            &'a mut self,
        ) -> CommandHostFuture<'a, Result<CommandResolvedWriteV1, CommandPublicationHostFailureV1>>
        {
            let result = {
                let mut state = self.state.lock().expect("fixture writer mutex");
                state.events.push(format!("commit:{}", self.uri));
                if self.commit_failure.as_deref() == Some(self.uri.as_str()) {
                    Err(CommandPublicationHostFailureV1::new(
                        "fixture.commit",
                        "fixture commit failure",
                    ))
                } else {
                    let bytes = state
                        .staged
                        .remove(&self.uri)
                        .expect("fixture staged bytes");
                    state.committed.insert(self.uri.clone(), bytes);
                    Ok(CommandResolvedWriteV1 {
                        uri: self.uri.clone(),
                    })
                }
            };
            Box::pin(std::future::ready(result))
        }

        fn rollback<'a>(
            &'a mut self,
        ) -> CommandHostFuture<'a, Result<(), CommandPublicationHostFailureV1>> {
            let result = {
                let mut state = self.state.lock().expect("fixture writer mutex");
                state.events.push(format!("rollback:{}", self.uri));
                if self.rollback_failure.as_deref() == Some(self.uri.as_str()) {
                    Err(CommandPublicationHostFailureV1::new(
                        "fixture.rollback",
                        "fixture rollback failure",
                    ))
                } else {
                    state.staged.remove(&self.uri);
                    state.committed.remove(&self.uri);
                    Ok(())
                }
            };
            Box::pin(std::future::ready(result))
        }
    }

    struct FixtureLedger {
        current: CommandRevisionLedgerV1,
        state: Arc<Mutex<FixtureWriterState>>,
        abort_after_read: Option<AbortSignal>,
    }

    impl CommandRevisionLedgerReaderV1 for FixtureLedger {
        fn current<'a>(
            &'a self,
            _request: CommandRevisionLedgerRequestV1,
        ) -> CommandHostFuture<'a, Result<CommandRevisionLedgerV1, CommandPublicationHostFailureV1>>
        {
            self.state
                .lock()
                .expect("fixture writer mutex")
                .events
                .push("ledger".to_owned());
            if let Some(signal) = &self.abort_after_read {
                signal.abort();
            }
            Box::pin(std::future::ready(Ok(self.current.clone())))
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
        CommandServiceRequestV1 {
            protocol_version: COMMAND_SERVICE_PROTOCOL_VERSION,
            request_id: "request:publication".to_owned(),
            project: CommandProjectRevisionV1 {
                project_id: "catalog".to_owned(),
                revision: 7,
            },
            resource_versions: CommandUriMapV1::new(),
            operation: PortableOperationRequestV1::VersionCapabilities,
            run_plan: CommandRunPlanV1::Null(()),
            resources: CommandUriMapV1::new(),
            policy_stamp: CommandPolicyStampV1 {
                resolver: "resolver:fixture".to_owned(),
                safety: "safety:fixture".to_owned(),
                budget: "budget:fixture".to_owned(),
            },
        }
    }

    fn ledger(request: &CommandServiceRequestV1) -> CommandRevisionLedgerV1 {
        CommandRevisionLedgerV1 {
            project: request.project.clone(),
            resource_versions: request.resource_versions.clone(),
        }
    }

    fn item(uri: &str, label: &str, bytes: &[u8]) -> CommandPublicationItemV1 {
        CommandPublicationItemV1 {
            label: label.to_owned(),
            uri: uri.to_owned(),
            kind: CommandArtifactKindV1::Output,
            purpose: ResolvePurpose::Output,
            content_type: "text/plain".to_owned(),
            bytes: bytes.to_vec(),
            source_map_id: None,
        }
    }

    fn fixture_operation(
        limits: CommandServiceLimitsV1,
        signal: &AbortSignal,
    ) -> OperationHandle<()> {
        OperationHandle::new(OperationControl::new(signal.clone()), limits.operation_host)
            .expect("fixture operation handle")
            .0
    }

    fn fixture_ledger(
        request: &CommandServiceRequestV1,
        state: Arc<Mutex<FixtureWriterState>>,
    ) -> FixtureLedger {
        FixtureLedger {
            current: ledger(request),
            state,
            abort_after_read: None,
        }
    }

    #[test]
    fn publication_is_uri_sorted_refreshes_before_commit_and_retains_bytes() {
        let request = request();
        let limits = CommandServiceLimitsV1::default();
        let signal = AbortSignal::new();
        let operation = fixture_operation(limits, &signal);
        let state = Arc::new(Mutex::new(FixtureWriterState::default()));
        let writer = FixtureWriter::new(Arc::clone(&state));
        let ledger = fixture_ledger(&request, Arc::clone(&state));

        let published = block_on(publish_command_artifacts_v1(
            &request,
            vec![
                item(SECOND_URI, "second", b"two"),
                item(FIRST_URI, "first", b"one"),
            ],
            &ledger,
            &writer,
            &operation,
            limits,
        ))
        .expect("publication succeeds");
        let CommandPublicationV1::Published(handles) = published else {
            panic!("expected published artifacts")
        };
        assert_eq!(
            state.lock().unwrap().events,
            [
                format!("prepare:{FIRST_URI}"),
                format!("prepare:{SECOND_URI}"),
                "ledger".to_owned(),
                format!("commit:{FIRST_URI}"),
                format!("commit:{SECOND_URI}"),
            ]
        );
        assert_eq!(handles.len(), 2);
        assert_eq!(handles[0].uri.as_deref(), Some(FIRST_URI));
        assert_eq!(handles[0].sha256, sha256_hex(b"one"));
        let metadata = RetainedHandleMetadata {
            operation_id: operation.operation_id(),
            handle_id: handles[0].handle_id,
            kind: RetainedHandleKind::Artifact,
            label: "first".to_owned(),
        };
        assert_eq!(
            operation
                .resolve_retained::<Vec<u8>>(&metadata, RetainedHandleKind::Artifact)
                .unwrap()
                .as_slice(),
            b"one"
        );
    }

    #[test]
    fn duplicate_destination_fails_before_host_calls() {
        let request = request();
        let limits = CommandServiceLimitsV1::default();
        let signal = AbortSignal::new();
        let operation = fixture_operation(limits, &signal);
        let state = Arc::new(Mutex::new(FixtureWriterState::default()));
        let writer = FixtureWriter::new(Arc::clone(&state));
        let ledger = fixture_ledger(&request, Arc::clone(&state));

        let error = block_on(publish_command_artifacts_v1(
            &request,
            vec![
                item(FIRST_URI, "one", b"one"),
                item(FIRST_URI, "two", b"two"),
            ],
            &ledger,
            &writer,
            &operation,
            limits,
        ))
        .expect_err("duplicate destination rejects publication");
        assert_eq!(
            error.code(),
            "cem.command_service.publication_destination_duplicate"
        );
        assert!(state.lock().unwrap().events.is_empty());
    }

    #[test]
    fn prepare_failure_rolls_back_earlier_staged_participants() {
        let request = request();
        let limits = CommandServiceLimitsV1::default();
        let signal = AbortSignal::new();
        let operation = fixture_operation(limits, &signal);
        let state = Arc::new(Mutex::new(FixtureWriterState::default()));
        let writer = FixtureWriter::new(Arc::clone(&state)).failing_prepare(SECOND_URI);
        let ledger = fixture_ledger(&request, Arc::clone(&state));

        let error = block_on(publish_command_artifacts_v1(
            &request,
            vec![
                item(SECOND_URI, "two", b"two"),
                item(FIRST_URI, "one", b"one"),
            ],
            &ledger,
            &writer,
            &operation,
            limits,
        ))
        .expect_err("prepare failure rejects publication");
        assert_eq!(error.code(), "cem.command_service.publication_prepare");
        let state = state.lock().unwrap();
        assert_eq!(
            state.events,
            [
                format!("prepare:{FIRST_URI}"),
                format!("prepare:{SECOND_URI}"),
                format!("rollback:{FIRST_URI}"),
            ]
        );
        assert!(state.staged.is_empty());
        assert!(state.committed.is_empty());
    }

    #[test]
    fn stale_and_cancelled_precommit_state_roll_back_without_committing() {
        let request = request();
        let limits = CommandServiceLimitsV1::default();
        let signal = AbortSignal::new();
        let operation = fixture_operation(limits, &signal);
        let stale_state = Arc::new(Mutex::new(FixtureWriterState::default()));
        let stale_writer = FixtureWriter::new(Arc::clone(&stale_state));
        let mut stale_ledger = fixture_ledger(&request, Arc::clone(&stale_state));
        stale_ledger.current.project.revision += 1;

        let outcome = block_on(publish_command_artifacts_v1(
            &request,
            vec![item(FIRST_URI, "one", b"one")],
            &stale_ledger,
            &stale_writer,
            &operation,
            limits,
        ))
        .expect("stale is a publication outcome");
        assert!(matches!(outcome, CommandPublicationV1::Stale(_)));
        assert_eq!(
            stale_state.lock().unwrap().events,
            [
                format!("prepare:{FIRST_URI}"),
                "ledger".to_owned(),
                format!("rollback:{FIRST_URI}"),
            ]
        );

        let cancel_signal = AbortSignal::new();
        let cancel_operation = fixture_operation(limits, &cancel_signal);
        let cancel_state = Arc::new(Mutex::new(FixtureWriterState::default()));
        let cancel_writer = FixtureWriter::new(Arc::clone(&cancel_state));
        let mut cancel_ledger = fixture_ledger(&request, Arc::clone(&cancel_state));
        cancel_ledger.abort_after_read = Some(cancel_signal);
        let error = block_on(publish_command_artifacts_v1(
            &request,
            vec![item(FIRST_URI, "one", b"one")],
            &cancel_ledger,
            &cancel_writer,
            &cancel_operation,
            limits,
        ))
        .expect_err("cancelled precommit state rejects publication");
        assert_eq!(error.code(), "host-cancellation");
        assert_eq!(
            cancel_state.lock().unwrap().events,
            [
                format!("prepare:{FIRST_URI}"),
                "ledger".to_owned(),
                format!("rollback:{FIRST_URI}"),
            ]
        );
    }

    #[test]
    fn later_commit_failure_rolls_back_committed_participant_and_reserved_handles() {
        let request = request();
        let mut limits = CommandServiceLimitsV1::default();
        limits.operation_host = OperationHostLimits {
            max_retained_handles: 2,
            ..limits.operation_host
        };
        let signal = AbortSignal::new();
        let operation = fixture_operation(limits, &signal);
        let state = Arc::new(Mutex::new(FixtureWriterState::default()));
        let writer = FixtureWriter::new(Arc::clone(&state)).failing_commit(SECOND_URI);
        let ledger = fixture_ledger(&request, Arc::clone(&state));

        let error = block_on(publish_command_artifacts_v1(
            &request,
            vec![
                item(FIRST_URI, "one", b"one"),
                item(SECOND_URI, "two", b"two"),
            ],
            &ledger,
            &writer,
            &operation,
            limits,
        ))
        .expect_err("commit failure rejects publication");
        assert_eq!(error.code(), "cem.command_service.publication_commit");
        let state = state.lock().unwrap();
        assert_eq!(
            state.events,
            [
                format!("prepare:{FIRST_URI}"),
                format!("prepare:{SECOND_URI}"),
                "ledger".to_owned(),
                format!("commit:{FIRST_URI}"),
                format!("commit:{SECOND_URI}"),
                format!("rollback:{SECOND_URI}"),
                format!("rollback:{FIRST_URI}"),
            ]
        );
        assert!(state.staged.is_empty());
        assert!(state.committed.is_empty());
        drop(state);
        operation
            .retain_artifact("probe", vec![9_u8])
            .expect("failed publication released reserved handle capacity");
    }

    #[test]
    fn rollback_failure_is_a_distinct_terminal_publication_error() {
        let request = request();
        let limits = CommandServiceLimitsV1::default();
        let signal = AbortSignal::new();
        let operation = fixture_operation(limits, &signal);
        let state = Arc::new(Mutex::new(FixtureWriterState::default()));
        let writer = FixtureWriter::new(Arc::clone(&state))
            .failing_commit(SECOND_URI)
            .failing_rollback(FIRST_URI);
        let ledger = fixture_ledger(&request, Arc::clone(&state));

        let error = block_on(publish_command_artifacts_v1(
            &request,
            vec![
                item(FIRST_URI, "one", b"one"),
                item(SECOND_URI, "two", b"two"),
            ],
            &ledger,
            &writer,
            &operation,
            limits,
        ))
        .expect_err("rollback failure rejects publication");
        let CommandPublicationErrorV1::Rollback {
            primary_code,
            failures,
            ..
        } = error
        else {
            panic!("expected rollback failure")
        };
        assert_eq!(primary_code, "cem.command_service.publication_commit");
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].uri, FIRST_URI);
        assert!(state.lock().unwrap().committed.contains_key(FIRST_URI));
    }
}
