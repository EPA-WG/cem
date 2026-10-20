use cem_ml::committed_checkpoint::{
    seal, Checkpoint, CommitReceipt, OwnerCheckpoint, OwnerReceipt,
};
use cem_ml::runtime_memory::{MemoryBudgetConfig, MemorySample, RuntimeMemoryBudget};

fn receipt() -> CommitReceipt {
    CommitReceipt {
        runtime_id: "runtime-a".into(),
        generation: 1,
        commit_sequence: 3,
        policy_stamp: "private-session".into(),
        operations: vec![],
        owners: vec![OwnerReceipt {
            instance_id: "editor".into(),
            declaration_identity: "source:1".into(),
            data_revision: "draft:2".into(),
            accepted_event_sequence: 2,
        }],
    }
}
fn candidate() -> Checkpoint {
    Checkpoint {
        version: "cem-committed-checkpoint/1".into(),
        receipt: receipt(),
        owners: vec![OwnerCheckpoint {
            instance_id: "editor".into(),
            island: "<template data-cem-data-island>unsaved text</template>".into(),
        }],
    }
}
fn encoded(value: &Checkpoint) -> String {
    serde_json::to_string(value).unwrap()
}
fn expected() -> String {
    serde_json::to_string(&receipt()).unwrap()
}

#[test]
fn committed_checkpoint_round_trip_has_only_portable_owned_state() {
    let bytes = seal(&encoded(&candidate()), &expected()).unwrap();
    let restored: Checkpoint = serde_json::from_str(&bytes).unwrap();
    assert_eq!(restored, candidate());
    assert!(seal(&bytes, &expected()).is_ok());
}
#[test]
fn rejects_stale_partial_mixed_duplicate_and_unacknowledged_state() {
    let mut changed = candidate();
    changed.receipt.generation = 2;
    assert!(seal(&encoded(&changed), &expected()).is_err());
    changed = candidate();
    changed.receipt.owners[0].data_revision = "uncommitted".into();
    assert!(seal(&encoded(&changed), &expected()).is_err());
    changed = candidate();
    changed.owners.clear();
    assert!(seal(&encoded(&changed), &expected()).is_err());
    changed = candidate();
    changed.owners.push(changed.owners[0].clone());
    assert!(seal(&encoded(&changed), &expected()).is_err());
    changed = candidate();
    changed.owners[0].instance_id = "other".into();
    assert!(seal(&encoded(&changed), &expected()).is_err());
}
#[test]
fn rejects_unknown_versions_fields_oversize_and_unsafe_counters() {
    let mut changed = candidate();
    changed.version = "future".into();
    assert!(seal(&encoded(&changed), &expected()).is_err());
    let json = encoded(&candidate()).replacen('{', "{\"pointer\":10,", 1);
    assert!(seal(&json, &expected()).is_err());
    changed = candidate();
    changed.owners[0].island = "x".repeat(1024 * 1024);
    assert!(seal(&encoded(&changed), &expected()).is_err());
    changed = candidate();
    changed.receipt.commit_sequence = 9_007_199_254_740_992;
    assert!(seal(
        &encoded(&changed),
        &serde_json::to_string(&changed.receipt).unwrap()
    )
    .is_err());
}
#[test]
fn recovery_reserves_exclusive_headroom_and_releases_on_cancel_or_drop() {
    let budget = RuntimeMemoryBudget::new(MemoryBudgetConfig {
        soft_limit_bytes: 70,
        hard_limit_bytes: 100,
        recovery_headroom_bytes: 20,
        max_concurrent_operations: 2,
    })
    .unwrap();
    let sample = MemorySample {
        allocator_live_bytes: 80,
        allocator_peak_bytes: 80,
        linear_memory_bytes: None,
    };
    assert!(budget.admit(sample, 1).is_err());
    assert!(budget.admit_recovery(sample, 21).is_err());
    let recovery = budget.admit_recovery(sample, 20).unwrap();
    assert!(budget.admit_recovery(sample, 1).is_err());
    assert!(budget
        .admit(
            MemorySample {
                allocator_live_bytes: 0,
                ..sample
            },
            1
        )
        .is_err());
    drop(recovery);
    let normal = budget
        .admit(
            MemorySample {
                allocator_live_bytes: 0,
                ..sample
            },
            1,
        )
        .unwrap();
    assert!(budget.admit_recovery(sample, 1).is_err());
    drop(normal);
    assert!(budget.admit_recovery(sample, 20).is_ok());
}

#[test]
fn external_action_ledger_is_exact_and_unknown_outcomes_are_not_replayed() {
    use cem_ml::committed_checkpoint::{OperationOutcome, OperationReceipt};
    let mut checkpoint = candidate();
    checkpoint.receipt.operations.push(OperationReceipt {
        instance_id: "editor".into(),
        operation_id: "save:1".into(),
        outcome: OperationOutcome::Unknown,
    });
    let expected = serde_json::to_string(&checkpoint.receipt).unwrap();
    let restored: Checkpoint =
        serde_json::from_str(&seal(&encoded(&checkpoint), &expected).unwrap()).unwrap();
    assert_eq!(
        restored.receipt.operations[0].outcome,
        OperationOutcome::Unknown
    );
    checkpoint.receipt.operations[0].outcome = OperationOutcome::Acknowledged;
    assert!(seal(&encoded(&checkpoint), &expected).is_err());
    checkpoint.receipt.operations[0].instance_id = "absent".into();
    assert!(seal(
        &encoded(&checkpoint),
        &serde_json::to_string(&checkpoint.receipt).unwrap()
    )
    .is_err());
}

#[test]
fn complete_multi_owner_set_is_bounded_and_duplicate_receipts_fail_closed() {
    let mut checkpoint = candidate();
    for index in 1..256 {
        let id = format!("owner-{index}");
        checkpoint.receipt.owners.push(OwnerReceipt {
            instance_id: id.clone(),
            ..receipt().owners[0].clone()
        });
        checkpoint.owners.push(OwnerCheckpoint {
            instance_id: id,
            island: "<template/>".into(),
        });
    }
    let expected = serde_json::to_string(&checkpoint.receipt).unwrap();
    assert!(seal(&encoded(&checkpoint), &expected).is_ok());
    checkpoint.owners.pop();
    assert!(seal(&encoded(&checkpoint), &expected).is_err());
    checkpoint.receipt.owners.push(receipt().owners[0].clone());
    assert!(seal(
        &encoded(&checkpoint),
        &serde_json::to_string(&checkpoint.receipt).unwrap()
    )
    .is_err());
    checkpoint = candidate();
    checkpoint.receipt.owners.push(receipt().owners[0].clone());
    checkpoint.owners.push(checkpoint.owners[0].clone());
    assert!(seal(
        &encoded(&checkpoint),
        &serde_json::to_string(&checkpoint.receipt).unwrap()
    )
    .is_err());
}
