//! Layer 3: name resolution shell.

pub mod overlay;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BindingId(pub u32);

#[derive(Debug, Clone, Default)]
pub struct BindingSet {
    pub bindings: Vec<BindingId>,
}

#[derive(Debug, Clone, Default)]
pub struct NameResolver {
    pub bindings: BindingSet,
}
