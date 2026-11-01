use super::*;
use cem_ml::runtime_admission::{generation::NativeGeneration, NativeAdmission};
use cem_ql::{
    native_registrations as nr,
    rule_source_endpoint::RuleSourceEndpoint,
    rule_source_transport::{self as t, RuleOrigin, RuleSource},
    structural_receiver::FreshNativeRuleCodeOwner,
};
#[derive(Debug)]
pub(super) struct Validator;
impl cem_ql::datatype_validation::NativeDatatypeValidator for Validator {
    fn validate(
        &self,
        call: cem_ql::datatype_validation::ValidationCall<'_>,
    ) -> cem_ql::datatype_validation::RuleExecution {
        cem_ql::datatype_validation::RuleExecution::Complete(cem_ql::eval::ItemStream::once(
            Item::Record(BTreeMap::from([
                (
                    "accepted".into(),
                    vec![Item::Atomic(AtomValue::Boolean(
                        call.value[0].atom() == Some(AtomValue::String("same".into())),
                    ))],
                ),
                ("diagnostics".into(), vec![]),
            ])),
        ))
    }
}
pub(super) fn native_identity() -> nr::ImplementationIdentity {
    nr::ImplementationIdentity::validator(
        "urn:test:validate",
        cem_ml::content_cache::ContentHash::from_blake3(b"native-rule-fixture").header_value(),
        "validate",
        contract(&source(&declaration("{accepted: true, diagnostics: ()}")))
            .signature()
            .clone(),
    )
}
pub(super) fn packet(native: bool, imported: bool) -> t::BuiltRuleTransport {
    let profile = super::profile_transport::custom_profile(false);
    let source = if native {
        let text = "@ns schema = https://cem.dev/ns/schema/1\n@default schema\n{schema @namespace=urn:example | {types | {type @name=sample @kind=scalar}} {behaviors | {behavior @name=check @implementation=engine @primitive=urn:test:validate @execution=datatype-validation | {inputs | {input-binding @name=value @type=schema:string @source=value @required=true @cardinality=one} {input-binding @name=datatype @type=schema:node @source=datatype @required=true @cardinality=one} {input-binding @name=candidate @type=schema:node @source=candidate @required=false @cardinality=zero-or-one}} {result @type=schema:datatype-validation-result}}}}";
        let capture = capture(text);
        let c = contract(&from_capture(capture.clone()));
        let b = nr::export_validator(
            &capture,
            &c,
            identity(text),
            native_identity(),
            &registry_hash(),
            &Default::default(),
        )
        .unwrap();
        RuleSource::Native(Arc::new(
            nr::verify(&b.bytes, &b.pin, &Default::default()).unwrap(),
        ))
    } else {
        RuleSource::Query(verified(&declaration(
            "{accepted: str:length(value) > 2, diagnostics: ()}",
        )))
    };
    let (id, module) = match &source {
        RuleSource::Query(q) => (datatype(q), q.pin().hash.clone()),
        RuleSource::Native(n) => (
            n.source()
                .ast()
                .nodes
                .iter()
                .find_map(|node| match node {
                    CemAstNode::Element {
                        node_id,
                        expanded_name,
                        ..
                    } if expanded_name.local_name == "type" => Some(*node_id),
                    _ => None,
                })
                .unwrap(),
            n.pin().hash.clone(),
        ),
    };
    t::export(
        &source,
        if imported { None } else { Some(id) },
        if imported {
            RuleOrigin::Imported { module }
        } else {
            RuleOrigin::Direct
        },
        &profile,
        &Default::default(),
    )
    .unwrap()
}
fn generation() -> Arc<NativeGeneration> {
    NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap()
}
#[test]
fn custom_query_and_native_rule_packets_restore_with_explicit_code_and_original_sources() {
    for native in [false, true] {
        for imported in [false, true] {
            let g = generation();
            let b = packet(native, imported);
            let fresh = t::verify(&b.bytes, &b.pin, &Default::default()).unwrap();
            assert_eq!(fresh.profile().sources().len(), 2);
            let code = if native {
                Some(
                    FreshNativeRuleCodeOwner::new(
                        &g,
                        Default::default(),
                        native_identity(),
                        Validator,
                    )
                    .unwrap(),
                )
            } else {
                None
            };
            let endpoint = RuleSourceEndpoint::new(
                &g,
                Default::default(),
                b.pin.clone(),
                functions(),
                code.as_ref(),
            )
            .unwrap();
            let session = endpoint
                .open(&b.bytes, &serde_json::to_string(&b.pin).unwrap())
                .unwrap();
            assert_eq!(session.capture().unwrap(), b.bytes);
            if imported {
                assert!(session
                    .validate(&input("same"), Default::default())
                    .is_err());
            } else {
                assert_eq!(
                    session
                        .validate(&input("same"), Default::default())
                        .unwrap()
                        .accepted,
                    Some(true)
                );
            }
            let mut changed = b.pin.clone();
            changed.profile.profile.diagnostic.local.push('x');
            assert!(endpoint
                .open(&b.bytes, &serde_json::to_string(&changed).unwrap())
                .is_err());
            assert_eq!(
                session.capture().unwrap(),
                b.bytes,
                "failed import changed the original session"
            );
            if let Some(code) = &code {
                code.retire();
                assert!(session.capture().is_err());
            }
            drop((session, endpoint, code));
            assert_eq!(g.inventory().unwrap().live_roots, 0);
        }
    }
}
#[test]
fn unavailable_foreign_or_dropped_native_code_does_not_authorize_rule_transport() {
    let g = generation();
    let b = packet(true, false);
    let pins = serde_json::to_string(&b.pin).unwrap();
    let unavailable =
        RuleSourceEndpoint::new(&g, Default::default(), b.pin.clone(), functions(), None).unwrap();
    assert!(unavailable.open(&b.bytes, &pins).is_err());
    let foreign = generation();
    let code =
        FreshNativeRuleCodeOwner::new(&foreign, Default::default(), native_identity(), Validator)
            .unwrap();
    let e = RuleSourceEndpoint::new(
        &g,
        Default::default(),
        b.pin.clone(),
        functions(),
        Some(&code),
    )
    .unwrap();
    assert!(e.open(&b.bytes, &pins).is_err());
    drop((e, code));
    let code = FreshNativeRuleCodeOwner::new(&g, Default::default(), native_identity(), Validator)
        .unwrap();
    let e = RuleSourceEndpoint::new(
        &g,
        Default::default(),
        b.pin.clone(),
        functions(),
        Some(&code),
    )
    .unwrap();
    let s = e.open(&b.bytes, &pins).unwrap();
    drop(code);
    assert!(s.capture().is_err());
    drop((s, e, unavailable));
    assert_eq!(g.inventory().unwrap().live_roots, 0);
}
#[test]
fn native_callback_retirement_and_closed_cleanup_cannot_publish_a_successful_result() {
    use std::sync::{Mutex, Weak};
    #[derive(Debug)]
    struct Retiring(Arc<Mutex<Weak<FreshNativeRuleCodeOwner>>>);
    impl cem_ql::datatype_validation::NativeDatatypeValidator for Retiring {
        fn validate(
            &self,
            call: cem_ql::datatype_validation::ValidationCall<'_>,
        ) -> cem_ql::datatype_validation::RuleExecution {
            self.0.lock().unwrap().upgrade().unwrap().retire();
            cem_ql::datatype_validation::NativeDatatypeValidator::validate(&Validator, call)
        }
    }
    let g = generation();
    let b = packet(true, false);
    let reference = Arc::new(Mutex::new(Weak::new()));
    let code = FreshNativeRuleCodeOwner::new(
        &g,
        Default::default(),
        native_identity(),
        Retiring(reference.clone()),
    )
    .unwrap();
    *reference.lock().unwrap() = Arc::downgrade(&code);
    let e = RuleSourceEndpoint::new(
        &g,
        Default::default(),
        b.pin.clone(),
        functions(),
        Some(&code),
    )
    .unwrap();
    let s = e
        .open(&b.bytes, &serde_json::to_string(&b.pin).unwrap())
        .unwrap();
    assert!(s.validate(&input("same"), Default::default()).is_err());
    assert!(s.capture().is_err());
    let drain = g
        .begin_drain(std::time::Duration::from_secs(5), Default::default())
        .unwrap();
    assert!(drain.seal().is_err());
    drop((s, e, code));
    assert!(drain.seal().unwrap().is_current(&g));
}
#[test]
fn emit_checked_rule_profile_fixtures_for_real_worker_fallback_transport() {
    if let Some(path) = std::env::var_os("CEM_QUERY_HANDOFF_FIXTURE_DIR") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        for native in [false, true] {
            for imported in [false, true] {
                let p = packet(native, imported);
                let name = format!(
                    "rule-{}-{}",
                    if native { "native" } else { "query" },
                    if imported { "imported" } else { "direct" }
                );
                std::fs::write(path.join(format!("{name}.bin")), p.bytes).unwrap();
                std::fs::write(
                    path.join(format!("{name}.json")),
                    serde_json::to_vec(&p.pin).unwrap(),
                )
                .unwrap();
            }
        }
    }
}
