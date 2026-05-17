//! `CemDocument` — owned container for a typed AST.
//!
//! Layer 6 / Document model per `cem-ml-stack-design-impl.md` §3.8. Stores
//! every `CemAstNode` in a flat arena addressed by `AstNodeId`; element
//! attributes and children reference into the same arena.

use crate::diagnostics::Diagnostic;
use crate::parser::{AstNodeId, CemAstNode, NameSlot};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct CemDocument {
    /// Flat arena. `nodes[i].node_id == i`. Index `0` is reserved for the
    /// root `Document` variant.
    pub nodes: Vec<CemAstNode>,
    /// Maps the value of an `id` attribute to the element node that owns
    /// it. Used by reference resolution and `query::find_by_id`.
    pub id_table: HashMap<String, AstNodeId>,
    /// Unresolved reference slots (e.g. `for=`, `aria-labelledby=`) that
    /// never matched an element with the corresponding id. Tier A emits
    /// these as Warning diagnostics at finalize per AC-P §reference slots.
    pub unresolved_slots: Vec<NameSlot>,
    /// Diagnostics accumulated from every layer below (decoder, tokenizer,
    /// schema machine) plus AST-builder diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

impl CemDocument {
    pub fn root(&self) -> Option<&CemAstNode> {
        self.nodes.first()
    }

    pub fn get(&self, id: AstNodeId) -> Option<&CemAstNode> {
        self.nodes.get(id as usize)
    }

    pub fn iter(&self) -> impl Iterator<Item = &CemAstNode> {
        self.nodes.iter()
    }
}
