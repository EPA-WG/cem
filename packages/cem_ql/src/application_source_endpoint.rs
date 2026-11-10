//! Native-provisioned complete application installation. Metadata pins identify
//! sources; only the original Rust bootstrap owner supplies code and grants.
pub use crate::structural_receiver::application::RetainedApplicationOwner;
use crate::{
    application_source_transport::{
        self as transport, ApplicationSourcePin, VerifiedApplicationSources,
    },
    structural_receiver::{
        application::{
            publication::{self, ApplicationSlot, LinkedApplication, RegistryOwner},
            ContractBinding,
        },
        FreshDependencyDatatypeCompilation, FreshStructuralHost,
    },
};
use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    runtime_admission::{generation::NativeGeneration, NativeResourceLease, NativeRootKind},
};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Weak,
    },
};

#[cfg(any(feature = "component-runtime", feature = "application-source-runtime"))]
mod frames;
#[cfg(feature = "application-source-runtime")]
mod receiver_checkpoint;
#[cfg(feature = "application-source-runtime")]
mod styles;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(any(feature = "component-runtime", feature = "application-source-runtime"))]
pub use frames::{ApplicationReceiverInput, ApplicationTypedReceiverProducer};
#[cfg(feature = "application-source-runtime")]
pub use receiver_checkpoint::{
    ReceiverCheckpointSeal, ReceiverRecoveryPin, ReceiverWriteJournal, VerifiedReceiverCheckpoint,
};
const INVALID: &str = "cem.application.endpoint.refused";
pub const MAX_PINS_BYTES: usize = 16 * 1024;
// Complete applications retain more code/compilation roots than query leaves.
// The shared 256-root generation inventory may refuse large apps earlier.
pub const MAX_SESSIONS: usize = 8;

/// Explicit native embedding code. It restores actual contexts, registrations,
/// scopes, inputs and directed grants before constructing compilation owners.
/// No callback, permission or context is inferred from a deployment pin.
pub trait ApplicationBootstrap: Send + Sync {
    fn prepare(
        &self,
        sources: Arc<VerifiedApplicationSources>,
        generation: &Arc<NativeGeneration>,
        control: OperationControl,
    ) -> Result<PreparedApplication, String>;
}
pub struct ApplicationBootstrapOwner {
    generation: Weak<NativeGeneration>,
    control: OperationControl,
    pin: ApplicationSourcePin,
    bootstrap: Arc<dyn ApplicationBootstrap>,
    active: AtomicBool,
    busy: Arc<AtomicBool>,
    _lease: NativeResourceLease,
}
impl ApplicationBootstrapOwner {
    pub fn new(
        generation: &Arc<NativeGeneration>,
        control: OperationControl,
        pin: ApplicationSourcePin,
        bootstrap: impl ApplicationBootstrap + 'static,
    ) -> Result<Arc<Self>, String> {
        let _entry = generation.admit().map_err(|_| INVALID)?;
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID)?;
        if encoded(&pin)?.len() > MAX_PINS_BYTES {
            return Err(INVALID.into());
        }
        Ok(Arc::new(Self {
            generation: Arc::downgrade(generation),
            control,
            pin,
            bootstrap: Arc::new(bootstrap),
            active: AtomicBool::new(true),
            busy: Arc::new(AtomicBool::new(false)),
            _lease: generation
                .resource(NativeRootKind::Registry)
                .map_err(|_| INVALID)?,
        }))
    }
    fn check(&self) -> Result<(), String> {
        if !self.active.load(Ordering::Acquire)
            || !self.generation.upgrade().is_some_and(|g| g.is_open())
        {
            return Err(INVALID.into());
        }
        self.control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID.into())
    }
    pub fn retire(&self) {
        self.active.store(false, Ordering::Release);
        cem_ml::runtime_inventory::note_mutation();
    }
}
fn encoded(v: &impl serde::Serialize) -> Result<Vec<u8>, String> {
    serde_json::to_vec(v).map_err(|_| INVALID.into())
}
struct BootstrapEntry(Arc<AtomicBool>);
impl Drop for BootstrapEntry {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// A staged original publication plus explicit native owners. The slot is
/// private: only an enrolled endpoint may publish this candidate.
pub struct PreparedApplication {
    host: FreshStructuralHost,
    sources: Arc<VerifiedApplicationSources>,
    staged: Option<LinkedApplication>,
    slot: ApplicationSlot,
    _registry: Arc<RegistryOwner>,
    compilations: BTreeMap<String, Arc<FreshDependencyDatatypeCompilation>>,
    retained: Vec<RetainedApplicationOwner>,
    #[cfg(feature = "application-source-runtime")]
    needs_styles: bool,
    #[cfg(feature = "application-source-runtime")]
    styles: Option<styles::ApplicationStyles>,
}
impl PreparedApplication {
    pub fn stage(
        mut host: FreshStructuralHost,
        sources: Arc<VerifiedApplicationSources>,
        registry: Arc<RegistryOwner>,
        bindings: BTreeMap<String, ContractBinding>,
        compilations: BTreeMap<String, Arc<FreshDependencyDatatypeCompilation>>,
        retained: Vec<RetainedApplicationOwner>,
    ) -> Result<Self, String> {
        if retained.len() > 256
            || compilations.len() > 8
            || retained.iter().any(|o| !o.is_live_on(&host))
        {
            return Err(INVALID.into());
        }
        let (base, mixed) = sources.receivers()?;
        let staged = publication::stage(
            &mut host,
            sources.application().clone(),
            &registry,
            base,
            bindings,
            mixed,
            &compilations,
        )?;
        if !sources.matches_publication(&host, &staged) {
            return Err(INVALID.into());
        }
        #[cfg(feature = "application-source-runtime")]
        let needs_styles = staged
            .envelope()
            .component_order()
            .iter()
            .any(|tag| staged.envelope().artifact(tag).is_some_and(|artifact| !artifact.stylesheets.is_empty()));
        Ok(Self {
            host,
            sources,
            staged: Some(staged),
            slot: Default::default(),
            _registry: registry,
            compilations,
            retained,
            #[cfg(feature = "application-source-runtime")]
            needs_styles,
            #[cfg(feature = "application-source-runtime")]
            styles: None,
        })
    }
    /// The original bootstrap supplies a live resolver for the entire pinned
    /// declaration set. A failed occurrence leaves the candidate unpublished.
    #[cfg(feature = "application-source-runtime")]
    pub fn with_stylesheet_policy(
        mut self,
        capability: cem_ml::module_resolution::CemModuleUrlResolutionCapability,
        response_policy: cem_ml::css_imports::CssImportResponsePolicy,
    ) -> Result<Self, String> {
        if self.styles.is_some() {
            return Err(INVALID.into());
        }
        if self.needs_styles {
            self.styles = Some(styles::ApplicationStyles::stage(
                self.staged.as_ref().ok_or(INVALID)?.envelope(),
                capability,
                response_policy,
                self.host.application_control(),
            )?);
        }
        Ok(self)
    }
    fn owners_live(&self) -> bool {
        #[cfg(feature = "application-source-runtime")]
        if self.needs_styles && !self.styles.as_ref().is_some_and(styles::ApplicationStyles::current) {
            return false;
        }
        self.compilations.values().all(|o| o.is_live(&self.host))
            && self.retained.iter().all(|o| o.is_live_on(&self.host))
    }
    fn current(&self) -> Result<&Arc<LinkedApplication>, String> {
        if !self.owners_live() {
            return Err(INVALID.into());
        }
        self.slot.current(&self.host)
    }
    fn publish(&mut self, endpoint: &Arc<ApplicationSourceEndpoint>) -> Result<(), String> {
        #[cfg(feature = "application-source-runtime")]
        if let Some(styles) = &mut self.styles {
            styles.enroll(&endpoint.generation.upgrade().ok_or(INVALID)?)?;
        }
        let staged = self.staged.take().ok_or(INVALID)?;
        staged.enroll_source_endpoint(endpoint, &self.host)?;
        self.slot.publish(&mut self.host, staged)?;
        self.current()?;
        Ok(())
    }
}
impl Drop for PreparedApplication {
    fn drop(&mut self) {
        self.slot.clear();
        self.staged.take();
        self.host.close();
    }
}

/// Independent original endpoint authority. Sessions borrow it and its original
/// bootstrap weakly, so neither a token nor a retained receiver prolongs a grant.
pub struct ApplicationSourceEndpoint {
    pub(crate) generation: Weak<NativeGeneration>,
    code: Weak<ApplicationBootstrapOwner>,
    control: OperationControl,
    pin: ApplicationSourcePin,
    active: AtomicBool,
    sessions: Arc<AtomicUsize>,
    _lease: NativeResourceLease,
}
impl ApplicationSourceEndpoint {
    pub fn new(code: &Arc<ApplicationBootstrapOwner>) -> Result<Arc<Self>, String> {
        code.check()?;
        let g = code.generation.upgrade().ok_or(INVALID)?;
        let _entry = g.admit().map_err(|_| INVALID)?;
        Ok(Arc::new(Self {
            generation: Arc::downgrade(&g),
            code: Arc::downgrade(code),
            control: code.control.clone(),
            pin: code.pin.clone(),
            active: AtomicBool::new(true),
            sessions: Arc::new(AtomicUsize::new(0)),
            _lease: g.resource(NativeRootKind::Registry).map_err(|_| INVALID)?,
        }))
    }
    fn check(&self) -> Result<(), String> {
        if !self.active.load(Ordering::Acquire)
            || !self.generation.upgrade().is_some_and(|g| g.is_open())
        {
            return Err(INVALID.into());
        }
        self.code.upgrade().ok_or(INVALID)?.check()?;
        self.control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID.into())
    }
    pub(crate) fn is_live_on(&self, host: &FreshStructuralHost) -> bool {
        self.check().is_ok()
            && self
                .generation
                .upgrade()
                .is_some_and(|g| host.belongs_to_application_runtime(&g, &self.control))
    }
    pub fn retire(&self) {
        self.active.store(false, Ordering::Release);
        cem_ml::runtime_inventory::note_mutation();
    }
    fn prepare(self: &Arc<Self>, bytes: &[u8], pins: &str) -> Result<PreparedApplication, String> {
        self.check()?;
        let g = self.generation.upgrade().ok_or(INVALID)?;
        let _entry = g.admit().map_err(|_| INVALID)?;
        if bytes.len() > transport::MAX_BYTES || pins.len() > MAX_PINS_BYTES {
            return Err(INVALID.into());
        }
        let pin: ApplicationSourcePin = serde_json::from_str(pins).map_err(|_| INVALID)?;
        if encoded(&pin)? != encoded(&self.pin)? {
            return Err(INVALID.into());
        }
        let sources = transport::verify(bytes, &self.pin, &self.control)?;
        let code = self.code.upgrade().ok_or(INVALID)?;
        code.check()?;
        code.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| INVALID)?;
        let _busy = BootstrapEntry(code.busy.clone());
        let bootstrap = code.bootstrap.clone();
        // The invocation holds code, not the original caller's authority lease.
        drop(code);
        self.check()?;
        let mut prepared = bootstrap.prepare(sources.clone(), &g, self.control.clone())?;
        drop(bootstrap);
        self.check()?;
        if !Arc::ptr_eq(&prepared.sources, &sources)
            || !self.is_live_on(&prepared.host)
            || !prepared.owners_live()
        {
            return Err(INVALID.into());
        }
        prepared.publish(self)?;
        self.check()?;
        Ok(prepared)
    }
    pub fn open(
        self: &Arc<Self>,
        bytes: &[u8],
        pins: &str,
    ) -> Result<ApplicationSourceSession, String> {
        self.check()?;
        let generation = self.generation.upgrade().ok_or(INVALID)?;
        let _entry = generation.admit().map_err(|_| INVALID)?;
        self.sessions
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < MAX_SESSIONS).then_some(n + 1)
            })
            .map_err(|_| INVALID)?;
        let slot = SessionSlot(self.sessions.clone());
        let prepared = self.prepare(bytes, pins)?;
        Ok(ApplicationSourceSession {
            endpoint: Arc::downgrade(self),
            prepared,
            _slot: slot,
            receiver_owners: Arc::new(AtomicUsize::new(0)),
            #[cfg(feature = "application-source-runtime")]
            receiver_checkpoint_epoch: Arc::new(AtomicUsize::new(0)),
        })
    }
}
struct SessionSlot(Arc<AtomicUsize>);
impl Drop for SessionSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
        cem_ml::runtime_inventory::note_mutation();
    }
}
pub struct ApplicationSourceSession {
    endpoint: Weak<ApplicationSourceEndpoint>,
    prepared: PreparedApplication,
    _slot: SessionSlot,
    receiver_owners: Arc<AtomicUsize>,
    #[cfg(feature = "application-source-runtime")]
    receiver_checkpoint_epoch: Arc<AtomicUsize>,
}
impl ApplicationSourceSession {
    fn enter(&self) -> Result<cem_ml::runtime_admission::OwnedNativeEntry, String> {
        self.check()?;
        self.endpoint
            .upgrade()
            .and_then(|e| e.generation.upgrade())
            .ok_or(INVALID)?
            .admit()
            .map_err(|_| INVALID.into())
    }
    fn check(&self) -> Result<(), String> {
        self.endpoint.upgrade().ok_or(INVALID)?.check()?;
        self.prepared.current()?;
        Ok(())
    }
    pub fn capture(&self) -> Result<Vec<u8>, String> {
        let _entry = self.enter()?;
        if self.receiver_owners.load(Ordering::Acquire) != 0 {
            return Err("cem.application.receiver_checkpoint.unavailable".into());
        }
        let inputs = self
            .prepared
            .sources
            .claims()
            .iter()
            .map(|c| c.uri.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|uri| {
                let s = self.prepared.sources.source(uri).ok_or(INVALID)?;
                Ok(transport::SourceInput {
                    identity: s.identity.clone(),
                    tree: s.tree.clone(),
                    capture: s.capture.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let control = self.endpoint.upgrade().ok_or(INVALID)?.control.clone();
        let bytes = transport::capture(
            &self.prepared.host,
            self.prepared.current()?,
            &inputs,
            &control,
        )?
        .bytes;
        self.check()?;
        Ok(bytes)
    }
    pub fn with_application<T>(
        &mut self,
        run: impl FnOnce(&LinkedApplication, &mut FreshStructuralHost) -> Result<T, String>,
    ) -> Result<T, String> {
        // Arbitrary host access can change scopes, including before unwinding.
        #[cfg(feature = "application-source-runtime")]
        self.receiver_checkpoint_epoch.fetch_add(1, Ordering::AcqRel);
        self.with_current_application(run)
    }
    fn with_current_application<T>(
        &mut self,
        run: impl FnOnce(&LinkedApplication, &mut FreshStructuralHost) -> Result<T, String>,
    ) -> Result<T, String> {
        self.check()?;
        let g = self
            .endpoint
            .upgrade()
            .and_then(|e| e.generation.upgrade())
            .ok_or(INVALID)?;
        let _entry = g.admit().map_err(|_| INVALID)?;
        let app = self.prepared.current()?.clone();
        app.renew_source_owners(&mut self.prepared.host)?;
        let result = run(&app, &mut self.prepared.host);
        app.renew_source_owners(&mut self.prepared.host)?;
        self.check()?;
        result
    }
    /// Prepare and publish a fresh candidate before retiring the prior slot.
    pub fn replace(&mut self, bytes: &[u8], pins: &str) -> Result<(), String> {
        let _entry = self.enter()?;
        let endpoint = self.endpoint.upgrade().ok_or(INVALID)?;
        let candidate = endpoint.prepare(bytes, pins)?;
        drop(endpoint);
        self.check()?;
        self.prepared = candidate;
        #[cfg(feature = "application-source-runtime")]
        self.receiver_checkpoint_epoch.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
}
