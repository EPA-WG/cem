//! Deterministic scheduler trace (AC-O-2).
//!
//! The trace records every scheduler decision (enqueue, dispatch,
//! finish, abort, overflow) with a monotonic sequence number. Two
//! runs of an identical input MUST emit identical traces; the unit
//! tests below pin that contract.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SchedulerEventKind {
    Enqueue,
    Dispatch,
    Finish,
    Abort,
    Overflow,
    /// External I/O queue acquired a slot.
    IoAcquire,
    /// External I/O queue released its slot.
    IoRelease,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchedulerEvent {
    pub sequence: u64,
    pub scope: u32,
    pub kind: SchedulerEventKind,
    pub task: String,
}

#[derive(Debug, Clone, Default)]
pub struct SchedulerTrace {
    events: Arc<Mutex<Vec<SchedulerEvent>>>,
    next_seq: Arc<Mutex<u64>>,
}

impl SchedulerTrace {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&self, scope: u32, kind: SchedulerEventKind, task: impl Into<String>) {
        let mut seq = self.next_seq.lock().expect("poisoned trace seq mutex");
        let event = SchedulerEvent {
            sequence: *seq,
            scope,
            kind,
            task: task.into(),
        };
        *seq += 1;
        self.events
            .lock()
            .expect("poisoned trace mutex")
            .push(event);
    }

    pub fn snapshot(&self) -> Vec<SchedulerEvent> {
        self.events.lock().expect("poisoned trace mutex").clone()
    }

    pub fn len(&self) -> usize {
        self.events.lock().expect("poisoned trace mutex").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Serialize to JSONL so consumers can pipe the trace alongside
    /// the [`crate::observability`] event stream.
    pub fn to_jsonl(&self) -> String {
        let mut out = String::new();
        for event in self.snapshot() {
            if let Ok(line) = serde_json::to_string(&event) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(trace: &SchedulerTrace) {
        trace.record(0, SchedulerEventKind::Enqueue, "tok");
        trace.record(0, SchedulerEventKind::Dispatch, "tok");
        trace.record(0, SchedulerEventKind::Finish, "tok");
        trace.record(1, SchedulerEventKind::Enqueue, "ast");
        trace.record(1, SchedulerEventKind::Dispatch, "ast");
        trace.record(1, SchedulerEventKind::Finish, "ast");
    }

    #[test]
    fn sequence_numbers_are_dense_and_monotonic() {
        let t = SchedulerTrace::new();
        fixture(&t);
        let events = t.snapshot();
        for (idx, e) in events.iter().enumerate() {
            assert_eq!(e.sequence as usize, idx);
        }
    }

    #[test]
    fn two_runs_produce_identical_traces() {
        let a = SchedulerTrace::new();
        let b = SchedulerTrace::new();
        fixture(&a);
        fixture(&b);
        assert_eq!(a.snapshot(), b.snapshot());
        assert_eq!(a.to_jsonl(), b.to_jsonl());
    }

    #[test]
    fn jsonl_lines_round_trip() {
        let t = SchedulerTrace::new();
        fixture(&t);
        let text = t.to_jsonl();
        for line in text.lines() {
            let _: SchedulerEvent = serde_json::from_str(line).unwrap();
        }
    }
}
