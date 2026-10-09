//! Explicit original-datatype facet registration and local contract compilation.
use crate::attribute_datatypes::BoundAttributeDatatype;
use cem_ml::schema::{
    datatype_registry::DatatypeSource,
    document_model::attribute_facets::{
        AttributeFacetContract, FacetCompilationError, FacetFamily, FacetLimits,
    },
};

#[derive(Debug, Clone)]
pub struct RegisteredFacetProfile {
    source: DatatypeSource,
    implementation: String,
    family: FacetFamily,
}
impl RegisteredFacetProfile {
    pub fn new(
        source: DatatypeSource,
        implementation: impl Into<String>,
        family: FacetFamily,
    ) -> Result<Self, &'static str> {
        let implementation = implementation.into();
        if implementation.trim().is_empty() {
            return Err("empty-facet-profile-id");
        }
        Ok(Self {
            source,
            implementation,
            family,
        })
    }
    pub fn source(&self) -> &DatatypeSource {
        &self.source
    }
    pub fn implementation(&self) -> &str {
        &self.implementation
    }
    pub fn family(&self) -> FacetFamily {
        self.family
    }
}
#[derive(Debug, Clone)]
pub enum FacetProfileBinding {
    Unavailable,
    Ready(RegisteredFacetProfile),
}
#[derive(Debug, Clone)]
pub enum AttributeFacetBindingError {
    MissingProfile,
    Contract(FacetCompilationError),
}
#[derive(Debug, Clone)]
pub struct BoundAttributeFacets {
    binding: BoundAttributeDatatype,
    profile: RegisteredFacetProfile,
    contract: AttributeFacetContract,
}
impl BoundAttributeFacets {
    pub fn binding(&self) -> &BoundAttributeDatatype {
        &self.binding
    }
    pub fn profile(&self) -> &RegisteredFacetProfile {
        &self.profile
    }
    pub fn contract(&self) -> &AttributeFacetContract {
        &self.contract
    }
}
impl BoundAttributeDatatype {
    /// Compile only local facet semantics. Source/default/diagnostic lifecycle
    /// readiness and datatype validation remain required before activation.
    pub fn compile_facets(
        &self,
        schema_uri: &str,
        limits: FacetLimits,
    ) -> Result<BoundAttributeFacets, AttributeFacetBindingError> {
        let profile = self
            .datatype
            .facet_profile()
            .ok_or(AttributeFacetBindingError::MissingProfile)?;
        let contract = AttributeFacetContract::compile(
            schema_uri,
            self.local_constraints(),
            profile.family(),
            limits,
        )
        .map_err(AttributeFacetBindingError::Contract)?;
        Ok(BoundAttributeFacets {
            binding: self.clone(),
            profile: profile.clone(),
            contract,
        })
    }
}
