//! Checked schema-owned scalar comparison and constant preparation records.
use super::*;
use crate::datatype_validation::{scalar, ValidationLimits};
use cem_ml::schema::{
    datatype_validation::ScalarRepresentation,
    value_contracts::{Cardinality, ValueFieldType},
};

#[derive(Debug, Clone)]
struct Envelope {
    contracts: Arc<ValueContracts>,
    result: ContractName,
}
impl Envelope {
    fn new(
        contracts: Arc<ValueContracts>,
        result: ContractName,
        diagnostic: ContractName,
        constant: bool,
    ) -> Result<Self, ValueContractError> {
        let shape = contracts
            .get(&result)
            .ok_or_else(|| ValueContractError::new("unknown-result-contract"))?;
        let diagnostics = shape.fields.get("diagnostics").is_some_and(|f| {
            f.value_type == ValueFieldType::Record(diagnostic.clone())
                && f.required
                && f.cardinality == Cardinality::ZeroOrMore
        });
        let outcome = if constant {
            shape.fields.get("status").is_some_and(|f| {
                f.value_type == ValueFieldType::String
                    && f.required
                    && f.cardinality == Cardinality::One
                    && f.values.as_ref()
                        == Some(&["prepared".into(), "rejected".into()].into_iter().collect())
            }) && shape.fields.get("value").is_some_and(|f| {
                f.value_type == ValueFieldType::Value
                    && !f.required
                    && f.cardinality == Cardinality::One
            })
        } else {
            shape.fields.get("equal").is_some_and(|f| {
                f.value_type == ValueFieldType::Boolean
                    && f.required
                    && f.cardinality == Cardinality::One
            })
        };
        if !diagnostics || !outcome {
            return Err(ValueContractError::new(
                "incompatible-enumeration-result-contract",
            ));
        }
        check_diagnostic_contract(&contracts, &diagnostic)?;
        Ok(Self { contracts, result })
    }
    fn consume(
        &self,
        stream: ItemStream,
        limits: ValidationLimits,
    ) -> Result<Snapshot, DatatypeResultError> {
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
        let result = Snapshot {
            fields,
            original,
            emitted: stream.diagnostics,
        };
        if result
            .get("diagnostics")
            .map_or(0, |v| v.len())
            .saturating_add(result.emitted.len())
            > limits.max_diagnostics
        {
            return Err(ValueContractError::new("enumeration-result-limit").into());
        }
        self.contracts.validate(&self.result, &[snapshot])?;
        Ok(result)
    }
}
struct Snapshot {
    fields: Vec<(String, Vec<QueryValue>)>,
    original: Item,
    emitted: Vec<Diagnostic>,
}
impl Snapshot {
    fn get(&self, name: &str) -> Option<&[QueryValue]> {
        self.fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, values)| values.as_slice())
    }
    fn diagnostics(
        &self,
        fallback: &DiagnosticAttribution,
    ) -> Result<Vec<Diagnostic>, ValueContractError> {
        decode_diagnostics(
            self.get("diagnostics")
                .ok_or_else(|| ValueContractError::new("diagnostics-required"))?,
            fallback,
        )
    }
}
#[derive(Debug, Clone)]
pub struct DatatypeEqualityResultAdapter(Envelope);
#[derive(Debug, Clone)]
pub struct DatatypeConstantResultAdapter(Envelope);
#[derive(Debug, Clone)]
pub struct DatatypeEqualityResult {
    pub equal: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub execution_diagnostics: Vec<Diagnostic>,
    pub original: Item,
}
#[derive(Debug, Clone)]
pub struct DatatypeConstantResult {
    /// None is explicit rejection. A prepared scalar must be present exactly once.
    pub value: Option<Item>,
    pub diagnostics: Vec<Diagnostic>,
    pub execution_diagnostics: Vec<Diagnostic>,
    pub original: Item,
}
macro_rules! adapter {
    ($name:ident, $constant:expr) => {
        impl $name {
            pub fn new(
                contracts: Arc<ValueContracts>,
                result: ContractName,
                diagnostic: ContractName,
            ) -> Result<Self, ValueContractError> {
                Envelope::new(contracts, result, diagnostic, $constant).map(Self)
            }
            pub fn result_contract(&self) -> &ContractName {
                &self.0.result
            }
        }
    };
}
adapter!(DatatypeEqualityResultAdapter, false);
adapter!(DatatypeConstantResultAdapter, true);
impl DatatypeEqualityResultAdapter {
    pub fn consume(
        &self,
        stream: ItemStream,
        fallback: &DiagnosticAttribution,
        limits: ValidationLimits,
    ) -> Result<DatatypeEqualityResult, DatatypeResultError> {
        let result = self.0.consume(stream, limits)?;
        let equal = result
            .get("equal")
            .and_then(|v| v.first())
            .and_then(ContractValue::boolean)
            .ok_or_else(|| ValueContractError::new("comparison-boolean-required"))?;
        Ok(DatatypeEqualityResult {
            equal,
            diagnostics: result.diagnostics(fallback)?,
            execution_diagnostics: result.emitted,
            original: result.original,
        })
    }
}
impl DatatypeConstantResultAdapter {
    pub fn consume(
        &self,
        stream: ItemStream,
        fallback: &DiagnosticAttribution,
        representation: ScalarRepresentation,
        limits: ValidationLimits,
    ) -> Result<DatatypeConstantResult, DatatypeResultError> {
        let result = self.0.consume(stream, limits)?;
        let status = result
            .get("status")
            .and_then(|v| v.first())
            .and_then(ContractValue::string)
            .ok_or_else(|| ValueContractError::new("constant-status-required"))?;
        let value = match (status.as_str(), result.get("value")) {
            ("prepared", Some([value]))
                if value
                    .atom
                    .get_or_init(|| value.read_atom())
                    .as_ref()
                    .is_some_and(|a| scalar(&Item::Atomic(a.clone()), representation)) =>
            {
                Some(value.original.clone())
            }
            ("rejected", None) => None,
            _ => return Err(ValueContractError::new("constant-tag-value-mismatch").into()),
        };
        Ok(DatatypeConstantResult {
            value,
            diagnostics: result.diagnostics(fallback)?,
            execution_diagnostics: result.emitted,
            original: result.original,
        })
    }
}
