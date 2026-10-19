//! Bounded ownership for the existing native artifact store. No executable
//! callbacks, trees, host handles or process-global sidecar inventory are cached.
use super::{
    CacheArtifact, CacheError, CacheKey, CacheMode, ContentHash, PolicyStamps, SourceMapSidecar,
    FORMAT_VERSION,
};
use serde::ser::Serialize;
use std::{
    collections::{BTreeMap, VecDeque},
    io::Write,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex, Weak,
    },
};

const METADATA_LIMIT: usize = 1024 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentCacheLimits {
    pub max_entries: usize,
    pub max_bytes: usize,
}
impl Default for ContentCacheLimits {
    fn default() -> Self {
        Self {
            max_entries: 64,
            max_bytes: 32 * 1024 * 1024,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentCacheStats {
    pub resident_entries: usize,
    pub retained_entries: usize,
    pub retained_bytes: usize,
    pub live_leases: usize,
    pub runtime_generation: u64,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub rejections: u64,
    pub invalidations: u64,
}
#[derive(Debug)]
struct Runtime {
    open: AtomicBool,
    generation: AtomicU64,
    entries: AtomicUsize,
    bytes: AtomicUsize,
    leases: AtomicUsize,
}
#[derive(Debug)]
struct Entry {
    artifact: CacheArtifact,
    sidecar: Option<SourceMapSidecar>,
    charge: usize,
    generation: u64,
    runtime: Arc<Runtime>,
}
impl Drop for Entry {
    fn drop(&mut self) {
        self.runtime.entries.fetch_sub(1, Ordering::Relaxed);
        self.runtime.bytes.fetch_sub(self.charge, Ordering::Relaxed);
    }
}
/// Opaque, nonportable admission ticket captured before native work. A weak
/// owner identity prevents equal generation counters in different stores from
/// aliasing, without retaining the store, payloads or retired native owners.
#[derive(Debug, Clone)]
pub struct ContentCacheRuntime {
    owner: Weak<Runtime>,
    generation: u64,
}

/// A read-only native artifact owner. Clone shares one charge; drop releases its
/// pin. Access checks runtime liveness and the caller's current policy each time.
#[derive(Debug)]
pub struct ContentCacheLease {
    entry: Arc<Entry>,
}
impl ContentCacheLease {
    fn new(entry: Arc<Entry>) -> Self {
        entry.runtime.leases.fetch_add(1, Ordering::Relaxed);
        Self { entry }
    }
    fn active(&self, policy: &PolicyStamps) -> Result<(), CacheError> {
        if !self.entry.runtime.open.load(Ordering::Acquire)
            || self.entry.generation != self.entry.runtime.generation.load(Ordering::Acquire)
        {
            return Err(error(
                "cem.cc.runtime_replaced",
                "the native cache owner was replaced or disposed",
            ));
        }
        self.entry.artifact.validate_policy(policy)
    }
    pub fn artifact(&self, policy: &PolicyStamps) -> Result<&CacheArtifact, CacheError> {
        self.active(policy)?;
        Ok(&self.entry.artifact)
    }
    pub fn sidecar(&self, policy: &PolicyStamps) -> Result<Option<&SourceMapSidecar>, CacheError> {
        self.active(policy)?;
        Ok(self.entry.sidecar.as_ref())
    }
}
impl Clone for ContentCacheLease {
    fn clone(&self) -> Self {
        Self::new(self.entry.clone())
    }
}
impl Drop for ContentCacheLease {
    fn drop(&mut self) {
        self.entry.runtime.leases.fetch_sub(1, Ordering::Relaxed);
    }
}

#[derive(Debug)]
pub struct ContentCache {
    artifacts: BTreeMap<CacheKey, Arc<Entry>>,
    order: Mutex<VecDeque<CacheKey>>,
    limits: ContentCacheLimits,
    runtime: Arc<Runtime>,
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: u64,
    rejections: u64,
    invalidations: u64,
}
impl Default for ContentCache {
    fn default() -> Self {
        Self::with_limits(ContentCacheLimits::default()).expect("valid native cache defaults")
    }
}
impl Drop for ContentCache {
    fn drop(&mut self) {
        self.runtime.open.store(false, Ordering::Release);
    }
}
fn error(code: &'static str, message: &'static str) -> CacheError {
    CacheError {
        code,
        message: message.into(),
    }
}
fn increment(counter: &AtomicU64) {
    let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
        Some(v.saturating_add(1))
    });
}
impl ContentCache {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_limits(limits: ContentCacheLimits) -> Result<Self, CacheError> {
        if limits.max_entries == 0 || limits.max_bytes == 0 {
            return Err(error(
                "cem.cc.invalid_limits",
                "native cache count and byte limits must be positive",
            ));
        }
        Ok(Self {
            artifacts: BTreeMap::new(),
            order: Mutex::new(VecDeque::new()),
            limits,
            runtime: Arc::new(Runtime {
                open: AtomicBool::new(true),
                generation: AtomicU64::new(1),
                entries: AtomicUsize::new(0),
                bytes: AtomicUsize::new(0),
                leases: AtomicUsize::new(0),
            }),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: 0,
            rejections: 0,
            invalidations: 0,
        })
    }
    pub fn stats(&self) -> ContentCacheStats {
        ContentCacheStats {
            resident_entries: self.artifacts.len(),
            retained_entries: self.runtime.entries.load(Ordering::Relaxed),
            retained_bytes: self.runtime.bytes.load(Ordering::Relaxed),
            live_leases: self.runtime.leases.load(Ordering::Relaxed),
            runtime_generation: self.runtime.generation.load(Ordering::Acquire),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions,
            rejections: self.rejections,
            invalidations: self.invalidations,
        }
    }
    pub fn runtime(&self) -> ContentCacheRuntime {
        ContentCacheRuntime {
            owner: Arc::downgrade(&self.runtime),
            generation: self.runtime.generation.load(Ordering::Acquire),
        }
    }
    fn accepts_runtime(&self, ticket: &ContentCacheRuntime) -> bool {
        Weak::ptr_eq(&ticket.owner, &Arc::downgrade(&self.runtime))
            && ticket.generation == self.runtime.generation.load(Ordering::Acquire)
            && self.runtime.open.load(Ordering::Acquire)
    }
    fn touch(&self, key: &CacheKey) {
        let mut order = self.order.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(index) = order.iter().position(|item| item == key) {
            let key = order.remove(index).expect("known recency entry");
            order.push_back(key);
        }
    }
    fn read(&self, key: &CacheKey, policy: &PolicyStamps) -> Result<&Arc<Entry>, CacheError> {
        let result = self
            .artifacts
            .get(key)
            .ok_or_else(|| error("cem.cc.cache_evicted", "cache artifact is not present"))
            .and_then(|entry| {
                entry.artifact.validate_policy(policy)?;
                Ok(entry)
            });
        if result.is_ok() {
            increment(&self.hits);
            self.touch(key);
        } else {
            increment(&self.misses);
        }
        result
    }
    /// Borrowed compatibility access cannot outlive a mutable store operation.
    pub fn get(&self, key: &CacheKey, policy: &PolicyStamps) -> Result<&CacheArtifact, CacheError> {
        self.read(key, policy).map(|entry| &entry.artifact)
    }
    pub fn lease(
        &self,
        key: &CacheKey,
        policy: &PolicyStamps,
    ) -> Result<ContentCacheLease, CacheError> {
        self.lease_for_runtime(&self.runtime(), key, policy)
    }
    /// Use the owner ticket captured before starting native/worker work. Matching
    /// portable bytes in a new runtime do not revive an old lookup request.
    pub fn lease_for_runtime(
        &self,
        ticket: &ContentCacheRuntime,
        key: &CacheKey,
        policy: &PolicyStamps,
    ) -> Result<ContentCacheLease, CacheError> {
        if !self.accepts_runtime(ticket) {
            increment(&self.misses);
            return Err(error(
                "cem.cc.runtime_replaced",
                "the cache request belongs to a replaced native runtime",
            ));
        }
        self.read(key, policy)
            .map(|entry| ContentCacheLease::new(entry.clone()))
    }
    /// Sidecars exist only while their owning resident artifact exists. Prefer a
    /// policy-checked lease for an owner that must survive a mutable operation.
    pub fn sidecar(&self, hash: &ContentHash) -> Option<&SourceMapSidecar> {
        self.artifacts
            .values()
            .filter_map(|entry| entry.sidecar.as_ref())
            .find(|sidecar| &sidecar.hash == hash)
    }
    /// Retired pinned allocations remain charged until their last lease drops.
    pub fn reset_runtime(&mut self) -> Result<(), CacheError> {
        let generation = self
            .runtime
            .generation
            .load(Ordering::Acquire)
            .checked_add(1)
            .ok_or_else(|| {
                error(
                    "cem.cc.generation_exhausted",
                    "native runtime generation exhausted",
                )
            })?;
        self.runtime.generation.store(generation, Ordering::Release);
        self.artifacts.clear();
        self.order.lock().unwrap_or_else(|e| e.into_inner()).clear();
        self.invalidations = self.invalidations.saturating_add(1);
        Ok(())
    }
    /// Validate and plan admission before evicting anything. Live leases are
    /// never victims. Backpressure preserves the prior store on any rejection.
    pub fn insert(
        &mut self,
        artifact: CacheArtifact,
        sidecar: Option<SourceMapSidecar>,
    ) -> Result<(), CacheError> {
        let result = self.admit(artifact, sidecar);
        if result.is_err() {
            self.rejections = self.rejections.saturating_add(1);
        }
        result
    }
    /// Async producers must capture the owner ticket before work and use this
    /// fenced admission. Rejection happens before validation, eviction or charge.
    pub fn insert_for_runtime(
        &mut self,
        ticket: &ContentCacheRuntime,
        artifact: CacheArtifact,
        sidecar: Option<SourceMapSidecar>,
    ) -> Result<(), CacheError> {
        if !self.accepts_runtime(ticket) {
            self.rejections = self.rejections.saturating_add(1);
            return Err(error(
                "cem.cc.runtime_replaced",
                "the cache result belongs to a replaced native runtime",
            ));
        }
        self.insert(artifact, sidecar)
    }
    fn admit(
        &mut self,
        artifact: CacheArtifact,
        sidecar: Option<SourceMapSidecar>,
    ) -> Result<(), CacheError> {
        let prepared = prepare(artifact, sidecar, self.limits.max_bytes)?;
        let key = &prepared.artifact.key;
        let mut victims = Vec::new();
        let mut bytes = self.runtime.bytes.load(Ordering::Relaxed);
        let mut entries = self.runtime.entries.load(Ordering::Relaxed);
        if let Some(existing) = self.artifacts.get(key) {
            if existing.artifact == prepared.artifact && existing.sidecar == prepared.sidecar {
                self.touch(key);
                return Ok(());
            }
            if Arc::strong_count(existing) > 1 {
                return Err(error(
                    "cem.cc.cache_pinned",
                    "a live native lease prevents replacement",
                ));
            }
            bytes -= existing.charge;
            entries -= 1;
            victims.push(key.clone());
        }
        let fits = |entries: usize, bytes: usize| {
            entries < self.limits.max_entries && bytes <= self.limits.max_bytes - prepared.charge
        };
        for candidate in self.order.lock().unwrap_or_else(|e| e.into_inner()).iter() {
            if fits(entries, bytes) {
                break;
            }
            if candidate == key {
                continue;
            }
            let entry = &self.artifacts[candidate];
            if Arc::strong_count(entry) == 1 {
                entries -= 1;
                bytes -= entry.charge;
                victims.push(candidate.clone());
            }
        }
        if !fits(entries, bytes) {
            return Err(error(
                "cem.cc.capacity",
                "live native owners leave insufficient cache capacity",
            ));
        }
        let mut order = self.order.lock().unwrap_or_else(|e| e.into_inner());
        for victim in victims {
            self.artifacts.remove(&victim);
            order.retain(|item| item != &victim);
            self.evictions = self.evictions.saturating_add(1);
        }
        let key = prepared.artifact.key.clone();
        self.runtime.entries.fetch_add(1, Ordering::Relaxed);
        self.runtime
            .bytes
            .fetch_add(prepared.charge, Ordering::Relaxed);
        self.artifacts.insert(
            key.clone(),
            Arc::new(Entry {
                artifact: prepared.artifact,
                sidecar: prepared.sidecar,
                charge: prepared.charge,
                generation: self.runtime.generation.load(Ordering::Acquire),
                runtime: self.runtime.clone(),
            }),
        );
        order.push_back(key);
        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Metadata {
    format_version: String,
    key: CacheKey,
    policy_stamps: PolicyStamps,
    source_map_sidecar_hash: Option<ContentHash>,
}
#[derive(serde::Serialize)]
struct MetadataRef<'a> {
    format_version: &'a str,
    key: &'a CacheKey,
    policy_stamps: &'a PolicyStamps,
    source_map_sidecar_hash: &'a Option<ContentHash>,
}
struct LimitedBytes {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for LimitedBytes {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit - self.bytes.len() {
            return Err(std::io::Error::other("native cache metadata limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn encode(value: &impl Serialize) -> Result<Vec<u8>, CacheError> {
    let mut writer = LimitedBytes {
        bytes: Vec::new(),
        limit: METADATA_LIMIT,
    };
    serde_json::to_writer(&mut writer, value).map_err(|_| {
        error(
            "cem.cc.capacity",
            "native cache metadata exceeds its bounded encoding budget",
        )
    })?;
    Ok(writer.bytes)
}
struct Prepared {
    artifact: CacheArtifact,
    sidecar: Option<SourceMapSidecar>,
    charge: usize,
}
fn prepare(
    mut artifact: CacheArtifact,
    sidecar: Option<SourceMapSidecar>,
    max_bytes: usize,
) -> Result<Prepared, CacheError> {
    if artifact.bytes.len() > max_bytes {
        return Err(error(
            "cem.cc.capacity",
            "artifact exceeds the native cache byte budget",
        ));
    }
    if artifact.format_version != FORMAT_VERSION {
        return Err(error(
            "cem.cc.format_version",
            "unsupported native cache artifact version",
        ));
    }
    if ContentHash::from_blake3(&artifact.bytes) != artifact.key.hash {
        return Err(error(
            "cem.cc.hash_mismatch",
            "artifact bytes do not match the native cache key",
        ));
    }
    match (&artifact.source_map_sidecar_hash, &sidecar) {
        (None, None) => (),
        (Some(expected), Some(sidecar)) if artifact.key.mode == CacheMode::Dev => {
            let bytes = encode(&sidecar.source_map)?;
            if expected != &sidecar.hash || ContentHash::from_blake3(&bytes) != sidecar.hash {
                return Err(error(
                    "cem.cc.sidecar_mismatch",
                    "native source-map sidecar does not match its owner",
                ));
            }
        }
        _ => {
            return Err(error(
                "cem.cc.sidecar_mismatch",
                "native source-map sidecar is absent or incompatible",
            ))
        }
    }
    let metadata = encode(&MetadataRef {
        format_version: &artifact.format_version,
        key: &artifact.key,
        policy_stamps: &artifact.policy_stamps,
        source_map_sidecar_hash: &artifact.source_map_sidecar_hash,
    })?;
    let key_bytes = encode(&artifact.key)?;
    let sidecar_bytes = sidecar.as_ref().map(encode).transpose()?;
    let control_bytes =
        metadata.len() + key_bytes.len() * 2 + sidecar_bytes.as_ref().map_or(0, Vec::len);
    if control_bytes > METADATA_LIMIT {
        return Err(error(
            "cem.cc.capacity",
            "native artifact metadata exceeds 1 MiB",
        ));
    }
    let charge = artifact
        .bytes
        .len()
        .checked_add(control_bytes)
        .and_then(|size| {
            size.checked_add(std::mem::size_of::<Entry>() + std::mem::size_of::<CacheKey>())
        })
        .ok_or_else(|| error("cem.cc.capacity", "native cache charge overflow"))?;
    if charge > max_bytes {
        return Err(error(
            "cem.cc.capacity",
            "complete artifact charge exceeds the native cache byte budget",
        ));
    }
    // Round-trip only bounded metadata, and compact payload capacity, so a tiny
    // value never retains a large former source/allocation buffer in the store.
    let metadata: Metadata = serde_json::from_slice(&metadata)
        .map_err(|_| error("cem.cc.format_version", "invalid native metadata"))?;
    artifact = CacheArtifact {
        format_version: metadata.format_version,
        key: metadata.key,
        policy_stamps: metadata.policy_stamps,
        source_map_sidecar_hash: metadata.source_map_sidecar_hash,
        bytes: artifact.bytes.into_boxed_slice().into_vec(),
    };
    let sidecar = sidecar_bytes
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()
        .map_err(|_| error("cem.cc.sidecar_mismatch", "invalid native sidecar"))?;
    Ok(Prepared {
        artifact,
        sidecar,
        charge,
    })
}
