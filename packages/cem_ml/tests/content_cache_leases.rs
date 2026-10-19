use cem_ml::content_cache::*;
use cem_ml::source::{ByteRange, SourceId};
use cem_ml::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};

fn policy(name: &str) -> PolicyStamps {
    PolicyStamps::new(name)
}
fn entry(text: &str) -> (CacheArtifact, Option<SourceMapSidecar>) {
    let map = SourceMapStack {
        frames: vec![SourceMapFrame {
            source_id: SourceId(1),
            span: FrameSpan::Single(ByteRange::new(0, 3)),
            transform: TransformKind::SourceBoundary {
                uri: "studio://project/source.cem".into(),
            },
        }],
    };
    CacheArtifact::new(
        ArtifactContentType::CemMlDocument,
        CacheMode::Dev,
        text.as_bytes().to_vec(),
        policy("A"),
        Some(map),
    )
}
fn cache(entries: usize, bytes: usize) -> ContentCache {
    ContentCache::with_limits(ContentCacheLimits {
        max_entries: entries,
        max_bytes: bytes,
    })
    .unwrap()
}
fn charge(value: &(CacheArtifact, Option<SourceMapSidecar>)) -> usize {
    let mut probe = ContentCache::new();
    probe.insert(value.0.clone(), value.1.clone()).unwrap();
    probe.stats().retained_bytes
}
#[test]
fn lru_eviction_removes_the_owning_sidecar_and_updates_accounting() {
    let (a, sa) = entry("A");
    let (b, mut sb) = entry("B");
    let (c, sc) = entry("C");
    sb.as_mut().unwrap().source_map.frames[0].span = FrameSpan::Single(ByteRange::new(3, 3));
    sb = Some(SourceMapSidecar::new(sb.unwrap().source_map));
    let mut b = b;
    b.source_map_sidecar_hash = sb.as_ref().map(|s| s.hash.clone());
    let bmap = sb.as_ref().unwrap().hash.clone();
    let mut store = cache(2, 100_000);
    store.insert(a.clone(), sa).unwrap();
    store.insert(b.clone(), sb).unwrap();
    store.get(&a.key, &policy("A")).unwrap(); // borrowed reads also promote recency
    store.insert(c.clone(), sc).unwrap();
    assert_eq!(
        store.get(&b.key, &policy("A")).unwrap_err().code,
        "cem.cc.cache_evicted"
    );
    assert!(store.sidecar(&bmap).is_none());
    assert!(store.get(&a.key, &policy("A")).is_ok());
    assert_eq!(store.stats().resident_entries, 2);
    assert_eq!(store.stats().evictions, 1);
}
#[test]
fn pins_survive_pressure_and_rejected_admission_does_not_evict_siblings() {
    let a = entry("A");
    let b = entry("B");
    let budget = charge(&a) + charge(&b);
    let mut store = cache(2, budget);
    store.insert(a.0.clone(), a.1).unwrap();
    store.insert(b.0.clone(), b.1).unwrap();
    let pin = store.lease(&b.0.key, &policy("A")).unwrap();
    let (large, sidecar) = entry(&"x".repeat(budget / 2 + 1));
    assert_eq!(
        store.insert(large, sidecar).unwrap_err().code,
        "cem.cc.capacity"
    );
    assert!(store.get(&a.0.key, &policy("A")).is_ok());
    assert_eq!(pin.artifact(&policy("A")).unwrap().bytes, b"B");
    assert_eq!(store.stats().evictions, 0);
    let second = store.lease(&a.0.key, &policy("A")).unwrap();
    let (c, sc) = entry("C");
    assert_eq!(
        store.insert(c.clone(), sc.clone()).unwrap_err().code,
        "cem.cc.capacity"
    );
    drop(second);
    store.insert(c, sc).unwrap();
    assert_eq!(store.stats().resident_entries, 2);
    assert_eq!(store.stats().live_leases, 1);
}
#[test]
fn byte_limit_evicts_even_below_entry_limit_and_oversize_rejects_atomically() {
    let a = entry(&"a".repeat(400));
    let b = entry(&"b".repeat(400));
    let size = charge(&a);
    let mut store = cache(8, size * 2 - 1);
    store.insert(a.0.clone(), a.1).unwrap();
    store.insert(b.0.clone(), b.1).unwrap();
    assert_eq!(store.stats().resident_entries, 1);
    assert_eq!(store.stats().retained_bytes, size);
    assert!(store.get(&a.0.key, &policy("A")).is_err());
    let oversized = entry(&"z".repeat(size * 2));
    assert_eq!(
        store.insert(oversized.0, oversized.1).unwrap_err().code,
        "cem.cc.capacity"
    );
    assert!(store.get(&b.0.key, &policy("A")).is_ok());
    assert_eq!(store.stats().evictions, 1);
}
#[test]
fn immutable_leases_deduplicate_and_block_conflicting_policy_replacement() {
    let (a, sidecar) = entry("A");
    let mut store = cache(1, 100_000);
    store.insert(a.clone(), sidecar.clone()).unwrap();
    let size = store.stats().retained_bytes;
    let lease = store.lease(&a.key, &policy("A")).unwrap();
    let copy = lease.clone();
    store.insert(a.clone(), sidecar.clone()).unwrap();
    assert_eq!(store.stats().retained_bytes, size);
    assert_eq!(store.stats().live_leases, 2);
    assert_eq!(
        lease.artifact(&policy("B")).unwrap_err().code,
        "cem.cc.policy_mismatch"
    );
    let mut changed = a.clone();
    changed.policy_stamps = policy("B");
    assert_eq!(
        store
            .insert(changed.clone(), sidecar.clone())
            .unwrap_err()
            .code,
        "cem.cc.cache_pinned"
    );
    drop(lease);
    assert_eq!(
        store
            .insert(changed.clone(), sidecar.clone())
            .unwrap_err()
            .code,
        "cem.cc.cache_pinned"
    );
    drop(copy);
    store.insert(changed, sidecar).unwrap();
    assert!(store.get(&a.key, &policy("B")).is_ok());
    assert_eq!(store.stats().retained_entries, 1);
}
#[test]
fn runtime_replacement_invalidates_reads_but_keeps_live_retired_bytes_charged() {
    let (a, sidecar) = entry("A");
    let mut store = cache(1, 100_000);
    store.insert(a.clone(), sidecar.clone()).unwrap();
    let lease = store.lease(&a.key, &policy("A")).unwrap();
    let size = store.stats().retained_bytes;
    store.reset_runtime().unwrap();
    assert_eq!(store.stats().runtime_generation, 2);
    assert_eq!(store.stats().resident_entries, 0);
    assert_eq!(store.stats().retained_entries, 1);
    assert_eq!(store.stats().retained_bytes, size);
    assert_eq!(
        lease.artifact(&policy("A")).unwrap_err().code,
        "cem.cc.runtime_replaced"
    );
    assert_eq!(
        lease.sidecar(&policy("A")).unwrap_err().code,
        "cem.cc.runtime_replaced"
    );
    assert_eq!(
        store.insert(a.clone(), sidecar.clone()).unwrap_err().code,
        "cem.cc.capacity"
    );
    drop(lease);
    assert_eq!(store.stats().retained_entries, 0);
    assert_eq!(store.stats().retained_bytes, 0);
    store.insert(a.clone(), sidecar).unwrap();
    let fresh = store.lease(&a.key, &policy("A")).unwrap();
    assert!(fresh.artifact(&policy("A")).is_ok());
    drop(store);
    assert_eq!(
        fresh.artifact(&policy("A")).unwrap_err().code,
        "cem.cc.runtime_replaced"
    );
}
#[test]
fn corrupt_artifact_or_sidecar_never_replaces_existing_owners() {
    let good = entry("A");
    let mut store = cache(1, 100_000);
    store.insert(good.0.clone(), good.1.clone()).unwrap();
    let mut corrupt = entry("B");
    corrupt.0.bytes[0] ^= 1;
    assert_eq!(
        store.insert(corrupt.0, corrupt.1).unwrap_err().code,
        "cem.cc.hash_mismatch"
    );
    let mut corrupt = entry("B");
    corrupt.1.as_mut().unwrap().source_map.frames.clear();
    assert_eq!(
        store.insert(corrupt.0, corrupt.1).unwrap_err().code,
        "cem.cc.sidecar_mismatch"
    );
    assert_eq!(
        store.insert(good.0.clone(), None).unwrap_err().code,
        "cem.cc.sidecar_mismatch"
    );
    assert!(store.get(&good.0.key, &policy("A")).is_ok());
    assert_eq!(store.stats().evictions, 0);
}
#[test]
fn compaction_drops_spare_source_buffer_capacity_and_invalid_limits_fail() {
    assert!(ContentCache::with_limits(ContentCacheLimits {
        max_entries: 0,
        max_bytes: 100
    })
    .is_err());
    assert!(ContentCache::with_limits(ContentCacheLimits {
        max_entries: 1,
        max_bytes: 0
    })
    .is_err());
    let (mut a, mut sidecar) = entry("small");
    let mut bytes = Vec::with_capacity(1_000_000);
    bytes.extend_from_slice(&a.bytes);
    a.bytes = bytes;
    let large_string = |text: &str| {
        let mut value = String::with_capacity(1_000_000);
        value.push_str(text);
        value
    };
    a.policy_stamps.scope_policy_fingerprint = large_string("A");
    sidecar.as_mut().unwrap().source_map.frames[0].transform = TransformKind::SourceBoundary {
        uri: large_string("studio://project/source.cem"),
    };
    let key = a.key.clone();
    assert_eq!(a.bytes.capacity(), 1_000_000);
    let mut store = ContentCache::new();
    // Move the original buffers: cloning would compact them before admission.
    store.insert(a, sidecar).unwrap();
    let lease = store.lease(&key, &policy("A")).unwrap();
    assert_eq!(lease.artifact(&policy("A")).unwrap().bytes.capacity(), 5);
    assert!(
        lease
            .artifact(&policy("A"))
            .unwrap()
            .policy_stamps
            .scope_policy_fingerprint
            .capacity()
            < 16
    );
    let uri = &lease
        .sidecar(&policy("A"))
        .unwrap()
        .unwrap()
        .source_map
        .frames[0]
        .transform;
    assert!(matches!(uri, TransformKind::SourceBoundary { uri } if uri.capacity() < 128));
    assert!(store.stats().retained_bytes < 10_000);
}

#[test]
fn cross_thread_retired_lease_release_reclaims_its_original_charge() {
    let (a, sidecar) = entry("A");
    let mut store = cache(1, 100_000);
    store.insert(a.clone(), sidecar).unwrap();
    let lease = store.lease(&a.key, &policy("A")).unwrap();
    let (start_tx, start_rx) = std::sync::mpsc::channel();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (finish_tx, finish_rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        start_rx.recv().unwrap();
        assert_eq!(
            lease.artifact(&policy("A")).unwrap_err().code,
            "cem.cc.runtime_replaced"
        );
        ready_tx.send(()).unwrap();
        finish_rx.recv().unwrap();
        drop(lease);
    });
    store.reset_runtime().unwrap();
    start_tx.send(()).unwrap();
    ready_rx.recv().unwrap();
    assert_eq!(store.stats().retained_entries, 1);
    assert_eq!(store.stats().live_leases, 1);
    finish_tx.send(()).unwrap();
    worker.join().unwrap();
    assert_eq!(store.stats().retained_bytes, 0);
    assert_eq!(store.stats().live_leases, 0);
}
#[test]
fn metadata_limit_and_prod_sidecar_rejection_are_atomic() {
    let a = entry("A");
    let mut store = cache(1, 10_000_000);
    store.insert(a.0.clone(), a.1.clone()).unwrap();
    let mut oversized = entry("B");
    oversized.0.policy_stamps.scope_policy_fingerprint = "\0".repeat(200_000);
    assert_eq!(
        store.insert(oversized.0, oversized.1).unwrap_err().code,
        "cem.cc.capacity"
    );
    let mut invalid = entry("B");
    invalid.0.key.mode = CacheMode::Prod;
    assert_eq!(
        store.insert(invalid.0, invalid.1).unwrap_err().code,
        "cem.cc.sidecar_mismatch"
    );
    let mut invalid = entry("B");
    invalid.0.format_version = "future-format".into();
    assert_eq!(
        store.insert(invalid.0, invalid.1).unwrap_err().code,
        "cem.cc.format_version"
    );
    assert_eq!(store.stats().resident_entries, 1);
    assert_eq!(store.stats().evictions, 0);
    assert!(store.get(&a.0.key, &policy("A")).is_ok());
    assert_eq!(store.stats().rejections, 3);
}

#[test]
fn late_old_runtime_admission_and_lookup_cannot_bind_a_new_owner() {
    let (a, sidecar) = entry("A");
    let mut store = cache(1, 100_000);
    let captured = store.runtime();
    store.reset_runtime().unwrap();
    assert_eq!(
        store
            .insert_for_runtime(&captured, a.clone(), sidecar.clone())
            .unwrap_err()
            .code,
        "cem.cc.runtime_replaced"
    );
    assert_eq!(store.stats().retained_entries, 0);
    assert_eq!(store.stats().retained_bytes, 0);
    let current = store.runtime();
    store
        .insert_for_runtime(&current, a.clone(), sidecar)
        .unwrap();
    assert_eq!(
        store
            .lease_for_runtime(&captured, &a.key, &policy("A"))
            .unwrap_err()
            .code,
        "cem.cc.runtime_replaced"
    );
    assert!(store
        .lease_for_runtime(&current, &a.key, &policy("A"))
        .unwrap()
        .artifact(&policy("A"))
        .is_ok());
    assert_eq!(store.stats().resident_entries, 1);
}

#[test]
fn different_cache_owners_cannot_accept_another_runtime_ticket_at_equal_generations() {
    let old = cache(1, 100_000);
    let ticket = old.runtime();
    let (a, sidecar) = entry("A");
    let mut other = cache(1, 100_000);
    assert_eq!(
        old.stats().runtime_generation,
        other.stats().runtime_generation
    );
    assert_eq!(
        other
            .insert_for_runtime(&ticket, a.clone(), sidecar.clone())
            .unwrap_err()
            .code,
        "cem.cc.runtime_replaced"
    );
    other.insert(a.clone(), sidecar.clone()).unwrap();
    assert_eq!(
        other
            .lease_for_runtime(&ticket, &a.key, &policy("A"))
            .unwrap_err()
            .code,
        "cem.cc.runtime_replaced"
    );
    drop(old); // A ticket alone must not keep that cache owner alive or rebind.
    assert_eq!(
        other
            .insert_for_runtime(&ticket, a.clone(), sidecar)
            .unwrap_err()
            .code,
        "cem.cc.runtime_replaced"
    );
    assert_eq!(other.stats().resident_entries, 1);
    assert_eq!(other.stats().live_leases, 0);
}
