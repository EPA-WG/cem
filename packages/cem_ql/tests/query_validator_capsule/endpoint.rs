use super::*;
use cem_ml::runtime_admission::{generation::NativeGeneration, NativeAdmission};
use cem_ql::{
    query_source_endpoint::QuerySourceEndpoint,
    query_validator_capsule::profiles::{QueryExecutionPins, DEPLOYMENT_ABI},
};

fn generation() -> Arc<NativeGeneration> {
    NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap()
}
fn packet(text: &str) -> capsule::handoff::BuiltQueryHandoff {
    let d = verified(text);
    let mut h = host();
    let s = register(&mut h, &d);
    let b = h
        .link_query(d.clone(), s, datatype(&d), &functions(), adapter())
        .unwrap();
    h.capture_query_handoff(&b).unwrap()
}
fn pins(packet: &capsule::handoff::BuiltQueryHandoff) -> QueryExecutionPins {
    QueryExecutionPins {
        version: DEPLOYMENT_ABI.into(),
        source: packet.pin.clone(),
        result_profile: super::execution::profile(super::execution::profile_text())
            .pin()
            .clone(),
    }
}
fn endpoint(g: &Arc<NativeGeneration>, pins: QueryExecutionPins) -> Arc<QuerySourceEndpoint> {
    QuerySourceEndpoint::new(
        g,
        Default::default(),
        pins,
        functions(),
        super::execution::profile(super::execution::profile_text()),
    )
    .unwrap()
}
#[test]
fn endpoint_requires_trusted_original_deployment_and_profile_not_packet_claims() {
    let g = generation();
    let a = packet(&declaration("{accepted: true, diagnostics: ()}"));
    let b = packet(&declaration("{accepted: false, diagnostics: ()}"));
    let e = endpoint(&g, pins(&a));
    assert_eq!(g.inventory().unwrap().live_roots, 1);
    assert!(e
        .open(&b.bytes, &serde_json::to_string(&pins(&b)).unwrap())
        .is_err());
    assert!(e
        .open(&a.bytes, &serde_json::to_string(&pins(&b)).unwrap())
        .is_err());
    let mut changed = pins(&a);
    changed.result_profile.hash = "b".repeat(64);
    assert!(QuerySourceEndpoint::new(
        &g,
        Default::default(),
        changed,
        functions(),
        super::execution::profile(super::execution::profile_text())
    )
    .is_err());
    assert_eq!(g.inventory().unwrap().live_roots, 1);
    let s = e
        .open(&a.bytes, &serde_json::to_string(&pins(&a)).unwrap())
        .unwrap();
    assert_eq!(
        s.validate(&input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    assert_eq!(s.capture().unwrap(), a.bytes);
}
#[test]
fn endpoint_sessions_borrow_original_provisioning_and_keep_roots_until_drop() {
    let g = generation();
    let a = packet(&declaration("{accepted: true, diagnostics: ()}"));
    let p = serde_json::to_string(&pins(&a)).unwrap();
    let e = endpoint(&g, pins(&a));
    let s = e.open(&a.bytes, &p).unwrap();
    assert_eq!(g.inventory().unwrap().live_roots, 8);
    e.retire();
    assert!(e.open(&a.bytes, &p).is_err());
    assert!(s.capture().is_err());
    assert!(s.validate(&input("same"), Default::default()).is_err());
    assert_eq!(g.inventory().unwrap().live_roots, 8);
    drop(e);
    assert_eq!(g.inventory().unwrap().live_roots, 7);
    let replacement = endpoint(&g, pins(&a));
    assert!(
        s.capture().is_err(),
        "new equal deployment renewed an old session"
    );
    drop((s, replacement));
    assert_eq!(g.inventory().unwrap().live_roots, 0);
}
#[test]
fn endpoint_missing_functions_and_cancelled_provisioning_do_not_leak_sessions() {
    let g = generation();
    let a = packet(&declaration(
        "{accepted: str:length(value) > 0, diagnostics: ()}",
    ));
    let p = pins(&a);
    let text = serde_json::to_string(&p).unwrap();
    let e = QuerySourceEndpoint::new(
        &g,
        Default::default(),
        p.clone(),
        SelectedFunctionRegistry::new([]).unwrap(),
        super::execution::profile(super::execution::profile_text()),
    )
    .unwrap();
    assert!(e.open(&a.bytes, &text).is_err());
    assert_eq!(g.inventory().unwrap().live_roots, 1);
    let signal = cem_ml::scheduler::AbortSignal::new();
    let control = OperationControl::new(signal.clone());
    let cancelled = QuerySourceEndpoint::new(
        &g,
        control,
        p,
        functions(),
        super::execution::profile(super::execution::profile_text()),
    )
    .unwrap();
    signal.abort();
    assert!(cancelled.open(&a.bytes, &text).is_err());
    drop((e, cancelled));
    assert_eq!(g.inventory().unwrap().live_roots, 0);
}
#[test]
fn endpoint_live_session_limit_reuses_capacity_only_after_actual_drop() {
    let g = generation();
    let a = packet(&declaration("{accepted: true, diagnostics: ()}"));
    let p = serde_json::to_string(&pins(&a)).unwrap();
    let e = endpoint(&g, pins(&a));
    let mut sessions: Vec<_> = (0..32).map(|_| e.open(&a.bytes, &p).unwrap()).collect();
    assert!(e.open(&a.bytes, &p).is_err());
    assert_eq!(g.inventory().unwrap().live_roots, 225);
    drop(sessions.pop());
    let again = e.open(&a.bytes, &p).unwrap();
    drop((again, sessions, e));
    assert_eq!(g.inventory().unwrap().live_roots, 0);
}
#[test]
fn endpoint_generation_and_closed_admission_cleanup_keep_original_roots_visible() {
    let g = generation();
    let a = packet(&declaration("{accepted: true, diagnostics: ()}"));
    let p = serde_json::to_string(&pins(&a)).unwrap();
    let e = endpoint(&g, pins(&a));
    let s = e.open(&a.bytes, &p).unwrap();
    let drain = g
        .begin_drain(std::time::Duration::from_secs(5), Default::default())
        .unwrap();
    assert!(s.capture().is_err());
    assert!(s.validate(&input("same"), Default::default()).is_err());
    assert!(drain.seal().is_err());
    drop((s, e));
    assert!(drain.seal().unwrap().is_current(&g));
}
