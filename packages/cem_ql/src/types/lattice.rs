//! Query type lattice and structural subtyping.

use super::{NodeKind, SchemaTypeRegistry, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeLattice<'schema> {
    schemas: &'schema SchemaTypeRegistry,
}

impl<'schema> TypeLattice<'schema> {
    pub fn new(schemas: &'schema SchemaTypeRegistry) -> Self {
        Self { schemas }
    }

    pub fn is_subtype(&self, actual: &Type, expected: &Type) -> bool {
        match (actual, expected) {
            (Type::Empty, _) => true,
            (_, Type::Any) => true,
            (Type::Any, _) => false,
            (Type::Node(actual), Type::Node(expected)) => self.node_subtype(actual, expected),
            (Type::SchemaElement(actual), Type::SchemaElement(expected)) => {
                self.schemas.is_structural_subtype(*actual, *expected)
            }
            (Type::SchemaElement(actual), Type::Node(NodeKind::Element(expected_name))) => self
                .schemas
                .get(*actual)
                .is_some_and(|info| &info.element_name == expected_name),
            (Type::SchemaElement(_), Type::Node(NodeKind::Node)) => true,
            (Type::Atom(actual), Type::Atom(expected)) => actual == expected,
            (Type::Record(actual), Type::Record(expected)) => actual == expected,
            (Type::Array(actual), Type::Array(expected))
            | (Type::Stream(actual), Type::Stream(expected)) => {
                self.is_subtype(actual, expected) && self.is_subtype(expected, actual)
            }
            (
                Type::Lambda {
                    params: actual_params,
                    ret: actual_ret,
                },
                Type::Lambda {
                    params: expected_params,
                    ret: expected_ret,
                },
            ) => {
                actual_params.len() == expected_params.len()
                    && actual_params
                        .iter()
                        .zip(expected_params)
                        .all(|(actual, expected)| {
                            self.is_subtype(actual, expected) && self.is_subtype(expected, actual)
                        })
                    && self.is_subtype(actual_ret, expected_ret)
            }
            (
                Type::Resource {
                    content_type: actual_content_type,
                    schema: actual_schema,
                },
                Type::Resource {
                    content_type: expected_content_type,
                    schema: expected_schema,
                },
            ) => actual_content_type == expected_content_type && actual_schema == expected_schema,
            _ => actual == expected,
        }
    }

    fn node_subtype(&self, actual: &NodeKind, expected: &NodeKind) -> bool {
        match (actual, expected) {
            (_, NodeKind::Node) => true,
            (NodeKind::Element(actual), NodeKind::Element(expected))
            | (NodeKind::Attribute(actual), NodeKind::Attribute(expected)) => actual == expected,
            _ => actual == expected,
        }
    }
}
