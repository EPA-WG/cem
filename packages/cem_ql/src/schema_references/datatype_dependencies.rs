//! Explicit source/name admission; ordinary QL registration never enables datatypes.
use super::*;
use cem_ml::schema::datatype_registry::{
    DatatypeDependency, DatatypeDependencyHost, DatatypeSource,
};
impl CemQlSchemaDeclarationHost {
    pub fn register_datatype_source(&mut self, source: DatatypeSource) -> Result<(), &'static str> {
        if self.source_tree(source.declaration()).is_none() {
            return Err("unregistered-datatype-owner");
        }
        let key = source.declaration().identity();
        if self
            .datatype_sources
            .get(&key)
            .is_some_and(|old| old.scope().identity() != source.scope().identity())
        {
            return Err("conflicting-datatype-scope");
        }
        self.datatype_sources.insert(key, source);
        Ok(())
    }
    /// Bind an already-resolved lexical QName in its original declaring scope.
    /// Lookup still traverses the shared scope/depth/work and singleton checks.
    /// This never imports a namespace or manufactures cross-scope permission.
    pub fn bind_literal_datatype(
        &mut self,
        scope: &SchemaDeclarationNode,
        qname: &str,
        target: SchemaDeclarationNode,
    ) -> Result<(), &'static str> {
        let key = (scope.identity(), qname.into());
        if qname.trim().is_empty() {
            return Err("empty-datatype-name");
        }
        if self
            .datatype_literals
            .get(&key)
            .is_some_and(|old| old.identity() != target.identity())
        {
            return Err("duplicate-datatype-name");
        }
        if self.source_tree(scope).is_none() || self.source_tree(&target).is_none() {
            return Err("unregistered-datatype-owner");
        }
        self.datatype_literals.insert(key, target);
        Ok(())
    }
}
impl DatatypeDependencyHost for CemQlSchemaDeclarationHost {
    fn datatype_source(&self, target: &SchemaDeclarationNode) -> Option<DatatypeSource> {
        self.datatype_sources.get(&target.identity()).cloned()
    }
    fn lookup_literal_type(
        &mut self,
        source: &DatatypeSource,
        _: &DatatypeDependency,
        qname: &str,
    ) -> ReferenceLinkEvaluation<Self::Node> {
        self.datatype_literals
            .get(&(source.scope().identity(), qname.into()))
            .map(|target| {
                ReferenceLinkEvaluation::Resolved(vec![self.source_reference(target.clone())])
            })
            .unwrap_or_else(|| {
                ReferenceLinkEvaluation::Pending("datatype-name-binding-unavailable".into())
            })
    }
}
