//! HTML parity tokenizer profile (stub).
//!
//! Tier A status: not yet implemented. The custom WHATWG-state HTML
//! tokenizer is required so every token carries source-map stacks through
//! decoded streams and nested embedded handoff layers
//! (`cem-ml-stack-design-impl.md` §3.2). Per `cem-ml-cli-plan.md` Phase 11,
//! this profile lands alongside the canonical CEM tokenizer; the boundary
//! type exists so consumers can program against it today.

use crate::diagnostics::{Diagnostic, Severity};
use crate::source::ByteSource;
use crate::tokenizer::{SchemaToken, SchemaTokenizer, TokenizerProfile};

pub struct HtmlTokenizer {
    diagnostics: Vec<Diagnostic>,
}

impl HtmlTokenizer {
    pub fn from_source<S: ByteSource>(_source: S) -> Self {
        Self {
            diagnostics: vec![Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: Some(0),
                code: "cem.tokenizer.profile_not_implemented".to_owned(),
                severity: Severity::Error,
                message:
                    "HTML parity tokenizer is reserved for Phase 11 (see cem-ml-cli-plan.md)."
                        .to_owned(),
                node: None,
                source_map: None,
            }],
        }
    }

    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }
}

impl SchemaTokenizer for HtmlTokenizer {
    fn profile(&self) -> TokenizerProfile {
        TokenizerProfile::Html
    }
    fn next_token(&mut self) -> Option<SchemaToken> {
        None
    }
}
