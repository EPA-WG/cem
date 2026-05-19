//! Layer 6 — `InputDomAstBuilder` / `InterpreterAstBuilder`.
//!
//! Public contract per AC-F-10 / `cem-ml-stack-design-impl.md` §3.8.
//! `CemAstNode` is the typed AST surface that downstream transforms and
//! interpreters consume.

pub mod builder;
pub mod document;

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
        /// All attributes on this element, including CEM annotations
        /// (attributes in the `cem:` namespace). Use
        /// `query::cem_annotations` to filter for the annotation subset.
        attributes: Vec<AstNodeId>,
        children: Vec<AstNodeId>,
        /// `true` when the source carries an explicit `|` (or `▷`)
        /// content boundary between the attribute list and the content
        /// plane; `false` when the relaxed form was used (content
        /// starts at the first non-attribute token). Drives the
        /// `cem.lint.relaxed_content_boundary` rule in
        /// `validation::rules`. Producers that don't have the
        /// information (binary decode, programmatic construction)
        /// default to `true` — canonical CEM-ML inserts `|`.
        has_explicit_boundary: bool,
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
