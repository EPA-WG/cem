use cem_ml::{
    css_emission::CssManagedScope, css_imports::CssImportClosure, module_resolution::*,
    scheduler::AbortSignal,
};
use cem_ql::{
    render::{adopt_dom_stylesheets, DomStylesheetSource},
    retained_template::RetainedTemplate,
    stylesheet_installation::{
        CompiledStylesheetHost, CompiledStylesheetObservation, CompiledStylesheetObserver,
        CompiledStylesheetSlot,
    },
};
use std::sync::Arc;
fn owner() -> RetainedTemplate {
    RetainedTemplate::new(adopt_dom_stylesheets(&[DomStylesheetSource {
        css: "p{color:red}".into(),
        scope: None,
        content_type: None,
    }]))
}
fn retain(owner: &mut RetainedTemplate, consumer: &str) -> AbortSignal {
    let context = CemResolutionContextHandle::new("fixture");
    let cap = CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(
            context.clone(),
            CemModuleUrlContext {
                identity: "fixture".into(),
                resolver_identity: "native".into(),
                resource_policy_stamp: "1".into(),
                frames: vec![CemModuleUrlFrame::new(
                    "fixture",
                    "https://example.test/body.cemt",
                )],
            },
        )),
        context,
    );
    let abort = AbortSignal::new();
    let closure = Arc::new(
        CssImportClosure::new(
            owner.artifact().stylesheets[0].css_tree().clone(),
            "https://example.test/body.cemt",
            cap,
            Default::default(),
            abort.clone(),
        )
        .unwrap(),
    );
    owner
        .retain_stylesheet(
            consumer,
            0,
            closure,
            &CssManagedScope::Private {
                tag: "cem-card".into(),
                context: None,
            },
            "card",
        )
        .unwrap();
    abort
}
struct Observer {
    revision: u32,
    mismatch: bool,
}
impl CompiledStylesheetObserver for Observer {
    fn environment_revision(&mut self) -> Result<String, String> {
        Ok(self.revision.to_string())
    }
    fn observe(
        &mut self,
        slot: CompiledStylesheetSlot<'_>,
    ) -> Result<CompiledStylesheetObservation, String> {
        assert!(slot.css.contains("@scope"));
        assert!(slot.css.contains("cem-card"));
        Ok(CompiledStylesheetObservation {
            installation: slot.index as u32 + 1,
            selected: true,
            source_matches: !self.mismatch,
        })
    }
}
#[test]
fn exact_native_emission_and_consumer_lifetime_are_required() {
    let mut owner = owner();
    retain(&mut owner, "a");
    retain(&mut owner, "b");
    let a = owner.stylesheet_installations("a", &[0]).unwrap();
    let b = owner.stylesheet_installations("b", &[0]).unwrap();
    let mut host = CompiledStylesheetHost::default();
    let mut observer = Observer {
        revision: 1,
        mismatch: false,
    };
    let evidence = host.capture(&a, &mut observer, 10, 10000).unwrap();
    assert!(host.validate(&evidence, &b, &mut observer).is_err());
    host.validate(&evidence, &a, &mut observer).unwrap();
    owner.release_stylesheet_consumer("a");
    assert!(host.validate(&evidence, &a, &mut observer).is_err());
    assert!(host.capture(&b, &mut observer, 10, 10000).is_ok());
}
#[test]
fn replacement_and_observed_drift_never_revive_old_evidence() {
    let mut owner = owner();
    retain(&mut owner, "a");
    let a = owner.stylesheet_installations("a", &[0]).unwrap();
    let mut host = CompiledStylesheetHost::default();
    let mut observer = Observer {
        revision: 1,
        mismatch: false,
    };
    let evidence = host.capture(&a, &mut observer, 10, 10000).unwrap();
    retain(&mut owner, "a");
    assert!(host.validate(&evidence, &a, &mut observer).is_err());
    let current = owner.stylesheet_installations("a", &[0]).unwrap();
    let evidence = host.capture(&current, &mut observer, 10, 10000).unwrap();
    observer.revision = 2;
    assert!(host.validate(&evidence, &current, &mut observer).is_err());
    observer.revision = 1;
    assert!(host.validate(&evidence, &current, &mut observer).is_err());
    observer.mismatch = true;
    assert!(host.capture(&current, &mut observer, 10, 10000).is_err());
    observer.mismatch = false;
    assert!(host.capture(&current, &mut observer, 0, 10000).is_err());
    assert!(host.capture(&current, &mut observer, 10, 1).is_err());
    assert!(owner.stylesheet_installations("a", &[0, 0]).is_err());
}

#[test]
fn consumer_release_inside_a_callback_cannot_publish() {
    struct Releasing<'a> {
        owner: &'a mut RetainedTemplate,
    }
    impl CompiledStylesheetObserver for Releasing<'_> {
        fn environment_revision(&mut self) -> Result<String, String> {
            Ok("1".into())
        }
        fn observe(
            &mut self,
            _: CompiledStylesheetSlot<'_>,
        ) -> Result<CompiledStylesheetObservation, String> {
            self.owner.release_stylesheet_consumer("a");
            Ok(CompiledStylesheetObservation {
                installation: 1,
                selected: true,
                source_matches: true,
            })
        }
    }
    let mut owner = owner();
    retain(&mut owner, "a");
    let set = owner.stylesheet_installations("a", &[0]).unwrap();
    assert!(CompiledStylesheetHost::default()
        .capture(&set, &mut Releasing { owner: &mut owner }, 10, 10000)
        .is_err());
}

#[test]
fn shared_emission_cannot_hide_a_consumers_cancelled_source_closure() {
    let mut owner = owner();
    retain(&mut owner, "a");
    let abort_b = retain(&mut owner, "b");
    assert!(Arc::ptr_eq(
        &owner.stylesheet("a", 0).unwrap(),
        &owner.stylesheet("b", 0).unwrap()
    ));
    let a = owner.stylesheet_installations("a", &[0]).unwrap();
    let b = owner.stylesheet_installations("b", &[0]).unwrap();
    abort_b.abort();
    assert!(b.validate_current().is_err());
    assert!(a.validate_current().is_ok());
}
