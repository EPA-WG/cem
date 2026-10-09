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
        self.attribute_datatype_literals.get(&slot.identity())
            .filter(|(original,_)| matches!(original.node(), CemAstNode::Attribute {value: Some(value),..} if value.trim() == qname))
            .map(|(_,target)| ReferenceLinkEvaluation::Resolved(vec![self.source_reference(target.clone())]))
            .unwrap_or_else(|| ReferenceLinkEvaluation::Pending("attribute-type-literal-binding-unavailable".into()))
    }
}
