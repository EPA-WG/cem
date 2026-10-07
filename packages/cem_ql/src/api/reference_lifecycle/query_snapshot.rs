//! Immutable, per-execution completed names over selected native ready forests.
use super::*;
use crate::{
    api::{evaluate_expression, ExpressionError, ExpressionEvaluation},
    eval::ItemStream,
    namespace_names::NamespaceQueryTree,
    types::{NodeKind, Type},
};
#[derive(Debug)]
pub struct ReferenceQuerySnapshot {
    source: RetainedReferenceSource,
    view: Option<Arc<NamespaceQueryTree>>,
    report: ReferenceConsumerReport,
}
impl ReferenceQuerySnapshot {
    pub(super) fn from_lifecycle(
        source: RetainedReferenceSource,
        names: &NamespaceLifecycleSnapshot,
    ) -> Result<Self, String> {
        let mut report = ReferenceConsumerReport::pending();
        ReferenceValidationSession::namespace_dependencies(&mut report, names, &source);
        report.complete = names.is_complete();
        report.placements = names.ready_roots.len();
        Self::from_completion(source, names.completion.clone(), report)
    }
    pub(super) fn from_completion(
        source: RetainedReferenceSource,
        completion: Arc<cem_ml::schema::namespace_references::NamespaceNameCompletion>,
        report: ReferenceConsumerReport,
    ) -> Result<Self, String> {
        let view = NamespaceQueryTree::new(source.ingress().source().clone(), completion)?;
        Ok(Self {
            source,
            view: Some(view),
            report,
        })
    }
    pub fn report(&self) -> &ReferenceConsumerReport {
        &self.report
    }
    pub fn source(&self) -> &RetainedReferenceSource {
        &self.source
    }
    pub fn evaluate(
        &self,
        expression: &str,
        context: &StandaloneExpressionContext,
    ) -> Result<ExpressionEvaluation, ExpressionError> {
        let input =
            ItemStream::from_items(self.view.as_ref().map(|v| v.roots()).unwrap_or_default());
        evaluate_expression(
            expression,
            &context
                .clone()
                .with_input(input, Type::Node(NodeKind::Node)),
        )
    }
}
impl ReferenceValidationSession {
    /// Namespace preparation is independent of schema validation readiness.
    /// Querying the snapshot never evaluates authored descendant references.
    pub fn query_snapshot(&self) -> Result<ReferenceQuerySnapshot, String> {
        let source = self.sources[0].source.clone();
        let mut report = ReferenceConsumerReport::pending();
        let Ok(capture) = source.require_lexical() else {
            report.dependencies.push(ReferenceConsumerDependency {
                kind: ReferenceConsumerDependencyKind::MissingLexicalMetadata,
                reason: "Verified lexical capture is required".into(),
                source_uri: source.ingress().source().source_uri().into(),
                node_id: None,
                source_map: None,
            });
            return Ok(ReferenceQuerySnapshot {
                source,
                view: None,
                report,
            });
        };
        let mut policy = self.sources[0]
            .policy
            .clone()
            .unwrap_or(ReferenceScopePolicy::schema_defaults().map_err(|e| format!("{e:?}"))?);
        if let Some(limits) = self.sources[0].limits {
            policy.limits = limits;
        }
        let mut host = self.host(&policy)?;
        let (snapshot, ()) = host
            .with_namespace_lifecycle(
                capture.clone(),
                &roots(&source),
                policy.limits,
                |node, _, _, _| (self.context_for(node), Default::default()),
                |_, _| (),
            )
            .map_err(|e| format!("{e:?}"))?;
        report.diagnostics.extend(
            source
                .ingress()
                .source()
                .ast()
                .diagnostics
                .iter()
                .chain(capture.diagnostics())
                .cloned()
                .map(|mut diagnostic| {
                    diagnostic
                        .uri
                        .get_or_insert_with(|| source.ingress().source().source_uri().into());
                    diagnostic
                }),
        );
        Self::namespace_dependencies(&mut report, &snapshot, &source);
        report.complete = snapshot.is_complete();
        report.placements = snapshot.ready_roots.len();
        report.failed |= report
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation());
        let view = NamespaceQueryTree::new(source.ingress().source().clone(), snapshot.completion)?;
        Ok(ReferenceQuerySnapshot {
            source,
            view: Some(view),
            report,
        })
    }
}
