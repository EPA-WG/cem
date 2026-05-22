//! Public CEM-QL entry points.

use cem_ml::diagnostics::Diagnostic;
use cem_ml::scheduler::ScopePolicy;
use cem_ml::schema::compiler::ContentHash;
use cem_ml::schema::SchemaFrame;
use cem_ml::source_map::SourceMapStack;

use crate::eval::{ItemStream, QueryContextScope};
use crate::ir::CompiledQuery;
use crate::parser::{Parser, SurfaceModule};
use crate::resolve::overlay::OverlayMap;

/// Compile a CEM-QL query module source string into a typed IR.
pub fn compile(_source: &str, _context: &CompileContext) -> Result<CompiledQuery, CompileError> {
    Err(CompileError::unsupported(
        "CEM-QL compilation is not implemented yet",
    ))
}

/// Evaluate a compiled query against a query context scope.
pub fn evaluate(_query: &CompiledQuery, _ctx: &EvaluationContext) -> ItemStream {
    ItemStream::empty()
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
