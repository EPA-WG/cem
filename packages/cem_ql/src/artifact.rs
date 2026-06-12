//! Compiled CEM-QL artifact shell.

use cem_ml::content_cache::ContentHash;

use crate::ir::deserialize::IrDeserializer;
use crate::ir::serialize::IrSerializer;
use crate::ir::CompiledQuery;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledArtifact {
    pub format: QueryArtifactFormat,
    pub content_hash: ContentHash,
    pub bytes: Vec<u8>,
}

impl CompiledArtifact {
    pub fn from_query(query: &CompiledQuery) -> Self {
        let bytes = IrSerializer::serialize(query);
        let content_hash = ContentHash::from_blake3(&bytes);
        Self {
            format: QueryArtifactFormat::CemQlIrV1,
            content_hash,
            bytes,
        }
    }

    pub fn reload(&self) -> Result<CompiledQuery, String> {
        if self.format != QueryArtifactFormat::CemQlIrV1 {
            return Err("unsupported CEM-QL artifact format".to_owned());
        }
        let actual = ContentHash::from_blake3(&self.bytes);
        if actual != self.content_hash {
            return Err("compiled artifact hash mismatch".to_owned());
        }
        IrDeserializer::deserialize(&self.bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryArtifactFormat {
    CemQlIrV1,
}
