//! Stable resolver configuration identity; never includes process-local handles.
use super::{CemModuleUrlContext, CemModuleUrlSpecifierMap};

struct Fingerprint(blake3::Hasher);
impl Fingerprint {
    fn field(&mut self, value: &str) {
        self.count(value.len());
        self.0.update(value.as_bytes());
    }
    fn count(&mut self, value: usize) {
        self.0.update(&(value as u64).to_le_bytes());
    }
    fn optional(&mut self, value: Option<&str>) {
        self.count(usize::from(value.is_some()));
        if let Some(value) = value {
            self.field(value);
        }
    }
    fn mappings(&mut self, maps: &CemModuleUrlSpecifierMap) {
        for entries in [&maps.imports, &maps.resources] {
            self.count(entries.len());
            for (key, value) in entries {
                self.field(key);
                self.optional(value.target.as_deref());
                self.optional(value.content_type_hint.as_deref());
                self.optional(value.integrity.as_deref());
            }
        }
    }
}

impl CemModuleUrlContext {
    /// Fingerprint the full ordered context, including mapping and policy data.
    /// Stable context/frame identities must survive hydration; lookup handles need
    /// not. Version this domain when resolver semantics or encoded fields change.
    pub fn cache_identity(&self) -> String {
        let mut hash = Fingerprint(blake3::Hasher::new_derive_key(
            "cem module resolution context/1",
        ));
        hash.field(&self.identity);
        hash.field(&self.resolver_identity);
        hash.field(&self.resource_policy_stamp);
        hash.count(self.frames.len());
        for frame in &self.frames {
            hash.field(&frame.frame_id);
            hash.field(&frame.base_url);
            hash.optional(frame.module_map_base_url.as_deref());
            hash.mappings(&frame.specifiers);
            hash.count(frame.scopes.len());
            for scope in &frame.scopes {
                hash.field(&scope.prefix);
                hash.mappings(&scope.specifiers);
            }
            hash.count(usize::from(frame.allowed_schemes.is_some()));
            if let Some(schemes) = &frame.allowed_schemes {
                hash.count(schemes.len());
                for scheme in schemes {
                    hash.field(scheme);
                }
            }
        }
        format!("cem-context-1-{}", hash.0.finalize().to_hex())
    }
}
