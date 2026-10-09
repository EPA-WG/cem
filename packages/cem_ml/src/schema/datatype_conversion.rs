//! Passive source contracts for explicitly registered conversion capabilities.
use super::{
    datatype_registry::DatatypeKind,
    datatype_validation::{
        BehaviorProfile, CandidateRequirement, DatatypeBehaviorContract, ResultRepresentation,
        ScalarRepresentation, ValidationImplementation, ValidationSignature, ValueRepresentation,
    },
    declaration_references::SchemaDeclarationNode,
    value_contracts::{ContractName, ValueContractError, ValueContractSource},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionRepresentation {
    Lexical,
    Values(ValueRepresentation),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversionSignature {
    pub kind: DatatypeKind,
    pub input: ConversionRepresentation,
    pub output: ValueRepresentation,
    pub candidate: CandidateRequirement,
}
impl ConversionSignature {
    pub fn check(self) -> Result<(), &'static str> {
        let valid = match self.kind {
            DatatypeKind::Node => self.output == ValueRepresentation::Nodes,
            DatatypeKind::List => matches!(self.output, ValueRepresentation::List(_)),
            _ => matches!(self.output, ValueRepresentation::Scalar(_)),
        };
        if !valid {
            return Err("converter-kind-output-mismatch");
        }
        if (self.input == ConversionRepresentation::Values(ValueRepresentation::Nodes))
            != (self.output == ValueRepresentation::Nodes)
        {
            return Err("converter-node-scalar-boundary");
        }
        Ok(())
    }
    /// Lexical query input is the original text; source maps stay on the call.
    pub fn value_representation(self) -> ValueRepresentation {
        match self.input {
            ConversionRepresentation::Lexical => {
                ValueRepresentation::Scalar(ScalarRepresentation::String)
            }
            ConversionRepresentation::Values(value) => value,
        }
    }
}
#[derive(Debug, Clone)]
pub struct ConversionBehaviorContract {
    source: DatatypeBehaviorContract,
    signature: ConversionSignature,
    result: ContractName,
}
impl ConversionBehaviorContract {
    pub fn compile(
        source: &ValueContractSource,
        behavior: &SchemaDeclarationNode,
        signature: ConversionSignature,
        result: ContractName,
    ) -> Result<Self, ValueContractError> {
        signature
            .check()
            .map_err(|code| ValueContractError::at(code, behavior))?;
        let contract = DatatypeBehaviorContract::compile_profile(
            source,
            behavior,
            ValidationSignature {
                kind: signature.kind,
                value: signature.value_representation(),
                candidate: signature.candidate,
                result: ResultRepresentation::Accepted(result.clone()),
            },
            BehaviorProfile::Conversion,
        )?;
        Ok(Self {
            source: contract,
            signature,
            result,
        })
    }
    pub fn signature(&self) -> ConversionSignature {
        self.signature
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
