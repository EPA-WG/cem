//! Compiled CEM-QL artifact shell.

use cem_ml::schema::compiler::ContentHash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledArtifact {
    pub format: QueryArtifactFormat,
    pub content_hash: ContentHash,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryArtifactFormat {
    CemQlIrV1,
}
