//! Explicit imported rule owners. Compilations retain weak owners and source
//! snapshots; invocation contexts are borrowed afresh from the actual host.
use super::*;
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
    module: String,
    source: Arc<RetainedCemTree>,
    behavior: SchemaDeclarationNode,
    scope: DeclarationScope,
    context: AttributeDatatypeContextLease,
    active: Arc<AtomicBool>,
    registry: DatatypeValidationRegistry,
}
impl FreshDatatypeRuleBinding {
    pub fn retire(&self) {
        self.active.store(false, Ordering::Release);
    }
    pub fn is_live(&self, host: &FreshStructuralHost) -> bool {
        host.check().is_ok()
            && self.active.load(Ordering::Acquire)
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
        self.check().map_err(|_| REFUSED)?;
        if input.registry() != self.registry_hash {
            return Err(REFUSED.into());
        }
        let source = input.view(Some(module))?.tree;
        self.inner
            .registered_validator_context(scope, source)
            .ok_or(REFUSED)?;
        let active = Arc::new(AtomicBool::new(true));
        let (registry, behavior) = match registration {
            DatatypeDependencyRuleRegistration::Query { functions, adapter } => {
                let declaration = input.query(module).ok_or(REFUSED)?;
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
                (registry, declaration.contract.behavior().clone())
            }
            DatatypeDependencyRuleRegistration::NativeValidator { adapter } => {
                let requirement = input.native(module).ok_or(REFUSED)?;
                let Requirement::Validator(contract) = &requirement.requirement else {
                    return Err(REFUSED.into());
                };
                let registry = self
                    .native_validator_registration(requirement, scope, adapter, &active)
                    .map_err(|_| REFUSED)?;
                (registry, contract.behavior().clone())
            }
        };
        let binding = FreshDatatypeRuleBinding {
            module: module.into(),
            source: source.clone(),
            behavior,
            scope,
            context: self.context.lease(),
            active,
            registry,
        };
        if !binding.is_live(self) {
            return Err(REFUSED.into());
        }
        Ok(binding)
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
}
impl RuleValidationContexts for Contexts<'_> {
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
