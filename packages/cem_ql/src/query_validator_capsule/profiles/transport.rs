//! Complete passive profile sources. No parser, scope, callback or authority is
//! reconstructed from metadata; fresh native compilation checks the profile pin.
use super::*;
use cem_ml::schema::declaration_references::SchemaDeclarationNode;
pub const MAGIC: &[u8] = b"CEMRP1\n";
pub const MAX_BYTES: usize = 4 * 1024 * 1024;
const ABI: &str = "cem.query-result-profile-sources/1";
struct Work<'a> {
    control: &'a OperationControl,
    left: std::cell::Cell<usize>,
}
impl<'a> Work<'a> {
    fn new(control: &'a OperationControl) -> Self {
        Self {
            control,
            left: std::cell::Cell::new(1_000_000),
        }
    }
    fn check(&self) -> Result<(), String> {
        self.left
            .set(self.left.get().checked_sub(1).ok_or(INVALID)?);
        self.control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID.into())
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileSourcePin {
    pub hash: String,
    pub profile: QueryResultProfilePin,
}
pub struct BuiltProfileSources {
    pub bytes: Vec<u8>,
    pub pin: ProfileSourcePin,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    identity: SourceIdentity,
    schema: u32,
    namespace: String,
    bindings: BTreeMap<String, String>,
    snapshot: NativeDependencySourceSnapshot,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capsule {
    abi: String,
    ml_version: String,
    ql_version: String,
    profile: QueryResultProfilePin,
    sources: Vec<Record>,
}
pub fn export(
    profile: &VerifiedQueryResultProfile,
    control: &OperationControl,
) -> Result<BuiltProfileSources, String> {
    let work = Work::new(control);
    work.check()?;
    let records = profile
        .sources()
        .iter()
        .map(|s| {
            Ok(Record {
                identity: s.identity.clone(),
                schema: s.contracts.schema.node_id(),
                namespace: s.contracts.namespace.clone(),
                bindings: s.contracts.bindings.clone(),
                snapshot: NativeDependencySourceSnapshot::capture(
                    &s.capture,
                    &s.identity,
                    &mut || work.check(),
                )?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let body = encode(&Capsule {
        abi: ABI.into(),
        ml_version: cem_ml::VERSION.into(),
        ql_version: crate::VERSION.into(),
        profile: profile.pin().clone(),
        sources: records,
    })?;
    if body.len() + MAGIC.len() > MAX_BYTES {
        return Err(INVALID.into());
    }
    let bytes = [MAGIC, &body].concat();
    work.check()?;
    Ok(BuiltProfileSources {
        pin: ProfileSourcePin {
            hash: hash(&bytes),
            profile: profile.pin().clone(),
        },
        bytes,
    })
}
pub fn verify(
    bytes: &[u8],
    expected: &ProfileSourcePin,
    control: &OperationControl,
) -> Result<Arc<VerifiedQueryResultProfile>, String> {
    let work = Work::new(control);
    work.check()?;
    if bytes.len() > MAX_BYTES
        || !bytes.starts_with(MAGIC)
        || !valid_hash(&expected.hash)
        || hash(bytes) != expected.hash
    {
        return Err(INVALID.into());
    }
    let body = &bytes[MAGIC.len()..];
    cem_ml::native_binary::preflight_messagepack(body, 1_000_000, 64, || work.check())?;
    let mut decoder = rmp_serde::Deserializer::from_read_ref(body);
    decoder.set_max_depth(64);
    let c = Capsule::deserialize(&mut decoder).map_err(|_| INVALID)?;
    if encode(&c)? != body
        || c.abi != ABI
        || c.ml_version != cem_ml::VERSION
        || c.ql_version != crate::VERSION
        || c.profile != expected.profile
        || c.sources.is_empty()
        || c.sources.len() > 16
        || c.sources
            .iter()
            .map(|s| s.snapshot.node_count())
            .try_fold(0usize, |n, m| n.checked_add(m))
            .is_none_or(|n| n > 16384)
    {
        return Err(INVALID.into());
    }
    let mut shared = BTreeMap::<
        String,
        (
            SourceIdentity,
            String,
            Arc<LexicallyScopedDocument>,
            Arc<RetainedCemTree>,
        ),
    >::new();
    let mut sources = Vec::new();
    for r in c.sources {
        work.check()?;
        let snapshot_hash = hash(&encode(&r.snapshot)?);
        let (capture, tree) = if let Some((id, graph, capture, tree)) = shared.get(&r.identity.uri)
        {
            if id != &r.identity || graph != &snapshot_hash {
                return Err(INVALID.into());
            }
            (capture.clone(), tree.clone())
        } else {
            let capture = r.snapshot.restore(&r.identity, &mut || work.check())?;
            let tree = RetainedCemTree::from_shared(
                capture.document().clone(),
                &r.identity.uri,
                "",
                Default::default(),
                None,
            )
            .map_err(|_| INVALID)?;
            shared.insert(
                r.identity.uri.clone(),
                (
                    r.identity.clone(),
                    snapshot_hash,
                    capture.clone(),
                    tree.clone(),
                ),
            );
            (capture, tree)
        };
        let schema =
            SchemaDeclarationNode::new(capture.document().clone(), r.schema).ok_or(INVALID)?;
        sources.push(QueryResultProfileSource {
            tree,
            capture,
            identity: r.identity,
            contracts: ValueContractSource::new(schema, r.namespace, r.bindings),
        });
    }
    let profile = build(sources, c.profile.result, c.profile.diagnostic, control)?;
    if profile.pin() != &expected.profile {
        return Err(INVALID.into());
    }
    work.check()?;
    Ok(profile)
}
