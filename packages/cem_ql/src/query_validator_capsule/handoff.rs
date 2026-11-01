//! Passive source-only transport. Pins and bytes carry no host, scope, request,
//! native implementation, credential or physical retirement authority.
use super::*;
use cem_ml::{parser::AstNodeId, schema::value_contracts::ContractName};
use declarations::{QueryDeclarationPin, VerifiedQueryDeclaration};

pub const MAGIC: &[u8] = b"CEMQH1\n";
pub const ABI: &str = "cem.native-query-source-handoff/1";
const MAX_METADATA_BYTES: usize = 4096;
pub const MAX_BYTES: usize = MAGIC.len() + 4 + MAX_METADATA_BYTES + super::MAX_BYTES;

/// Trusted build/capture expectation, supplied independently of packet bytes.
/// A pin proves agreement, never permission to install or recover a runtime.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryHandoffPin {
    pub hash: String,
    pub declaration: QueryDeclarationPin,
}
pub struct BuiltQueryHandoff {
    pub bytes: Vec<u8>,
    pub pin: QueryHandoffPin,
}
pub struct VerifiedQueryHandoff {
    declaration: Arc<VerifiedQueryDeclaration>,
    pub(crate) selection: Selection,
}
impl VerifiedQueryHandoff {
    pub fn declaration(&self) -> &Arc<VerifiedQueryDeclaration> {
        &self.declaration
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Selection {
    abi: String,
    ml_version: String,
    ql_version: String,
    pub(crate) datatype: AstNodeId,
    pub(crate) result: ContractName,
    pub(crate) diagnostic: ContractName,
}
fn check_selection(
    selection: &Selection,
    declaration: &VerifiedQueryDeclaration,
) -> Result<(), String> {
    if selection.abi != ABI
        || selection.ml_version != cem_ml::VERSION
        || selection.ql_version != crate::VERSION
        || [&selection.result, &selection.diagnostic]
            .iter()
            .any(|n| n.local.is_empty() || n.local.len() > 1024 || n.namespace.len() > 1024)
        || declaration.contract.signature().result
            != ResultRepresentation::Accepted(selection.result.clone())
    {
        return Err(INVALID.into());
    }
    declarations::datatype(declaration, selection.datatype)?;
    Ok(())
}
pub(crate) fn export(
    declaration: &VerifiedQueryDeclaration,
    datatype: AstNodeId,
    adapter: &crate::datatype_results::DatatypeResultAdapter,
    control: &OperationControl,
) -> Result<BuiltQueryHandoff, String> {
    let work = Work::new(control);
    work.check()?;
    let selection = Selection {
        abi: ABI.into(),
        ml_version: cem_ml::VERSION.into(),
        ql_version: crate::VERSION.into(),
        datatype,
        result: adapter.result_contract().clone(),
        diagnostic: adapter.diagnostic_contract().clone(),
    };
    check_selection(&selection, declaration)?;
    let metadata = encode(&selection)?;
    if metadata.len() > MAX_METADATA_BYTES {
        return Err(INVALID.into());
    }
    // Keep the existing CEMQD1 byte image intact. A separate small canonical
    // metadata frame avoids encoding binary source bytes as record arrays.
    let bytes = [
        MAGIC,
        &(metadata.len() as u32).to_le_bytes(),
        &metadata,
        &declaration.wire,
    ]
    .concat();
    if bytes.len() > MAX_BYTES {
        return Err(INVALID.into());
    }
    work.check()?;
    Ok(BuiltQueryHandoff {
        pin: QueryHandoffPin {
            hash: hash(&bytes),
            declaration: declaration.pin().clone(),
        },
        bytes,
    })
}
/// Restore fresh passive CEM arenas and checked instructions. This never parses
/// query text, registers a host scope, selects code, grants access or executes.
pub fn verify(
    bytes: &[u8],
    expected: &QueryHandoffPin,
    control: &OperationControl,
) -> Result<VerifiedQueryHandoff, String> {
    let work = Work::new(control);
    work.check()?;
    if bytes.len() > MAX_BYTES
        || !bytes.starts_with(MAGIC)
        || !valid_hash(&expected.hash)
        || hash(bytes) != expected.hash
    {
        return Err(INVALID.into());
    }
    let header_end = MAGIC.len() + 4;
    let length = u32::from_le_bytes(
        bytes
            .get(MAGIC.len()..header_end)
            .ok_or(INVALID)?
            .try_into()
            .map_err(|_| INVALID)?,
    ) as usize;
    if length == 0 || length > MAX_METADATA_BYTES {
        return Err(INVALID.into());
    }
    let metadata_end = header_end.checked_add(length).ok_or(INVALID)?;
    let metadata = bytes.get(header_end..metadata_end).ok_or(INVALID)?;
    cem_ml::native_binary::preflight_messagepack(metadata, 128, 8, || work.check())?;
    let mut decoder = rmp_serde::Deserializer::from_read_ref(metadata);
    decoder.set_max_depth(8);
    let selection = Selection::deserialize(&mut decoder).map_err(|_| INVALID)?;
    if encode(&selection)? != metadata {
        return Err(INVALID.into());
    }
    let declaration = Arc::new(declarations::verify_with_work(
        &bytes[metadata_end..],
        &expected.declaration,
        &work,
    )?);
    check_selection(&selection, &declaration)?;
    work.check()?;
    Ok(VerifiedQueryHandoff {
        declaration,
        selection,
    })
}
