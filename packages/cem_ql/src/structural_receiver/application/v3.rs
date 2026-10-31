//! Atomic native CEMAPPD3 installation. No component admission, physical
//! generation publication, mixed receiver permissions or executable wire values.
use super::{ComponentReceiverSource, ContractBinding};
use crate::{
    application_envelope::v3::VerifiedApplicationEnvelope,
    structural_receiver::{FreshDependencyDatatypeCompilation, FreshStructuralHost},
};
use std::{collections::BTreeMap, sync::Arc};
mod ownership;
mod roots;
pub use roots::InstalledDatatypeRoot;
const INVALID: &str = "native assembly application installation refused";

pub struct LinkedApplication {
    envelope: Arc<VerifiedApplicationEnvelope>,
    application: Arc<super::LinkedApplication>,
    ownership: Arc<ownership::Installation>,
    roots: BTreeMap<String, Arc<InstalledDatatypeRoot>>,
}
impl LinkedApplication {
    pub(super) fn check(&self, host: &FreshStructuralHost) -> Result<(), String> {
        self.ownership.check(host)
    }
    pub fn envelope(&self) -> &Arc<VerifiedApplicationEnvelope> {
        &self.envelope
    }
    /// Existing structural/query/native contract access keeps its own guards.
    pub fn application(
        &self,
        host: &FreshStructuralHost,
    ) -> Result<&super::LinkedApplication, String> {
        self.check(host)?;
        Ok(&self.application)
    }
    pub fn root_count(&self) -> usize {
        self.roots.len()
    }
    pub fn root(
        &self,
        host: &FreshStructuralHost,
        name: &str,
    ) -> Result<&Arc<InstalledDatatypeRoot>, String> {
        self.check(host)?;
        self.roots.get(name).ok_or_else(|| INVALID.into())
    }
}
impl Drop for LinkedApplication {
    fn drop(&mut self) {
        // Revoke only this installation's guards. Caller-held compilations and
        // operation/rule owners may also support another staged/current slot.
        self.ownership.retire();
    }
}
#[derive(Default)]
pub struct ApplicationSlot {
    current: Option<LinkedApplication>,
}
impl ApplicationSlot {
    pub fn publish(
        &mut self,
        host: &FreshStructuralHost,
        staged: LinkedApplication,
    ) -> Result<(), String> {
        staged.check(host)?;
        let retired = self.current.replace(staged);
        drop(retired);
        Ok(())
    }
    pub fn current(&self, host: &FreshStructuralHost) -> Result<&LinkedApplication, String> {
        let current = self.current.as_ref().ok_or(INVALID)?;
        current.check(host)?;
        Ok(current)
    }
    pub fn clear(&mut self) {
        self.current = None;
    }
}
/// All source scopes, directed grants, registrations and compilations must be
/// prepared by the actual embedding first. Staging invokes no user callback,
/// changes no host registry, and borrows reusable compilation owners weakly.
pub fn stage(
    host: &FreshStructuralHost,
    envelope: Arc<VerifiedApplicationEnvelope>,
    sources: BTreeMap<String, ComponentReceiverSource>,
    bindings: BTreeMap<String, ContractBinding>,
    compilations: &BTreeMap<String, Arc<FreshDependencyDatatypeCompilation>>,
) -> Result<LinkedApplication, String> {
    host.check().map_err(|_| INVALID)?;
    if envelope.pin().registry_hash != host.registry_hash
        || compilations.len() != envelope.assembly_count()
        || envelope
            .assembly_ids()
            .any(|id| !compilations.contains_key(id))
    {
        return Err(INVALID.into());
    }
    let application = Arc::new(super::stage(
        host,
        envelope.application_owner().clone(),
        sources,
        bindings,
    )?);
    let ownership = Arc::new(ownership::Installation::new(
        host,
        &envelope,
        &application,
        compilations,
    )?);
    let mut staged = LinkedApplication {
        envelope,
        application,
        ownership,
        roots: BTreeMap::new(),
    };
    for (name, root) in staged.envelope.roots() {
        let compilation = compilations.get(&root.assembly).ok_or(INVALID)?;
        let guard =
            InstalledDatatypeRoot::new(staged.ownership.clone(), compilation, root.datatype);
        staged.roots.insert(name.clone(), Arc::new(guard));
    }
    staged.check(host)?;
    Ok(staged)
}
