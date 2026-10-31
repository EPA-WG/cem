//! Original dependency-bearing operation sources. These records cannot become
//! CEMNR1 registrations or CEMOP1 profile inputs through deserialization.
use super::*;
use cem_ml::schema::{
    datatype_registry::DatatypeDependency,
    native_source::dependencies::NativeDependencySourceSnapshot,
};

pub const MAGIC: &[u8] = b"CEMOS1\n";
const ABI: &str = "cem.native-operation-source/1";
const INVALID: &str = "native operation source refused";
const MAX_SITES: usize = 128;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationSourcePin {
    pub hash: String,
    pub source: SourceIdentity,
    pub registry_hash: String,
}
pub struct BuiltOperationSource {
    pub bytes: Vec<u8>,
    pub pin: OperationSourcePin,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capsule {
    abi: String,
    ml_version: String,
    ql_version: String,
    registry_hash: String,
    identity: SourceIdentity,
    implementation: ImplementationIdentity,
    selection: Selection,
    source: NativeDependencySourceSnapshot,
}
pub struct VerifiedOperationSource {
    pin: OperationSourcePin,
    source: Arc<RetainedCemTree>,
    capture: Arc<LexicallyScopedDocument>,
    implementation: ImplementationIdentity,
    selection: Selection,
    requirement: Requirement,
    snapshot_hash: String,
    dependencies: Vec<DatatypeDependency>,
    constant_slots: Vec<SchemaDeclarationNode>,
    references: Vec<SchemaDeclarationNode>,
}
impl VerifiedOperationSource {
    pub fn pin(&self) -> &OperationSourcePin {
        &self.pin
    }
    pub fn source(&self) -> &Arc<RetainedCemTree> {
        &self.source
    }
    pub fn capture(&self) -> &Arc<LexicallyScopedDocument> {
        &self.capture
    }
    pub fn implementation(&self) -> &ImplementationIdentity {
        &self.implementation
    }
    pub fn datatype_source(&self) -> &DatatypeSource {
        // Validator selections are refused before constructing this sealed type.
        datatype_source(&self.requirement).expect("checked operation source")
    }
    pub fn dependencies(&self) -> &[DatatypeDependency] {
        &self.dependencies
    }
    /// Original nontrivia slots, not certified constants or selected targets.
    /// Only the shared enumeration compiler can resolve and admit these nodes.
    pub fn constant_slots(&self) -> &[SchemaDeclarationNode] {
        &self.constant_slots
    }
    /// All original references in the complete source, including other declarations.
    /// Their original contexts are retained; no expression has been evaluated.
    pub fn references(&self) -> &[SchemaDeclarationNode] {
        &self.references
    }
    pub fn selections_verified(&self) -> bool {
        false
    }
    pub fn executable_closure_verified(&self) -> bool {
        false
    }
    pub fn share_source(&self, source: &Self, control: &OperationControl) -> Result<Self, String> {
        let work = Work::new(control);
        work.check()?;
        if self.pin.source != source.pin.source
            || self.pin.registry_hash != source.pin.registry_hash
            || self.snapshot_hash != source.snapshot_hash
        {
            return Err(INVALID.into());
        }
        restored(
            self.pin.clone(),
            source.capture.clone(),
            source.source.clone(),
            self.implementation.clone(),
            self.selection.clone(),
            self.snapshot_hash.clone(),
            &work,
        )
    }
}
fn datatype_source(requirement: &Requirement) -> Option<&DatatypeSource> {
    match requirement {
        Requirement::Tokenizer(source, _)
        | Requirement::Operation(source, _)
        | Requirement::SourceOperation(source, _) => Some(source),
        Requirement::Validator(_) => None,
    }
}
fn restored(
    pin: OperationSourcePin,
    capture: Arc<LexicallyScopedDocument>,
    source: Arc<RetainedCemTree>,
    implementation: ImplementationIdentity,
    selection: Selection,
    snapshot_hash: String,
    work: &Work<'_>,
) -> Result<VerifiedOperationSource, String> {
    work.check()?;
    if matches!(selection, Selection::Validator { .. }) {
        return Err(INVALID.into());
    }
    let mut references = vec![];
    for node in &capture.document().nodes {
        work.check()?;
        match node {
            CemAstNode::Reference { node_id, .. } => {
                if references.len() >= MAX_SITES {
                    return Err(INVALID.into());
                }
                references.push(
                    SchemaDeclarationNode::new(capture.document().clone(), *node_id)
                        .ok_or(INVALID)?,
                );
            }
            CemAstNode::Element { expanded_name, .. } if expanded_name.local_name == "$" => {
                return Err(INVALID.into())
            }
            _ => (),
        }
    }
    // Identical shared checks for modes, signatures, original membership and
    // source-backed result contracts. No query compilation or callback linking.
    let requirement = restore_requirement(&capture, &implementation, selection.clone(), work)?;
    let datatype = datatype_source(&requirement).ok_or(INVALID)?;
    let plan = datatype.plan();
    if !plan.issues.is_empty()
        || plan.dependencies.len() > MAX_SITES
        || plan.constant_slots.len() > MAX_SITES
    {
        return Err(INVALID.into());
    }
    work.check()?;
    Ok(VerifiedOperationSource {
        pin,
        source,
        capture,
        implementation,
        selection,
        requirement,
        snapshot_hash,
        dependencies: plan.dependencies,
        constant_slots: plan.constant_slots,
        references,
    })
}
fn owns(capture: &LexicallyScopedDocument, source: &DatatypeSource) -> bool {
    Arc::ptr_eq(capture.document(), source.scope().document())
        && Arc::ptr_eq(capture.document(), source.declaration().document())
}
pub fn export_operation(
    capture: &LexicallyScopedDocument,
    source: &DatatypeSource,
    mode: OperationMode,
    identity: SourceIdentity,
    implementation: ImplementationIdentity,
    registry_hash: &str,
    control: &OperationControl,
) -> Result<BuiltOperationSource, String> {
    if !owns(capture, source) {
        return Err(INVALID.into());
    }
    build(
        capture,
        identity,
        implementation,
        Selection::Operation {
            scope: source.scope().node_id(),
            declaration: source.declaration().node_id(),
            mode,
        },
        registry_hash,
        control,
    )
}
pub fn export_tokenizer(
    capture: &LexicallyScopedDocument,
    source: &DatatypeSource,
    mode: TokenizerMode,
    identity: SourceIdentity,
    implementation: ImplementationIdentity,
    registry_hash: &str,
    control: &OperationControl,
) -> Result<BuiltOperationSource, String> {
    if !owns(capture, source) {
        return Err(INVALID.into());
    }
    build(
        capture,
        identity,
        implementation,
        Selection::Tokenizer {
            scope: source.scope().node_id(),
            declaration: source.declaration().node_id(),
            mode,
        },
        registry_hash,
        control,
    )
}
pub fn export_source_operation(
    capture: &LexicallyScopedDocument,
    source: &DatatypeSource,
    behavior: &SchemaDeclarationNode,
    profile: SourceProfile,
    identity: SourceIdentity,
    implementation: ImplementationIdentity,
    registry_hash: &str,
    control: &OperationControl,
) -> Result<BuiltOperationSource, String> {
    if !owns(capture, source) || !Arc::ptr_eq(capture.document(), behavior.document()) {
        return Err(INVALID.into());
    }
    build(
        capture,
        identity,
        implementation,
        Selection::SourceOperation {
            scope: source.scope().node_id(),
            declaration: source.declaration().node_id(),
            behavior: behavior.node_id(),
            profile,
        },
        registry_hash,
        control,
    )
}
fn build(
    capture: &LexicallyScopedDocument,
    identity: SourceIdentity,
    implementation: ImplementationIdentity,
    selection: Selection,
    registry_hash: &str,
    control: &OperationControl,
) -> Result<BuiltOperationSource, String> {
    let work = Work::new(control);
    work.check()?;
    implementation.check()?;
    if !valid_hash(registry_hash) {
        return Err(INVALID.into());
    }
    let source = NativeDependencySourceSnapshot::capture(capture, &identity, &mut || work.check())?;
    let c = Capsule {
        abi: ABI.into(),
        ml_version: cem_ml::VERSION.into(),
        ql_version: crate::VERSION.into(),
        registry_hash: registry_hash.into(),
        identity: identity.clone(),
        implementation,
        selection,
        source,
    };
    let bytes = [MAGIC, &encoded(&c)?].concat();
    let pin = OperationSourcePin {
        hash: hash(&bytes),
        source: identity,
        registry_hash: registry_hash.into(),
    };
    verify(&bytes, &pin, control)?;
    Ok(BuiltOperationSource { bytes, pin })
}
pub fn verify(
    bytes: &[u8],
    pin: &OperationSourcePin,
    control: &OperationControl,
) -> Result<VerifiedOperationSource, String> {
    let work = Work::new(control);
    work.check()?;
    if bytes.len() > super::MAX_BYTES
        || !bytes.starts_with(MAGIC)
        || !valid_hash(&pin.hash)
        || !valid_hash(&pin.registry_hash)
        || hash(bytes) != pin.hash
    {
        return Err(INVALID.into());
    }
    let body = &bytes[MAGIC.len()..];
    cem_ml::native_binary::preflight_messagepack(body, 16_384, 64, || work.check())?;
    let mut decoder = rmp_serde::Deserializer::from_read_ref(body);
    decoder.set_max_depth(64);
    let c = Capsule::deserialize(&mut decoder).map_err(|_| INVALID)?;
    if encoded(&c)? != body
        || c.abi != ABI
        || c.ml_version != cem_ml::VERSION
        || c.ql_version != crate::VERSION
        || c.registry_hash != pin.registry_hash
        || c.identity != pin.source
    {
        return Err(INVALID.into());
    }
    c.implementation.check()?;
    let capture = c.source.restore(&pin.source, &mut || work.check())?;
    let source = RetainedCemTree::from_shared(
        capture.document().clone(),
        &pin.source.uri,
        "",
        Default::default(),
        None,
    )
    .map_err(|_| INVALID)?;
    restored(
        pin.clone(),
        capture,
        source,
        c.implementation,
        c.selection,
        hash(&encoded(&c.source)?),
        &work,
    )
}
