use super::*;
use cem_ml::runtime_admission::{generation::NativeGeneration, NativeAdmission, NativeRootKind};
use cem_ql::structural_receiver::{FreshQueryBinding, FreshQueryInvocation};
use std::{cell::RefCell, rc::Rc, time::Duration};
thread_local! { static ACTION: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) }; }
#[derive(Debug)]
struct Hook;
impl NativeDatatypeValidator for Hook {
    fn validate(&self, call: ValidationCall<'_>) -> RuleExecution {
        let action = ACTION.with(|a| a.borrow_mut().take());
        if let Some(action) = action {
            action();
        }
        Validator.validate(call)
    }
}
fn generation() -> Arc<NativeGeneration> {
    NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap()
}
fn fixture(
    g: &Arc<NativeGeneration>,
    control: OperationControl,
) -> (
    Rc<FreshStructuralHost>,
    Rc<FreshQueryBinding>,
    Rc<FreshQueryInvocation>,
) {
    let req = validator();
    let mut h = FreshStructuralHost::new_in_generation(&hash(b"registry"), control, g).unwrap();
    h.register_native_validator(validator_id(), Hook).unwrap();
    let scope = h
        .register_scope(
            req.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let binding = h
        .link_native_validator(
            req.clone(),
            scope,
            datatype(req.capture(), "sample").declaration().node_id(),
            adapter(),
        )
        .unwrap();
    let call = h.query_invocation(scope).unwrap();
    (Rc::new(h), Rc::new(binding), Rc::new(call))
}
fn input() -> ValidationInput {
    ValidationInput {
        value: vec![Item::Atomic(AtomValue::String("same".into()))],
        candidate: vec![],
        fallback: Default::default(),
    }
}
#[test]
fn callback_closure_waits_for_real_query_work_and_retained_request_owners() {
    let g = generation();
    let (h, b, call) = fixture(&g, Default::default());
    assert_eq!(g.inventory().unwrap().live_roots, 3);
    let slot = Rc::new(RefCell::new(None));
    let pending = slot.clone();
    let runtime = g.clone();
    ACTION.with(|a| {
        *a.borrow_mut() = Some(Box::new(move || {
            let drain = runtime
                .begin_drain(Duration::from_secs(5), Default::default())
                .unwrap();
            assert!(drain.poll().unwrap().active_entries > 0);
            assert!(drain.seal().is_err());
            *pending.borrow_mut() = Some(drain);
        }))
    });
    assert!(h
        .validate_query(&call, &b, &input(), Default::default())
        .is_err());
    let drain = slot.borrow_mut().take().unwrap();
    assert_eq!(drain.poll().unwrap().active_entries, 0);
    b.retire();
    call.retire();
    drop(h);
    assert_eq!(drain.poll().unwrap().roots[&NativeRootKind::Registry], 1);
    assert_eq!(drain.poll().unwrap().roots[&NativeRootKind::Preparation], 1);
    drop(b);
    assert!(drain.seal().is_err());
    drop(call);
    assert!(drain.seal().unwrap().is_current(&g));
}
#[test]
fn request_reentry_refuses_without_poisoning_the_outer_call_and_retirement_withholds_late_output() {
    let g = generation();
    let (h, b, call) = fixture(&g, Default::default());
    let (host, binding, request) = (Rc::downgrade(&h), Rc::downgrade(&b), Rc::downgrade(&call));
    ACTION.with(|a| {
        *a.borrow_mut() = Some(Box::new(move || {
            assert!(host
                .upgrade()
                .unwrap()
                .validate_query(
                    &request.upgrade().unwrap(),
                    &binding.upgrade().unwrap(),
                    &input(),
                    Default::default()
                )
                .is_err());
        }))
    });
    assert_eq!(
        h.validate_query(&call, &b, &input(), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    let request = Rc::downgrade(&call);
    ACTION.with(|a| *a.borrow_mut() = Some(Box::new(move || request.upgrade().unwrap().retire())));
    assert!(h
        .validate_query(&call, &b, &input(), Default::default())
        .is_err());
    assert_eq!(g.inventory().unwrap().active_entries, 0);
    assert_eq!(
        g.inventory().unwrap().roots[&NativeRootKind::Preparation],
        1
    );
}
#[test]
fn callback_cancellation_preserves_failure_and_settles_owned_activity() {
    let g = generation();
    let control = OperationControl::default();
    let (h, b, call) = fixture(&g, control.clone());
    ACTION.with(|a| {
        *a.borrow_mut() = Some(Box::new(move || {
            control.cancel_root(None, None).unwrap();
        }))
    });
    assert!(h
        .validate_query(&call, &b, &input(), Default::default())
        .is_err());
    assert!(h
        .validate_query(&call, &b, &input(), Default::default())
        .is_err());
    assert_eq!(g.inventory().unwrap().active_entries, 0);
}
