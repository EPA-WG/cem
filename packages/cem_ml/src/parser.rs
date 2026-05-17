//! Layer 6 — `InputDomAstBuilder` / `InterpreterAstBuilder`.
//!
//! Public contract per AC-F-10 / `cem-ml-stack-design-impl.md` §3.8.
//! `CemAstNode` is the typed AST surface that downstream transforms and
//! interpreters consume.

use crate::source_map::SourceMapStack;

pub type AstNodeId = u32;

#[derive(Debug, Clone)]
pub struct ExpandedName {
    pub namespace_uri: String,
    pub local_name: String,
    pub schema_id: Option<u32>,
}

#[derive(Debug, Clone)]
pub enum CemAstNode {
    Document {
        node_id: AstNodeId,
        root_children: Vec<AstNodeId>,
        source: SourceMapStack,
    },
    Element {
        node_id: AstNodeId,
        expanded_name: ExpandedName,
        annotations: Vec<AstNodeId>,
        children: Vec<AstNodeId>,
        source: SourceMapStack,
    },
    Attribute {
        node_id: AstNodeId,
        expanded_name: ExpandedName,
        value: Option<String>,
        source: SourceMapStack,
    },
    Text {
        node_id: AstNodeId,
        data: String,
        source: SourceMapStack,
    },
    Whitespace {
        node_id: AstNodeId,
        data: String,
        source: SourceMapStack,
    },
    Comment {
        node_id: AstNodeId,
        data: String,
        source: SourceMapStack,
    },
    ProcessingInstruction {
        node_id: AstNodeId,
        target: String,
        data: String,
        source: SourceMapStack,
    },
    Cdata {
        node_id: AstNodeId,
        data: String,
        source: SourceMapStack,
    },
    RawText {
        node_id: AstNodeId,
        data: String,
        source: SourceMapStack,
    },
    Error {
        node_id: AstNodeId,
        code: String,
        source: SourceMapStack,
    },
}

#[derive(Debug, Clone)]
pub struct NameSlot {
    pub owner_scope: u32,
    pub target_name: String,
    pub resolved: Option<AstNodeId>,
    pub source: SourceMapStack,
}

pub trait InputDomAstBuilder: Send {
    fn finish(self) -> Vec<CemAstNode>;
}
