//! Shared profile checks; exact leaf types keep old/new admission separate.
use super::*;
use crate::native_registrations::dependency_sources::{
    BuiltOperationSource, OperationSourcePin, VerifiedOperationSource,
};
use serde::de::DeserializeOwned;
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
pub(super) struct Format {
    pub magic: &'static [u8],
    pub abi: &'static str,
    pub independent_sources: bool,
}
pub(super) trait DependencyPin: Clone + Serialize + DeserializeOwned {
    fn hash(&self) -> &str;
    fn source(&self) -> &SourceIdentity;
    fn registry(&self) -> &str;
}
macro_rules! pin {
    ($ty:ty) => {
        impl DependencyPin for $ty {
            fn hash(&self) -> &str {
                &self.hash
            }
            fn source(&self) -> &SourceIdentity {
                &self.source
            }
            fn registry(&self) -> &str {
                &self.registry_hash
            }
        }
    };
}
pin!(NativeRequirementPin);
pin!(OperationSourcePin);
pub(super) trait Leaf: Sized {
    type Pin: DependencyPin;
    fn pin(&self) -> &Self::Pin;
    fn datatype_source(&self) -> Option<&DatatypeSource>;
    fn implementation(&self) -> &ImplementationIdentity;
    fn capture(&self) -> &LexicallyScopedDocument;
    fn share(&self, root: &Self, control: &OperationControl) -> Result<Self, String>;
}
impl Leaf for VerifiedNativeRequirement {
    type Pin = NativeRequirementPin;
    fn pin(&self) -> &Self::Pin {
        self.pin()
    }
    fn datatype_source(&self) -> Option<&DatatypeSource> {
        self.datatype_source()
    }
    fn implementation(&self) -> &ImplementationIdentity {
        self.implementation()
    }
    fn capture(&self) -> &LexicallyScopedDocument {
        self.capture()
    }
    fn share(&self, root: &Self, control: &OperationControl) -> Result<Self, String> {
        self.share_source(root, control)
    }
}
impl Leaf for VerifiedOperationSource {
    type Pin = OperationSourcePin;
    fn pin(&self) -> &Self::Pin {
        self.pin()
    }
    fn datatype_source(&self) -> Option<&DatatypeSource> {
        Some(self.datatype_source())
    }
    fn implementation(&self) -> &ImplementationIdentity {
        self.implementation()
    }
    fn capture(&self) -> &LexicallyScopedDocument {
        self.capture()
    }
    fn share(&self, root: &Self, control: &OperationControl) -> Result<Self, String> {
        self.share_source(root, control)
    }
}
pub(super) trait Artifact {
    type Leaf: Leaf;
    fn bytes(&self) -> &[u8];
    fn pin(&self) -> &<Self::Leaf as Leaf>::Pin;
    fn verify(&self, control: &OperationControl) -> Result<Self::Leaf, String>;
}
impl Artifact for BuiltNativeRequirement {
    type Leaf = VerifiedNativeRequirement;
    fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    fn pin(&self) -> &NativeRequirementPin {
        &self.pin
    }
    fn verify(&self, control: &OperationControl) -> Result<Self::Leaf, String> {
        super::super::verify(&self.bytes, &self.pin, control)
    }
}
impl Artifact for BuiltOperationSource {
    type Leaf = VerifiedOperationSource;
    fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    fn pin(&self) -> &OperationSourcePin {
        &self.pin
    }
    fn verify(&self, control: &OperationControl) -> Result<Self::Leaf, String> {
        super::super::dependency_sources::verify(&self.bytes, &self.pin, control)
    }
}
type PinOf<A> = <<A as Artifact>::Leaf as Leaf>::Pin;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capsule<P> {
    abi: String,
    ml_version: String,
    ql_version: String,
    registry_hash: String,
    profiles: BTreeMap<String, OperationProfileRecord<P>>,
}
fn compatible<P>(
    p: &OperationProfileRecord<P>,
    role: OperationRole,
    source: &DatatypeSource,
    implementation: &ImplementationIdentity,
    identity: &SourceIdentity,
) -> bool {
    use NativeSignature as S;
    use OperationRole as R;
    if *identity != p.source
        || source.scope().node_id() != p.scope
        || source.declaration().node_id() != p.declaration
        || !source.plan().issues.is_empty()
        || !match source.plan().kind {
            DatatypeKindSource::Explicit(kind) => kind == p.kind,
            DatatypeKindSource::Inherited => true,
            _ => false,
        }
    {
        return false;
    }
    let shape = match p.kind {
        DatatypeKind::List => matches!(p.representation, ValueRepresentation::List(_)),
        DatatypeKind::Node => p.representation == ValueRepresentation::Nodes,
        _ => matches!(p.representation, ValueRepresentation::Scalar(_)),
    };
    shape
        && match (role, &implementation.signature) {
            (R::Tokenizer, S::TokenizerSpans) => p.kind == DatatypeKind::List,
            (R::Preparation, S::Preparation(s)) => p.kind == s.kind && p.representation == s.output,
            (R::Conversion, S::Conversion(s) | S::SourceConversion(s)) => {
                p.kind == s.kind && p.representation == s.output
            }
            (R::Equality, S::Equality(s) | S::SourceEquality(s))
            | (R::Constant, S::Constant(s) | S::SourceConstant(s)) => {
                p.representation == ValueRepresentation::Scalar(*s)
            }
            (R::ListSerializer, S::ListSerializer(s)) => {
                p.representation == ValueRepresentation::List(s.item)
            }
            (R::ListItems, S::ListItems(s)) => p.representation == ValueRepresentation::List(*s),
            (R::FacetProfile, S::FacetProfile(f)) => p.representation == f.representation(),
            _ => false,
        }
}
pub(super) fn export<A: Artifact>(
    profiles: &BTreeMap<String, OperationProfileRecord<PinOf<A>>>,
    artifacts: &[A],
    registry_hash: &str,
    control: &OperationControl,
    format: Format,
) -> Result<Vec<u8>, String> {
    let work = Work::new(control);
    work.check()?;
    // Bound caller-owned collections before cloning or serialization.
    if profiles.is_empty()
        || profiles.len() > MAX_PROFILES
        || artifacts.is_empty()
        || artifacts.len() > MAX_ARTIFACTS
        || !valid_hash(registry_hash)
        || profiles.iter().any(|(id, p)| {
            id.len() > 128
                || p.source.uri.len() > 4096
                || !valid_hash(&p.source.hash)
                || p.dependencies.len() > 8
                || p.dependencies.values().any(|dep| {
                    dep.source().uri.len() > 4096
                        || !valid_hash(&dep.source().hash)
                        || !valid_hash(dep.hash())
                        || !valid_hash(dep.registry())
                })
        })
    {
        return Err(INVALID.into());
    }
    let capsule = Capsule {
        abi: format.abi.into(),
        ml_version: cem_ml::VERSION.into(),
        ql_version: crate::VERSION.into(),
        registry_hash: registry_hash.into(),
        profiles: profiles.clone(),
    };
    let bytes = [format.magic, &encode(&capsule)?].concat();
    verify(
        &bytes,
        &hash(&bytes),
        registry_hash,
        artifacts,
        control,
        format,
    )?;
    Ok(bytes)
}
pub(super) fn verify<A: Artifact>(
    bytes: &[u8],
    pin_hash: &str,
    registry_hash: &str,
    artifacts: &[A],
    control: &OperationControl,
    format: Format,
) -> Result<BTreeMap<String, VerifiedOperationProfileView<A::Leaf>>, String> {
    let work = Work::new(control);
    work.check()?;
    if bytes.len() > MAX_BYTES
        || !bytes.starts_with(format.magic)
        || !valid_hash(pin_hash)
        || !valid_hash(registry_hash)
        || hash(bytes) != pin_hash
        || artifacts.is_empty()
        || artifacts.len() > MAX_ARTIFACTS
    {
        return Err(INVALID.into());
    }
    let total = artifacts.iter().try_fold(bytes.len(), |sum, a| {
        work.check()?;
        // Both exact leaf formats retain their 64 KiB bound before decoding.
        if a.bytes().len() > super::MAX_BYTES {
            return Err(INVALID.to_owned());
        }
        sum.checked_add(a.bytes().len())
            .filter(|n| *n <= MAX_TOTAL_BYTES)
            .ok_or(INVALID.to_owned())
    })?;
    if total > MAX_TOTAL_BYTES {
        return Err(INVALID.into());
    }
    let body = &bytes[format.magic.len()..];
    cem_ml::native_binary::preflight_messagepack(body, 100_000, 64, || work.check())?;
    let mut decoder = rmp_serde::Deserializer::from_read_ref(body);
    decoder.set_max_depth(64);
    let c = Capsule::<PinOf<A>>::deserialize(&mut decoder).map_err(|_| INVALID)?;
    if encode(&c)? != body
        || c.abi != format.abi
        || c.ml_version != cem_ml::VERSION
        || c.ql_version != crate::VERSION
        || c.registry_hash != registry_hash
        || c.profiles.is_empty()
        || c.profiles.len() > MAX_PROFILES
    {
        return Err(INVALID.into());
    }
    let mut required = BTreeMap::new();
    let mut subjects = BTreeSet::new();
    for (id, p) in &c.profiles {
        work.check()?;
        if id.is_empty()
            || id.len() > 128
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
            || p.dependencies.is_empty()
            || p.dependencies.len() > 8
            || !subjects.insert((p.source.uri.clone(), p.scope, p.declaration))
        {
            return Err(INVALID.into());
        }
        for dep in p.dependencies.values() {
            work.check()?;
            if dep.registry() != c.registry_hash
                || *dep.source() != p.source
                || !valid_hash(dep.hash())
                || required.insert(dep.hash().to_owned(), dep).is_some()
            {
                return Err(INVALID.into());
            }
        }
    }
    if required.len() != artifacts.len() {
        return Err(INVALID.into());
    }
    let mut leaves = BTreeMap::new();
    let mut sources: BTreeMap<String, Arc<A::Leaf>> = BTreeMap::new();
    let mut agreement = crate::datatype_dependency_capsule::SourceAgreement::new();
    for a in artifacts {
        work.check()?;
        let expected = required.get(a.pin().hash()).ok_or(INVALID)?;
        // Compare the complete pin, never just its hash/URI.
        if expected.hash() != a.pin().hash()
            || *expected.source() != *a.pin().source()
            || expected.registry() != a.pin().registry()
            || leaves.contains_key(a.pin().hash())
        {
            return Err(INVALID.into());
        }
        let checked = a.verify(control)?;
        if format.independent_sources {
            crate::datatype_dependency_capsule::remember_source(
                &mut agreement,
                checked.capture(),
                checked.pin().source(),
                &work,
            )?;
        }
        // Explicit metadata occurrence mapping after complete identity + snapshot
        // agreement. Each independently verified profile set keeps fresh owners.
        let checked = if let Some(root) = sources.get(&a.pin().source().uri) {
            checked.share(root, control)?
        } else {
            checked
        };
        let checked = Arc::new(checked);
        sources
            .entry(a.pin().source().uri.clone())
            .or_insert_with(|| checked.clone());
        leaves.insert(a.pin().hash().to_owned(), checked);
        work.check()?;
    }
    let mut profiles = BTreeMap::new();
    for (id, p) in c.profiles {
        work.check()?;
        let mut requirements = BTreeMap::new();
        for (role, dep) in &p.dependencies {
            work.check()?;
            let r = leaves.get(dep.hash()).ok_or(INVALID)?;
            let source = r.datatype_source().ok_or(INVALID)?;
            if !compatible(&p, *role, source, r.implementation(), r.pin().source()) {
                return Err(INVALID.into());
            }
            requirements.insert(*role, r.clone());
        }
        let source = requirements
            .values()
            .next()
            .and_then(|r| r.datatype_source())
            .ok_or(INVALID)?
            .clone();
        profiles.insert(
            id,
            VerifiedOperationProfileView {
                source,
                kind: p.kind,
                representation: p.representation,
                requirements,
            },
        );
    }
    work.check()?;
    Ok(profiles)
}
