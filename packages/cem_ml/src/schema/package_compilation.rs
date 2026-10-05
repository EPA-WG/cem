//! Opt-in package compilation at a consumer-selected document lifecycle stage.
//! Retain source arenas across unchanged reads, never evaluated reference results.
use super::document_model::{compile_document_model_from_document, SchemaDocumentModel};
use crate::{
    diagnostics::{Diagnostic, Severity},
    events::cem::CemEventNormalizer,
    parser::{
        builder::CemAstBuilder,
        tree::{CemTreeSemantics, RetainedCemTree},
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use std::{collections::BTreeMap, fmt::Debug, sync::Arc};

pub const SCHEMA_COMPILATION_FAILED: &str = "cem.schema_package.schema_compilation_failed";

#[derive(Debug, Clone)]
pub struct SchemaPackageCompilationRequest {
    pub package_id: String,
    pub manifest_uri: String,
    pub schema_uri: String,
    pub source: Arc<RetainedCemTree>,
}
/// The caller installs this explicitly when its runtime inputs are available.
/// Every invocation must compile against its own current context. A compiler
/// may return an incomplete model; publication still applies readiness gates.
pub trait SchemaPackageCompiler: Debug + Send + Sync {
    fn compile(
        &self,
        request: &SchemaPackageCompilationRequest,
    ) -> Result<SchemaDocumentModel, Vec<Diagnostic>>;
}
#[derive(Debug, Clone)]
struct RetainedSource {
    revision: blake3::Hash,
    tree: Arc<RetainedCemTree>,
}
#[derive(Debug, Clone, Default)]
pub struct SchemaPackageSourceRegistry {
    sources: BTreeMap<String, RetainedSource>,
}
impl SchemaPackageSourceRegistry {
    /// Latest valid retained source for this resolved URI, including an inactive
    /// candidate. Failed parses do not replace a previously valid source arena.
    pub fn get(&self, uri: &str) -> Option<&Arc<RetainedCemTree>> {
        self.sources.get(uri).map(|s| &s.tree)
    }
    pub(crate) fn retain(
        &mut self,
        uri: &str,
        source: &str,
    ) -> Result<Arc<RetainedCemTree>, Vec<Diagnostic>> {
        let revision = blake3::hash(source.as_bytes());
        if let Some(existing) = self.sources.get(uri).filter(|s| s.revision == revision) {
            return Ok(existing.tree.clone());
        }
        let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
            BytesSource::new(SourceId(1), source.as_bytes().to_vec()),
        )))
        .build();
        if document
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
        {
            return Err(document.diagnostics);
        }
        let tree = RetainedCemTree::new(document, uri, source, CemTreeSemantics::default(), None)
            .map_err(|reason| vec![compilation_failure(uri, reason)])?;
        self.sources.insert(
            uri.into(),
            RetainedSource {
                revision,
                tree: tree.clone(),
            },
        );
        Ok(tree)
    }
}
pub(crate) fn source_only_model(schema_uri: &str, source: &RetainedCemTree) -> SchemaDocumentModel {
    compile_document_model_from_document(schema_uri, source.ast())
}
pub fn compilation_failure(uri: &str, reason: impl Into<String>) -> Diagnostic {
    Diagnostic {
        code: SCHEMA_COMPILATION_FAILED.into(),
        severity: Severity::Error,
        uri: Some(uri.into()),
        message: reason.into(),
        ..Default::default()
    }
}
