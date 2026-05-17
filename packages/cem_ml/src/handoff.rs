//! Layer 5 — Scoped Embedded Handoff Stack.
//!
//! Public contract per AC-F-10 / `cem-ml-stack-design.md` §9. A child parser
//! cannot consume past the parent-owned return condition.

use crate::events::HandoffRecord;

#[derive(Debug, Clone)]
pub struct InheritedContext {
    pub schema_id: Option<u32>,
    pub namespace_uri: Option<String>,
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
}
