//! Structural subtype walk facade.

use super::{SchemaTypeRegistry, Type, TypeLattice};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubtypeChecker<'schema> {
    lattice: TypeLattice<'schema>,
}

impl<'schema> SubtypeChecker<'schema> {
    pub fn new(schemas: &'schema SchemaTypeRegistry) -> Self {
        Self {
            lattice: TypeLattice::new(schemas),
        }
    }

    pub fn is_subtype(&self, actual: &Type, expected: &Type) -> bool {
        self.lattice.is_subtype(actual, expected)
    }
}
