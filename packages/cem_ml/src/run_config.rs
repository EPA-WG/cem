//! Serializable run/root-scope configuration shared by library hosts,
//! WASM adapters, and the CLI.
//!
//! The model is intentionally an array shape: build/CI callers can
//! validate or transform several documents in one run while preserving
//! each document root as scope zero for diagnostics, source maps, schema
//! selection, and resource policy accounting.

use crate::diagnostics::{Diagnostic, Severity};
use crate::engine::{EngineInput, FailLevel, FormatIdentity, InputFormat};
use crate::resolver::{has_uri_scheme, local_file_uri_to_path};
use crate::scheduler::{OverflowPolicy, ScopePolicy};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const RUN_CONFIG_SCHEMA_URI: &str = "https://cem.dev/ns/cli/run-config/1";
pub const RUN_CONFIG_NAMESPACE_URI: &str = RUN_CONFIG_SCHEMA_URI;
pub const RUN_CONFIG_JSON_SCHEMA_URI: &str = "https://cem.dev/schema/cli/run-config.schema.json";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunConfig {
    #[serde(default)]
    pub inputs: Vec<InputSpec>,
    #[serde(default)]
    pub outputs: Vec<OutputSpec>,
    #[serde(default)]
    pub schema_packages: Vec<InputSpec>,
    #[serde(default)]
    pub resolvers: Vec<ResolverSpec>,
    #[serde(default)]
    pub scheduler: SchedulerConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputSpec {
    pub uri: String,
    #[serde(default)]
    pub root_scope: ScopeConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputSpec {
    #[serde(default)]
    pub input_ref: Option<String>,
    #[serde(default)]
    pub destination: Option<String>,
    #[serde(default)]
    pub root_scope: ScopeConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolverSpec {
    pub uri_prefix: String,
    pub local_root: String,
    #[serde(default)]
    pub read: bool,
    #[serde(default)]
    pub write: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeConfig {
    #[serde(default)]
    pub default_content_type: Option<String>,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_color_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cemt_formatter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cemt_formatter_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cemt_colorizer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cemt_color_profile: Option<String>,
    #[serde(default)]
    pub version_pins: BTreeMap<String, String>,
    #[serde(default)]
    pub default_namespace: Option<String>,
    #[serde(default)]
    pub namespaces: BTreeMap<String, String>,
    #[serde(default)]
    pub module_map: Option<String>,
    #[serde(default)]
    pub base_uri: Option<String>,
    #[serde(default)]
    pub policy: Option<String>,
    #[serde(default)]
    pub budgets: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerConfig {
    #[serde(default)]
    pub thread_pool: Option<String>,
    #[serde(default)]
    pub max_parallel_documents: Option<u32>,
}

impl ScopeConfig {
    pub fn format_identity(&self) -> FormatIdentity {
        FormatIdentity {
            content_type: self.default_content_type.clone(),
            schema: self.schema.clone(),
            default_namespace: self.default_namespace.clone(),
            namespaces: self.namespaces.clone(),
            base_uri: self.base_uri.clone(),
        }
    }

    pub fn format_identity_option(&self) -> Option<FormatIdentity> {
        let identity = self.format_identity();
        (identity.content_type.is_some()
            || identity.schema.is_some()
            || identity.default_namespace.is_some()
            || !identity.namespaces.is_empty()
            || identity.base_uri.is_some())
        .then_some(identity)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunConfigDefaults {
    #[serde(default)]
    pub input_scope: ScopeConfig,
    #[serde(default)]
    pub output_scope: ScopeConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_format_hint: Option<InputFormat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_format_fallback: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfigParseRequest {
    pub bytes: Vec<u8>,
    pub identity: FormatIdentity,
    pub base_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfigParseResponse {
    pub config: RunConfig,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedRunPlanRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_bytes: Option<Vec<u8>>,
    #[serde(default)]
    pub config_identity: FormatIdentity,
    #[serde(default)]
    pub config_base_uri: Option<String>,
    #[serde(default)]
    pub defaults: RunConfigDefaults,
    #[serde(default)]
    pub input_records: Vec<String>,
    #[serde(default)]
    pub output_records: Vec<String>,
    #[serde(default)]
    pub diagnostics_mode: NormalizedDiagnosticsMode,
    #[serde(default)]
    pub command_profile: Option<String>,
}

impl Default for NormalizedRunPlanRequest {
    fn default() -> Self {
        Self {
            config_bytes: None,
            config_identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                schema: Some(RUN_CONFIG_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            },
            config_base_uri: None,
            defaults: RunConfigDefaults::default(),
            input_records: Vec::new(),
            output_records: Vec::new(),
            diagnostics_mode: NormalizedDiagnosticsMode::default(),
            command_profile: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedRunPlan {
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_profile: Option<String>,
    pub config_identity: NormalizedConfigIdentity,
    #[serde(default)]
    pub effective_config: RunConfig,
    #[serde(default)]
    pub authored_sources: Vec<NormalizedAuthoredSource>,
    #[serde(default)]
    pub inputs: Vec<NormalizedInput>,
    #[serde(default)]
    pub outputs: Vec<NormalizedOutput>,
    #[serde(default)]
    pub schema_packages: Vec<NormalizedSchemaPackage>,
    #[serde(default)]
    pub resolvers: Vec<NormalizedResolverBinding>,
    #[serde(default)]
    pub scheduler: SchedulerConfig,
    #[serde(default)]
    pub diagnostics_mode: NormalizedDiagnosticsMode,
    #[serde(default)]
    pub provenance: Vec<NormalizedProvenance>,
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
}

impl NormalizedRunPlan {
    pub fn effective_run_config(&self) -> RunConfig {
        self.effective_config.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedConfigIdentity {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_identity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace_identity: Option<String>,
    pub source_kind: NormalizedConfigSourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_range: Option<NormalizedSourceRange>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NormalizedConfigSourceKind {
    File,
    FileUri,
    CustomUri,
    Bytes,
    CliRecords,
    #[default]
    HostObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedAuthoredSource {
    pub source_id: String,
    pub source_kind: NormalizedConfigSourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_uri: Option<String>,
    #[serde(default)]
    pub identity: FormatIdentity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_range: Option<NormalizedSourceRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedSourceRange {
    pub start_byte: u64,
    pub end_byte: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedInput {
    pub input_id: String,
    pub declared_uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_uri: Option<String>,
    pub byte_source_kind: NormalizedByteSourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_format_hint: Option<InputFormat>,
    pub identity: FormatIdentity,
    pub root_scope: NormalizedRootScope,
    #[serde(default)]
    pub provenance: Vec<NormalizedProvenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_range: Option<NormalizedSourceRange>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NormalizedByteSourceKind {
    #[default]
    Uri,
    Bytes,
    Stream,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedOutput {
    pub output_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared_destination: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_destination: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_format_fallback: Option<String>,
    pub identity: FormatIdentity,
    pub root_scope: NormalizedRootScope,
    pub primary_output_policy: NormalizedPrimaryOutputPolicy,
    #[serde(default)]
    pub sidecars: Vec<String>,
    #[serde(default)]
    pub provenance: Vec<NormalizedProvenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_range: Option<NormalizedSourceRange>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NormalizedPrimaryOutputPolicy {
    #[default]
    InMemory,
    WriteDestination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedSchemaPackage {
    pub schema_package_id: String,
    pub declared_uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_uri: Option<String>,
    pub identity: FormatIdentity,
    pub root_scope: NormalizedRootScope,
    #[serde(default)]
    pub provenance: Vec<NormalizedProvenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_range: Option<NormalizedSourceRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedRootScope {
    pub scope_id: String,
    pub direction: NormalizedScopeDirection,
    pub identity: FormatIdentity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_namespace: Option<String>,
    #[serde(default)]
    pub namespaces: BTreeMap<String, String>,
    #[serde(default)]
    pub version_pins: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_uri: Option<String>,
    pub resolver_context: NormalizedResolverContext,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_map: Option<NormalizedModuleMapIdentity>,
    pub policy: NormalizedScopePolicy,
    pub budgets: NormalizedBudgets,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_pipeline: Option<NormalizedOutputPipeline>,
    #[serde(default)]
    pub provenance: Vec<NormalizedProvenance>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NormalizedScopeDirection {
    #[default]
    Input,
    Output,
    SchemaPackage,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedResolverContext {
    #[serde(default)]
    pub resolver_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedModuleMapIdentity {
    pub declared_uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entries_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolver_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_uri: Option<String>,
    pub state: NormalizedModuleMapState,
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
    #[serde(default)]
    pub provenance: Vec<NormalizedProvenance>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NormalizedModuleMapState {
    Valid,
    #[default]
    Missing,
    Invalid,
    Unreadable,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedScopePolicy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_name: Option<String>,
    pub cpu_workers: u32,
    pub queue_size: u32,
    pub io_streams: u32,
    pub memory_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_time_budget_ms: Option<u64>,
    pub overflow: OverflowPolicy,
    #[serde(default)]
    pub provenance: Vec<NormalizedProvenance>,
}

impl NormalizedScopePolicy {
    fn from_scope_policy(policy_name: Option<String>, policy: ScopePolicy) -> Self {
        Self {
            policy_name,
            cpu_workers: policy.cpu_workers,
            queue_size: policy.queue_size,
            io_streams: policy.io_streams,
            memory_bytes: policy.memory_bytes,
            plugin_time_budget_ms: policy.plugin_time_budget_ms,
            overflow: policy.overflow,
            provenance: Vec::new(),
        }
    }
}

impl Default for NormalizedScopePolicy {
    fn default() -> Self {
        Self::from_scope_policy(None, deterministic_scope_policy())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedBudgets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validate_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convert_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inspect_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bench_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixture_validate_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixture_roundtrip_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observe_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,
    #[serde(default)]
    pub unknown: Vec<NormalizedBudgetEntry>,
    #[serde(default)]
    pub provenance: Vec<NormalizedProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedBudgetEntry {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedOutputPipeline {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_color_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cemt_formatter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cemt_formatter_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cemt_colorizer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cemt_color_profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedResolverBinding {
    pub resolver_id: String,
    pub scheme: String,
    #[serde(default)]
    pub purposes: Vec<NormalizedResolverPurpose>,
    #[serde(default)]
    pub directions: Vec<NormalizedResolverDirection>,
    pub declared_uri_prefix: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_local_root: Option<String>,
    pub support: NormalizedResolverSupport,
    #[serde(default)]
    pub provenance: Vec<NormalizedProvenance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NormalizedResolverPurpose {
    Config,
    Input,
    Template,
    ModuleMap,
    Output,
    Report,
    ObserveEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NormalizedResolverDirection {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NormalizedResolverSupport {
    #[default]
    Required,
    Optional,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedDiagnosticsMode {
    pub fail_level: FailLevel,
    pub primary_kind: NormalizedPrimaryKind,
    pub report_projection: NormalizedReportProjection,
    #[serde(default)]
    pub report_destinations: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observe_events_destination: Option<String>,
    #[serde(default)]
    pub quiet: bool,
    #[serde(default)]
    pub verbose: bool,
    #[serde(default)]
    pub no_color: bool,
}

impl Default for NormalizedDiagnosticsMode {
    fn default() -> Self {
        Self {
            fail_level: FailLevel::Validate,
            primary_kind: NormalizedPrimaryKind::Report,
            report_projection: NormalizedReportProjection::Json,
            report_destinations: Vec::new(),
            observe_events_destination: None,
            quiet: false,
            verbose: false,
            no_color: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NormalizedPrimaryKind {
    #[default]
    Report,
    Content,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NormalizedReportProjection {
    Text,
    #[default]
    Json,
    Xml,
    Cem,
    Html,
    Markdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedProvenance {
    pub field_path: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalized_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_range: Option<NormalizedSourceRange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfigError {
    pub code: &'static str,
    pub message: String,
}

impl std::fmt::Display for RunConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for RunConfigError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecParseError {
    pub message: String,
}

impl std::fmt::Display for SpecParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SpecParseError {}

pub fn parse_run_config(
    request: RunConfigParseRequest,
) -> Result<RunConfigParseResponse, RunConfigError> {
    validate_run_config_identity(&request.identity)?;

    let content_type = request
        .identity
        .content_type
        .as_deref()
        .map(content_type_essence)
        .unwrap_or_else(|| "application/json".to_owned());

    match content_type.as_str() {
        "application/json" | "text/json" => {
            let config = serde_json::from_slice::<RunConfig>(&request.bytes).map_err(|error| {
                run_config_error(
                    "cem.run_config.invalid_json",
                    format!("run config JSON could not be parsed: {error}"),
                )
            })?;
            Ok(RunConfigParseResponse {
                config,
                diagnostics: Vec::new(),
            })
        }
        other => Err(run_config_error(
            "cem.run_config.unsupported_content_type",
            format!("run config content type `{other}` is not supported yet; use application/json"),
        )),
    }
}

pub fn parse_normalized_run_plan(
    request: NormalizedRunPlanRequest,
) -> Result<NormalizedRunPlan, RunConfigError> {
    validate_run_config_identity(&request.config_identity)?;

    let has_config_bytes = request.config_bytes.is_some();
    let has_records = !request.input_records.is_empty() || !request.output_records.is_empty();
    let source_kind = normalized_config_source_kind(
        has_config_bytes,
        has_records,
        request.config_base_uri.as_deref(),
    );
    let config_identity = normalized_config_identity(&request, source_kind, has_config_bytes);
    let authored_sources = vec![NormalizedAuthoredSource {
        source_id: "config:0".to_owned(),
        source_kind,
        declared_uri: request.config_base_uri.clone(),
        resolved_uri: resolved_config_uri(request.config_base_uri.as_deref()),
        identity: request.config_identity.clone(),
        source_range: None,
    }];

    let mut diagnostics = Vec::new();
    let mut config = if let Some(bytes) = request.config_bytes.clone() {
        let parsed = parse_run_config(RunConfigParseRequest {
            bytes,
            identity: request.config_identity.clone(),
            base_uri: request.config_base_uri.clone(),
        })?;
        diagnostics.extend(parsed.diagnostics);
        parsed.config
    } else {
        RunConfig::default()
    };

    for (index, record) in request.input_records.iter().enumerate() {
        match parse_input_spec_record(record) {
            Ok(spec) => config.inputs.push(spec),
            Err(error) => diagnostics.push(record_parse_diagnostic(
                "cem.run_config.input_spec_invalid",
                "inputSpecRecords",
                index,
                error,
                request.config_base_uri.as_deref(),
            )),
        }
    }

    for (index, record) in request.output_records.iter().enumerate() {
        match parse_output_spec_record(record) {
            Ok(spec) => config.outputs.push(spec),
            Err(error) => diagnostics.push(record_parse_diagnostic(
                "cem.run_config.output_spec_invalid",
                "outputSpecRecords",
                index,
                error,
                request.config_base_uri.as_deref(),
            )),
        }
    }

    let authored_config = config.clone();
    let defaults = request.defaults.clone();
    let response =
        normalize_run_config(config, request.defaults, request.config_base_uri.as_deref());
    diagnostics.extend(response.diagnostics);

    Ok(build_normalized_run_plan(
        &authored_config,
        &response.config,
        config_identity,
        authored_sources,
        request.command_profile,
        request.diagnostics_mode,
        request.config_base_uri.as_deref(),
        defaults.from_format_hint,
        defaults.to_format_fallback,
        diagnostics,
    ))
}

fn validate_run_config_identity(identity: &FormatIdentity) -> Result<(), RunConfigError> {
    if let Some(schema) = identity.schema.as_deref().map(str::trim) {
        if !schema.is_empty() && schema != RUN_CONFIG_SCHEMA_URI {
            return Err(run_config_error(
                "cem.run_config.unsupported_schema_identity",
                format!(
                    "run config schema `{schema}` is not supported; expected `{RUN_CONFIG_SCHEMA_URI}`"
                ),
            ));
        }
    }

    if let Some(default_namespace) = identity.default_namespace.as_deref().map(str::trim) {
        if !default_namespace.is_empty() && default_namespace != RUN_CONFIG_NAMESPACE_URI {
            return Err(run_config_error(
                "cem.run_config.unsupported_schema_identity",
                format!(
                    "run config namespace `{default_namespace}` is not supported; expected `{RUN_CONFIG_NAMESPACE_URI}`"
                ),
            ));
        }
    }

    Ok(())
}

pub fn normalize_run_config(
    mut config: RunConfig,
    defaults: RunConfigDefaults,
    base_uri: Option<&str>,
) -> RunConfigParseResponse {
    for input in &mut config.inputs {
        merge_scope_defaults(&mut input.root_scope, &defaults.input_scope);
        resolve_scope_module_map(&mut input.root_scope, base_uri);
        if input.root_scope.default_content_type.is_none() {
            input.root_scope.default_content_type = infer_content_type_from_path(&input.uri);
        }
    }

    for output in &mut config.outputs {
        merge_scope_defaults(&mut output.root_scope, &defaults.output_scope);
        resolve_scope_module_map(&mut output.root_scope, base_uri);
        resolve_output_destination(output, base_uri);
        if output.root_scope.default_content_type.is_none() {
            if let Some(destination) = output.destination.as_deref() {
                output.root_scope.default_content_type = infer_content_type_from_path(destination);
            }
        }
    }

    for package in &mut config.schema_packages {
        resolve_scope_module_map(&mut package.root_scope, base_uri);
        if package.root_scope.default_content_type.is_none() {
            package.root_scope.default_content_type =
                Some(crate::schema::registry::CEM_SCHEMA_PACKAGE_CONTENT_TYPE.to_owned());
        }
        if package.root_scope.schema.is_none() {
            package.root_scope.schema =
                Some(crate::schema::registry::CEM_SCHEMA_PACKAGE_URI.to_owned());
        }
    }

    let mut diagnostics = validate_run_config_defaults(&defaults, base_uri);
    diagnostics.extend(validate_run_config(&config, base_uri));
    RunConfigParseResponse {
        config,
        diagnostics,
    }
}

fn build_normalized_run_plan(
    authored: &RunConfig,
    normalized: &RunConfig,
    config_identity: NormalizedConfigIdentity,
    authored_sources: Vec<NormalizedAuthoredSource>,
    command_profile: Option<String>,
    diagnostics_mode: NormalizedDiagnosticsMode,
    base_uri: Option<&str>,
    from_format_hint: Option<InputFormat>,
    to_format_fallback: Option<String>,
    mut diagnostics: Vec<Diagnostic>,
) -> NormalizedRunPlan {
    validate_resolver_specs_with_paths(&normalized.resolvers, base_uri, &mut diagnostics);

    let resolvers: Vec<_> = normalized
        .resolvers
        .iter()
        .enumerate()
        .map(|(index, resolver)| normalized_resolver_binding(resolver, index))
        .collect();
    let resolver_ids: Vec<_> = resolvers
        .iter()
        .map(|resolver| resolver.resolver_id.clone())
        .collect();

    let inputs: Vec<_> = normalized
        .inputs
        .iter()
        .enumerate()
        .map(|(index, input)| {
            let authored_input = authored.inputs.get(index).unwrap_or(input);
            NormalizedInput {
                input_id: format!("input:{index}"),
                declared_uri: authored_input.uri.clone(),
                resolved_uri: changed_value(&authored_input.uri, &input.uri),
                byte_source_kind: NormalizedByteSourceKind::Uri,
                from_format_hint,
                identity: input.root_scope.format_identity(),
                root_scope: normalized_root_scope(
                    &authored_input.root_scope,
                    &input.root_scope,
                    NormalizedScopeDirection::Input,
                    index,
                    "inputs",
                    base_uri,
                    &resolver_ids,
                    &mut diagnostics,
                ),
                provenance: from_format_hint
                    .map(input_format_hint_provenance)
                    .into_iter()
                    .collect(),
                source_range: None,
            }
        })
        .collect();

    let schema_packages: Vec<_> = normalized
        .schema_packages
        .iter()
        .enumerate()
        .map(|(index, package)| {
            let authored_package = authored.schema_packages.get(index).unwrap_or(package);
            NormalizedSchemaPackage {
                schema_package_id: format!("schemaPackage:{index}"),
                declared_uri: authored_package.uri.clone(),
                resolved_uri: changed_value(&authored_package.uri, &package.uri),
                identity: package.root_scope.format_identity(),
                root_scope: normalized_root_scope(
                    &authored_package.root_scope,
                    &package.root_scope,
                    NormalizedScopeDirection::SchemaPackage,
                    index,
                    "schemaPackages",
                    base_uri,
                    &resolver_ids,
                    &mut diagnostics,
                ),
                provenance: Vec::new(),
                source_range: None,
            }
        })
        .collect();

    let outputs: Vec<_> = normalized
        .outputs
        .iter()
        .enumerate()
        .map(|(index, output)| {
            let authored_output = authored.outputs.get(index).unwrap_or(output);
            let input_id = normalized_output_input_id(
                output,
                &normalized.inputs,
                index,
                base_uri,
                &mut diagnostics,
            );
            NormalizedOutput {
                output_id: format!("output:{index}"),
                input_id,
                declared_destination: authored_output.destination.clone(),
                resolved_destination: output.destination.clone(),
                to_format_fallback: to_format_fallback.clone(),
                identity: output.root_scope.format_identity(),
                root_scope: normalized_root_scope(
                    &authored_output.root_scope,
                    &output.root_scope,
                    NormalizedScopeDirection::Output,
                    index,
                    "outputs",
                    base_uri,
                    &resolver_ids,
                    &mut diagnostics,
                ),
                primary_output_policy: if output.destination.is_some() {
                    NormalizedPrimaryOutputPolicy::WriteDestination
                } else {
                    NormalizedPrimaryOutputPolicy::InMemory
                },
                sidecars: Vec::new(),
                provenance: to_format_fallback
                    .as_deref()
                    .map(to_format_fallback_provenance)
                    .into_iter()
                    .collect(),
                source_range: None,
            }
        })
        .collect();

    let run_id = stable_run_id(normalized, &diagnostics_mode, command_profile.as_deref());
    NormalizedRunPlan {
        run_id,
        command_profile,
        config_identity,
        effective_config: normalized.clone(),
        authored_sources,
        inputs,
        outputs,
        schema_packages,
        resolvers,
        scheduler: normalized.scheduler.clone(),
        diagnostics_mode,
        provenance: Vec::new(),
        diagnostics,
    }
}

fn input_format_hint_provenance(format: InputFormat) -> NormalizedProvenance {
    let value = input_format_id(format).to_owned();
    NormalizedProvenance {
        field_path: "defaults.fromFormatHint".to_owned(),
        source: "command-defaults".to_owned(),
        declared_value: Some(value.clone()),
        normalized_value: Some(value),
        source_range: None,
    }
}

fn to_format_fallback_provenance(format: &str) -> NormalizedProvenance {
    NormalizedProvenance {
        field_path: "defaults.toFormatFallback".to_owned(),
        source: "command-defaults".to_owned(),
        declared_value: Some(format.to_owned()),
        normalized_value: Some(format.to_owned()),
        source_range: None,
    }
}

fn input_format_id(format: InputFormat) -> &'static str {
    match format {
        InputFormat::Cem => "cem",
        InputFormat::Html => "html",
        InputFormat::Xml => "xml",
    }
}

fn normalized_root_scope(
    authored: &ScopeConfig,
    effective: &ScopeConfig,
    direction: NormalizedScopeDirection,
    index: usize,
    collection: &str,
    base_uri: Option<&str>,
    resolver_ids: &[String],
    diagnostics: &mut Vec<Diagnostic>,
) -> NormalizedRootScope {
    let field_prefix = format!("{collection}[{index}].rootScope");
    validate_scope_config_with_paths(effective, &field_prefix, base_uri, diagnostics);
    let (policy, budgets) =
        normalized_policy_and_budgets(effective, &field_prefix, base_uri, diagnostics);

    NormalizedRootScope {
        scope_id: format!("scope:{}:{index}", direction_id(direction)),
        direction,
        identity: effective.format_identity(),
        default_namespace: effective.default_namespace.clone(),
        namespaces: effective.namespaces.clone(),
        version_pins: effective.version_pins.clone(),
        base_uri: effective.base_uri.clone(),
        resolver_context: NormalizedResolverContext {
            resolver_ids: resolver_ids.to_vec(),
        },
        module_map: normalized_module_map_identity(authored, effective, base_uri),
        policy,
        budgets,
        output_pipeline: normalized_output_pipeline(effective),
        provenance: Vec::new(),
    }
}

fn normalized_policy_and_budgets(
    scope: &ScopeConfig,
    field_prefix: &str,
    base_uri: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> (NormalizedScopePolicy, NormalizedBudgets) {
    let policy_name = scope.policy.clone();
    let mut policy = match scope.policy.as_deref().map(normalize_key) {
        Some(name) if name == "host" => ScopePolicy::host_root(),
        Some(name) if name == "deterministic" || name == "default" => deterministic_scope_policy(),
        Some(_) => {
            diagnostics.push(config_field_diagnostic(
                "cem.run_config.scope_policy_unenforced",
                Severity::Warning,
                "scope policy is parsed and preserved, but runtime enforcement is not implemented yet"
                    .to_owned(),
                base_uri,
                &format!("{field_prefix}.policy"),
                None,
            ));
            deterministic_scope_policy()
        }
        None => deterministic_scope_policy(),
    };
    let mut budgets = NormalizedBudgets::default();

    for (field, value) in &scope.budgets {
        let field_path = format!("{field_prefix}.budgets.{field}");
        match normalize_key(field).as_str() {
            "cpu" | "cpuworkers" => match parse_u32_budget_value(field, value) {
                Ok(value) => policy.cpu_workers = value,
                Err(message) => diagnostics.push(budget_invalid_diagnostic(
                    message, base_uri, &field_path,
                )),
            },
            "queue" | "queuesize" => match parse_u32_budget_value(field, value) {
                Ok(value) => policy.queue_size = value,
                Err(message) => diagnostics.push(budget_invalid_diagnostic(
                    message, base_uri, &field_path,
                )),
            },
            "io" | "iostreams" => match parse_u32_budget_value(field, value) {
                Ok(value) => policy.io_streams = value,
                Err(message) => diagnostics.push(budget_invalid_diagnostic(
                    message, base_uri, &field_path,
                )),
            },
            "memory" | "memorybytes" => match parse_u64_budget_value(field, value) {
                Ok(value) => {
                    policy.memory_bytes = value;
                    budgets.memory_bytes = Some(value);
                }
                Err(message) => diagnostics.push(budget_invalid_diagnostic(
                    message, base_uri, &field_path,
                )),
            },
            "pluginms" | "plugintimebudgetms" => match parse_u64_budget_value(field, value) {
                Ok(value) => {
                    policy.plugin_time_budget_ms = Some(value);
                    budgets.plugin_ms = Some(value);
                }
                Err(message) => diagnostics.push(budget_invalid_diagnostic(
                    message, base_uri, &field_path,
                )),
            },
            "parsems" | "parsetimebudgetms" => {
                set_time_budget(&mut budgets.parse_ms, field, value, base_uri, &field_path, diagnostics)
            }
            "validatems" | "validatetimebudgetms" => set_time_budget(
                &mut budgets.validate_ms,
                field,
                value,
                base_uri,
                &field_path,
                diagnostics,
            ),
            "checkms" | "checktimebudgetms" => {
                set_time_budget(&mut budgets.check_ms, field, value, base_uri, &field_path, diagnostics)
            }
            "convertms" | "converttimebudgetms" => set_time_budget(
                &mut budgets.convert_ms,
                field,
                value,
                base_uri,
                &field_path,
                diagnostics,
            ),
            "tracems" | "tracetimebudgetms" => {
                set_time_budget(&mut budgets.trace_ms, field, value, base_uri, &field_path, diagnostics)
            }
            "inspectms" | "inspecttimebudgetms" => set_time_budget(
                &mut budgets.inspect_ms,
                field,
                value,
                base_uri,
                &field_path,
                diagnostics,
            ),
            "benchms" | "benchtimebudgetms" => {
                set_time_budget(&mut budgets.bench_ms, field, value, base_uri, &field_path, diagnostics)
            }
            "fixturevalidatems" | "fixturevalidatetimebudgetms" => set_time_budget(
                &mut budgets.fixture_validate_ms,
                field,
                value,
                base_uri,
                &field_path,
                diagnostics,
            ),
            "fixtureroundtripms" | "fixtureroundtriptimebudgetms" => set_time_budget(
                &mut budgets.fixture_roundtrip_ms,
                field,
                value,
                base_uri,
                &field_path,
                diagnostics,
            ),
            "observems" | "observetimebudgetms" => set_time_budget(
                &mut budgets.observe_ms,
                field,
                value,
                base_uri,
                &field_path,
                diagnostics,
            ),
            "overflow" => match normalize_key(value).as_str() {
                "block" => policy.overflow = OverflowPolicy::Block,
                "reject" => policy.overflow = OverflowPolicy::Reject,
                "spilltoparent" => policy.overflow = OverflowPolicy::SpillToParent,
                _ => diagnostics.push(budget_invalid_diagnostic(
                    format!("budget `overflow` expects block, reject, or spill-to-parent, got `{value}`"),
                    base_uri,
                    &field_path,
                )),
            },
            _ => {
                budgets.unknown.push(NormalizedBudgetEntry {
                    name: field.clone(),
                    value: value.clone(),
                });
                diagnostics.push(config_field_diagnostic(
                    "cem.run_config.scope_budget_unknown",
                    Severity::Warning,
                    format!("budget `{field}` is parsed and preserved, but runtime enforcement is not implemented yet"),
                    base_uri,
                    &field_path,
                    None,
                ));
            }
        }
    }

    (
        NormalizedScopePolicy::from_scope_policy(policy_name, policy),
        budgets,
    )
}

fn set_time_budget(
    target: &mut Option<u64>,
    field: &str,
    value: &str,
    base_uri: Option<&str>,
    field_path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match parse_u64_budget_value(field, value) {
        Ok(value) => *target = Some(value),
        Err(message) => diagnostics.push(budget_invalid_diagnostic(message, base_uri, field_path)),
    }
}

fn budget_invalid_diagnostic(
    message: String,
    base_uri: Option<&str>,
    field_path: &str,
) -> Diagnostic {
    config_field_diagnostic(
        "cem.run_config.scope_budget_invalid",
        Severity::Fatal,
        message,
        base_uri,
        field_path,
        None,
    )
}

fn deterministic_scope_policy() -> ScopePolicy {
    ScopePolicy {
        cpu_workers: 1,
        queue_size: 8,
        io_streams: 4,
        memory_bytes: 8 * 1024 * 1024,
        plugin_time_budget_ms: None,
        overflow: OverflowPolicy::Reject,
    }
}

fn normalized_module_map_identity(
    authored: &ScopeConfig,
    effective: &ScopeConfig,
    base_uri: Option<&str>,
) -> Option<NormalizedModuleMapIdentity> {
    let resolved = effective.module_map.as_deref()?;
    let declared = authored
        .module_map
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(resolved);
    let resolved_uri = changed_value(declared, resolved);
    let content_type = infer_content_type_from_path(resolved)
        .or_else(|| Some(crate::schema::registry::JSON_CONTENT_TYPE.to_owned()));
    Some(NormalizedModuleMapIdentity {
        declared_uri: declared.to_owned(),
        resolved_uri,
        content_type,
        entries_hash: None,
        resolver_id: None,
        base_uri: base_uri.map(str::to_owned),
        state: if resolved.trim().is_empty() {
            NormalizedModuleMapState::Invalid
        } else {
            NormalizedModuleMapState::Valid
        },
        diagnostics: Vec::new(),
        provenance: Vec::new(),
    })
}

fn normalized_output_pipeline(scope: &ScopeConfig) -> Option<NormalizedOutputPipeline> {
    let pipeline = NormalizedOutputPipeline {
        output_color_type: scope.output_color_type.clone(),
        cemt_formatter: scope.cemt_formatter.clone(),
        cemt_formatter_profile: scope.cemt_formatter_profile.clone(),
        cemt_colorizer: scope.cemt_colorizer.clone(),
        cemt_color_profile: scope.cemt_color_profile.clone(),
    };
    (pipeline.output_color_type.is_some()
        || pipeline.cemt_formatter.is_some()
        || pipeline.cemt_formatter_profile.is_some()
        || pipeline.cemt_colorizer.is_some()
        || pipeline.cemt_color_profile.is_some())
    .then_some(pipeline)
}

fn normalized_resolver_binding(resolver: &ResolverSpec, index: usize) -> NormalizedResolverBinding {
    let scheme = resolver
        .uri_prefix
        .split_once(':')
        .map(|(scheme, _)| scheme)
        .filter(|scheme| !scheme.is_empty())
        .unwrap_or("file")
        .to_owned();
    let mut directions = Vec::new();
    let mut purposes = Vec::new();
    if resolver.read {
        directions.push(NormalizedResolverDirection::Read);
        purposes.extend([
            NormalizedResolverPurpose::Config,
            NormalizedResolverPurpose::Input,
            NormalizedResolverPurpose::Template,
            NormalizedResolverPurpose::ModuleMap,
        ]);
    }
    if resolver.write {
        directions.push(NormalizedResolverDirection::Write);
        purposes.extend([
            NormalizedResolverPurpose::Output,
            NormalizedResolverPurpose::Report,
            NormalizedResolverPurpose::ObserveEvents,
        ]);
    }

    NormalizedResolverBinding {
        resolver_id: format!(
            "resolver:{}:{scheme}:{index}",
            resolver_purpose_id(&purposes)
        ),
        scheme,
        purposes,
        directions,
        declared_uri_prefix: resolver.uri_prefix.clone(),
        resolved_local_root: Some(resolver.local_root.clone()),
        support: NormalizedResolverSupport::Required,
        provenance: Vec::new(),
    }
}

fn validate_resolver_specs_with_paths(
    resolvers: &[ResolverSpec],
    base_uri: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (index, resolver) in resolvers.iter().enumerate() {
        let prefix = format!("resolvers[{index}]");
        if resolver.uri_prefix.trim().is_empty() {
            diagnostics.push(config_field_diagnostic(
                "cem.run_config.resolver_uri_prefix_invalid",
                Severity::Fatal,
                format!("resolver spec at index {index} requires `uriPrefix`"),
                base_uri,
                &format!("{prefix}.uriPrefix"),
                None,
            ));
        }
        if resolver.local_root.trim().is_empty() {
            diagnostics.push(config_field_diagnostic(
                "cem.run_config.resolver_local_root_invalid",
                Severity::Fatal,
                format!("resolver spec at index {index} requires `localRoot`"),
                base_uri,
                &format!("{prefix}.localRoot"),
                None,
            ));
        }
        if !resolver.read && !resolver.write {
            diagnostics.push(config_field_diagnostic(
                "cem.run_config.resolver_direction_invalid",
                Severity::Fatal,
                format!("resolver spec at index {index} must enable read or write"),
                base_uri,
                &prefix,
                None,
            ));
        }
    }
}

fn validate_scope_config_with_paths(
    scope: &ScopeConfig,
    field_prefix: &str,
    base_uri: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(module_map) = scope.module_map.as_deref() {
        if module_map.trim().is_empty() {
            diagnostics.push(config_field_diagnostic(
                "cem.run_config.scope_module_map_invalid",
                Severity::Fatal,
                "root scope has an empty moduleMap".to_owned(),
                base_uri,
                &format!("{field_prefix}.moduleMap"),
                None,
            ));
        }
    }

    if let Some(default_namespace) = scope.default_namespace.as_deref() {
        if default_namespace.trim().is_empty() {
            diagnostics.push(config_field_diagnostic(
                "cem.run_config.scope_namespace_invalid",
                Severity::Fatal,
                "root scope has an empty defaultNamespace URI".to_owned(),
                base_uri,
                &format!("{field_prefix}.defaultNamespace"),
                None,
            ));
        }
    }

    for (prefix, uri) in &scope.namespaces {
        if !valid_namespace_prefix(prefix) {
            diagnostics.push(config_field_diagnostic(
                "cem.run_config.scope_namespace_invalid",
                Severity::Fatal,
                format!("root scope has invalid namespace prefix `{prefix}`"),
                base_uri,
                &format!("{field_prefix}.namespaces.{prefix}"),
                None,
            ));
        }
        if uri.trim().is_empty() {
            diagnostics.push(config_field_diagnostic(
                "cem.run_config.scope_namespace_invalid",
                Severity::Fatal,
                format!("root scope has an empty namespace URI for `{prefix}`"),
                base_uri,
                &format!("{field_prefix}.namespaces.{prefix}"),
                None,
            ));
        }
        if prefix == "xml" && uri != "http://www.w3.org/XML/1998/namespace" {
            diagnostics.push(config_field_diagnostic(
                "cem.run_config.scope_namespace_invalid",
                Severity::Fatal,
                format!("root scope binds reserved prefix `xml` to `{uri}`"),
                base_uri,
                &format!("{field_prefix}.namespaces.xml"),
                None,
            ));
        }
        if prefix == "xmlns" {
            diagnostics.push(config_field_diagnostic(
                "cem.run_config.scope_namespace_invalid",
                Severity::Fatal,
                "root scope uses reserved prefix `xmlns`".to_owned(),
                base_uri,
                &format!("{field_prefix}.namespaces.xmlns"),
                None,
            ));
        }
    }

    for (name, constraint) in &scope.version_pins {
        if name.trim().is_empty() {
            diagnostics.push(config_field_diagnostic(
                "cem.run_config.scope_version_pin_invalid",
                Severity::Fatal,
                "root scope has an empty versionPins key".to_owned(),
                base_uri,
                &format!("{field_prefix}.versionPins"),
                None,
            ));
        }
        if constraint.trim().is_empty() {
            diagnostics.push(config_field_diagnostic(
                "cem.run_config.scope_version_pin_invalid",
                Severity::Fatal,
                format!("root scope has an empty versionPins value for `{name}`"),
                base_uri,
                &format!("{field_prefix}.versionPins.{name}"),
                None,
            ));
        }
    }
}

fn valid_namespace_prefix(prefix: &str) -> bool {
    !prefix.trim().is_empty()
        && !prefix.contains(':')
        && !prefix.chars().any(char::is_whitespace)
        && prefix
            .chars()
            .next()
            .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
        && prefix
            .chars()
            .all(|ch| ch == '_' || ch == '-' || ch == '.' || ch.is_ascii_alphanumeric())
}

fn normalized_output_input_id(
    output: &OutputSpec,
    inputs: &[InputSpec],
    output_index: usize,
    base_uri: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    if let Some(input_ref) = output.input_ref.as_deref() {
        if let Some(index) = inputs.iter().position(|input| input.uri == input_ref) {
            return Some(format!("input:{index}"));
        }
        diagnostics.push(config_field_diagnostic(
            "cem.run_config.output_input_ref_unknown",
            Severity::Fatal,
            format!("output spec at index {output_index} references unknown input `{input_ref}`"),
            base_uri,
            &format!("outputs[{output_index}].inputRef"),
            Some(format!("output:{output_index}")),
        ));
        return None;
    }

    if inputs.len() == 1 {
        Some("input:0".to_owned())
    } else if inputs.len() > 1 {
        diagnostics.push(config_field_diagnostic(
            "cem.run_config.output_input_ref_ambiguous",
            Severity::Fatal,
            format!(
                "output spec at index {output_index} must declare `inputRef` when multiple inputs are configured"
            ),
            base_uri,
            &format!("outputs[{output_index}].inputRef"),
            Some(format!("output:{output_index}")),
        ));
        None
    } else {
        None
    }
}

pub fn schema_package_manifest_input(uri: &str, mut root_scope: ScopeConfig) -> EngineInput {
    if root_scope.default_content_type.is_none() {
        root_scope.default_content_type =
            Some(crate::schema::registry::CEM_SCHEMA_PACKAGE_CONTENT_TYPE.to_owned());
    }
    if root_scope.schema.is_none() {
        root_scope.schema = Some(crate::schema::registry::CEM_SCHEMA_PACKAGE_URI.to_owned());
    }
    EngineInput {
        uri: uri.to_owned(),
        bytes: Vec::new(),
        from_format: Some(InputFormat::Cem),
        identity: root_scope.format_identity_option(),
        root_scope,
    }
}

pub fn parse_input_spec_record(record: &str) -> Result<InputSpec, SpecParseError> {
    let fields = parse_key_value_record(record)?;
    let mut spec = InputSpec::default();

    for (key, value) in fields {
        match normalize_key(&key).as_str() {
            "uri" | "path" => spec.uri = value,
            key => apply_scope_field(&mut spec.root_scope, key, value)?,
        }
    }

    if spec.uri.trim().is_empty() {
        return Err(parse_error("input spec requires `uri` or `path`"));
    }

    Ok(spec)
}

pub fn parse_output_spec_record(record: &str) -> Result<OutputSpec, SpecParseError> {
    let fields = parse_key_value_record(record)?;
    let mut spec = OutputSpec::default();

    for (key, value) in fields {
        match normalize_key(&key).as_str() {
            "input" | "inputref" => spec.input_ref = Some(value),
            "dest" | "destination" | "out" => spec.destination = Some(value),
            key => apply_scope_field(&mut spec.root_scope, key, value)?,
        }
    }

    Ok(spec)
}

pub fn validate_run_config(config: &RunConfig, base_uri: Option<&str>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut input_uris = std::collections::BTreeSet::new();
    let mut schema_package_uris = std::collections::BTreeSet::new();

    for (index, input) in config.inputs.iter().enumerate() {
        if input.uri.trim().is_empty() {
            diagnostics.push(config_diagnostic(
                "cem.run_config.input_uri_missing",
                format!("input spec at index {index} requires `uri`"),
                base_uri,
            ));
        } else if !input_uris.insert(input.uri.clone()) {
            diagnostics.push(config_diagnostic(
                "cem.run_config.input_uri_duplicate",
                format!("input URI `{}` is declared more than once", input.uri),
                base_uri,
            ));
        }
        validate_scope_config(
            &input.root_scope,
            "input",
            index,
            base_uri,
            &mut diagnostics,
        );
    }

    for (index, package) in config.schema_packages.iter().enumerate() {
        if package.uri.trim().is_empty() {
            diagnostics.push(config_diagnostic(
                "cem.run_config.schema_package_uri_missing",
                format!("schema package spec at index {index} requires `uri`"),
                base_uri,
            ));
        } else if !schema_package_uris.insert(package.uri.clone()) {
            diagnostics.push(config_diagnostic(
                "cem.run_config.schema_package_uri_duplicate",
                format!(
                    "schema package URI `{}` is declared more than once",
                    package.uri
                ),
                base_uri,
            ));
        }
        validate_scope_config(
            &package.root_scope,
            "schema package",
            index,
            base_uri,
            &mut diagnostics,
        );
    }

    for (index, output) in config.outputs.iter().enumerate() {
        if let Some(input_ref) = output.input_ref.as_deref() {
            if !input_uris.contains(input_ref) {
                diagnostics.push(config_diagnostic(
                    "cem.run_config.output_input_ref_unknown",
                    format!("output spec at index {index} references unknown input `{input_ref}`"),
                    base_uri,
                ));
            }
        }
        validate_scope_config(
            &output.root_scope,
            "output",
            index,
            base_uri,
            &mut diagnostics,
        );
    }

    diagnostics
}

fn validate_run_config_defaults(
    defaults: &RunConfigDefaults,
    base_uri: Option<&str>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    validate_scope_config(
        &defaults.input_scope,
        "input default",
        0,
        base_uri,
        &mut diagnostics,
    );
    validate_scope_config(
        &defaults.output_scope,
        "output default",
        0,
        base_uri,
        &mut diagnostics,
    );
    diagnostics
}

fn apply_scope_field(
    scope: &mut ScopeConfig,
    normalized_key: &str,
    value: String,
) -> Result<(), SpecParseError> {
    match normalized_key {
        "contenttype" | "defaultcontenttype" => scope.default_content_type = Some(value),
        "schema" => scope.schema = Some(value),
        "cemtformatter" | "formatter" => scope.cemt_formatter = Some(value),
        "cemtformatterprofile" | "formatterprofile" => scope.cemt_formatter_profile = Some(value),
        "cemtcolorizer" | "colorizer" => scope.cemt_colorizer = Some(value),
        "cemtcolorprofile" | "colorprofile" => scope.cemt_color_profile = Some(value),
        "defaultnamespace" | "defaultns" => scope.default_namespace = Some(value),
        "modulemap" => scope.module_map = Some(value),
        "baseuri" => scope.base_uri = Some(value),
        "policy" => scope.policy = Some(value),
        "namespaces" | "ns" => scope.namespaces = parse_map_field("namespaces", &value)?,
        "versions" | "versionpins" => scope.version_pins = parse_map_field("versionPins", &value)?,
        "budgets" => scope.budgets = parse_map_field("budgets", &value)?,
        other => {
            return Err(parse_error(format!(
                "unsupported spec field `{other}`; use config files for nested data"
            )));
        }
    }
    Ok(())
}

fn parse_key_value_record(record: &str) -> Result<Vec<(String, String)>, SpecParseError> {
    let mut fields = Vec::new();
    for field in split_escaped(record, ',')? {
        if field.trim().is_empty() {
            continue;
        }
        let Some((key, value)) = split_key_value(&field)? else {
            return Err(parse_error(format!(
                "spec field `{}` is missing `=`",
                field.trim()
            )));
        };
        fields.push((key.trim().to_owned(), value.trim().to_owned()));
    }
    Ok(fields)
}

fn split_key_value(field: &str) -> Result<Option<(String, String)>, SpecParseError> {
    let mut in_quote = false;
    let mut escape = false;

    for (idx, ch) in field.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match ch {
            '\\' => escape = true,
            '"' => in_quote = !in_quote,
            '=' if !in_quote => {
                let key = field[..idx].to_owned();
                let value = unquote(field[idx + 1..].trim())?;
                return Ok(Some((key, value)));
            }
            _ => {}
        }
    }

    if in_quote {
        return Err(parse_error("unterminated quoted spec field"));
    }
    Ok(None)
}

fn parse_map_field(
    field_name: &str,
    value: &str,
) -> Result<BTreeMap<String, String>, SpecParseError> {
    let mut map = BTreeMap::new();
    if value.trim().is_empty() {
        return Ok(map);
    }

    for pair in split_escaped(value, '|')? {
        let Some((key, value)) = pair.split_once(':') else {
            return Err(parse_error(format!(
                "{field_name} map entry `{pair}` is missing `:` separator"
            )));
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() {
            return Err(parse_error(format!(
                "{field_name} map entries require a non-empty key"
            )));
        }
        if value.is_empty() {
            return Err(parse_error(format!(
                "{field_name} map entry `{key}` requires a non-empty value"
            )));
        }
        map.insert(key.to_owned(), value.to_owned());
    }

    Ok(map)
}

fn split_escaped(input: &str, delimiter: char) -> Result<Vec<String>, SpecParseError> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut escape = false;

    for ch in input.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }

        match ch {
            '\\' => escape = true,
            '"' => {
                in_quote = !in_quote;
                current.push(ch);
            }
            c if c == delimiter && !in_quote => {
                parts.push(current.trim().to_owned());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if escape {
        current.push('\\');
    }
    if in_quote {
        return Err(parse_error("unterminated quoted spec record"));
    }

    parts.push(current.trim().to_owned());
    Ok(parts)
}

fn unquote(value: &str) -> Result<String, SpecParseError> {
    let trimmed = value.trim();
    if !(trimmed.starts_with('"') || trimmed.ends_with('"')) {
        return Ok(trimmed.to_owned());
    }
    if !(trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2) {
        return Err(parse_error(format!("malformed quoted value `{trimmed}`")));
    }
    Ok(trimmed[1..trimmed.len() - 1].to_owned())
}

fn normalize_key(key: &str) -> String {
    key.chars()
        .filter(|ch| *ch != '-' && *ch != '_' && !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn parse_error(message: impl Into<String>) -> SpecParseError {
    SpecParseError {
        message: message.into(),
    }
}

fn run_config_error(code: &'static str, message: impl Into<String>) -> RunConfigError {
    RunConfigError {
        code,
        message: message.into(),
    }
}

fn content_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

fn config_diagnostic(code: &str, message: String, base_uri: Option<&str>) -> Diagnostic {
    Diagnostic {
        uri: base_uri.map(str::to_owned),
        code: code.to_owned(),
        severity: Severity::Fatal,
        message,
        ..Diagnostic::default()
    }
}

fn config_field_diagnostic(
    code: &str,
    severity: Severity,
    message: String,
    base_uri: Option<&str>,
    field_path: &str,
    normalized_id: Option<String>,
) -> Diagnostic {
    let mut details = serde_json::Map::new();
    details.insert(
        "fieldPath".to_owned(),
        serde_json::Value::String(field_path.to_owned()),
    );
    if let Some(normalized_id) = normalized_id {
        details.insert(
            "normalizedId".to_owned(),
            serde_json::Value::String(normalized_id),
        );
    }
    Diagnostic {
        uri: base_uri.map(str::to_owned),
        code: code.to_owned(),
        severity,
        message,
        details: Some(serde_json::Value::Object(details)),
        ..Diagnostic::default()
    }
}

fn record_parse_diagnostic(
    code: &str,
    collection: &str,
    index: usize,
    error: SpecParseError,
    base_uri: Option<&str>,
) -> Diagnostic {
    config_field_diagnostic(
        code,
        Severity::Fatal,
        error.message,
        base_uri,
        &format!("{collection}[{index}]"),
        None,
    )
}

fn normalized_config_source_kind(
    has_config_bytes: bool,
    has_records: bool,
    base_uri: Option<&str>,
) -> NormalizedConfigSourceKind {
    if !has_config_bytes && has_records {
        return NormalizedConfigSourceKind::CliRecords;
    }
    if !has_config_bytes {
        return NormalizedConfigSourceKind::HostObject;
    }

    let Some(base_uri) = base_uri.map(str::trim).filter(|base| !base.is_empty()) else {
        return NormalizedConfigSourceKind::Bytes;
    };
    if !has_uri_scheme(base_uri) {
        return NormalizedConfigSourceKind::File;
    }
    if base_uri.starts_with("file:") {
        NormalizedConfigSourceKind::FileUri
    } else {
        NormalizedConfigSourceKind::CustomUri
    }
}

fn normalized_config_identity(
    request: &NormalizedRunPlanRequest,
    source_kind: NormalizedConfigSourceKind,
    has_config_bytes: bool,
) -> NormalizedConfigIdentity {
    NormalizedConfigIdentity {
        declared_uri: request.config_base_uri.clone(),
        resolved_uri: resolved_config_uri(request.config_base_uri.as_deref()),
        content_type: request
            .config_identity
            .content_type
            .clone()
            .or_else(|| has_config_bytes.then(|| "application/json".to_owned())),
        schema_identity: request
            .config_identity
            .schema
            .clone()
            .or_else(|| Some(RUN_CONFIG_SCHEMA_URI.to_owned())),
        namespace_identity: request.config_identity.default_namespace.clone(),
        source_kind,
        source_range: None,
    }
}

fn resolved_config_uri(base_uri: Option<&str>) -> Option<String> {
    let base_uri = base_uri?.trim();
    if base_uri.is_empty() {
        return None;
    }
    if base_uri.starts_with("file:") {
        return local_file_uri_to_path(base_uri).map(|path| path.to_string_lossy().into_owned());
    }
    Some(base_uri.to_owned())
}

fn changed_value(authored: &str, normalized: &str) -> Option<String> {
    (authored != normalized).then(|| normalized.to_owned())
}

fn parse_u32_budget_value(field: &str, value: &str) -> Result<u32, String> {
    value
        .trim()
        .parse::<u32>()
        .map(|value| value.max(1))
        .map_err(|_| format!("budget `{field}` expects an unsigned integer, got `{value}`"))
}

fn parse_u64_budget_value(field: &str, value: &str) -> Result<u64, String> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| format!("budget `{field}` expects an unsigned integer, got `{value}`"))
}

fn stable_run_id(
    config: &RunConfig,
    diagnostics_mode: &NormalizedDiagnosticsMode,
    command_profile: Option<&str>,
) -> String {
    let payload = serde_json::to_string(&(config, diagnostics_mode, command_profile))
        .unwrap_or_else(|_| String::new());
    format!("run:{:016x}", stable_hash(payload.as_bytes()))
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn direction_id(direction: NormalizedScopeDirection) -> &'static str {
    match direction {
        NormalizedScopeDirection::Input => "input",
        NormalizedScopeDirection::Output => "output",
        NormalizedScopeDirection::SchemaPackage => "schemaPackage",
    }
}

fn resolver_purpose_id(purposes: &[NormalizedResolverPurpose]) -> &'static str {
    if purposes.contains(&NormalizedResolverPurpose::Input) {
        "read"
    } else if purposes.contains(&NormalizedResolverPurpose::Output) {
        "write"
    } else {
        "none"
    }
}

fn validate_scope_config(
    scope: &ScopeConfig,
    direction: &str,
    index: usize,
    base_uri: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(module_map) = scope.module_map.as_deref() {
        if module_map.trim().is_empty() {
            diagnostics.push(config_diagnostic(
                "cem.run_config.scope_module_map_invalid",
                format!("{direction} scope at index {index} has an empty moduleMap"),
                base_uri,
            ));
        }
    }

    if let Some(default_namespace) = scope.default_namespace.as_deref() {
        validate_namespace_uri(
            "defaultNamespace",
            default_namespace,
            direction,
            index,
            base_uri,
            diagnostics,
        );
    }

    for (prefix, uri) in &scope.namespaces {
        validate_namespace_prefix(prefix, direction, index, base_uri, diagnostics);
        validate_namespace_uri(
            &format!("namespaces.{prefix}"),
            uri,
            direction,
            index,
            base_uri,
            diagnostics,
        );
        if prefix == "xml" && uri != "http://www.w3.org/XML/1998/namespace" {
            diagnostics.push(config_diagnostic(
                "cem.run_config.scope_namespace_invalid",
                format!(
                    "{direction} scope at index {index} binds reserved prefix `xml` to `{uri}`"
                ),
                base_uri,
            ));
        }
        if prefix == "xmlns" {
            diagnostics.push(config_diagnostic(
                "cem.run_config.scope_namespace_invalid",
                format!("{direction} scope at index {index} uses reserved prefix `xmlns`"),
                base_uri,
            ));
        }
    }

    for (name, constraint) in &scope.version_pins {
        if name.trim().is_empty() {
            diagnostics.push(config_diagnostic(
                "cem.run_config.scope_version_pin_invalid",
                format!("{direction} scope at index {index} has an empty versionPins key"),
                base_uri,
            ));
        }
        if constraint.trim().is_empty() {
            diagnostics.push(config_diagnostic(
                "cem.run_config.scope_version_pin_invalid",
                format!(
                    "{direction} scope at index {index} has an empty versionPins value for `{name}`"
                ),
                base_uri,
            ));
        }
    }
}

fn validate_namespace_prefix(
    prefix: &str,
    direction: &str,
    index: usize,
    base_uri: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let valid = !prefix.trim().is_empty()
        && !prefix.contains(':')
        && !prefix.chars().any(char::is_whitespace)
        && prefix
            .chars()
            .next()
            .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
        && prefix
            .chars()
            .all(|ch| ch == '_' || ch == '-' || ch == '.' || ch.is_ascii_alphanumeric());
    if !valid {
        diagnostics.push(config_diagnostic(
            "cem.run_config.scope_namespace_invalid",
            format!("{direction} scope at index {index} has invalid namespace prefix `{prefix}`"),
            base_uri,
        ));
    }
}

fn validate_namespace_uri(
    field: &str,
    uri: &str,
    direction: &str,
    index: usize,
    base_uri: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if uri.trim().is_empty() {
        diagnostics.push(config_diagnostic(
            "cem.run_config.scope_namespace_invalid",
            format!("{direction} scope at index {index} has an empty {field} URI"),
            base_uri,
        ));
    }
}

fn merge_scope_defaults(scope: &mut ScopeConfig, defaults: &ScopeConfig) {
    if scope.default_content_type.is_none() {
        scope.default_content_type = defaults.default_content_type.clone();
    }
    if scope.schema.is_none() {
        scope.schema = defaults.schema.clone();
    }
    if scope.output_color_type.is_none() {
        scope.output_color_type = defaults.output_color_type.clone();
    }
    if scope.cemt_formatter.is_none() {
        scope.cemt_formatter = defaults.cemt_formatter.clone();
    }
    if scope.cemt_formatter_profile.is_none() {
        scope.cemt_formatter_profile = defaults.cemt_formatter_profile.clone();
    }
    if scope.cemt_colorizer.is_none() {
        scope.cemt_colorizer = defaults.cemt_colorizer.clone();
    }
    if scope.cemt_color_profile.is_none() {
        scope.cemt_color_profile = defaults.cemt_color_profile.clone();
    }
    if scope.default_namespace.is_none() {
        scope.default_namespace = defaults.default_namespace.clone();
    }
    if scope.module_map.is_none() {
        scope.module_map = defaults.module_map.clone();
    }
    if scope.base_uri.is_none() {
        scope.base_uri = defaults.base_uri.clone();
    }
    if scope.policy.is_none() {
        scope.policy = defaults.policy.clone();
    }

    let mut version_pins = defaults.version_pins.clone();
    version_pins.extend(scope.version_pins.clone());
    scope.version_pins = version_pins;

    let mut namespaces = defaults.namespaces.clone();
    namespaces.extend(scope.namespaces.clone());
    scope.namespaces = namespaces;

    let mut budgets = defaults.budgets.clone();
    budgets.extend(scope.budgets.clone());
    scope.budgets = budgets;
}

fn resolve_scope_module_map(scope: &mut ScopeConfig, base_uri: Option<&str>) {
    let Some(module_map) = scope.module_map.as_deref() else {
        return;
    };
    let Some(resolved) = resolve_relative_path_like(module_map, base_uri) else {
        return;
    };
    scope.module_map = Some(resolved);
}

fn resolve_output_destination(output: &mut OutputSpec, base_uri: Option<&str>) {
    let Some(destination) = output.destination.as_deref() else {
        return;
    };
    let Some(resolved) = resolve_relative_path_like(destination, base_uri) else {
        return;
    };
    output.destination = Some(resolved);
}

fn resolve_relative_path_like(value: &str, base_uri: Option<&str>) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || has_uri_scheme(trimmed) || std::path::Path::new(trimmed).is_absolute()
    {
        return None;
    }
    let base = base_uri?.trim();
    if base.is_empty() {
        return None;
    }

    let local_file_base;
    let base_path = if has_uri_scheme(base) {
        local_file_base = local_file_uri_to_path(base)?;
        local_file_base.as_path()
    } else {
        std::path::Path::new(base)
    };
    let base_dir = if base.ends_with('/') {
        base_path
    } else {
        base_path.parent()?
    };
    Some(base_dir.join(trimmed).to_string_lossy().into_owned())
}

pub fn infer_content_type_from_path(path: &str) -> Option<String> {
    let lower_path = path.to_ascii_lowercase();
    if lower_path.ends_with(".schema.json") {
        return Some(crate::schema::registry::JSON_SCHEMA_CONTENT_TYPE.to_owned());
    }

    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())?
        .to_ascii_lowercase();
    match ext.as_str() {
        "cem" => Some("application/cem+xml".to_owned()),
        "html" | "htm" => Some(crate::schema::registry::HTML_CONTENT_TYPE.to_owned()),
        "xhtml" | "xht" => Some(crate::schema::registry::XHTML_CONTENT_TYPE.to_owned()),
        "xml" => Some(crate::schema::registry::XML_CONTENT_TYPE.to_owned()),
        "rng" => Some(crate::schema::registry::RELAX_NG_XML_CONTENT_TYPE.to_owned()),
        "rnc" => Some(crate::schema::registry::RELAX_NG_COMPACT_CONTENT_TYPE.to_owned()),
        "svg" | "svgz" => Some(crate::schema::registry::SVG_CONTENT_TYPE.to_owned()),
        "mml" | "mathml" => Some(crate::schema::registry::MATHML_CONTENT_TYPE.to_owned()),
        "xsl" | "xslt" => Some(crate::schema::registry::XSLT_CONTENT_TYPE.to_owned()),
        "css" => Some(crate::schema::registry::CSS_CONTENT_TYPE.to_owned()),
        "cemt" => Some(crate::schema::registry::CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
        "cemql" => Some(crate::schema::registry::CEM_QL_CONTENT_TYPE.to_owned()),
        "jsonschema" => Some(crate::schema::registry::JSON_SCHEMA_CONTENT_TYPE.to_owned()),
        "json" => Some(crate::schema::registry::JSON_CONTENT_TYPE.to_owned()),
        "yaml" | "yml" => Some(crate::schema::registry::YAML_CONTENT_TYPE.to_owned()),
        "csv" => Some(crate::schema::registry::CSV_CONTENT_TYPE.to_owned()),
        "md" | "markdown" => Some(crate::schema::registry::MARKDOWN_CONTENT_TYPE.to_owned()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagnostic_field_paths(diagnostics: &[Diagnostic]) -> Vec<String> {
        diagnostics
            .iter()
            .filter_map(|diagnostic| {
                diagnostic
                    .details
                    .as_ref()
                    .and_then(|details| details.get("fieldPath"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .collect()
    }

    fn has_field_path(diagnostics: &[Diagnostic], code: &str, field_path: &str) -> bool {
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == code
                && diagnostic
                    .details
                    .as_ref()
                    .and_then(|details| details.get("fieldPath"))
                    .and_then(serde_json::Value::as_str)
                    == Some(field_path)
        })
    }

    #[test]
    fn input_spec_record_maps_identity_and_scope_fields() {
        let spec = parse_input_spec_record(
            r#"uri=src/a.cem,contentType=application/cem+xml,schema=https://cem.dev/ns/core/1,defaultNs=https://cem.dev/ns/core,namespaces=html:https://www.w3.org/1999/xhtml|svg:http://www.w3.org/2000/svg,moduleMap=cem.modules.json"#,
        )
        .unwrap();

        assert_eq!(spec.uri, "src/a.cem");
        assert_eq!(
            spec.root_scope.default_content_type.as_deref(),
            Some("application/cem+xml")
        );
        assert_eq!(
            spec.root_scope.schema.as_deref(),
            Some("https://cem.dev/ns/core/1")
        );
        assert_eq!(
            spec.root_scope.namespaces.get("html").map(String::as_str),
            Some("https://www.w3.org/1999/xhtml")
        );
        assert_eq!(
            spec.root_scope.module_map.as_deref(),
            Some("cem.modules.json")
        );
    }

    #[test]
    fn spec_record_supports_quoted_commas() {
        let spec = parse_input_spec_record(
            r#"uri="src/a,one.cem",contentType="text/custom-element-xslt""#,
        )
        .unwrap();

        assert_eq!(spec.uri, "src/a,one.cem");
        assert_eq!(
            spec.root_scope.default_content_type.as_deref(),
            Some("text/custom-element-xslt")
        );
    }

    #[test]
    fn spec_record_rejects_empty_scope_map_entries() {
        let error = parse_input_spec_record("uri=src/a.cem,namespaces=:urn:widgets").unwrap_err();

        assert!(error
            .message
            .contains("namespaces map entries require a non-empty key"));
    }

    #[test]
    fn output_spec_record_maps_target_scope() {
        let spec = parse_output_spec_record(
            "input=src/a.cem,dest=dist/a.cem,contentType=application/cem+xml,schema=core,cemtFormatter=acme.showcase.format-tree,cemtFormatterProfile=acme.showcase.format-tree,cemtColorizer=acme.showcase.color-tree,cemtColorProfile=classes,defaultNs=https://cem.dev/ns/core/1,namespaces=html:http://www.w3.org/1999/xhtml",
        )
        .unwrap();

        assert_eq!(spec.input_ref.as_deref(), Some("src/a.cem"));
        assert_eq!(spec.destination.as_deref(), Some("dist/a.cem"));
        assert_eq!(
            spec.root_scope.default_content_type.as_deref(),
            Some("application/cem+xml")
        );
        assert_eq!(spec.root_scope.schema.as_deref(), Some("core"));
        assert_eq!(
            spec.root_scope.default_namespace.as_deref(),
            Some("https://cem.dev/ns/core/1")
        );
        assert_eq!(
            spec.root_scope.cemt_formatter.as_deref(),
            Some("acme.showcase.format-tree")
        );
        assert_eq!(
            spec.root_scope.cemt_formatter_profile.as_deref(),
            Some("acme.showcase.format-tree")
        );
        assert_eq!(
            spec.root_scope.cemt_colorizer.as_deref(),
            Some("acme.showcase.color-tree")
        );
        assert_eq!(
            spec.root_scope.cemt_color_profile.as_deref(),
            Some("classes")
        );
        assert_eq!(
            spec.root_scope.namespaces.get("html").map(String::as_str),
            Some("http://www.w3.org/1999/xhtml")
        );
    }

    #[test]
    fn cemt_extension_infers_transform_content_type() {
        assert_eq!(
            infer_content_type_from_path("templates/page.cemt").as_deref(),
            Some(crate::schema::registry::CEM_TRANSFORM_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("templates/PAGE.CEMT").as_deref(),
            Some(crate::schema::registry::CEM_TRANSFORM_CONTENT_TYPE)
        );
    }

    #[test]
    fn cemql_extension_infers_query_content_type() {
        assert_eq!(
            infer_content_type_from_path("queries/module.cemql").as_deref(),
            Some(crate::schema::registry::CEM_QL_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("queries/MODULE.CEMQL").as_deref(),
            Some(crate::schema::registry::CEM_QL_CONTENT_TYPE)
        );
    }

    #[test]
    fn json_extension_infers_json_content_type() {
        assert_eq!(
            infer_content_type_from_path("data/item.json").as_deref(),
            Some(crate::schema::registry::JSON_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("data/ITEM.JSON").as_deref(),
            Some(crate::schema::registry::JSON_CONTENT_TYPE)
        );
    }

    #[test]
    fn yaml_extensions_infer_yaml_content_type() {
        assert_eq!(
            infer_content_type_from_path("data/item.yaml").as_deref(),
            Some(crate::schema::registry::YAML_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("data/ITEM.YML").as_deref(),
            Some(crate::schema::registry::YAML_CONTENT_TYPE)
        );
    }

    #[test]
    fn csv_extension_infers_csv_content_type() {
        assert_eq!(
            infer_content_type_from_path("data/table.csv").as_deref(),
            Some(crate::schema::registry::CSV_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("data/TABLE.CSV").as_deref(),
            Some(crate::schema::registry::CSV_CONTENT_TYPE)
        );
    }

    #[test]
    fn markdown_extensions_infer_markdown_content_type() {
        assert_eq!(
            infer_content_type_from_path("docs/readme.md").as_deref(),
            Some(crate::schema::registry::MARKDOWN_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("docs/README.MARKDOWN").as_deref(),
            Some(crate::schema::registry::MARKDOWN_CONTENT_TYPE)
        );
    }

    #[test]
    fn xml_extension_infers_xml_content_type() {
        assert_eq!(
            infer_content_type_from_path("data/document.xml").as_deref(),
            Some(crate::schema::registry::XML_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("data/DOCUMENT.XML").as_deref(),
            Some(crate::schema::registry::XML_CONTENT_TYPE)
        );
    }

    #[test]
    fn relax_ng_extensions_infer_relax_ng_content_types() {
        assert_eq!(
            infer_content_type_from_path("schema/document.rng").as_deref(),
            Some(crate::schema::registry::RELAX_NG_XML_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("schema/DOCUMENT.RNC").as_deref(),
            Some(crate::schema::registry::RELAX_NG_COMPACT_CONTENT_TYPE)
        );
    }

    #[test]
    fn xhtml_extensions_infer_xhtml_content_type() {
        assert_eq!(
            infer_content_type_from_path("dist/page.xhtml").as_deref(),
            Some(crate::schema::registry::XHTML_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("dist/PAGE.XHT").as_deref(),
            Some(crate::schema::registry::XHTML_CONTENT_TYPE)
        );
    }

    #[test]
    fn html_extensions_infer_html_content_type() {
        assert_eq!(
            infer_content_type_from_path("dist/page.html").as_deref(),
            Some(crate::schema::registry::HTML_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("dist/PAGE.HTM").as_deref(),
            Some(crate::schema::registry::HTML_CONTENT_TYPE)
        );
    }

    #[test]
    fn svg_extensions_infer_svg_content_type() {
        assert_eq!(
            infer_content_type_from_path("assets/icon.svg").as_deref(),
            Some(crate::schema::registry::SVG_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("assets/ICON.SVGZ").as_deref(),
            Some(crate::schema::registry::SVG_CONTENT_TYPE)
        );
    }

    #[test]
    fn mathml_extensions_infer_mathml_content_type() {
        assert_eq!(
            infer_content_type_from_path("math/formula.mml").as_deref(),
            Some(crate::schema::registry::MATHML_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("math/FORMULA.MATHML").as_deref(),
            Some(crate::schema::registry::MATHML_CONTENT_TYPE)
        );
    }

    #[test]
    fn xslt_extensions_infer_xslt_content_type() {
        assert_eq!(
            infer_content_type_from_path("templates/view.xsl").as_deref(),
            Some(crate::schema::registry::XSLT_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("templates/VIEW.XSLT").as_deref(),
            Some(crate::schema::registry::XSLT_CONTENT_TYPE)
        );
    }

    #[test]
    fn css_extension_infers_css_content_type() {
        assert_eq!(
            infer_content_type_from_path("styles/theme.css").as_deref(),
            Some(crate::schema::registry::CSS_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("styles/THEME.CSS").as_deref(),
            Some(crate::schema::registry::CSS_CONTENT_TYPE)
        );
    }

    #[test]
    fn json_schema_paths_infer_json_schema_content_type() {
        assert_eq!(
            infer_content_type_from_path("schema/run-config.schema.json").as_deref(),
            Some(crate::schema::registry::JSON_SCHEMA_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("schema/REPORT.SCHEMA.JSON").as_deref(),
            Some(crate::schema::registry::JSON_SCHEMA_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("schema/root.jsonschema").as_deref(),
            Some(crate::schema::registry::JSON_SCHEMA_CONTENT_TYPE)
        );
    }

    #[test]
    fn run_config_schema_identity_constants_are_stable() {
        assert_eq!(RUN_CONFIG_SCHEMA_URI, "https://cem.dev/ns/cli/run-config/1");
        assert_eq!(RUN_CONFIG_NAMESPACE_URI, RUN_CONFIG_SCHEMA_URI);
        assert_eq!(
            RUN_CONFIG_JSON_SCHEMA_URI,
            "https://cem.dev/schema/cli/run-config.schema.json"
        );
    }

    #[test]
    fn run_config_json_schema_artifact_matches_constants() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../schema/cli/run-config.schema.json"))
                .expect("run config JSON Schema parses");

        assert_eq!(
            schema.get("$id").and_then(serde_json::Value::as_str),
            Some(RUN_CONFIG_JSON_SCHEMA_URI)
        );
        assert_eq!(
            schema
                .pointer("/properties/inputs/items/$ref")
                .and_then(serde_json::Value::as_str),
            Some("#/$defs/inputSpec")
        );
        assert_eq!(
            schema
                .pointer("/properties/outputs/items/$ref")
                .and_then(serde_json::Value::as_str),
            Some("#/$defs/outputSpec")
        );
        assert!(schema
            .pointer("/properties/outputs/description")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|description| description.contains("target-native bytes")));
        assert!(schema
            .pointer("/$defs/outputSpec/properties/destination/description")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|description| description.contains("debug JSON artifacts")));
        assert_eq!(
            schema
                .pointer("/properties/schemaPackages/items/$ref")
                .and_then(serde_json::Value::as_str),
            Some("#/$defs/inputSpec")
        );
        assert_eq!(
            schema
                .pointer("/$defs/scopeConfig/properties/defaultContentType/type")
                .and_then(serde_json::Value::as_str),
            Some("string")
        );
        assert_eq!(
            schema
                .pointer("/$defs/resolverSpec/required/0")
                .and_then(serde_json::Value::as_str),
            Some("uriPrefix")
        );
    }

    #[test]
    fn json_run_config_parses_by_content_type() {
        let response = parse_run_config(RunConfigParseRequest {
            bytes: br#"{"resolvers":[{"uriPrefix":"cem+vfs://workspace","localRoot":"/tmp/cem-vfs","read":true}],"inputs":[{"uri":"src/a.cem","rootScope":{"defaultContentType":"application/cem+xml"}}]}"#.to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json; charset=utf-8".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .unwrap();

        assert_eq!(response.config.inputs.len(), 1);
        assert_eq!(response.config.inputs[0].uri, "src/a.cem");
        assert_eq!(
            response.config.inputs[0]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("application/cem+xml")
        );
        assert_eq!(response.config.resolvers.len(), 1);
        assert_eq!(
            response.config.resolvers[0].uri_prefix,
            "cem+vfs://workspace"
        );
        assert!(response.config.resolvers[0].read);
        assert!(!response.config.resolvers[0].write);
        assert!(response.diagnostics.is_empty());
    }

    #[test]
    fn json_run_config_parses_and_normalizes_schema_packages() {
        let parsed = parse_run_config(RunConfigParseRequest {
            bytes: br#"{"schemaPackages":[{"uri":"packages/cem_ml/schema-packages/html/v1/package.cem"}]}"#.to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .unwrap();
        let response = normalize_run_config(parsed.config, RunConfigDefaults::default(), None);

        assert!(response.diagnostics.is_empty());
        assert_eq!(response.config.schema_packages.len(), 1);
        let package = &response.config.schema_packages[0];
        assert_eq!(
            package.uri,
            "packages/cem_ml/schema-packages/html/v1/package.cem"
        );
        assert_eq!(
            package.root_scope.default_content_type.as_deref(),
            Some(crate::schema::registry::CEM_SCHEMA_PACKAGE_CONTENT_TYPE)
        );
        assert_eq!(
            package.root_scope.schema.as_deref(),
            Some(crate::schema::registry::CEM_SCHEMA_PACKAGE_URI)
        );
    }

    #[test]
    fn schema_package_manifest_input_sets_manifest_identity_defaults() {
        let input = schema_package_manifest_input(
            "packages/cem_ml/schema-packages/html/v1/package.cem",
            ScopeConfig {
                default_namespace: Some("urn:package".to_owned()),
                ..ScopeConfig::default()
            },
        );

        assert_eq!(
            input.uri,
            "packages/cem_ml/schema-packages/html/v1/package.cem"
        );
        assert!(input.bytes.is_empty());
        assert_eq!(input.from_format, Some(InputFormat::Cem));
        assert_eq!(
            input.root_scope.default_content_type.as_deref(),
            Some(crate::schema::registry::CEM_SCHEMA_PACKAGE_CONTENT_TYPE)
        );
        assert_eq!(
            input.root_scope.schema.as_deref(),
            Some(crate::schema::registry::CEM_SCHEMA_PACKAGE_URI)
        );
        assert_eq!(
            input.root_scope.default_namespace.as_deref(),
            Some("urn:package")
        );
        let identity = input.identity.expect("schema package identity");
        assert_eq!(
            identity.content_type.as_deref(),
            Some(crate::schema::registry::CEM_SCHEMA_PACKAGE_CONTENT_TYPE)
        );
        assert_eq!(
            identity.schema.as_deref(),
            Some(crate::schema::registry::CEM_SCHEMA_PACKAGE_URI)
        );
    }

    #[test]
    fn json_run_config_accepts_run_config_schema_identity() {
        let response = parse_run_config(RunConfigParseRequest {
            bytes: b"{}".to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                schema: Some(RUN_CONFIG_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .expect("run config schema identity accepted");

        assert!(response.config.inputs.is_empty());
        assert!(response.config.outputs.is_empty());
    }

    #[test]
    fn json_run_config_accepts_run_config_namespace_identity() {
        let response = parse_run_config(RunConfigParseRequest {
            bytes: b"{}".to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                default_namespace: Some(RUN_CONFIG_NAMESPACE_URI.to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .expect("run config namespace identity accepted");

        assert!(response.config.inputs.is_empty());
        assert!(response.config.outputs.is_empty());
    }

    #[test]
    fn unsupported_run_config_schema_identity_is_rejected() {
        let error = parse_run_config(RunConfigParseRequest {
            bytes: b"{}".to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                schema: Some("https://cem.dev/ns/core/1".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .expect_err("CEM core schema is not run config schema");

        assert_eq!(error.code, "cem.run_config.unsupported_schema_identity");
        assert!(error.message.contains(RUN_CONFIG_SCHEMA_URI));
    }

    #[test]
    fn unsupported_run_config_namespace_identity_is_rejected() {
        let error = parse_run_config(RunConfigParseRequest {
            bytes: b"{}".to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                default_namespace: Some("https://cem.dev/ns/core/1".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .expect_err("CEM core namespace is not run config namespace");

        assert_eq!(error.code, "cem.run_config.unsupported_schema_identity");
        assert!(error.message.contains(RUN_CONFIG_NAMESPACE_URI));
    }

    #[test]
    fn normalize_run_config_applies_defaults_and_infers_content_type() {
        let response = normalize_run_config(
            RunConfig {
                inputs: vec![
                    InputSpec {
                        uri: "src/a.cem".to_owned(),
                        ..InputSpec::default()
                    },
                    InputSpec {
                        uri: "src/b.html".to_owned(),
                        root_scope: ScopeConfig {
                            schema: Some("explicit-schema".to_owned()),
                            ..ScopeConfig::default()
                        },
                    },
                    InputSpec {
                        uri: "src/icon.svg".to_owned(),
                        ..InputSpec::default()
                    },
                    InputSpec {
                        uri: "src/page.xhtml".to_owned(),
                        ..InputSpec::default()
                    },
                ],
                outputs: vec![
                    OutputSpec {
                        destination: Some("dist/a.cem".to_owned()),
                        ..OutputSpec::default()
                    },
                    OutputSpec {
                        destination: Some("dist/icon.svg".to_owned()),
                        ..OutputSpec::default()
                    },
                    OutputSpec {
                        destination: Some("dist/page.xhtml".to_owned()),
                        ..OutputSpec::default()
                    },
                ],
                schema_packages: Vec::new(),
                resolvers: Vec::new(),
                scheduler: SchedulerConfig::default(),
            },
            RunConfigDefaults {
                input_scope: ScopeConfig {
                    schema: Some("default-schema".to_owned()),
                    ..ScopeConfig::default()
                },
                output_scope: ScopeConfig {
                    schema: Some("target-schema".to_owned()),
                    ..ScopeConfig::default()
                },
                ..RunConfigDefaults::default()
            },
            None,
        );

        assert!(response.diagnostics.is_empty());
        assert_eq!(
            response.config.inputs[0]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("application/cem+xml")
        );
        assert_eq!(
            response.config.inputs[0].root_scope.schema.as_deref(),
            Some("default-schema")
        );
        assert_eq!(
            response.config.inputs[1]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("text/html")
        );
        assert_eq!(
            response.config.inputs[1].root_scope.schema.as_deref(),
            Some("explicit-schema")
        );
        assert_eq!(
            response.config.inputs[2]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("image/svg+xml")
        );
        assert_eq!(
            response.config.inputs[2].root_scope.schema.as_deref(),
            Some("default-schema")
        );
        assert_eq!(
            response.config.inputs[3]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("application/xhtml+xml")
        );
        assert_eq!(
            response.config.inputs[3].root_scope.schema.as_deref(),
            Some("default-schema")
        );
        assert_eq!(
            response.config.outputs[0]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("application/cem+xml")
        );
        assert_eq!(
            response.config.outputs[0].root_scope.schema.as_deref(),
            Some("target-schema")
        );
        assert_eq!(
            response.config.outputs[1]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("image/svg+xml")
        );
        assert_eq!(
            response.config.outputs[1].root_scope.schema.as_deref(),
            Some("target-schema")
        );
        assert_eq!(
            response.config.outputs[2]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("application/xhtml+xml")
        );
        assert_eq!(
            response.config.outputs[2].root_scope.schema.as_deref(),
            Some("target-schema")
        );
    }

    #[test]
    fn normalize_run_config_resolves_relative_module_map_against_config_path() {
        let response = normalize_run_config(
            RunConfig {
                inputs: vec![InputSpec {
                    uri: "src/a.cem".to_owned(),
                    root_scope: ScopeConfig {
                        module_map: Some("cem.modules.json".to_owned()),
                        ..ScopeConfig::default()
                    },
                }],
                outputs: vec![OutputSpec {
                    destination: Some("dist/a.cem".to_owned()),
                    root_scope: ScopeConfig {
                        module_map: Some("out.modules.json".to_owned()),
                        ..ScopeConfig::default()
                    },
                    ..OutputSpec::default()
                }],
                schema_packages: Vec::new(),
                resolvers: Vec::new(),
                scheduler: SchedulerConfig::default(),
            },
            RunConfigDefaults::default(),
            Some("/workspace/configs/run.json"),
        );

        assert!(response.diagnostics.is_empty());
        assert_eq!(
            response.config.inputs[0].root_scope.module_map.as_deref(),
            Some("/workspace/configs/cem.modules.json")
        );
        assert_eq!(
            response.config.outputs[0].root_scope.module_map.as_deref(),
            Some("/workspace/configs/out.modules.json")
        );
    }

    #[test]
    fn normalize_run_config_leaves_absolute_and_uri_module_maps_unchanged() {
        let response = normalize_run_config(
            RunConfig {
                inputs: vec![
                    InputSpec {
                        uri: "src/a.cem".to_owned(),
                        root_scope: ScopeConfig {
                            module_map: Some("/workspace/cem.modules.json".to_owned()),
                            ..ScopeConfig::default()
                        },
                    },
                    InputSpec {
                        uri: "src/b.cem".to_owned(),
                        root_scope: ScopeConfig {
                            module_map: Some("https://example.test/cem.modules.json".to_owned()),
                            ..ScopeConfig::default()
                        },
                    },
                ],
                ..RunConfig::default()
            },
            RunConfigDefaults::default(),
            Some("/workspace/configs/run.json"),
        );

        assert!(response.diagnostics.is_empty());
        assert_eq!(
            response.config.inputs[0].root_scope.module_map.as_deref(),
            Some("/workspace/cem.modules.json")
        );
        assert_eq!(
            response.config.inputs[1].root_scope.module_map.as_deref(),
            Some("https://example.test/cem.modules.json")
        );
    }

    #[test]
    fn normalize_run_config_preserves_uri_shaped_inputs_outputs_and_module_maps() {
        let response = normalize_run_config(
            RunConfig {
                inputs: vec![InputSpec {
                    uri: "https://example.test/src/a.cem".to_owned(),
                    root_scope: ScopeConfig {
                        module_map: Some("cem+vfs://workspace/cem.modules.json".to_owned()),
                        ..ScopeConfig::default()
                    },
                }],
                outputs: vec![OutputSpec {
                    input_ref: Some("https://example.test/src/a.cem".to_owned()),
                    destination: Some("cem+vfs://workspace/dist/a.json".to_owned()),
                    root_scope: ScopeConfig {
                        module_map: Some("file://example.test/out.modules.json".to_owned()),
                        ..ScopeConfig::default()
                    },
                }],
                ..RunConfig::default()
            },
            RunConfigDefaults::default(),
            Some("/workspace/configs/run.json"),
        );

        assert!(response.diagnostics.is_empty());
        assert_eq!(
            response.config.inputs[0].uri,
            "https://example.test/src/a.cem"
        );
        assert_eq!(
            response.config.inputs[0].root_scope.module_map.as_deref(),
            Some("cem+vfs://workspace/cem.modules.json")
        );
        assert_eq!(
            response.config.outputs[0].destination.as_deref(),
            Some("cem+vfs://workspace/dist/a.json")
        );
        assert_eq!(
            response.config.outputs[0].root_scope.module_map.as_deref(),
            Some("file://example.test/out.modules.json")
        );
    }

    #[test]
    fn normalize_run_config_resolves_relative_output_destination_against_config_path() {
        let response = normalize_run_config(
            RunConfig {
                inputs: vec![InputSpec {
                    uri: "src/a.cem".to_owned(),
                    ..InputSpec::default()
                }],
                outputs: vec![OutputSpec {
                    destination: Some("dist/a.cem".to_owned()),
                    ..OutputSpec::default()
                }],
                ..RunConfig::default()
            },
            RunConfigDefaults::default(),
            Some("/workspace/configs/run.json"),
        );

        assert!(response.diagnostics.is_empty());
        assert_eq!(
            response.config.outputs[0].destination.as_deref(),
            Some("/workspace/configs/dist/a.cem")
        );
        assert_eq!(
            response.config.outputs[0]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("application/cem+xml")
        );
    }

    #[test]
    fn normalize_run_config_resolves_relative_paths_against_local_file_uri_config_path() {
        let response = normalize_run_config(
            RunConfig {
                inputs: vec![InputSpec {
                    uri: "src/a.cem".to_owned(),
                    root_scope: ScopeConfig {
                        module_map: Some("cem.modules.json".to_owned()),
                        ..ScopeConfig::default()
                    },
                }],
                outputs: vec![OutputSpec {
                    destination: Some("dist/a.cem".to_owned()),
                    root_scope: ScopeConfig {
                        module_map: Some("out.modules.json".to_owned()),
                        ..ScopeConfig::default()
                    },
                    ..OutputSpec::default()
                }],
                ..RunConfig::default()
            },
            RunConfigDefaults::default(),
            Some("file:///workspace/configs/with%20space/run.json"),
        );

        assert!(response.diagnostics.is_empty());
        assert_eq!(
            response.config.inputs[0].root_scope.module_map.as_deref(),
            Some("/workspace/configs/with space/cem.modules.json")
        );
        assert_eq!(
            response.config.outputs[0].root_scope.module_map.as_deref(),
            Some("/workspace/configs/with space/out.modules.json")
        );
        assert_eq!(
            response.config.outputs[0].destination.as_deref(),
            Some("/workspace/configs/with space/dist/a.cem")
        );
    }

    #[test]
    fn run_config_validation_reports_unknown_output_input_ref() {
        let parsed = parse_run_config(RunConfigParseRequest {
            bytes: br#"{"inputs":[{"uri":"src/a.cem"}],"outputs":[{"inputRef":"missing.cem"}]}"#
                .to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: Some("file:///run-config.json".to_owned()),
        })
        .unwrap();
        let response = normalize_run_config(
            parsed.config,
            RunConfigDefaults::default(),
            Some("file:///run-config.json"),
        );

        assert_eq!(response.diagnostics.len(), 1);
        assert_eq!(
            response.diagnostics[0].code,
            "cem.run_config.output_input_ref_unknown"
        );
        assert_eq!(
            response.diagnostics[0].uri.as_deref(),
            Some("file:///run-config.json")
        );
    }

    #[test]
    fn run_config_validation_reports_duplicate_inputs() {
        let parsed = parse_run_config(RunConfigParseRequest {
            bytes: br#"{"inputs":[{"uri":"src/a.cem"},{"uri":"src/a.cem"}]}"#.to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .unwrap();
        let response = normalize_run_config(parsed.config, RunConfigDefaults::default(), None);

        assert!(response
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.run_config.input_uri_duplicate"));
    }

    #[test]
    fn run_config_validation_reports_duplicate_schema_packages() {
        let parsed = parse_run_config(RunConfigParseRequest {
            bytes: br#"{"schemaPackages":[{"uri":"pkg.cem"},{"uri":"pkg.cem"}]}"#.to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .unwrap();
        let response = normalize_run_config(parsed.config, RunConfigDefaults::default(), None);

        assert!(response
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.run_config.schema_package_uri_duplicate"));
    }

    #[test]
    fn run_config_validation_reports_invalid_scope_fields() {
        let parsed = parse_run_config(RunConfigParseRequest {
            bytes: br#"{
                "inputs": [{
                    "uri": "src/a.cem",
                    "rootScope": {
                        "defaultNamespace": "",
                        "namespaces": {
                            "1bad": "urn:widgets",
                            "xml": "urn:not-xml"
                        },
                        "versionPins": {
                            "core": ""
                        },
                        "moduleMap": ""
                    }
                }]
            }"#
            .to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: Some("file:///run-config.json".to_owned()),
        })
        .unwrap();
        let response = normalize_run_config(
            parsed.config,
            RunConfigDefaults::default(),
            Some("file:///run-config.json"),
        );

        let codes: Vec<_> = response
            .diagnostics
            .iter()
            .map(|diag| diag.code.as_str())
            .collect();
        assert!(codes.contains(&"cem.run_config.scope_module_map_invalid"));
        assert!(codes.contains(&"cem.run_config.scope_namespace_invalid"));
        assert!(codes.contains(&"cem.run_config.scope_version_pin_invalid"));
        assert!(response
            .diagnostics
            .iter()
            .all(|diag| diag.uri.as_deref() == Some("file:///run-config.json")));
    }

    #[test]
    fn normalize_run_config_validates_default_scope_fields() {
        let response = normalize_run_config(
            RunConfig::default(),
            RunConfigDefaults {
                input_scope: ScopeConfig {
                    namespaces: BTreeMap::from([("xml".to_owned(), "urn:not-xml".to_owned())]),
                    ..ScopeConfig::default()
                },
                output_scope: ScopeConfig {
                    default_namespace: Some(String::new()),
                    ..ScopeConfig::default()
                },
                ..RunConfigDefaults::default()
            },
            Some("file:///run-config.json"),
        );

        assert_eq!(response.diagnostics.len(), 2);
        assert!(response
            .diagnostics
            .iter()
            .all(|diag| diag.code == "cem.run_config.scope_namespace_invalid"));
        assert!(response
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("input default scope")));
        assert!(response
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("output default scope")));
        assert!(response
            .diagnostics
            .iter()
            .all(|diag| diag.uri.as_deref() == Some("file:///run-config.json")));
    }

    #[test]
    fn normalize_run_config_merges_output_pipeline_defaults() {
        let response = normalize_run_config(
            RunConfig {
                outputs: vec![OutputSpec::default()],
                ..RunConfig::default()
            },
            RunConfigDefaults {
                output_scope: ScopeConfig {
                    output_color_type: Some("html".to_owned()),
                    cemt_formatter: Some("acme.format-tree".to_owned()),
                    cemt_formatter_profile: Some("acme.format-tree".to_owned()),
                    cemt_colorizer: Some("acme.color-tree".to_owned()),
                    cemt_color_profile: Some("classes".to_owned()),
                    ..ScopeConfig::default()
                },
                ..RunConfigDefaults::default()
            },
            None,
        );

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        let scope = &response.config.outputs[0].root_scope;
        assert_eq!(scope.output_color_type.as_deref(), Some("html"));
        assert_eq!(scope.cemt_formatter.as_deref(), Some("acme.format-tree"));
        assert_eq!(
            scope.cemt_formatter_profile.as_deref(),
            Some("acme.format-tree")
        );
        assert_eq!(scope.cemt_colorizer.as_deref(), Some("acme.color-tree"));
        assert_eq!(scope.cemt_color_profile.as_deref(), Some("classes"));
    }

    #[test]
    fn normalized_run_plan_builds_effective_scopes_from_config_bytes() {
        let plan = parse_normalized_run_plan(NormalizedRunPlanRequest {
            config_bytes: Some(
                br#"{
                    "inputs": [{
                        "uri": "src/a.cem",
                        "rootScope": {
                            "defaultContentType": "text/html",
                            "moduleMap": "input.modules.json"
                        }
                    }],
                    "outputs": [{
                        "inputRef": "src/a.cem",
                        "destination": "dist/a.html",
                        "rootScope": {
                            "schema": "https://example.test/schema/html"
                        }
                    }]
                }"#
                .to_vec(),
            ),
            config_base_uri: Some("/workspace/configs/run.json".to_owned()),
            defaults: RunConfigDefaults {
                input_scope: ScopeConfig {
                    default_content_type: Some("application/cem+xml".to_owned()),
                    schema: Some("https://example.test/schema/default-input".to_owned()),
                    ..ScopeConfig::default()
                },
                output_scope: ScopeConfig {
                    default_content_type: Some("text/html".to_owned()),
                    ..ScopeConfig::default()
                },
                ..RunConfigDefaults::default()
            },
            ..NormalizedRunPlanRequest::default()
        })
        .unwrap();

        assert!(plan.run_id.starts_with("run:"));
        assert!(plan.diagnostics.is_empty());
        assert_eq!(
            plan.config_identity.schema_identity.as_deref(),
            Some(RUN_CONFIG_SCHEMA_URI)
        );
        assert_eq!(plan.inputs[0].input_id, "input:0");
        assert_eq!(plan.inputs[0].declared_uri, "src/a.cem");
        assert_eq!(
            plan.inputs[0].identity.content_type.as_deref(),
            Some("text/html")
        );
        assert_eq!(
            plan.inputs[0].identity.schema.as_deref(),
            Some("https://example.test/schema/default-input")
        );
        let module_map = plan.inputs[0].root_scope.module_map.as_ref().unwrap();
        assert_eq!(module_map.declared_uri, "input.modules.json");
        assert_eq!(
            module_map.resolved_uri.as_deref(),
            Some("/workspace/configs/input.modules.json")
        );
        assert_eq!(plan.outputs[0].input_id.as_deref(), Some("input:0"));
        assert_eq!(
            plan.outputs[0].declared_destination.as_deref(),
            Some("dist/a.html")
        );
        assert_eq!(
            plan.outputs[0].resolved_destination.as_deref(),
            Some("/workspace/configs/dist/a.html")
        );
        assert_eq!(
            plan.outputs[0].identity.content_type.as_deref(),
            Some("text/html")
        );
        assert_eq!(
            plan.outputs[0].identity.schema.as_deref(),
            Some("https://example.test/schema/html")
        );
        assert_eq!(
            plan.effective_run_config().outputs[0]
                .destination
                .as_deref(),
            Some("/workspace/configs/dist/a.html")
        );
    }

    #[test]
    fn normalized_run_plan_lowers_cli_records_and_resolves_paths() {
        let plan = parse_normalized_run_plan(NormalizedRunPlanRequest {
            config_bytes: None,
            config_base_uri: Some("/workspace/configs/run.json".to_owned()),
            input_records: vec![
                "uri=src/b.cem,moduleMap=mods.json,budgets=parseMs:12|cpuWorkers:2".to_owned(),
            ],
            output_records: vec![
                "input=src/b.cem,dest=dist/b.json,contentType=application/json".to_owned(),
            ],
            ..NormalizedRunPlanRequest::default()
        })
        .unwrap();

        assert!(plan.diagnostics.is_empty());
        assert_eq!(
            plan.config_identity.source_kind,
            NormalizedConfigSourceKind::CliRecords
        );
        assert_eq!(plan.inputs[0].input_id, "input:0");
        assert_eq!(
            plan.inputs[0]
                .root_scope
                .module_map
                .as_ref()
                .and_then(|module_map| module_map.resolved_uri.as_deref()),
            Some("/workspace/configs/mods.json")
        );
        assert_eq!(plan.inputs[0].root_scope.budgets.parse_ms, Some(12));
        assert_eq!(plan.inputs[0].root_scope.policy.cpu_workers, 2);
        assert_eq!(
            plan.outputs[0].resolved_destination.as_deref(),
            Some("/workspace/configs/dist/b.json")
        );
    }

    #[test]
    fn normalized_run_plan_projects_format_alias_defaults_as_hints() {
        let plan = parse_normalized_run_plan(NormalizedRunPlanRequest {
            input_records: vec![
                "uri=src/page.html,contentType=text/html,schema=http://www.w3.org/1999/xhtml"
                    .to_owned(),
            ],
            output_records: vec![
                "input=src/page.html,dest=dist/page.cem,contentType=application/cem+xml,schema=https://cem.dev/ns/core/1"
                    .to_owned(),
            ],
            defaults: RunConfigDefaults {
                from_format_hint: Some(InputFormat::Xml),
                to_format_fallback: Some("dom-json".to_owned()),
                ..RunConfigDefaults::default()
            },
            ..NormalizedRunPlanRequest::default()
        })
        .unwrap();

        assert!(plan.diagnostics.is_empty());
        assert_eq!(plan.inputs[0].from_format_hint, Some(InputFormat::Xml));
        assert_eq!(
            plan.inputs[0].identity.content_type.as_deref(),
            Some("text/html")
        );
        assert_eq!(
            plan.inputs[0].identity.schema.as_deref(),
            Some("http://www.w3.org/1999/xhtml")
        );
        assert_eq!(
            plan.inputs[0].provenance[0].field_path,
            "defaults.fromFormatHint"
        );
        assert_eq!(
            plan.inputs[0].provenance[0].declared_value.as_deref(),
            Some("xml")
        );

        assert_eq!(
            plan.outputs[0].to_format_fallback.as_deref(),
            Some("dom-json")
        );
        assert_eq!(
            plan.outputs[0].identity.content_type.as_deref(),
            Some("application/cem+xml")
        );
        assert_eq!(
            plan.outputs[0].identity.schema.as_deref(),
            Some("https://cem.dev/ns/core/1")
        );
        assert_eq!(
            plan.outputs[0].provenance[0].field_path,
            "defaults.toFormatFallback"
        );
        assert_eq!(
            plan.outputs[0].provenance[0].declared_value.as_deref(),
            Some("dom-json")
        );
    }

    #[test]
    fn normalized_run_plan_exposes_typed_policy_and_budget_aliases() {
        let plan = parse_normalized_run_plan(NormalizedRunPlanRequest {
            config_bytes: Some(
                br#"{
                    "inputs": [{
                        "uri": "src/b.cem",
                        "rootScope": {
                            "policy": "deterministic",
                            "budgets": {
                                "cpuWorkers": "0",
                                "queueSize": "16",
                                "ioStreams": "4",
                                "memoryBytes": "1024",
                                "pluginMs": "20",
                                "overflow": "spill-to-parent",
                                "parseMs": "5",
                                "validateTimeBudgetMs": "7",
                                "futureBudget": "kept"
                            }
                        }
                    }]
                }"#
                .to_vec(),
            ),
            config_base_uri: Some("file:///workspace/run.json".to_owned()),
            ..NormalizedRunPlanRequest::default()
        })
        .unwrap();

        let scope = &plan.inputs[0].root_scope;
        assert_eq!(scope.policy.cpu_workers, 1);
        assert_eq!(scope.policy.queue_size, 16);
        assert_eq!(scope.policy.io_streams, 4);
        assert_eq!(scope.policy.memory_bytes, 1024);
        assert_eq!(scope.policy.plugin_time_budget_ms, Some(20));
        assert_eq!(scope.policy.overflow, OverflowPolicy::SpillToParent);
        assert_eq!(scope.budgets.parse_ms, Some(5));
        assert_eq!(scope.budgets.validate_ms, Some(7));
        assert_eq!(scope.budgets.memory_bytes, Some(1024));
        assert_eq!(scope.budgets.plugin_ms, Some(20));
        assert_eq!(scope.budgets.unknown.len(), 1);
        assert!(has_field_path(
            &plan.diagnostics,
            "cem.run_config.scope_budget_unknown",
            "inputs[0].rootScope.budgets.futureBudget"
        ));
    }

    #[test]
    fn normalized_run_plan_reports_field_path_diagnostics() {
        let plan = parse_normalized_run_plan(NormalizedRunPlanRequest {
            config_bytes: Some(
                br#"{
                    "inputs": [{
                        "uri": "src/a.cem",
                        "rootScope": {
                            "defaultNamespace": "",
                            "namespaces": {
                                "1bad": "urn:widgets",
                                "xml": "urn:not-xml"
                            },
                            "versionPins": {
                                "core": ""
                            },
                            "moduleMap": "",
                            "budgets": {
                                "parseMs": "not-a-number",
                                "overflow": "explode"
                            }
                        }
                    }],
                    "resolvers": [{
                        "uriPrefix": "",
                        "localRoot": "",
                        "read": false,
                        "write": false
                    }]
                }"#
                .to_vec(),
            ),
            config_base_uri: Some("file:///run-config.json".to_owned()),
            ..NormalizedRunPlanRequest::default()
        })
        .unwrap();

        let paths = diagnostic_field_paths(&plan.diagnostics);
        assert!(paths.contains(&"inputs[0].rootScope.moduleMap".to_owned()));
        assert!(paths.contains(&"inputs[0].rootScope.defaultNamespace".to_owned()));
        assert!(paths.contains(&"inputs[0].rootScope.namespaces.1bad".to_owned()));
        assert!(paths.contains(&"inputs[0].rootScope.versionPins.core".to_owned()));
        assert!(paths.contains(&"inputs[0].rootScope.budgets.parseMs".to_owned()));
        assert!(paths.contains(&"inputs[0].rootScope.budgets.overflow".to_owned()));
        assert!(paths.contains(&"resolvers[0].uriPrefix".to_owned()));
        assert!(paths.contains(&"resolvers[0].localRoot".to_owned()));
        assert!(paths.contains(&"resolvers[0]".to_owned()));
    }

    #[test]
    fn normalized_run_plan_reports_invalid_cli_record_with_field_path() {
        let plan = parse_normalized_run_plan(NormalizedRunPlanRequest {
            config_bytes: None,
            input_records: vec!["uri".to_owned()],
            ..NormalizedRunPlanRequest::default()
        })
        .unwrap();

        assert!(has_field_path(
            &plan.diagnostics,
            "cem.run_config.input_spec_invalid",
            "inputSpecRecords[0]"
        ));
    }

    #[test]
    fn unsupported_run_config_content_type_is_rejected_before_document_work() {
        let error = parse_run_config(RunConfigParseRequest {
            bytes: b"{}".to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/cem+xml".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .unwrap_err();

        assert_eq!(error.code, "cem.run_config.unsupported_content_type");
    }
}
