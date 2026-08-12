//! Cooperative cancellation primitive shared by the scheduler and the
//! plugin runtime (AC-A-7, AC-PL-19).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::source_map::SourceMapStack;

/// Cheap clone-shareable cancellation flag. Producers `abort()` the
/// signal; consumers poll `is_aborted()` between work chunks.
#[derive(Debug, Clone, Default)]
pub struct AbortSignal {
    state: Arc<AbortState>,
}

#[derive(Debug, Default)]
struct AbortState {
    flag: AtomicBool,
    metadata: Mutex<Option<AbortMetadata>>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct AbortMetadata {
    pub(crate) reason: Option<String>,
    pub(crate) source_map: Option<SourceMapStack>,
}

impl AbortSignal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn abort(&self) {
        self.abort_with_metadata(None, None);
    }

    pub fn abort_with_source_map(&self, source_map: SourceMapStack) {
        self.abort_with_metadata(None, Some(source_map));
    }

    pub fn is_aborted(&self) -> bool {
        self.state.flag.load(Ordering::Acquire)
    }

    pub fn source_map(&self) -> Option<SourceMapStack> {
        self.metadata().and_then(|metadata| metadata.source_map)
    }

    pub(crate) fn reason(&self) -> Option<String> {
        self.metadata().and_then(|metadata| metadata.reason)
    }

    pub(crate) fn abort_with_metadata(
        &self,
        reason: Option<String>,
        source_map: Option<SourceMapStack>,
    ) -> bool {
        let mut metadata = self
            .state
            .metadata
            .lock()
            .expect("poisoned abort metadata mutex");
        if metadata.is_some() {
            return false;
        }
        *metadata = Some(AbortMetadata { reason, source_map });
        self.state.flag.store(true, Ordering::Release);
        true
    }

    fn metadata(&self) -> Option<AbortMetadata> {
        self.state
            .metadata
            .lock()
            .expect("poisoned abort metadata mutex")
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

    #[test]
    fn first_abort_metadata_wins() {
        let signal = AbortSignal::new();
        assert!(signal.abort_with_metadata(Some("first".to_owned()), None));
        assert!(!signal.abort_with_metadata(Some("second".to_owned()), None));
        assert_eq!(signal.reason().as_deref(), Some("first"));
    }
}
