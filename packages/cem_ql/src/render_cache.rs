//! Opt-in bounded pure-render reuse. Only owned native output bytes enter the
//! existing artifact store; events, commands and resources require admission on
//! every call, and live/native references are deliberately never retained.
use crate::render::{
    read_dependencies::{self, RenderFreshness},
    RenderPlan, TemplateArtifact, TemplateData, TemplateFailure,
};
use cem_ml::{
    content_cache::{
        ArtifactContentType, CacheArtifact, CacheError, CacheMode, ContentCache,
        ContentCacheLimits, ContentCacheRuntime, ContentCacheStats, PolicyStamps,
    },
    dependency_identity::DependencyAdmission,
    operation_control::{ExecutionScopeId, OperationControl},
};
mod output;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderAdmission {
    pub owner: DependencyAdmission,
    pub freshness: RenderFreshness,
}
pub struct PureRenderRequest {
    runtime: ContentCacheRuntime,
    owner: DependencyAdmission,
}
pub struct CheckedPureRender {
    pub plan: RenderPlan,
    pub failure: Option<TemplateFailure>,
    pub hit: bool,
}
#[derive(Default)]
pub struct PureRenderCache {
    store: ContentCache,
    evaluations: u64,
}
impl PureRenderCache {
    pub fn new(limits: ContentCacheLimits) -> Result<Self, CacheError> {
        Ok(Self {
            store: ContentCache::with_limits(limits)?,
            evaluations: 0,
        })
    }
    pub fn begin(&self, owner: DependencyAdmission) -> PureRenderRequest {
        PureRenderRequest {
            runtime: self.store.runtime(),
            owner,
        }
    }
    pub fn reset(&mut self) -> Result<(), CacheError> {
        self.store.reset_runtime()
    }
    pub fn stats(&self) -> ContentCacheStats {
        self.store.stats()
    }
    pub fn evaluations(&self) -> u64 {
        self.evaluations
    }

    /// `admit` must execute accepted-event, command and resource admission even
    /// on a hit. `current` reads pinned metadata, without replaying those actions.
    /// Both must attest the supplied data snapshot; changes require a fresh call.
    /// The cache never fabricates freshness or treats opaque providers as stable.
    pub fn render(
        &mut self,
        artifact: &TemplateArtifact,
        data: &TemplateData,
        request: &PureRenderRequest,
        control: &OperationControl,
        scope: ExecutionScopeId,
        admit: impl FnOnce() -> Result<RenderAdmission, CacheError>,
        mut current: impl FnMut() -> Option<RenderAdmission>,
    ) -> Result<CheckedPureRender, CacheError> {
        active(control, scope)?;
        let policy = PolicyStamps::new("cem-pure-render/1");
        // Reject foreign/reset tickets before invoking host callbacks or queries.
        self.store.lease_by_content_type_for_runtime(
            &request.runtime,
            &ArtifactContentType::Other("cem-pure-render/ineligible".into()),
            CacheMode::Dev,
            &policy,
        )?;
        let before = current().ok_or_else(superseded)?;
        if !request.owner.is_current(&before.owner) {
            return Err(superseded());
        }
        let admitted = admit()?;
        if admitted != before {
            return Err(superseded());
        }
        check_current(request, &admitted, &mut current, control, scope)?;
        let effective = control
            .scope_tree()
            .scope(scope)
            .ok_or_else(superseded)?
            .effective_policy;
        let key = read_dependencies::reuse_identity(artifact, data, &admitted.freshness, effective)
            .and_then(|identity| {
                crate::eval::dependencies::bounded_identity(
                    b"cem.ql.pure-render-owner/1\n",
                    &(
                        &identity,
                        &request.owner.owner,
                        request.owner.runtime_generation,
                    ),
                )
                .ok()
            })
            .map(|hash| ArtifactContentType::Other(format!("cem-pure-render/1/{}", hash.hex)));
        if let Some(key) = &key {
            if let Some(lease) = self.store.lease_by_content_type_for_runtime(
                &request.runtime,
                key,
                CacheMode::Dev,
                &policy,
            )? {
                if let Some(plan) = output::decode(&lease.artifact(&policy)?.bytes, control, scope)
                {
                    check_current(request, &admitted, &mut current, control, scope)?;
                    return Ok(CheckedPureRender {
                        plan,
                        failure: None,
                        hit: true,
                    });
                }
            }
        }
        self.evaluations = self.evaluations.saturating_add(1);
        let rendered = read_dependencies::render_compiled_template_with_dependencies_and_control(
            artifact,
            data,
            &admitted.freshness,
            control,
            scope,
        );
        check_current(request, &admitted, &mut current, control, scope)?;
        if let Some(key) = key.filter(|_| rendered.dependencies.is_ok()) {
            if let Some(bytes) = output::encode(&rendered.output.plan, control, scope) {
                check_current(request, &admitted, &mut current, control, scope)?;
                let (artifact, sidecar) =
                    CacheArtifact::new(key, CacheMode::Dev, bytes, policy, None);
                if let Err(error) =
                    self.store
                        .insert_for_runtime(&request.runtime, artifact, sidecar)
                {
                    if error.code == "cem.cc.runtime_replaced" {
                        return Err(error);
                    }
                    // Capacity/encoding refusal preserves a successful ordinary render.
                }
            }
        }
        check_current(request, &admitted, &mut current, control, scope)?;
        Ok(CheckedPureRender {
            plan: rendered.output.plan,
            failure: rendered.output.failure,
            hit: false,
        })
    }
}
fn superseded() -> CacheError {
    CacheError {
        code: "cem.render.superseded",
        message: "render owner or provider snapshot changed during admission/evaluation".into(),
    }
}
fn active(control: &OperationControl, scope: ExecutionScopeId) -> Result<(), CacheError> {
    control.check_scope(scope).map_err(|e| CacheError {
        code: "cem.render.control",
        message: e.to_string(),
    })
}
fn check_current(
    request: &PureRenderRequest,
    admitted: &RenderAdmission,
    current: &mut impl FnMut() -> Option<RenderAdmission>,
    control: &OperationControl,
    scope: ExecutionScopeId,
) -> Result<(), CacheError> {
    active(control, scope)?;
    if current().as_ref() != Some(admitted) || !request.owner.is_current(&admitted.owner) {
        return Err(superseded());
    }
    Ok(())
}
