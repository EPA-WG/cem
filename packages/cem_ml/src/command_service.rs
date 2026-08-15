//! Versioned host-neutral command-service wire contract.
//!
//! Common Rust owns these serializable request/result projections and admission
//! rules. Live resolver callbacks, revision ledgers, abort signals, streams,
//! terminal presentation, and process behavior remain host-owned values outside
//! the wire request.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::capability::{
    CapabilityManifest, CapabilityOperation, OperationHostLimits, ProductVersion, RuntimeKind,
    MAX_ARTIFACT_REFERENCES, MAX_IDENTITY_BYTES, MAX_TERMINAL_DIAGNOSTICS,
};
use crate::diagnostics::Diagnostic;
use crate::engine::{
    CheckResponse, ConvertResponse, FormatIdentity, InspectResponse, InspectView, LayerFormat,
    ParseProjection, ParseResponse, TraceProjection, TraceResponse, TransformGraphResponse,
    TransformResponse, TransformTemplateEntrypoint, ValidateProjection, ValidateResponse,
};
use crate::operation_control::MAX_SOURCE_URI_BYTES;
use crate::operation_handle::{BoundedList, RetainedHandleId};
use crate::query::{QueryExportFormat, QueryLanguage};
use crate::report::Report;
use crate::resolver::has_uri_scheme;
use crate::run_config::{NormalizedByteSourceKind, NormalizedRootScope, NormalizedRunPlan};
use crate::source_map::SourceMapStack;
use crate::worker_control::{
    WorkerCoordinatorLimits, MAX_TRANSFER_BUFFERS_PER_MESSAGE, MAX_TRANSFER_BYTES_PER_MESSAGE,
};

pub const COMMAND_SERVICE_PROTOCOL_VERSION: u16 = 1;
pub const MAX_JSON_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
pub const SHA256_HEX_BYTES: usize = 64;

/// URI-keyed wire map that rejects duplicate object keys while decoding JSON.
/// A transparent wrapper preserves the exact object shape on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "typescript-projections",
    ts(type = "{ [key in string]: T }", bound = "T: ts_rs::TS")
)]
#[serde(transparent)]
pub struct CommandUriMapV1<T>(BTreeMap<String, T>);

impl<T> CommandUriMapV1<T> {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn into_inner(self) -> BTreeMap<String, T> {
        self.0
    }
}

impl<T> Default for CommandUriMapV1<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<BTreeMap<String, T>> for CommandUriMapV1<T> {
    fn from(value: BTreeMap<String, T>) -> Self {
        Self(value)
    }
}

impl<T> FromIterator<(String, T)> for CommandUriMapV1<T> {
    fn from_iter<I: IntoIterator<Item = (String, T)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T> Deref for CommandUriMapV1<T> {
    type Target = BTreeMap<String, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for CommandUriMapV1<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'de, T> Deserialize<'de> for CommandUriMapV1<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct UriMapVisitor<T>(PhantomData<T>);

        impl<'de, T> Visitor<'de> for UriMapVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = CommandUriMapV1<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a URI-keyed object without duplicate keys")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut entries = BTreeMap::new();
                while let Some((uri, value)) = map.next_entry::<String, T>()? {
                    if entries.insert(uri.clone(), value).is_some() {
                        return Err(serde::de::Error::custom(format!(
                            "duplicate command resource URI `{uri}`"
                        )));
                    }
                }
                Ok(CommandUriMapV1(entries))
            }
        }

        deserializer.deserialize_map(UriMapVisitor(PhantomData))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandProjectRevisionV1 {
    pub project_id: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandResourceVersionV1 {
    pub revision: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct VirtualResourceV1 {
    pub bytes: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<FormatIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandPolicyStampV1 {
    pub resolver: String,
    pub safety: String,
    pub budget: String,
}

/// Required `runPlan` field that can carry a plan or an explicit wire `null`.
/// Unlike `Option`, a missing field fails deserialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(untagged)]
pub enum CommandRunPlanV1 {
    Plan(Box<NormalizedRunPlan>),
    Null(()),
}

impl CommandRunPlanV1 {
    pub fn plan(&self) -> Option<&NormalizedRunPlan> {
        match self {
            Self::Plan(plan) => Some(plan),
            Self::Null(()) => None,
        }
    }
}

impl From<NormalizedRunPlan> for CommandRunPlanV1 {
    fn from(value: NormalizedRunPlan) -> Self {
        Self::Plan(Box::new(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum CommandTransformSourceV1 {
    Direct {
        data_input_id: String,
        template_uri: String,
    },
    Graph {
        config_uri: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum PortableOperationRequestV1 {
    Parse {
        input_id: String,
        projection: ParseProjection,
        preserve_source_offsets: bool,
    },
    Validate {
        input_ids: Vec<String>,
        projection: ValidateProjection,
    },
    Check {
        input_ids: Vec<String>,
        projection: ValidateProjection,
        zero_hard_violations: bool,
    },
    Inspect {
        input_id: String,
        show: InspectView,
    },
    Convert {
        input_id: String,
        to_format: LayerFormat,
        preserve_source_offsets: bool,
    },
    Query {
        data_input_id: String,
        query_uri: String,
        output: QueryExportFormat,
    },
    Transform {
        source: CommandTransformSourceV1,
        #[serde(default)]
        params: BTreeMap<String, Value>,
        #[serde(default)]
        template_entrypoint: TransformTemplateEntrypoint,
        preserve_source_offsets: bool,
    },
    Trace {
        input_id: String,
        projection: TraceProjection,
    },
    VersionCapabilities,
}

impl PortableOperationRequestV1 {
    pub const fn operation(&self) -> CapabilityOperation {
        match self {
            Self::Parse { .. } => CapabilityOperation::Parse,
            Self::Validate { .. } => CapabilityOperation::Validate,
            Self::Check { .. } => CapabilityOperation::Check,
            Self::Inspect { .. } => CapabilityOperation::Inspect,
            Self::Convert { .. } => CapabilityOperation::Convert,
            Self::Query { .. } => CapabilityOperation::Query,
            Self::Transform { .. } => CapabilityOperation::Transform,
            Self::Trace { .. } => CapabilityOperation::Trace,
            Self::VersionCapabilities => CapabilityOperation::VersionCapabilities,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandServiceRequestV1 {
    pub protocol_version: u16,
    pub request_id: String,
    pub project: CommandProjectRevisionV1,
    pub resource_versions: CommandUriMapV1<CommandResourceVersionV1>,
    pub operation: PortableOperationRequestV1,
    pub run_plan: CommandRunPlanV1,
    pub resources: CommandUriMapV1<VirtualResourceV1>,
    pub policy_stamp: CommandPolicyStampV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum CommandArtifactKindV1 {
    Output,
    Report,
    SourceMap,
    Trace,
    Graph,
    Variables,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandArtifactHandleV1 {
    pub handle_id: RetainedHandleId,
    pub kind: CommandArtifactKindV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    pub content_type: String,
    pub byte_length: u64,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_map_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(
    tag = "storage",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum CommandPayloadV1<T> {
    Inline { value: T },
    Artifact { handle: CommandArtifactHandleV1 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum CommandSourceMapOwnerV1 {
    Result,
    Artifact { handle_id: RetainedHandleId },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandSourceMapReferenceV1 {
    pub source_map_id: String,
    pub owner: CommandSourceMapOwnerV1,
    pub source_map: CommandPayloadV1<SourceMapStack>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum CommandServiceStatusV1 {
    Succeeded,
    Failed,
    Cancelled,
    Fatal,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandExecutionIdentityV1 {
    pub common_version: String,
    pub runtime: RuntimeKind,
    pub target_identity: String,
    pub abi_identity: String,
    #[serde(default)]
    pub schema_package_versions: BTreeMap<String, String>,
    pub resolver_policy_stamp: String,
    pub safety_policy_stamp: String,
    pub budget_policy_stamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandChangedResourceV1 {
    pub uri: String,
    pub revision: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandStaleRevisionV1 {
    pub current_project_revision: u64,
    pub changed_resources: Vec<CommandChangedResourceV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandQueryResultV1 {
    pub language: QueryLanguage,
    pub inputs: [String; 2],
    pub output: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandOutputResultV1<T> {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    pub response: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandFanoutResultV1<T> {
    pub outputs: BoundedList<CommandOutputResultV1<T>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum CommandTransformResultV1 {
    Direct(CommandFanoutResultV1<TransformResponse>),
    Graph(TransformGraphResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandVersionCapabilitiesResultV1 {
    pub version: ProductVersion,
    pub capability: CapabilityManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum PortableOperationResultV1 {
    Parse(ParseResponse),
    Validate(ValidateResponse),
    Check(CheckResponse),
    Inspect(InspectResponse),
    Convert(CommandFanoutResultV1<ConvertResponse>),
    Query(CommandQueryResultV1),
    Transform(CommandTransformResultV1),
    Trace(TraceResponse),
    VersionCapabilities(CommandVersionCapabilitiesResultV1),
}

impl PortableOperationResultV1 {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandServiceResultV1 {
    pub protocol_version: u16,
    pub request_id: String,
    pub project: CommandProjectRevisionV1,
    pub resource_versions: CommandUriMapV1<CommandResourceVersionV1>,
    pub operation: CapabilityOperation,
    pub status: CommandServiceStatusV1,
    #[serde(default)]
    pub exit_code: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<CommandPayloadV1<PortableOperationResultV1>>,
    pub diagnostics: BoundedList<Diagnostic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<CommandPayloadV1<Report>>,
    pub artifacts: BoundedList<CommandArtifactHandleV1>,
    pub source_maps: BoundedList<CommandSourceMapReferenceV1>,
    pub identity: CommandExecutionIdentityV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale: Option<CommandStaleRevisionV1>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CommandServiceLimitsV1 {
    pub operation_host: OperationHostLimits,
    pub worker: WorkerCoordinatorLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CommandRevisionLedgerV1 {
    pub project: CommandProjectRevisionV1,
    pub resource_versions: CommandUriMapV1<CommandResourceVersionV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandServiceAdmissionV1 {
    Accepted,
    Stale(CommandStaleRevisionV1),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandServiceError {
    Decode(String),
    ProtocolVersion {
        requested: u16,
        supported: u16,
    },
    InvalidLimit {
        field: &'static str,
        requested: u64,
        maximum: u64,
    },
    InvalidIdentity {
        field: String,
    },
    RevisionOutOfRange {
        field: String,
        revision: u64,
    },
    InvalidUri {
        field: String,
        uri: String,
    },
    ResourceVersionCount {
        requested: usize,
        maximum: u32,
    },
    ResourceCount {
        requested: usize,
        maximum: u16,
    },
    ResourceBytes {
        requested: u64,
        maximum: u64,
    },
    ResourceVersionMissing {
        uri: String,
    },
    ResourceDigestInvalid {
        uri: String,
    },
    ResourceDigestMismatch {
        uri: String,
    },
    RunPlanRequired,
    RunPlanUnexpected,
    OperationInputsEmpty,
    DuplicateInputId {
        input_id: String,
    },
    UnknownInputId {
        input_id: String,
    },
    TransformGraphStageLocal {
        field: &'static str,
    },
    OperationMetadataTooLarge {
        requested: usize,
        maximum: usize,
    },
    RunPlanMetadataTooLarge {
        requested: usize,
        maximum: usize,
    },
    RunPlanHostState {
        field: &'static str,
    },
    LedgerProjectMismatch,
    LedgerResourceMissing {
        uri: String,
    },
    ResultContract {
        reason: &'static str,
    },
    Serialization(String),
}

impl CommandServiceError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Decode(_) => "cem.command_service.decode",
            Self::ProtocolVersion { .. } => "cem.command_service.protocol_version",
            Self::InvalidLimit { .. } => "cem.command_service.limit_invalid",
            Self::InvalidIdentity { .. } => "cem.command_service.identity_invalid",
            Self::RevisionOutOfRange { .. } => "cem.command_service.revision_out_of_range",
            Self::InvalidUri { .. } => "cem.command_service.uri_invalid",
            Self::ResourceVersionCount { .. } => "cem.command_service.resource_version_count",
            Self::ResourceCount { .. } => "cem.command_service.resource_count",
            Self::ResourceBytes { .. } => "cem.command_service.resource_bytes",
            Self::ResourceVersionMissing { .. } => "cem.command_service.resource_version_missing",
            Self::ResourceDigestInvalid { .. } => "cem.command_service.resource_digest_invalid",
            Self::ResourceDigestMismatch { .. } => "cem.command_service.resource_digest_mismatch",
            Self::RunPlanRequired => "cem.command_service.run_plan_required",
            Self::RunPlanUnexpected => "cem.command_service.run_plan_unexpected",
            Self::OperationInputsEmpty => "cem.command_service.operation_inputs_empty",
            Self::DuplicateInputId { .. } => "cem.command_service.input_id_duplicate",
            Self::UnknownInputId { .. } => "cem.command_service.input_id_unknown",
            Self::TransformGraphStageLocal { .. } => {
                "cem.command_service.transform_graph_stage_local"
            }
            Self::OperationMetadataTooLarge { .. } => {
                "cem.command_service.operation_metadata_too_large"
            }
            Self::RunPlanMetadataTooLarge { .. } => {
                "cem.command_service.run_plan_metadata_too_large"
            }
            Self::RunPlanHostState { .. } => "cem.command_service.run_plan_host_state",
            Self::LedgerProjectMismatch => "cem.command_service.ledger_project_mismatch",
            Self::LedgerResourceMissing { .. } => "cem.command_service.ledger_resource_missing",
            Self::ResultContract { .. } => "cem.command_service.result_invalid",
            Self::Serialization(_) => "cem.command_service.serialization_failed",
        }
    }
}

impl fmt::Display for CommandServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(message) | Self::Serialization(message) => formatter.write_str(message),
            Self::ProtocolVersion {
                requested,
                supported,
            } => write!(
                formatter,
                "command-service protocol version {requested} is unsupported; expected {supported}"
            ),
            Self::InvalidLimit {
                field,
                requested,
                maximum,
            } => write!(
                formatter,
                "command-service limit {field}={requested} is outside 1..={maximum}"
            ),
            Self::InvalidIdentity { field } => write!(
                formatter,
                "command-service identity {field} is empty, over-bound, or contains control characters"
            ),
            Self::RevisionOutOfRange { field, revision } => write!(
                formatter,
                "command-service revision {field}={revision} exceeds the JSON-safe integer range"
            ),
            Self::InvalidUri { field, uri } => write!(
                formatter,
                "command-service URI {field}=`{uri}` is invalid or over-bound"
            ),
            Self::ResourceVersionCount { requested, maximum } => write!(
                formatter,
                "command-service request has {requested} resource versions, exceeding {maximum}"
            ),
            Self::ResourceCount { requested, maximum } => write!(
                formatter,
                "command-service request has {requested} inline resources, exceeding {maximum}"
            ),
            Self::ResourceBytes { requested, maximum } => write!(
                formatter,
                "command-service inline resources contain {requested} bytes, exceeding {maximum}"
            ),
            Self::ResourceVersionMissing { uri } => write!(
                formatter,
                "command-service resource `{uri}` has no declared version snapshot"
            ),
            Self::ResourceDigestInvalid { uri } => write!(
                formatter,
                "command-service resource `{uri}` has an invalid SHA-256 digest"
            ),
            Self::ResourceDigestMismatch { uri } => write!(
                formatter,
                "command-service resource `{uri}` bytes do not match its SHA-256 digest"
            ),
            Self::RunPlanRequired => formatter.write_str(
                "command-service operation requires a normalized run plan",
            ),
            Self::RunPlanUnexpected => formatter.write_str(
                "version-capabilities must carry an explicit null run plan",
            ),
            Self::OperationInputsEmpty => {
                formatter.write_str("command-service operation input list must not be empty")
            }
            Self::DuplicateInputId { input_id } => write!(
                formatter,
                "normalized run plan contains duplicate input id `{input_id}`"
            ),
            Self::UnknownInputId { input_id } => write!(
                formatter,
                "command-service operation references unknown input id `{input_id}`"
            ),
            Self::TransformGraphStageLocal { field } => write!(
                formatter,
                "command-service transform graph field {field} must be empty or implicit; graph stages own params and entrypoints"
            ),
            Self::OperationMetadataTooLarge { requested, maximum } => write!(
                formatter,
                "command-service operation metadata is {requested} bytes, exceeding {maximum}"
            ),
            Self::RunPlanMetadataTooLarge { requested, maximum } => write!(
                formatter,
                "command-service normalized run plan is {requested} bytes, exceeding {maximum}"
            ),
            Self::RunPlanHostState { field } => write!(
                formatter,
                "command-service normalized run plan field {field} contains live host state"
            ),
            Self::LedgerProjectMismatch => formatter.write_str(
                "command-service revision ledger belongs to a different project",
            ),
            Self::LedgerResourceMissing { uri } => write!(
                formatter,
                "command-service revision ledger has no current entry for `{uri}`"
            ),
            Self::ResultContract { reason } => {
                write!(formatter, "command-service result is invalid: {reason}")
            }
        }
    }
}

impl std::error::Error for CommandServiceError {}

pub fn decode_command_service_request_v1(
    bytes: &[u8],
) -> Result<CommandServiceRequestV1, CommandServiceError> {
    serde_json::from_slice(bytes).map_err(|error| CommandServiceError::Decode(error.to_string()))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(SHA256_HEX_BYTES);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

pub fn validate_command_service_request_v1(
    request: &CommandServiceRequestV1,
    limits: CommandServiceLimitsV1,
) -> Result<(), CommandServiceError> {
    validate_limits(limits)?;
    if request.protocol_version != COMMAND_SERVICE_PROTOCOL_VERSION {
        return Err(CommandServiceError::ProtocolVersion {
            requested: request.protocol_version,
            supported: COMMAND_SERVICE_PROTOCOL_VERSION,
        });
    }
    validate_identity("requestId", &request.request_id)?;
    validate_identity("project.projectId", &request.project.project_id)?;
    validate_revision("project.revision", request.project.revision)?;
    validate_identity("policyStamp.resolver", &request.policy_stamp.resolver)?;
    validate_identity("policyStamp.safety", &request.policy_stamp.safety)?;
    validate_identity("policyStamp.budget", &request.policy_stamp.budget)?;
    validate_resource_versions(&request.resource_versions, limits)?;
    validate_inline_resources(request, limits)?;
    validate_operation_metadata(&request.operation)?;
    validate_run_plan(request, limits)?;
    Ok(())
}

pub fn admit_command_service_request_v1(
    request: &CommandServiceRequestV1,
    ledger: &CommandRevisionLedgerV1,
    limits: CommandServiceLimitsV1,
) -> Result<CommandServiceAdmissionV1, CommandServiceError> {
    validate_command_service_request_v1(request, limits)?;
    validate_identity("ledger.project.projectId", &ledger.project.project_id)?;
    validate_revision("ledger.project.revision", ledger.project.revision)?;
    validate_resource_versions(&ledger.resource_versions, limits)?;
    if request.project.project_id != ledger.project.project_id {
        return Err(CommandServiceError::LedgerProjectMismatch);
    }

    let mut changed_resources = Vec::new();
    for (uri, requested) in request.resource_versions.iter() {
        let current = ledger
            .resource_versions
            .get(uri)
            .ok_or_else(|| CommandServiceError::LedgerResourceMissing { uri: uri.clone() })?;
        if requested != current {
            changed_resources.push(CommandChangedResourceV1 {
                uri: uri.clone(),
                revision: current.revision,
                sha256: current.sha256.clone(),
            });
        }
    }

    if request.project.revision == ledger.project.revision && changed_resources.is_empty() {
        Ok(CommandServiceAdmissionV1::Accepted)
    } else {
        Ok(CommandServiceAdmissionV1::Stale(CommandStaleRevisionV1 {
            current_project_revision: ledger.project.revision,
            changed_resources,
        }))
    }
}

pub fn validate_command_service_result_v1(
    result: &CommandServiceResultV1,
    limits: CommandServiceLimitsV1,
) -> Result<(), CommandServiceError> {
    validate_limits(limits)?;
    if result.protocol_version != COMMAND_SERVICE_PROTOCOL_VERSION {
        return Err(CommandServiceError::ProtocolVersion {
            requested: result.protocol_version,
            supported: COMMAND_SERVICE_PROTOCOL_VERSION,
        });
    }
    validate_identity("requestId", &result.request_id)?;
    validate_identity("project.projectId", &result.project.project_id)?;
    validate_revision("project.revision", result.project.revision)?;
    validate_resource_versions(&result.resource_versions, limits)?;
    validate_result_status(result, limits)?;
    validate_bounded_list(
        &result.diagnostics,
        limits.operation_host.max_terminal_diagnostics,
        "diagnostics exceed the negotiated limit",
    )?;
    validate_bounded_list(
        &result.artifacts,
        limits.operation_host.max_artifact_references,
        "artifacts exceed the negotiated limit",
    )?;
    validate_bounded_list(
        &result.source_maps,
        limits.operation_host.max_artifact_references,
        "source maps exceed the negotiated limit",
    )?;
    validate_execution_identity(&result.identity, limits)?;
    let mut artifact_ids = BTreeSet::new();
    for artifact in &result.artifacts.items {
        validate_command_artifact_handle_v1(artifact)?;
        if !artifact_ids.insert(artifact.handle_id) {
            return Err(CommandServiceError::ResultContract {
                reason: "artifacts contain a duplicate handle id",
            });
        }
    }
    let mut source_map_ids = BTreeSet::new();
    for source_map in &result.source_maps.items {
        validate_identity("sourceMaps.sourceMapId", &source_map.source_map_id)?;
        if !source_map_ids.insert(source_map.source_map_id.as_str()) {
            return Err(CommandServiceError::ResultContract {
                reason: "source maps contain a duplicate source-map id",
            });
        }
        if let CommandSourceMapOwnerV1::Artifact { handle_id } = source_map.owner {
            if !artifact_ids.contains(&handle_id) {
                return Err(CommandServiceError::ResultContract {
                    reason: "source-map owner does not reference a published artifact",
                });
            }
        }
        if let CommandPayloadV1::Artifact { handle } = &source_map.source_map {
            if handle.kind != CommandArtifactKindV1::SourceMap {
                return Err(CommandServiceError::ResultContract {
                    reason: "artifact-backed source map must use the source-map artifact kind",
                });
            }
        }
        validate_payload(&source_map.source_map, limits)?;
    }
    for artifact in &result.artifacts.items {
        if let Some(source_map_id) = artifact.source_map_id.as_deref() {
            if !source_map_ids.contains(source_map_id) {
                return Err(CommandServiceError::ResultContract {
                    reason: "artifact source-map id does not reference a published source map",
                });
            }
        }
    }
    if let Some(payload) = &result.result {
        if let CommandPayloadV1::Inline { value } = payload {
            if value.operation() != result.operation {
                return Err(CommandServiceError::ResultContract {
                    reason: "inline result operation does not match the result envelope",
                });
            }
            validate_operation_result(value, limits)?;
        }
        validate_payload(payload, limits)?;
    }
    if let Some(report) = &result.report {
        validate_payload(report, limits)?;
    }
    Ok(())
}

fn validate_operation_result(
    result: &PortableOperationResultV1,
    limits: CommandServiceLimitsV1,
) -> Result<(), CommandServiceError> {
    match result {
        PortableOperationResultV1::Convert(result) => validate_fanout_result(result, limits),
        PortableOperationResultV1::Transform(CommandTransformResultV1::Direct(result)) => {
            validate_fanout_result(result, limits)
        }
        _ => Ok(()),
    }
}

fn validate_fanout_result<T>(
    result: &CommandFanoutResultV1<T>,
    limits: CommandServiceLimitsV1,
) -> Result<(), CommandServiceError> {
    validate_bounded_list(
        &result.outputs,
        limits.operation_host.max_artifact_references,
        "fan-out outputs exceed the negotiated limit",
    )?;
    if result.outputs.items.is_empty() || result.outputs.original_count == 0 {
        return Err(CommandServiceError::ResultContract {
            reason: "fan-out result must contain at least one output",
        });
    }
    for output in &result.outputs.items {
        if let Some(output_id) = output.output_id.as_deref() {
            validate_identity("result.outputs.outputId", output_id)?;
        }
        if let Some(destination) = output.destination.as_deref() {
            validate_uri("result.outputs.destination", destination)?;
        }
    }
    Ok(())
}

pub fn validate_command_service_limits_v1(
    limits: CommandServiceLimitsV1,
) -> Result<(), CommandServiceError> {
    validate_limits(limits)
}

fn validate_limits(limits: CommandServiceLimitsV1) -> Result<(), CommandServiceError> {
    validate_limit(
        "maxArtifactReferences",
        u64::from(limits.operation_host.max_artifact_references),
        u64::from(MAX_ARTIFACT_REFERENCES),
    )?;
    validate_limit(
        "maxTerminalDiagnostics",
        u64::from(limits.operation_host.max_terminal_diagnostics),
        u64::from(MAX_TERMINAL_DIAGNOSTICS),
    )?;
    validate_limit(
        "maxTransferBuffersPerMessage",
        u64::from(limits.worker.max_transfer_buffers_per_message),
        u64::from(MAX_TRANSFER_BUFFERS_PER_MESSAGE),
    )?;
    validate_limit(
        "maxTransferBytesPerMessage",
        limits.worker.max_transfer_bytes_per_message,
        MAX_TRANSFER_BYTES_PER_MESSAGE,
    )
}

fn validate_limit(
    field: &'static str,
    requested: u64,
    maximum: u64,
) -> Result<(), CommandServiceError> {
    if requested == 0 || requested > maximum {
        return Err(CommandServiceError::InvalidLimit {
            field,
            requested,
            maximum,
        });
    }
    Ok(())
}

fn validate_identity(field: impl Into<String>, value: &str) -> Result<(), CommandServiceError> {
    if value.is_empty() || value.len() > MAX_IDENTITY_BYTES || value.chars().any(char::is_control) {
        return Err(CommandServiceError::InvalidIdentity {
            field: field.into(),
        });
    }
    Ok(())
}

fn validate_bounded_text(
    field: impl Into<String>,
    value: &str,
    maximum: usize,
) -> Result<(), CommandServiceError> {
    if value.is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
        return Err(CommandServiceError::InvalidIdentity {
            field: field.into(),
        });
    }
    Ok(())
}

fn validate_revision(field: impl Into<String>, revision: u64) -> Result<(), CommandServiceError> {
    if revision > MAX_JSON_SAFE_INTEGER {
        return Err(CommandServiceError::RevisionOutOfRange {
            field: field.into(),
            revision,
        });
    }
    Ok(())
}

fn validate_uri(field: impl Into<String>, uri: &str) -> Result<(), CommandServiceError> {
    if uri.is_empty()
        || uri.len() > MAX_SOURCE_URI_BYTES
        || uri.chars().any(char::is_control)
        || !has_uri_scheme(uri)
    {
        return Err(CommandServiceError::InvalidUri {
            field: field.into(),
            uri: uri.to_owned(),
        });
    }
    Ok(())
}

fn validate_digest(uri: &str, digest: &str) -> Result<(), CommandServiceError> {
    if digest.len() != SHA256_HEX_BYTES
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(CommandServiceError::ResourceDigestInvalid {
            uri: uri.to_owned(),
        });
    }
    Ok(())
}

fn validate_resource_versions(
    versions: &CommandUriMapV1<CommandResourceVersionV1>,
    limits: CommandServiceLimitsV1,
) -> Result<(), CommandServiceError> {
    if versions.len() > limits.operation_host.max_artifact_references as usize {
        return Err(CommandServiceError::ResourceVersionCount {
            requested: versions.len(),
            maximum: limits.operation_host.max_artifact_references,
        });
    }
    for (uri, version) in versions.iter() {
        validate_uri("resourceVersions", uri)?;
        validate_revision(
            format!("resourceVersions[{uri}].revision"),
            version.revision,
        )?;
        validate_digest(uri, &version.sha256)?;
    }
    Ok(())
}

fn validate_inline_resources(
    request: &CommandServiceRequestV1,
    limits: CommandServiceLimitsV1,
) -> Result<(), CommandServiceError> {
    if request.resources.len() > usize::from(limits.worker.max_transfer_buffers_per_message) {
        return Err(CommandServiceError::ResourceCount {
            requested: request.resources.len(),
            maximum: limits.worker.max_transfer_buffers_per_message,
        });
    }
    let mut byte_length = 0_u64;
    for (uri, resource) in request.resources.iter() {
        validate_uri("resources", uri)?;
        if let Some(identity) = resource.identity.as_ref() {
            validate_format_identity("resources.identity", identity, limits)?;
        }
        byte_length = byte_length.saturating_add(resource.bytes.len() as u64);
        let version = request
            .resource_versions
            .get(uri)
            .ok_or_else(|| CommandServiceError::ResourceVersionMissing { uri: uri.clone() })?;
        if sha256_hex(&resource.bytes) != version.sha256 {
            return Err(CommandServiceError::ResourceDigestMismatch { uri: uri.clone() });
        }
    }
    if byte_length > limits.worker.max_transfer_bytes_per_message {
        return Err(CommandServiceError::ResourceBytes {
            requested: byte_length,
            maximum: limits.worker.max_transfer_bytes_per_message,
        });
    }
    Ok(())
}

fn validate_operation_metadata(
    operation: &PortableOperationRequestV1,
) -> Result<(), CommandServiceError> {
    let encoded = serde_json::to_vec(operation)
        .map_err(|error| CommandServiceError::Serialization(error.to_string()))?;
    let maximum = crate::worker_control::MAX_WORK_INLINE_PAYLOAD_BYTES;
    if encoded.len() > maximum {
        return Err(CommandServiceError::OperationMetadataTooLarge {
            requested: encoded.len(),
            maximum,
        });
    }

    let mut ids = Vec::new();
    match operation {
        PortableOperationRequestV1::Parse { input_id, .. }
        | PortableOperationRequestV1::Inspect { input_id, .. }
        | PortableOperationRequestV1::Convert { input_id, .. }
        | PortableOperationRequestV1::Trace { input_id, .. } => ids.push(input_id),
        PortableOperationRequestV1::Validate { input_ids, .. }
        | PortableOperationRequestV1::Check { input_ids, .. } => {
            if input_ids.is_empty() {
                return Err(CommandServiceError::OperationInputsEmpty);
            }
            ids.extend(input_ids);
        }
        PortableOperationRequestV1::Query {
            data_input_id,
            query_uri,
            ..
        } => {
            ids.push(data_input_id);
            validate_uri("operation.queryUri", query_uri)?;
        }
        PortableOperationRequestV1::Transform {
            source,
            params,
            template_entrypoint,
            ..
        } => match source {
            CommandTransformSourceV1::Direct {
                data_input_id,
                template_uri,
            } => {
                if let Some(name) = template_entrypoint.name.as_deref() {
                    validate_identity("operation.templateEntrypoint.name", name)?;
                }
                ids.push(data_input_id);
                validate_uri("operation.source.templateUri", template_uri)?;
            }
            CommandTransformSourceV1::Graph { config_uri } => {
                if !params.is_empty() {
                    return Err(CommandServiceError::TransformGraphStageLocal {
                        field: "operation.params",
                    });
                }
                if !template_entrypoint.is_implicit() {
                    return Err(CommandServiceError::TransformGraphStageLocal {
                        field: "operation.templateEntrypoint",
                    });
                }
                validate_uri("operation.source.configUri", config_uri)?;
            }
        },
        PortableOperationRequestV1::VersionCapabilities => {}
    }

    let mut unique = BTreeSet::new();
    for id in ids {
        validate_identity("operation.inputId", id)?;
        if !unique.insert(id) {
            return Err(CommandServiceError::DuplicateInputId {
                input_id: id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_run_plan(
    request: &CommandServiceRequestV1,
    limits: CommandServiceLimitsV1,
) -> Result<(), CommandServiceError> {
    if matches!(
        request.operation,
        PortableOperationRequestV1::VersionCapabilities
    ) {
        if request.run_plan.plan().is_some()
            || !request.resource_versions.is_empty()
            || !request.resources.is_empty()
        {
            return Err(CommandServiceError::RunPlanUnexpected);
        }
        return Ok(());
    }

    let plan = request
        .run_plan
        .plan()
        .ok_or(CommandServiceError::RunPlanRequired)?;
    let encoded = serde_json::to_vec(plan)
        .map_err(|error| CommandServiceError::Serialization(error.to_string()))?;
    let maximum = crate::worker_control::MAX_WORK_INLINE_PAYLOAD_BYTES;
    if encoded.len() > maximum {
        return Err(CommandServiceError::RunPlanMetadataTooLarge {
            requested: encoded.len(),
            maximum,
        });
    }
    validate_identity("runPlan.runId", &plan.run_id)?;
    if let Some(profile) = plan.command_profile.as_deref() {
        validate_identity("runPlan.commandProfile", profile)?;
    }
    let mut plan_input_ids = BTreeSet::new();
    for input in &plan.inputs {
        validate_identity("runPlan.inputs.inputId", &input.input_id)?;
        if input.byte_source_kind == NormalizedByteSourceKind::Stream {
            return Err(CommandServiceError::RunPlanHostState {
                field: "runPlan.inputs.byteSourceKind",
            });
        }
        if !plan_input_ids.insert(input.input_id.as_str()) {
            return Err(CommandServiceError::DuplicateInputId {
                input_id: input.input_id.clone(),
            });
        }
        validate_snapshot_uri(
            request,
            "runPlan.inputs.uri",
            input.resolved_uri.as_deref().unwrap_or(&input.declared_uri),
        )?;
        validate_format_identity("runPlan.inputs.identity", &input.identity, limits)?;
        validate_scope_resource(request, &input.root_scope, limits)?;
    }
    for schema_package in &plan.schema_packages {
        validate_identity(
            "runPlan.schemaPackages.schemaPackageId",
            &schema_package.schema_package_id,
        )?;
        validate_snapshot_uri(
            request,
            "runPlan.schemaPackages.uri",
            schema_package
                .resolved_uri
                .as_deref()
                .unwrap_or(&schema_package.declared_uri),
        )?;
        validate_format_identity(
            "runPlan.schemaPackages.identity",
            &schema_package.identity,
            limits,
        )?;
        validate_scope_resource(request, &schema_package.root_scope, limits)?;
    }
    for output in &plan.outputs {
        validate_identity("runPlan.outputs.outputId", &output.output_id)?;
        if let Some(input_id) = output.input_id.as_deref() {
            validate_identity("runPlan.outputs.inputId", input_id)?;
            if !plan_input_ids.contains(input_id) {
                return Err(CommandServiceError::UnknownInputId {
                    input_id: input_id.to_owned(),
                });
            }
        }
        if let Some(destination) = output.resolved_destination.as_deref() {
            validate_uri("runPlan.outputs.resolvedDestination", destination)?;
        }
        validate_format_identity("runPlan.outputs.identity", &output.identity, limits)?;
        validate_scope_resource(request, &output.root_scope, limits)?;
    }
    for resolver in &plan.resolvers {
        validate_identity("runPlan.resolvers.resolverId", &resolver.resolver_id)?;
        validate_identity("runPlan.resolvers.scheme", &resolver.scheme)?;
        validate_uri(
            "runPlan.resolvers.declaredUriPrefix",
            &resolver.declared_uri_prefix,
        )?;
        if resolver.resolved_local_root.is_some() {
            return Err(CommandServiceError::RunPlanHostState {
                field: "runPlan.resolvers.resolvedLocalRoot",
            });
        }
    }
    if let Some(uri) = plan
        .config_identity
        .resolved_uri
        .as_deref()
        .or(plan.config_identity.declared_uri.as_deref())
    {
        validate_snapshot_uri(request, "runPlan.configIdentity.uri", uri)?;
    }
    for source in &plan.authored_sources {
        validate_identity("runPlan.authoredSources.sourceId", &source.source_id)?;
        validate_format_identity("runPlan.authoredSources.identity", &source.identity, limits)?;
        if let Some(uri) = source
            .resolved_uri
            .as_deref()
            .or(source.declared_uri.as_deref())
        {
            validate_snapshot_uri(request, "runPlan.authoredSources.uri", uri)?;
        }
    }

    let referenced_ids = operation_input_ids(&request.operation);
    for input_id in referenced_ids {
        if !plan_input_ids.contains(input_id) {
            return Err(CommandServiceError::UnknownInputId {
                input_id: input_id.to_owned(),
            });
        }
    }
    for uri in operation_resource_uris(&request.operation) {
        validate_snapshot_uri(request, "operation.resourceUri", uri)?;
    }
    Ok(())
}

fn validate_scope_resource(
    request: &CommandServiceRequestV1,
    scope: &NormalizedRootScope,
    limits: CommandServiceLimitsV1,
) -> Result<(), CommandServiceError> {
    validate_identity("runPlan.rootScope.scopeId", &scope.scope_id)?;
    validate_format_identity("runPlan.rootScope.identity", &scope.identity, limits)?;
    if let Some(module_map) = scope.module_map.as_ref() {
        let uri = module_map
            .resolved_uri
            .as_deref()
            .unwrap_or(&module_map.declared_uri);
        validate_snapshot_uri(request, "runPlan.rootScope.moduleMap", uri)?;
    }
    Ok(())
}

fn validate_snapshot_uri(
    request: &CommandServiceRequestV1,
    field: &str,
    uri: &str,
) -> Result<(), CommandServiceError> {
    validate_uri(field, uri)?;
    if !request.resource_versions.contains_key(uri) {
        return Err(CommandServiceError::ResourceVersionMissing {
            uri: uri.to_owned(),
        });
    }
    Ok(())
}

fn operation_input_ids(operation: &PortableOperationRequestV1) -> Vec<&str> {
    match operation {
        PortableOperationRequestV1::Parse { input_id, .. }
        | PortableOperationRequestV1::Inspect { input_id, .. }
        | PortableOperationRequestV1::Convert { input_id, .. }
        | PortableOperationRequestV1::Trace { input_id, .. } => vec![input_id],
        PortableOperationRequestV1::Validate { input_ids, .. }
        | PortableOperationRequestV1::Check { input_ids, .. } => {
            input_ids.iter().map(String::as_str).collect()
        }
        PortableOperationRequestV1::Query { data_input_id, .. } => vec![data_input_id],
        PortableOperationRequestV1::Transform {
            source: CommandTransformSourceV1::Direct { data_input_id, .. },
            ..
        } => vec![data_input_id],
        PortableOperationRequestV1::Transform {
            source: CommandTransformSourceV1::Graph { .. },
            ..
        }
        | PortableOperationRequestV1::VersionCapabilities => Vec::new(),
    }
}

fn operation_resource_uris(operation: &PortableOperationRequestV1) -> Vec<&str> {
    match operation {
        PortableOperationRequestV1::Query { query_uri, .. } => vec![query_uri],
        PortableOperationRequestV1::Transform {
            source: CommandTransformSourceV1::Direct { template_uri, .. },
            ..
        } => vec![template_uri],
        PortableOperationRequestV1::Transform {
            source: CommandTransformSourceV1::Graph { config_uri },
            ..
        } => vec![config_uri],
        _ => Vec::new(),
    }
}

fn validate_bounded_list<T>(
    list: &BoundedList<T>,
    maximum: u32,
    reason: &'static str,
) -> Result<(), CommandServiceError> {
    if list.items.len() > maximum as usize || list.original_count < list.items.len() as u32 {
        return Err(CommandServiceError::ResultContract { reason });
    }
    Ok(())
}

fn validate_result_status(
    result: &CommandServiceResultV1,
    limits: CommandServiceLimitsV1,
) -> Result<(), CommandServiceError> {
    let valid_exit = match result.status {
        CommandServiceStatusV1::Succeeded => result.exit_code == Some(0),
        CommandServiceStatusV1::Failed => {
            matches!(result.exit_code, Some(1 | 2 | 3 | 6))
        }
        CommandServiceStatusV1::Cancelled => result.exit_code == Some(130),
        CommandServiceStatusV1::Fatal => result.exit_code == Some(7),
        CommandServiceStatusV1::Stale => result.exit_code.is_none(),
    };
    if !valid_exit {
        return Err(CommandServiceError::ResultContract {
            reason: "status and exit code do not match",
        });
    }
    match result.status {
        CommandServiceStatusV1::Stale => {
            if result.stale.is_none()
                || result.result.is_some()
                || result.report.is_some()
                || !result.artifacts.items.is_empty()
                || !result.source_maps.items.is_empty()
            {
                return Err(CommandServiceError::ResultContract {
                    reason: "stale results must contain stale details and no published payloads",
                });
            }
        }
        _ if result.stale.is_some() => {
            return Err(CommandServiceError::ResultContract {
                reason: "non-stale result contains stale details",
            });
        }
        _ => {}
    }
    if let Some(stale) = result.stale.as_ref() {
        validate_revision(
            "stale.currentProjectRevision",
            stale.current_project_revision,
        )?;
        if stale.changed_resources.len() > limits.operation_host.max_artifact_references as usize {
            return Err(CommandServiceError::ResultContract {
                reason: "changed resources exceed the negotiated limit",
            });
        }
        let mut changed_uris = BTreeSet::new();
        for changed in &stale.changed_resources {
            validate_uri("stale.changedResources.uri", &changed.uri)?;
            validate_revision("stale.changedResources.revision", changed.revision)?;
            validate_digest(&changed.uri, &changed.sha256)?;
            if !changed_uris.insert(changed.uri.as_str()) {
                return Err(CommandServiceError::ResultContract {
                    reason: "changed resources contain a duplicate URI",
                });
            }
        }
    }
    Ok(())
}

fn validate_execution_identity(
    identity: &CommandExecutionIdentityV1,
    limits: CommandServiceLimitsV1,
) -> Result<(), CommandServiceError> {
    validate_identity("identity.commonVersion", &identity.common_version)?;
    validate_identity("identity.targetIdentity", &identity.target_identity)?;
    validate_identity("identity.abiIdentity", &identity.abi_identity)?;
    validate_identity(
        "identity.resolverPolicyStamp",
        &identity.resolver_policy_stamp,
    )?;
    validate_identity("identity.safetyPolicyStamp", &identity.safety_policy_stamp)?;
    validate_identity("identity.budgetPolicyStamp", &identity.budget_policy_stamp)?;
    if identity.schema_package_versions.len()
        > limits.operation_host.max_artifact_references as usize
    {
        return Err(CommandServiceError::ResultContract {
            reason: "schema-package versions exceed the negotiated limit",
        });
    }
    for (package, version) in &identity.schema_package_versions {
        validate_identity("identity.schemaPackageVersions.package", package)?;
        validate_identity("identity.schemaPackageVersions.version", version)?;
    }
    Ok(())
}

pub fn validate_command_artifact_handle_v1(
    artifact: &CommandArtifactHandleV1,
) -> Result<(), CommandServiceError> {
    if artifact.handle_id.get() == 0 {
        return Err(CommandServiceError::ResultContract {
            reason: "artifact handle must be non-zero",
        });
    }
    if let Some(uri) = artifact.uri.as_deref() {
        validate_uri("artifacts.uri", uri)?;
    }
    validate_identity("artifacts.contentType", &artifact.content_type)?;
    validate_revision("artifacts.byteLength", artifact.byte_length)?;
    validate_digest(
        artifact.uri.as_deref().unwrap_or("artifact"),
        &artifact.sha256,
    )?;
    if let Some(source_map_id) = artifact.source_map_id.as_deref() {
        validate_identity("artifacts.sourceMapId", source_map_id)?;
    }
    Ok(())
}

fn validate_format_identity(
    field: &str,
    identity: &FormatIdentity,
    limits: CommandServiceLimitsV1,
) -> Result<(), CommandServiceError> {
    if let Some(content_type) = identity.content_type.as_deref() {
        validate_identity(format!("{field}.contentType"), content_type)?;
    }
    if let Some(schema) = identity.schema.as_deref() {
        validate_bounded_text(format!("{field}.schema"), schema, MAX_SOURCE_URI_BYTES)?;
    }
    if let Some(namespace) = identity.default_namespace.as_deref() {
        validate_uri(format!("{field}.defaultNamespace"), namespace)?;
    }
    if let Some(base_uri) = identity.base_uri.as_deref() {
        validate_uri(format!("{field}.baseUri"), base_uri)?;
    }
    if identity.namespaces.len() > limits.operation_host.max_artifact_references as usize {
        return Err(CommandServiceError::ResultContract {
            reason: "format identity namespaces exceed the negotiated limit",
        });
    }
    for (prefix, uri) in &identity.namespaces {
        validate_identity(format!("{field}.namespaces.prefix"), prefix)?;
        validate_uri(format!("{field}.namespaces[{prefix}]"), uri)?;
    }
    Ok(())
}

fn validate_payload<T: Serialize>(
    payload: &CommandPayloadV1<T>,
    limits: CommandServiceLimitsV1,
) -> Result<(), CommandServiceError> {
    match payload {
        CommandPayloadV1::Inline { value } => {
            let bytes = serde_json::to_vec(value)
                .map_err(|error| CommandServiceError::Serialization(error.to_string()))?;
            if bytes.len() as u64 > limits.worker.max_transfer_bytes_per_message {
                return Err(CommandServiceError::ResultContract {
                    reason: "inline payload exceeds the negotiated transfer-byte limit",
                });
            }
        }
        CommandPayloadV1::Artifact { handle } => validate_command_artifact_handle_v1(handle)?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::engine::FailLevel;
    use crate::report::{ReportOptionsSnapshot, DETERMINISTIC_TIMESTAMP};
    use crate::run_config::{
        parse_normalized_run_plan, NormalizedProvenance, NormalizedRunPlanRequest,
    };

    const DATA_URI: &str = "studio://catalog/data.cem";
    const QUERY_URI: &str = "studio://catalog/query.xpath";
    const TEMPLATE_URI: &str = "studio://catalog/template.cemt";
    const GRAPH_URI: &str = "studio://catalog/graph.cem";

    fn sample_plan() -> NormalizedRunPlan {
        parse_normalized_run_plan(NormalizedRunPlanRequest {
            input_records: vec![format!(
                "uri={DATA_URI},contentType=application/cem+xml,schema=https://cem.dev/ns/core/1"
            )],
            ..NormalizedRunPlanRequest::default()
        })
        .expect("sample normalized plan")
    }

    fn insert_resource(request: &mut CommandServiceRequestV1, uri: &str, bytes: &[u8]) {
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
                identity: None,
            },
        );
    }

    fn request(operation: PortableOperationRequestV1) -> CommandServiceRequestV1 {
        let version_capabilities =
            matches!(operation, PortableOperationRequestV1::VersionCapabilities);
        let mut request = CommandServiceRequestV1 {
            protocol_version: COMMAND_SERVICE_PROTOCOL_VERSION,
            request_id: "request:1".to_owned(),
            project: CommandProjectRevisionV1 {
                project_id: "catalog".to_owned(),
                revision: 1,
            },
            resource_versions: CommandUriMapV1::new(),
            operation,
            run_plan: if version_capabilities {
                CommandRunPlanV1::Null(())
            } else {
                sample_plan().into()
            },
            resources: CommandUriMapV1::new(),
            policy_stamp: CommandPolicyStampV1 {
                resolver: "resolver:1".to_owned(),
                safety: "safety:1".to_owned(),
                budget: "budget:1".to_owned(),
            },
        };
        if !version_capabilities {
            insert_resource(&mut request, DATA_URI, b"<catalog/>");
        }
        match &request.operation {
            PortableOperationRequestV1::Query { .. } => {
                insert_resource(&mut request, QUERY_URI, b"//item");
            }
            PortableOperationRequestV1::Transform {
                source: CommandTransformSourceV1::Direct { .. },
                ..
            } => insert_resource(&mut request, TEMPLATE_URI, b"<template/>"),
            PortableOperationRequestV1::Transform {
                source: CommandTransformSourceV1::Graph { .. },
                ..
            } => insert_resource(&mut request, GRAPH_URI, b"<graph/>"),
            _ => {}
        }
        request
    }

    fn portable_operations() -> Vec<PortableOperationRequestV1> {
        vec![
            PortableOperationRequestV1::Parse {
                input_id: "input:0".to_owned(),
                projection: ParseProjection::DomJson,
                preserve_source_offsets: true,
            },
            PortableOperationRequestV1::Validate {
                input_ids: vec!["input:0".to_owned()],
                projection: ValidateProjection::Json,
            },
            PortableOperationRequestV1::Check {
                input_ids: vec!["input:0".to_owned()],
                projection: ValidateProjection::Cem,
                zero_hard_violations: true,
            },
            PortableOperationRequestV1::Inspect {
                input_id: "input:0".to_owned(),
                show: InspectView::SourceOffsets,
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
        ]
    }

    fn result_identity() -> CommandExecutionIdentityV1 {
        CommandExecutionIdentityV1 {
            common_version: crate::VERSION.to_owned(),
            runtime: RuntimeKind::WasmBrowserWorker,
            target_identity: "wasm32-unknown-unknown".to_owned(),
            abi_identity: "wasm-bindgen:1".to_owned(),
            schema_package_versions: BTreeMap::from([("core".to_owned(), "1".to_owned())]),
            resolver_policy_stamp: "resolver:1".to_owned(),
            safety_policy_stamp: "safety:1".to_owned(),
            budget_policy_stamp: "budget:1".to_owned(),
        }
    }

    fn empty_result(
        status: CommandServiceStatusV1,
        exit_code: Option<u8>,
    ) -> CommandServiceResultV1 {
        CommandServiceResultV1 {
            protocol_version: COMMAND_SERVICE_PROTOCOL_VERSION,
            request_id: "request:1".to_owned(),
            project: CommandProjectRevisionV1 {
                project_id: "catalog".to_owned(),
                revision: 1,
            },
            resource_versions: CommandUriMapV1::new(),
            operation: CapabilityOperation::Inspect,
            status,
            exit_code,
            result: None,
            diagnostics: BoundedList::default(),
            report: None,
            artifacts: BoundedList::default(),
            source_maps: BoundedList::default(),
            identity: result_identity(),
            stale: None,
        }
    }

    #[test]
    fn request_wire_round_trips_all_nine_portable_operation_discriminators() {
        let expected = [
            ("parse", CapabilityOperation::Parse),
            ("validate", CapabilityOperation::Validate),
            ("check", CapabilityOperation::Check),
            ("inspect", CapabilityOperation::Inspect),
            ("convert", CapabilityOperation::Convert),
            ("query", CapabilityOperation::Query),
            ("transform", CapabilityOperation::Transform),
            ("trace", CapabilityOperation::Trace),
            (
                "version-capabilities",
                CapabilityOperation::VersionCapabilities,
            ),
        ];

        for (operation, (kind, capability)) in portable_operations().into_iter().zip(expected) {
            let request = request(operation);
            validate_command_service_request_v1(&request, CommandServiceLimitsV1::default())
                .unwrap_or_else(|error| panic!("{kind}: {error}"));
            assert_eq!(request.operation.operation(), capability);

            let value = serde_json::to_value(&request).expect("request serializes");
            let keys = value
                .as_object()
                .expect("request object")
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            assert_eq!(
                keys,
                BTreeSet::from([
                    "operation",
                    "policyStamp",
                    "project",
                    "protocolVersion",
                    "requestId",
                    "resourceVersions",
                    "resources",
                    "runPlan",
                ])
            );
            assert_eq!(value.pointer("/operation/kind"), Some(&json!(kind)));
            assert_eq!(value.pointer("/project/projectId"), Some(&json!("catalog")));
            assert!(value.get("protocol_version").is_none());
            if kind == "version-capabilities" {
                assert!(value.get("runPlan").is_some_and(Value::is_null));
            }

            let encoded = serde_json::to_vec(&request).expect("request bytes");
            let decoded = decode_command_service_request_v1(&encoded).expect("request decodes");
            validate_command_service_request_v1(&decoded, CommandServiceLimitsV1::default())
                .expect("decoded request validates");
            assert_eq!(decoded.operation.operation(), capability);
        }
    }

    #[test]
    fn request_decoder_preserves_additive_fields_and_rejects_duplicate_uri_keys() {
        let mut value =
            serde_json::to_value(request(PortableOperationRequestV1::VersionCapabilities))
                .expect("request value");
        value["futureField"] = json!({ "preservedAtExportBoundary": false });
        let decoded = decode_command_service_request_v1(
            &serde_json::to_vec(&value).expect("extended request bytes"),
        )
        .expect("same-major additive field is ignored");
        validate_command_service_request_v1(&decoded, CommandServiceLimitsV1::default())
            .expect("extended request validates");

        let duplicate = br#"{
            "protocolVersion":1,
            "requestId":"request:1",
            "project":{"projectId":"catalog","revision":1},
            "resourceVersions":{
                "studio://catalog/a":{"revision":1,"sha256":"0000000000000000000000000000000000000000000000000000000000000000"},
                "studio://catalog/a":{"revision":2,"sha256":"1111111111111111111111111111111111111111111111111111111111111111"}
            },
            "operation":{"kind":"version-capabilities"},
            "runPlan":null,
            "resources":{},
            "policyStamp":{"resolver":"r","safety":"s","budget":"b"}
        }"#;
        let error = decode_command_service_request_v1(duplicate).unwrap_err();
        assert_eq!(error.code(), "cem.command_service.decode");
        assert!(error.to_string().contains("duplicate command resource URI"));

        value.as_object_mut().expect("object").remove("runPlan");
        assert_eq!(
            decode_command_service_request_v1(&serde_json::to_vec(&value).unwrap())
                .unwrap_err()
                .code(),
            "cem.command_service.decode"
        );
    }

    #[test]
    fn request_admission_rejects_identity_revision_policy_uri_digest_and_size_violations() {
        let valid = request(PortableOperationRequestV1::Parse {
            input_id: "input:0".to_owned(),
            projection: ParseProjection::Json,
            preserve_source_offsets: false,
        });
        let limits = CommandServiceLimitsV1::default();

        let mut invalid = valid.clone();
        invalid.protocol_version += 1;
        assert_eq!(
            validate_command_service_request_v1(&invalid, limits)
                .unwrap_err()
                .code(),
            "cem.command_service.protocol_version"
        );

        let mut invalid = valid.clone();
        invalid.request_id = "x".repeat(MAX_IDENTITY_BYTES + 1);
        assert_eq!(
            validate_command_service_request_v1(&invalid, limits)
                .unwrap_err()
                .code(),
            "cem.command_service.identity_invalid"
        );

        let mut invalid = valid.clone();
        invalid.policy_stamp.resolver.clear();
        assert_eq!(
            validate_command_service_request_v1(&invalid, limits)
                .unwrap_err()
                .code(),
            "cem.command_service.identity_invalid"
        );

        let mut invalid = valid.clone();
        invalid.project.revision = MAX_JSON_SAFE_INTEGER + 1;
        assert_eq!(
            validate_command_service_request_v1(&invalid, limits)
                .unwrap_err()
                .code(),
            "cem.command_service.revision_out_of_range"
        );

        let mut invalid = valid.clone();
        let version = invalid.resource_versions.remove(DATA_URI).unwrap();
        invalid
            .resource_versions
            .insert("relative.cem".to_owned(), version);
        assert_eq!(
            validate_command_service_request_v1(&invalid, limits)
                .unwrap_err()
                .code(),
            "cem.command_service.uri_invalid"
        );

        let mut invalid = valid.clone();
        invalid.resource_versions.get_mut(DATA_URI).unwrap().sha256 = "A".repeat(64);
        assert_eq!(
            validate_command_service_request_v1(&invalid, limits)
                .unwrap_err()
                .code(),
            "cem.command_service.resource_digest_invalid"
        );

        let mut invalid = valid.clone();
        invalid.resource_versions.get_mut(DATA_URI).unwrap().sha256 = "0".repeat(64);
        assert_eq!(
            validate_command_service_request_v1(&invalid, limits)
                .unwrap_err()
                .code(),
            "cem.command_service.resource_digest_mismatch"
        );

        let mut invalid = valid.clone();
        invalid.resource_versions.remove(DATA_URI);
        assert_eq!(
            validate_command_service_request_v1(&invalid, limits)
                .unwrap_err()
                .code(),
            "cem.command_service.resource_version_missing"
        );

        let mut bounded = valid.clone();
        insert_resource(&mut bounded, QUERY_URI, b"query");
        let mut strict = limits;
        strict.worker.max_transfer_buffers_per_message = 1;
        assert_eq!(
            validate_command_service_request_v1(&bounded, strict)
                .unwrap_err()
                .code(),
            "cem.command_service.resource_count"
        );

        let mut strict = limits;
        strict.worker.max_transfer_bytes_per_message = 1;
        assert_eq!(
            validate_command_service_request_v1(&valid, strict)
                .unwrap_err()
                .code(),
            "cem.command_service.resource_bytes"
        );

        let mut oversized = valid.clone();
        if let PortableOperationRequestV1::Parse { input_id, .. } = &mut oversized.operation {
            *input_id = "x".repeat(crate::worker_control::MAX_WORK_INLINE_PAYLOAD_BYTES);
        }
        assert_eq!(
            validate_command_service_request_v1(&oversized, limits)
                .unwrap_err()
                .code(),
            "cem.command_service.operation_metadata_too_large"
        );

        let mut oversized = valid;
        let plan = match &mut oversized.run_plan {
            CommandRunPlanV1::Plan(plan) => plan,
            CommandRunPlanV1::Null(()) => unreachable!(),
        };
        plan.provenance = vec![NormalizedProvenance {
            field_path: "x".repeat(crate::worker_control::MAX_WORK_INLINE_PAYLOAD_BYTES),
            source: "fixture".to_owned(),
            declared_value: None,
            normalized_value: None,
            source_range: None,
        }];
        assert_eq!(
            validate_command_service_request_v1(&oversized, limits)
                .unwrap_err()
                .code(),
            "cem.command_service.run_plan_metadata_too_large"
        );
    }

    #[test]
    fn request_admission_requires_plan_resources_and_known_unique_inputs() {
        let limits = CommandServiceLimitsV1::default();
        let mut invalid = request(PortableOperationRequestV1::Validate {
            input_ids: vec![],
            projection: ValidateProjection::Json,
        });
        assert_eq!(
            validate_command_service_request_v1(&invalid, limits)
                .unwrap_err()
                .code(),
            "cem.command_service.operation_inputs_empty"
        );

        invalid.operation = PortableOperationRequestV1::Validate {
            input_ids: vec!["input:0".to_owned(), "input:0".to_owned()],
            projection: ValidateProjection::Json,
        };
        assert_eq!(
            validate_command_service_request_v1(&invalid, limits)
                .unwrap_err()
                .code(),
            "cem.command_service.input_id_duplicate"
        );

        invalid.operation = PortableOperationRequestV1::Parse {
            input_id: "input:missing".to_owned(),
            projection: ParseProjection::Json,
            preserve_source_offsets: false,
        };
        assert_eq!(
            validate_command_service_request_v1(&invalid, limits)
                .unwrap_err()
                .code(),
            "cem.command_service.input_id_unknown"
        );

        invalid.run_plan = CommandRunPlanV1::Null(());
        assert_eq!(
            validate_command_service_request_v1(&invalid, limits)
                .unwrap_err()
                .code(),
            "cem.command_service.run_plan_required"
        );

        let mut invalid = request(PortableOperationRequestV1::VersionCapabilities);
        invalid.run_plan = sample_plan().into();
        assert_eq!(
            validate_command_service_request_v1(&invalid, limits)
                .unwrap_err()
                .code(),
            "cem.command_service.run_plan_unexpected"
        );

        let mut invalid = request(PortableOperationRequestV1::Parse {
            input_id: "input:0".to_owned(),
            projection: ParseProjection::Json,
            preserve_source_offsets: false,
        });
        let plan = match &mut invalid.run_plan {
            CommandRunPlanV1::Plan(plan) => plan,
            CommandRunPlanV1::Null(()) => unreachable!(),
        };
        plan.inputs[0].byte_source_kind = NormalizedByteSourceKind::Stream;
        assert_eq!(
            validate_command_service_request_v1(&invalid, limits)
                .unwrap_err()
                .code(),
            "cem.command_service.run_plan_host_state"
        );
    }

    #[test]
    fn transform_graph_admission_keeps_invocation_metadata_stage_local() {
        let limits = CommandServiceLimitsV1::default();
        let accepted = request(PortableOperationRequestV1::Transform {
            source: CommandTransformSourceV1::Graph {
                config_uri: GRAPH_URI.to_owned(),
            },
            params: BTreeMap::new(),
            template_entrypoint: TransformTemplateEntrypoint::implicit(),
            preserve_source_offsets: true,
        });
        validate_command_service_request_v1(&accepted, limits)
            .expect("implicit graph invocation metadata is admitted");

        let mut params_override = accepted.clone();
        let PortableOperationRequestV1::Transform { params, .. } = &mut params_override.operation
        else {
            unreachable!()
        };
        params.insert("locale".to_owned(), json!("en"));
        let error = validate_command_service_request_v1(&params_override, limits)
            .expect_err("top-level graph params are rejected");
        assert_eq!(
            error.code(),
            "cem.command_service.transform_graph_stage_local"
        );
        assert!(error.to_string().contains("operation.params"));

        let mut entrypoint_override = accepted;
        let PortableOperationRequestV1::Transform {
            template_entrypoint,
            ..
        } = &mut entrypoint_override.operation
        else {
            unreachable!()
        };
        *template_entrypoint = TransformTemplateEntrypoint::named("main");
        let error = validate_command_service_request_v1(&entrypoint_override, limits)
            .expect_err("top-level graph entrypoint is rejected");
        assert_eq!(
            error.code(),
            "cem.command_service.transform_graph_stage_local"
        );
        assert!(error.to_string().contains("operation.templateEntrypoint"));
    }

    #[test]
    fn revision_classifier_is_reusable_for_admission_and_prepublication_freshness() {
        let request = request(PortableOperationRequestV1::Inspect {
            input_id: "input:0".to_owned(),
            show: InspectView::Summary,
        });
        let current = CommandRevisionLedgerV1 {
            project: request.project.clone(),
            resource_versions: request.resource_versions.clone(),
        };
        assert_eq!(
            admit_command_service_request_v1(&request, &current, CommandServiceLimitsV1::default())
                .unwrap(),
            CommandServiceAdmissionV1::Accepted
        );

        let mut changed = current.clone();
        changed.project.revision = 2;
        changed
            .resource_versions
            .get_mut(DATA_URI)
            .unwrap()
            .revision = 2;
        changed.resource_versions.get_mut(DATA_URI).unwrap().sha256 = "f".repeat(64);
        let CommandServiceAdmissionV1::Stale(stale) =
            admit_command_service_request_v1(&request, &changed, CommandServiceLimitsV1::default())
                .unwrap()
        else {
            panic!("changed prepublication ledger must classify the request as stale")
        };
        assert_eq!(stale.current_project_revision, 2);
        assert_eq!(stale.changed_resources.len(), 1);
        assert_eq!(stale.changed_resources[0].uri, DATA_URI);

        let mut foreign = current.clone();
        foreign.project.project_id = "other".to_owned();
        assert_eq!(
            admit_command_service_request_v1(&request, &foreign, CommandServiceLimitsV1::default())
                .unwrap_err()
                .code(),
            "cem.command_service.ledger_project_mismatch"
        );

        let mut missing = current;
        missing.resource_versions.remove(DATA_URI);
        assert_eq!(
            admit_command_service_request_v1(&request, &missing, CommandServiceLimitsV1::default())
                .unwrap_err()
                .code(),
            "cem.command_service.ledger_resource_missing"
        );
    }

    #[test]
    fn result_wire_validates_typed_payload_report_artifact_source_map_and_exit_contract() {
        let artifact = CommandArtifactHandleV1 {
            handle_id: RetainedHandleId::from_raw(1),
            kind: CommandArtifactKindV1::Output,
            uri: Some("studio://catalog/output.cem".to_owned()),
            content_type: "application/cem+xml".to_owned(),
            byte_length: 6,
            sha256: sha256_hex(b"output"),
            source_map_id: Some("source-map:1".to_owned()),
        };
        let report = Report::deterministic(
            vec![DATA_URI.to_owned()],
            Vec::new(),
            ReportOptionsSnapshot {
                fail_level: FailLevel::Validate,
                schema: None,
                content_type: Some("application/cem+xml".to_owned()),
                base_uri: Some(DATA_URI.to_owned()),
            },
        );
        assert_eq!(report.generated_at, DETERMINISTIC_TIMESTAMP);
        let mut result = empty_result(CommandServiceStatusV1::Succeeded, Some(0));
        result.result = Some(CommandPayloadV1::Inline {
            value: PortableOperationResultV1::Inspect(InspectResponse {
                view: InspectView::Summary,
                body: json!({ "nodes": 1 }),
                primary_bytes: None,
            }),
        });
        result.report = Some(CommandPayloadV1::Inline { value: report });
        result.artifacts = BoundedList::new(vec![artifact]);
        result.source_maps = BoundedList::new(vec![CommandSourceMapReferenceV1 {
            source_map_id: "source-map:1".to_owned(),
            owner: CommandSourceMapOwnerV1::Artifact {
                handle_id: RetainedHandleId::from_raw(1),
            },
            source_map: CommandPayloadV1::Inline {
                value: SourceMapStack::default(),
            },
        }]);
        validate_command_service_result_v1(&result, CommandServiceLimitsV1::default())
            .expect("typed result validates");

        let value = serde_json::to_value(&result).expect("result serializes");
        assert_eq!(value.pointer("/exitCode"), Some(&json!(0)));
        assert_eq!(value.pointer("/result/storage"), Some(&json!("inline")));
        assert_eq!(value.pointer("/result/value/kind"), Some(&json!("inspect")));
        assert_eq!(value.pointer("/report/storage"), Some(&json!("inline")));
        assert_eq!(
            value.pointer("/artifacts/items/0/handleId"),
            Some(&json!(1))
        );
        assert_eq!(
            value.pointer("/sourceMaps/items/0/sourceMapId"),
            Some(&json!("source-map:1"))
        );

        for exit_code in [1, 2, 3, 6] {
            validate_command_service_result_v1(
                &empty_result(CommandServiceStatusV1::Failed, Some(exit_code)),
                CommandServiceLimitsV1::default(),
            )
            .unwrap_or_else(|error| panic!("failed exit {exit_code}: {error}"));
        }
        validate_command_service_result_v1(
            &empty_result(CommandServiceStatusV1::Cancelled, Some(130)),
            CommandServiceLimitsV1::default(),
        )
        .expect("cancelled exit validates");
        validate_command_service_result_v1(
            &empty_result(CommandServiceStatusV1::Fatal, Some(7)),
            CommandServiceLimitsV1::default(),
        )
        .expect("fatal exit validates");

        let mut stale = empty_result(CommandServiceStatusV1::Stale, None);
        stale.stale = Some(CommandStaleRevisionV1 {
            current_project_revision: 2,
            changed_resources: vec![CommandChangedResourceV1 {
                uri: DATA_URI.to_owned(),
                revision: 2,
                sha256: "f".repeat(64),
            }],
        });
        validate_command_service_result_v1(&stale, CommandServiceLimitsV1::default())
            .expect("stale result validates");
        assert!(serde_json::to_value(stale).unwrap()["exitCode"].is_null());

        let invalid = empty_result(CommandServiceStatusV1::Failed, Some(0));
        assert_eq!(
            validate_command_service_result_v1(&invalid, CommandServiceLimitsV1::default())
                .unwrap_err()
                .code(),
            "cem.command_service.result_invalid"
        );

        result.operation = CapabilityOperation::Parse;
        assert_eq!(
            validate_command_service_result_v1(&result, CommandServiceLimitsV1::default())
                .unwrap_err()
                .code(),
            "cem.command_service.result_invalid"
        );
    }
}
