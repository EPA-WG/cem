//! Read-only source inputs for explicit host selection; never runtime authority.
use super::*;
use cem_ml::schema::declaration_references::SchemaDeclarationNode;
impl VerifiedDatatypeDependencies {
    pub fn roots(&self) -> &BTreeSet<AstNodeId> {
        &self.roots
    }
    pub fn dependency_ids(&self) -> impl Iterator<Item = &str> {
        self._dependencies.keys().map(String::as_str)
    }
    pub fn dependency_source(&self, id: &str) -> Option<&Arc<RetainedCemTree>> {
        Some(match self._dependencies.get(id)? {
            VerifiedArtifact::Datatype(v) => v.source(),
            VerifiedArtifact::Query(v) => v.source(),
            VerifiedArtifact::Native(v) => v.source(),
        })
    }
    pub fn dependency_capture(&self, id: &str) -> Option<&Arc<LexicallyScopedDocument>> {
        self._dependencies.get(id).map(VerifiedArtifact::capture)
    }
    pub(crate) fn dependency_schema(&self, id: &str) -> Option<SchemaDeclarationNode> {
        Some(match self._dependencies.get(id)? {
            VerifiedArtifact::Datatype(v) => v.sources().values().next()?.scope().clone(),
            VerifiedArtifact::Query(v) => v.contract.owner().clone(),
            VerifiedArtifact::Native(v) => match &v.requirement {
                native::Requirement::Validator(c) => c.owner().clone(),
                _ => return None,
            },
        })
    }
    pub fn dependency_datatypes(&self, id: &str) -> Option<&BTreeMap<AstNodeId, DatatypeSource>> {
        self.datatype_module(id)
            .map(VerifiedDatatypeSources::sources)
    }
    pub(crate) fn query_module(&self, id: &str) -> Option<&VerifiedQueryDeclaration> {
        match self._dependencies.get(id)? {
            VerifiedArtifact::Query(v) => Some(v),
            _ => None,
        }
    }
    pub(crate) fn native_module(&self, id: &str) -> Option<&VerifiedNativeRequirement> {
        match self._dependencies.get(id)? {
            VerifiedArtifact::Native(v) => Some(v),
            _ => None,
        }
    }
    pub(crate) fn datatype_module(&self, id: &str) -> Option<&VerifiedDatatypeSources> {
        match self._dependencies.get(id)? {
            VerifiedArtifact::Datatype(v) => Some(v),
            _ => None,
        }
    }
    pub(crate) fn datatype_modules(&self) -> impl Iterator<Item = &VerifiedDatatypeSources> {
        self._dependencies.values().filter_map(|v| match v {
            VerifiedArtifact::Datatype(v) => Some(v),
            _ => None,
        })
    }
    pub(crate) fn expected_target(
        &self,
        target: &DependencyTarget,
    ) -> Option<SchemaDeclarationNode> {
        Some(match target {
            DependencyTarget::LocalDatatype(id) => self.sources.get(id)?.declaration().clone(),
            DependencyTarget::Datatype { pin, root } => self
                .datatype_module(&pin.hash)?
                .sources()
                .get(root)?
                .declaration()
                .clone(),
            DependencyTarget::Query(pin) => match self._dependencies.get(&pin.hash)? {
                VerifiedArtifact::Query(v) => v.contract.behavior().clone(),
                _ => return None,
            },
            DependencyTarget::Native(pin) => match self._dependencies.get(&pin.hash)? {
                VerifiedArtifact::Native(v) => match &v.requirement {
                    native::Requirement::Validator(c) => c.behavior().clone(),
                    _ => return None,
                },
                _ => return None,
            },
        })
    }
}
