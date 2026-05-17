//! Layer 3 — `EventNormalizer`.
//!
//! Public contract per AC-F-10 / `cem-ml-stack-design-impl.md` §3.3.
//! Every tokenizer profile lowers into this shared event stream so layers
//! above don't see syntax-flavor-specific token shapes.

pub mod cem;

use crate::source::ByteRange;
use crate::source_map::SourceMapStack;

#[derive(Debug, Clone)]
pub struct QName {
    pub lexical_name: String,
    pub prefix: Option<String>,
    pub local_name: String,
    pub source_range: ByteRange,
}

#[derive(Debug, Clone)]
pub enum ScalarValue {
    Text(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriviaKind {
    Whitespace,
    Comment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeparatorKind {
    ElementBoundary,
    Comma,
    Colon,
    Delimiter,
    Newline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Synthesis {
    Real,
    SelfClosing,
    VoidElement,
    ImpliedByStartTag,
    ImpliedByAncestorClose,
    ImpliedByEof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnCondition {
    /// The child parser must stop at the parent scope's closing brace.
    /// This is the Tier A default for `@type="..."` anonymous-scope handoffs.
    ParentScopeClose,
    /// The child parser must stop at a matching close-tag (e.g. HTML
    /// `</style>` for the `style` element handoff).
    MatchingCloseTag(String),
    /// The child parser runs to end-of-stream (used only for top-level
    /// fragments).
    EndOfStream,
}

#[derive(Debug, Clone)]
pub struct InheritedContext {
    pub schema_id: Option<u32>,
    pub namespace_uri: Option<String>,
    /// Byte offset of the parent scope's expected close, if known. The
    /// child parser is forbidden from consuming past this offset.
    pub parent_close_byte_offset: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct HandoffRecord {
    pub content_type: String,
    pub schema_id: Option<u32>,
    pub source_span: ByteRange,
    pub inherited_context: InheritedContext,
    pub return_condition: ReturnCondition,
}

#[derive(Debug, Clone)]
pub enum NormalizedEvent {
    OpenScope {
        name: QName,
        byte_range: ByteRange,
        source_map: SourceMapStack,
    },
    CloseScope {
        name: QName,
        byte_range: ByteRange,
        synthesis: Synthesis,
        source_map: SourceMapStack,
    },
    Name {
        name: QName,
        byte_range: ByteRange,
    },
    Value {
        value: ScalarValue,
        byte_range: ByteRange,
    },
    Trivia {
        kind: TriviaKind,
        byte_range: ByteRange,
    },
    ProcessingInstruction {
        target: String,
        data: String,
        byte_range: ByteRange,
    },
    Separator {
        kind: SeparatorKind,
        byte_range: ByteRange,
    },
    ModeSwitch {
        content_type: String,
        handoff: HandoffRecord,
    },
    Error {
        code: String,
        byte_range: ByteRange,
        severity: crate::diagnostics::Severity,
    },
}

pub trait EventNormalizer: Send {
    fn next_event(&mut self) -> Option<NormalizedEvent>;
}
