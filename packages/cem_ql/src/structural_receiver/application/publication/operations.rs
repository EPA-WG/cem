//! Only guarded operations leave the publication. Raw parent installation/root
//! handles and whole-schema models are not exported by this boundary.
use super::*;
use crate::{
    datatype_compilation::DatatypeValidation,
    datatype_conversion::{ConversionInput, ConversionLimits, DatatypeConversion},
    datatype_preparation::{DatatypePreparation, PreparationInput, PreparationLimits},
    datatype_serialization::{DatatypeSerialization, SerializationLimits},
    datatype_validation::{ValidationInput, ValidationLimits},
};
impl LinkedApplication {
    fn with_root<T>(
        &self,
        host: &mut FreshStructuralHost,
        name: &str,
        run: impl FnOnce(&v3::InstalledDatatypeRoot, &FreshStructuralHost) -> Result<T, String>,
    ) -> Result<T, String> {
        let _entry = self.guard.enter(host)?;
        self.guard.refresh(host)?;
        let result = run(self.parent.root(host, name)?, host);
        self.guard.refresh(host)?;
        result
    }
    pub fn validate_root(
        &self,
        host: &mut FreshStructuralHost,
        name: &str,
        input: &ValidationInput,
        limits: ValidationLimits,
    ) -> Result<DatatypeValidation, String> {
        self.with_root(host, name, |r, h| r.validate(h, input, limits))
    }
    pub fn prepare_root(
        &self,
        host: &mut FreshStructuralHost,
        name: &str,
        input: &PreparationInput,
        limits: PreparationLimits,
    ) -> Result<DatatypePreparation, String> {
        self.with_root(host, name, |r, h| r.prepare_lexical(h, input, limits))
    }
    pub fn convert_root(
        &self,
        host: &mut FreshStructuralHost,
        name: &str,
        input: &ConversionInput,
        limits: ConversionLimits,
    ) -> Result<DatatypeConversion, String> {
        self.with_root(host, name, |r, h| r.convert(h, input, limits))
    }
    pub fn serialize_root(
        &self,
        host: &mut FreshStructuralHost,
        name: &str,
        input: &ValidationInput,
        limits: SerializationLimits,
    ) -> Result<DatatypeSerialization, String> {
        self.with_root(host, name, |r, h| r.serialize_list(h, input, limits))
    }
    pub fn with_registry<T>(
        &self,
        host: &mut FreshStructuralHost,
        run: impl FnOnce(&SelectedFunctionRegistry, &FreshStructuralHost) -> T,
    ) -> Result<T, String> {
        let _entry = self.guard.enter(host)?;
        self.guard.refresh(host)?;
        let owner = self.guard.registry()?;
        let result = run(&owner.registry, host);
        // Release the invocation lease before checking caller ownership again.
        // The last external owner may have been dropped during this callback.
        drop(owner);
        self.guard.refresh(host)?;
        Ok(result)
    }
    pub fn validate_base_input(
        &self,
        host: &mut FreshStructuralHost,
        tag: &str,
        source: Arc<cem_ml::parser::tree::RetainedCemTree>,
    ) -> Result<
        cem_ml::schema::input_references::StructuralInputValidation<
            crate::schema_references::CemQlSchemaReferenceNode,
        >,
        String,
    > {
        let _entry = self.guard.enter(host)?;
        self.guard.refresh(host)?;
        let model = self.parent.application(host)?.model(host, tag)?;
        let result = host
            .validate_input(source, model)
            .map_err(|_| INVALID.to_string());
        self.guard.refresh(host)?;
        result
    }
}
