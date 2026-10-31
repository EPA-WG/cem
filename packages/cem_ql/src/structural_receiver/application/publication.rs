//! One native publication transaction for parent code owners, exact registry
//! demand and every mixed receiver. No renderer or physical runtime admission.
use super::{mixed, v3, ComponentReceiverSource, ContractBinding};
use crate::{
    application_envelope::v4::VerifiedApplicationEnvelope,
    selected_registry::SelectedFunctionRegistry,
    structural_receiver::{FreshDependencyDatatypeCompilation, FreshStructuralHost},
};
use cem_ml::schema::attribute_datatypes::AttributeDatatypeContextLease;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Weak,
    },
};
mod guard;
mod operations;
pub(super) use guard::{Entry, Guard};
const INVALID: &str = "native complete application publication refused";
/// Caller-held immutable selected registry, stamped to this original package and
/// actual native host. It is not a linker-omission or deployment attestation.
pub struct RegistryOwner {
    envelope: Weak<VerifiedApplicationEnvelope>,
    context: AttributeDatatypeContextLease,
    registry: SelectedFunctionRegistry,
    active: AtomicBool,
}
impl RegistryOwner {
    fn check(
        &self,
        host: &FreshStructuralHost,
        envelope: &Arc<VerifiedApplicationEnvelope>,
    ) -> Result<(), String> {
        host.check().map_err(|_| INVALID)?;
        if !self.active.load(Ordering::Acquire)
            || !self.context.matches(&host.context)
            || !self
                .envelope
                .upgrade()
                .is_some_and(|e| Arc::ptr_eq(&e, envelope))
            || envelope.pin().registry_hash != host.registry_hash
        {
            return Err(INVALID.into());
        }
        Ok(())
    }
    pub fn retire(&self) {
        self.active.store(false, Ordering::Release);
    }
}
pub fn link_registry(
    host: &FreshStructuralHost,
    envelope: &Arc<VerifiedApplicationEnvelope>,
    registry: SelectedFunctionRegistry,
) -> Result<RegistryOwner, String> {
    host.check().map_err(|_| INVALID)?;
    if envelope.pin().registry_hash != host.registry_hash
        || registry
            .entries()
            .ne(envelope.registry_demand().registry().entries())
    {
        return Err(INVALID.into());
    }
    Ok(RegistryOwner {
        envelope: Arc::downgrade(envelope),
        context: host.context.lease(),
        registry,
        active: AtomicBool::new(true),
    })
}
pub struct MixedSource {
    pub receiver: mixed::MixedReceiverSource,
    pub scalars: BTreeMap<String, mixed::ScalarReceiverSource>,
}
pub struct LinkedApplication {
    envelope: Arc<VerifiedApplicationEnvelope>,
    parent: Arc<v3::LinkedApplication>,
    mixed: BTreeMap<String, mixed::LinkedMixedReceivers>,
    guard: Arc<Guard>,
}
impl LinkedApplication {
    pub fn envelope(&self) -> &Arc<VerifiedApplicationEnvelope> {
        &self.envelope
    }
    pub fn is_available(&self, host: &FreshStructuralHost) -> bool {
        self.guard.check_idle(host).is_ok()
    }
    pub fn receivers(
        &self,
        host: &FreshStructuralHost,
        tag: &str,
    ) -> Result<&mixed::LinkedMixedReceivers, String> {
        self.guard.check_idle(host)?;
        self.mixed.get(tag).ok_or_else(|| INVALID.into())
    }
    fn retire(&self) {
        self.guard.retire();
    }
}
impl Drop for LinkedApplication {
    fn drop(&mut self) {
        self.retire();
    }
}
#[derive(Default)]
pub struct ApplicationSlot {
    current: Option<Arc<LinkedApplication>>,
}
impl ApplicationSlot {
    pub fn publish(
        &mut self,
        host: &mut FreshStructuralHost,
        staged: LinkedApplication,
    ) -> Result<(), String> {
        {
            let _entry = staged.guard.enter(host)?;
            staged.guard.refresh(host)?;
        }
        staged.guard.check_idle(host)?;
        if let Some(previous) = self.current.replace(Arc::new(staged)) {
            previous.retire();
        }
        Ok(())
    }
    pub fn current(&self, host: &FreshStructuralHost) -> Result<&Arc<LinkedApplication>, String> {
        let value = self.current.as_ref().ok_or(INVALID)?;
        value.guard.check_idle(host)?;
        Ok(value)
    }
    pub fn clear(&mut self) {
        if let Some(current) = self.current.take() {
            current.retire();
        }
    }
}
impl Drop for ApplicationSlot {
    fn drop(&mut self) {
        self.clear();
    }
}
pub fn stage(
    host: &mut FreshStructuralHost,
    envelope: Arc<VerifiedApplicationEnvelope>,
    registry: &Arc<RegistryOwner>,
    base_sources: BTreeMap<String, ComponentReceiverSource>,
    bindings: BTreeMap<String, ContractBinding>,
    mut mixed_sources: BTreeMap<String, MixedSource>,
    compilations: &BTreeMap<String, Arc<FreshDependencyDatatypeCompilation>>,
) -> Result<LinkedApplication, String> {
    registry.check(host, &envelope)?;
    if mixed_sources
        .keys()
        .map(String::as_str)
        .ne(envelope.mixed_tags())
    {
        return Err(INVALID.into());
    }
    let parent = Arc::new(v3::stage(
        host,
        envelope.parent().clone(),
        base_sources,
        bindings,
        compilations,
    )?);
    let mut mixed = BTreeMap::new();
    for tag in envelope.mixed_tags() {
        let contract = envelope.mixed(tag).ok_or(INVALID)?;
        let input = mixed_sources.remove(tag).ok_or(INVALID)?;
        let mut roots = BTreeMap::new();
        for occurrence in contract.occurrences().values() {
            if let Some(crate::application_envelope::mixed::ReceiverContract::Executable {
                root,
                ..
            }) = contract.receiver(&occurrence.fields["type"])
            {
                roots.insert(root.clone(), parent.root(host, root)?.clone());
            }
        }
        mixed.insert(
            tag.into(),
            mixed::stage(host, contract.clone(), input.receiver, input.scalars, roots)?,
        );
    }
    let guard = Guard::new(
        host,
        &envelope,
        registry,
        &parent,
        mixed.values().map(|m| m.group.clone()).collect(),
    )?;
    for receiver in mixed.values() {
        receiver.group.enroll(&guard)?;
    }
    let staged = LinkedApplication {
        envelope,
        parent,
        mixed,
        guard,
    };
    {
        let _entry = staged.guard.enter(host)?;
        staged.guard.refresh(host)?;
    }
    staged.guard.check_idle(host)?;
    Ok(staged)
}
