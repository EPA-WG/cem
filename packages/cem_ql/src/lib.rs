//! CEM-QL compiler and evaluator crate.
//!
//! This crate owns the query-language layers described in
//! `docs/cem-ql-stack-design-impl.md` §3. The initial surface fixes the
//! public module and type names so downstream work can build against a
//! stable contract while layer implementations land incrementally.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod api;
pub mod artifact;
pub mod diagnostics;
pub mod eval;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod resolve;
pub mod stdlib;
pub mod types;

#[cfg(test)]
mod tests {
    use super::*;

    fn _accept<T>() {}

    #[test]
    fn version_matches_cargo() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn layered_runtime_contract_types_are_importable() {
        use crate::api::{
            compile, evaluate, load, parse, CompileContext, CompileError, EvaluationContext,
            LoadContext, LoadError, ParseResult,
        };
        use crate::artifact::{CompiledArtifact, QueryArtifactFormat};
        use crate::diagnostics::{DiagnosticCode, QueryDiagnostic};
        use crate::eval::{Evaluator, Item, ItemStream, QueryContextScope};
        use crate::ir::{CompiledQuery, IrId, IrNode};
        use crate::lexer::{Lexer, Token, TokenKind};
        use crate::parser::{
            Axis, BinaryOp, Expression, FunctionDecl, FunctionParam, ImportDecl, LiteralValue,
            ModuleDecl, NameTest, ParseError, Parser, PathStep, PipelineStep, QName,
            QuantifierKind, RecordEntry, SetOp, SurfaceModule, SurfaceNode, TypeExpr, UnaryOp,
            VariableDecl,
        };
        use crate::resolve::{
            Arity, BindingEntry, BindingId, BindingKind, BindingSet, BindingTable, FunctionKey,
            ImportKind, ImportPolicy, ImportResolution, ModuleUri, NameResolver,
            OverlayFingerprint, OverlayKey, OverlayMap, QNameKey, Resolution, ResolutionReport,
            ResolutionTraceEvent, SchemaTypeId, StateSlotId, StdlibOverlay, TemplateRefId,
        };
        use crate::stdlib::ModuleRegistry;
        use crate::types::{Type, TypeChecker};

        _accept::<CompileContext>();
        _accept::<CompileError>();
        _accept::<EvaluationContext>();
        _accept::<LoadContext>();
        _accept::<LoadError>();
        _accept::<ParseResult>();
        _accept::<CompiledArtifact>();
        _accept::<QueryArtifactFormat>();
        _accept::<DiagnosticCode>();
        _accept::<QueryDiagnostic>();
        _accept::<Evaluator>();
        _accept::<Item>();
        _accept::<ItemStream>();
        _accept::<QueryContextScope>();
        _accept::<CompiledQuery>();
        _accept::<IrId>();
        _accept::<IrNode>();
        _accept::<Lexer<'static>>();
        _accept::<Token>();
        _accept::<TokenKind>();
        _accept::<ParseError>();
        _accept::<Parser<'static>>();
        _accept::<SurfaceModule>();
        _accept::<SurfaceNode>();
        _accept::<ModuleDecl>();
        _accept::<ImportDecl>();
        _accept::<VariableDecl>();
        _accept::<FunctionDecl>();
        _accept::<FunctionParam>();
        _accept::<Expression>();
        _accept::<RecordEntry>();
        _accept::<LiteralValue>();
        _accept::<PathStep>();
        _accept::<PipelineStep>();
        _accept::<SetOp>();
        _accept::<BinaryOp>();
        _accept::<UnaryOp>();
        _accept::<QuantifierKind>();
        _accept::<QName>();
        _accept::<NameTest>();
        _accept::<Axis>();
        _accept::<TypeExpr>();
        _accept::<BindingId>();
        _accept::<Arity>();
        _accept::<SchemaTypeId>();
        _accept::<TemplateRefId>();
        _accept::<StateSlotId>();
        _accept::<QNameKey>();
        _accept::<FunctionKey>();
        _accept::<BindingKind>();
        _accept::<BindingEntry>();
        _accept::<BindingTable>();
        _accept::<BindingSet>();
        _accept::<NameResolver>();
        _accept::<ResolutionTraceEvent>();
        _accept::<Resolution>();
        _accept::<ResolutionReport>();
        _accept::<ImportPolicy>();
        _accept::<ImportResolution>();
        _accept::<ImportKind>();
        _accept::<ModuleUri>();
        _accept::<OverlayFingerprint>();
        _accept::<OverlayKey>();
        _accept::<OverlayMap>();
        _accept::<StdlibOverlay>();
        _accept::<ModuleRegistry>();
        _accept::<Type>();
        _accept::<TypeChecker>();

        let _compile: fn(&str, &CompileContext) -> Result<CompiledQuery, CompileError> = compile;
        let _evaluate: fn(&CompiledQuery, &EvaluationContext) -> ItemStream = evaluate;
        let _parse: fn(&str) -> ParseResult = parse;
        let _load: fn(
            cem_ml::schema::compiler::ContentHash,
            &LoadContext,
        ) -> Result<CompiledQuery, LoadError> = load;
    }
}
