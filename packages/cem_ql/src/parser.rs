//! Layer 2: surface parser shell.

pub mod module;
pub mod pratt;

use cem_ml::diagnostics::Diagnostic;

use crate::api::ParseResult;

#[derive(Debug, Clone)]
pub struct Parser<'src> {
    pub source: &'src str,
}

impl<'src> Parser<'src> {
    pub fn new(source: &'src str) -> Self {
        Self { source }
    }

    pub fn parse_module(self) -> ParseResult {
        ParseResult {
            module: SurfaceModule {
                source: self.source.to_owned(),
            },
            diagnostics: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceModule {
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub diagnostic: Diagnostic,
}
