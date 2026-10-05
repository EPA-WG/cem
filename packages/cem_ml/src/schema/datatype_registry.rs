//! Retained datatype declaration collection, before dependency compilation.
//!
//! The caller supplies the given lexical scope and performs namespace admission
//! and authorized reference selection. Membership supplies neither executable
//! semantics nor a scope-crossing grant, and does not establish readiness.
use super::declaration_references::SchemaDeclarationNode;
use crate::parser::CemAstNode;
use std::collections::BTreeMap;

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
