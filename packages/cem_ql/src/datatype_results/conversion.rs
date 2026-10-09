//! Closed tagged conversion envelopes; exact output representation is registered.
use super::*;
use crate::datatype_conversion::ConversionLimits;
use cem_ml::schema::{
    datatype_validation::ValueRepresentation,
    value_contracts::{Cardinality, ValueFieldType},
};
#[derive(Debug, Clone)]
pub struct DatatypeConversionResultAdapter {
    contracts: Arc<ValueContracts>,
    result: ContractName,
}
#[derive(Debug, Clone)]
pub struct DatatypeConversionResult {
    /// None means explicit rejection; Some(empty) is a completed empty conversion.
    pub value: Option<Vec<Item>>,
    pub diagnostics: Vec<Diagnostic>,
    pub execution_diagnostics: Vec<Diagnostic>,
    pub original: Item,
}
impl DatatypeConversionResultAdapter {
    pub fn new(
        contracts: Arc<ValueContracts>,
        result: ContractName,
        diagnostic: ContractName,
    ) -> Result<Self, ValueContractError> {
        let shape = contracts
            .get(&result)
            .ok_or_else(|| ValueContractError::new("unknown-result-contract"))?;
        let valid = shape.fields.get("status").is_some_and(|f| {
            f.value_type == ValueFieldType::String
                && f.required
                && f.cardinality == Cardinality::One
                && f.values.as_ref()
                    == Some(
                        &["converted".into(), "rejected".into()]
                            .into_iter()
                            .collect(),
                    )
        }) && shape.fields.get("value").is_some_and(|f| {
            f.value_type == ValueFieldType::Value
                && !f.required
                && f.cardinality == Cardinality::ZeroOrMore
        }) && shape.fields.get("diagnostics").is_some_and(|f| {
            f.value_type == ValueFieldType::Record(diagnostic.clone())
                && f.required
                && f.cardinality == Cardinality::ZeroOrMore
        });
        if !valid {
            return Err(ValueContractError::new(
                "incompatible-conversion-result-contract",
            ));
        }
        check_diagnostic_contract(&contracts, &diagnostic)?;
        Ok(Self { contracts, result })
    }
    pub fn result_contract(&self) -> &ContractName {
        &self.result
    }
    pub fn consume(
        &self,
        stream: ItemStream,
        fallback: &DiagnosticAttribution,
        output: ValueRepresentation,
        limits: ConversionLimits,
    ) -> Result<DatatypeConversionResult, DatatypeResultError> {
        if stream.error.is_some() {
            return Err(DatatypeResultError::Execution(stream));
        }
        if stream.items.len() != 1 {
            return Err(ValueContractError::new("result-cardinality").into());
        }
        let original = stream.items.into_iter().next().unwrap();
        let snapshot = QueryValue::new(original.clone());
        let fields = snapshot
            .record_fields()
            .ok_or_else(|| ValueContractError::new("record-required"))?;
        let get = |key: &str| {
            fields
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_slice())
        };
        if get("value").is_some_and(|v| v.len() > limits.max_output_values)
            || get("diagnostics")
                .map_or(0, |v| v.len())
                .saturating_add(stream.diagnostics.len())
                > limits.validation.max_diagnostics
        {
            return Err(ValueContractError::new("conversion-result-limit").into());
        }
        self.contracts.validate(&self.result, &[snapshot.clone()])?;
        let status = get("status")
            .and_then(|v| v.first())
            .and_then(ContractValue::string)
            .ok_or_else(|| ValueContractError::new("conversion-status-required"))?;
        let value = match (status.as_str(), get("value")) {
            ("converted", Some(values)) => {
                if matches!(output, ValueRepresentation::Scalar(_)) && values.len() != 1 {
                    return Err(ValueContractError::new("conversion-output-cardinality").into());
                }
                for value in values {
                    let valid = match output {
                        ValueRepresentation::Nodes => value.is_native_node(),
                        ValueRepresentation::Scalar(p) | ValueRepresentation::List(p) => {
                            value.matches_scalar(p)
                        }
                    };
                    if !valid {
                        return Err(
                            ValueContractError::new("conversion-output-representation").into()
                        );
                    }
                }
                Some(values.iter().map(|v| v.original.clone()).collect())
            }
            ("rejected", None) => None,
            _ => return Err(ValueContractError::new("conversion-tag-value-mismatch").into()),
        };
        let diagnostics = decode_diagnostics(
            get("diagnostics").ok_or_else(|| ValueContractError::new("diagnostics-required"))?,
            fallback,
        )?;
        Ok(DatatypeConversionResult {
            value,
            diagnostics,
            execution_diagnostics: stream.diagnostics,
            original,
        })
    }
}
