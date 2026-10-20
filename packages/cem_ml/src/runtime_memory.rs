//! Per-runtime admission accounting. Allocator requested bytes and WASM capacity
//! are distinct observations, never a claim about browser/process RSS. The
//! allocator only measures: returning null at a configured limit would trap.
use serde::{Deserialize, Serialize};
use std::alloc::{GlobalAlloc, Layout};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex, OnceLock, Weak,
};

pub struct TrackingAllocator<A> {
    inner: A,
    live: AtomicUsize,
    peak: AtomicUsize,
}
impl<A> TrackingAllocator<A> {
    pub const fn new(inner: A) -> Self {
        Self {
            inner,
            live: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
        }
    }
    fn allocated(&self, bytes: usize) {
        let live = self
            .live
            .fetch_add(bytes, Ordering::Relaxed)
            .wrapping_add(bytes);
        self.peak.fetch_max(live, Ordering::Relaxed);
    }
    pub fn sample(&self, linear_memory_bytes: Option<u64>) -> MemorySample {
        MemorySample {
            allocator_live_bytes: self.live.load(Ordering::Relaxed) as u64,
            allocator_peak_bytes: self.peak.load(Ordering::Relaxed) as u64,
            linear_memory_bytes,
        }
    }
}
// No locks, allocations, diagnostics or panics in allocator callbacks.
unsafe impl<A: GlobalAlloc> GlobalAlloc for TrackingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { self.inner.alloc(layout) };
        if !p.is_null() {
            self.allocated(layout.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { self.inner.alloc_zeroed(layout) };
        if !p.is_null() {
            self.allocated(layout.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { self.inner.dealloc(p, layout) };
        self.live.fetch_sub(layout.size(), Ordering::Relaxed);
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let next = unsafe { self.inner.realloc(p, layout, new_size) };
        if !next.is_null() {
            if new_size >= layout.size() {
                self.allocated(new_size - layout.size());
            } else {
                self.live
                    .fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
        }
        next
    }
}
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: TrackingAllocator<std::alloc::System> =
    TrackingAllocator::new(std::alloc::System);
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemorySample {
    pub allocator_live_bytes: u64,
    pub allocator_peak_bytes: u64,
    pub linear_memory_bytes: Option<u64>,
}
impl MemorySample {
    pub fn observed_bytes(self) -> u64 {
        self.allocator_live_bytes
            .max(self.linear_memory_bytes.unwrap_or(0))
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryBudgetConfig {
    pub soft_limit_bytes: u64,
    pub hard_limit_bytes: u64,
    pub recovery_headroom_bytes: u64,
    pub max_concurrent_operations: u32,
}
impl Default for MemoryBudgetConfig {
    fn default() -> Self {
        Self {
            soft_limit_bytes: 768 * 1024 * 1024,
            hard_limit_bytes: 1024 * 1024 * 1024,
            recovery_headroom_bytes: 64 * 1024 * 1024,
            max_concurrent_operations: 64,
        }
    }
}
impl MemoryBudgetConfig {
    pub fn work_limit(self) -> u64 {
        self.hard_limit_bytes
            .saturating_sub(self.recovery_headroom_bytes)
    }
    fn validate(self) -> Result<Self, MemoryBudgetError> {
        if self.recovery_headroom_bytes == 0
            || self.recovery_headroom_bytes >= self.hard_limit_bytes
            || self.hard_limit_bytes > 4 * 1024 * 1024 * 1024
            || self.soft_limit_bytes == 0
            || self.soft_limit_bytes > self.work_limit()
            || !(1..=64).contains(&self.max_concurrent_operations)
        {
            Err(MemoryBudgetError("cem.runtime.memory.config"))
        } else {
            Ok(self)
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryBudgetError(pub &'static str);
impl std::fmt::Display for MemoryBudgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for MemoryBudgetError {}
struct State {
    config: MemoryBudgetConfig,
    reserved: u64,
    active: u32,
    recovering: bool,
}
#[derive(Clone)]
pub struct RuntimeMemoryBudget {
    state: Arc<Mutex<State>>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryBudgetStatus {
    pub config: MemoryBudgetConfig,
    pub sample: MemorySample,
    pub reserved_bytes: u64,
    pub active_operations: u32,
    pub soft_pressure: bool,
    pub hard_pressure: bool,
}
pub struct RuntimeMemoryPermit {
    state: Weak<Mutex<State>>,
    bytes: u64,
    recovery: bool,
}
impl Drop for RuntimeMemoryPermit {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            let mut state = state.lock().expect("memory budget lock");
            state.reserved -= self.bytes;
            state.active -= 1;
            if self.recovery {
                state.recovering = false;
            }
        }
    }
}
impl RuntimeMemoryBudget {
    pub fn new(config: MemoryBudgetConfig) -> Result<Self, MemoryBudgetError> {
        Ok(Self {
            state: Arc::new(Mutex::new(State {
                config: config.validate()?,
                reserved: 0,
                active: 0,
                recovering: false,
            })),
        })
    }
    pub fn configure(&self, config: MemoryBudgetConfig) -> Result<(), MemoryBudgetError> {
        let config = config.validate()?;
        let mut state = self.state.lock().expect("memory budget lock");
        if state.active != 0 {
            return Err(MemoryBudgetError("cem.runtime.memory.busy"));
        }
        state.config = config;
        Ok(())
    }
    pub fn status(&self, sample: MemorySample) -> MemoryBudgetStatus {
        let state = self.state.lock().expect("memory budget lock");
        MemoryBudgetStatus {
            config: state.config,
            sample,
            reserved_bytes: state.reserved,
            active_operations: state.active,
            soft_pressure: sample.observed_bytes().saturating_add(state.reserved)
                >= state.config.soft_limit_bytes,
            hard_pressure: sample.observed_bytes() >= state.config.work_limit(),
        }
    }
    /// Reserve conservative workspace before admitting work. Reservations are
    /// deliberately not subtracted from observed allocations of other work.
    pub fn admit(
        &self,
        sample: MemorySample,
        bytes: u64,
    ) -> Result<RuntimeMemoryPermit, MemoryBudgetError> {
        let mut state = self.state.lock().expect("memory budget lock");
        let reserved = state
            .reserved
            .checked_add(bytes)
            .ok_or(MemoryBudgetError("cem.runtime.memory.admission"))?;
        if bytes == 0
            || state.recovering
            || sample.observed_bytes() >= state.config.soft_limit_bytes
            || sample
                .observed_bytes()
                .checked_add(reserved)
                .is_none_or(|total| {
                    total > state.config.soft_limit_bytes.min(state.config.work_limit())
                })
            || state.active >= state.config.max_concurrent_operations
        {
            return Err(MemoryBudgetError("cem.runtime.memory.admission"));
        }
        state.reserved = reserved;
        state.active += 1;
        Ok(RuntimeMemoryPermit {
            state: Arc::downgrade(&self.state),
            bytes,
            recovery: false,
        })
    }
    /// Exclusive bounded transport work after admissions have drained. This
    /// permission does not disable normal operation-control safe points.
    pub fn admit_recovery(
        &self,
        sample: MemorySample,
        bytes: u64,
    ) -> Result<RuntimeMemoryPermit, MemoryBudgetError> {
        let mut state = self.state.lock().expect("memory budget lock");
        if state.active != 0
            || bytes == 0
            || bytes > state.config.recovery_headroom_bytes
            || sample
                .observed_bytes()
                .checked_add(bytes)
                .is_none_or(|total| total > state.config.hard_limit_bytes)
        {
            return Err(MemoryBudgetError("cem.runtime.memory.recovery"));
        }
        state.reserved = bytes;
        state.active = 1;
        state.recovering = true;
        Ok(RuntimeMemoryPermit {
            state: Arc::downgrade(&self.state),
            bytes,
            recovery: true,
        })
    }
}
pub fn runtime_budget() -> &'static RuntimeMemoryBudget {
    static BUDGET: OnceLock<RuntimeMemoryBudget> = OnceLock::new();
    BUDGET
        .get_or_init(|| RuntimeMemoryBudget::new(Default::default()).expect("valid runtime budget"))
}
#[cfg(target_arch = "wasm32")]
pub fn runtime_sample() -> MemorySample {
    ALLOCATOR.sample(Some(core::arch::wasm32::memory_size::<0>() as u64 * 65536))
}
/// A bounded estimate for entrypoint workspace, not an allocator ceiling.
#[cfg(target_arch = "wasm32")]
pub fn admit_input(bytes: usize) -> Result<RuntimeMemoryPermit, MemoryBudgetError> {
    let work = (bytes as u64)
        .checked_mul(8)
        .and_then(|n| n.checked_add(64 * 1024))
        .ok_or(MemoryBudgetError("cem.runtime.memory.admission"))?;
    runtime_budget().admit(runtime_sample(), work)
}
