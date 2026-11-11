//! Passive original-response transport. A capsule/pin is not permission to
//! allocate a heap or bind UI: the trusted original retirement transaction
//! must supply that authority separately, just as for native receiver sources.
use super::*;
use cem_ml::committed_checkpoint::{self, WriteJournal};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const MAGIC: &[u8] = b"CEMAPP1\n";
const REFUSED: &str = "cem.application.response_checkpoint.refused";
const MAX_METADATA: usize = 64 * 1024;

pub(super) struct ResponseOwner {
    operation: String,
    uri: String,
    bytes: Vec<u8>,
    tree: Arc<RetainedCemTree>,
    journal: WriteJournal,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    deployment: String,
    operation: String,
    uri: String,
    sequence: u64,
    operations: BTreeSet<String>,
}

/// Native attestation of the exact original response and terminal journal.
/// Only the original physical transaction may deliver it to a fresh heap.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryPin {
    response: String,
    journal: String,
    deployment: String,
    runtime_id: String,
    instance_id: String,
}

fn hash(bytes: &[u8]) -> String {
    ContentHash::from_blake3(bytes).header_value()
}

impl SelectedApplication<'_> {
    pub(in crate::application_runtime) fn active(&self) -> Result<(), String> {
        if self.retired.get() {
            Err(REFUSED.into())
        } else {
            Ok(())
        }
    }

    pub(in crate::application_runtime) fn deployment(&self) -> &str {
        &self.deployment
    }

    fn prepare_response(
        &self,
        operation: &str,
        bytes: &[u8],
        uri: &str,
    ) -> Result<(String, Arc<RetainedCemTree>), String> {
        self.active()?;
        check_input(operation, bytes, uri)?;
        let control = render_control()?;
        let tree = self
            .graphql
            .admit_response(operation, bytes, uri, &control)?
            .into_retained_tree()
            .map_err(|_| "cem.application.graphql.projection")?;
        let html = self.render_tree(tree.clone(), &control)?;
        Ok((html, tree))
    }

    /// Commit only a fully guarded, successfully rendered native source.
    pub(in crate::application_runtime) fn retain_response(
        &self,
        operation: &str,
        bytes: &[u8],
        uri: &str,
    ) -> Result<String, String> {
        let (html, tree) = self.prepare_response(operation, bytes, uri)?;
        let mut state = self.response.borrow_mut();
        let journal = state
            .take()
            .map_or_else(WriteJournal::empty, |owner| owner.journal);
        *state = Some(ResponseOwner {
            operation: operation.into(),
            uri: uri.into(),
            bytes: bytes.into(),
            tree,
            journal,
        });
        cem_ml::runtime_inventory::note_mutation();
        Ok(html)
    }

    /// Claim before delivering a new explicit write. Failed claims are inert.
    pub(in crate::application_runtime) fn claim_mutation(
        &self,
        operation_id: &str,
    ) -> Result<u64, String> {
        self.active()?;
        let mut state = self.response.borrow_mut();
        let owner = state.as_mut().ok_or(REFUSED)?;
        if owner.journal.operations().contains(operation_id) {
            return Err(REFUSED.into());
        }
        let sequence = owner.journal.sequence().checked_add(1).ok_or(REFUSED)?;
        if !owner.journal.claim_write(sequence, operation_id) {
            return Err(REFUSED.into());
        }
        cem_ml::runtime_inventory::note_mutation();
        Ok(sequence)
    }

    pub(in crate::application_runtime) fn capture_response(&self) -> Result<Vec<u8>, String> {
        self.encode_response(true)
    }

    fn encode_response(&self, checkpoint_limits: bool) -> Result<Vec<u8>, String> {
        self.active()?;
        let state = self.response.borrow();
        let owner = state.as_ref().ok_or(REFUSED)?;
        let metadata = serde_json::to_vec(&Metadata {
            deployment: self.deployment.clone(),
            operation: owner.operation.clone(),
            uri: owner.uri.clone(),
            sequence: owner.journal.sequence(),
            operations: owner.journal.operations().clone(),
        })
        .map_err(|_| REFUSED)?;
        if checkpoint_limits
            && (metadata.len() > MAX_METADATA
                || MAGIC.len() + 4 + metadata.len() + owner.bytes.len()
                    > committed_checkpoint::MAX_CHECKPOINT_BYTES)
        {
            return Err(REFUSED.into());
        }
        let mut capsule = Vec::with_capacity(MAGIC.len() + 4 + metadata.len() + owner.bytes.len());
        capsule.extend_from_slice(MAGIC);
        capsule.extend_from_slice(&(metadata.len() as u32).to_be_bytes());
        capsule.extend_from_slice(&metadata);
        capsule.extend_from_slice(&owner.bytes);
        Ok(capsule)
    }

    pub(in crate::application_runtime) fn response_revision(&self) -> Result<String, String> {
        // Publication remains valid when an otherwise admitted response is too
        // large for a checkpoint. Such an owner simply cannot be captured.
        Ok(hash(&self.encode_response(false)?))
    }

    /// Freeze the original source only after checking its exact current capsule
    /// and the trusted host's terminal receipt. This is not physical retirement.
    pub(in crate::application_runtime) fn confirm_response_retirement(
        &self,
        capsule: &[u8],
        journal: &[u8],
        runtime_id: &str,
        instance_id: &str,
    ) -> Result<String, String> {
        if runtime_id.len() > 1024
            || instance_id.len() > 1024
            || journal.len() > committed_checkpoint::MAX_WORKSPACE_CHECKPOINT_BYTES
            || capsule != self.capture_response()?
        {
            return Err(REFUSED.into());
        }
        let pin = RecoveryPin {
            response: hash(capsule),
            journal: hash(journal),
            deployment: self.deployment.clone(),
            runtime_id: runtime_id.into(),
            instance_id: instance_id.into(),
        };
        let fence = pin.read_journal(journal)?;
        let state = self.response.borrow();
        let owner = state.as_ref().ok_or(REFUSED)?;
        if fence.sequence() != owner.journal.sequence()
            || fence.operations() != owner.journal.operations()
        {
            return Err(REFUSED.into());
        }
        let encoded = serde_json::to_string(&pin).map_err(|_| REFUSED)?;
        drop(state);
        self.retired.set(true);
        self.dispose_response();
        Ok(encoded)
    }

    /// Reimport passive original bytes under the linked guard and compiled
    /// entry. Never fetch, dispatch an action, restore pointers or grant restart.
    pub(in crate::application_runtime) fn restore_response(
        &self,
        capsule: &[u8],
        pin: &str,
        journal: &[u8],
        previous_runtime_id: &str,
    ) -> Result<String, String> {
        self.active()?;
        if self.response.borrow().is_some()
            || capsule.len() > committed_checkpoint::MAX_CHECKPOINT_BYTES
            || pin.len() > 16 * 1024
            || journal.len() > committed_checkpoint::MAX_WORKSPACE_CHECKPOINT_BYTES
            || previous_runtime_id.len() > 1024
        {
            return Err(REFUSED.into());
        }
        let pin: RecoveryPin = serde_json::from_str(pin).map_err(|_| REFUSED)?;
        if pin.deployment != self.deployment
            || pin.runtime_id != previous_runtime_id
            || pin.response != hash(capsule)
            || pin.journal != hash(journal)
        {
            return Err(REFUSED.into());
        }
        let fence = pin.read_journal(journal)?;
        let body = capsule.strip_prefix(MAGIC).ok_or(REFUSED)?;
        let length = u32::from_be_bytes(
            body.get(..4)
                .ok_or(REFUSED)?
                .try_into()
                .map_err(|_| REFUSED)?,
        ) as usize;
        if length > MAX_METADATA {
            return Err(REFUSED.into());
        }
        let encoded = body.get(4..4 + length).ok_or(REFUSED)?;
        let metadata: Metadata = serde_json::from_slice(encoded).map_err(|_| REFUSED)?;
        if serde_json::to_vec(&metadata).map_err(|_| REFUSED)? != encoded
            || metadata.deployment != self.deployment
            || metadata.sequence != fence.sequence()
            || &metadata.operations != fence.operations()
        {
            return Err(REFUSED.into());
        }
        let bytes = &body[4 + length..];
        let (html, tree) = self.prepare_response(&metadata.operation, bytes, &metadata.uri)?;
        *self.response.borrow_mut() = Some(ResponseOwner {
            operation: metadata.operation,
            uri: metadata.uri,
            bytes: bytes.into(),
            tree,
            journal: fence,
        });
        cem_ml::runtime_inventory::note_mutation();
        Ok(html)
    }

    pub(in crate::application_runtime) fn dispose_response(&self) {
        if self.response.borrow_mut().take().is_some() {
            cem_ml::runtime_inventory::note_mutation();
        }
    }

    pub(in crate::application_runtime) fn response_inventory(&self) -> (usize, usize) {
        self.response.borrow().as_ref().map_or((0, 0), |owner| {
            debug_assert_eq!(owner.tree.source_uri(), owner.uri);
            (1, owner.bytes.len())
        })
    }

    #[cfg(test)]
    pub(in crate::application_runtime) fn retained_response_tree(
        &self,
    ) -> Option<Arc<RetainedCemTree>> {
        self.response
            .borrow()
            .as_ref()
            .map(|owner| owner.tree.clone())
    }
}

impl RecoveryPin {
    fn read_journal(&self, bytes: &[u8]) -> Result<WriteJournal, String> {
        let checkpoint = committed_checkpoint::read_sealed(bytes)?;
        WriteJournal::from_terminal_owner(
            &checkpoint.receipt,
            &self.runtime_id,
            &self.instance_id,
            &self.deployment,
            &self.response,
        )
        .map_err(|_| REFUSED.into())
    }
}
