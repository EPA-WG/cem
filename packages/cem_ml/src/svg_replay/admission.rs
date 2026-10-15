//! Native origin-admission lifecycle. There is deliberately no production
//! provider implementation or public admission entry point. Candidates carry no
//! browser authority; the private protocol is exercised by test-only providers.
#![allow(dead_code)] // Private admission protocol awaits a verified browser adapter.
use super::{SvgReplayAuthorPlan, SvgReplayRoute};
use crate::{
    import::svg_presentation::RetainedSvgPresentationAttribute, parser::tree::RetainedCemTree,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Weak,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionState {
    Candidate,
    Admitted,
    Revoked,
}
struct Owner {
    plan_identity: Arc<()>,
    document: Arc<RetainedCemTree>,
    generation: u64,
    live: AtomicBool,
}
/// One native preparation lifetime. Replacement/recapture needs a new owner;
/// matching generation numbers alone cannot transfer a lease between owners.
pub struct AdmissionOwner {
    inner: Arc<Owner>,
}
impl AdmissionOwner {
    pub fn new(plan: &SvgReplayAuthorPlan, generation: u64) -> Result<Self, String> {
        if generation == 0 {
            return Err("nonzero worker generation required".into());
        }
        let document = plan
            .plan
            .routes()
            .first()
            .ok_or("live author plan required")?
            .consumer
            .document()
            .clone();
        Ok(Self {
            inner: Arc::new(Owner {
                plan_identity: plan.identity.clone(),
                document,
                generation,
                live: AtomicBool::new(true),
            }),
        })
    }
    pub fn candidate(&self, plan: SvgReplayAuthorPlan) -> Result<AdmissionLease, String> {
        if !self.inner.live.load(Ordering::Acquire)
            || !Arc::ptr_eq(&self.inner.plan_identity, &plan.identity)
            || plan.plan.routes().is_empty()
            || !Arc::ptr_eq(
                &self.inner.document,
                plan.plan.routes()[0].consumer.document(),
            )
        {
            return Err("live matching preparation owner required".into());
        }
        Ok(AdmissionLease {
            owner: Arc::downgrade(&self.inner),
            plan: Some(plan),
            state: AdmissionState::Candidate,
            provider: None,
        })
    }
    /// Call before base/policy, document or worker-generation changes. Terminal.
    pub fn invalidate(&self) {
        self.inner.live.store(false, Ordering::Release);
    }
}
impl Drop for AdmissionOwner {
    fn drop(&mut self) {
        self.invalidate();
    }
}

/// Non-cloneable and non-serializable. No routes, receipts or write grants escape.
/// Production callers cannot invoke admission with their own provider:
/// ```compile_fail
/// use cem_ml::svg_replay::admission::AdmissionLease;
/// fn forge(lease: &mut AdmissionLease) { lease.admit(&mut ()); }
/// ```
pub struct AdmissionLease {
    owner: Weak<Owner>,
    plan: Option<SvgReplayAuthorPlan>,
    state: AdmissionState,
    provider: Option<ProviderStamp>,
}
impl AdmissionLease {
    pub fn state(&self) -> AdmissionState {
        if self.state == AdmissionState::Revoked || self.live_owner().is_err() {
            AdmissionState::Revoked
        } else {
            self.state
        }
    }
    pub fn revoke(&mut self) {
        self.state = AdmissionState::Revoked;
        self.provider = None;
        self.plan = None;
    }
    pub fn blockers(&self) -> Vec<&'static str> {
        let mut blockers = vec![
            "svg-replay-alias-installation-required",
            "svg-resource-browser-response-identity-required",
            "svg-resource-consumption-required",
            "svg-resource-fragment-target-required",
            "svg-resource-transitive-dependencies-required",
            "general-runtime-resource-coverage-required",
        ];
        if self.state() != AdmissionState::Admitted {
            blockers.push("svg-replay-browser-origin-proof-required");
        }
        blockers
    }
    fn live_owner(&self) -> Result<Arc<Owner>, String> {
        let owner = self
            .owner
            .upgrade()
            .ok_or("origin preparation owner disposed")?;
        if !owner.live.load(Ordering::Acquire)
            || self
                .plan
                .as_ref()
                .is_none_or(|p| p.plan.routes().is_empty())
        {
            return Err("origin preparation revoked or cancelled".into());
        }
        Ok(owner)
    }
    fn admit(&mut self, provider: &mut impl OriginProvider) -> Result<(), String> {
        let result = (|| {
            if self.state != AdmissionState::Candidate {
                return Err("origin admission requires candidate".into());
            }
            let stamp = self.probe(provider)?;
            self.provider = Some(stamp);
            self.state = AdmissionState::Admitted;
            Ok(())
        })();
        if result.is_err() {
            self.revoke();
        }
        result
    }
    fn validate(&mut self, provider: &mut impl OriginProvider) -> Result<(), String> {
        let result = (|| {
            if self.state != AdmissionState::Admitted {
                return Err("origin read requires admission".into());
            }
            self.probe(provider)?;
            Ok(())
        })();
        if result.is_err() {
            self.revoke();
        }
        result
    }
    fn probe(&self, provider: &mut impl OriginProvider) -> Result<ProviderStamp, String> {
        let owner = self.live_owner()?;
        provider.preparation_current()?;
        let before = provider.snapshot()?;
        before.supported(owner.generation)?;
        if !Weak::ptr_eq(&before.owner, &self.owner) {
            return Err("foreign origin preparation owner".into());
        }
        if self.provider.as_ref().is_some_and(|p| !p.same(&before)) {
            return Err("origin provider identity or revision changed".into());
        }
        let plan = self.plan.as_ref().ok_or("origin author plan missing")?;
        for (slot, route) in plan.plan.routes().iter().enumerate() {
            let winner = provider.winner(route)?;
            let a = &route.consumer;
            let b = &winner.source;
            if !winner.presentation
                || !Arc::ptr_eq(a.document(), b.document())
                || !Arc::ptr_eq(a.css(), b.css())
                || a.owner() != b.owner()
                || a.attribute() != b.attribute()
            {
                return Err("browser winner is not the retained presentation source".into());
            }
            let property = &a
                .document()
                .node(a.attribute())
                .unwrap()
                .name
                .as_ref()
                .unwrap()
                .local_name;
            let (_, _, url) =
                crate::svg_resource_acquisition::computed_url(property, &winner.computed, slot)?;
            if url.as_str() != route.source_url {
                return Err("origin computed URL mismatch".into());
            }
        }
        provider.preparation_current()?;
        let after = provider.snapshot()?;
        if !before.same(&after) {
            return Err("origin changed during observation".into());
        }
        self.live_owner()?;
        Ok(before)
    }
}
impl Drop for AdmissionLease {
    fn drop(&mut self) {
        self.revoke();
    }
}

// Not public and not implemented by any production type. Raw page callbacks,
// options and report deserialization therefore cannot enter this protocol.
trait OriginProvider {
    fn snapshot(&mut self) -> Result<ProviderStamp, String>;
    /// Adapter must validate the bound native preparation's DOM/base/policy seal.
    fn preparation_current(&mut self) -> Result<(), String>;
    fn winner(&mut self, route: &SvgReplayRoute) -> Result<Winner, String>;
}
struct Winner {
    source: RetainedSvgPresentationAttribute,
    presentation: bool,
    computed: String,
}
#[derive(Clone)]
struct ProviderStamp {
    owner: Weak<Owner>,
    identity: Arc<()>,
    revision: u64,
    generation: u64,
    engine: String,
    version: String,
    protocol: u32,
    static_screen: bool,
    forced_colors: bool,
}
impl ProviderStamp {
    fn supported(&self, generation: u64) -> Result<(), String> {
        if self.engine != "chromium"
            || self.version != "148.0.7778.96"
            || self.protocol != 1
            || self.generation != generation
            || !self.static_screen
            || self.forced_colors
        {
            return Err("unsupported origin provider or environment".into());
        }
        Ok(())
    }
    fn same(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.owner, &other.owner)
            && Arc::ptr_eq(&self.identity, &other.identity)
            && self.revision == other.revision
            && self.generation == other.generation
            && self.engine == other.engine
            && self.version == other.version
            && self.protocol == other.protocol
            && self.static_screen == other.static_screen
            && self.forced_colors == other.forced_colors
    }
    #[cfg(test)]
    fn fixture(owner: &AdmissionOwner) -> Self {
        Self {
            owner: Arc::downgrade(&owner.inner),
            identity: Arc::new(()),
            revision: 1,
            generation: 1,
            engine: "chromium".into(),
            version: "148.0.7778.96".into(),
            protocol: 1,
            static_screen: true,
            forced_colors: false,
        }
    }
}
#[cfg(test)]
#[path = "admission_tests.rs"]
mod tests;
