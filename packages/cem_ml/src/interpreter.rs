//! Layer 9 — `ImplementationInterpreter` / Transform.
//!
//! Public contract per AC-F-10 / `cem-ml-stack-design-impl.md` §3.10.
//! Tier A target output is light-DOM custom-element markup compatible
//! with `@epa-wg/custom-element`.

pub mod light_dom;

use crate::diagnostics::Diagnostic;
use crate::parser::CemAstNode;
use crate::source::ByteRange;
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
    /// Stack describing the transform as a whole.
    pub source_map: SourceMapStack,
    /// Per-output-span mapping back to source byte ranges.
    /// Each `OutputSpan.output_range` is a byte range inside `rendered`
    /// (UTF-8 bytes); `origin` is the source-map stack that produced it.
    pub output_spans: Vec<OutputSpan>,
}

/// One contiguous run of generated output bytes paired with the source
/// frames they originated from. Tier A emits one span per AST node-derived
/// output chunk (open tag, attribute, text, close tag).
#[derive(Debug, Clone)]
pub struct OutputSpan {
    pub output_range: ByteRange,
    pub origin: SourceMapStack,
}

/// Boundary for the implementation interpreter / transform layer.
/// The `Interpreter` name in AC-F-10 / todo refers to this trait.
pub trait Interpreter: Send {
    fn transform(&self, nodes: &[CemAstNode], ctx: &TransformContext) -> TransformOutput;
}
