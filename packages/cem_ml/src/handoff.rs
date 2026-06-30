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
    "application/yaml",
    "application/x-yaml",
    "text/yaml",
    "text/x-yaml",
    "text/csv",
    "text/markdown",
    "application/xhtml+xml",
    "image/svg+xml",
    "application/mathml+xml",
    "application/mathml-presentation+xml",
    "application/mathml-content+xml",
    "application/xslt+xml",
    "text/xsl",
    "custom-element-xslt",
    "text/custom-element-xslt",
    "application/custom-element-xslt",
    "text/x-custom-element-xslt",
    "text/xml",
    "application/xml",
    "application/xml-external-parsed-entity",
    "text/xml-external-parsed-entity",
    "application/xml-dtd",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_content_types_are_supported_opaque_handoffs() {
        for content_type in [
            "application/yaml",
            "application/x-yaml",
            "text/yaml",
            "text/x-yaml",
        ] {
            assert!(is_supported_content_type(content_type));
        }
    }

    #[test]
    fn csv_content_type_is_supported_opaque_handoff() {
        assert!(is_supported_content_type("text/csv"));
    }

    #[test]
    fn markdown_content_type_is_supported_opaque_handoff() {
        assert!(is_supported_content_type("text/markdown"));
    }

    #[test]
    fn html_content_type_is_supported_opaque_handoff() {
        assert!(is_supported_content_type("text/html"));
    }

    #[test]
    fn css_content_type_is_supported_opaque_handoff() {
        assert!(is_supported_content_type("text/css"));
    }

    #[test]
    fn xhtml_content_type_is_supported_opaque_handoff() {
        assert!(is_supported_content_type("application/xhtml+xml"));
    }

    #[test]
    fn svg_content_type_is_supported_opaque_handoff() {
        assert!(is_supported_content_type("image/svg+xml"));
    }

    #[test]
    fn mathml_content_types_are_supported_opaque_handoffs() {
        for content_type in [
            "application/mathml+xml",
            "application/mathml-presentation+xml",
            "application/mathml-content+xml",
        ] {
            assert!(is_supported_content_type(content_type));
        }
    }

    #[test]
    fn xslt_content_types_are_supported_opaque_handoffs() {
        for content_type in [
            "application/xslt+xml",
            "text/xsl",
            "custom-element-xslt",
            "text/custom-element-xslt",
            "application/custom-element-xslt",
            "text/x-custom-element-xslt",
        ] {
            assert!(is_supported_content_type(content_type));
        }
    }

    #[test]
    fn xml_content_types_are_supported_opaque_handoffs() {
        for content_type in [
            "application/xml",
            "text/xml",
            "application/xml-external-parsed-entity",
            "text/xml-external-parsed-entity",
            "application/xml-dtd",
        ] {
            assert!(is_supported_content_type(content_type));
        }
    }
}
