//! Original-operation CPU accounting over explicitly admitted synchronous phases.
//! Unsupported hosts refuse a receipt. Wall time is never substituted for CPU.
use super::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
use std::{marker::PhantomData, rc::Rc, sync::Arc, thread::ThreadId};
#[derive(Debug, Default)]
pub(super) struct CpuState {
    limit: Option<u128>,
    total: u128,
    active: Option<(ThreadId, u128)>,
    failed: bool,
}
#[derive(Debug)]
pub struct NativeCpuAccounting {
    control: OperationControl,
}
pub struct NativeCpuGuard {
    owner: Arc<NativeCpuAccounting>,
    closed: bool,
    _thread_bound: PhantomData<Rc<()>>,
}
impl NativeCpuAccounting {
    pub const fn supported() -> bool {
        cfg!(any(target_os = "linux", windows))
    }
    pub fn new(control: &OperationControl, limit_ms: u64) -> Result<Self, String> {
        thread_cpu_ns()?;
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|e| e.code().to_owned())?;
        let limit = u128::from(limit_ms)
            .checked_mul(1_000_000)
            .ok_or("CPU limit overflow")?;
        let mut inner = control
            .inner
            .lock()
            .map_err(|_| "CPU accounting unavailable")?;
        if limit == 0 || inner.cpu.failed || inner.cpu.limit.is_some_and(|old| old != limit) {
            return Err("Original CPU budget cannot be renewed or replaced".into());
        }
        inner.cpu.limit = Some(limit);
        Ok(Self {
            control: control.clone(),
        })
    }
    pub fn begin(self: &Arc<Self>, control: &OperationControl) -> Result<NativeCpuGuard, String> {
        self.check_operation(control)?;
        let start = thread_cpu_ns()?;
        let mut inner = self
            .control
            .inner
            .lock()
            .map_err(|_| "CPU accounting unavailable")?;
        observe(&mut inner.cpu)?;
        if inner.cpu.active.is_some() {
            return Err("Overlapping CPU phases are refused".into());
        }
        inner.cpu.active = Some((std::thread::current().id(), start));
        Ok(NativeCpuGuard {
            owner: self.clone(),
            closed: false,
            _thread_bound: PhantomData,
        })
    }
    pub fn usage_ms(&self, control: &OperationControl) -> Result<u64, String> {
        self.check_operation(control)?;
        let mut inner = self
            .control
            .inner
            .lock()
            .map_err(|_| "CPU accounting unavailable")?;
        let nanos = observe(&mut inner.cpu)?;
        u64::try_from(nanos.div_ceil(1_000_000)).map_err(|_| "CPU usage overflow".into())
    }
    pub fn is_idle(&self) -> bool {
        self.control
            .inner
            .lock()
            .is_ok_and(|inner| inner.cpu.active.is_none() && !inner.cpu.failed)
    }
    fn check_operation(&self, control: &OperationControl) -> Result<(), String> {
        if !self.control.same_operation(control) {
            return Err("CPU accounting belongs to the original operation".into());
        }
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|e| e.code().to_owned())
    }
    fn close(&self, complete: bool) -> Result<(), String> {
        let mut inner = self
            .control
            .inner
            .lock()
            .map_err(|_| "CPU accounting unavailable")?;
        let total = observe(&mut inner.cpu);
        if let Ok(total) = total {
            inner.cpu.total = total;
        }
        inner.cpu.active = None;
        if !complete || total.is_err() {
            inner.cpu.failed = true;
        }
        total.map(|_| ())
    }
}
impl NativeCpuGuard {
    pub fn finish(mut self) -> Result<(), String> {
        self.closed = true;
        self.owner.close(true)
    }
}
impl Drop for NativeCpuGuard {
    fn drop(&mut self) {
        if !self.closed {
            let _ = self.owner.close(false);
        }
    }
}
fn observe(state: &mut CpuState) -> Result<u128, String> {
    if state.failed {
        return Err("CPU accounting is incomplete or exhausted".into());
    }
    if state
        .active
        .is_some_and(|(thread, _)| thread != std::thread::current().id())
    {
        return Err("Active CPU phase belongs to another thread".into());
    }
    let result: Result<u128, String> = (|| {
        let delta = match state.active {
            Some((thread, start)) => {
                if thread != std::thread::current().id() {
                    return Err("Active CPU phase belongs to another thread".into());
                }
                thread_cpu_ns()?
                    .checked_sub(start)
                    .ok_or("CPU counter regressed")?
            }
            None => 0,
        };
        let total = state.total.checked_add(delta).ok_or("CPU usage overflow")?;
        if total > state.limit.ok_or("CPU limit is unavailable")? {
            return Err("Original CPU budget exhausted".into());
        }
        Ok(total)
    })();
    if result.is_err() {
        state.failed = true;
    }
    result
}
#[cfg(target_os = "linux")]
fn thread_cpu_ns() -> Result<u128, String> {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // The OS writes the initialized native timespec for this calling thread.
    if unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut time) } != 0
        || time.tv_sec < 0
        || !(0..1_000_000_000).contains(&time.tv_nsec)
    {
        return Err("Native thread CPU clock unavailable".into());
    }
    Ok(time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128)
}
#[cfg(windows)]
fn thread_cpu_ns() -> Result<u128, String> {
    use windows_sys::Win32::{
        Foundation::FILETIME,
        System::Threading::{GetCurrentThread, GetThreadTimes},
    };
    let zero = || FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let (mut created, mut exited, mut kernel, mut user) = (zero(), zero(), zero(), zero());
    // Four distinct initialized FILETIMEs receive this calling thread's counters.
    if unsafe {
        GetThreadTimes(
            GetCurrentThread(),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
    } == 0
    {
        return Err("Native thread CPU clock unavailable".into());
    }
    let ticks =
        |time: FILETIME| (u128::from(time.dwHighDateTime) << 32) | u128::from(time.dwLowDateTime);
    Ok((ticks(kernel) + ticks(user)) * 100)
}
#[cfg(not(any(target_os = "linux", windows)))]
fn thread_cpu_ns() -> Result<u128, String> {
    Err("Native thread CPU clock unavailable on this target".into())
}
