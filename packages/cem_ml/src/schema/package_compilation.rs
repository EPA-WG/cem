//! Opt-in package compilation at a consumer-selected document lifecycle stage.
//! Retain source arenas across unchanged reads, never evaluated reference results.
use super::{
    document_model::{compile_document_model_from_document, SchemaDocumentModel},
    machine::{CemSchemaMachine, LexicallyScopedDocument},
    vocab::CompiledSchema,
};
use crate::{
    diagnostics::{Diagnostic, Severity},
    events::cem::CemEventNormalizer,
    parser::tree::{CemTreeSemantics, RetainedCemTree},
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
    /// Saved source-position bindings from the same original parser allocation.
    /// Runtime inputs, readiness and scope grants remain lifecycle supplied.
    pub lexical_scopes: Arc<LexicallyScopedDocument>,
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
    lexical_scopes: Arc<LexicallyScopedDocument>,
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
    /// Captured bindings and machine diagnostics paired with the retained source.
    /// Unchanged reads share metadata; runtime selections are never cached here.
    pub fn get_lexical_scopes(&self, uri: &str) -> Option<&Arc<LexicallyScopedDocument>> {
        self.sources.get(uri).map(|source| &source.lexical_scopes)
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
        let lexical_scopes = Arc::new(
            CemSchemaMachine::new(
                CompiledSchema::cem_core(),
                CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                    SourceId(1),
                    source.as_bytes().to_vec(),
                ))),
            )
            .build_with_lexical_scopes(),
        );
        let document = lexical_scopes.document();
        if document
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
        {
            return Err(document.diagnostics.clone());
        }
        let tree = RetainedCemTree::from_shared(
            document.clone(),
            uri,
            source,
            CemTreeSemantics::default(),
            None,
        )
        .map_err(|reason| vec![compilation_failure(uri, reason)])?;
        self.sources.insert(
            uri.into(),
            RetainedSource {
                revision,
                tree: tree.clone(),
                lexical_scopes,
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
