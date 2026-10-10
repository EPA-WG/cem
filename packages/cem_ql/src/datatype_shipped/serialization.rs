//! Canonical lexical export for explicitly registered shipped name lists.
use crate::{
    datatype_results::DiagnosticAttribution,
    datatype_serialization::{
        ListSerializationCall, ListSerializationSignature, NativeListSerializer,
        RegisteredListSerializer, SerializationExecution,
    },
    eval::AtomValue,
};
use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::schema::{
    datatype_registry::DatatypeSource,
    datatype_validation::{CandidateRequirement, ScalarRepresentation},
    document_model::shipped_datatypes::ShippedDatatype,
};

pub fn list_serializer(
    source: DatatypeSource,
    datatype: ShippedDatatype,
) -> Result<RegisteredListSerializer, &'static str> {
    let item = datatype.item().ok_or("shipped-list-required")?;
    RegisteredListSerializer::new(
        source,
        format!("cemml:serialize:{}:whitespace", datatype.name()),
        ListSerializationSignature {
            item: ScalarRepresentation::String,
            candidate: CandidateRequirement::Optional,
        },
        NameListSerializer(item),
    )
}
#[derive(Debug)]
struct NameListSerializer(ShippedDatatype);
impl NativeListSerializer for NameListSerializer {
    fn serialize(&self, call: ListSerializationCall<'_>) -> SerializationExecution {
        let mut text = String::new();
        for (index, item) in call.value.iter().enumerate() {
            if call
                .runtime
                .control
                .check_scope(call.runtime.scope)
                .is_err()
            {
                return SerializationExecution::Failed(vec![]);
            }
            let Some(AtomValue::String(value)) = item.atom() else {
                return SerializationExecution::Failed(vec![]);
            };
            let needed = text
                .len()
                .checked_add(usize::from(index != 0))
                .and_then(|len| len.checked_add(value.len()));
            if needed.is_none_or(|len| len > call.limits.max_output_bytes) {
                return SerializationExecution::Limit("output-bytes");
            }
            // Legacy lexical validation trims boundary whitespace. A typed item
            // must already be one spelling: trimming here would change its value.
            if value.chars().any(char::is_whitespace)
                || self.0.validate_lexical(&value) != Some(true)
            {
                let mut diagnostic = Diagnostic {
                    code: "cem.datatype.serialization.item".into(),
                    severity: Severity::Error,
                    message:
                        "List serialization requires one valid item spelling without whitespace"
                            .into(),
                    ..Default::default()
                };
                let attribution = if item.view().is_some() {
                    DiagnosticAttribution::from_node(item)
                } else {
                    call.fallback.clone()
                };
                attribution.apply(&mut diagnostic);
                return SerializationExecution::Rejected {
                    diagnostics: vec![diagnostic],
                };
            }
            if index != 0 {
                text.push(' ');
            }
            text.push_str(&value);
        }
        SerializationExecution::Serialized {
            text,
            diagnostics: vec![],
        }
    }
}
