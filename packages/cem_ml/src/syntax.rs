//! Generic lossless source-syntax AST stream.
//!
//! This is the runtime boundary consumed by both lifecycle formatters and
//! browser/Node presentation. Callers receive one flattened stream with
//! explicit content-type handoffs instead of selecting a highlighter by host
//! environment. The host JSON adapter is only another consumer of this stream.

pub use crate::api::source_highlight::{
    source_syntax_ast_stream, SourceSyntaxAstStream, SourceSyntaxTokenAst,
};
