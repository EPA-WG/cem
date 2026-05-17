use crate::diagnostics::Diagnostic;
use crate::report::Report;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
}

#[derive(Debug, Clone)]
pub struct EngineInput {
    pub uri: String,
    pub bytes: Vec<u8>,
    pub from_format: Option<InputFormat>,
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
    fn fixture_validate(
        &self,
        _: FixtureValidateRequest,
    ) -> EngineResult<FixtureValidateResponse> {
        Err(EngineError::NotImplemented)
    }
    fn fixture_roundtrip(
        &self,
        _: FixtureRoundtripRequest,
    ) -> EngineResult<FixtureRoundtripResponse> {
        Err(EngineError::NotImplemented)
    }
}
