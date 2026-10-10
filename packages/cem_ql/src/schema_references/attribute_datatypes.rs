//! Exact original-slot bindings supplied by the lexical lifecycle owner.
use super::*;
impl CemQlSchemaDeclarationHost {
    /// Record an already-resolved literal QName at its original source position.
    /// This does not grant crossings or choose a datatype implementation.
    pub fn bind_literal_attribute_type(
        &mut self,
        slot: SchemaDeclarationNode,
        target: SchemaDeclarationNode,
    ) -> Result<(), &'static str> {
        if !matches!(slot.node(), CemAstNode::Attribute { value_nodes, value: Some(_), .. } if value_nodes.is_empty())
        {
            return Err("literal-type-slot-required");
        }
        if self.source_tree(&slot).is_none() || self.source_tree(&target).is_none() {
            return Err("unregistered-datatype-owner");
        }
        let key = slot.identity();
        if self
            .attribute_datatype_literals
            .get(&key)
            .is_some_and(|(_, old)| old.identity() != target.identity())
        {
            return Err("conflicting-attribute-type-binding");
        }
        self.attribute_datatype_literals.insert(key, (slot, target));
        Ok(())
    }
}
impl crate::attribute_datatypes::AttributeDatatypeHost for CemQlSchemaDeclarationHost {
    fn lookup_attribute_type(
        &mut self,
        slot: &SchemaDeclarationNode,
        qname: &str,
    ) -> ReferenceLinkEvaluation<Self::Node> {
        if let Some(catalog) = &self.datatype_names {
            let Some(schema) = self.declaration_schema(slot) else {
                return ReferenceLinkEvaluation::Pending(
                    "attribute-type-schema-unavailable".into(),
                );
            };
            // An installed catalog is authoritative; unavailable names cannot fall
            // back to an older explicit binding or a same-named built-in.
            let (namespace, name) = if let Some((prefix, name)) = qname.split_once(':') {
                let Some(captured) = self
                    .captured_namespaces
                    .get(&(Arc::as_ptr(slot.document()) as usize))
                else {
                    return ReferenceLinkEvaluation::Pending(
                        "attribute-type-namespace-context-unavailable".into(),
                    );
                };
                let lexical = match self.datatype_captured_prefix(captured, slot, prefix) {
                    Ok(value) => value,
                    Err(reason) => return ReferenceLinkEvaluation::Pending(reason.into()),
                };
                let declared = catalog
                    .schema_aliases
                    .get(&schema.identity())
                    .and_then(|uses| uses.get(prefix));
                let uri = match (lexical, declared) {
                    (Some(Some(uri)), Some(declared)) if &uri != declared => {
                        return ReferenceLinkEvaluation::Invalid(vec![cem_ml::diagnostics::Diagnostic {
                            code: "cem.schema_definition.attribute_type_alias".into(),
                            severity: cem_ml::diagnostics::Severity::Error,
                            message: "Attribute type prefix conflicts with its declaring schema alias".into(),
                            node: Some(slot.identity()),
                            source_map: match slot.node() { CemAstNode::Attribute {source, ..} => Some(source.clone()), _ => None },
                            ..Default::default()
                        }])
                    }
                    (Some(Some(uri)), _) => uri,
                    (Some(None), _) => {
                        return ReferenceLinkEvaluation::Pending(
                            "attribute-type-prefix-pending".into(),
                        )
                    }
                    (None, Some(uri)) => uri.clone(),
                    _ => {
                        return ReferenceLinkEvaluation::Unresolved(
                            "unknown-attribute-type-prefix".into(),
                        )
                    }
                };
                (Some(uri), name)
            } else {
                (None, qname)
            };
            return match catalog.lookup_in_scope(&schema, namespace.as_deref(), name) {
                crate::datatype_names::DatatypeNameLookup::Target(target) => {
                    ReferenceLinkEvaluation::Resolved(vec![
                        self.source_reference(target.declaration().clone())
                    ])
                }
                crate::datatype_names::DatatypeNameLookup::Pending(reason) => {
                    ReferenceLinkEvaluation::Pending(reason.into())
                }
                crate::datatype_names::DatatypeNameLookup::Unresolved(reason) => {
                    ReferenceLinkEvaluation::Unresolved(reason.into())
                }
            };
        }
        self.attribute_datatype_literals.get(&slot.identity())
            .filter(|(original,_)| matches!(original.node(), CemAstNode::Attribute {value: Some(value),..} if value.trim() == qname))
            .map(|(_,target)| ReferenceLinkEvaluation::Resolved(vec![self.source_reference(target.clone())]))
            .unwrap_or_else(|| ReferenceLinkEvaluation::Pending("attribute-type-literal-binding-unavailable".into()))
    }
}
