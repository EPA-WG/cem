//! Public CEM-QL entry points.

use std::collections::BTreeMap;

use cem_ml::diagnostics::Diagnostic;
use cem_ml::scheduler::ScopePolicy;
use cem_ml::schema::compiler::ContentHash;
use cem_ml::schema::SchemaFrame;
use cem_ml::source_map::SourceMapStack;

use crate::artifact::CompiledArtifact;
use crate::eval::{Evaluator, ItemStream, QueryContextScope};
use crate::ir::lower::IrLowerer;
use crate::ir::CompiledQuery;
use crate::parser::{Parser, SurfaceModule};
use crate::resolve::overlay::OverlayMap;

/// Compile a CEM-QL query module source string into a typed IR.
pub fn compile(source: &str, context: &CompileContext) -> Result<CompiledQuery, CompileError> {
    let parsed = parse(source);
    if let Some(diagnostic) = parsed
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Err(CompileError::diagnostic(diagnostic));
    }
    let lowered = IrLowerer::new()
        .with_policy_bindings(context.policy_bindings.keys().cloned())
        .lower_module(&parsed.module);
    if let Some(diagnostic) = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Err(CompileError::diagnostic(diagnostic));
    }
    Ok(lowered.query)
}

/// Evaluate a compiled query against a query context scope.
pub fn evaluate(query: &CompiledQuery, ctx: &EvaluationContext) -> ItemStream {
    Evaluator::evaluate(query, ctx)
}

pub fn compile_artifact(
    source: &str,
    context: &CompileContext,
) -> Result<CompiledArtifact, CompileError> {
    compile(source, context).map(|query| CompiledArtifact::from_query(&query))
}

pub fn reload_artifact(artifact: &CompiledArtifact) -> Result<CompiledQuery, LoadError> {
    artifact.reload().map_err(LoadError::unsupported)
}

/// Parse-only entry point for tooling.
pub fn parse(source: &str) -> ParseResult {
    Parser::new(source).parse_module()
}

/// Load a compiled binary artifact by content hash.
pub fn load(_hash: ContentHash, _ctx: &LoadContext) -> Result<CompiledQuery, LoadError> {
    Err(LoadError::unsupported(
        "CEM-QL artifact loading is not implemented yet",
    ))
}

#[derive(Debug, Clone, Default)]
pub struct CompileContext {
    pub schema_frame: Option<SchemaFrame>,
    pub overlay: OverlayMap,
    pub diagnostics: Vec<Diagnostic>,
    pub source_map_base: SourceMapStack,
    pub policy_bindings: BTreeMap<String, ItemStream>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    pub code: &'static str,
    pub message: String,
}

impl CompileError {
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self {
            code: "cem.ql.unsupported",
            message: message.into(),
        }
    }

    pub fn diagnostic(diagnostic: &Diagnostic) -> Self {
        Self {
            code: "cem.ql.compile_failed",
            message: format!("{}: {}", diagnostic.code, diagnostic.message),
        }
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CompileError {}

#[derive(Debug, Clone)]
pub struct EvaluationContext {
    pub scope: QueryContextScope,
    pub scope_policy: ScopePolicy,
    pub diagnostics: Vec<Diagnostic>,
    pub policy_bindings: BTreeMap<String, ItemStream>,
}

#[derive(Debug, Clone)]
pub struct ParseResult {
    pub module: SurfaceModule,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Default)]
pub struct LoadContext {
    pub expected_hash: Option<ContentHash>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    pub code: &'static str,
    pub message: String,
}

impl LoadError {
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self {
            code: "cem.ql.unsupported",
            message: message.into(),
        }
    }
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for LoadError {}
