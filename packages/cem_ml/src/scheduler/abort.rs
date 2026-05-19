//! Cooperative cancellation primitive shared by the scheduler and the
//! plugin runtime (AC-A-7, AC-PL-19).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Cheap clone-shareable cancellation flag. Producers `abort()` the
/// signal; consumers poll `is_aborted()` between work chunks.
#[derive(Debug, Clone, Default)]
pub struct AbortSignal {
    flag: Arc<AtomicBool>,
}

impl AbortSignal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn abort(&self) {
        self.flag.store(true, Ordering::Release);
    }

    pub fn is_aborted(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abort_signal_is_visible_through_clones() {
        let a = AbortSignal::new();
        let b = a.clone();
        assert!(!b.is_aborted());
        a.abort();
        assert!(b.is_aborted(), "abort must propagate to clones");
    }
}
