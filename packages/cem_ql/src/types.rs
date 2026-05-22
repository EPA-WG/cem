//! Layer 4: type checker shell.

pub mod lattice;
pub mod subtype;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Item,
    Node,
    Atomic,
    Boolean,
    Number,
    String,
    EmptySequence,
    Unknown,
}

#[derive(Debug, Clone, Default)]
pub struct TypeChecker;
