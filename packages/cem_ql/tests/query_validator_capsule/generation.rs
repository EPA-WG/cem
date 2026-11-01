use super::*;
use cem_ml::runtime_admission::{generation::NativeGeneration, NativeAdmission, NativeRootKind};
use std::time::Duration;
fn generation() -> Arc<NativeGeneration> {
    NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap()
}
fn enrolled(g: &Arc<NativeGeneration>) -> FreshStructuralHost {
    FreshStructuralHost::new_in_generation(&registry_hash(), Default::default(), g).unwrap()
}
#[test]
fn query_owners_count_original_source_binding_and_request_until_actual_drop() {
    let g = generation();
    let d = verified(&declaration(
        "{accepted: str:length(value) > 0, diagnostics: ()}",
    ));
    let mut h = enrolled(&g);
    let scope = register(&mut h, &d);
    let source = h.query_source(d.clone(), scope).unwrap();
    let binding = h
        .link_query_source(&source, datatype(&d), &functions(), adapter())
        .unwrap();
    let call = h.query_invocation(scope).unwrap();
    assert_eq!(
        h.validate_query(&call, &binding, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    let i = g.inventory().unwrap();
    for kind in [
        NativeRootKind::Host,
        NativeRootKind::Compilation,
        NativeRootKind::Registry,
        NativeRootKind::Preparation,
    ] {
        assert_eq!(i.roots[&kind], 1, "{kind:?}");
    }
    source.retire();
    binding.retire();
    call.retire();
    h.close();
    assert_eq!(g.inventory().unwrap().live_roots, 4);
    let drain = g
        .begin_drain(Duration::from_secs(5), Default::default())
        .unwrap();
    drop(h);
    assert_eq!(drain.poll().unwrap().live_roots, 3);
    drop(source);
    assert_eq!(drain.poll().unwrap().live_roots, 2);
    drop(binding);
    assert_eq!(drain.poll().unwrap().live_roots, 1);
    assert!(drain.seal().is_err());
    drop(call);
    assert!(drain.seal().unwrap().is_current(&g));
}
#[test]
fn explicit_query_source_loss_and_context_changes_fence_bindings_without_reconstructing_owners() {
    let g = generation();
    let d = verified(&declaration("{accepted: true, diagnostics: ()}"));
    let mut h = enrolled(&g);
    let scope = register(&mut h, &d);
    let source = h.query_source(d.clone(), scope).unwrap();
    let binding = h
        .link_query_source(&source, datatype(&d), &functions(), adapter())
        .unwrap();
    let call = h.query_invocation(scope).unwrap();
    drop(source);
    assert!(h
        .validate_query(&call, &binding, &input("same"), Default::default())
        .is_err());
    let source = h.query_source(d.clone(), scope).unwrap();
    assert!(h.set_context(scope, Some(Default::default())));
    assert!(h
        .link_query_source(&source, datatype(&d), &functions(), adapter())
        .is_err());
    let source = h.query_source(d.clone(), scope).unwrap();
    let fresh = h
        .link_query_source(&source, datatype(&d), &functions(), adapter())
        .unwrap();
    assert!(h
        .validate_query(&call, &fresh, &input("same"), Default::default())
        .is_err());
    let call = h.query_invocation(scope).unwrap();
    assert_eq!(
        h.validate_query(&call, &fresh, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    call.retire();
    assert!(h
        .validate_query(&call, &fresh, &input("same"), Default::default())
        .is_err());
}
#[test]
fn query_replacement_is_atomic_and_reimported_bytes_do_not_transfer_old_source_or_call_authority() {
    let text = declaration("{accepted: str:length(value) > 0, diagnostics: ()}");
    let raw = built(&text);
    let old = generation();
    let mut h = enrolled(&old);
    let d = Arc::new(portable::verify(&raw.bytes, &raw.pin, &Default::default()).unwrap());
    let scope = register(&mut h, &d);
    let source = h.query_source(d.clone(), scope).unwrap();
    let binding = h
        .link_query_source(&source, datatype(&d), &functions(), adapter())
        .unwrap();
    let call = h.query_invocation(scope).unwrap();
    assert!(h
        .replace_query_source(
            &binding,
            &source,
            datatype(&d),
            &SelectedFunctionRegistry::new([]).unwrap(),
            adapter()
        )
        .is_err());
    assert_eq!(
        h.validate_query(&call, &binding, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    let next = h
        .replace_query_source(&binding, &source, datatype(&d), &functions(), adapter())
        .unwrap();
    assert!(h
        .validate_query(&call, &binding, &input("same"), Default::default())
        .is_err());
    let fresh = generation();
    let mut other = enrolled(&fresh);
    let restored = Arc::new(portable::verify(&raw.bytes, &raw.pin, &Default::default()).unwrap());
    assert!(!Arc::ptr_eq(
        d.source().ast_owner(),
        restored.source().ast_owner()
    ));
    let scope = register(&mut other, &restored);
    assert!(other
        .link_query_source(&source, datatype(&restored), &functions(), adapter())
        .is_err());
    let owned = other.query_source(restored.clone(), scope).unwrap();
    let linked = other
        .link_query_source(&owned, datatype(&restored), &functions(), adapter())
        .unwrap();
    let request = other.query_invocation(scope).unwrap();
    assert!(other
        .validate_query(&call, &linked, &input("same"), Default::default())
        .is_err());
    assert!(other
        .validate_query(&request, &next, &input("same"), Default::default())
        .is_err());
    let _drain = old
        .begin_drain(Duration::from_secs(5), Default::default())
        .unwrap();
    assert!(h
        .validate_query(&call, &next, &input("same"), Default::default())
        .is_err());
    assert_eq!(
        other
            .validate_query(&request, &linked, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
}
#[test]
fn legacy_query_convenience_enrolls_its_implicit_source_and_refuses_closed_or_lost_generations() {
    let g = generation();
    let d = verified(&declaration("{accepted: true, diagnostics: ()}"));
    let mut h = enrolled(&g);
    let scope = register(&mut h, &d);
    let binding = h
        .link_query(d.clone(), scope, datatype(&d), &functions(), adapter())
        .unwrap();
    let call = h.query_invocation(scope).unwrap();
    assert_eq!(g.inventory().unwrap().live_roots, 4);
    assert_eq!(
        h.validate_query(&call, &binding, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    drop(g);
    assert!(h
        .validate_query(&call, &binding, &input("same"), Default::default())
        .is_err());
    assert!(h.query_source(d, scope).is_err());
}

#[test]
fn query_owner_limits_refuse_replacement_without_retiring_the_current_binding() {
    let g = generation();
    let d = verified(&declaration("{accepted: true, diagnostics: ()}"));
    let mut h = enrolled(&g);
    let scope = register(&mut h, &d);
    let source = h.query_source(d.clone(), scope).unwrap();
    let binding = h
        .link_query_source(&source, datatype(&d), &functions(), adapter())
        .unwrap();
    let call = h.query_invocation(scope).unwrap();
    let held = (0..252)
        .map(|_| g.resource(NativeRootKind::Evidence).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(g.inventory().unwrap().live_roots, 256);
    assert!(h
        .replace_query_source(&binding, &source, datatype(&d), &functions(), adapter())
        .is_err());
    assert_eq!(g.inventory().unwrap().live_roots, 256);
    assert_eq!(g.inventory().unwrap().active_entries, 0);
    assert_eq!(
        h.validate_query(&call, &binding, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    drop(held);
    assert!(h
        .replace_query_source(&binding, &source, datatype(&d), &functions(), adapter())
        .is_ok());
}
