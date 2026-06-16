use crate::diagnostics::Diagnostic;
use crate::report::{Report, SchedulerTraceReport};
use crate::resolver::ResolverRegistry;
use crate::run_config::{SchedulerConfig, ScopeConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
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

#[derive(Debug, Clone, Default)]
pub struct EngineContext {
    pub schema: Option<String>,
    pub content_type: Option<String>,
    pub base_uri: Option<String>,
    pub scheduler: SchedulerConfig,
    pub resolver_registry: ResolverRegistry,
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
    if let Some(content_type) = identity.content_type.as_deref() {
        if crate::legacy_custom_element::is_legacy_custom_element_content_type(content_type) {
            return Ok(TransformTemplateKind::Xslt);
        }
        if is_cem_native_template_content_type(content_type) {
            return Ok(TransformTemplateKind::CemNative);
        }
        return Err(transform_template_identity_error(format!(
            "no transform template adapter matched content type `{}`",
            content_type_essence(content_type)
        )));
    }

    let schema = identity
        .schema
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    if !schema.is_empty() {
        if schema == crate::schema::ir::CEM_CORE_NAMESPACE {
            return Ok(TransformTemplateKind::CemNative);
        }
        return Err(transform_template_identity_error(format!(
            "no transform template adapter matched schema `{schema}`"
        )));
    }

    if identity
        .default_namespace
        .as_deref()
        .is_some_and(|uri| uri == crate::schema::ir::CEM_CORE_NAMESPACE)
        || identity
            .namespaces
            .values()
            .any(|uri| uri == crate::schema::ir::CEM_CORE_NAMESPACE)
    {
        return Ok(TransformTemplateKind::CemNative);
    }

    if identity
        .default_namespace
        .as_deref()
        .is_some_and(crate::schema::xslt::is_xslt_namespace)
        || identity
            .namespaces
            .values()
            .any(|uri| crate::schema::xslt::is_xslt_namespace(uri))
    {
        return Ok(TransformTemplateKind::Xslt);
    }

    Err(transform_template_identity_error(
        "transform template identity requires a supported content type, schema, or namespace",
    ))
}

fn is_cem_native_template_content_type(content_type: &str) -> bool {
    matches!(
        content_type_essence(content_type).as_str(),
        "application/cem+xml" | "application/cem" | "text/cem" | "text/cem-ml"
    )
}

fn content_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

fn transform_template_identity_error(message: impl Into<String>) -> TransformTemplateIdentityError {
    TransformTemplateIdentityError {
        code: TRANSFORM_TEMPLATE_UNSUPPORTED_CODE,
        message: message.into(),
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
    pub preserve_source_offsets: bool,
    pub context: EngineContext,
    pub target: Option<FormatIdentity>,
    pub target_scope: ScopeConfig,
    pub scheduler_scope_ids: TransformSchedulerScopeIds,
}

#[derive(Debug, Clone)]
pub struct TransformGraphRequest {
    pub imports: Vec<TransformGraphImport>,
    pub stages: Vec<TransformGraphStage>,
    pub exports: Vec<TransformGraphExport>,
    pub edges: Vec<TransformGraphDependency>,
    pub preserve_source_offsets: bool,
    pub context: EngineContext,
}

#[derive(Debug, Clone)]
pub struct TransformGraphImport {
    pub id: String,
    pub input: EngineInput,
    pub scheduler_scope_id: u32,
}

#[derive(Debug, Clone)]
pub struct TransformGraphStage {
    pub id: String,
    pub template: TemplateInput,
    pub template_kind: TransformTemplateKind,
    pub primary_input: String,
    pub secondary_inputs: BTreeMap<String, String>,
    pub scheduler_scope_ids: TransformStageSchedulerScopeIds,
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
    pub scheduler_scope_id: u32,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertResponse {
    pub primary: Value,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(rename = "schedulerTrace", default)]
    pub scheduler_trace: SchedulerTraceReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformResponse {
    pub primary: Value,
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
    #[serde(default)]
    pub destination: Option<String>,
    #[serde(default)]
    pub identity: Option<FormatIdentity>,
    pub primary: Value,
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
    fn transform_request_models_data_template_and_target_separately() {
        let target_scope = ScopeConfig {
            default_content_type: Some("text/html".to_owned()),
            ..ScopeConfig::default()
        };
        let request = TransformRequest {
            data: engine_input("data.xml", "application/xml"),
            template: template_input("view.xsl", "application/xslt+xml"),
            template_kind: TransformTemplateKind::Xslt,
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
        };

        assert_eq!(request.data.uri, "data.xml");
        assert_eq!(request.template.uri, "view.xsl");
        assert_eq!(request.template_kind, TransformTemplateKind::Xslt);
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
            stages: vec![TransformGraphStage {
                id: "report".to_owned(),
                template: template_input("report.xsl", "application/xslt+xml"),
                template_kind: TransformTemplateKind::Xslt,
                primary_input: "book".to_owned(),
                secondary_inputs,
                scheduler_scope_ids: TransformStageSchedulerScopeIds {
                    template_load: 3,
                    execution: 4,
                },
            }],
            exports: vec![TransformGraphExport {
                id: "html".to_owned(),
                input: "report".to_owned(),
                destination: Some("dist/report.html".to_owned()),
                target: target_scope.format_identity_option(),
                target_scope,
                scheduler_scope_id: 5,
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
                    from: "report".to_owned(),
                    to: "html".to_owned(),
                    role: TransformGraphDependencyRole::Parent,
                },
            ],
            preserve_source_offsets: true,
            context: EngineContext::default(),
        };

        assert_eq!(request.imports.len(), 2);
        assert_eq!(request.stages[0].primary_input, "book");
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
        };

        let err = NotImplementedEngine.transform(request).unwrap_err();
        assert!(matches!(err, EngineError::NotImplemented));
    }

    #[test]
    fn transform_graph_defaults_to_not_implemented() {
        let request = TransformGraphRequest {
            imports: Vec::new(),
            stages: Vec::new(),
            exports: Vec::new(),
            edges: Vec::new(),
            preserve_source_offsets: false,
            context: EngineContext::default(),
        };

        let err = NotImplementedEngine.transform_graph(request).unwrap_err();
        assert!(matches!(err, EngineError::NotImplemented));
    }
}
