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
        relationships: vec![],
        native_runtimes: vec![],
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

#[test]
fn explicitly_reconciled_not_applied_outcomes_are_terminal_metadata_not_replay() {
    use cem_ml::committed_checkpoint::{OperationOutcome, OperationReceipt};
    let mut checkpoint = candidate();
    checkpoint.receipt.operations.push(OperationReceipt {
        instance_id: "editor".into(),
        operation_id: "opaque:1".into(),
        outcome: OperationOutcome::NotApplied,
    });
    let receipt = serde_json::to_string(&checkpoint.receipt).unwrap();
    let restored: Checkpoint =
        serde_json::from_str(&seal(&encoded(&checkpoint), &receipt).unwrap()).unwrap();
    assert_eq!(
        restored.receipt.operations[0].outcome,
        OperationOutcome::NotApplied
    );
    checkpoint.receipt.operations.clear();
    assert!(seal(&encoded(&checkpoint), &receipt).is_err());
}

#[test]
fn relationship_contracts_require_complete_current_owners_and_fresh_authority() {
    use serde_json::{json, Value};
    let mut c = serde_json::to_value(candidate()).unwrap();
    c["receipt"]["owners"].as_array_mut().unwrap().push(json!({"instanceId":"menu","declarationIdentity":"source:menu","dataRevision":"view:3","acceptedEventSequence":0}));
    c["owners"]
        .as_array_mut()
        .unwrap()
        .push(json!({"instanceId":"menu","island":"<template/>"}));
    let relationship = json!({"id":"editor-menu","kind":"editor-provider","requester":{"instanceId":"menu","dataRevision":"view:3","nodeId":"host"},"provider":{"instanceId":"editor","dataRevision":"draft:2","nodeId":"field.1"},"property":"suggestions","disposition":"reacquire"});
    c["receipt"]["relationships"] = json!([relationship]);
    let check = |v: &Value| seal(&v.to_string(), &v["receipt"].to_string());
    assert!(check(&c).is_ok());
    let mut placement = c.clone();
    placement["receipt"]["relationships"][0]["kind"] = "placement".into();
    placement["receipt"]["relationships"][0]["property"] = "aria-controls".into();
    assert!(check(&placement).is_ok());
    for (field, value) in [
        ("disposition", json!("restore-token")),
        ("property", json!("onclick")),
        ("token", json!("old-authority")),
    ] {
        let mut invalid = placement.clone();
        invalid["receipt"]["relationships"][0][field] = value;
        assert!(check(&invalid).is_err(), "{invalid}");
    }
    for (field, value) in [
        ("instanceId", json!("outside-domain")),
        ("dataRevision", json!("obsolete")),
        ("nodeId", json!("")),
    ] {
        let mut invalid = c.clone();
        invalid["receipt"]["relationships"][0]["provider"][field] = value;
        assert!(check(&invalid).is_err());
    }
    let mut duplicate = c.clone();
    let mut second = relationship.clone();
    second["id"] = "another".into();
    duplicate["receipt"]["relationships"]
        .as_array_mut()
        .unwrap()
        .push(second);
    assert!(check(&duplicate).is_err());
    let mut excessive = c.clone();
    excessive["receipt"]["relationships"] = json!(vec![relationship; 257]);
    assert!(check(&excessive).is_err());
}

#[test]
fn shared_domain_cross_runtime_endpoints_require_complete_matching_owner_receipts() {
    use cem_ml::committed_checkpoint::{Endpoint, Relationship};
    let mut c = candidate();
    c.receipt.owners.push(OwnerReceipt {
        instance_id: "other-runtime:surface".into(),
        declaration_identity: "dialog:1".into(),
        data_revision: "7".into(),
        accepted_event_sequence: 0,
    });
    c.owners.push(OwnerCheckpoint {
        instance_id: "other-runtime:surface".into(),
        island: "<template>dialog</template>".into(),
    });
    c.receipt.relationships.push(Relationship {
        id: "return:1".into(),
        kind: "placement".into(),
        requester: Endpoint {
            instance_id: "other-runtime:surface".into(),
            data_revision: "7".into(),
            node_id: "host".into(),
        },
        provider: Endpoint {
            instance_id: "editor".into(),
            data_revision: "draft:2".into(),
            node_id: "field.1".into(),
        },
        property: "return-focus".into(),
        disposition: "reacquire".into(),
    });
    let receipt = serde_json::to_string(&c.receipt).unwrap();
    let saved = seal(&encoded(&c), &receipt).unwrap();
    assert_eq!(seal(&saved, &receipt).unwrap(), saved);
    let mut foreign_receipt = c.receipt.clone();
    foreign_receipt.runtime_id = "foreign-domain".into();
    assert!(seal(
        &encoded(&c),
        &serde_json::to_string(&foreign_receipt).unwrap()
    )
    .is_err());
    for revision in ["foreign-domain:7", "unpublished:8"] {
        let mut bad = c.clone();
        bad.receipt.relationships[0].provider.data_revision = revision.into();
        assert!(seal(
            &encoded(&bad),
            &serde_json::to_string(&bad.receipt).unwrap()
        )
        .is_err());
    }
    let mut bad = c.clone();
    bad.receipt.owners.pop();
    bad.owners.pop();
    assert!(seal(
        &encoded(&bad),
        &serde_json::to_string(&bad.receipt).unwrap()
    )
    .is_err());
    let mut bad: serde_json::Value = serde_json::from_str(&encoded(&c)).unwrap();
    bad["receipt"]["relationships"][0]["provider"]["runtimeId"] =
        serde_json::json!("forged-runtime");
    assert!(seal(&bad.to_string(), &receipt).is_err());
}

#[test]
fn physical_runtime_client_coverage_is_complete_bounded_and_inert() {
    let mut candidate = serde_json::to_value(candidate()).unwrap();
    candidate["receipt"]["nativeRuntimes"] = serde_json::json!([{
        "runtimeId":"worker-actual-generation", "generation":1, "mode":"worker", "revision":4,
        "clients":[{"clientId":"root-native-client","owners":["editor"]}]
    }]);
    let seal_value =
        |value: &serde_json::Value| seal(&value.to_string(), &value["receipt"].to_string());
    assert!(seal_value(&candidate).is_ok());
    let mut shared = candidate.clone();
    shared["receipt"]["nativeRuntimes"].as_array_mut().unwrap().push(serde_json::json!({
        "runtimeId":"external-source-generation", "generation":1, "mode":"main-thread", "revision":2,
        "clients":[{"clientId":"external-native-source","owners":["editor"]}]
    }));
    assert!(seal_value(&shared).is_ok());
    for field in ["generation", "revision"] {
        let mut bad = candidate.clone();
        bad["receipt"]["nativeRuntimes"][0][field] = serde_json::json!(9_007_199_254_740_992_u64);
        assert!(seal_value(&bad).is_err());
    }
    for owners in [
        serde_json::json!([]),
        serde_json::json!(["foreign"]),
        serde_json::json!(["editor", "editor"]),
    ] {
        let mut bad = candidate.clone();
        bad["receipt"]["nativeRuntimes"][0]["clients"][0]["owners"] = owners;
        assert!(seal_value(&bad).is_err());
    }
    let mut bad = candidate.clone();
    bad["receipt"]["nativeRuntimes"][0]["mode"] = serde_json::json!("other");
    assert!(seal_value(&bad).is_err());
    let mut bad = candidate.clone();
    bad["receipt"]["nativeRuntimes"][0]["pointer"] = serde_json::json!(1);
    assert!(seal_value(&bad).is_err());
    let mut bad = candidate.clone();
    let runtime = bad["receipt"]["nativeRuntimes"][0].clone();
    bad["receipt"]["nativeRuntimes"]
        .as_array_mut()
        .unwrap()
        .push(runtime);
    assert!(seal_value(&bad).is_err());
    let mut bad = candidate.clone();
    let client = bad["receipt"]["nativeRuntimes"][0]["clients"][0].clone();
    bad["receipt"]["nativeRuntimes"][0]["clients"]
        .as_array_mut()
        .unwrap()
        .push(client);
    assert!(seal_value(&bad).is_err());
    let mut stale = candidate.clone();
    stale["receipt"]["nativeRuntimes"][0]["runtimeId"] = serde_json::json!("old");
    assert!(seal(&stale.to_string(), &candidate["receipt"].to_string()).is_err());
}
