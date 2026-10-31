//! Revocable installation guards borrow, rather than own, reusable compilations.
use super::*;
use crate::{
    datatype_compilation::DatatypeValidation,
    datatype_conversion::{ConversionInput, ConversionLimits, DatatypeConversion},
    datatype_preparation::{DatatypePreparation, PreparationInput, PreparationLimits},
    datatype_serialization::{DatatypeSerialization, SerializationLimits},
    datatype_validation::{ValidationInput, ValidationLimits},
};
use cem_ml::parser::AstNodeId;
use std::sync::Weak;
pub struct InstalledDatatypeRoot {
    ownership: Arc<ownership::Installation>,
    compilation: Weak<FreshDependencyDatatypeCompilation>,
    datatype: AstNodeId,
}
impl InstalledDatatypeRoot {
    pub(super) fn new(
        ownership: Arc<ownership::Installation>,
        compilation: &Arc<FreshDependencyDatatypeCompilation>,
        datatype: AstNodeId,
    ) -> Self {
        Self {
            ownership,
            compilation: Arc::downgrade(compilation),
            datatype,
        }
    }
    fn compilation(
        &self,
        host: &FreshStructuralHost,
    ) -> Result<Arc<FreshDependencyDatatypeCompilation>, String> {
        self.ownership.check(host)?;
        let owner = self.compilation.upgrade().ok_or(INVALID)?;
        Ok(owner)
    }
    pub(in crate::structural_receiver) fn matches_original_root(
        &self,
        host: &FreshStructuralHost,
        parent: &Arc<crate::application_envelope::v3::VerifiedApplicationEnvelope>,
        target: &crate::application_envelope::v3::AssemblyRoot,
    ) -> bool {
        self.datatype == target.datatype
            && self.ownership.matches_parent(parent)
            && parent
                .assembly_owner(&target.assembly)
                .is_some_and(|graph| {
                    self.compilation(host)
                        .is_ok_and(|c| c.matches_assembly_root(host, graph, self.datatype))
                })
    }
    pub(in crate::structural_receiver) fn with_runtime<T>(
        &self,
        host: &FreshStructuralHost,
        run: impl FnOnce(
            &crate::datatype_compilation::ExecutableDatatype,
            &crate::datatype_validation::ValidationRuntime<'_>,
            &dyn crate::datatype_validation::RuleValidationContexts,
        ) -> T,
    ) -> Result<T, String> {
        let owner = self.compilation(host)?;
        let result = owner.with_root(host, self.datatype, true, run)?;
        self.compilation(host)?;
        Ok(result)
    }
    pub(in crate::structural_receiver) fn bind_receiver(
        &self,
        host: &mut FreshStructuralHost,
        declaration: cem_ml::schema::declaration_references::SchemaDeclarationNode,
    ) -> Result<crate::attribute_datatypes::BoundAttributeDatatype, String> {
        let owner = self.compilation(host)?;
        let result = owner.bind_original_receiver(host, self.datatype, declaration)?;
        self.compilation(host)?;
        Ok(result)
    }
    pub fn is_available(&self, host: &FreshStructuralHost) -> bool {
        self.compilation(host).is_ok()
    }
    pub fn retire(&self) {
        self.ownership.retire();
    }
    fn invoke<T>(
        &self,
        host: &FreshStructuralHost,
        run: impl FnOnce(&FreshDependencyDatatypeCompilation) -> Result<T, String>,
    ) -> Result<T, String> {
        let owner = self.compilation(host)?;
        let result = run(&owner);
        // Clearing/replacing the installation during a callback withholds late
        // output, while the invocation lease still bounds the borrowed owner.
        self.compilation(host)?;
        result
    }
    pub fn validate(
        &self,
        host: &FreshStructuralHost,
        input: &ValidationInput,
        limits: ValidationLimits,
    ) -> Result<DatatypeValidation, String> {
        self.invoke(host, |owner| {
            owner.validate(host, self.datatype, input, limits)
        })
    }
    pub fn prepare_lexical(
        &self,
        host: &FreshStructuralHost,
        input: &PreparationInput,
        limits: PreparationLimits,
    ) -> Result<DatatypePreparation, String> {
        self.invoke(host, |owner| {
            owner.prepare_lexical(host, self.datatype, input, limits)
        })
    }
    pub fn convert(
        &self,
        host: &FreshStructuralHost,
        input: &ConversionInput,
        limits: ConversionLimits,
    ) -> Result<DatatypeConversion, String> {
        self.invoke(host, |owner| {
            owner.convert(host, self.datatype, input, limits)
        })
    }
    pub fn serialize_list(
        &self,
        host: &FreshStructuralHost,
        input: &ValidationInput,
        limits: SerializationLimits,
    ) -> Result<DatatypeSerialization, String> {
        self.invoke(host, |owner| {
            owner.serialize_list(host, self.datatype, input, limits)
        })
    }
}
