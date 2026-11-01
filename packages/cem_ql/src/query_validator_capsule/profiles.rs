//! Original result-schema compilation and inert source agreement. A fingerprint
//! is not an executable/native registration or a linked-code attestation.
use super::*;
use crate::datatype_results::DatatypeResultAdapter;
use cem_ml::{
    parser::tree::RetainedCemTree,
    schema::{
        machine::LexicallyScopedDocument,
        native_source::{dependencies::NativeDependencySourceSnapshot, SourceIdentity},
        value_contracts::{ContractName, ValueContractSource, ValueContracts},
    },
};
const PROFILE_ABI: &str = "cem.query-result-profile/1";
pub const DEPLOYMENT_ABI: &str = "cem-query-execution/1";
pub struct QueryResultProfileSource {
    pub tree: Arc<RetainedCemTree>,
    pub capture: Arc<LexicallyScopedDocument>,
    pub contracts: ValueContractSource,
    pub identity: SourceIdentity,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryResultProfilePin {
    pub hash: String,
    pub result: ContractName,
    pub diagnostic: ContractName,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryExecutionPins {
    pub version: String,
    pub source: super::handoff::QueryHandoffPin,
    pub result_profile: QueryResultProfilePin,
}
impl QueryExecutionPins {
    pub fn check(&self) -> Result<(), String> {
        if self.version != DEPLOYMENT_ABI || !valid_hash(&self.result_profile.hash) {
            return Err(INVALID.into());
        }
        Ok(())
    }
}
pub struct VerifiedQueryResultProfile {
    pin: QueryResultProfilePin,
    sources: Vec<QueryResultProfileSource>,
    pub(crate) adapter: DatatypeResultAdapter,
}
impl VerifiedQueryResultProfile {
    pub fn pin(&self) -> &QueryResultProfilePin {
        &self.pin
    }
    pub fn sources(&self) -> &[QueryResultProfileSource] {
        &self.sources
    }
}
#[derive(Serialize)]
struct SourceRecord<'a> {
    identity: &'a SourceIdentity,
    schema: u32,
    namespace: &'a str,
    bindings: &'a BTreeMap<String, String>,
    snapshot: NativeDependencySourceSnapshot,
}
/// Compile only actual original CEM sources. Capture includes the complete
/// original arena/lexical metadata; no synthetic trimmed schema or AST is made.
pub fn build(
    sources: Vec<QueryResultProfileSource>,
    result: ContractName,
    diagnostic: ContractName,
    control: &OperationControl,
) -> Result<Arc<VerifiedQueryResultProfile>, String> {
    let mut left = 1_000_000usize;
    let mut poll = || {
        left = left.checked_sub(1).ok_or(INVALID)?;
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID.into())
    };
    poll()?;
    if sources.is_empty()
        || sources.len() > 16
        || sources
            .iter()
            .map(|s| s.tree.ast().nodes.len())
            .sum::<usize>()
            > 16384
    {
        return Err(INVALID.into());
    }
    let mut compiled_sources = Vec::new();
    let mut records = Vec::new();
    let mut size = 0usize;
    let mut claims = BTreeMap::new();
    for source in &sources {
        poll()?;
        if !Arc::ptr_eq(source.tree.ast_owner(), source.capture.document())
            || !Arc::ptr_eq(
                source.contracts.schema.document(),
                source.capture.document(),
            )
            || source.tree.source_uri() != source.identity.uri
        {
            return Err(INVALID.into());
        }
        let snapshot =
            NativeDependencySourceSnapshot::capture(&source.capture, &source.identity, &mut poll)?;
        let graph = hash(&encode(&snapshot)?);
        if let Some((identity, previous)) = claims.insert(
            source.identity.uri.clone(),
            (source.identity.clone(), graph.clone()),
        ) {
            if identity != source.identity || previous != graph {
                return Err(INVALID.into());
            }
        }
        let record = SourceRecord {
            identity: &source.identity,
            schema: source.contracts.schema.node_id(),
            namespace: &source.contracts.namespace,
            bindings: &source.contracts.bindings,
            snapshot,
        };
        size = size.checked_add(encode(&record)?.len()).ok_or(INVALID)?;
        if size > 4 * 1024 * 1024 {
            return Err(INVALID.into());
        }
        records.push(record);
        compiled_sources.push(
            source
                .contracts
                .clone()
                .with_captured_names(source.capture.clone())
                .map_err(|_| INVALID)?,
        );
    }
    let contracts =
        ValueContracts::compile(&compiled_sources, Default::default()).map_err(|_| INVALID)?;
    let adapter =
        DatatypeResultAdapter::new(Arc::new(contracts), result.clone(), diagnostic.clone())
            .map_err(|_| INVALID)?;
    let fingerprint = encode(&(
        PROFILE_ABI,
        cem_ml::VERSION,
        crate::VERSION,
        &result,
        &diagnostic,
        records,
    ))?;
    if fingerprint.len() > 4 * 1024 * 1024 {
        return Err(INVALID.into());
    }
    poll()?;
    Ok(Arc::new(VerifiedQueryResultProfile {
        pin: QueryResultProfilePin {
            hash: hash(&fingerprint),
            result,
            diagnostic,
        },
        sources,
        adapter,
    }))
}
