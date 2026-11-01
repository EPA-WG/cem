use super::*;
pub(in crate::structural_receiver::application) struct Guard {
    envelope: Weak<VerifiedApplicationEnvelope>,
    context: AttributeDatatypeContextLease,
    parent: Weak<v3::LinkedApplication>,
    registry: Weak<RegistryOwner>,
    groups: Vec<Weak<mixed::Group>>,
    active: AtomicBool,
    busy: AtomicBool,
    _runtime_resource: Option<cem_ml::runtime_admission::NativeResourceLease>,
}
impl Guard {
    pub(super) fn new(
        host: &FreshStructuralHost,
        envelope: &Arc<VerifiedApplicationEnvelope>,
        registry: &Arc<RegistryOwner>,
        parent: &Arc<v3::LinkedApplication>,
        groups: Vec<Arc<mixed::Group>>,
    ) -> Result<Arc<Self>, String> {
        let value = Arc::new(Self {
            envelope: Arc::downgrade(envelope),
            context: host.context.lease(),
            parent: Arc::downgrade(parent),
            registry: Arc::downgrade(registry),
            groups: groups.iter().map(Arc::downgrade).collect(),
            active: AtomicBool::new(true),
            busy: AtomicBool::new(false),
            _runtime_resource: host
                .runtime_resource(cem_ml::runtime_admission::NativeRootKind::Publication)?,
        });
        value.check(host)?;
        Ok(value)
    }
    pub(in crate::structural_receiver::application) fn check(
        &self,
        host: &FreshStructuralHost,
    ) -> Result<(), String> {
        host.check().map_err(|_| INVALID)?;
        if !self.active.load(Ordering::Acquire) || !self.context.matches(&host.context) {
            return Err(INVALID.into());
        }
        let envelope = self.envelope.upgrade().ok_or(INVALID)?;
        self.registry
            .upgrade()
            .ok_or(INVALID)?
            .check(host, &envelope)?;
        let parent = self.parent.upgrade().ok_or(INVALID)?;
        if !Arc::ptr_eq(parent.envelope(), envelope.parent()) {
            return Err(INVALID.into());
        }
        parent.check(host)?;
        for group in &self.groups {
            group.upgrade().ok_or(INVALID)?.check_local(host)?;
        }
        Ok(())
    }
    pub(in crate::structural_receiver::application) fn refresh(
        &self,
        host: &mut FreshStructuralHost,
    ) -> Result<(), String> {
        self.check(host)?;
        for group in &self.groups {
            group.upgrade().ok_or(INVALID)?.refresh_local(host)?;
            self.check(host)?;
        }
        Ok(())
    }
    pub(in crate::structural_receiver::application) fn is_busy(&self) -> bool {
        self.busy.load(Ordering::Acquire)
    }
    pub(super) fn check_idle(&self, host: &FreshStructuralHost) -> Result<(), String> {
        self.check(host)?;
        if self.is_busy() {
            return Err(INVALID.into());
        }
        Ok(())
    }
    pub(in crate::structural_receiver::application) fn enter(
        self: &Arc<Self>,
        host: &FreshStructuralHost,
    ) -> Result<Entry, String> {
        self.check(host)?;
        let runtime = host.runtime_entry()?;
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| INVALID)?;
        Ok(Entry {
            guard: self.clone(),
            _runtime: runtime,
        })
    }
    pub(super) fn retire(&self) {
        self.active.store(false, Ordering::Release);
        for group in &self.groups {
            if let Some(g) = group.upgrade() {
                g.retire();
            }
        }
    }
    pub(super) fn registry(&self) -> Result<Arc<RegistryOwner>, String> {
        self.registry.upgrade().ok_or_else(|| INVALID.into())
    }
}
pub(in crate::structural_receiver::application) struct Entry {
    guard: Arc<Guard>,
    _runtime: Option<cem_ml::runtime_admission::OwnedNativeEntry>,
}
impl Drop for Entry {
    fn drop(&mut self) {
        self.guard.busy.store(false, Ordering::Release);
    }
}
