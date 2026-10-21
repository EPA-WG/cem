use cem_ml::runtime_inventory::{
    inventory_revision, NativeActivity, NativeRegistry, RegistryInventory,
    RuntimeResourceInventory, COMMAND_REGISTRIES,
};

static MUTATIONS: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn empty() -> Vec<RegistryInventory> {
    COMMAND_REGISTRIES
        .iter()
        .map(|name| RegistryInventory::new(name, 0, 0))
        .collect()
}

#[test]
fn census_requires_the_entire_linked_manifest_and_bounded_counters() {
    let inventory = RuntimeResourceInventory::collect(empty()).unwrap();
    assert_eq!(inventory.blocking_resources, 0);
    assert!(!inventory.replacement_allowed);
    assert_eq!(inventory.registries.len(), COMMAND_REGISTRIES.len());
    let mut missing = empty();
    missing.pop();
    assert!(RuntimeResourceInventory::collect(missing).is_err());
    let mut duplicate = empty();
    duplicate.push(duplicate[0].clone());
    assert!(RuntimeResourceInventory::collect(duplicate).is_err());
    let mut unknown = empty();
    unknown[0].name = "unaccounted".into();
    assert!(RuntimeResourceInventory::collect(unknown).is_err());
    let mut unsafe_discard = empty();
    unsafe_discard[0].discardable = 1;
    assert!(RuntimeResourceInventory::collect(unsafe_discard).is_err());
    let mut huge = empty();
    huge[0].live = u64::MAX;
    assert!(RuntimeResourceInventory::collect(huge).is_err());
}

#[test]
fn live_roots_block_while_acknowledgeable_history_and_cache_counts_remain_visible() {
    let mut entries = empty();
    entries
        .iter_mut()
        .find(|e| e.name == "command-artifacts")
        .unwrap()
        .live = 2;
    entries
        .iter_mut()
        .find(|e| e.name == "command-artifact-journal")
        .unwrap()
        .discardable = 4;
    entries
        .iter_mut()
        .find(|e| e.name == "template-cache")
        .unwrap()
        .discardable = 3;
    let inventory = RuntimeResourceInventory::collect(entries).unwrap();
    assert_eq!(inventory.blocking_resources, 2);
    assert_eq!(
        inventory
            .registries
            .iter()
            .map(|e| e.discardable)
            .sum::<u64>(),
        7
    );
    assert!(!inventory.replacement_allowed);
}

#[test]
fn registry_mutations_and_async_bridge_lifetimes_invalidate_count_equal_snapshots() {
    let _serial = MUTATIONS.lock().unwrap();
    let registry = NativeRegistry::new(vec![1]);
    let before = inventory_revision();
    assert_eq!(registry.borrow().len(), 1);
    assert_eq!(inventory_revision(), before);
    registry.borrow_mut()[0] = 2;
    assert!(inventory_revision() > before);
    let bridge = NativeActivity::begin();
    assert!(NativeActivity::active() > 0);
    let before_drop = inventory_revision();
    drop(bridge);
    assert_eq!(NativeActivity::active(), 0);
    assert!(inventory_revision() > before_drop);
}

#[test]
fn command_registries_count_real_active_controls_artifacts_and_tombstones() {
    let _serial = MUTATIONS.lock().unwrap();
    use cem_ml::command_artifact::CommandServiceArtifactRegistryV1;
    use cem_ml::command_runtime::CommandServiceOperationRegistryV1;
    let operations = CommandServiceOperationRegistryV1::default();
    assert_eq!(operations.inventory_count(), 0);
    let active = operations
        .register(
            "inventory-case",
            cem_ml::operation_control::OperationControl::default(),
        )
        .unwrap();
    assert_eq!(operations.inventory_count(), 1);
    drop(active);
    assert_eq!(operations.inventory_count(), 0);
    let artifacts = CommandServiceArtifactRegistryV1::default();
    assert_eq!(artifacts.inventory_counts(), (0, 0));
    use cem_ml::command_artifact::CommandServiceRetainedArtifactV1;
    use cem_ml::command_service::{sha256_hex, CommandArtifactHandleV1, CommandArtifactKindV1};
    use cem_ml::operation_handle::RetainedHandleId;
    let handle = RetainedHandleId::from_raw(1);
    artifacts.begin_request("inventory-case").unwrap();
    assert_eq!(artifacts.inventory_counts(), (0, 0));
    artifacts
        .publish(
            "inventory-case",
            vec![CommandServiceRetainedArtifactV1 {
                handle: CommandArtifactHandleV1 {
                    handle_id: handle,
                    kind: CommandArtifactKindV1::Output,
                    uri: Some("memory:inventory".into()),
                    content_type: "application/octet-stream".into(),
                    byte_length: 3,
                    sha256: sha256_hex(b"abc"),
                    source_map_id: None,
                },
                bytes: b"abc".to_vec(),
            }],
        )
        .unwrap();
    assert_eq!(artifacts.inventory_counts(), (1, 1));
    artifacts.dispose("inventory-case", handle).unwrap();
    assert_eq!(artifacts.inventory_counts(), (0, 2));
    artifacts.dispose_request("inventory-case").unwrap();
    assert_eq!(artifacts.inventory_counts(), (0, 2));
}

#[test]
fn wasm_registry_boundaries_are_versioned_instead_of_untracked_raw_refcells() {
    // This architectural fixture makes new retained stores opt into the census.
    let sources: &[(&str, &[&str])] = &[
        (
            include_str!("../src/api/wasm.rs"),
            &[
                "PARSE_OBSERVER",
                "VALIDATE_OBSERVER",
                "TRANSFORM_OBSERVER",
                "RESOLVER_READ",
                "RESOLVER_WRITE",
                "RESUMABLE_OPERATION_HOSTS",
                "NEXT_RESUMABLE_OPERATION_HOST_ID",
            ],
        ),
        (include_str!("../src/api/wasm/documents.rs"), &["DOCUMENTS"]),
        (
            include_str!("../src/api/wasm/xpath_artifact.rs"),
            &["PROGRAMS"],
        ),
        (
            include_str!("../src/api/wasm/command_service.rs"),
            &[
                "COMMAND_RUNTIME_FACTORY",
                "COMMAND_SERVICE_OPERATIONS",
                "COMMAND_SERVICE_ARTIFACTS",
            ],
        ),
        (
            include_str!("../../cem_ql/src/api/wasm.rs"),
            &["COMPILATION_CACHE", "ARTIFACTS"],
        ),
        (
            include_str!("../../cem_ql/src/api/wasm/render_reuse.rs"),
            &["RENDERS"],
        ),
        (
            include_str!("../../cem_ql/src/api/wasm/values.rs"),
            &["INPUTS", "SOURCES", "OUTPUT", "NEXT", "VALIDATIONS"],
        ),
        (
            include_str!("../../cem_ql/src/api/wasm/values/lifecycle.rs"),
            &["EXECUTIONS", "SNAPSHOTS"],
        ),
        (
            include_str!("../../cem_ql/src/api/wasm/values/capability_sessions.rs"),
            &["SESSIONS"],
        ),
        (
            include_str!("../../cem_ql/src/api/wasm/xpath_functions.rs"),
            &["COMPANIONS"],
        ),
        (
            include_str!("../../cem_ql/src/api/wasm/xslt_bundle.rs"),
            &["BUNDLES", "COMPONENTS"],
        ),
        (
            include_str!("../src/transform_template.rs"),
            &["CEMT_FUNCTION_CALL_ARG_CACHE", "CEMT_OBJECT_LITERAL_CACHE"],
        ),
    ];
    for (source, expected) in sources {
        assert!(!source.contains("use std::cell::RefCell"));
        // A newly retained store must be reviewed for census/revision coverage.
        let actual: Vec<_> = source
            .lines()
            .flat_map(|line| line.split("static ").skip(1))
            .filter_map(|part| part.split_once(':').map(|(name, _)| name.trim()))
            .filter(|name| name.chars().all(|c| c.is_ascii_uppercase() || c == '_'))
            .collect();
        assert_eq!(&actual, expected);
    }
}
