use cem_ml::{scheduler::AbortSignal, stylesheet_execution::*};
use std::sync::Arc;
#[path = "support/stylesheet_fixture.rs"]
mod fixture;
use fixture::*;
struct Observer {
    revision: u64,
    wrong: bool,
    change: bool,
}
impl StylesheetExecutionObserver for Observer {
    fn environment_revision(&mut self) -> Result<String, String> {
        Ok(self.revision.to_string())
    }
    fn observe(
        &mut self,
        slot: StylesheetExecutionSlot<'_>,
    ) -> Result<StylesheetObservation, String> {
        if self.change {
            self.revision += 1;
        }
        Ok(StylesheetObservation {
            selected: !slot.disabled,
            conditions_match: true,
            loaded_source: Some(if self.wrong {
                "wrong".into()
            } else {
                slot.source_key.into()
            }),
            installation: Some((slot.root * 100 + slot.sheet + 1) as u64),
        })
    }
}
#[test]
fn html_execution_records_exact_loaded_occurrences_and_rejects_stale_or_foreign_hosts() {
    let (mut html, document, cap) =
        setup("<style>p{color:red}</style><link rel='stylesheet' href='theme.css'/>");
    let snapshot = snapshot();
    let css = Arc::new(
        html.inspect_css_imports(
            document,
            &cap,
            snapshot.clone(),
            Default::default(),
            AbortSignal::new(),
            slots,
            BASE,
        )
        .unwrap(),
    );
    let mut host = StylesheetExecutionHost::default();
    let mut observer = Observer {
        revision: 1,
        wrong: false,
        change: false,
    };
    let evidence = css
        .record_execution(
            &mut html,
            &cap,
            &snapshot,
            &mut host,
            &mut observer,
            100,
            slots,
            BASE,
        )
        .unwrap();
    assert_eq!(evidence.execution().observations().len(), 3);
    assert!(!evidence
        .blockers()
        .iter()
        .any(|b| b == "stylesheet-execution-history-required"));
    assert!(evidence
        .blockers()
        .iter()
        .any(|b| b == "atomic-host-transaction-required"));
    evidence
        .validate(
            &mut html,
            &cap,
            &snapshot,
            &mut host,
            &mut observer,
            slots,
            BASE,
        )
        .unwrap();
    assert!(evidence
        .validate(
            &mut html,
            &cap,
            &snapshot,
            &mut StylesheetExecutionHost::default(),
            &mut observer,
            slots,
            BASE
        )
        .is_err());
    host.invalidate().unwrap();
    assert!(evidence
        .validate(
            &mut html,
            &cap,
            &snapshot,
            &mut host,
            &mut observer,
            slots,
            BASE
        )
        .is_err());
}
#[test]
fn incomplete_loads_drift_duplicate_installations_and_bounds_cannot_publish() {
    for mode in ["wrong", "drift", "budget"] {
        let (mut html, document, cap) = setup("<link rel='stylesheet' href='theme.css'/>");
        let snapshot = snapshot();
        let css = Arc::new(
            html.inspect_css_imports(
                document,
                &cap,
                snapshot.clone(),
                Default::default(),
                AbortSignal::new(),
                slots,
                BASE,
            )
            .unwrap(),
        );
        let mut host = StylesheetExecutionHost::default();
        let mut observer = Observer {
            revision: 1,
            wrong: mode == "wrong",
            change: mode == "drift",
        };
        assert!(css
            .record_execution(
                &mut html,
                &cap,
                &snapshot,
                &mut host,
                &mut observer,
                if mode == "budget" { 1 } else { 100 },
                slots,
                BASE
            )
            .is_err());
    }
}
#[test]
fn changed_observations_revoke_evidence_even_after_restoration() {
    let (mut html, document, cap) = setup("<link rel='stylesheet' disabled='' href='theme.css'/>");
    let snapshot = snapshot();
    let css = Arc::new(
        html.inspect_css_imports(
            document,
            &cap,
            snapshot.clone(),
            Default::default(),
            AbortSignal::new(),
            slots,
            BASE,
        )
        .unwrap(),
    );
    let mut host = StylesheetExecutionHost::default();
    let mut observer = Observer {
        revision: 1,
        wrong: false,
        change: false,
    };
    let evidence = css
        .record_execution(
            &mut html,
            &cap,
            &snapshot,
            &mut host,
            &mut observer,
            100,
            slots,
            BASE,
        )
        .unwrap();
    observer.revision = 2;
    assert!(evidence
        .validate(
            &mut html,
            &cap,
            &snapshot,
            &mut host,
            &mut observer,
            slots,
            BASE
        )
        .is_err());
    observer.revision = 1;
    assert!(evidence
        .validate(
            &mut html,
            &cap,
            &snapshot,
            &mut host,
            &mut observer,
            slots,
            BASE
        )
        .is_err());
}

#[test]
fn incomplete_or_contradictory_observations_and_duplicate_handles_are_rejected() {
    struct Invalid(&'static str);
    impl StylesheetExecutionObserver for Invalid {
        fn environment_revision(&mut self) -> Result<String, String> {
            Ok("1".into())
        }
        fn observe(
            &mut self,
            slot: StylesheetExecutionSlot<'_>,
        ) -> Result<StylesheetObservation, String> {
            if self.0 == "error" {
                return Err("observer failed".into());
            }
            Ok(StylesheetObservation {
                selected: self.0 != "orphan" || slot.sheet != 0,
                conditions_match: self.0 != "parent-condition" || slot.sheet != 0,
                loaded_source: if self.0 == "unloaded" {
                    None
                } else {
                    Some(slot.source_key.into())
                },
                installation: Some(if self.0 == "duplicate" {
                    1
                } else {
                    slot.sheet as u64 + 1
                }),
            })
        }
    }
    let (mut html, document, cap) = setup("<link rel='stylesheet' href='theme.css'/>");
    let snapshot = snapshot();
    let css = Arc::new(
        html.inspect_css_imports(
            document,
            &cap,
            snapshot.clone(),
            Default::default(),
            AbortSignal::new(),
            slots,
            BASE,
        )
        .unwrap(),
    );
    for mode in [
        "error",
        "unloaded",
        "duplicate",
        "orphan",
        "parent-condition",
    ] {
        assert!(
            css.record_execution(
                &mut html,
                &cap,
                &snapshot,
                &mut StylesheetExecutionHost::default(),
                &mut Invalid(mode),
                100,
                slots,
                BASE
            )
            .is_err(),
            "{mode}"
        );
    }
}
#[test]
fn observation_drift_and_root_metadata_substitution_revoke_the_host() {
    let (mut html, document, cap) = setup("<link rel='stylesheet' href='theme.css'/>");
    let snapshot = snapshot();
    let css = html
        .inspect_css_imports(
            document,
            &cap,
            snapshot,
            Default::default(),
            AbortSignal::new(),
            slots,
            BASE,
        )
        .unwrap();
    let mut host = StylesheetExecutionHost::default();
    let mut observer = Observer {
        revision: 1,
        wrong: false,
        change: false,
    };
    let mut roots = css.execution_roots();
    let evidence = host.capture(&roots, &mut observer, 100).unwrap();
    roots[0].media = "print".into();
    assert!(host.validate(&evidence, &roots, &mut observer).is_err());
    roots[0].media.clear();
    assert!(host.validate(&evidence, &roots, &mut observer).is_err());
    let evidence = host.capture(&roots, &mut observer, 100).unwrap();
    observer.wrong = true;
    assert!(host.validate(&evidence, &roots, &mut observer).is_err());
    observer.wrong = false;
    assert!(host.validate(&evidence, &roots, &mut observer).is_err());
}
