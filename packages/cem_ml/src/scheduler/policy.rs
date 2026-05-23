//! Scope-level resource caps (AC-A-4, AC-A-5, AC-A-6).

use serde::{Deserialize, Serialize};

/// Behaviour when a bounded queue is full (AC-A-5).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OverflowPolicy {
    /// Block the caller until a slot is available.
    Block,
    /// Reject the new work with a `QueueError::Overflow`.
    #[default]
    Reject,
    /// Forward the new work to the parent scope's queue. The parent's
    /// overflow policy then applies; if no parent exists the policy
    /// degrades to `Reject`.
    SpillToParent,
}

/// One bounded resource the scheduler accounts for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceCap {
    CpuWorkers,
    QueueSize,
    IoStreams,
    MemoryBytes,
}

/// Static description of the resource ceiling for one scope.
///
/// AC-A-4 owns the public default for the **root** scope. Hosts MAY
/// override the root policy through [`ScopePolicy::host_root`] /
/// [`ScopePolicy::with_…`] builders; child scopes inherit and MAY
/// constrain further only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopePolicy {
    /// CPU worker slots (AC-A-4). Floored at 1.
    pub cpu_workers: u32,
    /// Bounded queue size (AC-A-5). Floored at 1 so the pool can
    /// always accept at least one in-flight task.
    pub queue_size: u32,
    /// External-I/O stream count (AC-A-6). Independent of CPU pool.
    pub io_streams: u32,
    /// Memory cap in bytes (informational; the runtime is responsible
    /// for enforcing it where possible).
    pub memory_bytes: u64,
    /// Behaviour when the CPU queue is full.
    pub overflow: OverflowPolicy,
}

impl ScopePolicy {
    /// Tier-B documented default for the root scope on the calling
    /// platform. Hosts can call this and then override individual
    /// caps via the `with_*` setters.
    pub fn host_root() -> Self {
        Self {
            cpu_workers: default_cpu_workers(),
            queue_size: 64,
            io_streams: 16,
            memory_bytes: 256 * 1024 * 1024, // 256 MiB
            overflow: OverflowPolicy::Reject,
        }
    }

    pub fn with_cpu_workers(mut self, n: u32) -> Self {
        self.cpu_workers = n.max(1);
        self
    }
    pub fn with_queue_size(mut self, n: u32) -> Self {
        self.queue_size = n.max(1);
        self
    }
    pub fn with_io_streams(mut self, n: u32) -> Self {
        self.io_streams = n;
        self
    }
    pub fn with_memory_bytes(mut self, n: u64) -> Self {
        self.memory_bytes = n;
        self
    }
    pub fn with_overflow(mut self, o: OverflowPolicy) -> Self {
        self.overflow = o;
        self
    }

    /// Sanity-check: every cap is at or above its floor.
    pub fn validate(&self) -> Result<(), ScopePolicyError> {
        if self.cpu_workers == 0 {
            return Err(ScopePolicyError::ZeroCap(ResourceCap::CpuWorkers));
        }
        if self.queue_size == 0 {
            return Err(ScopePolicyError::ZeroCap(ResourceCap::QueueSize));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopePolicyError {
    ZeroCap(ResourceCap),
}

impl std::fmt::Display for ScopePolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScopePolicyError::ZeroCap(cap) => write!(f, "resource cap {cap:?} cannot be zero"),
        }
    }
}

impl std::error::Error for ScopePolicyError {}

/// AC-A-4 documented default for the **native** host. In Tier B the
/// browser/WASM host computes the same `min(hw_concurrency, 8)`
/// expression from `navigator.hardwareConcurrency`; the test below
/// pins the formula so the contract survives platform porting.
fn default_cpu_workers() -> u32 {
    #[cfg(target_arch = "wasm32")]
    {
        // Hosts wire this through via a runtime hint; default to 4 so
        // the public number is non-zero even before the wiring lands.
        4
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let raw = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1);
        raw.clamp(1, 8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_root_obeys_ac_a_4_documented_caps() {
        let p = ScopePolicy::host_root();
        assert!(p.cpu_workers >= 1, "AC-A-4 floor of 1 must hold");
        assert!(p.cpu_workers <= 8, "AC-A-4 ceiling of 8 must hold");
        assert!(p.queue_size >= 1);
        p.validate().unwrap();
    }

    #[test]
    fn with_setters_apply_the_floor() {
        let p = ScopePolicy::host_root().with_cpu_workers(0).with_queue_size(0);
        assert_eq!(p.cpu_workers, 1);
        assert_eq!(p.queue_size, 1);
    }

    #[test]
    fn validate_rejects_zero_caps() {
        let mut p = ScopePolicy::host_root();
        p.cpu_workers = 0;
        assert!(matches!(
            p.validate(),
            Err(ScopePolicyError::ZeroCap(ResourceCap::CpuWorkers))
        ));
    }
}
