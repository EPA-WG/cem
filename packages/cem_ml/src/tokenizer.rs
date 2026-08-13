//! Layer 2 — `SchemaTokenizer`.
//!
//! Public contract per AC-F-10 / `cem-ml-stack-design-impl.md` §3.2.
//! Tier A defines three profiles (canonical CEM curly, WHATWG HTML, XML 1.0)
//! that all emit the same `SchemaToken` shape.

pub mod cem;
pub mod html;
pub mod xml;

use crate::source::ByteRange;
use crate::source_map::SourceMapStack;

pub(crate) struct TokenizerControl {
    safe_points: crate::operation_control::SafePointPoller,
    failure: Option<crate::operation_control::ControlError>,
}

impl TokenizerControl {
    pub(crate) fn new(
        control: crate::operation_control::OperationControl,
        scope: crate::operation_control::ExecutionScopeId,
    ) -> Self {
        Self {
            safe_points: crate::operation_control::SafePointPoller::new(control, scope),
            failure: None,
        }
    }

    pub(crate) fn force(&mut self) -> bool {
        if self.failed() {
            return false;
        }
        let result = self.safe_points.force();
        self.accept(result)
    }

    pub(crate) fn poll(&mut self) -> bool {
        if self.failed() {
            return false;
        }
        let result = self.safe_points.poll_one();
        self.accept(result)
    }

    fn accept(&mut self, result: Result<(), crate::operation_control::ControlError>) -> bool {
        match result {
            Ok(()) => true,
            Err(error) => {
                self.failure.get_or_insert(error);
                false
            }
        }
    }

    pub(crate) fn failed(&self) -> bool {
        self.failure.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenizerProfile {
    Cem,
    Html,
    Xml,
}

/// Profile-agnostic token kind. The richer per-profile token payloads
/// (`CemToken`, `HtmlToken`, `XmlToken` in the design doc) lower into this
/// shape before reaching Layer 3.
#[derive(Debug, Clone)]
pub enum SchemaTokenKind {
    NodeStart {
        name: String,
    },
    NodeEnd {
        name: Option<String>,
    },
    Attribute {
        name: String,
        value: Option<String>,
        name_range: ByteRange,
        value_range: Option<ByteRange>,
    },
    Text(String),
    Trivia(String),
    Comment(String),
    ProcessingInstruction {
        target: String,
        data: String,
    },
    ExpressionNode(String),
    AnonymousScopeStart,
    Directive {
        name: String,
        data: String,
    },
    RichContent {
        data: String,
    },
    Error {
        code: String,
    },
}

#[derive(Debug, Clone)]
pub struct SchemaToken {
    pub kind: SchemaTokenKind,
    pub byte_range: ByteRange,
    pub profile: TokenizerProfile,
    pub source_map: SourceMapStack,
}

pub trait SchemaTokenizer: Send {
    fn profile(&self) -> TokenizerProfile;
    fn next_token(&mut self) -> Option<SchemaToken>;
}
