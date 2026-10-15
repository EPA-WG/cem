use cem_ml::{
    import::svg_presentation::RetainedSvgPresentationAttribute,
    scheduler::AbortSignal,
    svg_computed_execution::{SvgComputedHost, SvgComputedObserver, SvgComputedSlot},
    svg_resource_acquisition::{SvgResourceRecord, SvgResourceResponse},
};
use std::sync::Arc;
#[path = "support/stylesheet_fixture.rs"]
mod fixture;
const URL: &str = "https://page.test/paint.svg";
const SVG: &str = "<svg xmlns='http://www.w3.org/2000/svg'><linearGradient id='p'/></svg>";
struct Observer {
    value: String,
    revision: String,
    dual: bool,
}
impl Default for Observer {
    fn default() -> Self {
        Self {
            value: format!("url(\"{URL}#p\")"),
            revision: "1".into(),
            dual: false,
        }
    }
}
impl SvgComputedObserver for Observer {
    fn environment_revision(&mut self) -> Result<String, String> {
        Ok(self.revision.clone())
    }
    fn observe(&mut self, slot: SvgComputedSlot<'_>) -> Result<String, String> {
        Ok(if self.dual && slot.property == "stroke" {
            self.value.replace("#p", "#q")
        } else {
            self.value.clone()
        })
    }
}
fn sources() -> Vec<RetainedSvgPresentationAttribute> {
    let (_, inv, _) = fixture::setup("<svg xmlns='http://www.w3.org/2000/svg'><path fill='url(authored.svg#x)' stroke='url(other.svg#x)'/></svg>");
    inv.svg_presentations()
        .iter()
        .map(|p| p.source.clone())
        .collect()
}
fn response() -> SvgResourceResponse {
    SvgResourceResponse {
        requested_url: URL.into(),
        final_url: URL.into(),
        content_type: "image/svg+xml".into(),
        revision: "v1".into(),
        bytes: SVG.as_bytes().to_vec(),
    }
}
#[test]
fn computed_css_references_share_exact_acquired_bytes_without_claiming_consumption() {
    let mut host = SvgComputedHost::default();
    let mut observer = Observer {
        dual: true,
        ..Default::default()
    };
    let evidence = Arc::new(host.capture(&sources(), &mut observer).unwrap());
    let mut record = SvgResourceRecord::capture(
        evidence.clone(),
        &mut host,
        &mut observer,
        vec![response()],
        AbortSignal::new(),
    )
    .unwrap();
    assert_eq!(record.bindings().len(), 2);
    let a = &record.bindings()[0];
    let b = &record.bindings()[1];
    assert!(Arc::ptr_eq(&a.body, &b.body));
    assert_eq!(a.fragment.as_deref(), Some("p"));
    assert_eq!(b.fragment.as_deref(), Some("q"));
    assert_eq!(a.url, format!("{URL}#p"));
    assert_eq!(a.body.bytes(), SVG.as_bytes());
    assert_eq!(a.body.revision(), "v1");
    assert_eq!(
        a.body.digest(),
        cem_ml::content_cache::ContentHash::from_blake3(SVG.as_bytes()).header_value()
    );
    assert!(!a.css.node(a.reference).unwrap().source.frames.is_empty());
    assert!(a.css.node(a.reference).unwrap().range.length > 0);
    assert!(!Arc::ptr_eq(&a.css, evidence.sources()[a.slot].css()));
    assert_eq!(
        record.computed().sources()[0].value(),
        "url(authored.svg#x)"
    );
    assert!(a.body.tree().node(0).unwrap().children.len() > 0);
    assert!(record
        .blockers()
        .contains(&"svg-resource-browser-response-identity-required"));
    assert!(record
        .blockers()
        .contains(&"svg-resource-consumption-required"));
    record.validate(&mut host, &mut observer).unwrap();
}
#[test]
fn missing_duplicate_unused_redirected_and_invalid_svg_responses_reject() {
    let mut host = SvgComputedHost::default();
    let mut o = Observer::default();
    let e = Arc::new(host.capture(&sources(), &mut o).unwrap());
    let mut cases = vec![vec![], vec![response(), response()]];
    for mode in 0..7 {
        let mut r = response();
        match mode {
            0 => r.final_url = "https://page.test/redirect.svg".into(),
            1 => r.content_type = "text/html".into(),
            2 => r.bytes = b"broken".to_vec(),
            3 => r.bytes = b"<html/>".to_vec(),
            4 => r.revision.clear(),
            5 => r.requested_url = "https://page.test/unused.svg".into(),
            _ => r.requested_url.push_str("#p"),
        }
        cases.push(vec![r]);
    }
    for responses in cases {
        assert!(SvgResourceRecord::capture(
            e.clone(),
            &mut host,
            &mut o,
            responses,
            AbortSignal::new()
        )
        .is_err());
    }
    let mut extra = response();
    extra.requested_url = "https://page.test/extra.svg".into();
    extra.final_url = extra.requested_url.clone();
    assert!(SvgResourceRecord::capture(
        e,
        &mut host,
        &mut o,
        vec![response(), extra],
        AbortSignal::new()
    )
    .is_err());
}
#[test]
fn only_one_absolute_http_svg_url_is_admitted_per_computed_property() {
    for value in [
        "none",
        "red",
        "url(#local)",
        "url(relative.svg)",
        "url(data:image/svg+xml,abc)",
        "url(https://page.test/paint.svg) red",
        "url(https://page.test/paint.svg),url(https://page.test/paint.svg)",
        "url(https://page.test/paint.svg);stroke:red",
        "url(https://user@page.test/paint.svg)",
    ] {
        let mut host = SvgComputedHost::default();
        let mut o = Observer {
            value: value.into(),
            ..Default::default()
        };
        let e = Arc::new(host.capture(&sources(), &mut o).unwrap());
        assert!(
            SvgResourceRecord::capture(e, &mut host, &mut o, vec![response()], AbortSignal::new())
                .is_err(),
            "{value}"
        );
    }
}
#[test]
fn cancelled_or_stale_bindings_release_bodies_and_cannot_revive() {
    for mode in 0..4 {
        let mut host = SvgComputedHost::default();
        let mut o = Observer::default();
        let cancel = AbortSignal::new();
        let e = Arc::new(host.capture(&sources(), &mut o).unwrap());
        let mut r =
            SvgResourceRecord::capture(e, &mut host, &mut o, vec![response()], cancel.clone())
                .unwrap();
        let weak = Arc::downgrade(&r.bindings()[0].body);
        match mode {
            0 => cancel.abort(),
            1 => o.revision = "2".into(),
            2 => host.invalidate().unwrap(),
            _ => o.value = format!("url(\"{URL}#changed\")"),
        }
        assert!(r.validate(&mut host, &mut o).is_err());
        o.revision = "1".into();
        o.value = Observer::default().value;
        assert!(r.validate(&mut host, &mut o).is_err());
        assert!(r.bindings().is_empty());
        assert!(weak.upgrade().is_none());
    }
}
#[test]
fn foreign_computed_host_and_preaborted_capture_reject() {
    let mut host = SvgComputedHost::default();
    let mut o = Observer::default();
    let e = Arc::new(host.capture(&sources(), &mut o).unwrap());
    assert!(SvgResourceRecord::capture(
        e.clone(),
        &mut SvgComputedHost::default(),
        &mut o,
        vec![response()],
        AbortSignal::new()
    )
    .is_err());
    let cancel = AbortSignal::new();
    cancel.abort();
    assert!(SvgResourceRecord::capture(e, &mut host, &mut o, vec![response()], cancel).is_err());
}
#[test]
fn response_count_and_total_bytes_are_bounded_before_import() {
    let mut host = SvgComputedHost::default();
    let mut o = Observer::default();
    let e = Arc::new(host.capture(&sources(), &mut o).unwrap());
    assert!(SvgResourceRecord::capture(
        e.clone(),
        &mut host,
        &mut o,
        (0..257).map(|_| response()).collect(),
        AbortSignal::new()
    )
    .err()
    .unwrap()
    .contains("count bound"));
    let mut r = response();
    r.bytes = vec![b' '; 4 * 1024 * 1024 + 1];
    assert!(
        SvgResourceRecord::capture(e, &mut host, &mut o, vec![r], AbortSignal::new())
            .err()
            .unwrap()
            .contains("byte bound")
    );
}

#[test]
fn shared_css_decoding_preserves_fragment_and_fragmentless_body_identity() {
    for (value, fragment) in [
        (format!("url({URL})"), None),
        (
            r"url(https://page.test/pa\69nt.svg#%70)".into(),
            Some("%70"),
        ),
    ] {
        let mut host = SvgComputedHost::default();
        let mut o = Observer {
            value,
            ..Default::default()
        };
        let e = Arc::new(host.capture(&sources(), &mut o).unwrap());
        let r =
            SvgResourceRecord::capture(e, &mut host, &mut o, vec![response()], AbortSignal::new())
                .unwrap();
        assert_eq!(r.bindings()[0].fragment.as_deref(), fragment);
        assert_eq!(r.bindings()[0].body.url(), URL);
    }
}
#[test]
fn cancellation_during_final_observation_cannot_publish_bindings() {
    struct Cancelling {
        inner: Observer,
        calls: usize,
        cancel: AbortSignal,
    }
    impl SvgComputedObserver for Cancelling {
        fn environment_revision(&mut self) -> Result<String, String> {
            self.inner.environment_revision()
        }
        fn observe(&mut self, slot: SvgComputedSlot<'_>) -> Result<String, String> {
            self.calls += 1;
            if self.calls == 3 {
                self.cancel.abort();
            }
            self.inner.observe(slot)
        }
    }
    let mut host = SvgComputedHost::default();
    let mut o = Observer::default();
    let e = Arc::new(host.capture(&sources(), &mut o).unwrap());
    let cancel = AbortSignal::new();
    let mut o = Cancelling {
        inner: o,
        calls: 0,
        cancel: cancel.clone(),
    };
    assert!(SvgResourceRecord::capture(e, &mut host, &mut o, vec![response()], cancel).is_err());
}

fn acquired() -> (SvgResourceRecord, SvgComputedHost, Observer) {
    let mut host = SvgComputedHost::default();
    let mut observer = Observer::default();
    let evidence = Arc::new(host.capture(&sources(), &mut observer).unwrap());
    let record = SvgResourceRecord::capture(
        evidence,
        &mut host,
        &mut observer,
        vec![response()],
        AbortSignal::new(),
    )
    .unwrap();
    (record, host, observer)
}
fn delivery() -> cem_ml::svg_resource_acquisition::SvgDeliveryResponse {
    cem_ml::svg_resource_acquisition::SvgDeliveryResponse {
        requested_url: URL.into(),
        final_url: URL.into(),
        content_type: "image/svg+xml".into(),
        bytes: SVG.as_bytes().to_vec(),
    }
}
#[test]
fn delivery_bytes_join_native_bindings_and_transport_lifetime() {
    use cem_ml::svg_resource_acquisition::SvgDeliveryCapture;
    let (mut record, mut host, mut observer) = acquired();
    let cancel = AbortSignal::new();
    let mut ledger = SvgDeliveryCapture::new(cancel.clone());
    ledger.accept(delivery()).unwrap();
    record
        .bind_delivery(ledger, &mut host, &mut observer)
        .unwrap();
    assert_eq!(record.delivered_bindings(), 2);
    assert!(record
        .blockers()
        .contains(&"svg-resource-browser-response-identity-required"));
    record.validate(&mut host, &mut observer).unwrap();
    cancel.abort();
    assert_eq!(record.delivered_bindings(), 0);
    assert!(record.validate(&mut host, &mut observer).is_err());
    assert!(record.bindings().is_empty());
}
#[test]
fn delivery_missing_changed_extra_and_stale_evidence_revoke_record() {
    use cem_ml::svg_resource_acquisition::SvgDeliveryCapture;
    for mode in ["missing", "changed", "extra", "stale", "cancel", "repeat"] {
        let (mut record, mut host, mut observer) = acquired();
        let cancel = AbortSignal::new();
        let mut ledger = SvgDeliveryCapture::new(cancel.clone());
        if mode != "missing" {
            let mut input = delivery();
            if mode == "changed" {
                input.bytes.extend_from_slice(b" ");
            }
            ledger.accept(input).unwrap();
        }
        if mode == "extra" {
            let mut input = delivery();
            input.requested_url = "https://page.test/extra.svg".into();
            input.final_url = input.requested_url.clone();
            ledger.accept(input).unwrap();
        }
        if mode == "stale" {
            observer.revision = "changed".into();
        }
        if mode == "cancel" {
            cancel.abort();
        }
        if mode == "repeat" {
            let mut first = SvgDeliveryCapture::new(AbortSignal::new());
            first.accept(delivery()).unwrap();
            record
                .bind_delivery(first, &mut host, &mut observer)
                .unwrap();
        }
        assert!(
            record
                .bind_delivery(ledger, &mut host, &mut observer)
                .is_err(),
            "{mode}"
        );
        assert!(record.bindings().is_empty(), "{mode}");
        assert_eq!(record.delivered_bindings(), 0);
    }
}
#[test]
fn delivery_ingress_rejects_ambiguity_metadata_and_bounds_irreversibly() {
    use cem_ml::svg_resource_acquisition::SvgDeliveryCapture;
    for mode in [
        "duplicate",
        "redirect",
        "fragment",
        "mime",
        "empty",
        "bounds",
        "credentials",
        "closed",
    ] {
        let mut ledger = SvgDeliveryCapture::new(AbortSignal::new());
        let mut input = delivery();
        match mode {
            "duplicate" => ledger.accept(delivery()).unwrap(),
            "redirect" => input.final_url = "https://page.test/other.svg".into(),
            "fragment" => {
                input.requested_url.push_str("#p");
                input.final_url = input.requested_url.clone();
            }
            "mime" => input.content_type = "text/html".into(),
            "empty" => input.bytes.clear(),
            "bounds" => input.bytes = vec![0; 4 * 1024 * 1024],
            "credentials" => {
                input.requested_url = "https://user@page.test/a.svg".into();
                input.final_url = input.requested_url.clone();
            }
            "closed" => ledger.close(),
            _ => unreachable!(),
        }
        assert!(ledger.accept(input).is_err(), "{mode}");
        assert!(ledger.accept(delivery()).is_err(), "{mode}");
        let (mut record, mut host, mut observer) = acquired();
        assert!(
            record
                .bind_delivery(ledger, &mut host, &mut observer)
                .is_err(),
            "{mode}"
        );
    }
}

#[test]
fn delivery_receipt_count_is_bounded_and_final_observation_cannot_publish_after_abort() {
    use cem_ml::svg_resource_acquisition::SvgDeliveryCapture;
    let mut ledger = SvgDeliveryCapture::new(AbortSignal::new());
    for index in 0..256 {
        let mut input = delivery();
        input.requested_url = format!("https://page.test/{index}.svg");
        input.final_url = input.requested_url.clone();
        ledger.accept(input).unwrap();
    }
    assert!(ledger.accept(delivery()).is_err());
    let (mut record, mut host, mut observer) = acquired();
    let cancel = AbortSignal::new();
    let mut ledger = SvgDeliveryCapture::new(cancel.clone());
    ledger.accept(delivery()).unwrap();
    struct Cancelling<'a> {
        observer: &'a mut Observer,
        cancel: AbortSignal,
    }
    impl SvgComputedObserver for Cancelling<'_> {
        fn environment_revision(&mut self) -> Result<String, String> {
            self.observer.environment_revision()
        }
        fn observe(&mut self, slot: SvgComputedSlot<'_>) -> Result<String, String> {
            self.cancel.abort();
            self.observer.observe(slot)
        }
    }
    assert!(record
        .bind_delivery(
            ledger,
            &mut host,
            &mut Cancelling {
                observer: &mut observer,
                cancel
            }
        )
        .is_err());
    assert_eq!(record.delivered_bindings(), 0);
    assert!(record.bindings().is_empty());
}
