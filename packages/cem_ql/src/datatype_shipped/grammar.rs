//! Bounded syntax preparation; conversion yields validated grammar text rather
//! than an executable rule, expanded QName or document matcher.
use super::*;
use crate::datatype_conversion::{
    ConversionCall, ConversionExecution, ConversionRepresentation, ConversionSignature,
    ConversionValue, NativeDatatypeConverter, RegisteredDatatypeConverter,
};
use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    schema::{
        datatype_registry::DatatypeSource, datatype_validation::CandidateRequirement,
        document_model::TypedAttributeValue,
    },
};

pub fn content_model_converter(
    source: DatatypeSource,
    limits: GrammarLimits,
) -> Result<RegisteredDatatypeConverter, &'static str> {
    RegisteredDatatypeConverter::new(
        source,
        "cemml:convert:content-model",
        ConversionSignature {
            kind: ShippedDatatype::ContentModel.kind(),
            input: ConversionRepresentation::Lexical,
            output: ShippedDatatype::ContentModel.representation(),
            candidate: CandidateRequirement::Optional,
        },
        GrammarConverter(limits),
    )
}
#[derive(Debug)]
struct GrammarConverter(GrammarLimits);
impl NativeDatatypeConverter for GrammarConverter {
    fn convert(&self, call: ConversionCall<'_>) -> ConversionExecution {
        let ConversionValue::Lexical(input) = call.value else {
            return ConversionExecution::Failed(vec![]);
        };
        let mut limits = self.0;
        limits.max_bytes = limits.max_bytes.min(call.limits.max_lexical_bytes);
        match validate_with_check(&input.text, limits, &mut || {
            call.runtime.control.check_scope(call.runtime.scope)
        }) {
            Ok(()) => ConversionExecution::Converted {
                value: vec![crate::typed_scalar::from_typed_value(
                    TypedAttributeValue {
                        datatype: "content-model".into(),
                        lexical: input.text.trim().into(),
                    },
                    Some(input.source.clone()),
                )],
                diagnostics: vec![],
            },
            Err(GrammarError::Limit(reason)) => ConversionExecution::Limit(reason),
            Err(GrammarError::Interrupted(_)) => ConversionExecution::Failed(vec![]),
            Err(GrammarError::Invalid { span }) => {
                let mut diagnostic = Diagnostic {
                    code: "cem.datatype.content-model.invalid".into(),
                    severity: Severity::Error,
                    message: "Malformed content-model grammar".into(),
                    details: Some(
                        serde_json::json!({"decodedSpan": {"start": span.start, "end": span.end}}),
                    ),
                    ..Default::default()
                };
                call.fallback.apply(&mut diagnostic);
                diagnostic.source_map = Some(input.source.clone());
                ConversionExecution::Rejected {
                    diagnostics: vec![diagnostic],
                }
            }
        }
    }
}
pub(super) fn limit_diagnostic(reason: &str, call: &ValidationCall<'_>) -> Diagnostic {
    let mut diagnostic = Diagnostic {
        code: "cem.datatype.content-model.limit".into(),
        severity: Severity::Error,
        message: format!("Content-model grammar exhausted its {reason} limit"),
        ..Default::default()
    };
    crate::datatype_results::DiagnosticAttribution::from_node(
        call.candidate.first().unwrap_or(call.datatype),
    )
    .apply(&mut diagnostic);
    diagnostic.source_map = call
        .value
        .first()
        .and_then(Item::source_map)
        .or(diagnostic.source_map);
    diagnostic
}

use crate::datatype_enumeration::{
    ConstantCall, ConstantExecution, NativeConstantInterpreter, RegisteredConstantInterpreter,
};
/// Interpret each existing whitespace-delimited vocabulary token independently.
/// No grammar equivalence or richer constant authoring is implied.
pub fn content_model_constant_interpreter(
    source: DatatypeSource,
    limits: GrammarLimits,
) -> Result<RegisteredConstantInterpreter, &'static str> {
    RegisteredConstantInterpreter::new(
        source,
        "cemml:constant:content-model",
        cem_ml::schema::datatype_validation::ScalarRepresentation::String,
        GrammarConverter(limits),
    )
}
impl NativeConstantInterpreter for GrammarConverter {
    fn interpret(&self, call: ConstantCall<'_>) -> ConstantExecution {
        match validate_with_check(call.token.text(), self.0, &mut || {
            call.runtime.control.check_scope(call.runtime.scope)
        }) {
            Ok(()) => ConstantExecution::Prepared {
                value: vec![crate::typed_scalar::from_typed_value(
                    TypedAttributeValue {
                        datatype: "content-model".into(),
                        lexical: call.token.text().trim().into(),
                    },
                    call.candidate
                        .source_map()
                        .or_else(|| call.fallback.source_map.clone()),
                )],
                diagnostics: vec![],
            },
            Err(GrammarError::Interrupted(_)) => ConstantExecution::Pending(vec![]),
            Err(GrammarError::Limit(reason)) => ConstantExecution::Unavailable(vec![Diagnostic {
                code: "cem.datatype.content-model.limit".into(),
                severity: Severity::Error,
                message: format!("Content-model constant exhausted its {reason} limit"),
                ..Default::default()
            }]),
            Err(GrammarError::Invalid { span }) => ConstantExecution::Rejected(vec![Diagnostic {
                code: "cem.datatype.content-model.invalid".into(),
                severity: Severity::Error,
                message: "Malformed content-model constant".into(),
                details: Some(
                    serde_json::json!({"decodedTokenSpan": {"start": span.start, "end": span.end}}),
                ),
                ..Default::default()
            }]),
        }
    }
}
