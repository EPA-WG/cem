//! Inert transport of acknowledged state, never a runtime image or execution
//! authority. The host supplies its independently held publication receipt.
//! Island text must pass the normal lifecycle/domain admission when resumed.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MAX_CHECKPOINT_BYTES: usize = 1024 * 1024;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerReceipt {
    pub instance_id: String,
    pub declaration_identity: String,
    pub data_revision: String,
    pub accepted_event_sequence: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommitReceipt {
    pub runtime_id: String,
    pub generation: u64,
    pub commit_sequence: u64,
    pub policy_stamp: String,
    pub owners: Vec<OwnerReceipt>,
    /// Outstanding external actions must be recorded as unknown, not retried.
    pub operations: Vec<OperationReceipt>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationReceipt {
    pub instance_id: String,
    pub operation_id: String,
    pub outcome: OperationOutcome,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationOutcome {
    Acknowledged,
    Unknown,
    NotApplied,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerCheckpoint {
    pub instance_id: String,
    /// Policy-filtered DOM-native data island, including portable semantic
    /// state. Not rendered output, JS AST, pointers, or live native handles.
    pub island: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Checkpoint {
    pub version: String,
    pub receipt: CommitReceipt,
    pub owners: Vec<OwnerCheckpoint>,
}
fn identity(value: &str) -> bool {
    !value.is_empty() && value.len() <= 1024 && !value.chars().any(char::is_control)
}
fn valid_receipt(receipt: &CommitReceipt) -> bool {
    if !identity(&receipt.runtime_id)
        || !identity(&receipt.policy_stamp)
        || receipt.generation == 0
        || receipt.generation > MAX_SAFE_INTEGER
        || receipt.commit_sequence == 0
        || receipt.commit_sequence > MAX_SAFE_INTEGER
        || receipt.owners.is_empty()
        || receipt.owners.len() > 256
        || receipt.operations.len() > 256
    {
        return false;
    }
    let mut owners = BTreeSet::new();
    for owner in &receipt.owners {
        if !identity(&owner.instance_id)
            || !identity(&owner.declaration_identity)
            || !identity(&owner.data_revision)
            || owner.accepted_event_sequence > MAX_SAFE_INTEGER
            || !owners.insert(owner.instance_id.as_str())
        {
            return false;
        }
    }
    let mut operations = BTreeSet::new();
    receipt.operations.iter().all(|operation| {
        owners.contains(operation.instance_id.as_str())
            && identity(&operation.operation_id)
            && operations.insert((&operation.instance_id, &operation.operation_id))
    })
}
/// Both JSON inputs are bounded before parsing. This seal verifies the
/// transport and exact receipt, not the semantic safety of island content.
/// Cancellation/commit freshness must also be checked by the host after return.
pub fn seal(candidate_json: &str, receipt_json: &str) -> Result<String, &'static str> {
    if candidate_json.len() > MAX_CHECKPOINT_BYTES || receipt_json.len() > MAX_CHECKPOINT_BYTES {
        return Err("cem.checkpoint.limit");
    }
    let candidate: Checkpoint =
        serde_json::from_str(candidate_json).map_err(|_| "cem.checkpoint.format")?;
    let expected: CommitReceipt =
        serde_json::from_str(receipt_json).map_err(|_| "cem.checkpoint.receipt")?;
    if candidate.version != "cem-committed-checkpoint/1"
        || !valid_receipt(&expected)
        || candidate.receipt != expected
        || candidate.owners.len() != expected.owners.len()
    {
        return Err("cem.checkpoint.receipt");
    }
    let mut owners = BTreeSet::new();
    for owner in &candidate.owners {
        if owner.island.is_empty()
            || !owners.insert(owner.instance_id.as_str())
            || !expected
                .owners
                .iter()
                .any(|receipt| receipt.instance_id == owner.instance_id)
        {
            return Err("cem.checkpoint.owner");
        }
    }
    let sealed = serde_json::to_string(&candidate).map_err(|_| "cem.checkpoint.format")?;
    if sealed.len() > MAX_CHECKPOINT_BYTES {
        return Err("cem.checkpoint.limit");
    }
    Ok(sealed)
}
