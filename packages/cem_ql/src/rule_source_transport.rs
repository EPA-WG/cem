//! Source-backed rule leaves and custom profiles. Existing QD1/NR1 formats stay
//! intact. Native code, host scopes, consumer bindings and recovery are separate.
use crate::{
    native_registrations::{
        self as native, NativeRequirementPin, Requirement, VerifiedNativeRequirement,
    },
    query_validator_capsule::{
        declarations::{self as query, QueryDeclarationPin, VerifiedQueryDeclaration},
        profiles::{transport as profiles, VerifiedQueryResultProfile},
    },
};
use cem_ml::{
    content_cache::ContentHash,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    schema::{
        datatype_validation::ResultRepresentation,
        native_source::dependencies::NativeDependencySourceSnapshot,
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
pub const MAGIC: &[u8] = b"CEMRH1\n";
pub const MAX_BYTES: usize = 4 * 1024 * 1024;
const INVALID: &str = "cem.rule.source.refused";
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum RuleSourcePin {
    Query(QueryDeclarationPin),
    Native(NativeRequirementPin),
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum RuleOrigin {
    Direct,
    Imported { module: String },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleTransportPin {
    pub hash: String,
    pub source: RuleSourcePin,
    pub profile: profiles::ProfileSourcePin,
    pub datatype: Option<u32>,
    pub origin: RuleOrigin,
}
pub struct BuiltRuleTransport {
    pub bytes: Vec<u8>,
    pub pin: RuleTransportPin,
}
pub enum RuleSource {
    Query(Arc<VerifiedQueryDeclaration>),
    Native(Arc<VerifiedNativeRequirement>),
}
#[derive(Clone, Copy)]
pub enum RuleSourceView<'a> {
    Query(&'a VerifiedQueryDeclaration),
    Native(&'a VerifiedNativeRequirement),
}
impl<'a> From<&'a RuleSource> for RuleSourceView<'a> {
    fn from(source: &'a RuleSource) -> Self {
        match source {
            RuleSource::Query(q) => Self::Query(q),
            RuleSource::Native(n) => Self::Native(n),
        }
    }
}
pub struct VerifiedRuleTransport {
    pub(crate) source: RuleSource,
    pub(crate) profile: Arc<VerifiedQueryResultProfile>,
    pub(crate) wire: Arc<[u8]>,
    pin: RuleTransportPin,
}
impl VerifiedRuleTransport {
    pub fn pin(&self) -> &RuleTransportPin {
        &self.pin
    }
    pub fn profile(&self) -> &Arc<VerifiedQueryResultProfile> {
        &self.profile
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    abi: String,
    ml: String,
    ql: String,
    datatype: Option<u32>,
    origin: RuleOrigin,
}
pub(crate) fn encoded(v: &impl Serialize) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec_named(v).map_err(|_| INVALID.into())
}
fn hash(v: &[u8]) -> String {
    ContentHash::from_blake3(v).header_value()
}
fn check(control: &OperationControl) -> Result<(), String> {
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|_| INVALID.into())
}
fn agree(
    source: RuleSourceView<'_>,
    profile: &VerifiedQueryResultProfile,
    control: &OperationControl,
) -> Result<(), String> {
    let (capture, tree, identity, result, id) = match source {
        RuleSourceView::Query(q) => (
            q.capture(),
            q.source(),
            &q.pin().program.source,
            q.contract.signature().result.clone(),
            q.pin().hash.as_str(),
        ),
        RuleSourceView::Native(n) => {
            let Requirement::Validator(contract) = &n.requirement else {
                return Err(INVALID.into());
            };
            (
                n.capture(),
                n.source(),
                &n.pin().source,
                contract.signature().result.clone(),
                n.pin().hash.as_str(),
            )
        }
    };
    if result != ResultRepresentation::Accepted(profile.pin().result.clone()) || id.is_empty() {
        return Err(INVALID.into());
    }
    for p in profile.sources() {
        check(control)?;
        if p.identity.uri == identity.uri {
            let a =
                NativeDependencySourceSnapshot::capture(capture, identity, &mut || check(control))?;
            let b = NativeDependencySourceSnapshot::capture(&p.capture, &p.identity, &mut || {
                check(control)
            })?;
            if &p.identity != identity
                || encoded(&a)? != encoded(&b)?
                || tree.source_uri() != p.tree.source_uri()
            {
                return Err(INVALID.into());
            }
        }
    }
    Ok(())
}
pub fn export<'a>(
    source: impl Into<RuleSourceView<'a>>,
    datatype: Option<u32>,
    origin: RuleOrigin,
    profile: &VerifiedQueryResultProfile,
    control: &OperationControl,
) -> Result<BuiltRuleTransport, String> {
    let source = source.into();
    check(control)?;
    agree(source, profile, control)?;
    let (leaf, source_pin) = match source {
        RuleSourceView::Query(q) => {
            if let Some(id) = datatype {
                query::datatype(q, id)?;
            }
            (q.wire_bytes(), RuleSourcePin::Query(q.pin().clone()))
        }
        RuleSourceView::Native(n) => {
            let Requirement::Validator(contract) = &n.requirement else {
                return Err(INVALID.into());
            };
            if let Some(id) = datatype {
                let selected = native::datatype(n.capture(), contract.owner().node_id(), id)?;
                if selected.plan().kind
                    != cem_ml::schema::datatype_registry::DatatypeKindSource::Explicit(
                        contract.signature().kind,
                    )
                {
                    return Err(INVALID.into());
                }
            }
            (n.wire_bytes(), RuleSourcePin::Native(n.pin().clone()))
        }
    };
    if let RuleOrigin::Imported { module } = &origin {
        let id = match &source_pin {
            RuleSourcePin::Query(p) => &p.hash,
            RuleSourcePin::Native(p) => &p.hash,
        };
        if datatype.is_some() || module != id {
            return Err(INVALID.into());
        }
    }
    let profile = profiles::export(profile, control)?;
    let metadata = encoded(&Metadata {
        abi: "cem.rule-source/1".into(),
        ml: cem_ml::VERSION.into(),
        ql: crate::VERSION.into(),
        datatype,
        origin: origin.clone(),
    })?;
    if metadata.len() > 4096 {
        return Err(INVALID.into());
    }
    let bytes = [
        MAGIC,
        &(metadata.len() as u32).to_le_bytes(),
        &(leaf.len() as u32).to_le_bytes(),
        &metadata,
        leaf,
        &profile.bytes,
    ]
    .concat();
    if bytes.len() > MAX_BYTES {
        return Err(INVALID.into());
    }
    check(control)?;
    Ok(BuiltRuleTransport {
        pin: RuleTransportPin {
            hash: hash(&bytes),
            source: source_pin,
            profile: profile.pin,
            datatype,
            origin,
        },
        bytes,
    })
}
pub fn verify(
    bytes: &[u8],
    pin: &RuleTransportPin,
    control: &OperationControl,
) -> Result<Arc<VerifiedRuleTransport>, String> {
    check(control)?;
    if bytes.len() > MAX_BYTES || !bytes.starts_with(MAGIC) || hash(bytes) != pin.hash {
        return Err(INVALID.into());
    }
    let length = |at| -> Result<usize, String> {
        Ok(u32::from_le_bytes(
            bytes
                .get(at..at + 4)
                .ok_or(INVALID)?
                .try_into()
                .map_err(|_| INVALID)?,
        ) as usize)
    };
    let m = length(MAGIC.len())?;
    let l = length(MAGIC.len() + 4)?;
    let start = MAGIC.len() + 8;
    if m == 0 || m > 4096 || l > 65536 {
        return Err(INVALID.into());
    }
    let meta = bytes.get(start..start + m).ok_or(INVALID)?;
    cem_ml::native_binary::preflight_messagepack(meta, 4096, 32, || check(control))?;
    let metadata: Metadata = rmp_serde::from_slice(meta).map_err(|_| INVALID)?;
    if encoded(&metadata)? != meta
        || metadata.abi != "cem.rule-source/1"
        || metadata.ml != cem_ml::VERSION
        || metadata.ql != crate::VERSION
        || metadata.datatype != pin.datatype
        || metadata.origin != pin.origin
    {
        return Err(INVALID.into());
    }
    let leaf = bytes.get(start + m..start + m + l).ok_or(INVALID)?;
    let source = match &pin.source {
        RuleSourcePin::Query(p) => RuleSource::Query(Arc::new(query::verify(leaf, p, control)?)),
        RuleSourcePin::Native(p) => RuleSource::Native(Arc::new(native::verify(leaf, p, control)?)),
    };
    let profile = profiles::verify(
        bytes.get(start + m + l..).ok_or(INVALID)?,
        &pin.profile,
        control,
    )?;
    let rebuilt = export(&source, pin.datatype, pin.origin.clone(), &profile, control)?;
    if rebuilt.bytes != bytes {
        return Err(INVALID.into());
    }
    check(control)?;
    Ok(Arc::new(VerifiedRuleTransport {
        source,
        profile,
        wire: bytes.into(),
        pin: pin.clone(),
    }))
}
