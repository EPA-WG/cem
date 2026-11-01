//! Caller-held execution owners. Bindings retain inert data but borrow authority
//! weakly from exact original code, source and result-profile owners.
use super::*;
use crate::query_validator_capsule::profiles::{QueryResultProfilePin, VerifiedQueryResultProfile};
use std::sync::Weak;
pub struct FreshQueryCodeOwner {
    source: Weak<FreshQuerySource>,
    functions: SelectedFunctionRegistry,
    context: AttributeDatatypeContextLease,
    runtime: RuntimeOwner,
    active: AtomicBool,
}
pub struct FreshQueryResultProfileOwner {
    pub(crate) profile: Arc<VerifiedQueryResultProfile>,
    scopes: Vec<DeclarationScope>,
    context: AttributeDatatypeContextLease,
    runtime: RuntimeOwner,
    active: AtomicBool,
}
pub struct FreshQueryRegistryOwner {
    source: Weak<FreshQuerySource>,
    code: Weak<FreshQueryCodeOwner>,
    profile: Weak<FreshQueryResultProfileOwner>,
    context: AttributeDatatypeContextLease,
    runtime: RuntimeOwner,
    active: AtomicBool,
}
macro_rules! retire {
    () => {
        pub fn retire(&self) {
            self.active.store(false, Ordering::Release);
        }
    };
}
impl FreshQueryCodeOwner {
    retire!();
    pub fn is_live_on(&self, host: &FreshStructuralHost) -> bool {
        host.check().is_ok()
            && self.active.load(Ordering::Acquire)
            && self.runtime.matches(host)
            && self.context.matches(&host.context)
            && self.source.upgrade().is_some_and(|s| s.live(host))
    }
}
impl FreshQueryResultProfileOwner {
    retire!();
    pub fn is_live_on(&self, host: &FreshStructuralHost) -> bool {
        host.check().is_ok()
            && self.active.load(Ordering::Acquire)
            && self.runtime.matches(host)
            && self.context.matches(&host.context)
            && self
                .profile
                .sources()
                .iter()
                .zip(&self.scopes)
                .all(|(s, scope)| {
                    host.inner
                        .registered_validator_context(*scope, &s.tree)
                        .is_some()
                })
    }
}
impl FreshQueryRegistryOwner {
    retire!();
    pub fn is_live_on(&self, host: &FreshStructuralHost) -> bool {
        host.check().is_ok()
            && self.active.load(Ordering::Acquire)
            && self.runtime.matches(host)
            && self.context.matches(&host.context)
            && self.source.upgrade().is_some_and(|s| s.live(host))
            && self.code.upgrade().is_some_and(|c| c.is_live_on(host))
            && self.profile.upgrade().is_some_and(|p| p.is_live_on(host))
    }
}
impl FreshStructuralHost {
    pub fn query_code_owner(
        &self,
        source: &Arc<FreshQuerySource>,
        available: &SelectedFunctionRegistry,
    ) -> Result<Arc<FreshQueryCodeOwner>, String> {
        let _entry = self.runtime_entry()?;
        self.check().map_err(|_| INVALID)?;
        if !source.live(self) {
            return Err(INVALID.into());
        }
        let required = source.declaration().program.required_functions()?;
        if required.iter().any(|entry| {
            let (module, name, arity) = entry.key();
            available.resolve(module, name, arity) != Some(*entry)
        }) {
            return Err(INVALID.into());
        }
        let owner = Arc::new(FreshQueryCodeOwner {
            source: Arc::downgrade(source),
            functions: SelectedFunctionRegistry::new(required)?,
            context: self.context.lease(),
            runtime: RuntimeOwner::new(self, NativeRootKind::Registry)?,
            active: AtomicBool::new(true),
        });
        if !owner.is_live_on(self) {
            return Err(INVALID.into());
        }
        Ok(owner)
    }
    pub fn query_result_profile_owner(
        &self,
        profile: Arc<VerifiedQueryResultProfile>,
        scopes: &[DeclarationScope],
        expected: &QueryResultProfilePin,
    ) -> Result<Arc<FreshQueryResultProfileOwner>, String> {
        let _entry = self.runtime_entry()?;
        self.check().map_err(|_| INVALID)?;
        if profile.pin() != expected || scopes.len() != profile.sources().len() {
            return Err(INVALID.into());
        }
        let owner = Arc::new(FreshQueryResultProfileOwner {
            profile,
            scopes: scopes.to_vec(),
            context: self.context.lease(),
            runtime: RuntimeOwner::new(self, NativeRootKind::Compilation)?,
            active: AtomicBool::new(true),
        });
        if !owner.is_live_on(self) {
            return Err(INVALID.into());
        }
        Ok(owner)
    }
    pub fn query_registry_owner(
        &self,
        source: &Arc<FreshQuerySource>,
        code: &Arc<FreshQueryCodeOwner>,
        profile: &Arc<FreshQueryResultProfileOwner>,
    ) -> Result<Arc<FreshQueryRegistryOwner>, String> {
        let _entry = self.runtime_entry()?;
        self.check().map_err(|_| INVALID)?;
        if !source.live(self)
            || !code.is_live_on(self)
            || !profile.is_live_on(self)
            || !code
                .source
                .upgrade()
                .is_some_and(|s| Arc::ptr_eq(&s, source))
            || source.declaration().contract.signature().result
                != cem_ml::schema::datatype_validation::ResultRepresentation::Accepted(
                    profile.profile.pin().result.clone(),
                )
        {
            return Err(INVALID.into());
        }
        let owner = Arc::new(FreshQueryRegistryOwner {
            source: Arc::downgrade(source),
            code: Arc::downgrade(code),
            profile: Arc::downgrade(profile),
            context: self.context.lease(),
            runtime: RuntimeOwner::new(self, NativeRootKind::Registry)?,
            active: AtomicBool::new(true),
        });
        if !owner.is_live_on(self) {
            return Err(INVALID.into());
        }
        Ok(owner)
    }
    pub fn link_query_owned(
        &self,
        source: &Arc<FreshQuerySource>,
        datatype: AstNodeId,
        registry: &Arc<FreshQueryRegistryOwner>,
    ) -> Result<FreshQueryBinding, String> {
        let _entry = self.runtime_entry()?;
        if !registry.is_live_on(self)
            || !registry
                .source
                .upgrade()
                .is_some_and(|s| Arc::ptr_eq(&s, source))
        {
            return Err(INVALID.into());
        }
        let code = registry.code.upgrade().ok_or(INVALID)?;
        let profile = registry.profile.upgrade().ok_or(INVALID)?;
        let mut binding = self.link_query_source(
            source,
            datatype,
            &code.functions,
            profile.profile.adapter.clone(),
        )?;
        binding.execution = Some(Arc::downgrade(registry));
        if !binding.live(self) {
            return Err(INVALID.into());
        }
        Ok(binding)
    }
    pub fn replace_query_owned(
        &self,
        previous: &FreshQueryBinding,
        source: &Arc<FreshQuerySource>,
        datatype: AstNodeId,
        registry: &Arc<FreshQueryRegistryOwner>,
    ) -> Result<FreshQueryBinding, String> {
        let _entry = self.runtime_entry()?;
        if previous.execution.is_none()
            || !previous.live(self)
            || previous.scope != source.owner.scope
        {
            return Err(INVALID.into());
        }
        let staged = self.link_query_owned(source, datatype, registry)?;
        if !previous.live(self)
            || !staged.live(self)
            || previous
                .active
                .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return Err(INVALID.into());
        }
        Ok(staged)
    }
}
