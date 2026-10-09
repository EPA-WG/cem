//! Explicit scalar capabilities. Sharing a conversion kernel does not select or
//! invoke the runtime converter during enumeration constant preparation.
use super::*;
use crate::{
    datatype_conversion::{
        ConversionCall, ConversionExecution, ConversionRepresentation, ConversionSignature,
        ConversionValue, NativeDatatypeConverter, RegisteredDatatypeConverter,
    },
    datatype_enumeration::{
        ConstantCall, ConstantExecution, NativeConstantInterpreter, RegisteredConstantInterpreter,
    },
};
use cem_ml::{
    diagnostics::Diagnostic,
    schema::{
        datatype_registry::DatatypeSource,
        datatype_validation::{CandidateRequirement, ValueRepresentation},
        document_model::AttributeValueConversionError,
    },
};
fn supported(datatype: ShippedDatatype) -> Result<(), &'static str> {
    if datatype.supports_scalar_conversion() {
        Ok(())
    } else {
        Err("shipped-scalar-conversion-unavailable")
    }
}

pub fn converter(
    source: DatatypeSource,
    datatype: ShippedDatatype,
) -> Result<RegisteredDatatypeConverter, &'static str> {
    if datatype == ShippedDatatype::ContentModel {
        return super::content_model_converter(source, GrammarLimits::default());
    }
    if datatype.item().is_some() {
        return super::lists::converter(source, datatype);
    }
    supported(datatype)?;
    RegisteredDatatypeConverter::new(
        source,
        format!("cemml:convert:{}", datatype.name()),
        ConversionSignature {
            kind: datatype.kind(),
            input: ConversionRepresentation::Lexical,
            output: datatype.representation(),
            candidate: CandidateRequirement::Optional,
        },
        ScalarConverter(datatype),
    )
}
pub fn constant_interpreter(
    source: DatatypeSource,
    datatype: ShippedDatatype,
) -> Result<RegisteredConstantInterpreter, &'static str> {
    if datatype == ShippedDatatype::ContentModel {
        return super::content_model_constant_interpreter(source, GrammarLimits::default());
    }
    supported(datatype)?;
    let ValueRepresentation::Scalar(representation) = datatype.representation() else {
        unreachable!()
    };
    RegisteredConstantInterpreter::new(
        source,
        format!("cemml:constant:{}", datatype.name()),
        representation,
        ScalarConverter(datatype),
    )
}
#[derive(Debug)]
struct ScalarConverter(ShippedDatatype);
impl NativeDatatypeConverter for ScalarConverter {
    fn convert(&self, call: ConversionCall<'_>) -> ConversionExecution {
        let ConversionValue::Lexical(input) = call.value else {
            return ConversionExecution::Failed(vec![]);
        };
        match self
            .0
            .convert_lexical(&input.text, &input.source, &mut || {
                call.runtime.control.check_scope(call.runtime.scope)
            })
            .unwrap()
        {
            Ok(value) => ConversionExecution::Converted {
                value: vec![crate::typed_scalar::from_typed_value(
                    value,
                    Some(input.source.clone()),
                )],
                diagnostics: vec![],
            },
            Err(AttributeValueConversionError::Invalid(mut diagnostics)) => {
                attribute(&mut diagnostics, call.fallback);
                ConversionExecution::Rejected { diagnostics }
            }
            Err(AttributeValueConversionError::Interrupted(_)) => {
                ConversionExecution::Failed(vec![])
            }
        }
    }
}
impl NativeConstantInterpreter for ScalarConverter {
    fn interpret(&self, call: ConstantCall<'_>) -> ConstantExecution {
        let source = call
            .candidate
            .source_map()
            .or_else(|| call.fallback.source_map.clone())
            .unwrap_or_default();
        match self
            .0
            .convert_lexical(call.token.text(), &source, &mut || {
                call.runtime.control.check_scope(call.runtime.scope)
            })
            .unwrap()
        {
            Ok(value) => ConstantExecution::Prepared {
                value: vec![crate::typed_scalar::from_typed_value(value, Some(source))],
                diagnostics: vec![],
            },
            Err(AttributeValueConversionError::Invalid(mut diagnostics)) => {
                attribute(&mut diagnostics, call.fallback);
                ConstantExecution::Rejected(diagnostics)
            }
            Err(AttributeValueConversionError::Interrupted(_)) => ConstantExecution::Failed(vec![]),
        }
    }
}

fn attribute(
    diagnostics: &mut [Diagnostic],
    fallback: &crate::datatype_results::DiagnosticAttribution,
) {
    for diagnostic in diagnostics {
        if diagnostic.uri.is_none() && diagnostic.node.is_none() {
            let source_map = diagnostic.source_map.clone();
            fallback.apply(diagnostic);
            if source_map
                .as_ref()
                .is_some_and(|source| !source.frames.is_empty())
                || diagnostic.source_map.is_none()
            {
                diagnostic.source_map = source_map;
            }
        }
    }
}
