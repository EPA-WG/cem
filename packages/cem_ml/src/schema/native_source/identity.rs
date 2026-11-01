//! Build source identity shared by passive source and structural capsules.
use crate::source::SourceId;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    pub uri: String,
    pub hash: String,
    pub byte_length: usize,
    pub id: SourceId,
}
impl SourceIdentity {
    /// Trusted build input, tied to the original capture by its caller.
    pub fn new(uri: &str, id: SourceId, bytes: &[u8]) -> Self {
        Self {
            uri: uri.into(),
            hash: crate::content_cache::ContentHash::from_blake3(bytes).header_value(),
            byte_length: bytes.len(),
            id,
        }
    }
}
