//! Original native generation enrollment. These scoped drain proofs are neither
//! the complete command census nor physical worker/fallback retirement permits.
use super::*;
use crate::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
use std::{sync::Weak, time::Duration};
use web_time::Instant;
const INVALID: &str = "cem.runtime.generation.unavailable";
pub struct NativeGeneration {
    gate: Arc<NativeAdmission>,
    drain: Mutex<Weak<DrainState>>,
}
impl NativeGeneration {
    pub fn claim(gate: Arc<NativeAdmission>) -> Result<Arc<Self>, &'static str> {
        {
            let mut state = gate.state.lock().map_err(|_| UNCERTAIN)?;
            if state.closed
                || state.frozen
                || state.uncertain
                || state.generation_claimed
                || state.active != 0
                || !state.resources.is_empty()
            {
                return Err(INVALID);
            }
            state.generation_claimed = true;
            mutation(&mut state);
        }
        Ok(Arc::new(Self {
            gate,
            drain: Mutex::new(Weak::new()),
        }))
    }
    pub fn claim_current() -> Result<Arc<Self>, &'static str> {
        Self::claim(current_gate())
    }
    pub fn is_open(&self) -> bool {
        self.gate
            .state
            .lock()
            .is_ok_and(|s| !s.closed && !s.frozen && !s.uncertain)
    }
    pub fn admit(&self) -> Result<OwnedNativeEntry, &'static str> {
        self.gate.admit_owned(false)
    }
    pub fn resource(&self, kind: NativeRootKind) -> Result<NativeResourceLease, &'static str> {
        self.gate.enroll(kind)
    }
    pub fn inventory(&self) -> Result<EnrolledInventory, &'static str> {
        self.gate.enrolled_inventory()
    }
    pub fn begin_drain(
        self: &Arc<Self>,
        timeout: Duration,
        control: OperationControl,
    ) -> Result<GenerationDrain, &'static str> {
        if timeout.is_zero() || timeout > Duration::from_secs(5) {
            return Err("cem.runtime.generation.deadline");
        }
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID)?;
        let mut current = self.drain.lock().map_err(|_| UNCERTAIN)?;
        if current.upgrade().is_some() {
            return Err("cem.runtime.generation.busy");
        }
        self.gate.close()?;
        let state = Arc::new(DrainState {
            generation: Arc::downgrade(self),
            deadline: Instant::now() + timeout,
            control,
        });
        *current = Arc::downgrade(&state);
        Ok(GenerationDrain { state })
    }
}
impl Drop for NativeGeneration {
    fn drop(&mut self) {
        let _ = self.gate.close();
    }
}
struct DrainState {
    generation: Weak<NativeGeneration>,
    deadline: Instant,
    control: OperationControl,
}
impl DrainState {
    fn current(&self) -> Result<Arc<NativeGeneration>, &'static str> {
        self.control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID)?;
        if Instant::now() >= self.deadline {
            return Err("cem.runtime.generation.timeout");
        }
        let generation = self.generation.upgrade().ok_or(INVALID)?;
        if !generation
            .drain
            .lock()
            .map_err(|_| UNCERTAIN)?
            .upgrade()
            .is_some_and(|s| std::ptr::eq(s.as_ref(), self))
        {
            return Err(INVALID);
        }
        Ok(generation)
    }
}
pub struct GenerationDrain {
    state: Arc<DrainState>,
}
impl GenerationDrain {
    pub fn poll(&self) -> Result<EnrolledInventory, &'static str> {
        let generation = self.state.current()?;
        let inventory = generation.inventory()?;
        if !inventory.closed || inventory.frozen {
            return Err(INVALID);
        }
        Ok(inventory)
    }
    pub fn seal(&self) -> Result<GenerationDrainWitness, &'static str> {
        let inventory = self.poll()?;
        if inventory.active_entries != 0 || inventory.live_roots != 0 {
            return Err("cem.runtime.generation.not-drained");
        }
        Ok(GenerationDrainWitness {
            drain: Arc::downgrade(&self.state),
            revision: inventory.revision,
        })
    }
}
/// Private original issuer + revision; a serialized inventory is not this proof.
pub struct GenerationDrainWitness {
    drain: Weak<DrainState>,
    revision: u64,
}
impl GenerationDrainWitness {
    pub fn is_current(&self, generation: &Arc<NativeGeneration>) -> bool {
        self.drain.upgrade().is_some_and(|d| {
            d.current().is_ok_and(|g| Arc::ptr_eq(&g, generation))
                && generation.inventory().is_ok_and(|i| {
                    i.closed
                        && !i.frozen
                        && i.active_entries == 0
                        && i.live_roots == 0
                        && i.revision == self.revision
                })
        })
    }
}
