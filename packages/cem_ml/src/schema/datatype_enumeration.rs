//! Passive scalar capability profiles; compilation grants no execution authority.
use super::{
    datatype_registry::DatatypeKind,
    datatype_validation::{
        BehaviorProfile, CandidateRequirement, DatatypeBehaviorContract, ResultRepresentation,
        ScalarRepresentation, ValidationImplementation, ValidationSignature, ValueRepresentation,
    },
    declaration_references::SchemaDeclarationNode,
    function_references::{FunctionSelection, FunctionSelectionBudget},
    value_contracts::{ContractName, ValueContractError, ValueContractSource},
};
macro_rules! contract {
    ($name:ident, $profile:ident, $input:expr) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            source: DatatypeBehaviorContract,
            representation: ScalarRepresentation,
            result: ContractName,
        }
        impl $name {
            pub fn compile_selected(
                selection: &FunctionSelection,
                representation: ScalarRepresentation,
                result: ContractName,
                budget: &mut FunctionSelectionBudget,
            ) -> Result<Self, ValueContractError> {
                let input: fn(ScalarRepresentation) -> ScalarRepresentation = $input;
                let source = DatatypeBehaviorContract::compile_selected_profile(
                    selection,
                    ValidationSignature {
                        kind: DatatypeKind::Scalar,
                        value: ValueRepresentation::Scalar(input(representation)),
                        candidate: CandidateRequirement::Required,
                        result: ResultRepresentation::Accepted(result.clone()),
                    },
                    BehaviorProfile::$profile,
                    budget,
                )?;
                Ok(Self {
                    source,
                    representation,
                    result,
                })
            }
            pub fn compile(
                source: &ValueContractSource,
                behavior: &SchemaDeclarationNode,
                representation: ScalarRepresentation,
                result: ContractName,
            ) -> Result<Self, ValueContractError> {
                let input: fn(ScalarRepresentation) -> ScalarRepresentation = $input;
                let source = DatatypeBehaviorContract::compile_profile(
                    source,
                    behavior,
                    ValidationSignature {
                        kind: DatatypeKind::Scalar,
                        value: ValueRepresentation::Scalar(input(representation)),
                        candidate: CandidateRequirement::Required,
                        result: ResultRepresentation::Accepted(result.clone()),
                    },
                    BehaviorProfile::$profile,
                )?;
                Ok(Self {
                    source,
                    representation,
                    result,
                })
            }
            pub fn representation(&self) -> ScalarRepresentation {
                self.representation
            }
            pub fn owner(&self) -> &SchemaDeclarationNode {
                self.source.owner()
            }
            pub fn behavior(&self) -> &SchemaDeclarationNode {
                self.source.behavior()
            }
            pub fn implementation(&self) -> &ValidationImplementation {
                self.source.implementation()
            }
            pub fn result_contract(&self) -> &ContractName {
                &self.result
            }
        }
    };
}
contract!(EqualityBehaviorContract, Equality, |p| p);
contract!(ConstantBehaviorContract, Constant, |_| {
    ScalarRepresentation::String
});
