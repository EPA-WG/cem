//! Retained datatype declaration collection, before dependency compilation.
//!
//! The caller supplies the given lexical scope and performs namespace admission
//! and authorized reference selection. Membership supplies neither executable
//! semantics nor a scope-crossing grant, and does not establish readiness.
use super::declaration_references::SchemaDeclarationNode;
use crate::parser::CemAstNode;
use std::collections::BTreeMap;

mod source_plan;
pub use source_plan::{
    DatatypeDependency, DatatypeDependencyRole, DatatypeDependencyValue, DatatypeKind,
    DatatypeKindSource, DatatypePlanIssue, DatatypePlanIssueKind, DatatypeSourcePlan,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatatypeRegistration {
    Inserted,
    Reused,
}

/// Retain both source handles for later source-attributed compilation diagnostics.
#[derive(Debug, Clone)]
pub enum DatatypeRegistryError {
    InvalidDeclaration {
        declaration: SchemaDeclarationNode,
    },
    Duplicate {
        name: String,
        scope: SchemaDeclarationNode,
        existing: SchemaDeclarationNode,
        incoming: SchemaDeclarationNode,
    },
}

#[derive(Debug)]
struct ScopeDeclarations {
    // Keep the owner alive while its internal identity is a map key.
    _scope: SchemaDeclarationNode,
    names: BTreeMap<String, SchemaDeclarationNode>,
}

/// Original authored declaration fields and their caller-supplied lexical scope.
/// This is a source descriptor, not an executable/ready datatype contract.
#[derive(Debug, Clone)]
pub struct DatatypeSource {
    scope: SchemaDeclarationNode,
    declaration: SchemaDeclarationNode,
    attributes: Vec<SchemaDeclarationNode>,
}

impl DatatypeSource {
    pub fn scope(&self) -> &SchemaDeclarationNode {
        &self.scope
    }

    pub fn declaration(&self) -> &SchemaDeclarationNode {
        &self.declaration
    }

    /// All authored fields, including unknown fields and repeated occurrences.
    /// The compiler owns namespace/facet admission and unsupported-field errors.
    pub fn attributes(&self) -> &[SchemaDeclarationNode] {
        &self.attributes
    }

    /// Last-authored field view, preserving the original attribute and native
    /// value nodes. No kind inference, dependency resolution or lexical extraction.
    pub fn attribute(&self, name: &str) -> Option<&SchemaDeclarationNode> {
        self.attributes.iter().rev().find(|attribute| {
            matches!(attribute.node(), CemAstNode::Attribute { expanded_name, .. }
                if expanded_name.local_name == name)
        })
    }
}

/// Collection foundation only: entries are original declarations, not compiled
/// datatype descriptors. Unknown names do not acquire a built-in/string fallback.
#[derive(Debug, Default)]
pub struct DatatypeRegistry {
    scopes: BTreeMap<String, ScopeDeclarations>,
    declarations: BTreeMap<String, SchemaDeclarationNode>,
}

impl DatatypeRegistry {
    /// Collect a named declaration in a caller-selected lexical scope. Native
    /// dependencies and registered signatures are checked by the later compiler.
    /// A duplicate error leaves the previous binding and native index unchanged.
    pub fn insert(
        &mut self,
        scope: SchemaDeclarationNode,
        declaration: SchemaDeclarationNode,
    ) -> Result<DatatypeRegistration, DatatypeRegistryError> {
        let Some(name) = declaration_name(&declaration).map(str::to_owned) else {
            return Err(DatatypeRegistryError::InvalidDeclaration { declaration });
        };
        let identity = declaration.identity();
        let entries = self
            .scopes
            .entry(scope.identity())
            .or_insert_with(|| ScopeDeclarations {
                _scope: scope.clone(),
                names: BTreeMap::new(),
            });
        if let Some(existing) = entries.names.get(&name) {
            if existing.identity() == identity {
                return Ok(DatatypeRegistration::Reused);
            }
            return Err(DatatypeRegistryError::Duplicate {
                name,
                scope,
                existing: existing.clone(),
                incoming: declaration,
            });
        }
        entries.names.insert(name, declaration.clone());
        self.declarations.entry(identity).or_insert(declaration);
        Ok(DatatypeRegistration::Inserted)
    }

    /// Local-name collection lookup. QName alias/export binding remains the
    /// compiler's responsibility; this method does not resolve strings as URLs.
    pub fn get(&self, scope: &SchemaDeclarationNode, name: &str) -> Option<&SchemaDeclarationNode> {
        self.scopes.get(&scope.identity())?.names.get(name)
    }

    /// Retain source fields for a collected declaration without claiming that its
    /// dependencies or executable capabilities have been compiled. Use the given
    /// lexical scope; never infer it from the consuming attribute's aliases.
    pub fn source(&self, scope: &SchemaDeclarationNode, name: &str) -> Option<DatatypeSource> {
        let declaration = self.get(scope, name)?.clone();
        let CemAstNode::Element { attributes, .. } = declaration.node() else {
            return None;
        };
        let attributes = attributes
            .iter()
            .map(|id| SchemaDeclarationNode::new(declaration.document().clone(), *id))
            .collect::<Option<Vec<_>>>()?;
        Some(DatatypeSource {
            scope: scope.clone(),
            declaration,
            attributes,
        })
    }

    /// Exact original-source lookup after authorized native selection. A matching
    /// local name or arena node ID in a different owner is not the same declaration.
    pub fn get_by_declaration(
        &self,
        declaration: &SchemaDeclarationNode,
    ) -> Option<&SchemaDeclarationNode> {
        self.declarations.get(&declaration.identity())
    }
}

fn declaration_name(declaration: &SchemaDeclarationNode) -> Option<&str> {
    let CemAstNode::Element {
        expanded_name,
        attributes,
        ..
    } = declaration.node()
    else {
        return None;
    };
    if expanded_name.local_name != "type" {
        return None;
    }
    // Preserve the existing scalar declaration field's last-authored precedence.
    let name = attributes
        .iter()
        .rev()
        .find_map(|id| match declaration.document().get(*id) {
            Some(node @ CemAstNode::Attribute { expanded_name, .. })
                if expanded_name.local_name == "name" =>
            {
                Some(node)
            }
            _ => None,
        })?;
    let CemAstNode::Attribute {
        value, value_nodes, ..
    } = name
    else {
        return None;
    };
    if !value_nodes.is_empty() {
        return None;
    }
    value
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
}
