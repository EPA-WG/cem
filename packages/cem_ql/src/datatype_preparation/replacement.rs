//! Checked replacement compares immutable primitive representations, not query
//! equality, formatting, user-defined coercion, or public representation tags.
use super::*;
use crate::eval::AtomValue;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Fingerprint {
    String(String),
    Integer(i64),
    WideInteger(String),
    Decimal(String),
    Double(u64),
    Boolean(bool),
    AnyUri(String),
}

pub(super) fn fingerprint(
    value: &Item,
    input: &PreparationInput,
    span: Option<&Range<usize>>,
    max_bytes: usize,
) -> Result<Fingerprint, PreparationStop> {
    let bounded = |text: &str| {
        if text.len() > max_bytes {
            Err(PreparationStop::Limit("replacement-value-bytes"))
        } else {
            Ok(())
        }
    };
    if let Item::Atomic(atom) = value {
        if let AtomValue::String(text) | AtomValue::Decimal(text) | AtomValue::AnyUri(text) = atom {
            bounded(text)?;
        }
    } else if let Some((text, source)) = crate::typed_scalar::immutable_storage(value) {
        bounded(text)?;
        if source.is_some_and(|source| source != &input.lexical.source) {
            return Err(PreparationStop::IncompatibleReplacement);
        }
        if let Some(integer) = crate::typed_scalar::integer_lexical(value) {
            return Ok(Fingerprint::WideInteger(integer.to_owned()));
        }
    } else if let Some((original, token)) = crate::datatype_shipped::token_source(value) {
        if !Arc::ptr_eq(&original.text, &input.lexical.text)
            || original.source != input.lexical.source
            || span != Some(&token)
        {
            return Err(PreparationStop::IncompatibleReplacement);
        }
        bounded(&original.text[token])?;
    } else {
        return Err(PreparationStop::IncompatibleReplacement);
    }
    match value.atom() {
        Some(AtomValue::String(value)) => Ok(Fingerprint::String(value)),
        Some(AtomValue::Integer(value)) => Ok(Fingerprint::Integer(value)),
        Some(AtomValue::Decimal(value)) => Ok(Fingerprint::Decimal(value)),
        Some(AtomValue::Double(value)) => Ok(Fingerprint::Double(value.to_bits())),
        Some(AtomValue::Boolean(value)) => Ok(Fingerprint::Boolean(value)),
        Some(AtomValue::AnyUri(value)) => Ok(Fingerprint::AnyUri(value)),
        _ => Err(PreparationStop::IncompatibleReplacement),
    }
}
