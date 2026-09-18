//! Explicit host retention; importing never changes an existing template or a
//! default registry. Disposal releases the handle, not previously cloned calls.
use super::*;

const MAX_COMPANIONS: usize = 64;
const MAX_RETAINED_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Default)]
pub struct CemtXPathCompanions {
    next_id: u32,
    byte_count: usize,
    entries: BTreeMap<u32, (NativeFunctionRegistry, usize)>,
}

impl CemtXPathCompanions {
    pub fn import(
        &mut self,
        bytes: &[u8],
        expected_content_hash: &ContentHash,
        expected_source_hash: &ContentHash,
    ) -> Result<u32> {
        if self.entries.len() >= MAX_COMPANIONS
            || bytes.len() > MAX_RETAINED_BYTES.saturating_sub(self.byte_count)
        {
            return Err(CompanionError::limit());
        }
        let next = self
            .next_id
            .checked_add(1)
            .ok_or_else(CompanionError::limit)?;
        let library = CemtXPathFunctions::from_companion_bytes(
            bytes,
            expected_content_hash,
            expected_source_hash,
        )?;
        let mut functions = NativeFunctionRegistry::default();
        // This host grants no additional resource/module-URL capabilities.
        library
            .install(
                &mut functions,
                Arc::new(ResolverRegistry::new()),
                Arc::new(ResolverPolicy::new()),
            )
            .map_err(CompanionError::invalid)?;
        self.entries.insert(next, (functions, bytes.len()));
        self.byte_count += bytes.len();
        self.next_id = next;
        Ok(next)
    }

    pub fn functions(&self, id: u32) -> Option<NativeFunctionRegistry> {
        self.entries
            .get(&id)
            .map(|(functions, _)| functions.clone())
    }

    pub fn dispose(&mut self, id: u32) -> bool {
        if let Some((_, bytes)) = self.entries.remove(&id) {
            self.byte_count -= bytes;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_bytes_and_exhausted_ids_are_checked_before_decode() {
        let hash = ContentHash::from_blake3(b"invalid");
        for mut host in [
            CemtXPathCompanions {
                byte_count: MAX_RETAINED_BYTES,
                ..Default::default()
            },
            CemtXPathCompanions {
                next_id: u32::MAX,
                ..Default::default()
            },
        ] {
            assert_eq!(
                host.import(b"invalid", &hash, &hash).unwrap_err().code,
                "cem.ql.xpath_companion_limit"
            );
            assert!(host.entries.is_empty());
        }
    }
}
