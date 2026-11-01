use super::*;
use cem_ml::runtime_admission::{generation::NativeGeneration, NativeAdmission};
use cem_ql::{
    application_source_endpoint::{
        ApplicationBootstrap, ApplicationBootstrapOwner, ApplicationSourceEndpoint,
        PreparedApplication, RetainedApplicationOwner,
    },
    application_source_transport::{self as transport, VerifiedApplicationSources},
};
use std::sync::Mutex;

thread_local! {static BOOTSTRAP_ACTION:std::cell::RefCell<Option<Box<dyn Fn()>>>=const{std::cell::RefCell::new(None)};}

struct Bootstrap {
    mode: Arc<AtomicUsize>,
    input: Arc<Mutex<Option<(Arc<RetainedCemTree>, BTreeMap<String, u32>, Arc<State>)>>>,
}
impl ApplicationBootstrap for Bootstrap {
    fn prepare(
        &self,
        sources: Arc<VerifiedApplicationSources>,
        g: &Arc<NativeGeneration>,
        control: OperationControl,
    ) -> Result<PreparedApplication, String> {
        let mode = self.mode.load(Ordering::SeqCst);
        if mode == 1 {
            return Err("bootstrap refused".into());
        }
        let foreign;
        let g = if mode == 2 {
            foreign = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
            &foreign
        } else {
            g
        };
        let sources = if mode == 3 {
            transport::verify(sources.bytes(), sources.pin(), &control)?
        } else {
            sources
        };
        let control = if mode == 4 {
            OperationControl::default()
        } else {
            control
        };
        let p = fixture_sources_control(true, Some(g), Some(&sources), Some(control));
        *self.input.lock().unwrap() = Some((p.f.input.clone(), p.f.ids.clone(), p.f.state.clone()));
        let mut retained =
            p.f.rules
                .into_values()
                .map(RetainedApplicationOwner::Rule)
                .collect::<Vec<_>>();
        retained.extend(p.f.bindings.into_values().map(|b| match b {
            DatatypeAssemblyOperationBinding::Operation(o) => {
                RetainedApplicationOwner::Operation(o)
            }
            DatatypeAssemblyOperationBinding::Tokenizer(o) => {
                RetainedApplicationOwner::Tokenizer(o)
            }
        }));
        if mode == 5 {
            retained.retain(|o| !matches!(o, RetainedApplicationOwner::Rule(_)));
        }
        let result = PreparedApplication::stage(
            p.f.h,
            sources,
            p.registry,
            BTreeMap::new(),
            BTreeMap::from([(
                p.package.parent().roots()["scalar"].assembly.clone(),
                p.f.owner,
            )]),
            retained,
        );
        BOOTSTRAP_ACTION.with(|a| {
            if let Some(a) = a.borrow().as_ref() {
                a();
            }
        });
        result
    }
}
fn packet() -> transport::BuiltApplicationSources {
    let mut p = fixture_rule(true);
    let app = p.stage().unwrap();
    transport::capture(
        &p.f.h,
        &app,
        &super::source_handoff::inputs(&p),
        &p.f.control,
    )
    .unwrap()
}

#[test]
fn application_endpoint_reimports_real_consumers_and_fences_escaped_handles() {
    let bytes = packet();
    let g = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
    let control = OperationControl::default();
    let input = Arc::new(Mutex::new(None));
    let code = ApplicationBootstrapOwner::new(
        &g,
        control.clone(),
        bytes.pin.clone(),
        Bootstrap {
            mode: Arc::new(AtomicUsize::new(0)),
            input: input.clone(),
        },
    )
    .unwrap();
    let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
    let pin = serde_json::to_string(&bytes.pin).unwrap();
    let mut session = endpoint.open(&bytes.bytes, &pin).unwrap();
    assert_eq!(session.capture().unwrap(), bytes.bytes);
    let invocation = session
        .with_application(|app, host| {
            let input = input.lock().unwrap();
            let (tree, ids, _) = input.as_ref().unwrap();
            let items = mixed::ReceiverInput {
                source: tree.clone(),
                attribute: ids["items"],
            };
            let count = mixed::ReceiverInput {
                source: tree.clone(),
                attribute: ids["count"],
            };
            let receiver = app.receivers(host, "cem-pub-one")?;
            let report = receiver.validate_node(host, "items", &items)?;
            assert!(report.complete && !report.failed);
            assert!(receiver.validate_scalar(host, "count", &count)?.accepted);
            receiver.preparation_invocation(
                host,
                "choice",
                mixed::ReceiverInput {
                    source: tree.clone(),
                    attribute: ids["choice"],
                },
                Default::default(),
            )
        })
        .unwrap();
    session
        .with_application(|_, host| {
            assert!(invocation.prepare(host).unwrap().stopped.is_none());
            Ok(())
        })
        .unwrap();
    code.retire();
    assert!(session.capture().is_err());
    assert!(session
        .with_application(|_, host| invocation.prepare(host))
        .is_err());
    drop(session);
    drop(invocation);
    drop(endpoint);
    drop(code);
    assert!(g
        .begin_drain(std::time::Duration::from_secs(5), Default::default())
        .unwrap()
        .seal()
        .unwrap()
        .is_current(&g));
}

#[test]
fn application_endpoint_rejects_foreign_sources_contexts_missing_owners_and_preserves_prior_publication(
) {
    let bytes = packet();
    let g = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
    let input = Arc::new(Mutex::new(None));
    let mode = Arc::new(AtomicUsize::new(0));
    let code = ApplicationBootstrapOwner::new(
        &g,
        Default::default(),
        bytes.pin.clone(),
        Bootstrap {
            input: input.clone(),
            mode: mode.clone(),
        },
    )
    .unwrap();
    let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
    let pin = serde_json::to_string(&bytes.pin).unwrap();
    let mut session = endpoint.open(&bytes.bytes, &pin).unwrap();
    let old = session
        .with_application(|app, _| Ok(app.envelope().clone()))
        .unwrap();
    let before = g.inventory().unwrap().live_roots;
    for invalid in 1..=5 {
        mode.store(invalid, Ordering::SeqCst);
        assert!(
            session.replace(&bytes.bytes, &pin).is_err(),
            "mode {invalid}"
        );
        assert_eq!(session.capture().unwrap(), bytes.bytes);
        assert_eq!(
            g.inventory().unwrap().live_roots,
            before,
            "mode {invalid} leaked a root"
        );
        assert_eq!(g.inventory().unwrap().active_entries, 0);
    }
    mode.store(0, Ordering::SeqCst);
    let mut bad = bytes.pin.clone();
    bad.hash = hash(b"different");
    assert!(session
        .replace(&bytes.bytes, &serde_json::to_string(&bad).unwrap())
        .is_err());
    assert!(session.replace(&bytes.bytes[..20], &pin).is_err());
    session.replace(&bytes.bytes, &pin).unwrap();
    session
        .with_application(|app, _| {
            assert!(!Arc::ptr_eq(&old, app.envelope()));
            Ok(())
        })
        .unwrap();
    assert_eq!(session.capture().unwrap(), bytes.bytes);
}

#[test]
fn application_endpoint_withholds_callback_time_code_retirement_and_owner_drop() {
    for dropped in [false, true] {
        let bytes = packet();
        let g = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
        let code = ApplicationBootstrapOwner::new(
            &g,
            Default::default(),
            bytes.pin.clone(),
            Bootstrap {
                input: Arc::new(Mutex::new(None)),
                mode: Arc::new(AtomicUsize::new(0)),
            },
        )
        .unwrap();
        let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
        let holder = std::rc::Rc::new(std::cell::RefCell::new(Some(code)));
        let original = holder.clone();
        BOOTSTRAP_ACTION.with(|a| {
            *a.borrow_mut() = Some(Box::new(move || {
                if dropped {
                    original.borrow_mut().take();
                } else {
                    original.borrow().as_ref().unwrap().retire();
                }
            }))
        });
        assert!(endpoint
            .open(&bytes.bytes, &serde_json::to_string(&bytes.pin).unwrap())
            .is_err());
        BOOTSTRAP_ACTION.with(|a| *a.borrow_mut() = None);
        holder.borrow_mut().take();
        drop(endpoint);
        assert_eq!(g.inventory().unwrap().live_roots, 0);
        assert_eq!(g.inventory().unwrap().active_entries, 0);
    }
}

#[test]
fn application_endpoint_retirement_reaches_escaped_receivers_inside_native_calls() {
    let bytes = packet();
    let g = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
    let input = Arc::new(Mutex::new(None));
    let code = ApplicationBootstrapOwner::new(
        &g,
        Default::default(),
        bytes.pin.clone(),
        Bootstrap {
            input: input.clone(),
            mode: Arc::new(AtomicUsize::new(0)),
        },
    )
    .unwrap();
    let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
    let mut session = endpoint
        .open(&bytes.bytes, &serde_json::to_string(&bytes.pin).unwrap())
        .unwrap();
    assert!(session
        .with_application(|app, host| {
            let input = input.lock().unwrap();
            let (tree, ids, _) = input.as_ref().unwrap();
            let invocation = app.receivers(host, "cem-pub-one")?.preparation_invocation(
                host,
                "choice",
                mixed::ReceiverInput {
                    source: tree.clone(),
                    attribute: ids["choice"],
                },
                Default::default(),
            )?;
            let prepared = invocation.prepare(host)?;
            code.retire();
            assert!(invocation.prepare(host).is_err());
            assert!(app.receivers(host, "cem-pub-one").is_err());
            drop(prepared);
            Ok(())
        })
        .is_err());
}

#[test]
fn application_endpoint_rechecks_original_bootstrap_after_native_operation_callbacks() {
    let bytes = packet();
    let g = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
    let input = Arc::new(Mutex::new(None));
    let code = ApplicationBootstrapOwner::new(
        &g,
        Default::default(),
        bytes.pin.clone(),
        Bootstrap {
            input: input.clone(),
            mode: Arc::new(AtomicUsize::new(0)),
        },
    )
    .unwrap();
    let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
    let mut session = endpoint
        .open(&bytes.bytes, &serde_json::to_string(&bytes.pin).unwrap())
        .unwrap();
    input
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .2
        .mode
        .store(7, Ordering::SeqCst);
    let original = code.clone();
    let runtime = g.clone();
    ACTION.with(|a| {
        *a.borrow_mut() = Some(Box::new(move || {
            assert!(runtime.inventory().unwrap().active_entries > 0);
            original.retire();
        }))
    });
    assert!(session
        .with_application(|app, host| app.prepare_root(
            host,
            "scalar",
            &lexical("callback"),
            Default::default()
        ))
        .is_err());
    ACTION.with(|a| *a.borrow_mut() = None);
    assert!(session.capture().is_err());
}

#[test]
fn application_endpoint_owner_cancellation_and_closed_cleanup_remain_counted() {
    for cause in ["drop", "cancel", "drain"] {
        let bytes = packet();
        let g = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
        let control = OperationControl::default();
        let code = ApplicationBootstrapOwner::new(
            &g,
            control.clone(),
            bytes.pin.clone(),
            Bootstrap {
                input: Arc::new(Mutex::new(None)),
                mode: Arc::new(AtomicUsize::new(0)),
            },
        )
        .unwrap();
        let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
        let pin = serde_json::to_string(&bytes.pin).unwrap();
        let session = endpoint.open(&bytes.bytes, &pin).unwrap();
        if cause == "cancel" {
            control
                .cancel_root(Some("user cancellation".into()), None)
                .unwrap();
        }
        let code = if cause == "drop" {
            drop(code);
            None
        } else {
            Some(code)
        };
        let drain = g
            .begin_drain(std::time::Duration::from_secs(5), Default::default())
            .unwrap();
        assert!(session.capture().is_err());
        assert!(endpoint.open(&bytes.bytes, &pin).is_err());
        assert!(drain.seal().is_err());
        drop(session);
        drop(endpoint);
        drop(code);
        assert_eq!(g.inventory().unwrap().active_entries, 0);
        assert!(drain.seal().unwrap().is_current(&g));
    }
}

#[test]
fn application_endpoint_session_limits_and_failed_reservations_release_original_roots() {
    let bytes = packet();
    let g = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
    let code = ApplicationBootstrapOwner::new(
        &g,
        Default::default(),
        bytes.pin.clone(),
        Bootstrap {
            input: Arc::new(Mutex::new(None)),
            mode: Arc::new(AtomicUsize::new(0)),
        },
    )
    .unwrap();
    let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
    let pin = serde_json::to_string(&bytes.pin).unwrap();
    let mut sessions = vec![];
    for _ in 0..=cem_ql::application_source_endpoint::MAX_SESSIONS {
        match endpoint.open(&bytes.bytes, &pin) {
            Ok(s) => sessions.push(s),
            Err(_) => break,
        }
    }
    assert!(
        !sessions.is_empty() && sessions.len() <= cem_ql::application_source_endpoint::MAX_SESSIONS
    );
    let roots = g.inventory().unwrap().live_roots;
    assert!(endpoint.open(&bytes.bytes, &pin).is_err());
    assert_eq!(g.inventory().unwrap().live_roots, roots);
    sessions.pop();
    let replacement = endpoint.open(&bytes.bytes, &pin).unwrap();
    drop(replacement);
    drop(sessions);
    drop(endpoint);
    drop(code);
    assert_eq!(g.inventory().unwrap().live_roots, 0);
}
