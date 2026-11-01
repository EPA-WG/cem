//! Original native validator code authority, never a callback reconstructed
//! from a name/hash. Hosts and imported registries borrow it weakly.
use super::*;
use cem_ml::runtime_admission::{
    generation::NativeGeneration, NativeResourceLease, NativeRootKind,
};
pub struct FreshNativeRuleCodeOwner {
    identity: ImplementationIdentity,
    callback: Arc<dyn NativeDatatypeValidator>,
    generation: Weak<NativeGeneration>,
    control: OperationControl,
    active: AtomicBool,
    _lease: NativeResourceLease,
}
impl FreshNativeRuleCodeOwner {
    pub fn new(
        generation: &Arc<NativeGeneration>,
        control: OperationControl,
        identity: ImplementationIdentity,
        callback: impl NativeDatatypeValidator + 'static,
    ) -> Result<Arc<Self>, NativeLinkError> {
        let _entry = generation.admit().map_err(|_| NativeLinkError::Expired)?;
        identity
            .check()
            .map_err(|_| NativeLinkError::IdentityMismatch)?;
        if !matches!(identity.signature, NativeSignature::Validator(_))
            || control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_err()
        {
            return Err(NativeLinkError::IdentityMismatch);
        }
        Ok(Arc::new(Self {
            identity,
            callback: Arc::new(callback),
            generation: Arc::downgrade(generation),
            control,
            active: AtomicBool::new(true),
            _lease: generation
                .resource(NativeRootKind::Registry)
                .map_err(|_| NativeLinkError::Expired)?,
        }))
    }
    pub fn identity(&self) -> &ImplementationIdentity {
        &self.identity
    }
    pub fn retire(&self) {
        self.active.store(false, Ordering::Release);
        cem_ml::runtime_inventory::note_mutation();
    }
    pub(crate) fn is_live(&self) -> bool {
        self.active.load(Ordering::Acquire)
            && self.generation.upgrade().is_some_and(|g| g.is_open())
            && self.control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_ok()
    }
    pub fn is_live_on(&self, host: &FreshStructuralHost) -> bool {
        self.is_live()
            && host.check().is_ok()
            && host
                .runtime
                .as_ref()
                .is_some_and(|g| Weak::ptr_eq(g, &self.generation))
    }
    pub(super) fn validate(&self, call: ValidationCall<'_>) -> RuleExecution {
        if !self.is_live() {
            return RuleExecution::Unavailable(vec![]);
        }
        let result = self.callback.validate(call);
        if self.is_live() {
            result
        } else {
            RuleExecution::Unavailable(vec![])
        }
    }
}
impl FreshStructuralHost {
    pub fn register_native_validator_owner(
        &mut self,
        owner: &Arc<FreshNativeRuleCodeOwner>,
    ) -> Result<(), NativeLinkError> {
        if !owner.is_live_on(self) {
            return Err(NativeLinkError::Expired);
        }
        self.register_implementation(
            owner.identity.clone(),
            Actual::OwnedValidator(Arc::downgrade(owner)),
        )
    }
    pub(crate) fn native_validator_owner_matches(
        &self,
        owner: &Arc<FreshNativeRuleCodeOwner>,
    ) -> bool {
        owner.is_live_on(self) && self.native_implementations.get(&owner.identity.id).is_some_and(|entry|
            entry.identity == owner.identity && matches!(&entry.actual, Actual::OwnedValidator(w) if Weak::ptr_eq(w, &Arc::downgrade(owner))))
    }
}
