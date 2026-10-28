//! Typed output capture at native producers, before explicit debug/control JSON.
use super::*;

pub(super) fn native_text_primary(
    context: Option<&EngineContext>,
    input: &EngineInput,
    text: &str,
    content_type: &str,
    schema: &str,
    spans: Vec<OutputSpan>,
) -> PrimaryBytes {
    let mut primary = primary_bytes_from_text_document(text, content_type, schema);
    let uri = context
        .map(|context| input_uri(input, context))
        .unwrap_or_else(|| input.uri.clone());
    primary.provenance = Some(crate::primary_provenance::PrimaryProvenance::from_writer(
        &uri,
        &input.bytes,
        &primary.bytes,
        spans,
    ));
    primary
}
