//! Explicit registration of shipped validation semantics against retained profiles.
//! Choosing this capability is host authority; type names and rule prose never do it.
use crate::{
    datatype_results::DatatypeResultAdapter,
    datatype_validation::{
        DatatypeValidationRegistry, NativeDatatypeValidator, RuleExecution, ValidationCall,
    },
    eval::{AtomValue, Item, ItemStream},
};
use cem_ml::schema::{
    datatype_validation::{DatatypeBehaviorContract, ResultRepresentation},
    document_model::shipped_datatypes::ShippedDatatype,
    value_contracts::ValueContractError,
};
use std::collections::BTreeMap;

pub fn register_validation(
    registry: &mut DatatypeValidationRegistry,
    datatype: ShippedDatatype,
    contract: DatatypeBehaviorContract,
    adapter: DatatypeResultAdapter,
) -> Result<(), ValueContractError> {
    let fail = |code| {
        let mut error = ValueContractError::new(code);
        error.source = Some(contract.behavior().clone());
        error
    };
    if datatype == ShippedDatatype::ContentModel {
        return Err(fail("shipped-grammar-unavailable"));
    }
    let signature = contract.signature();
    if signature.kind != datatype.kind()
        || signature.value != datatype.representation()
        || !matches!(signature.result, ResultRepresentation::Accepted(_))
    {
        return Err(fail("shipped-validation-signature-mismatch"));
    }
    registry.register_native(
        &format!("cemml:datatype:{}", datatype.name()),
        contract,
        adapter,
        None,
        Validator(datatype),
    )
}
#[derive(Debug)]
struct Validator(ShippedDatatype);
impl NativeDatatypeValidator for Validator {
    fn validate(&self, call: ValidationCall<'_>) -> RuleExecution {
        let mut accepted = if self.0.item().is_some() {
            !call.value.is_empty()
        } else {
            call.value.len() == 1
        };
        let scalar = self.0.item().unwrap_or(self.0);
        for (index, value) in call.value.iter().enumerate() {
            if index % 64 == 0
                && call
                    .runtime
                    .control
                    .check_scope(call.runtime.scope)
                    .is_err()
            {
                return RuleExecution::Pending(vec![]);
            }
            let valid = if let Some(lexical) = crate::typed_scalar::integer_lexical(value) {
                scalar == ShippedDatatype::Integer && scalar.validate_lexical(lexical) == Some(true)
            } else {
                match (scalar, value.atom()) {
                    (ShippedDatatype::Boolean, Some(AtomValue::Boolean(_)))
                    | (ShippedDatatype::Integer, Some(AtomValue::Integer(_))) => true,
                    (ShippedDatatype::Number, Some(AtomValue::Decimal(value))) => {
                        scalar.validate_lexical(&value) == Some(true)
                    }
                    (_, Some(AtomValue::String(value))) => {
                        // Validate each supplied item under its own contract; do not
                        // concatenate, split or rewrite an already-typed sequence.
                        scalar.validate_lexical(&value) == Some(true)
                    }
                    _ => false,
                }
            };
            accepted &= valid;
        }
        let diagnostics = if accepted {
            vec![]
        } else {
            let text = |value: String| vec![Item::Atomic(AtomValue::String(value))];
            vec![Item::Record(BTreeMap::from([
                ("code".into(), text("cem.datatype.shipped.invalid".into())),
                ("severity".into(), text("error".into())),
                (
                    "message".into(),
                    text(format!(
                        "Value does not satisfy shipped {} validation",
                        self.0.name()
                    )),
                ),
            ]))]
        };
        RuleExecution::Complete(ItemStream::once(Item::Record(BTreeMap::from([
            (
                "accepted".into(),
                vec![Item::Atomic(AtomValue::Boolean(accepted))],
            ),
            ("diagnostics".into(), diagnostics),
        ]))))
    }
}

#[path = "datatype_shipped/conversion.rs"]
mod conversion;
pub use conversion::{constant_interpreter, converter};

#[path = "datatype_shipped/lists.rs"]
mod lists;
pub use lists::{list_implementation, token_source};
