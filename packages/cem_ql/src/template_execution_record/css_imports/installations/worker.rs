//! Realm-local preparation registry. Handles select retained native records;
//! addresses, local IDs and read diagnostics cannot reconstruct those records.
use super::output::{
    RecordedTemplateOutput, RecordedTemplateOutputEvidence, RecordedTemplateOutputHost,
};
use cem_ml::{
    css_import_snapshot::CssSnapshotCaptureLimits,
    html_base_host::{HtmlBaseDomSlot, HtmlBaseExecutionHost},
    html_execution_inventory::HtmlExecutionInventory,
    scheduler::AbortSignal,
    stylesheet_execution::{
        StylesheetExecutionObserver, StylesheetExecutionSlot, StylesheetObservation,
    },
    worker_control::WorkerAddress,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Weak},
};

pub const MAX_RECORDED_PREPARATIONS: usize = 256;

struct WorkerState {
    address: WorkerAddress,
    closed: AbortSignal,
}

/// Cloneable revocation control, safe to close from an observation callback.
/// The execution host closes this before worker failure/termination/replacement.
#[derive(Clone)]
pub struct RecordedPreparationLifetime {
    state: Arc<WorkerState>,
}
impl RecordedPreparationLifetime {
    pub fn close(&self) {
        self.state.closed.abort();
    }
    pub fn is_closed(&self) -> bool {
        self.state.closed.is_aborted()
    }
}

/// Native capability with no public constructor or serialization implementation.
/// A clone refers to the same record and shares release/cancellation state.
#[derive(Clone)]
pub struct RecordedPreparationHandle {
    owner: Weak<WorkerState>,
    id: u64,
    released: AbortSignal,
    cancel: AbortSignal,
}
impl RecordedPreparationHandle {
    /// Revoke immediately; the registry reclaims storage at its next operation.
    pub fn release(&self) {
        self.released.abort();
    }
    /// Lifetime status only. Successful consumption still requires `read`.
    pub fn is_revoked(&self) -> bool {
        self.check().is_err()
    }
    fn check(&self) -> Result<(), String> {
        let owner = self.owner.upgrade().ok_or("preparation worker dropped")?;
        if owner.closed.is_aborted() || self.released.is_aborted() || self.cancel.is_aborted() {
            return Err("preparation lifetime revoked".into());
        }
        Ok(())
    }
}

struct Entry {
    handle: RecordedPreparationHandle,
    html: HtmlBaseExecutionHost,
    host: RecordedTemplateOutputHost,
    evidence: RecordedTemplateOutputEvidence,
}
impl Drop for Entry {
    fn drop(&mut self) {
        self.handle.release();
    }
}

/// One exclusive native registry per execution realm/worker generation.
/// Capturing transfers both observation hosts into this owner. Every read
/// revalidates the complete joined record; no seal or host escapes the registry.
pub struct RecordedPreparationWorker {
    state: Arc<WorkerState>,
    entries: BTreeMap<u64, Entry>,
    next_id: u64,
    limit: usize,
}
impl RecordedPreparationWorker {
    pub fn new(address: WorkerAddress, limit: usize) -> Result<Self, String> {
        if address.slot.get() == 0
            || address.generation.get() == 0
            || limit == 0
            || limit > MAX_RECORDED_PREPARATIONS
        {
            return Err("nonzero worker address and bounded preparation capacity required".into());
        }
        Ok(Self {
            state: Arc::new(WorkerState {
                address,
                closed: AbortSignal::new(),
            }),
            entries: BTreeMap::new(),
            next_id: 1,
            limit,
        })
    }
    pub fn address(&self) -> WorkerAddress {
        self.state.address
    }
    pub fn lifetime(&self) -> RecordedPreparationLifetime {
        RecordedPreparationLifetime {
            state: self.state.clone(),
        }
    }
    pub fn len(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| !entry.handle.is_revoked())
            .count()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn close(&mut self) {
        self.state.closed.abort();
        self.entries.clear();
    }
    /// Apply the next address issued by the shared worker coordinator. Closing
    /// the old identity happens before a fresh generation can retain records.
    pub fn replace(&mut self, next: WorkerAddress) -> Result<(), String> {
        if next.slot != self.state.address.slot
            || self.state.address.generation.get().checked_add(1) != Some(next.generation.get())
        {
            return Err("preparation replacement requires the next worker generation".into());
        }
        self.close();
        self.state = Arc::new(WorkerState {
            address: next,
            closed: AbortSignal::new(),
        });
        self.next_id = 1;
        Ok(())
    }
    fn reap(&mut self) {
        self.entries.retain(|_, entry| !entry.handle.is_revoked());
    }
    fn check_owner(&self, handle: &RecordedPreparationHandle) -> Result<(), String> {
        if !Weak::ptr_eq(&handle.owner, &Arc::downgrade(&self.state)) {
            return Err("foreign preparation worker or generation".into());
        }
        Ok(())
    }
    pub fn release(&mut self, handle: &RecordedPreparationHandle) -> Result<(), String> {
        self.check_owner(handle)?;
        handle.release();
        self.entries.remove(&handle.id);
        self.reap();
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &mut self,
        output: Arc<RecordedTemplateOutput>,
        mut html: HtmlBaseExecutionHost,
        document: Arc<HtmlExecutionInventory>,
        cancel: AbortSignal,
        observer: &mut impl StylesheetExecutionObserver,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base: &str,
        limits: CssSnapshotCaptureLimits,
    ) -> Result<RecordedPreparationHandle, String> {
        self.reap();
        if self.entries.len() >= self.limit {
            return Err("preparation handle capacity exceeded".into());
        }
        let handle = RecordedPreparationHandle {
            owner: Arc::downgrade(&self.state),
            id: self.next_id,
            released: AbortSignal::new(),
            cancel,
        };
        handle.check()?;
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or("preparation identity exhausted")?;
        let mut host = RecordedTemplateOutputHost::default();
        let evidence = output.capture(
            &mut html,
            document,
            &mut host,
            &mut CheckedObserver {
                handle: &handle,
                observer,
            },
            |slot| {
                handle.check()?;
                let identity = matches(slot)?;
                handle.check()?;
                Ok(identity)
            },
            base,
            limits,
        )?;
        handle.check()?;
        self.entries.insert(
            handle.id,
            Entry {
                handle: handle.clone(),
                html,
                host,
                evidence,
            },
        );
        Ok(handle)
    }
    /// Returns diagnostic blockers only, never transaction authority. Failed
    /// validation removes the record permanently, including callback failures.
    pub fn read(
        &mut self,
        handle: &RecordedPreparationHandle,
        observer: &mut impl StylesheetExecutionObserver,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base: &str,
    ) -> Result<Vec<String>, String> {
        self.check_owner(handle)?;
        self.reap();
        handle.check()?;
        // Removal before callbacks also makes unwinding revoke the handle.
        let mut entry = self
            .entries
            .remove(&handle.id)
            .ok_or("preparation handle unavailable")?;
        entry.evidence.validate(
            &mut entry.html,
            &mut entry.host,
            &mut CheckedObserver { handle, observer },
            |slot| {
                handle.check()?;
                let identity = matches(slot)?;
                handle.check()?;
                Ok(identity)
            },
            base,
        )?;
        handle.check()?;
        let blockers = entry.evidence.blockers().to_vec();
        self.entries.insert(handle.id, entry);
        Ok(blockers)
    }
}
impl Drop for RecordedPreparationWorker {
    fn drop(&mut self) {
        self.close();
    }
}

struct CheckedObserver<'a, T> {
    handle: &'a RecordedPreparationHandle,
    observer: &'a mut T,
}
impl<T: StylesheetExecutionObserver> StylesheetExecutionObserver for CheckedObserver<'_, T> {
    fn environment_revision(&mut self) -> Result<String, String> {
        self.handle.check()?;
        let revision = self.observer.environment_revision()?;
        self.handle.check()?;
        Ok(revision)
    }
    fn observe(
        &mut self,
        slot: StylesheetExecutionSlot<'_>,
    ) -> Result<StylesheetObservation, String> {
        self.handle.check()?;
        let observation = self.observer.observe(slot)?;
        self.handle.check()?;
        Ok(observation)
    }
}
