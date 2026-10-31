//! Root-only assembly execution borrows actual contexts and shares the compiler
//! snapshot's owner/lifetime and reentry guards. Reports grant no portable proof.
use super::*;
use crate::{
    datatype_conversion::{ConversionInput, ConversionLimits, DatatypeConversion},
    datatype_preparation::{DatatypePreparation, PreparationInput, PreparationLimits},
    datatype_serialization::{DatatypeSerialization, SerializationLimits},
};
impl FreshDependencyDatatypeCompilation {
    pub fn prepare_lexical(
        &self,
        host: &FreshStructuralHost,
        root: AstNodeId,
        input: &PreparationInput,
        limits: PreparationLimits,
    ) -> Result<DatatypePreparation, String> {
        self.with_root(host, root, true, |d, r, c| {
            d.prepare_with_contexts(input, r, limits, c)
        })
    }
    pub fn convert(
        &self,
        host: &FreshStructuralHost,
        root: AstNodeId,
        input: &ConversionInput,
        limits: ConversionLimits,
    ) -> Result<DatatypeConversion, String> {
        self.with_root(host, root, true, |d, r, c| {
            d.convert_with_contexts(input, r, limits, c)
        })
    }
    pub fn serialize_list(
        &self,
        host: &FreshStructuralHost,
        root: AstNodeId,
        input: &ValidationInput,
        limits: SerializationLimits,
    ) -> Result<DatatypeSerialization, String> {
        self.with_root(host, root, true, |d, r, c| {
            d.serialize_with_contexts(input, r, limits, c)
        })
    }
}
