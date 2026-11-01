//! Explicit imported rule owners. Compilations retain weak owners and source
//! snapshots; invocation contexts are borrowed afresh from the actual host.
use super::*;
use crate::structural_receiver::query::{QuerySourceLease, RuntimeOwner};
use crate::{
    api::EvaluationContext,
    datatype_results::DatatypeResultAdapter,
    datatype_validation::{DatatypeValidationRegistry, RuleValidationContexts},
    native_registrations::Requirement,
    selected_registry::SelectedFunctionRegistry,
};
use std::sync::Weak;

/// An explicit execution selection for one verified dependency. Native callbacks
/// must already be registered on the host under the exact NR1 identity.
pub enum DatatypeDependencyRuleRegistration {
    QueryProfile { functions: SelectedFunctionRegistry, profile: Arc<crate::structural_receiver::FreshQueryResultProfileOwner> },
    NativeProfile { code: Arc<crate::structural_receiver::FreshNativeRuleCodeOwner>, profile: Arc<crate::structural_receiver::FreshQueryResultProfileOwner> },
    Query {
        functions: SelectedFunctionRegistry,
        adapter: DatatypeResultAdapter,
    },
    NativeValidator {
        adapter: DatatypeResultAdapter,
    },
}
/// Retain this explicit owner while using a compilation. Dropping/retiring it
/// invalidates every dependent snapshot without retaining the embedding callback.
pub struct FreshDatatypeRuleBinding {
    profile_owner: Option<Weak<crate::structural_receiver::FreshQueryResultProfileOwner>>,
    native_owner: Option<Weak<crate::structural_receiver::FreshNativeRuleCodeOwner>>,
    module: String,
    source: Arc<RetainedCemTree>,
    behavior: SchemaDeclarationNode,
    scope: DeclarationScope,
    context: AttributeDatatypeContextLease,
    active: Arc<AtomicBool>,
    registry: DatatypeValidationRegistry,
    source_owner: Option<QuerySourceLease>,
    _runtime: RuntimeOwner,
}
impl FreshDatatypeRuleBinding {
    pub fn retire(&self) {
        self.active.store(false, Ordering::Release);
    }
    fn owners_current(&self) -> bool {
        self._runtime.current() && self.source_owner.as_ref().is_none_or(|s| s.current())
    }
    pub fn is_live(&self, host: &FreshStructuralHost) -> bool {
        host.check().is_ok()
            && self.active.load(Ordering::Acquire)
            && self.owners_current()
            && self.profile_owner.as_ref().is_none_or(|p| p.upgrade().is_some_and(|p| p.is_live_on(host)))
            && self.native_owner.as_ref().is_none_or(|n| n.upgrade().is_some_and(|n| host.native_validator_owner_matches(&n)))
            && self._runtime.matches(host)
            && self.context.matches(&host.context)
            && host
                .inner
                .registered_validator_context(self.scope, &self.source)
                .is_some()
    }
}
impl Drop for FreshDatatypeRuleBinding {
    fn drop(&mut self) {
        self.retire();
    }
}
impl FreshStructuralHost {
    pub fn link_datatype_dependency_rule(
        &self,
        capsule: &VerifiedDatatypeDependencies,
        module: &str,
        scope: DeclarationScope,
        registration: DatatypeDependencyRuleRegistration,
    ) -> Result<FreshDatatypeRuleBinding, String> {
        self.link_input_rule(InputRef::Dependencies(capsule), module, scope, registration)
    }
    pub(super) fn link_input_rule(
        &self,
        input: InputRef<'_>,
        module: &str,
        scope: DeclarationScope,
        registration: DatatypeDependencyRuleRegistration,
    ) -> Result<FreshDatatypeRuleBinding, String> {
        let _runtime = self.runtime_entry()?;
        self.check().map_err(|_| REFUSED)?;
        if input.registry() != self.registry_hash {
            return Err(REFUSED.into());
        }
        let source = input.view(Some(module))?.tree;
        self.inner
            .registered_validator_context(scope, source)
            .ok_or(REFUSED)?;
        let active = Arc::new(AtomicBool::new(true));
        let (registration, profile_owner, native_owner) = match registration {
            DatatypeDependencyRuleRegistration::QueryProfile { functions, profile } => {
                if !profile.is_live_on(self) { return Err(REFUSED.into()); }
                (DatatypeDependencyRuleRegistration::Query { functions, adapter: profile.profile.adapter.clone() }, Some(Arc::downgrade(&profile)), None)
            },
            DatatypeDependencyRuleRegistration::NativeProfile { code, profile } => {
                if !profile.is_live_on(self) || !self.native_validator_owner_matches(&code)
                    || input.native(module).is_none_or(|n| n.implementation() != code.identity()) { return Err(REFUSED.into()); }
                (DatatypeDependencyRuleRegistration::NativeValidator { adapter: profile.profile.adapter.clone() }, Some(Arc::downgrade(&profile)), Some(Arc::downgrade(&code)))
            },
            registration => (registration, None, None),
        };
        let (registry, behavior, source_owner) = match registration {
            DatatypeDependencyRuleRegistration::Query { functions, adapter } => {
                let declaration = input.query(module).ok_or(REFUSED)?;
                let source_owner = self.query_source_lease(declaration, scope)?;
                let mut registry = DatatypeValidationRegistry::default();
                registry
                    .register_verified_query(
                        declaration.contract.clone(),
                        &declaration.pin().program.source,
                        &self.registry_hash,
                        &functions,
                        declaration.program.clone(),
                        adapter,
                    )
                    .map_err(|_| REFUSED)?;
                (
                    registry,
                    declaration.contract.behavior().clone(),
                    Some(source_owner),
                )
            }
            DatatypeDependencyRuleRegistration::NativeValidator { adapter } => {
                let requirement = input.native(module).ok_or(REFUSED)?;
                let Requirement::Validator(contract) = &requirement.requirement else {
                    return Err(REFUSED.into());
                };
                let registry = self
                    .native_validator_registration(requirement, scope, adapter, &active)
                    .map_err(|_| REFUSED)?;
                (registry, contract.behavior().clone(), None)
            }
            _ => return Err(REFUSED.into()),
        };
        let binding = FreshDatatypeRuleBinding {
            profile_owner, native_owner,
            module: module.into(),
            source: source.clone(),
            behavior,
            scope,
            context: self.context.lease(),
            active,
            registry,
            source_owner,
            _runtime: RuntimeOwner::new(self, cem_ml::runtime_admission::NativeRootKind::Registry)?,
        };
        if !binding.is_live(self) {
            return Err(REFUSED.into());
        }
        Ok(binding)
    }
    pub fn capture_datatype_dependency_rule(&self, capsule: &VerifiedDatatypeDependencies, binding: &FreshDatatypeRuleBinding) -> Result<crate::rule_source_transport::BuiltRuleTransport, String> {
        use crate::rule_source_transport::{self as t, RuleSourceView, RuleOrigin};
        let _entry = self.runtime_entry()?;
        let input = InputRef::Dependencies(capsule);
        let module = &binding.module;
        if !binding.is_live(self) || !Arc::ptr_eq(input.view(Some(module))?.tree, &binding.source)
            || input.rule_behavior(module).is_none_or(|b| b.identity() != binding.behavior.identity()) { return Err(REFUSED.into()); }
        let profile = binding.profile_owner.as_ref().and_then(Weak::upgrade).ok_or(REFUSED)?;
        let source = if let Some(q) = input.query(module) { RuleSourceView::Query(q) }
            else { if binding.native_owner.is_none() { return Err(REFUSED.into()); } RuleSourceView::Native(input.native(module).ok_or(REFUSED)?) };
        let packet = t::export(source, None, RuleOrigin::Imported { module: module.clone() }, &profile.profile, &self.control)?;
        if !binding.is_live(self) { return Err(REFUSED.into()); }
        Ok(packet)
    }
    pub fn link_transported_dependency_rule(&self, capsule: &VerifiedDatatypeDependencies, module: &str, scope: DeclarationScope,
        packet: &crate::rule_source_transport::VerifiedRuleTransport, profile: Arc<crate::structural_receiver::FreshQueryResultProfileOwner>,
        code: crate::structural_receiver::TransportRuleCode) -> Result<FreshDatatypeRuleBinding, String> {
        use crate::{rule_source_transport::{self as t, RuleOrigin, RuleSourcePin}, structural_receiver::TransportRuleCode};
        if packet.pin().origin != (RuleOrigin::Imported { module: module.into() }) || !Arc::ptr_eq(&profile.profile, packet.profile()) { return Err(REFUSED.into()); }
        let input = InputRef::Dependencies(capsule);
        let registration = match (&packet.pin().source, code) {
            (RuleSourcePin::Query(pin), TransportRuleCode::Query(functions)) => {
                if t::encoded(pin)? != t::encoded(input.query(module).ok_or(REFUSED)?.pin())? { return Err(REFUSED.into()); }
                DatatypeDependencyRuleRegistration::QueryProfile { functions, profile }
            },
            (RuleSourcePin::Native(pin), TransportRuleCode::Native(code)) => {
                if t::encoded(pin)? != t::encoded(input.native(module).ok_or(REFUSED)?.pin())? { return Err(REFUSED.into()); }
                DatatypeDependencyRuleRegistration::NativeProfile { code, profile }
            }, _ => return Err(REFUSED.into()),
        };
        self.link_input_rule(input, module, scope, registration)
    }
}
pub(super) type RuleOwners = BTreeMap<String, Weak<FreshDatatypeRuleBinding>>;
pub(super) fn registrations(
    host: &FreshStructuralHost,
    input: InputRef<'_>,
    scopes: &BTreeMap<String, DeclarationScope>,
    bindings: &BTreeMap<String, Arc<FreshDatatypeRuleBinding>>,
    required: &BTreeSet<String>,
) -> Result<(DatatypeValidationRegistry, RuleOwners), String> {
    if required.len() != bindings.len() || required.iter().any(|id| !bindings.contains_key(id)) {
        return Err(REFUSED.into());
    }
    let mut registry = DatatypeValidationRegistry::default();
    let mut owners = BTreeMap::new();
    for (id, binding) in bindings {
        if !binding.is_live(host)
            || binding.module != *id
            || scopes.get(id) != Some(&binding.scope)
            || !Arc::ptr_eq(input.view(Some(id))?.tree, &binding.source)
            || input
                .rule_behavior(id)
                .is_none_or(|n| n.identity() != binding.behavior.identity())
        {
            return Err(REFUSED.into());
        }
        registry
            .import_registrations(&binding.registry)
            .map_err(|_| REFUSED)?;
        if owners
            .insert(binding.behavior.identity(), Arc::downgrade(binding))
            .is_some()
        {
            return Err(REFUSED.into());
        }
    }
    Ok((registry, owners))
}
pub(super) struct Contexts<'a> {
    pub host: &'a FreshStructuralHost,
    pub owners: &'a RuleOwners,
    pub source_operations: bool,
}
impl RuleValidationContexts for Contexts<'_> {
    fn operation_context(
        &self,
        source: &SchemaDeclarationNode,
    ) -> Result<Option<EvaluationContext>, ()> {
        if !self.source_operations {
            return Ok(None);
        }
        self.host.check().map_err(|_| ())?;
        let scope = self
            .host
            .inner
            .scope(&self.host.inner.source_reference(source.clone()))
            .ok_or(())?;
        self.host
            .inner
            .registered_scope_validator_context(scope)
            .map(Some)
            .ok_or(())
    }

    fn context(&self, behavior: &SchemaDeclarationNode) -> Option<EvaluationContext> {
        let owner = self.owners.get(&behavior.identity())?.upgrade()?;
        if !owner.is_live(self.host) {
            return None;
        }
        self.host
            .inner
            .registered_validator_context(owner.scope, &owner.source)
    }
    fn is_available(&self, behavior: &SchemaDeclarationNode) -> bool {
        self.owners
            .get(&behavior.identity())
            .and_then(Weak::upgrade)
            .is_some_and(|b| b.is_live(self.host))
    }
}

/// Actual source contexts borrowed for one native invocation, never retained in
/// descriptors. The resolver itself remains the mutable original host.
pub(super) struct CompilationContexts<'a> {
    pub sources: Vec<(Arc<RetainedCemTree>, EvaluationContext)>,
    pub owners: &'a RuleOwners,
    pub lease: &'a AttributeDatatypeContextLease,
    pub control: &'a OperationControl,
}
impl CompilationContexts<'_> {
    fn source(&self, n: &SchemaDeclarationNode) -> Option<EvaluationContext> {
        if !self.lease.is_live() || self.control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_err() {
            return None;
        }
        self.sources
            .iter()
            .find(|(tree, _)| Arc::ptr_eq(tree.ast_owner(), n.document()))
            .map(|(_, c)| c.clone())
    }
}
impl RuleValidationContexts for CompilationContexts<'_> {
    fn context(&self, n: &SchemaDeclarationNode) -> Option<EvaluationContext> {
        if self.is_available(n) {
            self.source(n)
        } else {
            None
        }
    }
    fn is_available(&self, n: &SchemaDeclarationNode) -> bool {
        self.source(n).is_some()
            && self
                .owners
                .get(&n.identity())
                .and_then(Weak::upgrade)
                .is_some_and(|o| {
                    o.active.load(Ordering::Acquire) && o.context.is_live() && o.owners_current()
                })
    }
    fn operation_context(
        &self,
        n: &SchemaDeclarationNode,
    ) -> Result<Option<EvaluationContext>, ()> {
        self.source(n).map(Some).ok_or(())
    }
}
