//! Independent, ready selected-forest name metadata over original source owners.
use super::{NamespaceScopeTarget, PendingNamespaceValue};
use crate::{
    parser::{AstNodeId, CemAstNode, ExpandedName},
    schema::{
        declaration_references::SchemaDeclarationNode,
        machine::{LexicalScopeSnapshot, LexicallyScopedDocument},
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceNameCompletionError {
    OwnerMismatch,
    InvalidRoot(AstNodeId),
    OverlappingRoots(AstNodeId),
    InvalidDeclaration(AstNodeId),
    OutsideSelection(AstNodeId),
    InvalidOccurrence(AstNodeId),
    Pending {
        node: AstNodeId,
        declaration: AstNodeId,
    },
    UnboundName(AstNodeId),
}
#[derive(Debug)]
pub struct NamespaceNameCompletion {
    captured: Arc<LexicallyScopedDocument>,
    roots: Vec<AstNodeId>,
    nodes: BTreeSet<AstNodeId>,
    names: BTreeMap<AstNodeId, ExpandedName>,
    targets: BTreeMap<AstNodeId, NamespaceScopeTarget>,
}

/// Execution-only binding completion, retaining the original declaration rather
/// than assigning it an authored ID or rewriting namespace capture records.
#[derive(Debug, Clone)]
pub struct CompletedNamespaceBinding {
    pub declaration: SchemaDeclarationNode,
    pub namespace_uri: String,
}

/// Original scalar lexical metadata plus this execution's ready URI overlays.
/// Original namespace binding records, schema metadata and AST owners stay fixed.
#[derive(Debug, Clone)]
pub struct NamespaceLexicalSnapshot {
    original: LexicalScopeSnapshot,
    completed: BTreeMap<String, CompletedNamespaceBinding>,
}
impl NamespaceLexicalSnapshot {
    pub fn original(&self) -> &LexicalScopeSnapshot {
        &self.original
    }
    pub fn completed_bindings(&self) -> &BTreeMap<String, CompletedNamespaceBinding> {
        &self.completed
    }
    /// Empty default URIs are ready resets; absent prefixes remain unbound.
    pub fn namespace_uri(&self, prefix: &str) -> Option<&str> {
        self.completed
            .get(prefix)
            .map(|binding| binding.namespace_uri.as_str())
            .or_else(|| {
                self.original
                    .namespaces
                    .binding(prefix)
                    .map(|binding| binding.namespace_uri.as_str())
            })
    }
}
impl NamespaceNameCompletion {
    /// The caller supplies results of its explicit bounded/authorized consumer.
    /// This constructor checks selected forest readiness; it creates no grants,
    /// evaluates no slots and changes no original name/binding/source arena.
    pub fn new(
        captured: Arc<LexicallyScopedDocument>,
        roots: &[AstNodeId],
        targets: BTreeMap<AstNodeId, NamespaceScopeTarget>,
    ) -> Result<Self, NamespaceNameCompletionError> {
        for id in targets.keys() {
            if !captured
                .pending_namespace_declaration(captured.document(), *id)
                .is_some_and(|decl| matches!(decl.value, PendingNamespaceValue::Native))
            {
                return Err(NamespaceNameCompletionError::InvalidDeclaration(*id));
            }
        }
        let mut nodes = BTreeSet::new();
        for root in roots {
            if captured.document().get(*root).is_none() {
                return Err(NamespaceNameCompletionError::InvalidRoot(*root));
            }
            let mut pending = vec![*root];
            while let Some(id) = pending.pop() {
                if !nodes.insert(id) {
                    return Err(NamespaceNameCompletionError::OverlappingRoots(id));
                }
                match captured
                    .document()
                    .get(id)
                    .ok_or(NamespaceNameCompletionError::InvalidRoot(id))?
                {
                    CemAstNode::Document { root_children, .. } => pending.extend(root_children),
                    CemAstNode::Element {
                        children,
                        attributes,
                        ..
                    } => {
                        pending.extend(children);
                        pending.extend(attributes);
                    }
                    CemAstNode::Attribute { value_nodes, .. } => pending.extend(value_nodes),
                    _ => {}
                }
            }
        }
        let mut names = BTreeMap::new();
        for id in &nodes {
            if !matches!(
                captured.document().get(*id),
                Some(CemAstNode::Element { .. } | CemAstNode::Attribute { .. })
            ) {
                continue;
            }
            if let Some(name) = captured.expanded_name(captured.document(), *id) {
                names.insert(*id, name.clone());
                continue;
            }
            let Some(name) = captured.pending_namespace_name(captured.document(), *id) else {
                // General attribute expressions have a builder-owned intrinsic
                // `$` name, with no lexical QName event. Admit only the original
                // captured expression wrapper, never arbitrary unbound elements.
                if let Some(CemAstNode::Element { expanded_name, .. }) =
                    captured.document().get(*id)
                {
                    if expanded_name.local_name == "$"
                        && expanded_name.namespace_uri.is_empty()
                        && captured.snapshot(captured.document(), *id).is_some()
                    {
                        names.insert(*id, expanded_name.clone());
                        continue;
                    }
                }
                // Namespace declaration attributes have reserved lexical names,
                // and do not depend on an authored 'xmlns' namespace binding.
                if let Some(CemAstNode::Attribute { expanded_name, .. }) =
                    captured.document().get(*id)
                {
                    if expanded_name.namespace_uri == "xmlns" {
                        names.insert(
                            *id,
                            ExpandedName {
                                namespace_uri: "http://www.w3.org/2000/xmlns/".into(),
                                local_name: expanded_name.local_name.clone(),
                                schema_id: None,
                            },
                        );
                        continue;
                    }
                }
                return Err(NamespaceNameCompletionError::UnboundName(*id));
            };
            let uri = binding_uri(&captured, &targets, name.declaration, *id)?;
            names.insert(
                *id,
                ExpandedName {
                    namespace_uri: uri.into(),
                    local_name: name.local_name.clone(),
                    schema_id: None,
                },
            );
        }
        Ok(Self {
            captured,
            roots: roots.to_vec(),
            nodes,
            names,
            targets,
        })
    }
    pub fn captured(&self) -> &Arc<LexicallyScopedDocument> {
        &self.captured
    }
    pub fn roots(&self) -> &[AstNodeId] {
        &self.roots
    }
    pub fn contains(&self, node: AstNodeId) -> bool {
        self.nodes.contains(&node)
    }
    pub fn expanded_name(&self, node: AstNodeId) -> Option<&ExpandedName> {
        self.names.get(&node)
    }
    pub fn targets(&self) -> &BTreeMap<AstNodeId, NamespaceScopeTarget> {
        &self.targets
    }
    /// Prepare only selected original expression occurrences. Every captured
    /// pending prefix must complete, including prefixes unused by selected QNames.
    /// The property's own selector retains its pre-declaration snapshot. Reading
    /// inherited dependencies outside this forest does not expand query axes.
    pub fn lexical_snapshot(
        &self,
        occurrence: &SchemaDeclarationNode,
    ) -> Result<NamespaceLexicalSnapshot, NamespaceNameCompletionError> {
        let owner = self.captured.document();
        if !Arc::ptr_eq(occurrence.document(), owner) {
            return Err(NamespaceNameCompletionError::OwnerMismatch);
        }
        let node = occurrence.node_id();
        if !self.contains(node) {
            return Err(NamespaceNameCompletionError::OutsideSelection(node));
        }
        let original = self
            .captured
            .snapshot(owner, node)
            .ok_or(NamespaceNameCompletionError::InvalidOccurrence(node))?;
        let mut completed = BTreeMap::new();
        if let Some(bindings) = self.captured.pending_namespace_bindings(owner, node) {
            for (prefix, declaration) in bindings {
                let namespace_uri = binding_uri(&self.captured, &self.targets, *declaration, node)?;
                completed.insert(
                    prefix.clone(),
                    CompletedNamespaceBinding {
                        declaration: SchemaDeclarationNode::new(owner.clone(), *declaration)
                            .ok_or(NamespaceNameCompletionError::InvalidDeclaration(
                                *declaration,
                            ))?,
                        namespace_uri: namespace_uri.into(),
                    },
                );
            }
        }
        Ok(NamespaceLexicalSnapshot {
            original: original.clone(),
            completed,
        })
    }
    /// Read an original binding dependency without creating an expression
    /// context. Declarations outside the selected forest can supply inherited
    /// bindings; this metadata lookup does not expose their nodes through axes.
    /// Ready names do not imply that every unused binding dependency is ready.
    pub fn binding_namespace_uri(
        &self,
        declaration: &SchemaDeclarationNode,
    ) -> Result<&str, NamespaceNameCompletionError> {
        if !Arc::ptr_eq(declaration.document(), self.captured.document()) {
            return Err(NamespaceNameCompletionError::OwnerMismatch);
        }
        binding_uri(
            &self.captured,
            &self.targets,
            declaration.node_id(),
            declaration.node_id(),
        )
    }
}

fn binding_uri<'a>(
    captured: &'a LexicallyScopedDocument,
    targets: &'a BTreeMap<AstNodeId, NamespaceScopeTarget>,
    mut declaration: AstNodeId,
    node: AstNodeId,
) -> Result<&'a str, NamespaceNameCompletionError> {
    let mut visited = BTreeSet::new();
    loop {
        if !visited.insert(declaration) {
            return Err(NamespaceNameCompletionError::InvalidDeclaration(
                declaration,
            ));
        }
        if let Some(binding) = captured.namespace_binding(captured.document(), declaration) {
            return Ok(&binding.namespace_uri);
        }
        let Some(pending) =
            captured.pending_namespace_declaration(captured.document(), declaration)
        else {
            return Err(NamespaceNameCompletionError::InvalidDeclaration(
                declaration,
            ));
        };
        match pending.value {
            PendingNamespaceValue::Alias(alias) => declaration = alias,
            PendingNamespaceValue::Native => {
                return targets
                    .get(&declaration)
                    .map(NamespaceScopeTarget::namespace_uri)
                    .ok_or(NamespaceNameCompletionError::Pending { node, declaration })
            }
        }
    }
}
