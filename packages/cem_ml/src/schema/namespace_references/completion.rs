//! Independent, ready selected-forest name metadata over original source owners.
use super::{NamespaceScopeTarget, PendingNamespaceValue};
use crate::{
    parser::{AstNodeId, CemAstNode, ExpandedName},
    schema::machine::LexicallyScopedDocument,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceNameCompletionError {
    InvalidRoot(AstNodeId),
    OverlappingRoots(AstNodeId),
    InvalidDeclaration(AstNodeId),
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
            let mut declaration = name.declaration;
            let mut visited = BTreeSet::new();
            let uri = loop {
                if !visited.insert(declaration) {
                    return Err(NamespaceNameCompletionError::InvalidDeclaration(
                        declaration,
                    ));
                }
                if let Some(target) = targets.get(&declaration) {
                    break target.namespace_uri();
                }
                match captured
                    .pending_namespace_declaration(captured.document(), declaration)
                    .map(|decl| &decl.value)
                {
                    Some(PendingNamespaceValue::Alias(alias)) => declaration = *alias,
                    _ => {
                        return Err(NamespaceNameCompletionError::Pending {
                            node: *id,
                            declaration,
                        })
                    }
                }
            };
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
}
