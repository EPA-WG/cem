//! Literal validation preparation. These capabilities do not normalize authored
//! values or invoke the registered conversion path.
use super::*;
use crate::datatype_preparation::{
    NativeLexicalPreparer, PreparationCall, PreparationExecution, PreparationSignature,
    RegisteredLexicalPreparation,
};
use cem_ml::schema::{
    datatype_registry::DatatypeSource,
    datatype_validation::{CandidateRequirement, ValueRepresentation},
    document_model::TypedAttributeValue,
};

/// Register explicitly for the original root implementation. Derived descriptors
/// inherit their base preparer; their own validators still restrict its result.
pub fn lexical_preparation(
    source: DatatypeSource,
    datatype: ShippedDatatype,
) -> Result<RegisteredLexicalPreparation, &'static str> {
    if let ValueRepresentation::List(item) = datatype.representation() {
        return RegisteredLexicalPreparation::list_items(
            source,
            format!("cemml:prepare:{}", datatype.name()),
            item,
        );
    }
    RegisteredLexicalPreparation::new(
        source,
        format!("cemml:prepare:{}", datatype.name()),
        PreparationSignature {
            kind: datatype.kind(),
            output: datatype.representation(),
            candidate: CandidateRequirement::Optional,
        },
        Preparer(datatype),
    )
}
#[derive(Debug)]
struct Preparer(ShippedDatatype);
impl NativeLexicalPreparer for Preparer {
    fn prepare(&self, call: PreparationCall<'_>) -> PreparationExecution {
        if call
            .runtime
            .control
            .check_scope(call.runtime.scope)
            .is_err()
        {
            return PreparationExecution::Pending(vec![]);
        }
        let text = &call.lexical.text;
        // Grammar syntax is checked by the bounded registered validation rule.
        if self.0.validate_lexical(text) == Some(false) {
            return PreparationExecution::Rejected(vec![]);
        }
        let value = match self.0 {
            ShippedDatatype::Boolean => Item::Atomic(AtomValue::Boolean(text.trim() != "false")),
            ShippedDatatype::Integer => crate::typed_scalar::from_typed_value(
                TypedAttributeValue {
                    datatype: "integer".into(),
                    lexical: text.trim().into(),
                },
                Some(call.lexical.source.clone()),
            ),
            ShippedDatatype::Number => Item::Atomic(AtomValue::Decimal(text.trim().into())),
            ShippedDatatype::String => Item::Atomic(AtomValue::String(text.to_string())),
            // These shipped lexical predicates already disregard boundary whitespace.
            // Keep the complete authored spelling in the preparation input.
            _ => Item::Atomic(AtomValue::String(text.trim().to_string())),
        };
        PreparationExecution::Prepared {
            value: vec![value],
            diagnostics: vec![],
        }
    }
}
