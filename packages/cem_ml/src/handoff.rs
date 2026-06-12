//! Layer 5 — Scoped Embedded Handoff Stack.
//!
//! Public contract per AC-F-10 / `cem-ml-stack-design.md` §9. The handoff
//! record carries the parent-owned return condition; the child parser is
//! forbidden from consuming past it.

use crate::events::{HandoffRecord, ReturnCondition};
use crate::source::ByteRange;

/// Tier A content types for which a child-parser body lands in Phase 11.
/// When seen at a handoff boundary, the schema machine emits an Info
/// diagnostic and preserves the region as opaque text.
pub const SUPPORTED_CONTENT_TYPES: &[&str] = &[
    "text/html",
    "text/css",
    "text/javascript",
    "application/json",
    "text/xml",
    "application/xml",
];

pub fn is_supported_content_type(ct: &str) -> bool {
    SUPPORTED_CONTENT_TYPES.contains(&ct)
}

#[derive(Debug, Default)]
pub struct HandoffStack {
    frames: Vec<HandoffRecord>,
}

impl HandoffStack {
    pub fn push(&mut self, record: HandoffRecord) {
        self.frames.push(record);
    }

    pub fn pop(&mut self) -> Option<HandoffRecord> {
        self.frames.pop()
    }

    pub fn top(&self) -> Option<&HandoffRecord> {
        self.frames.last()
    }

    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Tier A safety check: returns true when `offset` is inside the top
    /// handoff's parent-owned bounds. The child parser must stop consuming
    /// scalars at or before this offset.
    pub fn within_bounds(&self, offset: u64) -> bool {
        match self.top() {
            None => true,
            Some(top) => match top.inherited_context.parent_close_byte_offset {
                None => true,
                Some(close) => offset < close,
            },
        }
    }
}

/// Construct the Tier A canonical return condition for a `@type="..."`
/// anonymous-scope handoff. The child parser stops at the parent scope's
/// closing brace.
pub fn anonymous_scope_return_condition() -> ReturnCondition {
    ReturnCondition::ParentScopeClose
}

#[allow(dead_code)]
fn _byte_range_referenced(_: ByteRange) {}
