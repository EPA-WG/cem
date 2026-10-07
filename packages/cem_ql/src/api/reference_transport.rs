//! Executable source bundles retain AST and capture; materialized CEMV is a separate boundary.
use super::{
    evaluate_expression, ExpressionError, ExpressionEvaluation, StandaloneExpressionContext,
};
use crate::{
    eval::{imported_cem_tree, ItemStream},
    types::{NodeKind, Type},
};
use cem_ml::{
    ast::reload::{
        ReferenceReloadBundle, ReloadDependency, ReloadError, ReloadIngress, ReloadLimits,
        ReloadSource,
    },
    import::import_bytes_with_lexical_scopes,
    schema::{machine::LexicallyScopedDocument, vocab::CompiledSchema},
    source::SourceId,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RetainedReferenceSource {
    ingress: ReloadIngress,
    bundle: Arc<ReferenceReloadBundle>,
}
impl RetainedReferenceSource {
    pub fn reload(
        bytes: &[u8],
        primary_source: u32,
        limits: ReloadLimits,
    ) -> Result<Self, ReloadError> {
        let (bundle, document) = ReferenceReloadBundle::decode_with_document(bytes, limits)?;
        let ingress = ReloadIngress::new(Arc::new(document), SourceId(primary_source))?;
        Ok(Self {
            ingress,
            bundle: Arc::new(bundle),
        })
    }
    /// Explicit source export preparation, using the normal lexical importer once.
    /// It does not run references, activate schemas or serialize runtime contexts.
    pub fn parse(
        bytes: &[u8],
        content_type: &str,
        uri: &str,
        limits: ReloadLimits,
    ) -> Result<Self, ReloadError> {
        if bytes.len() > limits.max_bytes {
            return Err(ReloadError::LimitExceeded);
        }
        let imported =
            import_bytes_with_lexical_scopes(bytes, content_type, uri, CompiledSchema::cem_core())
                .map_err(ReloadError::InvalidPayload)?;
        Self::from_capture(imported.captured, uri, bytes, limits)
    }
    pub fn from_capture(
        capture: Arc<LexicallyScopedDocument>,
        uri: &str,
        bytes: &[u8],
        limits: ReloadLimits,
    ) -> Result<Self, ReloadError> {
        let bundle = ReferenceReloadBundle::export(
            &capture,
            vec![
                ReloadSource::new(SourceId(0), uri, bytes, true),
                ReloadSource::new(SourceId(1), uri, bytes, true),
            ],
            limits,
        )?;
        let reloaded = cem_ml::ast::reload::ReloadedReferenceDocument {
            document: capture.document().clone(),
            lexical: Some(capture),
            sources: bundle.sources.clone(),
        };
        let ingress = ReloadIngress::new(Arc::new(reloaded), SourceId(1))?;
        Ok(Self {
            ingress,
            bundle: Arc::new(bundle),
        })
    }
    pub fn ingress(&self) -> &ReloadIngress {
        &self.ingress
    }
    pub fn require_lexical(&self) -> Result<&Arc<LexicallyScopedDocument>, ReloadDependency> {
        self.ingress.reloaded().require_lexical()
    }
    pub fn export_bundle(&self, limits: ReloadLimits) -> Result<Vec<u8>, ReloadError> {
        self.bundle.encode(limits)
    }
    /// Atomic passive attachment. Existing query/session clones keep their view.
    pub fn attach_bundle(&mut self, bytes: &[u8], limits: ReloadLimits) -> Result<(), ReloadError> {
        let (bundle, reloaded) =
            ReferenceReloadBundle::attach_to(bytes, &self.bundle, self.ingress.reloaded(), limits)?;
        let ingress = ReloadIngress::new(Arc::new(reloaded), self.ingress.primary_source())?;
        self.ingress = ingress;
        self.bundle = Arc::new(bundle);
        Ok(())
    }
    /// Runtime bindings/capabilities belong to this execution, not the saved source.
    /// Authored source references stay inert until an explicit lifecycle consumer.
    pub fn evaluate(
        &self,
        expression: &str,
        context: &StandaloneExpressionContext,
    ) -> Result<ExpressionEvaluation, ExpressionError> {
        let context = context.clone().with_input(
            ItemStream::once(imported_cem_tree(self.ingress.source().clone())),
            Type::Node(NodeKind::Node),
        );
        evaluate_expression(expression, &context)
    }
}
