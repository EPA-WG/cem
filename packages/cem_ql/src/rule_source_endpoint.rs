//! Original deployment/code owners for complete rule/profile byte transport.
use crate::{
    datatype_validation::{ValidationBatch, ValidationInput, ValidationLimits},
    rule_source_transport::{self as transport, RuleSource, RuleTransportPin},
    selected_registry::SelectedFunctionRegistry,
    structural_receiver::{
        FreshNativeRuleCodeOwner, FreshQueryInvocation, FreshStructuralHost,
        FreshTransportRuleBinding, TransportRuleCode,
    },
};
use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    runtime_admission::{generation::NativeGeneration, NativeResourceLease, NativeRootKind},
    schema::reference_policy::ReferenceScopePolicy,
};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Weak,
};
const INVALID: &str = "cem.rule.endpoint.refused";
#[cfg(target_arch = "wasm32")]
pub mod wasm;
pub struct RuleSourceEndpoint {
    pub(crate) generation: Weak<NativeGeneration>,
    control: OperationControl,
    pin: RuleTransportPin,
    functions: SelectedFunctionRegistry,
    native: Option<Weak<FreshNativeRuleCodeOwner>>,
    active: AtomicBool,
    sessions: Arc<AtomicUsize>,
    _lease: NativeResourceLease,
}
pub struct RuleSourceSession {
    endpoint: Weak<RuleSourceEndpoint>,
    host: FreshStructuralHost,
    binding: FreshTransportRuleBinding,
    request: Option<FreshQueryInvocation>,
    _slot: Slot,
}
struct Slot(Arc<AtomicUsize>);
impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
        cem_ml::runtime_inventory::note_mutation();
    }
}
impl RuleSourceEndpoint {
    pub fn new(
        g: &Arc<NativeGeneration>,
        control: OperationControl,
        pin: RuleTransportPin,
        functions: SelectedFunctionRegistry,
        native: Option<&Arc<FreshNativeRuleCodeOwner>>,
    ) -> Result<Arc<Self>, String> {
        let _entry = g.admit().map_err(|_| INVALID)?;
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID)?;
        if serde_json::to_vec(&pin).map_err(|_| INVALID)?.len() > 16384 {
            return Err(INVALID.into());
        }
        Ok(Arc::new(Self {
            generation: Arc::downgrade(g),
            control,
            pin,
            functions,
            native: native.map(Arc::downgrade),
            active: AtomicBool::new(true),
            sessions: Arc::new(AtomicUsize::new(0)),
            _lease: g.resource(NativeRootKind::Registry).map_err(|_| INVALID)?,
        }))
    }
    fn check(&self) -> Result<(), String> {
        if !self.active.load(Ordering::Acquire)
            || !self.generation.upgrade().is_some_and(|g| g.is_open())
            || self
                .native
                .as_ref()
                .is_some_and(|n| !n.upgrade().is_some_and(|n| n.is_live()))
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
    pub fn open(self: &Arc<Self>, bytes: &[u8], pins: &str) -> Result<RuleSourceSession, String> {
        self.check()?;
        let g = self.generation.upgrade().ok_or(INVALID)?;
        let _entry = g.admit().map_err(|_| INVALID)?;
        if bytes.len() > transport::MAX_BYTES || pins.len() > 16384 {
            return Err(INVALID.into());
        }
        let pin: RuleTransportPin = serde_json::from_str(pins).map_err(|_| INVALID)?;
        if transport::encoded(&pin)? != transport::encoded(&self.pin)? {
            return Err(INVALID.into());
        }
        self.sessions
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < 32).then_some(n + 1)
            })
            .map_err(|_| INVALID)?;
        let slot = Slot(self.sessions.clone());
        let packet = transport::verify(bytes, &self.pin, &self.control)?;
        let mut host = FreshStructuralHost::new_in_generation(
            match &packet.source {
                RuleSource::Query(q) => &q.pin().program.registry_hash,
                RuleSource::Native(n) => &n.pin().registry_hash,
            },
            self.control.clone(),
            &g,
        )?;
        let code = match &packet.source {
            RuleSource::Query(_) => {
                if self.native.is_some() {
                    return Err(INVALID.into());
                }
                TransportRuleCode::Query(self.functions.clone())
            }
            RuleSource::Native(n) => {
                let native = self
                    .native
                    .as_ref()
                    .and_then(Weak::upgrade)
                    .ok_or(INVALID)?;
                if n.implementation() != native.identity() {
                    return Err(INVALID.into());
                }
                host.register_native_validator_owner(&native)
                    .map_err(|_| INVALID)?;
                TransportRuleCode::Native(native)
            }
        };
        let (tree, capture) = match &packet.source {
            RuleSource::Query(q) => (q.source(), q.capture()),
            RuleSource::Native(n) => (n.source(), n.capture()),
        };
        let scope = host.register_captured_scope(
            tree.clone(),
            capture.clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().map_err(|_| INVALID)?,
        )?;
        let scopes = packet
            .profile
            .sources()
            .iter()
            .map(|s| {
                host.register_captured_scope(
                    s.tree.clone(),
                    s.capture.clone(),
                    Some(Default::default()),
                    ReferenceScopePolicy::schema_defaults().map_err(|_| INVALID)?,
                )
            })
            .collect::<Result<Vec<_>, String>>()?;
        let profile = host.query_result_profile_owner(
            packet.profile.clone(),
            &scopes,
            &self.pin.profile.profile,
        )?;
        let request = if self.pin.datatype.is_some() {
            Some(host.query_invocation(scope)?)
        } else {
            None
        };
        let binding = host.link_transported_rule(packet, scope, profile, code)?;
        self.check()?;
        Ok(RuleSourceSession {
            endpoint: Arc::downgrade(self),
            host,
            binding,
            request,
            _slot: slot,
        })
    }
}
impl RuleSourceSession {
    fn endpoint(&self) -> Result<Arc<RuleSourceEndpoint>, String> {
        let endpoint = self.endpoint.upgrade().ok_or(INVALID)?;
        endpoint.check()?;
        Ok(endpoint)
    }
    pub fn capture(&self) -> Result<Vec<u8>, String> {
        let endpoint = self.endpoint()?;
        let bytes = self.host.capture_transported_rule(&self.binding)?;
        endpoint.check()?;
        Ok(bytes)
    }
    pub fn validate(
        &self,
        input: &ValidationInput,
        limits: ValidationLimits,
    ) -> Result<ValidationBatch, String> {
        let endpoint = self.endpoint()?;
        let batch = self.host.validate_transported_rule(
            self.request.as_ref().ok_or(INVALID)?,
            &self.binding,
            input,
            limits,
        )?;
        endpoint.check()?;
        Ok(batch)
    }
}
