use crate::conversion::{ConversionRegistry, DomProjectionParityCemtAdapter};
use crate::diagnostics::{Diagnostic, Severity};
use crate::interpreter::OutputSpan;
use crate::report::{Report, SchedulerTraceReport};
use crate::resolver::ResolverRegistry;
use crate::run_config::{SchedulerConfig, ScopeConfig};
use crate::schema::registry::{
    CSS_CONTENT_TYPE, CSS_SCHEMA_URI, HTML_CONTENT_TYPE, HTML_SCHEMA_URI, XHTML_CONTENT_TYPE,
    XHTML_SCHEMA_URI,
};
use crate::schema::SchemaRegistry;
use crate::source_map::SourceMapStack;
use crate::transform_template::{
    TransformTemplateAdapterRegistry, TransformTemplateAdapterResolution,
    TransformTemplateEncodeImplementationRegistry,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FailLevel {
    Parse,
    Validate,
    Strict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputFormat {
    Cem,
    Html,
    Xml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayerFormat {
    Cem,
    Html,
    Xml,
    DomJson,
    Ast,
    Events,
    DomBin,
    AstBin,
    EventsBin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParseProjection {
    DomJson,
    Json,
    Ast,
    Events,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidateProjection {
    Json,
    Xml,
    Cem,
    Text,
    Html,
    Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceProjection {
    Json,
    Xml,
    Cem,
    Text,
    Html,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BenchProjection {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InspectView {
    Summary,
    Ast,
    Events,
    Diagnostics,
    SourceOffsets,
    Tree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BenchProfile {
    Cpu,
    Memory,
}

#[derive(Debug, Clone)]
pub struct EngineContext {
    pub schema: Option<String>,
    pub content_type: Option<String>,
    pub base_uri: Option<String>,
    pub scheduler: SchedulerConfig,
    pub schema_registry: SchemaRegistry,
    pub converter_registry: ConversionRegistry,
    pub schema_package_manifests: Vec<EngineInput>,
    pub resolver_registry: ResolverRegistry,
    pub template_adapter_registry: TransformTemplateAdapterRegistry,
    pub transform_template_encode_registry: TransformTemplateEncodeImplementationRegistry,
}

impl Default for EngineContext {
    fn default() -> Self {
        let mut template_adapter_registry =
            TransformTemplateAdapterRegistry::with_builtin_adapters();
        template_adapter_registry.register(DomProjectionParityCemtAdapter);

        Self {
            schema: None,
            content_type: None,
            base_uri: None,
            scheduler: SchedulerConfig::default(),
            schema_registry: SchemaRegistry::with_builtin_schemas(),
            converter_registry: ConversionRegistry::with_builtin_converters(),
            schema_package_manifests: Vec::new(),
            resolver_registry: ResolverRegistry::default(),
            template_adapter_registry,
            transform_template_encode_registry:
                TransformTemplateEncodeImplementationRegistry::with_builtin_encoders(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatIdentity {
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
    pub schema: Option<String>,
    #[serde(rename = "defaultNamespace", default)]
    pub default_namespace: Option<String>,
    #[serde(default)]
    pub namespaces: BTreeMap<String, String>,
    #[serde(rename = "baseUri")]
    pub base_uri: Option<String>,
}

impl From<&EngineContext> for FormatIdentity {
    fn from(context: &EngineContext) -> Self {
        Self {
            content_type: context.content_type.clone(),
            schema: context.schema.clone(),
            default_namespace: None,
            namespaces: BTreeMap::new(),
            base_uri: context.base_uri.clone(),
        }
    }
}

pub const TRANSFORM_TEMPLATE_UNSUPPORTED_CODE: &str = "cem.transform_template.identity_unsupported";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformTemplateKind {
    Xslt,
    CemNative,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformRuntimePhase {
    #[default]
    CemQlFragment,
    CemNativeModules,
    XsltParity,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformCardinalityMode {
    #[default]
    OneToOne,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformDuplicateDestinationPolicy {
    #[default]
    Reject,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformFailurePolicy {
    #[default]
    FailFast,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformOutputPolicy {
    #[default]
    ContentPrimary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformDiagnosticOrigin {
    Config,
    Import,
    TemplateLoad,
    TemplateCompile,
    TemplateExecution,
    Export,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformExecutionPolicy {
    pub runtime_phase: TransformRuntimePhase,
    pub cardinality: TransformCardinalityMode,
    pub duplicate_destination_policy: TransformDuplicateDestinationPolicy,
    pub failure_policy: TransformFailurePolicy,
    pub output_policy: TransformOutputPolicy,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformTemplateEntrypoint {
    #[serde(default)]
    pub name: Option<String>,
}

impl TransformTemplateEntrypoint {
    pub fn implicit() -> Self {
        Self { name: None }
    }

    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
        }
    }

    pub fn is_implicit(&self) -> bool {
        self.name.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformTemplateIdentityError {
    pub code: &'static str,
    pub message: String,
}

impl std::fmt::Display for TransformTemplateIdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for TransformTemplateIdentityError {}

pub fn classify_transform_template_identity(
    identity: &FormatIdentity,
) -> Result<TransformTemplateKind, TransformTemplateIdentityError> {
    classify_transform_template_identity_with_registry(
        identity,
        &TransformTemplateAdapterRegistry::with_builtin_adapters(),
    )
}

pub fn classify_transform_template_identity_with_registry(
    identity: &FormatIdentity,
    registry: &TransformTemplateAdapterRegistry,
) -> Result<TransformTemplateKind, TransformTemplateIdentityError> {
    match registry.select(identity) {
        TransformTemplateAdapterResolution::Matched(selection) => Ok(selection.kind),
        TransformTemplateAdapterResolution::Ambiguous(ids) => {
            Err(transform_template_identity_error(format!(
                "transform template identity matched multiple adapters: {}",
                ids.join(", ")
            )))
        }
        TransformTemplateAdapterResolution::Unsupported => Err(transform_template_identity_error(
            unsupported_transform_template_identity_message(identity),
        )),
    }
}

fn unsupported_transform_template_identity_message(identity: &FormatIdentity) -> String {
    if let Some(content_type) = identity.content_type.as_deref() {
        return format!(
            "no transform template adapter matched content type `{}`",
            content_type_essence(content_type)
        );
    }
    if let Some(schema) = identity.schema.as_deref().map(str::trim) {
        if !schema.is_empty() {
            return format!("no transform template adapter matched schema `{schema}`");
        }
    }
    if let Some(namespace) = identity
        .default_namespace
        .as_deref()
        .or_else(|| identity.namespaces.values().next().map(String::as_str))
    {
        return format!("no transform template adapter matched namespace `{namespace}`");
    }
    "transform template identity requires a supported content type, schema, or namespace".to_owned()
}

pub fn validate_transform_request_runtime_contract(request: &TransformRequest) -> Vec<Diagnostic> {
    let mut diagnostics = validate_transform_execution_policy(&request.execution_policy);
    validate_transform_stage_runtime_contract(
        "transform",
        Some(&request.template.uri),
        request.template_kind,
        &request.template_entrypoint,
        &request.params,
        &request.execution_policy,
        &mut diagnostics,
    );
    diagnostics
}

pub fn validate_transform_graph_runtime_contract(
    request: &TransformGraphRequest,
) -> Vec<Diagnostic> {
    let mut diagnostics = validate_transform_execution_policy(&request.execution_policy);
    let mut ids = BTreeSet::new();
    let mut artifacts = BTreeSet::new();

    for import in &request.imports {
        validate_graph_id("import", &import.id, &mut ids, &mut diagnostics);
        artifacts.insert(import.id.clone());
    }
    for join in &request.joins {
        validate_graph_id("join", &join.id, &mut ids, &mut diagnostics);
        artifacts.insert(join.id.clone());
    }
    for stage in &request.stages {
        validate_graph_id("transform", &stage.id, &mut ids, &mut diagnostics);
        artifacts.insert(stage.id.clone());
        diagnostics.extend(validate_transform_execution_policy(&stage.execution_policy));
        validate_transform_stage_runtime_contract(
            &stage.id,
            Some(&stage.template.uri),
            stage.template_kind,
            &stage.template_entrypoint,
            &stage.params,
            &stage.execution_policy,
            &mut diagnostics,
        );
    }
    for rewrite in &request.importmap_rewrites {
        validate_graph_id("rewrite-importmap", &rewrite.id, &mut ids, &mut diagnostics);
        artifacts.insert(rewrite.id.clone());
        if rewrite.target_imports.is_empty() {
            diagnostics.push(transform_runtime_diagnostic(
                None,
                "cem.transform_runtime.importmap_target_imports_empty",
                format!(
                    "rewrite-importmap node `{}` requires at least one target import",
                    rewrite.id
                ),
            ));
        }
    }
    for export in &request.exports {
        validate_graph_id("export", &export.id, &mut ids, &mut diagnostics);
    }
    for export in &request.exports {
        validate_transform_graph_export_style_policy(export, &request.exports, &mut diagnostics);
    }

    for stage in &request.stages {
        validate_artifact_ref(
            &stage.id,
            "primaryInput",
            &stage.primary_input,
            &artifacts,
            &mut diagnostics,
        );
        for (name, artifact_id) in &stage.secondary_inputs {
            validate_artifact_ref(
                &stage.id,
                &format!("secondaryInputs.{name}"),
                artifact_id,
                &artifacts,
                &mut diagnostics,
            );
        }
    }
    for join in &request.joins {
        for (index, input) in join.inputs.iter().enumerate() {
            validate_artifact_ref(
                &join.id,
                &format!("inputs.{index}"),
                &input.artifact_id,
                &artifacts,
                &mut diagnostics,
            );
        }
    }
    for rewrite in &request.importmap_rewrites {
        validate_artifact_ref(
            &rewrite.id,
            "primaryInput",
            &rewrite.primary_input,
            &artifacts,
            &mut diagnostics,
        );
    }
    for export in &request.exports {
        validate_artifact_ref(
            &export.id,
            "input",
            &export.input,
            &artifacts,
            &mut diagnostics,
        );
    }

    if request.execution_policy.duplicate_destination_policy
        == TransformDuplicateDestinationPolicy::Reject
    {
        let mut destinations = BTreeSet::new();
        for export in &request.exports {
            let Some(destination) = export.destination.as_deref() else {
                continue;
            };
            if !destinations.insert(destination.to_owned()) {
                diagnostics.push(transform_runtime_diagnostic(
                    export.destination.as_deref(),
                    "cem.transform_runtime.duplicate_destination",
                    format!(
                        "transform export destination `{destination}` is declared more than once"
                    ),
                ));
            }
        }
    }

    diagnostics
}

fn content_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

fn validate_transform_graph_export_style_policy(
    export: &TransformGraphExport,
    exports: &[TransformGraphExport],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if export.style_policy != TransformGraphStylePolicy::Link
        || !transform_graph_export_target_is_html(export)
    {
        return;
    }

    let has_css_destination = exports.iter().any(|candidate| {
        candidate.id != export.id
            && candidate.input == export.input
            && candidate.destination.is_some()
            && transform_graph_export_target_is_css(candidate)
    });
    if !has_css_destination {
        diagnostics.push(transform_runtime_diagnostic(
            export.destination.as_deref(),
            "cem.transform_runtime.stylesheet_export_missing",
            format!(
                "export `{}` uses `@style-policy=link` but no sibling CSS export with a destination targets input `{}`",
                export.id, export.input
            ),
        ));
    }
}

fn transform_graph_export_target_is_html(export: &TransformGraphExport) -> bool {
    export
        .target
        .as_ref()
        .map(format_identity_is_html)
        .unwrap_or_else(|| format_identity_is_html(&export.target_scope.format_identity()))
}

fn transform_graph_export_target_is_css(export: &TransformGraphExport) -> bool {
    export
        .target
        .as_ref()
        .map(format_identity_is_css)
        .unwrap_or_else(|| format_identity_is_css(&export.target_scope.format_identity()))
}

fn format_identity_is_html(identity: &FormatIdentity) -> bool {
    identity
        .content_type
        .as_deref()
        .map(content_type_essence)
        .is_some_and(|essence| matches!(essence.as_str(), HTML_CONTENT_TYPE | XHTML_CONTENT_TYPE))
        || matches!(
            identity.schema.as_deref(),
            Some(HTML_SCHEMA_URI) | Some(XHTML_SCHEMA_URI)
        )
}

fn format_identity_is_css(identity: &FormatIdentity) -> bool {
    identity
        .content_type
        .as_deref()
        .map(content_type_essence)
        .is_some_and(|essence| essence == CSS_CONTENT_TYPE)
        || identity.schema.as_deref() == Some(CSS_SCHEMA_URI)
}

fn transform_template_identity_error(message: impl Into<String>) -> TransformTemplateIdentityError {
    TransformTemplateIdentityError {
        code: TRANSFORM_TEMPLATE_UNSUPPORTED_CODE,
        message: message.into(),
    }
}

fn validate_transform_execution_policy(policy: &TransformExecutionPolicy) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if !matches!(
        policy.runtime_phase,
        TransformRuntimePhase::CemQlFragment
            | TransformRuntimePhase::CemNativeModules
            | TransformRuntimePhase::XsltParity
    ) {
        diagnostics.push(transform_runtime_diagnostic(
            None,
            "cem.transform_runtime.phase_unsupported",
            "transform runtime currently supports only the `cem-ql-fragment`, `cem-native-modules`, and `xslt-parity` phases",
        ));
    }
    if policy.cardinality != TransformCardinalityMode::OneToOne {
        diagnostics.push(transform_runtime_diagnostic(
            None,
            "cem.transform_runtime.cardinality_unsupported",
            "transform runtime currently supports only one-to-one stages",
        ));
    }
    if policy.duplicate_destination_policy != TransformDuplicateDestinationPolicy::Reject {
        diagnostics.push(transform_runtime_diagnostic(
            None,
            "cem.transform_runtime.duplicate_destination_policy_unsupported",
            "transform runtime currently requires duplicate output destinations to be rejected",
        ));
    }
    if policy.failure_policy != TransformFailurePolicy::FailFast {
        diagnostics.push(transform_runtime_diagnostic(
            None,
            "cem.transform_runtime.failure_policy_unsupported",
            "transform runtime currently supports only fail-fast execution",
        ));
    }
    if policy.output_policy != TransformOutputPolicy::ContentPrimary {
        diagnostics.push(transform_runtime_diagnostic(
            None,
            "cem.transform_runtime.output_policy_unsupported",
            "transform runtime currently supports only content-primary output",
        ));
    }
    diagnostics
}

fn validate_transform_stage_runtime_contract(
    stage_id: &str,
    uri: Option<&str>,
    template_kind: TransformTemplateKind,
    template_entrypoint: &TransformTemplateEntrypoint,
    params: &BTreeMap<String, Value>,
    policy: &TransformExecutionPolicy,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if policy.runtime_phase == TransformRuntimePhase::CemQlFragment
        && template_kind != TransformTemplateKind::CemNative
    {
        diagnostics.push(transform_runtime_diagnostic(
            uri,
            "cem.transform_runtime.template_kind_unsupported",
            format!(
                "transform stage `{stage_id}` uses `{template_kind:?}` template kind; the first runtime slice supports only CEM-native templates"
            ),
        ));
    }
    if policy.runtime_phase == TransformRuntimePhase::XsltParity
        && template_kind != TransformTemplateKind::Xslt
    {
        diagnostics.push(transform_runtime_diagnostic(
            uri,
            "cem.transform_runtime.template_kind_unsupported",
            format!(
                "transform stage `{stage_id}` uses `{template_kind:?}` template kind; the XSLT parity phase supports only XSLT templates"
            ),
        ));
    }
    if policy.runtime_phase == TransformRuntimePhase::CemQlFragment
        && !template_entrypoint.is_implicit()
    {
        diagnostics.push(transform_runtime_diagnostic(
            uri,
            "cem.transform_runtime.entrypoint_unsupported",
            format!(
                "transform stage `{stage_id}` declares a named template entrypoint; the first runtime slice supports only the implicit entrypoint"
            ),
        ));
    }
    if policy.runtime_phase == TransformRuntimePhase::CemQlFragment && !params.is_empty() {
        diagnostics.push(transform_runtime_diagnostic(
            uri,
            "cem.transform_runtime.params_unsupported",
            format!(
                "transform stage `{stage_id}` declares params; template params are reserved for the native module layer"
            ),
        ));
    }
}

fn validate_graph_id(
    kind: &str,
    id: &str,
    ids: &mut BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if id.trim().is_empty() {
        diagnostics.push(transform_runtime_diagnostic(
            None,
            "cem.transform_runtime.id_missing",
            format!("transform graph {kind} node requires a non-empty id"),
        ));
    } else if !ids.insert(id.to_owned()) {
        diagnostics.push(transform_runtime_diagnostic(
            None,
            "cem.transform_runtime.id_duplicate",
            format!("transform graph node id `{id}` is declared more than once"),
        ));
    }
}

fn validate_artifact_ref(
    owner_id: &str,
    field: &str,
    target_id: &str,
    artifacts: &BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if target_id.trim().is_empty() {
        diagnostics.push(transform_runtime_diagnostic(
            None,
            "cem.transform_runtime.ref_empty",
            format!("transform graph node `{owner_id}` has an empty `{field}` reference"),
        ));
    } else if !artifacts.contains(target_id) {
        diagnostics.push(transform_runtime_diagnostic(
            None,
            "cem.transform_runtime.ref_unknown",
            format!(
                "transform graph node `{owner_id}` references unknown artifact `{target_id}` via `{field}`"
            ),
        ));
    }
}

fn transform_runtime_diagnostic(
    uri: Option<&str>,
    code: &str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        uri: uri.map(str::to_owned),
        code: code.to_owned(),
        severity: Severity::Fatal,
        message: message.into(),
        ..Diagnostic::default()
    }
}

#[derive(Debug, Clone)]
pub struct EngineInput {
    pub uri: String,
    pub bytes: Vec<u8>,
    pub from_format: Option<InputFormat>,
    pub identity: Option<FormatIdentity>,
    pub root_scope: ScopeConfig,
}

#[derive(Debug, Clone)]
pub struct ParseRequest {
    pub input: EngineInput,
    pub projection: ParseProjection,
    pub fail_level: FailLevel,
    pub preserve_source_offsets: bool,
    pub context: EngineContext,
}

#[derive(Debug, Clone)]
pub struct ValidateRequest {
    pub inputs: Vec<EngineInput>,
    pub projection: ValidateProjection,
    pub fail_level: FailLevel,
    pub context: EngineContext,
}

#[derive(Debug, Clone)]
pub struct CheckRequest {
    pub inputs: Vec<EngineInput>,
    pub projection: ValidateProjection,
    pub fail_level: FailLevel,
    pub zero_hard_violations: bool,
    pub context: EngineContext,
}

#[derive(Debug, Clone)]
pub struct InspectRequest {
    pub input: EngineInput,
    pub show: InspectView,
    pub context: EngineContext,
}

#[derive(Debug, Clone)]
pub struct ConvertRequest {
    pub input: EngineInput,
    pub to_format: LayerFormat,
    pub preserve_source_offsets: bool,
    pub context: EngineContext,
    pub target: Option<FormatIdentity>,
    pub target_scope: ScopeConfig,
    pub scheduler_scope_id: u32,
}

#[derive(Debug, Clone)]
pub struct TemplateInput {
    pub uri: String,
    pub bytes: Vec<u8>,
    pub identity: Option<FormatIdentity>,
    pub root_scope: ScopeConfig,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TransformSchedulerScopeIds {
    pub data_load: u32,
    pub template_load: u32,
    pub execution: u32,
    pub output: u32,
}

#[derive(Debug, Clone)]
pub struct TransformRequest {
    pub data: EngineInput,
    pub template: TemplateInput,
    pub template_kind: TransformTemplateKind,
    pub template_entrypoint: TransformTemplateEntrypoint,
    pub params: BTreeMap<String, Value>,
    pub preserve_source_offsets: bool,
    pub context: EngineContext,
    pub target: Option<FormatIdentity>,
    pub target_scope: ScopeConfig,
    pub scheduler_scope_ids: TransformSchedulerScopeIds,
    pub execution_policy: TransformExecutionPolicy,
}

#[derive(Debug, Clone)]
pub struct TransformGraphRequest {
    pub imports: Vec<TransformGraphImport>,
    pub joins: Vec<TransformGraphJoin>,
    pub stages: Vec<TransformGraphStage>,
    pub importmap_rewrites: Vec<TransformGraphImportMapRewrite>,
    pub exports: Vec<TransformGraphExport>,
    pub edges: Vec<TransformGraphDependency>,
    pub preserve_source_offsets: bool,
    pub context: EngineContext,
    pub execution_policy: TransformExecutionPolicy,
}

#[derive(Debug, Clone)]
pub struct TransformGraphImport {
    pub id: String,
    pub input: EngineInput,
    pub scheduler_scope_id: u32,
}

#[derive(Debug, Clone)]
pub struct TransformGraphJoin {
    pub id: String,
    pub mode: TransformGraphJoinMode,
    pub input_names: Vec<String>,
    pub inputs: Vec<TransformGraphJoinInput>,
    pub bindings: BTreeMap<String, String>,
    pub scheduler_scope_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformGraphJoinMode {
    Collect,
    GroupBy,
    MatchBy,
    Zip,
}

#[derive(Debug, Clone)]
pub struct TransformGraphJoinInput {
    pub input_name: String,
    pub artifact_id: String,
    pub bindings: BTreeMap<String, String>,
    pub destination: Option<String>,
    pub target: Option<FormatIdentity>,
}

#[derive(Debug, Clone)]
pub struct TransformGraphStage {
    pub id: String,
    pub template: TemplateInput,
    pub template_kind: TransformTemplateKind,
    pub template_entrypoint: TransformTemplateEntrypoint,
    pub params: BTreeMap<String, Value>,
    pub execution_policy: TransformExecutionPolicy,
    pub target: Option<FormatIdentity>,
    pub primary_input: String,
    pub secondary_inputs: BTreeMap<String, String>,
    pub scheduler_scope_ids: TransformStageSchedulerScopeIds,
}

#[derive(Debug, Clone)]
pub struct TransformGraphImportMapRewrite {
    pub id: String,
    pub primary_input: String,
    pub source_imports: BTreeMap<String, String>,
    pub target_imports: BTreeMap<String, String>,
    pub mode: TransformGraphImportMapRewriteMode,
    pub missing_policy: TransformGraphImportMapMissingPolicy,
    pub scheduler_scope_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformGraphImportMapRewriteMode {
    ReplaceImports,
    Merge,
    ReplaceScript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformGraphImportMapMissingPolicy {
    Error,
    Ignore,
    Insert,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TransformStageSchedulerScopeIds {
    pub template_load: u32,
    pub execution: u32,
}

#[derive(Debug, Clone)]
pub struct TransformGraphExport {
    pub id: String,
    pub input: String,
    pub destination: Option<String>,
    pub target: Option<FormatIdentity>,
    pub target_scope: ScopeConfig,
    pub style_policy: TransformGraphStylePolicy,
    pub scheduler_scope_id: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformGraphStylePolicy {
    #[default]
    Auto,
    Inline,
    Link,
    Omit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformGraphDependency {
    pub from: String,
    pub to: String,
    pub role: TransformGraphDependencyRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformGraphDependencyRole {
    Parent,
    PrimaryInput,
    SecondaryInput,
}

#[derive(Debug, Clone)]
pub struct TraceRequest {
    pub input: EngineInput,
    pub projection: TraceProjection,
    pub context: EngineContext,
}

#[derive(Debug, Clone)]
pub struct BenchRequest {
    pub inputs: Vec<EngineInput>,
    pub projection: BenchProjection,
    pub iterations: u32,
    pub budget_ms: Option<u64>,
    pub profile: Option<BenchProfile>,
    pub cold_cache: bool,
    pub context: EngineContext,
}

#[derive(Debug, Clone)]
pub struct FixtureValidateRequest {
    pub inputs: Vec<EngineInput>,
    pub fail_level: FailLevel,
    pub zero_hard_violations: bool,
    pub context: EngineContext,
}

#[derive(Debug, Clone)]
pub struct FixtureRoundtripRequest {
    pub inputs: Vec<EngineInput>,
    pub to_format: LayerFormat,
    pub context: EngineContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResponse {
    pub primary: Value,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResponse {
    pub report: Report,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResponse {
    pub report: Report,
    #[serde(rename = "hardViolationCount")]
    pub hard_violation_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectResponse {
    pub view: InspectView,
    pub body: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimaryBytes {
    pub content_type: String,
    pub schema: Option<String>,
    pub format_version: String,
    pub hash_scheme: String,
    pub hash: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConvertExecutionMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub converter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rust_fallback: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_pipeline: Option<ConvertOutputPipelineMetadata>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConvertOutputPipelineMetadata {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stages: Vec<ConvertOutputPipelineStageMetadata>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConvertOutputPipelineStageMetadata {
    pub stage: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produces: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertResponse {
    pub primary: Value,
    #[serde(skip)]
    pub primary_bytes: Option<PrimaryBytes>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversion: Option<ConvertExecutionMetadata>,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(rename = "schedulerTrace", default)]
    pub scheduler_trace: SchedulerTraceReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformResponse {
    pub primary: Value,
    #[serde(rename = "sourceMap", default, skip_serializing_if = "Option::is_none")]
    pub source_map: Option<SourceMapStack>,
    #[serde(rename = "outputSpans", default, skip_serializing_if = "Vec::is_empty")]
    pub output_spans: Vec<OutputSpan>,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(rename = "schedulerTrace", default)]
    pub scheduler_trace: SchedulerTraceReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformGraphResponse {
    #[serde(default)]
    pub artifacts: Vec<TransformGraphArtifact>,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(rename = "schedulerTrace", default)]
    pub scheduler_trace: SchedulerTraceReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformGraphArtifact {
    pub export_id: String,
    pub input: String,
    #[serde(default)]
    pub destination: Option<String>,
    #[serde(default)]
    pub identity: Option<FormatIdentity>,
    pub primary: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_map: Option<SourceMapStack>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_spans: Vec<OutputSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceResponse {
    pub body: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchResponse {
    pub body: Value,
    #[serde(rename = "budgetExceeded")]
    pub budget_exceeded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureValidateResponse {
    pub report: Report,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureRoundtripResponse {
    pub report: Report,
    pub artifacts: Vec<Value>,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum EngineError {
    NotImplemented,
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    SchemaResolution(String),
    Internal(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::NotImplemented => f.write_str("parser engine not yet implemented"),
            EngineError::Io { path, source } => {
                write!(f, "I/O error for `{}`: {}", path.display(), source)
            }
            EngineError::SchemaResolution(msg) => write!(f, "schema resolution error: {msg}"),
            EngineError::Internal(msg) => write!(f, "internal engine error: {msg}"),
        }
    }
}

impl std::error::Error for EngineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EngineError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub type EngineResult<T> = Result<T, EngineError>;

pub trait CemMlEngine {
    fn parse(&self, request: ParseRequest) -> EngineResult<ParseResponse>;
    fn validate(&self, request: ValidateRequest) -> EngineResult<ValidateResponse>;
    fn check(&self, request: CheckRequest) -> EngineResult<CheckResponse>;
    fn inspect(&self, request: InspectRequest) -> EngineResult<InspectResponse>;
    fn convert(&self, request: ConvertRequest) -> EngineResult<ConvertResponse>;
    fn transform(&self, _: TransformRequest) -> EngineResult<TransformResponse> {
        Err(EngineError::NotImplemented)
    }
    fn transform_graph(&self, _: TransformGraphRequest) -> EngineResult<TransformGraphResponse> {
        Err(EngineError::NotImplemented)
    }
    fn trace(&self, request: TraceRequest) -> EngineResult<TraceResponse>;
    fn bench(&self, request: BenchRequest) -> EngineResult<BenchResponse>;
    fn fixture_validate(
        &self,
        request: FixtureValidateRequest,
    ) -> EngineResult<FixtureValidateResponse>;
    fn fixture_roundtrip(
        &self,
        request: FixtureRoundtripRequest,
    ) -> EngineResult<FixtureRoundtripResponse>;
}

#[derive(Debug, Default)]
pub struct NotImplementedEngine;

impl CemMlEngine for NotImplementedEngine {
    fn parse(&self, _: ParseRequest) -> EngineResult<ParseResponse> {
        Err(EngineError::NotImplemented)
    }
    fn validate(&self, _: ValidateRequest) -> EngineResult<ValidateResponse> {
        Err(EngineError::NotImplemented)
    }
    fn check(&self, _: CheckRequest) -> EngineResult<CheckResponse> {
        Err(EngineError::NotImplemented)
    }
    fn inspect(&self, _: InspectRequest) -> EngineResult<InspectResponse> {
        Err(EngineError::NotImplemented)
    }
    fn convert(&self, _: ConvertRequest) -> EngineResult<ConvertResponse> {
        Err(EngineError::NotImplemented)
    }
    fn trace(&self, _: TraceRequest) -> EngineResult<TraceResponse> {
        Err(EngineError::NotImplemented)
    }
    fn bench(&self, _: BenchRequest) -> EngineResult<BenchResponse> {
        Err(EngineError::NotImplemented)
    }
    fn fixture_validate(&self, _: FixtureValidateRequest) -> EngineResult<FixtureValidateResponse> {
        Err(EngineError::NotImplemented)
    }
    fn fixture_roundtrip(
        &self,
        _: FixtureRoundtripRequest,
    ) -> EngineResult<FixtureRoundtripResponse> {
        Err(EngineError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine_input(uri: &str, content_type: &str) -> EngineInput {
        let root_scope = ScopeConfig {
            default_content_type: Some(content_type.to_owned()),
            ..ScopeConfig::default()
        };
        EngineInput {
            uri: uri.to_owned(),
            bytes: Vec::new(),
            from_format: None,
            identity: root_scope.format_identity_option(),
            root_scope,
        }
    }

    fn template_input(uri: &str, content_type: &str) -> TemplateInput {
        let root_scope = ScopeConfig {
            default_content_type: Some(content_type.to_owned()),
            ..ScopeConfig::default()
        };
        TemplateInput {
            uri: uri.to_owned(),
            bytes: Vec::new(),
            identity: root_scope.format_identity_option(),
            root_scope,
        }
    }

    fn has_diagnostic(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn transform_template_identity_classifies_xslt_and_cem_native_templates() {
        let xslt = FormatIdentity {
            content_type: Some("application/xslt+xml; charset=utf-8".to_owned()),
            ..FormatIdentity::default()
        };
        let legacy_xslt = FormatIdentity {
            content_type: Some("text/custom-element-xslt".to_owned()),
            ..FormatIdentity::default()
        };
        let cem_content_type = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let cem_schema = FormatIdentity {
            schema: Some(crate::schema::ir::CEM_CORE_NAMESPACE.to_owned()),
            ..FormatIdentity::default()
        };
        let xslt_namespace = FormatIdentity {
            namespaces: BTreeMap::from([(
                "xsl".to_owned(),
                crate::schema::xslt::XSL_NAMESPACE.to_owned(),
            )]),
            ..FormatIdentity::default()
        };
        let xslt_schema = FormatIdentity {
            schema: Some(crate::schema::registry::XSLT_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        assert_eq!(
            classify_transform_template_identity(&xslt),
            Ok(TransformTemplateKind::Xslt)
        );
        assert_eq!(
            classify_transform_template_identity(&legacy_xslt),
            Ok(TransformTemplateKind::Xslt)
        );
        assert_eq!(
            classify_transform_template_identity(&cem_content_type),
            Ok(TransformTemplateKind::CemNative)
        );
        assert_eq!(
            classify_transform_template_identity(&cem_schema),
            Ok(TransformTemplateKind::CemNative)
        );
        assert_eq!(
            classify_transform_template_identity(&xslt_namespace),
            Ok(TransformTemplateKind::Xslt)
        );
        assert_eq!(
            classify_transform_template_identity(&xslt_schema),
            Ok(TransformTemplateKind::Xslt)
        );
    }

    #[test]
    fn transform_template_identity_rejects_unknown_template_identity() {
        let unknown = FormatIdentity {
            content_type: Some("application/octet-stream".to_owned()),
            ..FormatIdentity::default()
        };

        let error = classify_transform_template_identity(&unknown).unwrap_err();
        assert_eq!(error.code, TRANSFORM_TEMPLATE_UNSUPPORTED_CODE);
        assert!(error.message.contains("application/octet-stream"));
    }

    #[test]
    fn transform_template_identity_uses_runtime_adapter_registry() {
        let mut registry = TransformTemplateAdapterRegistry::new();
        registry.register(
            crate::transform_template::StaticTransformTemplateAdapter::new(
                "cem-native-template-v2",
                TransformTemplateKind::CemNative,
                &[],
                &["https://cem.dev/ns/template/cem-native/2"],
                &[],
            ),
        );
        let identity = FormatIdentity {
            schema: Some("https://cem.dev/ns/template/cem-native/2".to_owned()),
            ..FormatIdentity::default()
        };

        assert_eq!(
            classify_transform_template_identity_with_registry(&identity, &registry),
            Ok(TransformTemplateKind::CemNative)
        );
    }

    #[test]
    fn transform_template_identity_rejects_ambiguous_runtime_adapters() {
        let mut registry = TransformTemplateAdapterRegistry::new();
        registry.register(
            crate::transform_template::StaticTransformTemplateAdapter::new(
                "one",
                TransformTemplateKind::CemNative,
                &["text/cem-ml"],
                &[],
                &[],
            ),
        );
        registry.register(
            crate::transform_template::StaticTransformTemplateAdapter::new(
                "two",
                TransformTemplateKind::CemNative,
                &["text/cem-ml"],
                &[],
                &[],
            ),
        );
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };

        let error = classify_transform_template_identity_with_registry(&identity, &registry)
            .expect_err("ambiguous template adapters should fail");
        assert_eq!(error.code, TRANSFORM_TEMPLATE_UNSUPPORTED_CODE);
        assert!(error.message.contains("one, two"));
    }

    #[test]
    fn transform_execution_policy_defaults_to_first_runtime_slice() {
        let policy = TransformExecutionPolicy::default();

        assert_eq!(policy.runtime_phase, TransformRuntimePhase::CemQlFragment);
        assert_eq!(policy.cardinality, TransformCardinalityMode::OneToOne);
        assert_eq!(
            policy.duplicate_destination_policy,
            TransformDuplicateDestinationPolicy::Reject
        );
        assert_eq!(policy.failure_policy, TransformFailurePolicy::FailFast);
        assert_eq!(policy.output_policy, TransformOutputPolicy::ContentPrimary);
        assert!(TransformTemplateEntrypoint::implicit().is_implicit());
        assert!(!TransformTemplateEntrypoint::named("main").is_implicit());
        assert_eq!(
            serde_json::to_value(TransformDiagnosticOrigin::TemplateCompile).unwrap(),
            serde_json::Value::String("template-compile".to_owned())
        );
    }

    #[test]
    fn transform_runtime_contract_accepts_minimal_cem_native_request() {
        let request = TransformRequest {
            data: engine_input("data.xml", "application/xml"),
            template: template_input("view.cem", "text/cem-ml"),
            template_kind: TransformTemplateKind::CemNative,
            template_entrypoint: TransformTemplateEntrypoint::implicit(),
            params: BTreeMap::new(),
            preserve_source_offsets: true,
            context: EngineContext::default(),
            target: None,
            target_scope: ScopeConfig::default(),
            scheduler_scope_ids: TransformSchedulerScopeIds::default(),
            execution_policy: TransformExecutionPolicy::default(),
        };

        assert!(validate_transform_request_runtime_contract(&request).is_empty());
    }

    #[test]
    fn transform_runtime_contract_accepts_native_module_phase() {
        let request = TransformRequest {
            data: engine_input("data.xml", "application/xml"),
            template: template_input("view.cem", "text/cem-ml"),
            template_kind: TransformTemplateKind::CemNative,
            template_entrypoint: TransformTemplateEntrypoint::named("main"),
            params: BTreeMap::from([("locale".to_owned(), serde_json::json!("en-US"))]),
            preserve_source_offsets: true,
            context: EngineContext::default(),
            target: None,
            target_scope: ScopeConfig::default(),
            scheduler_scope_ids: TransformSchedulerScopeIds::default(),
            execution_policy: TransformExecutionPolicy {
                runtime_phase: TransformRuntimePhase::CemNativeModules,
                ..TransformExecutionPolicy::default()
            },
        };

        assert!(validate_transform_request_runtime_contract(&request).is_empty());
    }

    #[test]
    fn transform_runtime_contract_accepts_xslt_entrypoint_and_params() {
        let request = TransformRequest {
            data: engine_input("data.xml", "application/xml"),
            template: template_input("view.xsl", "application/xslt+xml"),
            template_kind: TransformTemplateKind::Xslt,
            template_entrypoint: TransformTemplateEntrypoint::named("main"),
            params: BTreeMap::from([("locale".to_owned(), serde_json::json!("en-US"))]),
            preserve_source_offsets: true,
            context: EngineContext::default(),
            target: None,
            target_scope: ScopeConfig::default(),
            scheduler_scope_ids: TransformSchedulerScopeIds::default(),
            execution_policy: TransformExecutionPolicy {
                runtime_phase: TransformRuntimePhase::XsltParity,
                ..TransformExecutionPolicy::default()
            },
        };

        assert!(validate_transform_request_runtime_contract(&request).is_empty());
    }

    #[test]
    fn transform_runtime_contract_rejects_deferred_single_transform_features() {
        let request = TransformRequest {
            data: engine_input("data.xml", "application/xml"),
            template: template_input("view.xsl", "application/xslt+xml"),
            template_kind: TransformTemplateKind::Xslt,
            template_entrypoint: TransformTemplateEntrypoint::named("main"),
            params: BTreeMap::from([("locale".to_owned(), serde_json::json!("en-US"))]),
            preserve_source_offsets: true,
            context: EngineContext::default(),
            target: None,
            target_scope: ScopeConfig::default(),
            scheduler_scope_ids: TransformSchedulerScopeIds::default(),
            execution_policy: TransformExecutionPolicy::default(),
        };

        let diagnostics = validate_transform_request_runtime_contract(&request);
        assert!(has_diagnostic(
            &diagnostics,
            "cem.transform_runtime.template_kind_unsupported"
        ));
        assert!(has_diagnostic(
            &diagnostics,
            "cem.transform_runtime.entrypoint_unsupported"
        ));
        assert!(has_diagnostic(
            &diagnostics,
            "cem.transform_runtime.params_unsupported"
        ));
    }

    #[test]
    fn transform_graph_runtime_contract_validates_refs_and_destinations() {
        let request = TransformGraphRequest {
            imports: vec![TransformGraphImport {
                id: "book".to_owned(),
                input: engine_input("book.xml", "application/xml"),
                scheduler_scope_id: 1,
            }],
            joins: Vec::new(),
            stages: vec![
                TransformGraphStage {
                    id: "book".to_owned(),
                    template: template_input("report.cem", "text/cem-ml"),
                    template_kind: TransformTemplateKind::CemNative,
                    template_entrypoint: TransformTemplateEntrypoint::implicit(),
                    params: BTreeMap::new(),
                    execution_policy: TransformExecutionPolicy::default(),
                    target: None,
                    primary_input: "missing".to_owned(),
                    secondary_inputs: BTreeMap::from([(
                        "stats".to_owned(),
                        "missing-stats".to_owned(),
                    )]),
                    scheduler_scope_ids: TransformStageSchedulerScopeIds::default(),
                },
                TransformGraphStage {
                    id: "chart".to_owned(),
                    template: template_input("chart.cem", "text/cem-ml"),
                    template_kind: TransformTemplateKind::CemNative,
                    template_entrypoint: TransformTemplateEntrypoint::implicit(),
                    params: BTreeMap::new(),
                    execution_policy: TransformExecutionPolicy::default(),
                    target: None,
                    primary_input: "book".to_owned(),
                    secondary_inputs: BTreeMap::new(),
                    scheduler_scope_ids: TransformStageSchedulerScopeIds::default(),
                },
            ],
            importmap_rewrites: Vec::new(),
            exports: vec![
                TransformGraphExport {
                    id: "main".to_owned(),
                    input: "chart".to_owned(),
                    destination: Some("dist/report.html".to_owned()),
                    target: None,
                    target_scope: ScopeConfig::default(),
                    style_policy: TransformGraphStylePolicy::default(),
                    scheduler_scope_id: 4,
                },
                TransformGraphExport {
                    id: "chart-out".to_owned(),
                    input: "missing-output".to_owned(),
                    destination: Some("dist/report.html".to_owned()),
                    target: None,
                    target_scope: ScopeConfig::default(),
                    style_policy: TransformGraphStylePolicy::default(),
                    scheduler_scope_id: 5,
                },
            ],
            edges: Vec::new(),
            preserve_source_offsets: true,
            context: EngineContext::default(),
            execution_policy: TransformExecutionPolicy::default(),
        };

        let diagnostics = validate_transform_graph_runtime_contract(&request);
        assert!(has_diagnostic(
            &diagnostics,
            "cem.transform_runtime.id_duplicate"
        ));
        assert!(has_diagnostic(
            &diagnostics,
            "cem.transform_runtime.ref_unknown"
        ));
        assert!(has_diagnostic(
            &diagnostics,
            "cem.transform_runtime.duplicate_destination"
        ));
    }

    #[test]
    fn transform_graph_runtime_contract_rejects_link_style_policy_without_css_export() {
        let target_scope = ScopeConfig {
            default_content_type: Some("text/html".to_owned()),
            ..ScopeConfig::default()
        };
        let request = TransformGraphRequest {
            imports: vec![TransformGraphImport {
                id: "page".to_owned(),
                input: engine_input("page.html", "text/html"),
                scheduler_scope_id: 1,
            }],
            joins: Vec::new(),
            stages: Vec::new(),
            importmap_rewrites: Vec::new(),
            exports: vec![TransformGraphExport {
                id: "html".to_owned(),
                input: "page".to_owned(),
                destination: Some("dist/page.html".to_owned()),
                target: target_scope.format_identity_option(),
                target_scope,
                style_policy: TransformGraphStylePolicy::Link,
                scheduler_scope_id: 2,
            }],
            edges: Vec::new(),
            preserve_source_offsets: true,
            context: EngineContext::default(),
            execution_policy: TransformExecutionPolicy::default(),
        };

        let diagnostics = validate_transform_graph_runtime_contract(&request);
        assert!(has_diagnostic(
            &diagnostics,
            "cem.transform_runtime.stylesheet_export_missing"
        ));
    }

    #[test]
    fn transform_request_models_data_template_and_target_separately() {
        let target_scope = ScopeConfig {
            default_content_type: Some("text/html".to_owned()),
            ..ScopeConfig::default()
        };
        let request = TransformRequest {
            data: engine_input("data.xml", "application/xml"),
            template: template_input("view.xsl", "application/xslt+xml"),
            template_kind: TransformTemplateKind::Xslt,
            template_entrypoint: TransformTemplateEntrypoint::implicit(),
            params: BTreeMap::new(),
            preserve_source_offsets: true,
            context: EngineContext::default(),
            target: target_scope.format_identity_option(),
            target_scope,
            scheduler_scope_ids: TransformSchedulerScopeIds {
                data_load: 1,
                template_load: 2,
                execution: 3,
                output: 4,
            },
            execution_policy: TransformExecutionPolicy::default(),
        };

        assert_eq!(request.data.uri, "data.xml");
        assert_eq!(request.template.uri, "view.xsl");
        assert_eq!(request.template_kind, TransformTemplateKind::Xslt);
        assert!(request.template_entrypoint.is_implicit());
        assert!(request.params.is_empty());
        assert_eq!(
            request
                .template
                .identity
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some("application/xslt+xml")
        );
        assert_eq!(
            request
                .target
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some("text/html")
        );
        assert_eq!(request.scheduler_scope_ids.execution, 3);
        assert_eq!(
            request.execution_policy.runtime_phase,
            TransformRuntimePhase::CemQlFragment
        );
    }

    #[test]
    fn transform_graph_request_models_import_stage_export_and_join_edges() {
        let target_scope = ScopeConfig {
            default_content_type: Some("text/html".to_owned()),
            ..ScopeConfig::default()
        };
        let mut secondary_inputs = BTreeMap::new();
        secondary_inputs.insert("stats".to_owned(), "stats".to_owned());

        let request = TransformGraphRequest {
            imports: vec![
                TransformGraphImport {
                    id: "book".to_owned(),
                    input: engine_input("book.xml", "application/xml"),
                    scheduler_scope_id: 1,
                },
                TransformGraphImport {
                    id: "stats".to_owned(),
                    input: engine_input("stats.xml", "application/xml"),
                    scheduler_scope_id: 2,
                },
            ],
            joins: vec![TransformGraphJoin {
                id: "all-inputs".to_owned(),
                mode: TransformGraphJoinMode::Collect,
                input_names: vec!["primary".to_owned(), "stats".to_owned()],
                inputs: vec![
                    TransformGraphJoinInput {
                        input_name: "primary".to_owned(),
                        artifact_id: "book".to_owned(),
                        bindings: BTreeMap::from([("stem".to_owned(), "book".to_owned())]),
                        destination: None,
                        target: None,
                    },
                    TransformGraphJoinInput {
                        input_name: "stats".to_owned(),
                        artifact_id: "stats".to_owned(),
                        bindings: BTreeMap::from([("stem".to_owned(), "stats".to_owned())]),
                        destination: None,
                        target: None,
                    },
                ],
                bindings: BTreeMap::from([("count".to_owned(), "2".to_owned())]),
                scheduler_scope_id: 3,
            }],
            stages: vec![TransformGraphStage {
                id: "report".to_owned(),
                template: template_input("report.cem", "text/cem-ml"),
                template_kind: TransformTemplateKind::CemNative,
                template_entrypoint: TransformTemplateEntrypoint {
                    name: Some("main".to_owned()),
                },
                params: BTreeMap::from([(
                    "locale".to_owned(),
                    serde_json::Value::String("en-US".to_owned()),
                )]),
                execution_policy: TransformExecutionPolicy {
                    runtime_phase: TransformRuntimePhase::CemNativeModules,
                    ..TransformExecutionPolicy::default()
                },
                target: None,
                primary_input: "book".to_owned(),
                secondary_inputs,
                scheduler_scope_ids: TransformStageSchedulerScopeIds {
                    template_load: 4,
                    execution: 5,
                },
            }],
            importmap_rewrites: Vec::new(),
            exports: vec![TransformGraphExport {
                id: "html".to_owned(),
                input: "report".to_owned(),
                destination: Some("dist/report.html".to_owned()),
                target: target_scope.format_identity_option(),
                target_scope,
                style_policy: TransformGraphStylePolicy::default(),
                scheduler_scope_id: 6,
            }],
            edges: vec![
                TransformGraphDependency {
                    from: "book".to_owned(),
                    to: "report".to_owned(),
                    role: TransformGraphDependencyRole::PrimaryInput,
                },
                TransformGraphDependency {
                    from: "stats".to_owned(),
                    to: "report".to_owned(),
                    role: TransformGraphDependencyRole::SecondaryInput,
                },
                TransformGraphDependency {
                    from: "book".to_owned(),
                    to: "all-inputs".to_owned(),
                    role: TransformGraphDependencyRole::PrimaryInput,
                },
                TransformGraphDependency {
                    from: "report".to_owned(),
                    to: "html".to_owned(),
                    role: TransformGraphDependencyRole::Parent,
                },
            ],
            preserve_source_offsets: true,
            context: EngineContext::default(),
            execution_policy: TransformExecutionPolicy::default(),
        };

        assert_eq!(request.imports.len(), 2);
        assert_eq!(request.joins[0].mode, TransformGraphJoinMode::Collect);
        assert_eq!(request.joins[0].inputs.len(), 2);
        assert_eq!(request.stages[0].primary_input, "book");
        assert_eq!(
            request.stages[0].template_entrypoint.name.as_deref(),
            Some("main")
        );
        assert_eq!(
            request.stages[0].params.get("locale"),
            Some(&serde_json::Value::String("en-US".to_owned()))
        );
        assert_eq!(
            request.stages[0].execution_policy.runtime_phase,
            TransformRuntimePhase::CemNativeModules
        );
        assert_eq!(
            request.stages[0]
                .secondary_inputs
                .get("stats")
                .map(String::as_str),
            Some("stats")
        );
        assert_eq!(
            request.exports[0]
                .target
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some("text/html")
        );
        assert!(request.edges.iter().any(|edge| {
            edge.from == "stats"
                && edge.to == "report"
                && edge.role == TransformGraphDependencyRole::SecondaryInput
        }));
    }

    #[test]
    fn transform_defaults_to_not_implemented() {
        let request = TransformRequest {
            data: engine_input("data.xml", "application/xml"),
            template: template_input("view.xsl", "application/xslt+xml"),
            template_kind: TransformTemplateKind::Xslt,
            preserve_source_offsets: false,
            context: EngineContext::default(),
            target: None,
            target_scope: ScopeConfig::default(),
            scheduler_scope_ids: TransformSchedulerScopeIds::default(),
            template_entrypoint: TransformTemplateEntrypoint::implicit(),
            params: BTreeMap::new(),
            execution_policy: TransformExecutionPolicy::default(),
        };

        let err = NotImplementedEngine.transform(request).unwrap_err();
        assert!(matches!(err, EngineError::NotImplemented));
    }

    #[test]
    fn transform_graph_defaults_to_not_implemented() {
        let request = TransformGraphRequest {
            imports: Vec::new(),
            joins: Vec::new(),
            stages: Vec::new(),
            importmap_rewrites: Vec::new(),
            exports: Vec::new(),
            edges: Vec::new(),
            preserve_source_offsets: false,
            context: EngineContext::default(),
            execution_policy: TransformExecutionPolicy::default(),
        };

        let err = NotImplementedEngine.transform_graph(request).unwrap_err();
        assert!(matches!(err, EngineError::NotImplemented));
    }
}
