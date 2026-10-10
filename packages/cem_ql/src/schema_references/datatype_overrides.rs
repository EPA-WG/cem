//! Exact input stamps for datatype transactions. No context or grant is serialized.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatatypeOverrideHostStamp {
    identity: u64,
    contexts: u64,
    scopes: usize,
    assignments: BTreeMap<(usize, AstNodeId), DeclarationScope>,
    following: BTreeMap<(usize, AstNodeId), BTreeMap<usize, DeclarationScope>>,
    grants: BTreeSet<(DeclarationScope, DeclarationScope)>,
    names: Vec<(usize, AstNodeId, String, String, Option<u32>)>,
    completions: Vec<(usize, usize)>,
    captures: Vec<(usize, usize)>,
    schema_forms: BTreeMap<usize, BTreeMap<AstNodeId, cem_ml::schema::machine::SchemaElementForm>>,
    schema_loads: BTreeMap<(usize, AstNodeId, String), u64>,
    publications: Vec<(usize, AstNodeId)>,
    catalog: Option<usize>,
    datatypes: Vec<(String, String)>,
    literals: Vec<((String, String), String)>,
    attributes: Vec<(String, String)>,
    enabled: bool,
}
impl CemQlSchemaDeclarationHost {
    pub(crate) fn datatype_override_stamp(&self) -> DatatypeOverrideHostStamp {
        DatatypeOverrideHostStamp {
            identity: self.identity,
            contexts: self.namespace_input_snapshot,
            scopes: self.scopes.len(),
            assignments: self.node_scopes.clone(),
            following: self.following_scopes.clone(),
            grants: self.grants.clone(),
            names: self
                .captured_names
                .iter()
                .flat_map(|(&owner, names)| {
                    names.iter().map(move |(&id, name)| {
                        (
                            owner,
                            id,
                            name.namespace_uri.clone(),
                            name.local_name.clone(),
                            name.schema_id,
                        )
                    })
                })
                .collect(),
            completions: self
                .namespace_name_completions
                .iter()
                .map(|(&k, v)| (k, Arc::as_ptr(v) as usize))
                .collect(),
            captures: self
                .captured_namespaces
                .iter()
                .map(|(&k, v)| (k, Arc::as_ptr(v) as usize))
                .collect(),
            schema_forms: self.captured_schema_forms.clone(),
            schema_loads: self.schema_uri_generations.clone(),
            publications: self.namespace_publications.keys().copied().collect(),
            catalog: self
                .datatype_names
                .as_ref()
                .map(|c| Arc::as_ptr(c) as usize),
            datatypes: self
                .datatype_sources
                .iter()
                .map(|(key, s)| (key.clone(), s.scope().identity()))
                .collect(),
            literals: self
                .datatype_literals
                .iter()
                .map(|(key, n)| (key.clone(), n.identity()))
                .collect(),
            attributes: self
                .attribute_datatype_literals
                .iter()
                .map(|(key, (_, n))| (key.clone(), n.identity()))
                .collect(),
            enabled: self.compiled_attribute_types,
        }
    }
    pub(crate) fn datatype_override_catalog_matches(
        &self,
        names: &Arc<crate::datatype_names::DatatypeNameCatalog>,
    ) -> bool {
        self.datatype_names
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, names))
    }
    pub(crate) fn datatype_override_operation_ready(&self) -> bool {
        self.check_operation().is_ok()
    }
}
