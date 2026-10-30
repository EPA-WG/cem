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
    availability: crate::datatype_availability::Availability,
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
            availability: Default::default(),
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
    pub fn with_availability(
        mut self,
        guard: std::sync::Arc<dyn crate::datatype_availability::DatatypeAvailability>,
    ) -> Self {
        self.availability.restrict(guard);
        self
    }
    pub(crate) fn is_available(&self) -> bool {
        self.availability.is_available()
    }
}
impl crate::datatype_compilation::ExecutableDatatype {
    pub(crate) fn facets_available(&self) -> bool {
        self.facet_profiles().iter().all(|p| p.is_available())
            && self.item().is_none_or(|item| item.facets_available())
    }
}
#[derive(Debug, Clone)]
pub enum FacetProfileBinding {
    Unavailable,
    Ready(RegisteredFacetProfile),
    /// Intersect the selected profile with every retained ancestor profile.
    CheckedReplacement(RegisteredFacetProfile),
}
#[derive(Debug, Clone)]
pub enum AttributeFacetBindingError {
    MissingProfile,
    Unavailable,
    Contract(FacetCompilationError),
}
#[derive(Debug, Clone)]
pub struct BoundAttributeFacets {
    identity: std::sync::Arc<()>,
    binding: BoundAttributeDatatype,
    profile: RegisteredFacetProfile,
    contract: AttributeFacetContract,
}
impl BoundAttributeFacets {
    pub(crate) fn is_available(&self) -> bool {
        self.binding.datatype.facets_available()
    }
    pub(crate) fn same_binding(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.identity, &other.identity)
    }
    pub fn binding(&self) -> &BoundAttributeDatatype {
        &self.binding
    }
    pub fn profile(&self) -> &RegisteredFacetProfile {
        &self.profile
    }
    /// Original registrations in validation order, including the selected profile.
    pub fn profiles(&self) -> &[RegisteredFacetProfile] {
        self.binding.datatype.facet_profiles()
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
        if !self.datatype.facets_available() {
            return Err(AttributeFacetBindingError::Unavailable);
        }
        let profile = self
            .datatype
            .facet_profile()
            .ok_or(AttributeFacetBindingError::MissingProfile)?;
        let families: Vec<_> = self
            .datatype
            .facet_profiles()
            .iter()
            .map(|p| p.family())
            .collect();
        let contract = AttributeFacetContract::compile_profiles(
            schema_uri,
            self.local_constraints(),
            &families,
            limits,
        )
        .map_err(AttributeFacetBindingError::Contract)?;
        Ok(BoundAttributeFacets {
            identity: std::sync::Arc::new(()),
            binding: self.clone(),
            profile: profile.clone(),
            contract,
        })
    }
}
