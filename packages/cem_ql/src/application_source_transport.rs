//! Original CEMAPPD4 receiver schemas, never invocation contexts or live grants.
use crate::{
    application_envelope::{
        mixed::ReceiverContract,
        v4::{self, ApplicationEnvelopePin, VerifiedApplicationEnvelope},
    },
    structural_receiver::{
        application::{
            mixed::{MixedReceiverSource, ScalarReceiverSource},
            publication::{LinkedApplication, MixedSource},
            ComponentReceiverSource,
        },
        FreshStructuralHost,
    },
};
use cem_ml::{
    content_cache::ContentHash,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::tree::RetainedCemTree,
    schema::{
        machine::LexicallyScopedDocument,
        native_source::{dependencies::NativeDependencySourceSnapshot, SourceIdentity},
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
pub const MAGIC: &[u8] = b"CEMAH1\n";
pub const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_SOURCES: usize = 32 + 32 * (1 + 64);
const MAX_NODES: usize = 16_384;
const INVALID: &str = "cem.application.source.refused";
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SourceRole {
    Base(String),
    Mixed(String),
    Scalar { tag: String, alias: String },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Selectors {
    Named(BTreeMap<String, u32>),
    Models(Vec<u32>),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceClaim {
    pub role: SourceRole,
    pub uri: String,
    pub schema_uri: String,
    pub selectors: Selectors,
}
pub(crate) struct OriginalClaim {
    claim: SourceClaim,
    tree: Arc<RetainedCemTree>,
    capture: Option<Arc<LexicallyScopedDocument>>,
    identity: Option<SourceIdentity>,
}
pub struct SourceInput {
    /// Trusted build identity for this exact original capture, never a grant.
    pub identity: SourceIdentity,
    pub tree: Arc<RetainedCemTree>,
    pub capture: Arc<LexicallyScopedDocument>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationSourcePin {
    pub hash: String,
    pub application: ApplicationEnvelopePin,
}
pub struct BuiltApplicationSources {
    pub bytes: Vec<u8>,
    pub pin: ApplicationSourcePin,
}
pub struct VerifiedApplicationSources {
    wire: Arc<[u8]>,
    pin: ApplicationSourcePin,
    application: Arc<VerifiedApplicationEnvelope>,
    sources: BTreeMap<String, SourceInput>,
    claims: Vec<SourceClaim>,
}
impl VerifiedApplicationSources {
    pub fn bytes(&self) -> &[u8] {
        &self.wire
    }
    pub fn pin(&self) -> &ApplicationSourcePin {
        &self.pin
    }
    pub fn application(&self) -> &Arc<VerifiedApplicationEnvelope> {
        &self.application
    }
    pub fn source(&self, uri: &str) -> Option<&SourceInput> {
        self.sources.get(uri)
    }
    pub fn claims(&self) -> &[SourceClaim] {
        &self.claims
    }
    pub fn receivers(
        &self,
    ) -> Result<
        (
            BTreeMap<String, ComponentReceiverSource>,
            BTreeMap<String, MixedSource>,
        ),
        String,
    > {
        let mut base = BTreeMap::new();
        let mut mixed = BTreeMap::new();
        for c in &self.claims {
            let source = self.sources.get(&c.uri).ok_or(INVALID)?;
            match (&c.role, &c.selectors) {
                (SourceRole::Base(tag), Selectors::Named(attributes)) => {
                    base.insert(
                        tag.clone(),
                        ComponentReceiverSource {
                            schema_uri: c.schema_uri.clone(),
                            source: source.tree.clone(),
                            attributes: attributes.clone(),
                        },
                    );
                }
                (SourceRole::Mixed(tag), Selectors::Named(attributes)) => {
                    mixed.insert(
                        tag.clone(),
                        MixedSource {
                            receiver: MixedReceiverSource {
                                schema_uri: c.schema_uri.clone(),
                                source: source.tree.clone(),
                                capture: source.capture.clone(),
                                attributes: attributes.clone(),
                            },
                            scalars: BTreeMap::new(),
                        },
                    );
                }
                _ => {}
            }
        }
        for c in &self.claims {
            if let (SourceRole::Scalar { tag, alias }, Selectors::Models(attributes)) =
                (&c.role, &c.selectors)
            {
                let source = self.sources.get(&c.uri).ok_or(INVALID)?;
                mixed.get_mut(tag).ok_or(INVALID)?.scalars.insert(
                    alias.clone(),
                    ScalarReceiverSource {
                        identity: source.identity.clone(),
                        source: source.tree.clone(),
                        attributes: attributes.clone(),
                    },
                );
            }
        }
        Ok((base, mixed))
    }
    pub fn matches_publication(
        &self,
        host: &FreshStructuralHost,
        application: &LinkedApplication,
    ) -> bool {
        application.is_available(host)
            && Arc::ptr_eq(application.envelope(), &self.application)
            && application.source_claims.len() == self.claims.len()
            && application.source_claims.iter().all(|actual| {
                self.claims.contains(&actual.claim)
                    && self.sources.get(&actual.claim.uri).is_some_and(|s| {
                        Arc::ptr_eq(&s.tree, &actual.tree)
                            && actual
                                .capture
                                .as_ref()
                                .is_none_or(|c| Arc::ptr_eq(c, &s.capture))
                            && actual.identity.as_ref().is_none_or(|id| id == &s.identity)
                    })
            })
    }
}
pub(crate) fn original_claims(
    app: &VerifiedApplicationEnvelope,
    base: &BTreeMap<String, ComponentReceiverSource>,
    mixed: &BTreeMap<String, MixedSource>,
) -> Result<Vec<OriginalClaim>, String> {
    if base.len() > 32 || mixed.len() > 32 || mixed.values().any(|s| s.scalars.len() > 64) {
        return Err(INVALID.into());
    }
    let mut out = Vec::new();
    for (tag, s) in base {
        out.push(OriginalClaim {
            claim: SourceClaim {
                role: SourceRole::Base(tag.clone()),
                uri: s.source.source_uri().into(),
                schema_uri: s.schema_uri.clone(),
                selectors: Selectors::Named(s.attributes.clone()),
            },
            tree: s.source.clone(),
            capture: None,
            identity: None,
        });
    }
    for (tag, s) in mixed {
        out.push(OriginalClaim {
            claim: SourceClaim {
                role: SourceRole::Mixed(tag.clone()),
                uri: s.receiver.source.source_uri().into(),
                schema_uri: s.receiver.schema_uri.clone(),
                selectors: Selectors::Named(s.receiver.attributes.clone()),
            },
            tree: s.receiver.source.clone(),
            capture: Some(s.receiver.capture.clone()),
            identity: None,
        });
        for (alias, scalar) in &s.scalars {
            let Some(ReceiverContract::Scalar(contract)) =
                app.mixed(tag).and_then(|m| m.receiver(alias))
            else {
                return Err(INVALID.into());
            };
            out.push(OriginalClaim {
                claim: SourceClaim {
                    role: SourceRole::Scalar {
                        tag: tag.clone(),
                        alias: alias.clone(),
                    },
                    uri: scalar.source.source_uri().into(),
                    schema_uri: contract.schema_uri.clone(),
                    selectors: Selectors::Models(scalar.attributes.clone()),
                },
                tree: scalar.source.clone(),
                capture: None,
                identity: Some(scalar.identity.clone()),
            });
        }
    }
    out.sort_by(|a, b| a.claim.role.cmp(&b.claim.role));
    Ok(out)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    identity: SourceIdentity,
    snapshot: NativeDependencySourceSnapshot,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sources {
    abi: String,
    ml: String,
    ql: String,
    claims: Vec<SourceClaim>,
    sources: BTreeMap<String, Record>,
}
fn hash(bytes: &[u8]) -> String {
    ContentHash::from_blake3(bytes).header_value()
}
pub(crate) fn encode(v: &impl Serialize) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec_named(v).map_err(|_| INVALID.into())
}
fn check(c: &OperationControl) -> Result<(), String> {
    c.check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|_| INVALID.into())
}
fn source_agreement(
    app: &VerifiedApplicationEnvelope,
    inputs: &[&SourceInput],
    control: &OperationControl,
) -> Result<(), String> {
    crate::application_envelope::v3::check_external_sources(
        app.parent(),
        inputs.iter().map(|s| (&*s.capture, &s.identity)),
        control,
    )?;
    let mut hashes = BTreeMap::new();
    let mut remember = |uri: &str, hash: &str| -> Result<(), String> {
        check(control)?;
        if hashes
            .insert(uri.to_owned(), hash.to_owned())
            .is_some_and(|old| old != hash)
        {
            return Err(INVALID.into());
        }
        Ok(())
    };
    for d in app.requirements().declarations.values() {
        remember(&d.source_uri, &d.declaration_source_hash)?;
    }
    for tag in app.mixed_tags() {
        let mixed = app.mixed(tag).ok_or(INVALID)?;
        for occurrence in mixed.occurrences().values() {
            if let Some(ReceiverContract::Scalar(s)) = mixed.receiver(&occurrence.fields["type"]) {
                remember(&s.source_uri, &s.source_hash)?;
            }
        }
    }
    for source in inputs {
        remember(&source.identity.uri, &source.identity.hash)?;
    }
    Ok(())
}
fn claim_shapes(
    app: &VerifiedApplicationEnvelope,
    claims: &[SourceClaim],
    control: &OperationControl,
) -> Result<(), String> {
    let mut expected = BTreeMap::new();
    let base = app.parent().application();
    for item in base.dependency_order() {
        if let crate::application_envelope::DependencyKey::Component(tag) = item {
            let fields =
                crate::structural_receiver::application::receiver_fields(base, tag, control)?;
            if !fields.is_empty() {
                expected.insert(SourceRole::Base(tag.clone()), Some(fields));
            }
        }
    }
    for tag in app.mixed_tags() {
        let mixed = app.mixed(tag).ok_or(INVALID)?;
        expected.insert(
            SourceRole::Mixed(tag.into()),
            Some(mixed.occurrences().keys().cloned().collect()),
        );
        for occurrence in mixed.occurrences().values() {
            let alias = &occurrence.fields["type"];
            if matches!(mixed.receiver(alias), Some(ReceiverContract::Scalar(_))) {
                expected.insert(
                    SourceRole::Scalar {
                        tag: tag.into(),
                        alias: alias.clone(),
                    },
                    None,
                );
            }
        }
    }
    if claims.len() != expected.len() {
        return Err(INVALID.into());
    }
    for claim in claims {
        check(control)?;
        match (expected.remove(&claim.role), &claim.selectors) {
            (Some(Some(fields)), Selectors::Named(actual))
                if fields == actual.keys().cloned().collect() => {}
            (Some(None), Selectors::Models(_)) => {}
            _ => return Err(INVALID.into()),
        }
    }
    Ok(())
}
pub fn capture(
    host: &FreshStructuralHost,
    application: &LinkedApplication,
    inputs: &[SourceInput],
    control: &OperationControl,
) -> Result<BuiltApplicationSources, String> {
    application.capture_sources(host, || {
        check(control)?;
        if inputs.len() > MAX_SOURCES {
            return Err(INVALID.into());
        }
        if application
            .source_claims
            .iter()
            .any(|s| match &s.claim.selectors {
                Selectors::Named(fields) => fields.len() > 64,
                Selectors::Models(models) => models.len() > 64,
            })
        {
            return Err(INVALID.into());
        }
        if inputs
            .iter()
            .map(|s| s.tree.ast().nodes.len())
            .try_fold(0usize, |n, m| n.checked_add(m))
            .is_none_or(|n| n > MAX_NODES)
        {
            return Err(INVALID.into());
        }
        let envelope = application.envelope();
        let data = envelope.wire_bytes();
        let mut metadata = Sources {
            abi: "cem.application-source/1".into(),
            ml: cem_ml::VERSION.into(),
            ql: crate::VERSION.into(),
            claims: application
                .source_claims
                .iter()
                .map(|s| s.claim.clone())
                .collect(),
            sources: BTreeMap::new(),
        };
        let mut accounted = MAGIC.len() + 4 + data.len() + encode(&metadata)?.len();
        if accounted > MAX_BYTES {
            return Err(INVALID.into());
        }
        let mut records = BTreeMap::<String, Record>::new();
        let mut used = BTreeSet::new();
        for actual in &application.source_claims {
            check(control)?;
            let (index, source) = inputs
                .iter()
                .enumerate()
                .find(|(_, s)| Arc::ptr_eq(&s.tree, &actual.tree))
                .ok_or(INVALID)?;
            used.insert(index);
            if !Arc::ptr_eq(source.tree.ast_owner(), source.capture.document())
                || source.identity.uri != actual.claim.uri
                || actual
                    .capture
                    .as_ref()
                    .is_some_and(|c| !Arc::ptr_eq(c, &source.capture))
                || actual
                    .identity
                    .as_ref()
                    .is_some_and(|id| id != &source.identity)
            {
                return Err(INVALID.into());
            }
            let snapshot = NativeDependencySourceSnapshot::capture(
                &source.capture,
                &source.identity,
                &mut || check(control),
            )?;
            let record = Record {
                identity: source.identity.clone(),
                snapshot,
            };
            if let Some(old) = records.get(&source.identity.uri) {
                if encode(old)? != encode(&record)? {
                    return Err(INVALID.into());
                }
            } else {
                // Bound cumulative source copying before keeping another record.
                // Sixteen bytes cover map/string framing beyond this URI.
                accounted = accounted
                    .checked_add(encode(&record)?.len())
                    .and_then(|n| n.checked_add(source.identity.uri.len()))
                    .and_then(|n| n.checked_add(16))
                    .ok_or(INVALID)?;
                if accounted > MAX_BYTES {
                    return Err(INVALID.into());
                }
            }
            records.insert(source.identity.uri.clone(), record);
        }
        if used.len() != inputs.len() {
            return Err(INVALID.into());
        }
        source_agreement(
            application.envelope(),
            &inputs.iter().collect::<Vec<_>>(),
            control,
        )?;
        metadata.sources = records;
        let metadata = encode(&metadata)?;
        let bytes = [MAGIC, &(data.len() as u32).to_le_bytes(), data, &metadata].concat();
        if bytes.len() > MAX_BYTES {
            return Err(INVALID.into());
        }
        check(control)?;
        Ok(BuiltApplicationSources {
            pin: ApplicationSourcePin {
                hash: hash(&bytes),
                application: envelope.pin().clone(),
            },
            bytes,
        })
    })
}
pub fn verify(
    bytes: &[u8],
    pin: &ApplicationSourcePin,
    control: &OperationControl,
) -> Result<Arc<VerifiedApplicationSources>, String> {
    check(control)?;
    if bytes.len() > MAX_BYTES || !bytes.starts_with(MAGIC) || hash(bytes) != pin.hash {
        return Err(INVALID.into());
    }
    let at = MAGIC.len();
    let n = u32::from_le_bytes(
        bytes
            .get(at..at + 4)
            .ok_or(INVALID)?
            .try_into()
            .map_err(|_| INVALID)?,
    ) as usize;
    let start = at + 4;
    let end = start
        .checked_add(n)
        .filter(|n| *n <= bytes.len())
        .ok_or(INVALID)?;
    let application = Arc::new(v4::verify(&bytes[start..end], &pin.application, control)?);
    let body = &bytes[end..];
    let mut work = 1_000_000usize;
    let mut poll = || {
        work = work.checked_sub(1).ok_or(INVALID)?;
        check(control)
    };
    cem_ml::native_binary::preflight_messagepack(body, 1_000_000, 64, &mut poll)?;
    let mut decoder = rmp_serde::Deserializer::from_read_ref(body);
    decoder.set_max_depth(64);
    let s = Sources::deserialize(&mut decoder).map_err(|_| INVALID)?;
    if encode(&s)? != body
        || s.abi != "cem.application-source/1"
        || s.ml != cem_ml::VERSION
        || s.ql != crate::VERSION
        || s.claims.len() > MAX_SOURCES
        || s.sources.len() > MAX_SOURCES
        || s.sources
            .values()
            .map(|s| s.snapshot.node_count())
            .try_fold(0usize, |n, m| n.checked_add(m))
            .is_none_or(|n| n > MAX_NODES)
    {
        return Err(INVALID.into());
    }
    if s.claims.windows(2).any(|c| c[0].role >= c[1].role)
        || s.claims
            .iter()
            .map(|c| c.uri.clone())
            .collect::<BTreeSet<_>>()
            != s.sources.keys().cloned().collect()
    {
        return Err(INVALID.into());
    }
    claim_shapes(&application, &s.claims, control)?;
    let mut sources = BTreeMap::new();
    for (uri, record) in s.sources {
        poll()?;
        if uri != record.identity.uri {
            return Err(INVALID.into());
        }
        let capture = record.snapshot.restore(&record.identity, &mut poll)?;
        let tree = RetainedCemTree::from_shared(
            capture.document().clone(),
            &uri,
            "",
            Default::default(),
            None,
        )
        .map_err(|_| INVALID)?;
        sources.insert(
            uri,
            SourceInput {
                identity: record.identity,
                tree,
                capture,
            },
        );
    }
    for c in &s.claims {
        poll()?;
        let source = sources.get(&c.uri).ok_or(INVALID)?;
        let ids = match &c.selectors {
            Selectors::Named(m) => m.values().copied().collect::<Vec<_>>(),
            Selectors::Models(m) => m.clone(),
        };
        if ids.len() > 64 || ids.iter().any(|id| source.tree.ast().get(*id).is_none()) {
            return Err(INVALID.into());
        }
        match (&c.role,&c.selectors) {
            (SourceRole::Base(tag),Selectors::Named(_)) if application.parent().application().declaration(tag).is_some()=>{},
            (SourceRole::Mixed(tag),Selectors::Named(_)) if application.mixed(tag).is_some()=>{},
            (SourceRole::Scalar{tag,alias},Selectors::Models(_)) if application.mixed(tag).and_then(|m|m.receiver(alias)).is_some_and(|r|matches!(r,ReceiverContract::Scalar(value) if value.schema_uri==c.schema_uri&&value.source_uri==c.uri&&value.source_hash==source.identity.hash))=>{},
            _=>return Err(INVALID.into())
        }
    }
    source_agreement(&application, &sources.values().collect::<Vec<_>>(), control)?;
    check(control)?;
    Ok(Arc::new(VerifiedApplicationSources {
        wire: bytes.into(),
        pin: pin.clone(),
        application,
        sources,
        claims: s.claims,
    }))
}
