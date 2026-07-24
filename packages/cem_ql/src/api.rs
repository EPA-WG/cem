//! Public CEM-QL entry points.

use std::collections::BTreeMap;

use cem_ml::content_cache::{CacheMode, ContentHash};
use cem_ml::diagnostics::Diagnostic;
use cem_ml::scheduler::ScopePolicy;
use cem_ml::schema::SchemaFrame;
use cem_ml::source_map::SourceMapStack;

use crate::artifact::CompiledArtifact;
use crate::eval::{Evaluator, ItemStream, QueryContextScope};
use crate::ir::lower::IrLowerer;
use crate::ir::CompiledQuery;
use crate::parser::{Parser, SurfaceModule};
use crate::resolve::overlay::OverlayMap;
use crate::resolve::{ImportPolicy, NameResolver};
use crate::semantic::validate_module_shape;
use crate::types::{TyConfig, Type, TypeChecker};

#[cfg(any(target_arch = "wasm32", test))]
mod json_boundary;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

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
    let import_report = resolve_imports(&parsed.module, &context.import_policy);
    if let Some(diagnostic) = import_report
        .iter()
        .find(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Err(CompileError::diagnostic(diagnostic));
    }
    let type_report = type_check(&parsed.module, context);
    if let Some(diagnostic) = type_report
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
    compile(source, context).map(|query| CompiledArtifact::from_query_with_context(&query, context))
}

pub fn reload_artifact(artifact: &CompiledArtifact) -> Result<CompiledQuery, LoadError> {
    artifact.reload().map_err(LoadError::artifact)
}

pub fn reload_artifact_with_context(
    artifact: &CompiledArtifact,
    context: &CompileContext,
) -> Result<CompiledQuery, LoadError> {
    artifact
        .reload_with_context(context)
        .map_err(LoadError::artifact)
}

/// Parse-only entry point for tooling.
pub fn parse(source: &str) -> ParseResult {
    let mut parsed = Parser::new(source).parse_module();
    parsed
        .diagnostics
        .extend(validate_module_shape(&parsed.module));
    parsed
}

/// Resolve module import declarations without running full name binding.
pub fn resolve_imports(module: &SurfaceModule, import_policy: &ImportPolicy) -> Vec<Diagnostic> {
    NameResolver::new()
        .resolve_module_imports(module, import_policy)
        .diagnostics
}

/// Run strict or profile-configured static type checks for a parsed module.
pub fn type_check(module: &SurfaceModule, context: &CompileContext) -> Vec<Diagnostic> {
    let mut checker = TypeChecker::with_config(context.type_config.clone());
    checker.seed_runtime_import_surface(module);
    for name in context.policy_bindings.keys() {
        checker.declare_variable(crate::resolve::QNameKey::new(None, name.clone()), Type::Any);
    }
    checker.check_surface_module(module).diagnostics
}

/// Load a compiled binary artifact by content hash.
pub fn load(_hash: ContentHash, _ctx: &LoadContext) -> Result<CompiledQuery, LoadError> {
    Err(LoadError::unsupported(
        "CEM-QL artifact loading is not implemented yet",
    ))
}

#[derive(Debug, Clone)]
pub struct CompileContext {
    pub schema_frame: Option<SchemaFrame>,
    pub overlay: OverlayMap,
    pub import_policy: ImportPolicy,
    pub type_config: TyConfig,
    pub cache_mode: CacheMode,
    pub source_uri: Option<String>,
    pub expected_source_hash: Option<ContentHash>,
    pub diagnostics: Vec<Diagnostic>,
    pub source_map_base: SourceMapStack,
    pub policy_bindings: BTreeMap<String, ItemStream>,
}

impl Default for CompileContext {
    fn default() -> Self {
        Self {
            schema_frame: None,
            overlay: OverlayMap::default(),
            import_policy: ImportPolicy::default(),
            type_config: TyConfig::default(),
            cache_mode: CacheMode::Prod,
            source_uri: None,
            expected_source_hash: None,
            diagnostics: Vec::new(),
            source_map_base: SourceMapStack::default(),
            policy_bindings: BTreeMap::new(),
        }
    }
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

    pub fn artifact(error: crate::artifact::ArtifactLoadError) -> Self {
        Self {
            code: error.code,
            message: error.message,
        }
    }
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for LoadError {}
