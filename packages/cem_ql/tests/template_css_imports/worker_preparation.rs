use super::*;
use cem_ml::{
    scheduler::AbortSignal,
    worker_control::{WorkerAddress, WorkerCoordinator, WorkerGeneration, WorkerSlotId},
};

fn address(generation: u64) -> WorkerAddress {
    WorkerAddress::new(
        WorkerSlotId::from_raw(1),
        WorkerGeneration::from_raw(generation),
    )
}
fn worker(limit: usize) -> RecordedPreparationWorker {
    RecordedPreparationWorker::new(address(1), limit).unwrap()
}
fn prepare(
    worker: &mut RecordedPreparationWorker,
    cancel: AbortSignal,
) -> (
    RecordedPreparationHandle,
    Arc<RecordedBodyStylesheetInstallations>,
    Fixture,
) {
    let mut f = fixture(
        ROOT_SOURCE,
        &CHILD_SOURCE.replace(" layer(base)", ""),
        false,
    );
    let (compiled, output) = composed(&mut f);
    let document = composed_document(&mut f, &output);
    let html = std::mem::replace(&mut f.host, empty_host());
    let handle = worker
        .prepare(
            output,
            html,
            document,
            cancel,
            &mut OutputObserver { wrong: false },
            slots,
            BASE,
            Default::default(),
        )
        .unwrap();
    (handle, compiled, f)
}
fn empty_host() -> HtmlBaseExecutionHost {
    HtmlBaseExecutionHost::new(
        HtmlBaseCompatibilityProfile::StandardAgreement,
        HtmlBaseBrowser {
            engine: "chromium".into(),
            version: CHROMIUM_BASE_PROFILE_VERSION.into(),
        },
        HtmlBaseLimits {
            nodes: 10000,
            url_bytes: 4096,
        },
    )
    .unwrap()
}
fn read(
    worker: &mut RecordedPreparationWorker,
    handle: &RecordedPreparationHandle,
) -> Result<Vec<String>, String> {
    worker.read(handle, &mut OutputObserver { wrong: false }, slots, BASE)
}

#[test]
fn preparation_worker_retains_complete_native_record_and_revalidates_every_read() {
    let mut worker = worker(2);
    let (handle, compiled, f) = prepare(&mut worker, AbortSignal::new());
    drop(f);
    let blockers = read(&mut worker, &handle).unwrap();
    assert!(blockers
        .iter()
        .any(|b| b == "atomic-host-transaction-required"));
    assert!(blockers
        .iter()
        .any(|b| b == "css-runtime-dependency-closure-required"));
    assert_eq!(worker.len(), 1);
    compiled.release();
    assert!(read(&mut worker, &handle).is_err());
    assert_eq!(worker.len(), 0);
    assert!(handle.is_revoked());
}

#[test]
fn same_address_and_local_id_cannot_substitute_a_different_worker_owner() {
    let mut first = worker(1);
    let mut second = worker(1);
    let (a, _, _) = prepare(&mut first, AbortSignal::new());
    let (b, _, _) = prepare(&mut second, AbortSignal::new());
    assert!(read(&mut second, &a).is_err());
    assert!(second.release(&a).is_err());
    assert!(read(&mut first, &a).is_ok());
    assert!(read(&mut second, &b).is_ok());
    first.release(&a).unwrap();
    first.release(&a).unwrap();
    assert!(a.is_revoked());
    let (next, _, _) = prepare(&mut first, AbortSignal::new());
    assert!(read(&mut first, &a).is_err());
    assert!(read(&mut first, &next).is_ok());
}

#[test]
fn preparation_capacity_is_bounded_and_released_handles_are_reclaimed() {
    assert!(RecordedPreparationWorker::new(address(0), 1).is_err());
    assert!(RecordedPreparationWorker::new(
        WorkerAddress::new(WorkerSlotId::from_raw(0), WorkerGeneration::from_raw(1)),
        1
    )
    .is_err());
    assert!(RecordedPreparationWorker::new(address(1), 0).is_err());
    assert!(RecordedPreparationWorker::new(address(1), 257).is_err());
    let mut worker = worker(1);
    let (handle, _, _) = prepare(&mut worker, AbortSignal::new());
    let mut f = fixture(
        ROOT_SOURCE,
        &CHILD_SOURCE.replace(" layer(base)", ""),
        false,
    );
    let (_, output) = composed(&mut f);
    let document = composed_document(&mut f, &output);
    let result = worker.prepare(
        output,
        f.host,
        document,
        AbortSignal::new(),
        &mut OutputObserver { wrong: false },
        |_| panic!("capacity must reject before callbacks"),
        BASE,
        Default::default(),
    );
    assert!(result.is_err());
    assert!(read(&mut worker, &handle).is_ok());
    handle.release();
    let (next, _, _) = prepare(&mut worker, AbortSignal::new());
    assert_eq!(worker.len(), 1);
    assert!(read(&mut worker, &handle).is_err());
    assert!(read(&mut worker, &next).is_ok());
}

#[test]
fn cancellation_and_close_during_capture_never_publish_a_handle() {
    for close in [false, true] {
        let mut worker = worker(1);
        let lifetime = worker.lifetime();
        let cancel = AbortSignal::new();
        let mut f = fixture(
            ROOT_SOURCE,
            &CHILD_SOURCE.replace(" layer(base)", ""),
            false,
        );
        let (_, output) = composed(&mut f);
        let document = composed_document(&mut f, &output);
        let result = worker.prepare(
            output,
            f.host,
            document,
            cancel.clone(),
            &mut OutputObserver { wrong: false },
            |slot| {
                if close {
                    lifetime.close();
                } else {
                    cancel.abort();
                }
                slots(slot)
            },
            BASE,
            Default::default(),
        );
        assert!(result.is_err());
        assert_eq!(worker.len(), 0);
    }
}

#[test]
fn cancellation_and_close_during_read_revoke_before_returning_metadata() {
    for close in [false, true] {
        let mut worker = worker(1);
        let lifetime = worker.lifetime();
        let cancel = AbortSignal::new();
        let (handle, _, _) = prepare(&mut worker, cancel.clone());
        let result = worker.read(
            &handle,
            &mut OutputObserver { wrong: false },
            |slot| {
                if close {
                    lifetime.close();
                } else {
                    cancel.abort();
                }
                slots(slot)
            },
            BASE,
        );
        assert!(result.is_err());
        assert!(handle.is_revoked());
        assert_eq!(worker.len(), 0);
        assert!(read(&mut worker, &handle).is_err());
    }
}

#[test]
fn cancelled_handle_and_worker_drop_cannot_be_revived() {
    let mut worker = worker(1);
    let cancel = AbortSignal::new();
    let (handle, _, _) = prepare(&mut worker, cancel.clone());
    cancel.abort();
    assert!(handle.is_revoked());
    assert!(read(&mut worker, &handle).is_err());
    let (next, _, _) = prepare(&mut worker, AbortSignal::new());
    let lifetime = worker.lifetime();
    drop(worker);
    assert!(next.is_revoked());
    assert!(lifetime.is_closed());
}

#[test]
fn generation_replacement_rejects_old_handles_and_allows_fresh_preparation() {
    let mut coordinator = WorkerCoordinator::new(1).unwrap();
    let first = coordinator.worker(WorkerSlotId::from_raw(1)).unwrap();
    let mut worker = RecordedPreparationWorker::new(first, 1).unwrap();
    let (handle, _, _) = prepare(&mut worker, AbortSignal::new());
    let old_lifetime = worker.lifetime();
    assert!(worker.replace(first).is_err());
    assert!(worker.replace(address(3)).is_err());
    assert!(read(&mut worker, &handle).is_ok());
    let replacement = coordinator.replace_worker(first.slot).unwrap();
    worker.replace(replacement.replacement).unwrap();
    assert!(handle.is_revoked());
    assert!(old_lifetime.is_closed());
    assert!(read(&mut worker, &handle).is_err());
    let (fresh, _, _) = prepare(&mut worker, AbortSignal::new());
    old_lifetime.close();
    assert!(read(&mut worker, &fresh).is_ok());
    worker.close();
    assert!(fresh.is_revoked());
    assert!(read(&mut worker, &fresh).is_err());
}

#[test]
fn failed_document_css_and_policy_reads_remove_the_record_permanently() {
    for mode in ["dom", "css", "policy"] {
        let mut worker = worker(1);
        let (handle, _, f) = prepare(&mut worker, AbortSignal::new());
        if mode == "policy" {
            f.revision.store(1, Ordering::SeqCst);
        }
        let result = worker.read(
            &handle,
            &mut OutputObserver {
                wrong: mode == "css",
            },
            |slot| {
                if mode == "dom" {
                    Ok(slot.id + 100)
                } else {
                    slots(slot)
                }
            },
            BASE,
        );
        assert!(result.is_err(), "{mode}");
        f.revision.store(0, Ordering::SeqCst);
        assert_eq!(worker.len(), 0);
        assert!(handle.is_revoked());
        assert!(read(&mut worker, &handle).is_err());
    }
}

#[test]
fn empty_stylesheet_preparation_still_checks_body_policies() {
    let root = ROOT_SOURCE
        .lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n")
        .replace(" @style='background:url(./document.svg)'", "");
    let child = CHILD_SOURCE.lines().skip(1).collect::<Vec<_>>().join("\n");
    let mut f = fixture(&root, &child, false);
    let (_, output) = composed(&mut f);
    assert!(output.stylesheet_nodes().is_empty());
    let document = composed_document(&mut f, &output);
    let mut worker = worker(1);
    let handle = worker
        .prepare(
            output,
            f.host,
            document,
            AbortSignal::new(),
            &mut OutputObserver { wrong: false },
            slots,
            BASE,
            Default::default(),
        )
        .unwrap();
    assert!(read(&mut worker, &handle).is_ok());
    f.revision.store(1, Ordering::SeqCst);
    assert!(read(&mut worker, &handle).is_err());
    assert!(handle.is_revoked());
}

#[test]
fn preaborted_and_closed_workers_reject_before_observation() {
    for close in [false, true] {
        let mut worker = worker(1);
        let cancel = AbortSignal::new();
        if close {
            worker.close();
        } else {
            cancel.abort();
        }
        let mut f = fixture(
            ROOT_SOURCE,
            &CHILD_SOURCE.replace(" layer(base)", ""),
            false,
        );
        let (_, output) = composed(&mut f);
        let document = composed_document(&mut f, &output);
        assert!(worker
            .prepare(
                output,
                f.host,
                document,
                cancel,
                &mut OutputObserver { wrong: false },
                |_| panic!("revoked lifetime must reject before callbacks"),
                BASE,
                Default::default()
            )
            .is_err());
        assert!(worker.is_empty());
    }
}

#[test]
fn release_or_unwinding_in_a_callback_revokes_the_record() {
    let mut worker = worker(1);
    let (handle, _, _) = prepare(&mut worker, AbortSignal::new());
    let clone = handle.clone();
    assert!(worker
        .read(
            &handle,
            &mut OutputObserver { wrong: false },
            |slot| {
                clone.release();
                slots(slot)
            },
            BASE
        )
        .is_err());
    assert!(clone.is_revoked());
    let (next, _, _) = prepare(&mut worker, AbortSignal::new());
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        worker.read(
            &next,
            &mut OutputObserver { wrong: false },
            |_| panic!("fixture callback panic"),
            BASE,
        )
    }))
    .is_err());
    assert!(next.is_revoked());
    assert!(worker.is_empty());
}

#[test]
fn stylesheet_callbacks_cannot_publish_after_cancellation() {
    use cem_ml::stylesheet_execution::*;
    struct CancellingObserver {
        cancel: AbortSignal,
        on_revision: bool,
    }
    impl StylesheetExecutionObserver for CancellingObserver {
        fn environment_revision(&mut self) -> Result<String, String> {
            if self.on_revision {
                self.cancel.abort();
            }
            Ok("output:1".into())
        }
        fn observe(
            &mut self,
            slot: StylesheetExecutionSlot<'_>,
        ) -> Result<StylesheetObservation, String> {
            self.cancel.abort();
            OutputObserver { wrong: false }.observe(slot)
        }
    }
    for on_revision in [false, true] {
        let mut worker = worker(1);
        let cancel = AbortSignal::new();
        let (handle, _, _) = prepare(&mut worker, cancel.clone());
        assert!(worker
            .read(
                &handle,
                &mut CancellingObserver {
                    cancel,
                    on_revision
                },
                slots,
                BASE
            )
            .is_err());
        assert!(handle.is_revoked());
        assert!(worker.is_empty());
    }
}
