#![cfg(any(target_os = "linux", windows))]
use cem_ml::{
    operation_control::{cpu::NativeCpuAccounting, OperationControl},
    scheduler::AbortSignal,
};
use std::sync::Arc;
fn control() -> OperationControl {
    OperationControl::new(AbortSignal::new())
}
fn burn() {
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_millis(50) {
        std::hint::black_box((0..10000u64).fold(0u64, |v, n| v.wrapping_add(n)));
    }
}
#[test]
fn original_cpu_is_cumulative_and_excludes_transport_wait() {
    let c = control();
    let a = Arc::new(NativeCpuAccounting::new(&c, 1000).unwrap());
    let guard = a.begin(&c).unwrap();
    burn();
    guard.finish().unwrap();
    let before = a.usage_ms(&c).unwrap();
    assert!(before > 0);
    std::thread::sleep(std::time::Duration::from_millis(30));
    assert_eq!(a.usage_ms(&c).unwrap(), before);
    let b = Arc::new(NativeCpuAccounting::new(&c, 1000).unwrap());
    assert_eq!(b.usage_ms(&c).unwrap(), before);
    let g = b.begin(&c).unwrap();
    burn();
    g.finish().unwrap();
    assert!(a.usage_ms(&c).unwrap() > before);
}
#[test]
fn foreign_overlap_and_unfinished_cpu_phases_do_not_create_receipts() {
    let c = control();
    let a = Arc::new(NativeCpuAccounting::new(&c, 1000).unwrap());
    assert!(a.begin(&control()).is_err());
    let guard = a.begin(&c).unwrap();
    assert!(a.begin(&c).is_err());
    drop(guard);
    assert!(a.usage_ms(&c).is_err());
    assert!(NativeCpuAccounting::new(&c, 1000).is_err());
}
#[test]
fn exhausted_cpu_cannot_be_reset_by_another_handle() {
    let c = control();
    let a = Arc::new(NativeCpuAccounting::new(&c, 1).unwrap());
    let g = a.begin(&c).unwrap();
    burn();
    assert!(g.finish().is_err());
    assert!(a.usage_ms(&c).is_err());
    assert!(NativeCpuAccounting::new(&c, 1000).is_err());
}

#[test]
fn sequential_threads_share_the_original_cpu_total() {
    let c = control();
    let owner = Arc::new(NativeCpuAccounting::new(&c, 1000).unwrap());
    let guard = owner.begin(&c).unwrap();
    burn();
    guard.finish().unwrap();
    let before = owner.usage_ms(&c).unwrap();
    let other = owner.clone();
    let cc = c.clone();
    let after = std::thread::spawn(move || {
        let guard = other.begin(&cc).unwrap();
        burn();
        guard.finish().unwrap();
        other.usage_ms(&cc).unwrap()
    })
    .join()
    .unwrap();
    assert!(after > before);
    assert_eq!(owner.usage_ms(&c).unwrap(), after);
}
