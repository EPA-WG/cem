//! Bounded native ownership for materialized document loaders.
use super::{import_data_bytes, RetainedCemTree, MAX_DOCUMENT_BYTES};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Default)]
pub struct CemDocuments {
    next_id: u32,
    bytes: usize,
    entries: BTreeMap<u32, (Arc<RetainedCemTree>, usize)>,
}

impl CemDocuments {
    pub fn retain(&mut self, bytes: &[u8], content_type: &str, uri: &str) -> Result<u32, String> {
        if self.entries.len() >= 64 || bytes.len() > MAX_DOCUMENT_BYTES.saturating_sub(self.bytes) {
            return Err("CEM retained-document limit exceeded.".into());
        }
        let id = self
            .next_id
            .checked_add(1)
            .ok_or("CEM document handle space exhausted.")?;
        let tree = import_data_bytes(bytes, content_type, "cem", uri)?;
        self.entries.insert(id, (tree, bytes.len()));
        self.next_id = id;
        self.bytes += bytes.len();
        Ok(id)
    }

    pub fn get(&self, id: u32) -> Option<Arc<RetainedCemTree>> {
        self.entries.get(&id).map(|(tree, _)| tree.clone())
    }

    pub fn dispose(&mut self, id: u32) -> bool {
        if let Some((_, bytes)) = self.entries.remove(&id) {
            self.bytes -= bytes;
            true
        } else {
            false
        }
    }
}
