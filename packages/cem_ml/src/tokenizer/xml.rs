//! XML 1.0 parity tokenizer profile (stub).
//!
//! Tier A status: not yet implemented. XML token output uses the same
//! `SchemaToken` shape; the body lands in Phase 11 alongside the HTML
//! profile and the canonical CEM tokenizer (`cem-ml-stack-design-impl.md`
//! §3.2).

use crate::diagnostics::{Diagnostic, Severity};
use crate::source::ByteSource;
use crate::tokenizer::{SchemaToken, SchemaTokenizer, TokenizerProfile};

pub struct XmlTokenizer {
    diagnostics: Vec<Diagnostic>,
}

impl XmlTokenizer {
    pub fn from_source<S: ByteSource>(_source: S) -> Self {
        Self {
            diagnostics: vec![Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: Some(0),
                code: "cem.tokenizer.profile_not_implemented".to_owned(),
                severity: Severity::Error,
                message: "XML 1.0 parity tokenizer is reserved for Phase 11 (see cem-ml-cli-plan.md)."
                    .to_owned(),
                node: None,
            }],
        }
    }

    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }
}

impl SchemaTokenizer for XmlTokenizer {
    fn profile(&self) -> TokenizerProfile {
        TokenizerProfile::Xml
    }
    fn next_token(&mut self) -> Option<SchemaToken> {
        None
    }
}
