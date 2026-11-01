use super::*;
use cem_ml::{
    runtime_admission::{generation::NativeGeneration, NativeAdmission},
    schema::native_source::SourceIdentity,
};
use cem_ql::query_validator_capsule::profiles::{self, QueryResultProfileSource};

pub(super) fn profile_text() -> &'static str {
    cem_ml::schema::package_sources::builtin_schema_package_source("schema")
        .unwrap()
        .schema_source
}
pub(super) fn profile(text: &str) -> Arc<profiles::VerifiedQueryResultProfile> {
    let captured = capture(text);
    let tree = RetainedCemTree::from_shared(
        captured.document().clone(),
        "https://example.test/result-profile.cem",
        text,
        Default::default(),
        None,
    )
    .unwrap();
    profiles::build(
        vec![QueryResultProfileSource {
            tree,
            capture: captured.clone(),
            contracts: from_capture(captured),
            identity: SourceIdentity::new(
                "https://example.test/result-profile.cem",
                SourceId(1),
                text.as_bytes(),
            ),
        }],
        ContractName::new(CEM_SCHEMA_URI, "datatype-validation-result"),
        ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"),
        &Default::default(),
    )
    .unwrap()
}
fn generation() -> Arc<NativeGeneration> {
    NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap()
}
fn register_profile(
    h: &mut FreshStructuralHost,
    p: &profiles::VerifiedQueryResultProfile,
) -> Vec<cem_ql::schema_references::DeclarationScope> {
    p.sources()
        .iter()
        .map(|s| {
            h.register_captured_scope(
                s.tree.clone(),
                s.capture.clone(),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap()
        })
        .collect()
}
#[test]
fn source_backed_profile_pins_bind_complete_original_sources_and_shapes() {
    let a = profile(profile_text());
    let b = profile(profile_text());
    assert_eq!(a.pin(), b.pin());
    assert!(!Arc::ptr_eq(
        a.sources()[0].tree.ast_owner(),
        b.sources()[0].tree.ast_owner()
    ));
    let changed = profile(&format!(
        "{}\n// a different complete source",
        profile_text()
    ));
    assert_ne!(a.pin(), changed.pin());
    let changed_shape = profile(&profile_text().replace(
        "{value-contract @name=\"datatype-validation-result\" |",
        "{value-contract @name=\"datatype-validation-result\" @allow-extra=true |",
    ));
    assert_ne!(a.pin(), changed_shape.pin());
}

#[test]
fn profile_build_checks_cancellation_and_original_capture_owners() {
    let p = profile(profile_text());
    let original = &p.sources()[0];
    let signal = cem_ml::scheduler::AbortSignal::new();
    let control = OperationControl::new(signal.clone());
    signal.abort();
    let copy = || QueryResultProfileSource {
        tree: original.tree.clone(),
        capture: original.capture.clone(),
        contracts: original.contracts.clone(),
        identity: original.identity.clone(),
    };
    assert!(profiles::build(
        vec![copy()],
        p.pin().result.clone(),
        p.pin().diagnostic.clone(),
        &control
    )
    .is_err());
    let other = profile(profile_text());
    let mut wrong = copy();
    wrong.capture = other.sources()[0].capture.clone();
    assert!(profiles::build(
        vec![wrong],
        p.pin().result.clone(),
        p.pin().diagnostic.clone(),
        &Default::default()
    )
    .is_err());
    let mut wrong = copy();
    wrong.identity.id = SourceId(999);
    assert!(profiles::build(
        vec![wrong],
        p.pin().result.clone(),
        p.pin().diagnostic.clone(),
        &Default::default()
    )
    .is_err());
    assert!(profiles::build(
        vec![],
        p.pin().result.clone(),
        p.pin().diagnostic.clone(),
        &Default::default()
    )
    .is_err());
}

#[test]
fn profile_retirement_and_closed_generations_leave_retained_bindings_counted() {
    let g = generation();
    let mut h =
        FreshStructuralHost::new_in_generation(&registry_hash(), Default::default(), &g).unwrap();
    let p = profile(profile_text());
    let scopes = register_profile(&mut h, &p);
    let d = verified(&declaration("{accepted: true, diagnostics: ()}"));
    let scope = register(&mut h, &d);
    let source = h.query_source(d.clone(), scope).unwrap();
    let code = h.query_code_owner(&source, &functions()).unwrap();
    let profile = h
        .query_result_profile_owner(p.clone(), &scopes, p.pin())
        .unwrap();
    let registry = h.query_registry_owner(&source, &code, &profile).unwrap();
    let binding = h
        .link_query_owned(&source, datatype(&d), &registry)
        .unwrap();
    let call = h.query_invocation(scope).unwrap();
    profile.retire();
    assert!(!registry.is_live_on(&h));
    assert!(h
        .validate_query(&call, &binding, &input("same"), Default::default())
        .is_err());
    assert_eq!(g.inventory().unwrap().live_roots, 7);
    let drain = g
        .begin_drain(std::time::Duration::from_secs(5), Default::default())
        .unwrap();
    assert!(drain.seal().is_err());
    drop((profile, code, registry, source, binding, call, h));
    assert!(drain.seal().unwrap().is_current(&g));
}
#[test]
fn query_bindings_require_original_code_registry_and_profile_owners_until_actual_drop() {
    let g = generation();
    let mut h =
        FreshStructuralHost::new_in_generation(&registry_hash(), Default::default(), &g).unwrap();
    let d = verified(&declaration(
        "{accepted: str:length(value) > 0, diagnostics: ()}",
    ));
    let p = profile(profile_text());
    let scopes = register_profile(&mut h, &p);
    let scope = register(&mut h, &d);
    let source = h.query_source(d.clone(), scope).unwrap();
    let code = h.query_code_owner(&source, &functions()).unwrap();
    let result = h
        .query_result_profile_owner(p.clone(), &scopes, p.pin())
        .unwrap();
    let registry = h.query_registry_owner(&source, &code, &result).unwrap();
    let binding = h
        .link_query_owned(&source, datatype(&d), &registry)
        .unwrap();
    let call = h.query_invocation(scope).unwrap();
    assert_eq!(g.inventory().unwrap().live_roots, 7);
    assert_eq!(
        h.validate_query(&call, &binding, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    drop(code);
    assert!(h
        .validate_query(&call, &binding, &input("same"), Default::default())
        .is_err());
    assert!(h.capture_query_handoff(&binding).is_err());
    assert_eq!(g.inventory().unwrap().live_roots, 6);
    result.retire();
    registry.retire();
    binding.retire();
    call.retire();
    h.close();
    assert_eq!(g.inventory().unwrap().live_roots, 6);
    drop((result, registry, binding, call, source, h));
    assert_eq!(g.inventory().unwrap().live_roots, 0);
}
#[test]
fn profile_pins_foreign_sources_and_missing_code_refuse_before_linking() {
    let g = generation();
    let mut h =
        FreshStructuralHost::new_in_generation(&registry_hash(), Default::default(), &g).unwrap();
    let p = profile(profile_text());
    let other = profile(profile_text());
    let scopes = register_profile(&mut h, &p);
    let d = verified(&declaration(
        "{accepted: str:length(value) > 0, diagnostics: ()}",
    ));
    let scope = register(&mut h, &d);
    let source = h.query_source(d.clone(), scope).unwrap();
    assert!(h
        .query_code_owner(&source, &SelectedFunctionRegistry::new([]).unwrap())
        .is_err());
    assert!(h
        .query_result_profile_owner(other, &scopes, p.pin())
        .is_err());
    let changed = profile(&format!("{}\n// changed", profile_text()));
    assert!(h
        .query_result_profile_owner(p.clone(), &scopes, changed.pin())
        .is_err());
    assert_eq!(g.inventory().unwrap().live_roots, 2);
    let code = h.query_code_owner(&source, &functions()).unwrap();
    let result = h
        .query_result_profile_owner(p.clone(), &scopes, p.pin())
        .unwrap();
    let copied_source = h.query_source(d.clone(), scope).unwrap();
    assert!(h
        .query_registry_owner(&copied_source, &code, &result)
        .is_err());
    let registry = h.query_registry_owner(&source, &code, &result).unwrap();
    assert!(h
        .link_query_owned(&copied_source, datatype(&d), &registry)
        .is_err());
    let other_generation = generation();
    let foreign = FreshStructuralHost::new_in_generation(
        &registry_hash(),
        Default::default(),
        &other_generation,
    )
    .unwrap();
    assert!(!registry.is_live_on(&foreign));
}
#[test]
fn owner_replacement_stages_before_retirement_and_context_changes_fence_every_owner() {
    let g = generation();
    let mut h =
        FreshStructuralHost::new_in_generation(&registry_hash(), Default::default(), &g).unwrap();
    let p = profile(profile_text());
    let scopes = register_profile(&mut h, &p);
    let d = verified(&declaration("{accepted: true, diagnostics: ()}"));
    let scope = register(&mut h, &d);
    let source = h.query_source(d.clone(), scope).unwrap();
    let code = h.query_code_owner(&source, &functions()).unwrap();
    let result = h
        .query_result_profile_owner(p.clone(), &scopes, p.pin())
        .unwrap();
    let registry = h.query_registry_owner(&source, &code, &result).unwrap();
    let first = h
        .link_query_owned(&source, datatype(&d), &registry)
        .unwrap();
    let call = h.query_invocation(scope).unwrap();
    assert!(h
        .replace_query_owned(&first, &source, 0, &registry)
        .is_err());
    assert_eq!(
        h.validate_query(&call, &first, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    let next = h
        .replace_query_owned(&first, &source, datatype(&d), &registry)
        .unwrap();
    assert!(h
        .validate_query(&call, &first, &input("same"), Default::default())
        .is_err());
    assert_eq!(
        h.validate_query(&call, &next, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    assert!(h.set_context(scope, Some(Default::default())));
    assert!(!registry.is_live_on(&h));
    assert!(h
        .validate_query(&call, &next, &input("same"), Default::default())
        .is_err());
}
