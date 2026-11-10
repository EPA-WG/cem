//! Portable passive sources for one original receiver frame. The wire carries
//! selectors, while the trusted embedding retains a short-lived native seal.
use super::*;
use crate::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    application_source_transport::{Record, SourceInput},
    eval::{ItemStream, RetainedCemNode},
    types::Type,
};
use cem_ml::{
    committed_checkpoint::{self, OperationOutcome},
    content_cache::ContentHash,
    parser::{AstNodeId, CemAstNode},
    schema::native_source::dependencies::NativeDependencySourceSnapshot,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const REFUSED: &str = "cem.application.receiver_checkpoint.refused";
const MAGIC: &[u8] = b"CEMRC1\n";
const MAX_BYTES: usize = cem_ml::committed_checkpoint::MAX_CHECKPOINT_BYTES;
const MAX_WORKSPACE_BYTES: usize = cem_ml::committed_checkpoint::MAX_WORKSPACE_CHECKPOINT_BYTES;
const MAX_SOURCES: usize = 16;
const MAX_NODES: usize = 16_384;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selector {
    uri: String,
    node: AstNodeId,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet {
    deployment: String,
    tag: String,
    input: Selector,
    bindings: BTreeMap<String, Selector>,
    sources: BTreeMap<String, Record>,
}

/// Original native authorization for one exact packet. The bytes may be
/// transported separately. Only an embedding can hold and consume this seal.
pub struct ReceiverCheckpointSeal {
    digest: ContentHash,
    byte_length: usize,
    declaration: String,
    origin: Weak<NativeGeneration>,
    epoch: Arc<AtomicUsize>,
    revision: usize,
    control: OperationControl,
    authority: OperationControl,
    browser_instance: Option<String>,
}

pub struct VerifiedReceiverCheckpoint {
    packet: PacketOwners,
    admission: ReceiverCheckpointAdmission,
}

enum ReceiverCheckpointAdmission {
    Local(ReceiverCheckpointSeal),
    Physical {
        generation: Weak<NativeGeneration>,
        control: OperationControl,
        byte_length: usize,
        instance_id: String,
    },
}

impl ReceiverCheckpointAdmission {
    fn check(&self) -> Result<(), String> {
        match self {
            Self::Local(seal) => seal.check(),
            Self::Physical { control, .. } => check(control),
        }
    }

    fn control(&self) -> &OperationControl {
        match self {
            Self::Local(seal) => &seal.control,
            Self::Physical { control, .. } => control,
        }
    }
}

/// This pin is issued by the original native owner. It must reach the fresh
/// embedding through the original physical recovery transaction, independently
/// of the packet and journal. Deserializing a caller-supplied pin grants nothing.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiverRecoveryPin {
    receiver: ContentHash,
    journal: ContentHash,
    declaration: String,
    runtime_id: String,
    instance_id: String,
}

/// A fresh owner claims a new operation before delivering a write. The original
/// terminal journal and accepted sequence fence both explicit and implicit replay.
pub struct ReceiverWriteJournal {
    sequence: u64,
    captured_sequence: u64,
    operations: BTreeSet<String>,
}

impl ReceiverWriteJournal {
    pub fn claim_write(&mut self, sequence: u64, operation_id: &str) -> bool {
        if sequence <= self.captured_sequence
            || sequence < self.sequence
            || sequence > 9_007_199_254_740_991
            || operation_id.is_empty()
            || operation_id.len() > 1024
            || operation_id.chars().any(char::is_control)
            || self.operations.len() >= 256
        {
            return false;
        }
        self.sequence = sequence;
        self.operations.insert(operation_id.into())
    }
}

struct PacketOwners {
    deployment: String,
    tag: String,
    input: Selector,
    bindings: BTreeMap<String, Selector>,
    sources: BTreeMap<String, SourceInput>,
}

fn check(control: &OperationControl) -> Result<(), String> {
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|_| REFUSED.into())
}

impl ReceiverCheckpointSeal {
    fn check(&self) -> Result<(), String> {
        if self.epoch.load(Ordering::Acquire) != self.revision {
            return Err(REFUSED.into());
        }
        check(&self.authority)?;
        check(&self.control)
    }

    /// Open only the bytes attested by this original, single-use native seal.
    /// Decoding restores passive owners before the fresh embedding links scopes.
    pub fn verify(self, bytes: &[u8]) -> Result<VerifiedReceiverCheckpoint, String> {
        self.check()?;
        if bytes.len() > MAX_BYTES || ContentHash::from_blake3(bytes) != self.digest {
            return Err(REFUSED.into());
        }
        let packet = decode_packet(bytes, &mut || self.check())?;
        self.check()?;
        Ok(VerifiedReceiverCheckpoint {
            packet,
            admission: ReceiverCheckpointAdmission::Local(self),
        })
    }

    pub fn revision(&self) -> String {
        self.digest.header_value()
    }

    /// Consume the original local seal immediately before physical retirement.
    fn confirm_physical_handoff(
        self,
        bytes: &[u8],
        journal: &[u8],
        runtime_id: &str,
        instance_id: &str,
    ) -> Result<ReceiverRecoveryPin, String> {
        self.check()?;
        if bytes.len() != self.byte_length
            || journal.len() > MAX_WORKSPACE_BYTES
            || ContentHash::from_blake3(bytes) != self.digest
        {
            return Err(REFUSED.into());
        }
        let pin = ReceiverRecoveryPin {
            receiver: self.digest.clone(),
            journal: ContentHash::from_blake3(journal),
            declaration: self.declaration.clone(),
            runtime_id: runtime_id.into(),
            instance_id: instance_id.into(),
        };
        pin.read_journal(journal)?;
        self.check()?;
        Ok(pin)
    }
}

impl ReceiverRecoveryPin {
    fn read_journal(&self, bytes: &[u8]) -> Result<ReceiverWriteJournal, String> {
        let checkpoint = committed_checkpoint::read_sealed(bytes)?;
        let receipt = checkpoint.receipt;
        let owner = receipt
            .owners
            .iter()
            .find(|owner| owner.instance_id == self.instance_id)
            .ok_or(REFUSED)?;
        if receipt.runtime_id != self.runtime_id
            || owner.declaration_identity != self.declaration
            || owner.data_revision != self.receiver.header_value()
            || receipt
                .operations
                .iter()
                .any(|op| op.outcome == OperationOutcome::Unknown)
        {
            return Err(REFUSED.into());
        }
        Ok(ReceiverWriteJournal {
            sequence: owner.accepted_event_sequence,
            captured_sequence: owner.accepted_event_sequence,
            operations: receipt
                .operations
                .into_iter()
                .filter(|op| op.instance_id == self.instance_id)
                .map(|op| op.operation_id)
                .collect(),
        })
    }

    /// The trusted embedding supplies the original out-of-band pin and the
    /// actual newly allocated generation. Only passive owners are decoded here.
    pub fn verify(
        self,
        bytes: &[u8],
        journal: &[u8],
        previous_runtime_id: &str,
        generation: &Arc<NativeGeneration>,
        control: &OperationControl,
    ) -> Result<(VerifiedReceiverCheckpoint, ReceiverWriteJournal), String> {
        let _entry = generation.admit().map_err(|_| REFUSED)?;
        check(control)?;
        if bytes.len() > MAX_BYTES
            || journal.len() > MAX_WORKSPACE_BYTES
            || self.runtime_id != previous_runtime_id
            || ContentHash::from_blake3(bytes) != self.receiver
            || ContentHash::from_blake3(journal) != self.journal
        {
            return Err(REFUSED.into());
        }
        let journal = self.read_journal(journal)?;
        let packet = decode_packet(bytes, &mut || check(control))?;
        check(control)?;
        Ok((
            VerifiedReceiverCheckpoint {
                packet,
                admission: ReceiverCheckpointAdmission::Physical {
                    generation: Arc::downgrade(generation),
                    control: control.clone(),
                    byte_length: bytes.len(),
                    instance_id: self.instance_id,
                },
            },
            journal,
        ))
    }
}

fn decode_packet(
    bytes: &[u8],
    poll: &mut dyn FnMut() -> Result<(), String>,
) -> Result<PacketOwners, String> {
    let body = bytes.strip_prefix(MAGIC).ok_or(REFUSED)?;
    cem_ml::native_binary::preflight_messagepack(body, 1_000_000, 64, &mut *poll)?;
    let mut decoder = rmp_serde::Deserializer::from_read_ref(body);
    decoder.set_max_depth(64);
    let packet = Packet::deserialize(&mut decoder).map_err(|_| REFUSED)?;
    if transport::encode(&packet)? != body
        || packet.sources.len() > MAX_SOURCES
        || packet
            .sources
            .values()
            .map(|s| s.snapshot.node_count())
            .sum::<usize>()
            > MAX_NODES
    {
        return Err(REFUSED.into());
    }
    let sources: BTreeMap<String, SourceInput> = packet
        .sources
        .into_iter()
        .map(|(uri, record)| {
            if uri != record.identity.uri {
                return Err(REFUSED.into());
            }
            let source = record.restore(poll)?;
            NativeDependencySourceSnapshot::check_checkpoint_privacy(
                &source.capture,
                &source.identity,
                poll,
            )?;
            Ok((uri, source))
        })
        .collect::<Result<_, String>>()?;
    if !sources.contains_key(&packet.input.uri) {
        return Err(REFUSED.into());
    }
    Ok(PacketOwners {
        deployment: packet.deployment,
        tag: packet.tag,
        input: packet.input,
        bindings: packet.bindings,
        sources,
    })
}

impl VerifiedReceiverCheckpoint {
    pub fn input_source(&self) -> &SourceInput {
        &self.packet.sources[&self.packet.input.uri]
    }

    pub fn sources(&self) -> impl Iterator<Item = &SourceInput> {
        self.packet.sources.values()
    }

    /// Passive input bindings for the trusted bootstrap to install in the
    /// fresh scopes. Registering sources and granting crossings remain explicit.
    pub fn input_context(&self) -> Result<StandaloneExpressionContext, String> {
        self.admission.check()?;
        let mut context = StandaloneExpressionContext::default();
        for (name, selector) in &self.packet.bindings {
            let root = RetainedCemNode::new(
                self.packet
                    .sources
                    .get(&selector.uri)
                    .ok_or(REFUSED)?
                    .tree
                    .clone(),
                selector.node,
            )
            .ok_or(REFUSED)?;
            context = context.with_binding(
                name,
                StandaloneExpressionBinding::any(ItemStream::once(root.query_item())),
            );
        }
        Ok(context)
    }
}

fn checkpoint_budget(
    sources: &VerifiedApplicationSources,
    available: usize,
) -> Result<usize, String> {
    if available > MAX_WORKSPACE_BYTES {
        return Err(REFUSED.into());
    }
    available
        .checked_sub(sources.bytes().len() + encoded(sources.pin())?.len())
        .map(|bytes| bytes.min(MAX_BYTES))
        .ok_or_else(|| REFUSED.into())
}

fn deployment_pin(sources: &VerifiedApplicationSources) -> Result<String, String> {
    Ok(ContentHash::from_blake3(&encoded(sources.pin())?).header_value())
}

fn binding_selectors(
    context: &StandaloneExpressionContext,
    sources: &BTreeMap<String, &SourceInput>,
) -> Result<BTreeMap<String, Selector>, String> {
    let default = StandaloneExpressionContext::default();
    if !context.functions.is_empty()
        || !context.native_functions.is_empty()
        || context.context_item.is_some()
        || context.expected_type.is_some()
        || context.type_config != default.type_config
        || context.scope != default.scope
        || context.scope_policy != default.scope_policy
        || !context.diagnostics.is_empty()
        || context.source_uri.is_some()
        || context.resolver_policy_stamp.is_some()
        || context.host_capability_profile.is_some()
        || context.module_resolution.is_some()
        || context.bindings.len() > 64
    {
        return Err(REFUSED.into());
    }
    context
        .bindings
        .iter()
        .map(|(name, binding)| {
            let [item] = binding.value.items.as_slice() else {
                return Err(REFUSED.into());
            };
            let root = crate::eval::retained_cem_node(item).ok_or(REFUSED)?;
            if binding.ty != Type::Any
                || !crate::instance_island::name_allowed(name)
                || !matches!(root.node(), CemAstNode::Element { .. })
            {
                return Err(REFUSED.into());
            }
            let uri = root.owner().source_uri();
            if sources
                .get(uri)
                .is_none_or(|source| !Arc::ptr_eq(&source.tree, root.owner()))
            {
                return Err(REFUSED.into());
            }
            Ok((
                name.clone(),
                Selector {
                    uri: uri.into(),
                    node: root.node_id(),
                },
            ))
        })
        .collect()
}

/// Verify the actual source scope and every original lexical occurrence. The
/// first profile accepts one shared binding set and passive dependency sources.
fn scoped_bindings(
    host: &FreshStructuralHost,
    input: &Selector,
    sources: &BTreeMap<String, &SourceInput>,
    control: &OperationControl,
) -> Result<BTreeMap<String, Selector>, String> {
    let root = sources.get(&input.uri).ok_or(REFUSED)?;
    let from = host.registered_source_scope(&root.tree).ok_or(REFUSED)?;
    let bindings = binding_selectors(
        host.receiver_checkpoint_context(root, 0, false)
            .ok_or(REFUSED)?,
        sources,
    )?;
    let mut elements = vec![input.node];
    while let Some(node) = elements.pop() {
        check(control)?;
        let Some(CemAstNode::Element {
            attributes,
            children,
            ..
        }) = root.tree.ast().get(node)
        else {
            return Err(REFUSED.into());
        };
        host.receiver_checkpoint_context(root, node, false)
            .ok_or(REFUSED)?;
        for &attribute in attributes {
            check(control)?;
            host.receiver_checkpoint_context(root, attribute, false)
                .ok_or(REFUSED)?;
        }
        elements.extend(
            children
                .iter()
                .copied()
                .filter(|&id| matches!(root.tree.ast().get(id), Some(CemAstNode::Element { .. }))),
        );
    }
    for node in root.capture.occurrences() {
        check(control)?;
        let context = host
            .receiver_checkpoint_context(root, node, true)
            .ok_or(REFUSED)?;
        if binding_selectors(context, sources)? != bindings {
            return Err(REFUSED.into());
        }
    }
    for node in &root.tree.ast().nodes {
        check(control)?;
        if let CemAstNode::Reference {
            expression,
            targets,
            ..
        } = node
        {
            if targets.is_some()
                || !expression
                    .trim()
                    .strip_prefix('#')
                    .is_some_and(|name| bindings.contains_key(name))
            {
                return Err(REFUSED.into());
            }
        }
    }
    let used = bindings
        .values()
        .map(|target| target.uri.as_str())
        .chain(std::iter::once(input.uri.as_str()))
        .collect::<BTreeSet<_>>();
    if used.len() != sources.len() {
        return Err(REFUSED.into());
    }
    for (uri, source) in sources {
        check(control)?;
        if uri != &input.uri {
            let to = host.registered_source_scope(&source.tree).ok_or(REFUSED)?;
            let context = host
                .receiver_checkpoint_context(source, 0, false)
                .ok_or(REFUSED)?;
            if source.capture.occurrences().next().is_some()
                || !host.receiver_checkpoint_crossing(from, to)
                || !binding_selectors(context, sources)?.is_empty()
            {
                return Err(REFUSED.into());
            }
        }
    }
    for target in bindings.values() {
        check(control)?;
        host.receiver_checkpoint_context(sources[&target.uri], target.node, false)
            .ok_or(REFUSED)?;
    }
    Ok(bindings)
}

impl ApplicationSourceSession {
    /// Validate the original seal on the exact session before retiring it.
    pub fn confirm_receiver_retirement(
        &self,
        seal: ReceiverCheckpointSeal,
        bytes: &[u8],
        journal: &[u8],
        runtime_id: &str,
        instance_id: &str,
    ) -> Result<ReceiverRecoveryPin, String> {
        let _entry = self.enter()?;
        if !Arc::ptr_eq(&seal.epoch, &self.receiver_checkpoint_epoch)
            || self.receiver_owners.load(Ordering::Acquire) != 1
            || seal.browser_instance.as_deref().is_some_and(|original| original != instance_id)
        {
            return Err(REFUSED.into());
        }
        let pin = seal.confirm_physical_handoff(bytes, journal, runtime_id, instance_id)?;
        self.check()?;
        Ok(pin)
    }

    /// Capture one exact current owner. Available bytes include the application
    /// deployment and its pins; callers subtract other retained resources first.
    pub fn capture_receiver_checkpoint(
        &self,
        input: &ApplicationReceiverInput,
        sources: &[&SourceInput],
        available_bytes: usize,
        authority: &OperationControl,
        may_capture: impl Fn(&SourceInput) -> bool,
    ) -> Result<(Vec<u8>, ReceiverCheckpointSeal), String> {
        let _entry = self.enter()?;
        check(authority)?;
        let control = OperationControl::with_root_policy(
            authority.abort_signal().clone(),
            cem_ml::scheduler::ScopePolicy::host_root().with_timeout_ms(Some(5000)),
        )
        .map_err(|_| REFUSED)?;
        let original = self.prepared.current()?;
        let budget = checkpoint_budget(&self.prepared.sources, available_bytes)?;
        if self.receiver_owners.load(Ordering::Acquire) != 1
            || input.has_typed_producers()
            || input
                .app
                .upgrade()
                .is_none_or(|app| !Arc::ptr_eq(&app, original))
            || checkpoint_artifact(original, &input.tag, input.browser_instance.is_some()).is_err()
            || sources.len() > MAX_SOURCES
            || sources
                .iter()
                .map(|s| s.tree.ast().nodes.len())
                .sum::<usize>()
                > MAX_NODES
            || sources
                .iter()
                .try_fold(0usize, |n, s| n.checked_add(s.identity.byte_length))
                .is_none_or(|n| n > budget)
        {
            return Err(REFUSED.into());
        }
        let revision = self.receiver_checkpoint_epoch.load(Ordering::Acquire);
        let bytes = original.capture_sources(&self.prepared.host, || {
            let mut by_uri = BTreeMap::new();
            for &source in sources {
                check(authority)?;
                check(&control)?;
                if source.tree.source_uri() != source.identity.uri
                    || !Arc::ptr_eq(source.tree.ast_owner(), source.capture.document())
                    || by_uri.insert(source.identity.uri.clone(), source).is_some()
                    || !may_capture(source)
                {
                    return Err(REFUSED.into());
                }
            }
            let uri = input.root.owner().source_uri();
            if by_uri
                .get(uri)
                .is_none_or(|source| !Arc::ptr_eq(&source.tree, input.root.owner()))
            {
                return Err(REFUSED.into());
            }
            let input_root = Selector {
                uri: uri.into(),
                node: input.root.node_id(),
            };
            let bindings = scoped_bindings(&self.prepared.host, &input_root, &by_uri, &control)?;
            let mut packet = Packet {
                deployment: deployment_pin(&self.prepared.sources)?,
                tag: input.tag.clone(),
                input: input_root,
                bindings,
                sources: BTreeMap::new(),
            };
            let mut accounted = MAGIC.len() + transport::encode(&packet)?.len();
            for (uri, source) in by_uri {
                let mut poll = || {
                    check(authority)?;
                    check(&control)
                };
                NativeDependencySourceSnapshot::check_checkpoint_privacy(
                    &source.capture,
                    &source.identity,
                    &mut poll,
                )?;
                let record = Record {
                    identity: source.identity.clone(),
                    snapshot: NativeDependencySourceSnapshot::capture(
                        &source.capture,
                        &source.identity,
                        &mut poll,
                    )?,
                };
                accounted = accounted
                    .checked_add(transport::encode(&record)?.len() + uri.len() + 16)
                    .ok_or(REFUSED)?;
                if accounted > budget {
                    return Err(REFUSED.into());
                }
                packet.sources.insert(uri, record);
            }
            let bytes = [MAGIC, &transport::encode(&packet)?].concat();
            if bytes.len() > budget {
                return Err(REFUSED.into());
            }
            Ok(bytes)
        })?;
        let seal = ReceiverCheckpointSeal {
            digest: ContentHash::from_blake3(&bytes),
            byte_length: bytes.len(),
            declaration: self.prepared.sources.pin().hash.clone(),
            origin: self.endpoint.upgrade().ok_or(REFUSED)?.generation.clone(),
            epoch: self.receiver_checkpoint_epoch.clone(),
            revision,
            control,
            authority: authority.clone(),
            browser_instance: input.browser_instance.clone(),
        };
        self.check()?;
        seal.check()?;
        Ok((bytes, seal))
    }

    /// Enroll only after the trusted bootstrap has registered these exact fresh
    /// source owners and their scope bindings on the replacement publication.
    pub fn restore_receiver_checkpoint(
        &mut self,
        checkpoint: VerifiedReceiverCheckpoint,
        available_bytes: usize,
    ) -> Result<ApplicationReceiverInput, String> {
        let _entry = self.enter()?;
        let VerifiedReceiverCheckpoint { packet, admission } = checkpoint;
        admission.check()?;
        let endpoint = self.endpoint.upgrade().ok_or(REFUSED)?;
        let byte_length = match &admission {
            ReceiverCheckpointAdmission::Local(seal)
                if !Weak::ptr_eq(&seal.origin, &endpoint.generation) =>
            {
                seal.byte_length
            }
            ReceiverCheckpointAdmission::Physical {
                generation,
                byte_length,
                ..
            } if Weak::ptr_eq(generation, &endpoint.generation) => *byte_length,
            _ => return Err(REFUSED.into()),
        };
        let provider = self.prepared.browser_behaviors.get(&packet.tag) == Some(&ApplicationBrowserBehavior::FormControl);
        let browser_instance = match &admission {
            ReceiverCheckpointAdmission::Local(seal) => seal.browser_instance.as_deref(),
            ReceiverCheckpointAdmission::Physical { instance_id, .. } if provider => Some(instance_id.as_str()),
            _ => None,
        };
        if packet.deployment != deployment_pin(&self.prepared.sources)?
            || provider != browser_instance.is_some()
            || checkpoint_artifact(self.prepared.current()?, &packet.tag, provider).is_err()
            || self.receiver_owners.load(Ordering::Acquire) != 0
            || byte_length > checkpoint_budget(&self.prepared.sources, available_bytes)?
        {
            return Err(REFUSED.into());
        }
        drop(endpoint);
        let sources = packet
            .sources
            .iter()
            .map(|(uri, source)| (uri.clone(), source))
            .collect();
        if scoped_bindings(
            &self.prepared.host,
            &packet.input,
            &sources,
            admission.control(),
        )? != packet.bindings
        {
            return Err(REFUSED.into());
        }
        let root = RetainedCemNode::new(
            packet
                .sources
                .get(&packet.input.uri)
                .ok_or(REFUSED)?
                .tree
                .clone(),
            packet.input.node,
        )
        .ok_or(REFUSED)?;
        let input = match browser_instance {
            Some(instance) => self.register_browser_receiver_input(&packet.tag, root, instance)?,
            None => self.register_receiver_input(&packet.tag, root)?,
        };
        admission.check()?;
        Ok(input)
    }
}

fn checkpoint_artifact(app: &LinkedApplication, tag: &str, provider: bool) -> Result<(), String> {
    let artifact = super::frames::receiver_artifact(app, tag, provider.then_some("form-control"))?;
    if artifact.requires_legacy_runtime_resources() {
        return Err(REFUSED.into());
    }
    Ok(())
}
