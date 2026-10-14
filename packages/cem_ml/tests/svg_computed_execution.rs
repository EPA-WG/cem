use cem_ml::{
    import::svg_presentation::RetainedSvgPresentationAttribute,
    svg_computed_execution::{SvgComputedHost, SvgComputedObserver, SvgComputedSlot},
};
#[path = "support/stylesheet_fixture.rs"]
mod fixture;

struct Observer {
    value: String,
    revision: String,
    calls: usize,
    change_at: Option<usize>,
}
impl Default for Observer {
    fn default() -> Self {
        Self {
            value: "url(\"https://page.test/assets/override.svg#p\")".into(),
            revision: "initial".into(),
            calls: 0,
            change_at: None,
        }
    }
}
impl SvgComputedObserver for Observer {
    fn environment_revision(&mut self) -> Result<String, String> {
        Ok(self.revision.clone())
    }
    fn observe(&mut self, slot: SvgComputedSlot<'_>) -> Result<String, String> {
        assert_eq!(slot.property, "fill");
        assert!(!slot.source.source().frames.is_empty());
        self.calls += 1;
        if self.change_at == Some(self.calls) {
            return Ok("none".into());
        }
        Ok(self.value.clone())
    }
}
fn sources() -> Vec<RetainedSvgPresentationAttribute> {
    let (_, inventory, _) = fixture::setup(
        "<svg xmlns='http://www.w3.org/2000/svg'><path fill='url(authored.svg#p)'/></svg>",
    );
    inventory
        .svg_presentations()
        .iter()
        .map(|p| p.source.clone())
        .collect()
}
#[test]
fn computed_override_is_an_observation_not_authored_selection_or_acquisition() {
    let sources = sources();
    let mut host = SvgComputedHost::default();
    let mut observer = Observer::default();
    let evidence = host.capture(&sources, &mut observer).unwrap();
    assert_eq!(evidence.values(), &[observer.value.clone()]);
    assert_eq!(evidence.sources()[0].value(), "url(authored.svg#p)");
    assert!(std::sync::Arc::ptr_eq(
        evidence.sources()[0].document(),
        sources[0].document()
    ));
    assert!(evidence
        .blockers()
        .contains(&"svg-computed-resource-acquisition-required"));
    host.validate(&evidence, &sources, &mut observer).unwrap();
}
#[test]
fn same_value_restoration_cannot_revive_evidence() {
    let sources = sources();
    let mut host = SvgComputedHost::default();
    let mut observer = Observer::default();
    let evidence = host.capture(&sources, &mut observer).unwrap();
    let original = observer.value.clone();
    observer.value = "none".into();
    assert!(host.validate(&evidence, &sources, &mut observer).is_err());
    observer.value = original;
    assert!(host.validate(&evidence, &sources, &mut observer).is_err());
    let fresh = host.capture(&sources, &mut observer).unwrap();
    host.validate(&fresh, &sources, &mut observer).unwrap();
    assert!(host.validate(&evidence, &sources, &mut observer).is_err());
}
#[test]
fn foreign_host_and_equal_reimported_source_do_not_supply_authority() {
    let sources = sources();
    let mut host = SvgComputedHost::default();
    let mut observer = Observer::default();
    let evidence = host.capture(&sources, &mut observer).unwrap();
    assert!(SvgComputedHost::default()
        .validate(&evidence, &sources, &mut observer)
        .is_err());
    let (_, other, _) = fixture::setup(
        "<svg xmlns='http://www.w3.org/2000/svg'><path fill='url(authored.svg#p)'/></svg>",
    );
    assert!(host
        .validate(
            &evidence,
            &[other.svg_presentations()[0].source.clone()],
            &mut observer
        )
        .is_err());
}
#[test]
fn unstable_capture_and_failed_recapture_supersede_old_evidence() {
    let sources = sources();
    let mut host = SvgComputedHost::default();
    let mut observer = Observer::default();
    let evidence = host.capture(&sources, &mut observer).unwrap();
    observer.change_at = Some(observer.calls + 2);
    assert!(host.capture(&sources, &mut observer).is_err());
    observer.change_at = None;
    assert!(host.validate(&evidence, &sources, &mut observer).is_err());
}
#[test]
fn revision_changes_revoke_even_when_computed_values_match() {
    let sources = sources();
    let mut host = SvgComputedHost::default();
    let mut observer = Observer::default();
    let evidence = host.capture(&sources, &mut observer).unwrap();
    observer.revision = "restyled".into();
    assert!(host.validate(&evidence, &sources, &mut observer).is_err());
    observer.revision = "initial".into();
    assert!(host.validate(&evidence, &sources, &mut observer).is_err());
}
#[test]
fn bounded_values_revisions_and_unique_slots_are_required() {
    let sources = sources();
    let mut host = SvgComputedHost::default();
    let mut observer = Observer::default();
    assert!(host
        .capture(&[sources[0].clone(), sources[0].clone()], &mut observer)
        .is_err());
    assert!(host
        .capture(&vec![sources[0].clone(); 257], &mut observer)
        .is_err());
    for value in [String::new(), "x".repeat(4097)] {
        observer.value = value;
        assert!(host.capture(&sources, &mut observer).is_err());
    }
    observer = Observer::default();
    observer.revision = " ".into();
    assert!(host.capture(&sources, &mut observer).is_err());
}

#[test]
fn observer_failure_and_explicit_invalidation_revoke_evidence() {
    struct Failed;
    impl SvgComputedObserver for Failed {
        fn environment_revision(&mut self) -> Result<String, String> {
            Ok("initial".into())
        }
        fn observe(&mut self, _: SvgComputedSlot<'_>) -> Result<String, String> {
            Err("owner detached".into())
        }
    }
    let sources = sources();
    let mut host = SvgComputedHost::default();
    let mut observer = Observer::default();
    let evidence = host.capture(&sources, &mut observer).unwrap();
    assert!(host.validate(&evidence, &sources, &mut Failed).is_err());
    assert!(host.validate(&evidence, &sources, &mut observer).is_err());
    let fresh = host.capture(&sources, &mut observer).unwrap();
    host.invalidate().unwrap();
    assert!(host.validate(&fresh, &sources, &mut observer).is_err());
}

#[test]
fn mixed_documents_and_changed_environment_during_capture_are_rejected() {
    let sources = sources();
    let (_, inventory, _) =
        fixture::setup("<svg xmlns='http://www.w3.org/2000/svg'><path fill='red'/></svg>");
    let mut host = SvgComputedHost::default();
    assert!(host
        .capture(
            &[
                sources[0].clone(),
                inventory.svg_presentations()[0].source.clone()
            ],
            &mut Observer::default()
        )
        .is_err());
    struct Changing(usize);
    impl SvgComputedObserver for Changing {
        fn environment_revision(&mut self) -> Result<String, String> {
            self.0 += 1;
            Ok(self.0.to_string())
        }
        fn observe(&mut self, _: SvgComputedSlot<'_>) -> Result<String, String> {
            Ok("none".into())
        }
    }
    assert!(host.capture(&sources, &mut Changing(0)).is_err());
}
