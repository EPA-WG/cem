//! Layer 5: typed IR shell.

pub mod deserialize;
pub mod lower;
pub mod serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IrId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrNode {
    Module { id: IrId },
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledQuery {
    pub root: IrNode,
}
