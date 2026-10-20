use cem_ml::runtime_memory::{
    MemoryBudgetConfig, MemorySample, RuntimeMemoryBudget, TrackingAllocator,
};
use std::alloc::{GlobalAlloc, Layout, System};
fn config() -> MemoryBudgetConfig {
    MemoryBudgetConfig {
        soft_limit_bytes: 80,
        hard_limit_bytes: 100,
        recovery_headroom_bytes: 20,
        max_concurrent_operations: 2,
    }
}
fn sample(live: u64, linear: Option<u64>) -> MemorySample {
    MemorySample {
        allocator_live_bytes: live,
        allocator_peak_bytes: live,
        linear_memory_bytes: linear,
    }
}
#[test]
fn actual_allocator_counts_successful_alloc_realloc_and_release() {
    let allocator = TrackingAllocator::new(System);
    unsafe {
        let layout = Layout::from_size_align(64, 8).unwrap();
        let ptr = allocator.alloc(layout);
        assert!(!ptr.is_null());
        assert_eq!(allocator.sample(None).allocator_live_bytes, 64);
        let ptr = allocator.realloc(ptr, layout, 128);
        assert!(!ptr.is_null());
        assert_eq!(allocator.sample(None).allocator_live_bytes, 128);
        let ptr = allocator.realloc(ptr, Layout::from_size_align(128, 8).unwrap(), 32);
        assert!(!ptr.is_null());
        assert_eq!(allocator.sample(None).allocator_live_bytes, 32);
        allocator.dealloc(ptr, Layout::from_size_align(32, 8).unwrap());
    }
    assert_eq!(allocator.sample(None).allocator_live_bytes, 0);
    assert_eq!(allocator.sample(None).allocator_peak_bytes, 128);
}
#[test]
fn independent_capacity_and_reserved_headroom_bound_admission() {
    let budget = RuntimeMemoryBudget::new(config()).unwrap();
    assert!(budget.admit(sample(10, Some(70)), 11).is_err());
    assert_eq!(budget.status(sample(10, Some(70))).reserved_bytes, 0);
    let permit = budget.admit(sample(10, Some(70)), 10).unwrap();
    assert!(budget.admit(sample(10, Some(70)), 1).is_err());
    assert!(budget.configure(config()).is_err());
    drop(permit);
    assert_eq!(budget.status(sample(10, None)).reserved_bytes, 0);
    assert!(budget.status(sample(10, Some(80))).hard_pressure);
    assert!(budget.status(sample(10, Some(101))).hard_pressure);
}
#[test]
fn bounded_concurrency_invalid_limits_and_failed_admission_are_atomic() {
    let budget = RuntimeMemoryBudget::new(config()).unwrap();
    let a = budget.admit(sample(0, None), 1).unwrap();
    let b = budget.admit(sample(0, None), 1).unwrap();
    assert!(budget.admit(sample(0, None), 1).is_err());
    assert_eq!(budget.status(sample(0, None)).active_operations, 2);
    drop(a);
    drop(b);
    let mut invalid = config();
    invalid.recovery_headroom_bytes = 30;
    assert!(budget.configure(invalid).is_err());
    assert_eq!(budget.status(sample(0, None)).config, config());
    assert!(budget.admit(sample(0, None), u64::MAX).is_err());
    assert_eq!(budget.status(sample(0, None)).active_operations, 0);
}
#[test]
fn hard_pressure_uses_existing_cancellation_and_memory_failure_contract() {
    use cem_ml::operation_control::{
        ControlCause, ControlError, OperationControl, ROOT_EXECUTION_SCOPE_ID,
    };
    let budget = RuntimeMemoryBudget::new(config()).unwrap();
    let control = OperationControl::default();
    control
        .check_runtime_memory(ROOT_EXECUTION_SCOPE_ID, &budget, sample(79, None))
        .unwrap();
    let error = control
        .check_runtime_memory(ROOT_EXECUTION_SCOPE_ID, &budget, sample(80, None))
        .unwrap_err();
    assert!(
        matches!(error,ControlError::Triggered(f) if matches!(f.cause,ControlCause::MemoryExceeded{charged:80,limit:80,..}))
    );
    assert!(control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_err());
    let permit = budget.admit(sample(0, None), 20).unwrap();
    drop(permit);
    assert_eq!(budget.status(sample(0, None)).active_operations, 0);
}

#[test]
fn earlier_cancellation_wins_and_pressure_does_not_poison_other_controls() {
    use cem_ml::operation_control::{
        ControlCause, ControlError, OperationControl, ROOT_EXECUTION_SCOPE_ID,
    };
    let budget = RuntimeMemoryBudget::new(config()).unwrap();
    let control = OperationControl::default();
    control
        .cancel_root(Some("cancelled before pressure".into()), None)
        .unwrap();
    assert!(
        matches!(control.check_runtime_memory(ROOT_EXECUTION_SCOPE_ID,&budget,sample(99,None)),Err(ControlError::Triggered(f)) if matches!(f.cause,ControlCause::HostCancellation{..}))
    );
    OperationControl::default()
        .check_runtime_memory(ROOT_EXECUTION_SCOPE_ID, &budget, sample(10, None))
        .unwrap();
}
#[test]
fn failed_allocation_does_not_create_a_charge() {
    struct Refuse;
    unsafe impl GlobalAlloc for Refuse {
        unsafe fn alloc(&self, _: Layout) -> *mut u8 {
            std::ptr::null_mut()
        }
        unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
    }
    let allocator = TrackingAllocator::new(Refuse);
    unsafe {
        assert!(allocator
            .alloc(Layout::from_size_align(16, 8).unwrap())
            .is_null());
    }
    assert_eq!(allocator.sample(None), sample(0, None));
}
