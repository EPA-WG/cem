//! Explicit source/name admission; ordinary QL registration never enables datatypes.
use super::*;
use cem_ml::schema::datatype_registry::{
    DatatypeDependency, DatatypeDependencyHost, DatatypeSource,
};
impl CemQlSchemaDeclarationHost {
    pub(crate) fn datatype_captured_prefix(
        &self,
        captured: &cem_ml::schema::machine::LexicallyScopedDocument,
        attribute: &SchemaDeclarationNode,
        prefix: &str,
    ) -> Result<Option<Option<String>>, &'static str> {
        let snapshot = captured
            .attribute_namespaces(attribute.document(), attribute.node_id())
            .ok_or("datatype-literal-namespace-context-unavailable")?;
        if let Some(declaration) = snapshot.pending.get(prefix) {
            let owner = Arc::as_ptr(attribute.document()) as usize;
            let completed = self
                .namespace_name_completions
                .get(&owner)
                .filter(|view| view.contains(attribute.node_id()));
            let declaration =
                SchemaDeclarationNode::new(attribute.document().clone(), *declaration)
                    .ok_or("datatype-namespace-declaration-unavailable")?;
            return Ok(Some(
                completed
                    .and_then(|view| view.binding_namespace_uri(&declaration).ok())
                    .map(str::to_owned),
            ));
        }
        Ok(snapshot
            .namespaces
            .binding(prefix)
            .map(|binding| Some(binding.namespace_uri.clone())))
    }

    /// Replace the complete name snapshot only after every original owner and
    /// scope is available. This does not register runtime inputs or crossing grants.
    pub fn install_datatype_names(
        &mut self,
        catalog: Arc<crate::datatype_names::DatatypeNameCatalog>,
    ) -> Result<(), &'static str> {
        for scope in catalog.scopes() {
            if self.source_tree(scope).is_none() {
                return Err("unregistered-datatype-owner");
            }
        }
        for source in catalog.sources() {
            if self.source_tree(source.declaration()).is_none() {
                return Err("unregistered-datatype-owner");
            }
            if self
                .datatype_names
                .as_ref()
                .and_then(|old| old.source(source.declaration()))
                .is_some_and(|old| old.scope().identity() != source.scope().identity())
            {
                return Err("conflicting-datatype-scope");
            }
            if self
                .datatype_sources
                .get(&source.declaration().identity())
                .is_some_and(|old| old.scope().identity() != source.scope().identity())
            {
                return Err("conflicting-datatype-scope");
            }
        }
        self.datatype_names = Some(catalog);
        Ok(())
    }

    pub fn register_datatype_source(&mut self, source: DatatypeSource) -> Result<(), &'static str> {
        if self.source_tree(source.declaration()).is_none() {
            return Err("unregistered-datatype-owner");
        }
        let key = source.declaration().identity();
        if self
            .datatype_names
            .as_ref()
            .and_then(|c| c.source(source.declaration()))
            .is_some_and(|old| old.scope().identity() != source.scope().identity())
        {
            return Err("conflicting-datatype-scope");
        }
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
        self.datatype_names
            .as_ref()
            .and_then(|catalog| catalog.source(target))
            .or_else(|| self.datatype_sources.get(&target.identity()))
            .cloned()
    }
    fn lookup_literal_type(
        &mut self,
        source: &DatatypeSource,
        _: &DatatypeDependency,
        qname: &str,
    ) -> ReferenceLinkEvaluation<Self::Node> {
        if let Some(catalog) = &self.datatype_names {
            if catalog.contains_scope(source.scope()) {
                return match catalog.lookup(source, qname) {
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
        }
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
