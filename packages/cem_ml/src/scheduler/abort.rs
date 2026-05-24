//! Cooperative cancellation primitive shared by the scheduler and the
//! plugin runtime (AC-A-7, AC-PL-19).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::source_map::SourceMapStack;

/// Cheap clone-shareable cancellation flag. Producers `abort()` the
/// signal; consumers poll `is_aborted()` between work chunks.
#[derive(Debug, Clone, Default)]
pub struct AbortSignal {
    flag: Arc<AtomicBool>,
    source_map: Arc<Mutex<Option<SourceMapStack>>>,
}

impl AbortSignal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn abort(&self) {
        self.flag.store(true, Ordering::Release);
    }

    pub fn abort_with_source_map(&self, source_map: SourceMapStack) {
        if let Ok(mut slot) = self.source_map.lock() {
            *slot = Some(source_map);
        }
        self.abort();
    }

    pub fn is_aborted(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }

    pub fn source_map(&self) -> Option<SourceMapStack> {
        self.source_map
            .lock()
            .expect("poisoned abort source-map mutex")
            .clone()
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
