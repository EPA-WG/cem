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
    /// Inert host-described dependencies. Fresh grants/leases are mandatory.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relationships: Vec<Relationship>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Endpoint {
    pub instance_id: String,
    pub data_revision: String,
    pub node_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Relationship {
    pub id: String,
    pub kind: String,
    pub requester: Endpoint,
    pub provider: Endpoint,
    pub property: String,
    pub disposition: String,
}
fn valid_relationships(receipt: &CommitReceipt) -> bool {
    if receipt.relationships.len() > 256 {
        return false;
    }
    let mut ids = BTreeSet::new();
    let mut targets = BTreeSet::new();
    let mut editors = BTreeSet::new();
    for r in &receipt.relationships {
        if !identity(&r.id) || !ids.insert(&r.id) || r.disposition != "reacquire" {
            return false;
        }
        for endpoint in [&r.requester, &r.provider] {
            if !identity(&endpoint.node_id)
                || !receipt.owners.iter().any(|owner| {
                    owner.instance_id == endpoint.instance_id
                        && owner.data_revision == endpoint.data_revision
                })
            {
                return false;
            }
        }
        if !targets.insert((&r.requester.instance_id, &r.requester.node_id, &r.property)) {
            return false;
        }
        match r.kind.as_str() {
            "editor-provider" => {
                if r.property != "suggestions"
                    || r.requester == r.provider
                    || !editors.insert((&r.provider.instance_id, &r.provider.node_id))
                {
                    return false;
                }
            }
            "placement" => {
                if !placement_property(&r.property) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
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
    if !valid_relationships(receipt) {
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

pub(crate) fn placement_property(property: &str) -> bool {
    matches!(
        property,
        "commandfor"
            | "popovertarget"
            | "interestfor"
            | "form"
            | "list"
            | "for"
            | "aria-controls"
            | "aria-labelledby"
            | "aria-describedby"
            | "aria-owns"
            | "aria-activedescendant"
            | "aria-details"
            | "aria-errormessage"
            | "headers"
            | "command-target"
            | "interaction"
            | "trigger-for"
            | "parent-item"
            | "focus-target"
            | "return-focus"
            | "anchor"
            | "boundary"
            | "editor-for"
    )
}
