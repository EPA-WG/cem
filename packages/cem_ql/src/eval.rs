//! Layer 6: evaluator shell.

pub mod pipeline;
pub mod set_ops;
pub mod types_runtime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QueryContextScope(pub u32);

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Node(String),
    Atomic(String),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemStream {
    pub items: Vec<Item>,
}

impl ItemStream {
    pub fn empty() -> Self {
        Self { items: Vec::new() }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Evaluator;
