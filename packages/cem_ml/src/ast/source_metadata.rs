//! Versioned passive metadata, separate from the native owning arena and runtime state.
use super::decode::DecodeError;
use crate::{
    parser::{document::CemDocument, format::DocumentFormatIdentity, AstNodeId},
    schema::ir::SemVer,
    tokenizer::cem::TypedPreludeValue,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceMetadata {
    version: u16,
    required_version: SemVer,
    format_identity: Option<DocumentFormatIdentity>,
    typed_preludes: BTreeMap<AstNodeId, TypedPreludeValue>,
}
impl SourceMetadata {
    pub(super) fn encode(document: &CemDocument) -> Vec<u8> {
        rmp_serde::to_vec_named(&Self {
            version: 1,
            required_version: SemVer::new(1, u64::from(!document.typed_preludes.is_empty()), 0),
            format_identity: document.format_identity.clone(),
            typed_preludes: document.typed_preludes.clone(),
        })
        .expect("source metadata contains only serializable fields")
    }
    pub(super) fn decode(
        bytes: &[u8],
        document: &mut CemDocument,
        max_entries: usize,
    ) -> Result<(), DecodeError> {
        let mut decoder = rmp_serde::Deserializer::new(std::io::Cursor::new(bytes));
        let metadata = Self::deserialize(&mut decoder).map_err(|_| DecodeError::InvalidMetadata)?;
        if decoder.position() != bytes.len() as u64 {
            return Err(DecodeError::InvalidMetadata);
        }
        if metadata.version != 1
            || metadata.required_version
                != SemVer::new(1, u64::from(!metadata.typed_preludes.is_empty()), 0)
            || metadata.typed_preludes.len() > max_entries
            || metadata.format_identity.as_ref().is_some_and(|identity| {
                (!metadata.typed_preludes.is_empty() && identity.format_version.minor < 1)
                    || identity.format_id != "cem-ml"
                    || identity.content_type != "text/cem-ml"
                    || !matches!(
                        identity.format_version,
                        SemVer {
                            major: 1,
                            minor: 0 | 1,
                            patch: 0,
                            prerelease: None,
                            ..
                        }
                    )
            })
        {
            return Err(DecodeError::InvalidMetadata);
        }
        document.format_identity = metadata.format_identity;
        document.typed_preludes = metadata.typed_preludes;
        crate::schema::prelude_values::validate_document_slots(document)
            .map_err(|_| DecodeError::InvalidMetadata)
    }
}
