//! Explicit list preparation. Item validation remains owned by the compiled item
//! descriptor; token offsets refer to decoded input, never fabricated source bytes.
use super::*;
use crate::{
    datatype_compilation::{DatatypeImplementation, TokenizerBinding},
    datatype_conversion::{
        ConversionCall, ConversionExecution, ConversionRepresentation, ConversionSignature,
        ConversionValue, NativeDatatypeConverter, RegisteredDatatypeConverter,
    },
    eval::{QueryItemView, QueryItemViewKind},
};
use cem_ml::{
    schema::{
        datatype_contracts::{
            ItemBounds, LexicalInput, RegisteredTokenizer, TokenizationError, TokenizationLimits,
            TokenizedInput,
        },
        datatype_registry::DatatypeSource,
        datatype_validation::CandidateRequirement,
    },
    source_map::SourceMapStack,
};
use std::{ops::Range, sync::Arc};

/// Select shipped nonempty bounds and whitespace tokenization for an original
/// list declaration. The host must still register/resolve its original item and
/// explicitly select conversion. No declaration name or rule prose selects this.
pub fn list_implementation(
    source: DatatypeSource,
    datatype: ShippedDatatype,
) -> Result<DatatypeImplementation, &'static str> {
    if datatype.item().is_none() {
        return Err("shipped-list-required");
    }
    Ok(DatatypeImplementation {
        source,
        kind: datatype.kind(),
        representation: datatype.representation(),
        accepted_bases: vec![],
        bounds: ItemBounds::new(1, None).unwrap(),
        tokenizer: TokenizerBinding::Ready(RegisteredTokenizer::whitespace()),
        validator: None,
    })
}

pub(super) fn converter(
    source: DatatypeSource,
    datatype: ShippedDatatype,
) -> Result<RegisteredDatatypeConverter, &'static str> {
    RegisteredDatatypeConverter::new(
        source,
        format!("cemml:convert:{}", datatype.name()),
        ConversionSignature {
            kind: datatype.kind(),
            input: ConversionRepresentation::Lexical,
            output: datatype.representation(),
            candidate: CandidateRequirement::Optional,
        },
        ListConverter,
    )
}
#[derive(Debug)]
struct ListConverter;
impl NativeDatatypeConverter for ListConverter {
    fn convert(&self, call: ConversionCall<'_>) -> ConversionExecution {
        let ConversionValue::Lexical(input) = call.value else {
            return ConversionExecution::Failed(vec![]);
        };
        let Some(tokenizer) = call.tokenizer else {
            return ConversionExecution::Unavailable(vec![]);
        };
        let tokens = match tokenizer.tokenize(
            Some(input.clone()),
            call.runtime.control,
            call.runtime.scope,
            TokenizationLimits {
                max_bytes: call.limits.max_lexical_bytes,
                max_tokens: call.limits.max_output_values,
                ..Default::default()
            },
        ) {
            Ok(tokens) => Arc::new(tokens),
            Err(TokenizationError::Limit) => return ConversionExecution::Limit("tokenization"),
            Err(TokenizationError::Rejected) => return ConversionExecution::Rejected { diagnostics: vec![] },
            Err(_) => return ConversionExecution::Failed(vec![]),
        };
        let mut value = Vec::with_capacity(tokens.tokens.len());
        for index in 0..tokens.tokens.len() {
            if index % 64 == 0
                && call
                    .runtime
                    .control
                    .check_scope(call.runtime.scope)
                    .is_err()
            {
                return ConversionExecution::Failed(vec![]);
            }
            value.push(Item::native(Token {
                tokens: tokens.clone(),
                index,
            }));
        }
        ConversionExecution::Converted {
            value,
            diagnostics: vec![],
        }
    }
}
#[derive(Debug)]
struct Token {
    tokens: Arc<TokenizedInput>,
    index: usize,
}
impl QueryItemView for Token {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.lexical-token"
    }
    fn identity(&self) -> String {
        format!(
            "{:?}:{}",
            self.tokens.tokens[self.index],
            &self.tokens.input.text[self.tokens.tokens[self.index].clone()]
        )
    }
    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Atomic
    }
    fn atom(&self) -> Option<AtomValue> {
        Some(AtomValue::String(
            self.tokens.input.text[self.tokens.tokens[self.index].clone()].into(),
        ))
    }
    fn source_map(&self) -> Option<SourceMapStack> {
        Some(self.tokens.input.source.clone())
    }
}
/// Return the retained decoded input and this occurrence's UTF-8 span. The
/// original source map is unchanged, including when decoding changed byte lengths.
pub fn token_source(value: &Item) -> Option<(&LexicalInput, Range<usize>)> {
    let token = value.view()?.downcast_ref::<Token>()?;
    Some((
        &token.tokens.input,
        token.tokens.tokens[token.index].clone(),
    ))
}
