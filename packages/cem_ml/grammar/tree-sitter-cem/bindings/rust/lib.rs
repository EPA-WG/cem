//! Rust binding for the canonical CEM-ML tree-sitter grammar.
//!
//! Builds `src/parser.c` (regenerated from `grammar.js` via the
//! `tree-sitter generate` CLI) and exposes a single
//! [`LANGUAGE`](LANGUAGE) handle that downstream tests use to validate
//! tree-sitter parity against the Rust tokenizer / AST builder.

use tree_sitter_language::LanguageFn;

extern "C" {
    fn tree_sitter_cem() -> *const ();
}

/// Tree-sitter `LanguageFn` for the CEM-ML grammar.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_cem) };
