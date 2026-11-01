//! Fresh-host query binding and invocation receipts. Nothing here is serialized
//! or inferred from source hashes, public IDs or a previous runtime's handles.
use super::*;
use crate::{
    datatype_results::DatatypeResultAdapter,
    datatype_validation::{
        validate_rules, BoundDatatypeRule, DatatypeValidationRegistry, ValidationBatch,
        ValidationInput, ValidationLimits, ValidationRuntime,
    },
    query_validator_capsule::declarations::{self, VerifiedQueryDeclaration},
    selected_registry::SelectedFunctionRegistry,
};
use cem_ml::runtime_admission::NativeRootKind;
use cem_ml::schema::attribute_datatypes::AttributeDatatypeContextLease;
mod owners;
mod handoff;
mod execution;
pub use execution::{FreshQueryCodeOwner, FreshQueryRegistryOwner, FreshQueryResultProfileOwner};
pub use owners::FreshQuerySource;
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::structural_receiver) use owners::QuerySourceLease;
pub(in crate::structural_receiver) use owners::RuntimeOwner;

pub struct FreshQueryBinding {
    pub(super) declaration: BindingOwner,
    pub(super) scope: DeclarationScope,
    pub(super) context: AttributeDatatypeContextLease,
    pub(super) rule: BoundDatatypeRule,
    pub(super) active: Arc<AtomicBool>,
    pub(super) source_owner: Option<std::sync::Weak<FreshQuerySource>>,
    pub(super) _owned_source: Option<Arc<FreshQuerySource>>,
    pub(super) _runtime: RuntimeOwner,
    pub(super) execution: Option<std::sync::Weak<FreshQueryRegistryOwner>>,
}
pub(super) enum BindingOwner {
    Query(Arc<VerifiedQueryDeclaration>),
    #[cfg(not(target_arch = "wasm32"))]
    Native(Arc<crate::native_registrations::VerifiedNativeRequirement>),
}
impl BindingOwner {
    fn source(&self) -> &Arc<RetainedCemTree> {
        match self {
            Self::Query(owner) => owner.source(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Native(owner) => owner.source(),
        }
    }
}
impl FreshQueryBinding {
    pub fn retire(&self) {
        self.active.store(false, Ordering::Release);
    }
    pub(super) fn live(&self, host: &FreshStructuralHost) -> bool {
        self.active.load(Ordering::Acquire)
            && self._runtime.matches(host)
            && self.execution.as_ref().is_none_or(|e| e.upgrade().is_some_and(|e| e.is_live_on(host)))
            && self
                .source_owner
                .as_ref()
                .is_none_or(|s| s.upgrade().is_some_and(|s| s.live(host)))
            && self.context.matches(&host.context)
            && host
                .inner
                .registered_validator_context(self.scope, self.declaration.source())
                .is_some()
    }
}
/// An enclosing request shares one budget across all of its validated rules.
/// Runtime query scope/policy are obtained from the real registered host scope.
pub struct FreshQueryInvocation {
    scope: DeclarationScope,
    context: AttributeDatatypeContextLease,
    query: crate::api::EvaluationContext,
    active: AtomicBool,
    busy: AtomicBool,
    _runtime: RuntimeOwner,
}
impl FreshQueryInvocation {
    pub fn retire(&self) {
        self.active.store(false, Ordering::Release);
    }
}
struct QueryEntry<'a>(&'a AtomicBool);
impl Drop for QueryEntry<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
impl FreshStructuralHost {
    pub fn replace_query(
        &self,
        previous: &FreshQueryBinding,
        declaration: Arc<VerifiedQueryDeclaration>,
        scope: DeclarationScope,
        datatype: AstNodeId,
        functions: &SelectedFunctionRegistry,
        adapter: DatatypeResultAdapter,
    ) -> Result<FreshQueryBinding, String> {
        let source = self.query_source(declaration, scope)?;
        self.replace_query_source_inner(previous, &source, datatype, functions, adapter, true)
    }
    pub fn replace_query_source(
        &self,
        previous: &FreshQueryBinding,
        source: &Arc<FreshQuerySource>,
        datatype: AstNodeId,
        functions: &SelectedFunctionRegistry,
        adapter: DatatypeResultAdapter,
    ) -> Result<FreshQueryBinding, String> {
        self.replace_query_source_inner(previous, source, datatype, functions, adapter, false)
    }
    fn replace_query_source_inner(
        &self,
        previous: &FreshQueryBinding,
        source: &Arc<FreshQuerySource>,
        datatype: AstNodeId,
        functions: &SelectedFunctionRegistry,
        adapter: DatatypeResultAdapter,
        own_source: bool,
    ) -> Result<FreshQueryBinding, String> {
        let _runtime = self.runtime_entry()?;
        self.check().map_err(|_| INVALID)?;
        if !previous.live(self) || previous.scope != source.owner.scope {
            return Err(INVALID.into());
        }
        let staged =
            self.link_query_source_inner(source, datatype, functions, adapter, own_source)?;
        self.check().map_err(|_| INVALID)?;
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
    pub fn link_query(
        &self,
        declaration: Arc<VerifiedQueryDeclaration>,
        scope: DeclarationScope,
        datatype: AstNodeId,
        functions: &SelectedFunctionRegistry,
        adapter: DatatypeResultAdapter,
    ) -> Result<FreshQueryBinding, String> {
        let source = self.query_source(declaration, scope)?;
        self.link_query_source_inner(&source, datatype, functions, adapter, true)
    }
    pub fn link_query_source(
        &self,
        source: &Arc<FreshQuerySource>,
        datatype: AstNodeId,
        functions: &SelectedFunctionRegistry,
        adapter: DatatypeResultAdapter,
    ) -> Result<FreshQueryBinding, String> {
        self.link_query_source_inner(source, datatype, functions, adapter, false)
    }
    fn link_query_source_inner(
        &self,
        source: &Arc<FreshQuerySource>,
        datatype: AstNodeId,
        functions: &SelectedFunctionRegistry,
        adapter: DatatypeResultAdapter,
        own_source: bool,
    ) -> Result<FreshQueryBinding, String> {
        let _runtime = self.runtime_entry()?;
        self.check().map_err(|_| INVALID)?;
        if !source.live(self) {
            return Err(INVALID.into());
        }
        let scope = source.owner.scope;
        let declaration = source.declaration().clone();
        self.inner
            .registered_validator_context(scope, declaration.source())
            .ok_or(INVALID)?;
        let value = declarations::datatype(&declaration, datatype)?;
        let mut registry = DatatypeValidationRegistry::default();
        registry
            .register_verified_query(
                declaration.contract.clone(),
                &declaration.pin().program.source,
                &self.registry_hash,
                functions,
                declaration.program.clone(),
                adapter,
            )
            .map_err(|_| INVALID)?;
        let rule = registry
            .bind(
                declaration.contract.owner(),
                declaration.contract.behavior(),
                value,
                declaration.contract.signature().kind,
            )
            .map_err(|_| INVALID)?;
        self.check().map_err(|_| INVALID)?;
        let binding = FreshQueryBinding {
            declaration: BindingOwner::Query(declaration),
            scope,
            context: self.context.lease(),
            rule,
            active: Arc::new(AtomicBool::new(true)),
            source_owner: Some(Arc::downgrade(source)),
            _owned_source: own_source.then(|| source.clone()),
            _runtime: RuntimeOwner::new(self, NativeRootKind::Registry)?,
            execution: None,
        };
        self.check().map_err(|_| INVALID)?;
        if !binding.live(self) {
            return Err(INVALID.into());
        }
        Ok(binding)
    }
    pub fn query_invocation(
        &self,
        scope: DeclarationScope,
    ) -> Result<FreshQueryInvocation, String> {
        let _runtime = self.runtime_entry()?;
        self.check().map_err(|_| INVALID)?;
        // The actual scope handle must belong to this host; obtain its tree
        // through a private concrete-host accessor, never an identity lookup.
        let mut query = self
            .inner
            .registered_scope_validator_context(scope)
            .ok_or(INVALID)?;
        query.execution_budget = Some(Default::default());
        self.check().map_err(|_| INVALID)?;
        let invocation = FreshQueryInvocation {
            scope,
            context: self.context.lease(),
            query,
            active: AtomicBool::new(true),
            busy: AtomicBool::new(false),
            _runtime: RuntimeOwner::new(self, NativeRootKind::Preparation)?,
        };
        self.check().map_err(|_| INVALID)?;
        if !invocation._runtime.matches(self) {
            return Err(INVALID.into());
        }
        Ok(invocation)
    }
    pub fn validate_query(
        &self,
        invocation: &FreshQueryInvocation,
        binding: &FreshQueryBinding,
        input: &ValidationInput,
        limits: ValidationLimits,
    ) -> Result<ValidationBatch, String> {
        let _runtime = self.runtime_entry()?;
        let check = || {
            self.check().map_err(|_| INVALID)?;
            if !invocation.active.load(Ordering::Acquire)
                || !invocation._runtime.matches(self)
                || !invocation.context.matches(&self.context)
                || !binding.live(self)
                || invocation.scope != binding.scope
            {
                return Err(INVALID);
            }
            Ok(())
        };
        check()?;
        invocation
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| INVALID)?;
        let _request = QueryEntry(&invocation.busy);
        check()?;
        let runtime = ValidationRuntime {
            control: &self.control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: invocation.query.clone(),
        };
        let result = validate_rules(&[binding.rule.clone()], input, &runtime, limits);
        check()?;
        Ok(result)
    }
}
