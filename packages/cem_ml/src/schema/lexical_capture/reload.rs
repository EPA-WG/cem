//! Passive lexical export. No AST, runtime policy, capability or live context is serialized.
use super::*;
use crate::{
    ast::reload::ReloadError, parser::format::DocumentFormatIdentity, source_map::SourceMapStack,
};

pub const LEXICAL_RELOAD_VERSION: u16 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LexicalReloadMetadata {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub typed_preludes: BTreeMap<AstNodeId, TypedPreludeSlot>,
    pub version: u16,
    pub payload_fingerprint: [u8; 32],
    pub occurrences: BTreeMap<AstNodeId, LexicalScopeSnapshot>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attribute_namespaces: BTreeMap<AstNodeId, AttributeNamespaceSnapshot>,
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
            typed_preludes: capture.typed_preludes.clone(),
            version: LEXICAL_RELOAD_VERSION,
            payload_fingerprint,
            occurrences: capture.occurrences.clone(),
            attribute_namespaces: capture.attribute_namespaces.clone(),
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
        if !(1..=LEXICAL_RELOAD_VERSION).contains(&self.version)
            || (self.version < 2
                && (!self.typed_preludes.is_empty() || !document.typed_preludes.is_empty()))
        {
            return Err(ReloadError::UnsupportedVersion);
        }
        if self.payload_fingerprint != fingerprint {
            return Err(ReloadError::FingerprintMismatch);
        }
        let invalid = || ReloadError::InvalidMetadata;
        if self.format_identity.as_ref().is_some_and(|identity| {
            identity.format_id != "cem-ml"
                || identity.content_type != "text/cem-ml"
                || !matches!(
                    identity.format_version,
                    crate::schema::ir::SemVer {
                        major: 1,
                        minor: 0 | 1,
                        patch: 0,
                        prerelease: None,
                        ..
                    }
                )
        }) {
            return Err(ReloadError::UnsupportedVersion);
        }
        crate::schema::prelude_values::validate_document_slots(document).map_err(|_| invalid())?;
        if self.typed_preludes.len() != document.typed_preludes.len()
            || ((self.version >= 2 || document.format_identity.is_some())
                && self.format_identity != document.format_identity)
        {
            return Err(invalid());
        }
        for (&id, slot) in &self.typed_preludes {
            if slot.directive != id
                || document.typed_preludes.get(&id) != Some(&slot.syntax)
                || slot.required_version != crate::schema::ir::SemVer::new(1, 1, 0)
                || slot.form != SchemaElementForm::Prelude
                || slot.extent != crate::schema::scope_controls::SchemaScopeControlExtent::Following
                || crate::schema::prelude_values::validate_source_slot(document, id, &slot.syntax)
                    .ok()
                    != Some(slot.value)
                || !self.occurrences.contains_key(&slot.value)
                || rmp_serde::to_vec_named(&slot.preceding).ok()
                    != self
                        .occurrences
                        .get(&slot.value)
                        .and_then(|s| rmp_serde::to_vec_named(s).ok())
            {
                return Err(invalid());
            }
        }
        for (&id, slot) in &self.typed_preludes {
            use crate::schema::namespace_references::PendingNamespaceValue;
            use crate::tokenizer::cem::TypedPreludeRole;
            if slot.syntax.role != TypedPreludeRole::SchemaSelector
                && !self
                    .pending_namespace_declarations
                    .get(&id)
                    .is_some_and(|declaration| {
                        Some(&declaration.prefix) == slot.syntax.prefix.as_ref()
                            && matches!(declaration.value, PendingNamespaceValue::Native)
                    })
            {
                return Err(invalid());
            }
        }
        for (&id, snapshot) in &self.occurrences {
            if let crate::schema::scoping::SchemaSource::PendingPrelude {
                directive,
                value_range,
            } = &snapshot.schema.active
            {
                if !directive
                    .and_then(|id| self.typed_preludes.get(&id))
                    .is_some_and(|slot| {
                        slot.syntax.role == crate::tokenizer::cem::TypedPreludeRole::SchemaSelector
                            && slot.syntax.value_range == *value_range
                    })
                {
                    return Err(invalid());
                }
            }
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
        for (&id, snapshot) in &self.attribute_namespaces {
            if !matches!(document.get(id), Some(CemAstNode::Attribute { .. }))
                || !snapshot.namespaces.valid_snapshot()
            {
                return Err(invalid());
            }
            for (prefix, target) in &snapshot.pending {
                if !self
                    .pending_namespace_declarations
                    .get(target)
                    .is_some_and(|d| &d.prefix == prefix)
                    || snapshot.namespaces.binding(prefix).is_some()
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
            .chain(
                self.attribute_namespaces
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
            attribute_namespaces: self.attribute_namespaces.clone(),
            names: self.names.clone(),
            schema_element_forms: self.schema_element_forms.clone(),
            namespace_bindings: self.namespace_bindings.clone(),
            pending_namespace_declarations: self.pending_namespace_declarations.clone(),
            pending_namespace_names: self.pending_namespace_names.clone(),
            pending_namespace_bindings: self.pending_namespace_bindings.clone(),
            diagnostics: self.diagnostics.clone(),
            typed_preludes: self.typed_preludes.clone(),
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
