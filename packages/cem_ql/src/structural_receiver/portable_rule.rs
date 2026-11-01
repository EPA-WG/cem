//! Actual-host registry owners for transported query/native rule leaves. An
//! imported leaf does not manufacture a consumer datatype or crossing grant.
use super::query::{BindingOwner, RuntimeOwner};
use super::*;
use crate::{
    datatype_validation::{
        DatatypeValidationRegistry, ValidationBatch, ValidationInput, ValidationLimits,
    },
    native_registrations::Requirement,
    rule_source_transport::{self as transport, RuleSource, VerifiedRuleTransport},
    selected_registry::SelectedFunctionRegistry,
};
use cem_ml::{
    runtime_admission::NativeRootKind, schema::attribute_datatypes::AttributeDatatypeContextLease,
};
use std::sync::Weak;
pub enum TransportRuleCode {
    Query(SelectedFunctionRegistry),
    Native(Arc<FreshNativeRuleCodeOwner>),
}
pub struct FreshTransportRuleBinding {
    active: Arc<AtomicBool>,
    packet: Arc<VerifiedRuleTransport>,
    scope: DeclarationScope,
    context: AttributeDatatypeContextLease,
    profile: Arc<FreshQueryResultProfileOwner>,
    query: Option<(Arc<FreshQuerySource>, Arc<FreshQueryCodeOwner>)>,
    native: Option<Weak<FreshNativeRuleCodeOwner>>,
    _source_root: Option<RuntimeOwner>,
    _registry_root: Option<RuntimeOwner>,
    _registry: DatatypeValidationRegistry,
    direct: Option<FreshQueryBinding>,
}
impl FreshTransportRuleBinding {
    pub fn retire(&self) {
        self.active.store(false, Ordering::Release);
        cem_ml::runtime_inventory::note_mutation();
    }
    pub fn is_live_on(&self, host: &FreshStructuralHost) -> bool {
        self.active.load(Ordering::Acquire)
            && host.check().is_ok()
            && self.context.matches(&host.context)
            && self.profile.is_live_on(host)
            && self
                .query
                .as_ref()
                .is_none_or(|(s, c)| s.live(host) && c.is_live_on(host))
            && self.native.as_ref().is_none_or(|n| {
                n.upgrade()
                    .is_some_and(|n| host.native_validator_owner_matches(&n))
            })
            && self.direct.as_ref().is_none_or(|b| b.live(host))
    }
}
impl FreshStructuralHost {
    pub fn link_transported_rule(
        &self,
        packet: Arc<VerifiedRuleTransport>,
        scope: DeclarationScope,
        profile: Arc<FreshQueryResultProfileOwner>,
        code: TransportRuleCode,
    ) -> Result<FreshTransportRuleBinding, String> {
        let _entry = self.runtime_entry()?;
        if !profile.is_live_on(self) || !Arc::ptr_eq(&profile.profile, &packet.profile) {
            return Err(INVALID.into());
        }
        let active = Arc::new(AtomicBool::new(true));
        let adapter = profile.profile.adapter.clone();
        let (registry, contract, declaration, query, native, source_root) =
            match (&packet.source, code) {
                (RuleSource::Query(q), TransportRuleCode::Query(functions)) => {
                    let source = self.query_source(q.clone(), scope)?;
                    let code = self.query_code_owner(&source, &functions)?;
                    let mut registry = DatatypeValidationRegistry::default();
                    registry
                        .register_verified_query(
                            q.contract.clone(),
                            &q.pin().program.source,
                            &self.registry_hash,
                            &functions,
                            q.program.clone(),
                            adapter,
                        )
                        .map_err(|_| INVALID)?;
                    (
                        registry,
                        q.contract.clone(),
                        BindingOwner::Query(q.clone()),
                        Some((source, code)),
                        None,
                        None,
                    )
                }
                (RuleSource::Native(n), TransportRuleCode::Native(code)) => {
                    if n.implementation() != code.identity()
                        || !self.native_validator_owner_matches(&code)
                    {
                        return Err(INVALID.into());
                    }
                    let Requirement::Validator(contract) = &n.requirement else {
                        return Err(INVALID.into());
                    };
                    let registry = self
                        .native_validator_registration(n, scope, adapter, &active)
                        .map_err(|_| INVALID)?;
                    (
                        registry,
                        contract.clone(),
                        BindingOwner::Native(n.clone()),
                        None,
                        Some(Arc::downgrade(&code)),
                        Some(RuntimeOwner::new(self, NativeRootKind::Compilation)?),
                    )
                }
                _ => return Err(INVALID.into()),
            };
        let direct = if let Some(datatype) = packet.pin().datatype {
            let item = match &packet.source {
                RuleSource::Query(q) => {
                    crate::query_validator_capsule::declarations::datatype(q, datatype)?
                }
                RuleSource::Native(n) => {
                    crate::eval::RetainedCemNode::new(n.source().clone(), datatype)
                        .ok_or(INVALID)?
                        .query_item()
                }
            };
            let rule = registry
                .bind(
                    contract.owner(),
                    contract.behavior(),
                    item,
                    contract.signature().kind,
                )
                .map_err(|_| INVALID)?;
            Some(FreshQueryBinding {
                declaration,
                scope,
                context: self.context.lease(),
                rule,
                active: active.clone(),
                source_owner: query.as_ref().map(|(s, _)| Arc::downgrade(s)),
                _owned_source: None,
                _runtime: RuntimeOwner::new(self, NativeRootKind::Registry)?,
                execution: None,
            })
        } else {
            None
        };
        let registry_root = if direct.is_none() {
            Some(RuntimeOwner::new(self, NativeRootKind::Registry)?)
        } else {
            None
        };
        let binding = FreshTransportRuleBinding {
            active,
            packet,
            scope,
            context: self.context.lease(),
            profile,
            query,
            native,
            _source_root: source_root,
            _registry_root: registry_root,
            _registry: registry,
            direct,
        };
        if !binding.is_live_on(self) {
            return Err(INVALID.into());
        }
        Ok(binding)
    }
    pub fn capture_transported_rule(
        &self,
        binding: &FreshTransportRuleBinding,
    ) -> Result<Vec<u8>, String> {
        let _entry = self.runtime_entry()?;
        if !binding.is_live_on(self) {
            return Err(INVALID.into());
        }
        let packet = transport::export(
            &binding.packet.source,
            binding.packet.pin().datatype,
            binding.packet.pin().origin.clone(),
            &binding.profile.profile,
            &self.control,
        )?;
        if packet.bytes != binding.packet.wire.as_ref() || !binding.is_live_on(self) {
            return Err(INVALID.into());
        }
        Ok(packet.bytes)
    }
    pub fn validate_transported_rule(
        &self,
        invocation: &FreshQueryInvocation,
        binding: &FreshTransportRuleBinding,
        input: &ValidationInput,
        limits: ValidationLimits,
    ) -> Result<ValidationBatch, String> {
        if !binding.is_live_on(self) {
            return Err(INVALID.into());
        }
        let direct = binding.direct.as_ref().ok_or(INVALID)?;
        if direct.scope != binding.scope {
            return Err(INVALID.into());
        }
        let result = self.validate_query(invocation, direct, input, limits)?;
        if !binding.is_live_on(self) {
            return Err(INVALID.into());
        }
        Ok(result)
    }
}
