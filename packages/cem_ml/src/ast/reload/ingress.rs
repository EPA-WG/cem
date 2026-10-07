//! Shared admission after verification. This boundary never reparses source or prepares runtime authority.
use super::{ReloadDependency, ReloadError, ReloadedReferenceDocument};
use crate::{
    parser::tree::{CemTreeRange, CemTreeSemantics, RetainedCemTree},
    query::QuerySourceOwner,
    run_config::ScopeConfig,
    schema::{
        document_model::{SchemaBehaviorEvaluator, SchemaDocumentModel},
        input_validation::InputValidationRequest,
        reference_policy::ReferenceScopePolicy,
    },
    source::SourceId,
    value::artifact::CemValueProvenance,
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, Clone)]
pub struct ReloadIngress {
    reloaded: Arc<ReloadedReferenceDocument>,
    source: Arc<RetainedCemTree>,
    primary_source: SourceId,
}
impl ReloadIngress {
    /// Primary source is an explicit manifest identity, never an inferred fallback.
    pub fn new(
        reloaded: Arc<ReloadedReferenceDocument>,
        primary_source: SourceId,
    ) -> Result<Self, ReloadError> {
        let primary = reloaded
            .sources
            .iter()
            .find(|s| s.source_id == primary_source)
            .ok_or(ReloadError::InvalidSource)?;
        let text = reloaded.source_text(primary_source)?.unwrap_or("");
        if reloaded
            .lexical
            .as_ref()
            .is_some_and(|capture| !Arc::ptr_eq(capture.document(), &reloaded.document))
        {
            return Err(ReloadError::InvalidMetadata);
        }
        let mut semantics = CemTreeSemantics::default();
        let sources: BTreeMap<_, _> = reloaded
            .sources
            .iter()
            .map(|s| (s.source_id.0, s))
            .collect();
        let indexes: BTreeMap<_, _> = reloaded
            .sources
            .iter()
            .filter_map(|s| {
                s.bytes.as_ref().map(|bytes| {
                    (
                        s.source_id.0,
                        crate::source::line_index::LineIndex::from_bytes_lossy(bytes),
                    )
                })
            })
            .collect();
        for (id, node) in reloaded.document.nodes.iter().enumerate() {
            let id = id as u32;
            if let Some(frame) = super::node_source(node).frames.first() {
                let manifest = sources
                    .get(&frame.source_id.0)
                    .ok_or(ReloadError::InvalidSource)?;
                let span = match &frame.span {
                    crate::source_map::FrameSpan::Single(span) => Some(span),
                    crate::source_map::FrameSpan::Multi(spans) => spans.first(),
                };
                let coordinate = span.and_then(|s| {
                    indexes
                        .get(&frame.source_id.0)
                        .map(|index| index.project(s.start))
                });
                if coordinate.is_none() {
                    semantics.unknown_source_lines.insert(id);
                }
                if let Some(span) = span {
                    semantics.ranges.insert(
                        id,
                        CemTreeRange {
                            line: coordinate.map_or(0, |p| p.line),
                            column: coordinate.map_or(0, |p| p.column),
                            offset: span.start,
                            length: span.len as u64,
                        },
                    );
                }
                semantics.provenance.insert(
                    id,
                    CemValueProvenance {
                        source_uri: Some(manifest.uri.clone()),
                        line_number: coordinate.map(|p| p.line),
                        ..Default::default()
                    },
                );
            } else {
                semantics.unknown_source_lines.insert(id);
            }
        }
        let source = RetainedCemTree::from_shared(
            reloaded.document.clone(),
            primary.uri.clone(),
            text,
            semantics,
            None,
        )
        .map_err(ReloadError::InvalidPayload)?;
        Ok(Self {
            reloaded,
            source,
            primary_source,
        })
    }
    pub fn reloaded(&self) -> &Arc<ReloadedReferenceDocument> {
        &self.reloaded
    }
    pub fn source(&self) -> &Arc<RetainedCemTree> {
        &self.source
    }
    pub fn primary_source(&self) -> SourceId {
        self.primary_source
    }
    /// Inert structural inspection does not require lexical metadata. Consumers
    /// evaluating authored declarations must explicitly require it first.
    pub fn query_source_owner(&self) -> QuerySourceOwner {
        QuerySourceOwner::Cem {
            source: self.source.clone(),
            lexical_scopes: self.reloaded.lexical.clone(),
        }
    }
    /// A missing sidecar is a retryable dependency, never inherited/default capture.
    /// The caller supplies a fresh model, policy, contexts, loaders and grants.
    pub fn validation_request<'a>(
        &self,
        model: &'a SchemaDocumentModel,
        root_scope: &'a ScopeConfig,
        policy: ReferenceScopePolicy,
        behavior_evaluator: Option<&'a dyn SchemaBehaviorEvaluator>,
    ) -> Result<InputValidationRequest<'a>, ReloadDependency> {
        let lexical = self.reloaded.require_lexical()?.clone();
        Ok(InputValidationRequest {
            source: self.source.clone(),
            lexical_scopes: Some(lexical),
            model,
            root_scope,
            policy,
            behavior_evaluator,
        })
    }
}
