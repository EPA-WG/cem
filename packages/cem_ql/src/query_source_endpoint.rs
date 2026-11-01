//! Provisioned pure-query transport. Only an original native deployment owner
//! can authorize source reimport; packet pins are agreement, never authority.
use crate::{
    datatype_validation::{ValidationBatch, ValidationInput, ValidationLimits},
    query_validator_capsule::{
        handoff,
        profiles::{QueryExecutionPins, VerifiedQueryResultProfile},
    },
    selected_registry::SelectedFunctionRegistry,
    structural_receiver::{
        FreshQueryBinding, FreshQueryCodeOwner, FreshQueryInvocation, FreshQueryRegistryOwner,
        FreshQueryResultProfileOwner, FreshQuerySource, FreshStructuralHost,
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

#[cfg(target_arch = "wasm32")]
pub mod wasm;
const INVALID: &str = "cem.query.endpoint.refused";
pub const MAX_PINS_BYTES: usize = 16 * 1024;
// Seven roots per session plus the deployment root fit the existing 256-root
// generation ceiling. Other native work can still exhaust that shared ceiling.
pub const MAX_SESSIONS: usize = 32;

/// Non-serialized caller-held authority. No general schema or function registry
/// is loaded here; the trusted embedding supplies its original actual profile.
pub struct QuerySourceEndpoint {
    pub(crate) generation: Weak<NativeGeneration>,
    control: OperationControl,
    pins: QueryExecutionPins,
    pin_bytes: Vec<u8>,
    functions: SelectedFunctionRegistry,
    profile: Arc<VerifiedQueryResultProfile>,
    live: AtomicBool,
    sessions: Arc<AtomicUsize>,
    _lease: NativeResourceLease,
}
pub struct QuerySourceSession {
    endpoint: Weak<QuerySourceEndpoint>,
    host: FreshStructuralHost,
    binding: FreshQueryBinding,
    request: FreshQueryInvocation,
    _source: Arc<FreshQuerySource>,
    _code: Arc<FreshQueryCodeOwner>,
    _profile: Arc<FreshQueryResultProfileOwner>,
    _registry: Arc<FreshQueryRegistryOwner>,
    _slot: SessionSlot,
}
struct SessionSlot(Arc<AtomicUsize>);
impl Drop for SessionSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
        cem_ml::runtime_inventory::note_mutation();
    }
}
impl QuerySourceEndpoint {
    pub fn new(
        generation: &Arc<NativeGeneration>,
        control: OperationControl,
        pins: QueryExecutionPins,
        functions: SelectedFunctionRegistry,
        profile: Arc<VerifiedQueryResultProfile>,
    ) -> Result<Arc<Self>, String> {
        let _entry = generation.admit().map_err(|_| INVALID)?;
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID)?;
        pins.check()?;
        let pin_bytes = serde_json::to_vec(&pins).map_err(|_| INVALID)?;
        if profile.pin() != &pins.result_profile || pin_bytes.len() > MAX_PINS_BYTES {
            return Err(INVALID.into());
        }
        Ok(Arc::new(Self {
            generation: Arc::downgrade(generation),
            control,
            pins,
            pin_bytes,
            functions,
            profile,
            live: AtomicBool::new(true),
            sessions: Arc::new(AtomicUsize::new(0)),
            _lease: generation
                .resource(NativeRootKind::Registry)
                .map_err(|_| INVALID)?,
        }))
    }
    fn check(&self) -> Result<(), String> {
        if !self.live.load(Ordering::Acquire)
            || !self.generation.upgrade().is_some_and(|g| g.is_open())
        {
            return Err(INVALID.into());
        }
        self.control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID.into())
    }
    pub fn retire(&self) {
        self.live.store(false, Ordering::Release);
        cem_ml::runtime_inventory::note_mutation();
    }
    pub fn open(self: &Arc<Self>, bytes: &[u8], pins: &str) -> Result<QuerySourceSession, String> {
        self.check()?;
        let generation = self.generation.upgrade().ok_or(INVALID)?;
        let _entry = generation.admit().map_err(|_| INVALID)?;
        if bytes.len() > handoff::MAX_BYTES || pins.len() > MAX_PINS_BYTES {
            return Err(INVALID.into());
        }
        let supplied: QueryExecutionPins = serde_json::from_str(pins).map_err(|_| INVALID)?;
        supplied.check()?;
        if serde_json::to_vec(&supplied).map_err(|_| INVALID)? != self.pin_bytes {
            return Err(INVALID.into());
        }
        self.sessions
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < MAX_SESSIONS).then_some(n + 1)
            })
            .map_err(|_| INVALID)?;
        let slot = SessionSlot(self.sessions.clone());
        let source = handoff::verify(bytes, &self.pins.source, &self.control)?;
        if source.selection.result != self.pins.result_profile.result
            || source.selection.diagnostic != self.pins.result_profile.diagnostic
        {
            return Err(INVALID.into());
        }
        let d = source.declaration();
        let mut host = FreshStructuralHost::new_in_generation(
            &d.pin().program.registry_hash,
            self.control.clone(),
            &generation,
        )?;
        let scope = host.register_captured_scope(
            d.source().clone(),
            d.capture().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().map_err(|_| INVALID)?,
        )?;
        let scopes = self
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
        let source_owner = host.query_source(d.clone(), scope)?;
        let code = host.query_code_owner(&source_owner, &self.functions)?;
        let profile = host.query_result_profile_owner(
            self.profile.clone(),
            &scopes,
            &self.pins.result_profile,
        )?;
        let registry = host.query_registry_owner(&source_owner, &code, &profile)?;
        let binding = host.link_query_owned(&source_owner, source.datatype(), &registry)?;
        let request = host.query_invocation(scope)?;
        self.check()?;
        Ok(QuerySourceSession {
            endpoint: Arc::downgrade(self),
            host,
            binding,
            request,
            _source: source_owner,
            _code: code,
            _profile: profile,
            _registry: registry,
            _slot: slot,
        })
    }
}
impl QuerySourceSession {
    fn endpoint(&self) -> Result<Arc<QuerySourceEndpoint>, String> {
        let endpoint = self.endpoint.upgrade().ok_or(INVALID)?;
        endpoint.check()?;
        Ok(endpoint)
    }
    pub fn capture(&self) -> Result<Vec<u8>, String> {
        let endpoint = self.endpoint()?;
        let bytes = self.host.capture_query_handoff(&self.binding)?.bytes;
        endpoint.check()?;
        Ok(bytes)
    }
    /// Typed native input only. The transport does not parse or retain a JS data
    /// object, reset request budgets, or include values in source checkpoints.
    pub fn validate(
        &self,
        input: &ValidationInput,
        limits: ValidationLimits,
    ) -> Result<ValidationBatch, String> {
        let endpoint = self.endpoint()?;
        let batch = self
            .host
            .validate_query(&self.request, &self.binding, input, limits)?;
        endpoint.check()?;
        Ok(batch)
    }
}
