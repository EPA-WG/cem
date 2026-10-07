//! Passive lexical export. No AST, runtime policy, capability or live context is serialized.
use super::*;
use crate::{
    ast::reload::ReloadError, parser::format::DocumentFormatIdentity, source_map::SourceMapStack,
};

pub const LEXICAL_RELOAD_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LexicalReloadMetadata {
    pub version: u16,
    pub payload_fingerprint: [u8; 32],
    pub occurrences: BTreeMap<AstNodeId, LexicalScopeSnapshot>,
    pub names: BTreeMap<AstNodeId, ExpandedName>,
    pub schema_element_forms: BTreeMap<AstNodeId, SchemaElementForm>,
    pub namespace_bindings: BTreeMap<AstNodeId, NamespaceBinding>,
    pub pending_namespace_declarations: BTreeMap<AstNodeId, PendingNamespaceDeclaration>,
    pub pending_namespace_names: BTreeMap<AstNodeId, PendingNamespaceName>,
    pub pending_namespace_bindings: BTreeMap<AstNodeId, BTreeMap<String, AstNodeId>>,
    pub diagnostics: Vec<Diagnostic>,
    pub format_identity: Option<DocumentFormatIdentity>,
}

impl LexicalReloadMetadata {
    pub(crate) fn export(capture: &LexicallyScopedDocument, payload_fingerprint: [u8; 32]) -> Self {
        Self {
            version: LEXICAL_RELOAD_VERSION,
            payload_fingerprint,
            occurrences: capture.occurrences.clone(),
            names: capture.names.clone(),
            schema_element_forms: capture.schema_element_forms.clone(),
            namespace_bindings: capture.namespace_bindings.clone(),
            pending_namespace_declarations: capture.pending_namespace_declarations.clone(),
            pending_namespace_names: capture.pending_namespace_names.clone(),
            pending_namespace_bindings: capture.pending_namespace_bindings.clone(),
            diagnostics: capture.diagnostics.clone(),
            format_identity: capture.document.format_identity.clone(),
        }
    }

    pub(crate) fn validate(
        &self,
        document: &CemDocument,
        fingerprint: [u8; 32],
    ) -> Result<(), ReloadError> {
        if self.version != LEXICAL_RELOAD_VERSION {
            return Err(ReloadError::UnsupportedVersion);
        }
        if self.payload_fingerprint != fingerprint {
            return Err(ReloadError::FingerprintMismatch);
        }
        let invalid = || ReloadError::InvalidMetadata;
        for (&id, snapshot) in &self.occurrences {
            if !(matches!(document.get(id), Some(CemAstNode::Reference { .. }))
                || matches!(document.get(id), Some(CemAstNode::Element { expanded_name, .. }) if expanded_name.local_name == "$"))
                || !snapshot.namespaces.valid_snapshot()
            {
                return Err(invalid());
            }
            for (name, declaration) in snapshot
                .schema
                .declared_inlines
                .iter()
                .chain(&snapshot.schema.inherited_inlines)
            {
                if name != &declaration.name
                    || declaration.source_node.is_some_and(|node| {
                        !matches!(document.get(node), Some(CemAstNode::Element { .. }))
                    })
                {
                    return Err(invalid());
                }
            }
        }
        // A partial sidecar cannot turn a source reference into a complete lexical handoff.
        for node in &document.nodes {
            if let CemAstNode::Reference { node_id, .. } = node {
                if !self.occurrences.contains_key(node_id) {
                    return Err(invalid());
                }
            }
        }
        for &id in self.names.keys().chain(self.pending_namespace_names.keys()) {
            if !matches!(
                document.get(id),
                Some(CemAstNode::Element { .. } | CemAstNode::Attribute { .. })
            ) {
                return Err(invalid());
            }
        }
        for (&id, form) in &self.schema_element_forms {
            if !matches!(document.get(id), Some(CemAstNode::Element { .. })) {
                return Err(invalid());
            }
            if *form == SchemaElementForm::Prelude
                && !self
                    .names
                    .get(&id)
                    .is_some_and(|n| n.local_name == "@schema")
            {
                return Err(invalid());
            }
        }
        for (&id, binding) in &self.namespace_bindings {
            if !namespace_declaration(document.get(id), &binding.name)
                || binding.binding_id == 0
                || self.pending_namespace_declarations.contains_key(&id)
            {
                return Err(invalid());
            }
        }
        for (&id, declaration) in &self.pending_namespace_declarations {
            if !namespace_declaration(document.get(id), &declaration.prefix) {
                return Err(invalid());
            }
            if let crate::schema::namespace_references::PendingNamespaceValue::Alias(target) =
                declaration.value
            {
                if !self.pending_namespace_declarations.contains_key(&target) {
                    return Err(invalid());
                }
            }
        }
        for (&id, name) in &self.pending_namespace_names {
            if self.names.contains_key(&id)
                || !self
                    .pending_namespace_declarations
                    .contains_key(&name.declaration)
            {
                return Err(invalid());
            }
        }
        for (&id, bindings) in &self.pending_namespace_bindings {
            if !self.occurrences.contains_key(&id) {
                return Err(invalid());
            }
            for (prefix, target) in bindings {
                if !self
                    .pending_namespace_declarations
                    .get(target)
                    .is_some_and(|d| &d.prefix == prefix)
                {
                    return Err(invalid());
                }
            }
        }
        Ok(())
    }

    pub(crate) fn source_maps(&self) -> impl Iterator<Item = &SourceMapStack> {
        self.namespace_bindings
            .values()
            .map(|b| &b.source_map)
            .chain(
                self.occurrences
                    .values()
                    .flat_map(|s| s.namespaces.retained_bindings().map(|b| &b.source_map)),
            )
            .chain(self.occurrences.values().flat_map(|s| {
                s.schema
                    .declared_inlines
                    .values()
                    .chain(s.schema.inherited_inlines.values())
                    .map(|d| &d.source_map)
            }))
    }

    pub(crate) fn restore(&self, document: Arc<CemDocument>) -> LexicallyScopedDocument {
        LexicallyScopedDocument {
            document,
            occurrences: self.occurrences.clone(),
            names: self.names.clone(),
            schema_element_forms: self.schema_element_forms.clone(),
            namespace_bindings: self.namespace_bindings.clone(),
            pending_namespace_declarations: self.pending_namespace_declarations.clone(),
            pending_namespace_names: self.pending_namespace_names.clone(),
            pending_namespace_bindings: self.pending_namespace_bindings.clone(),
            diagnostics: self.diagnostics.clone(),
        }
    }
}

fn namespace_declaration(node: Option<&CemAstNode>, prefix: &str) -> bool {
    match node {
        Some(CemAstNode::Element { expanded_name, .. }) => {
            expanded_name.local_name == "@ns"
                || (expanded_name.local_name == "@default" && prefix.is_empty())
        }
        Some(CemAstNode::Attribute { expanded_name, .. }) => {
            let local = &expanded_name.local_name;
            if local == "xmlns" {
                return prefix.is_empty();
            }
            if let Some(name) = local.strip_prefix("xmlns:") {
                return name == prefix;
            }
            matches!(
                expanded_name.namespace_uri.as_str(),
                "xmlns" | "http://www.w3.org/2000/xmlns/"
            ) && local == prefix
        }
        _ => false,
    }
}
