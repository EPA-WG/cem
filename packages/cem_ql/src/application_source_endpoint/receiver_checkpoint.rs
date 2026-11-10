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
    origin: Weak<NativeGeneration>,
    epoch: Arc<AtomicUsize>,
    revision: usize,
    control: OperationControl,
    authority: OperationControl,
}

pub struct VerifiedReceiverCheckpoint {
    packet: PacketOwners,
    seal: ReceiverCheckpointSeal,
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
        let body = bytes.strip_prefix(MAGIC).ok_or(REFUSED)?;
        let mut poll = || self.check();
        cem_ml::native_binary::preflight_messagepack(body, 1_000_000, 64, &mut poll)?;
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
        let sources = packet
            .sources
            .into_iter()
            .map(|(uri, record)| {
                let source = record.restore(&mut poll)?;
                NativeDependencySourceSnapshot::check_checkpoint_privacy(
                    &source.capture,
                    &source.identity,
                    &mut poll,
                )?;
                Ok((uri, source))
            })
            .collect::<Result<_, String>>()?;
        let packet = PacketOwners {
            deployment: packet.deployment,
            tag: packet.tag,
            input: packet.input,
            bindings: packet.bindings,
            sources,
        };
        self.check()?;
        Ok(VerifiedReceiverCheckpoint { packet, seal: self })
    }
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
        self.seal.check()?;
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
    let Some(CemAstNode::Element { attributes, .. }) = root.tree.ast().get(input.node) else {
        return Err(REFUSED.into());
    };
    for node in std::iter::once(input.node).chain(attributes.iter().copied()) {
        check(control)?;
        host.receiver_checkpoint_context(root, node, false)
            .ok_or(REFUSED)?;
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
        if self.receiver_inputs.load(Ordering::Acquire) != 1
            || input
                .app
                .upgrade()
                .is_none_or(|app| !Arc::ptr_eq(&app, original))
            || checkpoint_artifact(original, &input.tag).is_err()
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
            origin: self.endpoint.upgrade().ok_or(REFUSED)?.generation.clone(),
            epoch: self.receiver_checkpoint_epoch.clone(),
            revision,
            control,
            authority: authority.clone(),
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
        let VerifiedReceiverCheckpoint { packet, seal } = checkpoint;
        seal.check()?;
        let endpoint = self.endpoint.upgrade().ok_or(REFUSED)?;
        if Weak::ptr_eq(&seal.origin, &endpoint.generation)
            || packet.deployment != deployment_pin(&self.prepared.sources)?
            || checkpoint_artifact(self.prepared.current()?, &packet.tag).is_err()
            || self.receiver_inputs.load(Ordering::Acquire) != 0
            || seal.byte_length > checkpoint_budget(&self.prepared.sources, available_bytes)?
        {
            return Err(REFUSED.into());
        }
        drop(endpoint);
        let sources = packet
            .sources
            .iter()
            .map(|(uri, source)| (uri.clone(), source))
            .collect();
        if scoped_bindings(&self.prepared.host, &packet.input, &sources, &seal.control)?
            != packet.bindings
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
        let input = self.register_receiver_input(&packet.tag, root)?;
        seal.check()?;
        Ok(input)
    }
}

fn checkpoint_artifact(app: &LinkedApplication, tag: &str) -> Result<(), String> {
    let artifact = super::frames::receiver_artifact(app, tag)?;
    if artifact.requires_legacy_runtime_resources() {
        return Err(REFUSED.into());
    }
    Ok(())
}
