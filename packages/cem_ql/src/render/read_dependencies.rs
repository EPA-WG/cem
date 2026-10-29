//! Whole-render native read receipts with conservative provider freshness.
//! All provider axes are required: local bindings, focus and synthesized datadom
//! derive from this complete snapshot, never from a local name or DOM identity.
use super::*;
use crate::eval::dependencies::{
    self as query_reads, EvaluationDependencies, EvaluationFreshness, EvaluationTrackingError,
};
use cem_ml::content_cache::ContentHash;
use cem_ml::dependency_identity::{DependencyInput, DependencyVersion, RENDER_DEPENDENCY_INPUTS};

pub const MAX_RENDER_QUERIES: usize = 1024;
pub const MAX_RENDER_READ_NAMES: usize = 1024;

/// The native host attests pure reads and complete value/provenance/absence/access
/// freshness for every axis. Environment covers hidden providers, scope/resolver
/// configuration and input-schema policy. Unknown/pending inputs remain opaque.
/// Providers must remain pinned/immutable for the render and be revalidated at
/// publication. Binding, focus or datadom changes must advance their axes even when
/// they render equal output. No defaults claim provider stability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderFreshness {
    pub environment: DependencyVersion,
    pub inputs: BTreeMap<DependencyInput, DependencyVersion>,
}
impl RenderFreshness {
    /// Bridge the existing native provider/closure contract. Unknown freshness,
    /// incomplete closures and compilation-only snapshots cannot attest a render.
    /// The producer still owns the pure-read guarantee; key equality grants no
    /// operation admission. No provider values or source buffers are retained.
    pub fn from_snapshot(
        snapshot: &cem_ml::dependency_identity::DependencySnapshot,
    ) -> Result<Self, cem_ml::dependency_identity::DependencyIdentityError> {
        use cem_ml::dependency_identity::{DependencyIdentityError, DependencyLayer};
        if snapshot.layer != DependencyLayer::RenderProjection {
            return Err(DependencyIdentityError::InvalidContext);
        }
        let identity = snapshot.identity()?;
        Ok(Self {
            environment: DependencyVersion::Content(identity.hash),
            inputs: snapshot.inputs.clone(),
        })
    }
}
impl Default for RenderFreshness {
    fn default() -> Self {
        Self {
            environment: DependencyVersion::Opaque,
            inputs: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTrackingError {
    Input(EvaluationTrackingError),
    MissingInput,
    OpaqueRuntimeScope,
    OpaqueTemplateCalls,
    UnsuccessfulRender,
    Bounds,
}
impl From<EvaluationTrackingError> for RenderTrackingError {
    fn from(error: EvaluationTrackingError) -> Self {
        Self::Input(error)
    }
}

pub struct TemplateDependencyRender {
    pub output: TemplateCallResult,
    pub dependencies: Result<RenderDependencies, RenderTrackingError>,
}

/// Bounded metadata only: no query trees, values, outputs, provider callbacks,
/// captured hooks, DOM/native handles or preceding render owners are retained.
#[derive(Debug, Clone)]
pub struct RenderDependencies {
    template: ContentHash,
    environment: DependencyVersion,
    inputs: BTreeMap<DependencyInput, DependencyVersion>,
    policy: ScopePolicy,
    queries: BTreeSet<ContentHash>,
    bindings: BTreeSet<String>,
    evaluations: u64,
    reads_focus: bool,
}
impl RenderDependencies {
    pub fn binding_names(&self) -> impl Iterator<Item = &str> {
        self.bindings.iter().map(String::as_str)
    }
    pub fn reads_current_item(&self) -> bool {
        self.reads_focus
    }
    pub fn distinct_query_count(&self) -> usize {
        self.queries.len()
    }
    pub fn evaluation_count(&self) -> u64 {
        self.evaluations
    }

    /// Necessary input check only. All axes participate, even when a query's
    /// taken path omits a binding. Independent operation/resource/publication
    /// admission and current runtime ownership are still required for any reuse.
    pub fn matches(
        &self,
        artifact: &TemplateArtifact,
        data: &TemplateData,
        freshness: &RenderFreshness,
    ) -> bool {
        self.matches_with_policy(artifact, data, freshness, default_policy())
    }
    pub fn matches_with_policy(
        &self,
        artifact: &TemplateArtifact,
        data: &TemplateData,
        freshness: &RenderFreshness,
        policy: ScopePolicy,
    ) -> bool {
        ordinary_scope(data)
            && self.policy == policy
            && self.environment == freshness.environment
            && self.inputs == freshness.inputs
            && template_identity(artifact, data).is_ok_and(|identity| self.template == identity)
    }
}

pub fn render_compiled_template_with_dependencies(
    artifact: &TemplateArtifact,
    data: &TemplateData,
    freshness: &RenderFreshness,
) -> TemplateDependencyRender {
    render_with_dependencies(artifact, data, freshness, None, None, false)
}
pub fn render_compiled_template_with_dependencies_and_control(
    artifact: &TemplateArtifact,
    data: &TemplateData,
    freshness: &RenderFreshness,
    control: &OperationControl,
    scope: ExecutionScopeId,
) -> TemplateDependencyRender {
    render_with_dependencies(
        artifact,
        data,
        freshness,
        Some((control, scope)),
        None,
        false,
    )
}
/// Custom handlers execute normally but remain opaque; a pure/fresh query stamp
/// cannot attest an arbitrary callback's effects or its nested render closure.
pub fn render_compiled_template_with_calls_and_dependencies(
    artifact: &TemplateArtifact,
    data: &TemplateData,
    freshness: &RenderFreshness,
    control: Option<(&OperationControl, ExecutionScopeId)>,
    calls: &dyn TemplateCallHandler,
    protected: bool,
) -> TemplateDependencyRender {
    render_with_dependencies(artifact, data, freshness, control, Some(calls), protected)
}
fn render_with_dependencies(
    artifact: &TemplateArtifact,
    data: &TemplateData,
    freshness: &RenderFreshness,
    control: Option<(&OperationControl, ExecutionScopeId)>,
    calls: Option<&dyn TemplateCallHandler>,
    protected: bool,
) -> TemplateDependencyRender {
    let policy = control
        .and_then(|(c, s)| c.scope_tree().scope(s).map(|s| s.effective_policy))
        .unwrap_or_else(default_policy);
    let mut tracker = RenderReadTracker::new(artifact, data, freshness, policy, calls.is_some());
    let output = render_compiled_template_internal(
        artifact,
        data,
        control,
        calls,
        protected,
        Some(&mut tracker),
    );
    let dependencies = tracker.finish(&output);
    TemplateDependencyRender {
        output,
        dependencies,
    }
}
fn default_policy() -> ScopePolicy {
    ScopePolicy::host_root().with_queue_size(128)
}
fn ordinary_scope(data: &TemplateData) -> bool {
    #[cfg(any(not(target_arch = "wasm32"), feature = "component-runtime"))]
    if data.selected_registry.is_some() {
        return false;
    }
    data.expression_scope.scopes.is_empty()
        && data.expression_scope.active.is_empty()
        && data.expression_scope.call_depth == 0
        && matches!(
            data.expression_scope.target,
            hooks::ExpressionTarget::Content
        )
}

pub(super) struct RenderReadTracker {
    receipt: Option<RenderDependencies>,
    stamp: Option<DependencyVersion>,
    error: Option<RenderTrackingError>,
    remaining_bytes: usize,
}
impl RenderReadTracker {
    fn new(
        artifact: &TemplateArtifact,
        data: &TemplateData,
        freshness: &RenderFreshness,
        policy: ScopePolicy,
        calls: bool,
    ) -> Self {
        let mut tracker = Self {
            receipt: None,
            stamp: None,
            error: None,
            remaining_bytes: query_reads::MAX_EVALUATION_RECEIPT_BYTES,
        };
        let prepared = (|| {
            if calls {
                return Err(RenderTrackingError::OpaqueTemplateCalls);
            }
            if !ordinary_scope(data) {
                return Err(RenderTrackingError::OpaqueRuntimeScope);
            }
            query_reads::validate_stamp(&freshness.environment, &mut tracker.remaining_bytes)?;
            for axis in RENDER_DEPENDENCY_INPUTS {
                let version = freshness
                    .inputs
                    .get(axis)
                    .ok_or(RenderTrackingError::MissingInput)?;
                query_reads::validate_stamp(version, &mut tracker.remaining_bytes)?;
            }
            let template = template_identity(artifact, data)?;
            // Local/derived reads are attested only under the full template and
            // provider snapshot. This is deliberately not selective invalidation.
            let stamp = query_reads::bounded_identity(
                b"cem.ql.render-inputs/1\n",
                &(&template, &freshness.environment, &freshness.inputs, policy),
            )?;
            Ok((
                RenderDependencies {
                    template,
                    environment: freshness.environment.clone(),
                    inputs: freshness.inputs.clone(),
                    policy,
                    queries: BTreeSet::new(),
                    bindings: BTreeSet::new(),
                    evaluations: 0,
                    reads_focus: false,
                },
                DependencyVersion::Content(stamp),
            ))
        })();
        match prepared {
            Ok((receipt, stamp)) => {
                tracker.receipt = Some(receipt);
                tracker.stamp = Some(stamp);
            }
            Err(error) => tracker.fail(error),
        }
        tracker
    }
    fn fail(&mut self, error: RenderTrackingError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
        self.receipt = None;
        self.stamp = None;
    }
    pub(super) fn query_freshness(&mut self, query: &CompiledQuery) -> Option<EvaluationFreshness> {
        let stamp = self.stamp.as_ref()?;
        if query.policy_bindings.len() > query_reads::MAX_EVALUATION_READS
            || query
                .policy_bindings
                .values()
                .try_fold(0usize, |n, name| n.checked_add(name.len()))
                .is_none_or(|n| n > query_reads::MAX_EVALUATION_RECEIPT_BYTES)
        {
            self.fail(RenderTrackingError::Bounds);
            return None;
        }
        // A loop or named-template call can change local values while retaining
        // the same query and external snapshot. Fence each invocation; these
        // transient query receipts are only aggregated, never cached separately.
        let ordinal = self.receipt.as_ref()?.evaluations;
        let stamp = match query_reads::bounded_identity(
            b"cem.ql.render-query-inputs/1\n",
            &(stamp, ordinal),
        ) {
            Ok(hash) => DependencyVersion::Content(hash),
            Err(error) => {
                self.fail(error.into());
                return None;
            }
        };
        Some(EvaluationFreshness {
            environment: stamp.clone(),
            current_item: stamp.clone(),
            bindings: query
                .policy_bindings
                .values()
                .map(|name| (name.clone(), stamp.clone()))
                .collect(),
        })
    }
    pub(super) fn observe(
        &mut self,
        receipt: Result<EvaluationDependencies, EvaluationTrackingError>,
    ) {
        let receipt = match receipt {
            Ok(value) => value,
            Err(error) => {
                self.fail(error.into());
                return;
            }
        };
        let Some(aggregate) = &mut self.receipt else {
            return;
        };
        aggregate.evaluations = aggregate.evaluations.saturating_add(1);
        aggregate.reads_focus |= receipt.reads_current_item();
        let query = receipt.query_identity();
        if !aggregate.queries.contains(query) {
            if aggregate.queries.len() == MAX_RENDER_QUERIES {
                self.fail(RenderTrackingError::Bounds);
                return;
            }
            let bytes = query.scheme.len() + query.hex.len();
            let Some(remaining) = self.remaining_bytes.checked_sub(bytes) else {
                self.fail(RenderTrackingError::Bounds);
                return;
            };
            self.remaining_bytes = remaining;
            aggregate.queries.insert(query.clone());
        }
        for name in receipt.binding_names() {
            if !aggregate.bindings.contains(name) {
                if aggregate.bindings.len() == MAX_RENDER_READ_NAMES {
                    self.fail(RenderTrackingError::Bounds);
                    return;
                }
                let Some(remaining) = self.remaining_bytes.checked_sub(name.len()) else {
                    self.fail(RenderTrackingError::Bounds);
                    return;
                };
                self.remaining_bytes = remaining;
                aggregate.bindings.insert(name.to_owned());
            }
        }
    }
    fn finish(
        self,
        output: &TemplateCallResult,
    ) -> Result<RenderDependencies, RenderTrackingError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        if output.failure.is_some() || !output.plan.diagnostics.is_empty() {
            return Err(RenderTrackingError::UnsuccessfulRender);
        }
        self.receipt.ok_or(RenderTrackingError::UnsuccessfulRender)
    }
}

// Original native template nodes contain all embedded query IR and provenance;
// contract metadata affects non-query construction and receiver validation too.
fn template_identity(
    artifact: &TemplateArtifact,
    data: &TemplateData,
) -> Result<ContentHash, RenderTrackingError> {
    Ok(query_reads::bounded_identity(
        b"cem.ql.render-reads/1\n",
        &(
            cem_ml::VERSION,
            crate::VERSION,
            &artifact.nodes,
            &artifact.stylesheets,
            &artifact.module_map,
            &artifact.diagnostics,
            &data.value_types,
            &data.attribute_contracts,
            &data.input_attribute_contracts,
        ),
    )?)
}

pub(super) fn evaluate_query<'a>(
    query: &'a CompiledQuery,
    context: &'a EvaluationContext,
    freshness: Option<&EvaluationFreshness>,
    control: &OperationControl,
    scope: ExecutionScopeId,
    protected: bool,
    host: Option<&'a mut dyn crate::native::TemplateQueryHost>,
) -> (
    ItemStream,
    Option<Result<EvaluationDependencies, EvaluationTrackingError>>,
) {
    if let Some(freshness) = freshness {
        let (stream, receipt) = query_reads::evaluate_tracked_internal(
            query, context, freshness, control, scope, protected, host,
        );
        (stream, Some(receipt))
    } else {
        (
            crate::eval::Evaluator::evaluate_internal(
                query, context, control, scope, protected, host,
            ),
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inherited_expression_scope_remains_opaque() {
        let mut data = TemplateData::default();
        data.expression_scope.call_depth = 1;
        let artifact = compile_template("{p | static}", &Default::default());
        let freshness = RenderFreshness {
            environment: DependencyVersion::Content(ContentHash::from_blake3(b"environment")),
            inputs: RENDER_DEPENDENCY_INPUTS
                .iter()
                .map(|axis| {
                    (
                        *axis,
                        DependencyVersion::Content(ContentHash::from_blake3(b"input")),
                    )
                })
                .collect(),
        };
        let tracked = render_compiled_template_with_dependencies(&artifact, &data, &freshness);
        assert!(matches!(
            tracked.dependencies,
            Err(RenderTrackingError::OpaqueRuntimeScope)
        ));
        assert_eq!(render_plan_to_html(&tracked.output.plan), "<p>static</p>");
    }
}

// Population requires a successful full render receipt. This key only allows a
// previously proven entry to be considered under the identical complete inputs.
pub(crate) fn reuse_identity(artifact: &TemplateArtifact, data: &TemplateData, freshness: &RenderFreshness, policy: ScopePolicy) -> Option<ContentHash> {
    let tracker = RenderReadTracker::new(artifact, data, freshness, policy, false);
    match tracker.stamp? { DependencyVersion::Content(hash) => Some(hash), _ => None }
}
