//! Cross-cutting source-map contract per AC-P-7 and `cem-ml-stack-design-impl.md` §2.

use crate::source::{ByteRange, SourceId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TransformKind {
    HtmlTokenizer,
    XmlTokenizer,
    CemTokenizer,
    EventNormalizer,
    SchemaValidation { schema_id: u32 },
    CemAstBuilder,
    Query,
    QueryStep,
    HandoffBoundary { child_content_type: String },
    ContentTypeTransform { content_type: String },
    InterpreterRender,
    /// Host → cem-ql embedding boundary per AC-T-7. `host` is the byte
    /// range the host parser owned (whole attribute value, `{...}` AVT
    /// span, or `{$ ... }` expression-node body); the next frame the
    /// cem-ql parser pushes carries the sub-span inside that range.
    TemplateEmbedding { host: ByteRange },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "ranges")]
pub enum FrameSpan {
    Single(ByteRange),
    Multi(Vec<ByteRange>),
}

/// One frame of the origin-first source-map stack. `byte_range` (via
/// `FrameSpan`) is the durable location identity; `line`/`column` are
/// projections derived on demand from a `LineIndex` and never stored here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMapFrame {
    pub source_id: SourceId,
    pub span: FrameSpan,
    pub transform: TransformKind,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceMapStack {
    /// Ordered origin-first; the current frame is last (AC-P-7).
    pub frames: Vec<SourceMapFrame>,
}

impl SourceMapStack {
    pub fn push(&mut self, frame: SourceMapFrame) {
        self.frames.push(frame);
    }

    pub fn origin(&self) -> Option<&SourceMapFrame> {
        self.frames.first()
    }

    pub fn current(&self) -> Option<&SourceMapFrame> {
        self.frames.last()
    }
}
