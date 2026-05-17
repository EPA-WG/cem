//! Layer 9 — `ImplementationInterpreter` / Transform.
//!
//! Public contract per AC-F-10 / `cem-ml-stack-design-impl.md` §3.10.
//! Tier A target output is light-DOM custom-element markup compatible
//! with `@epa-wg/custom-element`.

use crate::diagnostics::Diagnostic;
use crate::parser::CemAstNode;
use crate::source_map::SourceMapStack;

#[derive(Debug, Clone)]
pub enum OutputTarget {
    LightDomCustomElements,
    CanonicalCemMl,
    DomJson,
}

#[derive(Debug, Clone)]
pub struct TransformContext {
    pub target: OutputTarget,
    pub source: SourceMapStack,
}

#[derive(Debug, Clone)]
pub struct TransformOutput {
    pub target: OutputTarget,
    pub rendered: String,
    pub diagnostics: Vec<Diagnostic>,
    pub source_map: SourceMapStack,
}

/// Boundary for the implementation interpreter / transform layer.
/// The `Interpreter` name in AC-F-10 / todo refers to this trait.
pub trait Interpreter: Send {
    fn transform(&self, nodes: &[CemAstNode], ctx: &TransformContext) -> TransformOutput;
}
