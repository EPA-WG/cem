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

#[test]
fn external_fragment_targets_retain_snapshot_and_source_identity() {
    for kind in ["linearGradient", "radialGradient", "pattern"] {
        let mut host = SvgComputedHost::default();
        let mut o = Observer::default();
        let e = Arc::new(host.capture(&sources(), &mut o).unwrap());
        let cancel = AbortSignal::new();
        let mut r = response();
        r.bytes =
            format!("<svg xmlns='http://www.w3.org/2000/svg'><{kind} id='p'/></svg>").into_bytes();
        let mut record =
            SvgResourceRecord::capture(e.clone(), &mut host, &mut o, vec![r], cancel.clone())
                .unwrap();
        assert!(record.fragment_target(0).is_none());
        record.validate_fragments(&mut host, &mut o, 10000).unwrap();
        let (binding, target) = record.fragment_target(0).unwrap();
        assert!(std::ptr::eq(binding, &record.bindings()[0]));
        assert_eq!(
            binding
                .body
                .tree()
                .node(target)
                .unwrap()
                .name
                .as_ref()
                .unwrap()
                .local_name,
            kind
        );
        assert_eq!(binding.fragment.as_deref(), Some("p"));
        assert!(Arc::ptr_eq(record.computed(), &e));
        assert!(record
            .blockers()
            .contains(&"svg-resource-consumption-required"));
        cancel.abort();
        assert!(record.fragment_target(0).is_none());
        assert!(record.validate(&mut host, &mut o).is_err());
    }
}
#[test]
fn external_fragment_failures_revoke_the_record() {
    for (fragment, children, budget, message) in [
        ("#missing", "<linearGradient id='p'/>", 10000, "missing"),
        (
            "#p",
            "<linearGradient id='p'/><pattern id='p'/>",
            10000,
            "ambiguous",
        ),
        ("#p", "<path id='p'/>", 10000, "paint server"),
        (
            "#p",
            "<linearGradient xmlns='urn:other' id='p'/>",
            10000,
            "paint server",
        ),
        ("#p", "<linearGradient xml:id='p'/>", 10000, "missing"),
        ("", "<linearGradient id='p'/>", 10000, "fragment"),
        ("#", "<linearGradient id='p'/>", 10000, "fragment"),
        ("#%70", "<linearGradient id='p'/>", 10000, "fragment"),
        (
            "#svgView(viewBox(0,0,1,1))",
            "<linearGradient id='p'/>",
            10000,
            "fragment",
        ),
        ("#p", "<linearGradient id='p'/>", 0, "budget"),
    ] {
        let mut host = SvgComputedHost::default();
        let mut o = Observer {
            value: format!("url(\"{URL}{fragment}\")"),
            ..Default::default()
        };
        let e = Arc::new(host.capture(&sources(), &mut o).unwrap());
        let mut r = response();
        r.bytes = format!("<svg xmlns='http://www.w3.org/2000/svg'>{children}</svg>").into_bytes();
        let mut record =
            SvgResourceRecord::capture(e, &mut host, &mut o, vec![r], AbortSignal::new()).unwrap();
        let error = record
            .validate_fragments(&mut host, &mut o, budget)
            .unwrap_err();
        assert!(error.contains(message), "{error}");
        assert!(record.fragment_target(0).is_none());
        assert!(record.bindings().is_empty());
    }
}
#[test]
fn external_fragment_validation_rechecks_observation_lifetime() {
    let mut host = SvgComputedHost::default();
    let mut o = Observer::default();
    let e = Arc::new(host.capture(&sources(), &mut o).unwrap());
    let mut record =
        SvgResourceRecord::capture(e, &mut host, &mut o, vec![response()], AbortSignal::new())
            .unwrap();
    record.validate_fragments(&mut host, &mut o, 10000).unwrap();
    o.revision = "changed".into();
    assert!(record.validate(&mut host, &mut o).is_err());
    assert!(record.fragment_target(0).is_none());
}

#[test]
fn external_fragment_targets_preserve_shared_body_slots_and_reject_partial_results() {
    let mut host = SvgComputedHost::default();
    let mut o = Observer {
        dual: true,
        ..Default::default()
    };
    let e = Arc::new(host.capture(&sources(), &mut o).unwrap());
    let mut r = response();
    r.bytes =
        b"<svg xmlns='http://www.w3.org/2000/svg'><linearGradient id='p'/><pattern id='q'/></svg>"
            .to_vec();
    let mut record =
        SvgResourceRecord::capture(e, &mut host, &mut o, vec![r], AbortSignal::new()).unwrap();
    record.validate_fragments(&mut host, &mut o, 10000).unwrap();
    let (a, first) = record.fragment_target(0).unwrap();
    let (b, second) = record.fragment_target(1).unwrap();
    assert!(Arc::ptr_eq(&a.body, &b.body));
    assert_ne!(a.slot, b.slot);
    assert_ne!(first, second);
    assert_eq!(a.fragment.as_deref(), Some("p"));
    assert_eq!(b.fragment.as_deref(), Some("q"));
    assert!(record.fragment_target(2).is_none());
    assert!(record.validate_fragments(&mut host, &mut o, 0).is_err());
    assert!(record.fragment_target(0).is_none());
    assert!(record.fragment_target(1).is_none());

    let e = Arc::new(host.capture(&sources(), &mut o).unwrap());
    let mut record =
        SvgResourceRecord::capture(e, &mut host, &mut o, vec![response()], AbortSignal::new())
            .unwrap();
    assert!(record
        .validate_fragments(&mut host, &mut o, 10000)
        .unwrap_err()
        .contains("missing"));
    assert!(record.fragment_target(0).is_none());
    assert!(record.bindings().is_empty());
}

fn template_record(children: &str) -> (SvgResourceRecord, SvgComputedHost, Observer, AbortSignal) {
    let mut host = SvgComputedHost::default();
    let mut observer = Observer::default();
    let evidence = Arc::new(host.capture(&sources(), &mut observer).unwrap());
    let cancel = AbortSignal::new();
    let mut r = response();
    r.bytes = format!("<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'>{children}</svg>").into_bytes();
    let record =
        SvgResourceRecord::capture(evidence, &mut host, &mut observer, vec![r], cancel.clone())
            .unwrap();
    (record, host, observer, cancel)
}
#[test]
fn acquired_paint_templates_keep_links_in_the_original_body() {
    for children in [
        "<linearGradient id='p' href='#q' xlink:href='#missing'/><radialGradient id='q' xlink:href='#r'/><linearGradient id='r'/>",
        "<pattern id='p' href='#q'/><pattern id='q' href='#r'/><pattern id='r'/>",
    ] {
        let (mut record, mut host, mut o, cancel) = template_record(children);
        assert!(record.paint_template_links(0).is_none());
        record.validate_paint_templates(&mut host, &mut o, 10000).unwrap();
        let (binding, links) = record.paint_template_links(0).unwrap();
        assert_eq!(links.len(), 2);
        assert!(std::ptr::eq(binding, &record.bindings()[0]));
        assert_eq!(links[0].owner, record.fragment_target(0).unwrap().1);
        assert_eq!(links[0].target, links[1].owner);
        for (i, link) in links.iter().enumerate() {
            let attr = binding.body.tree().node(link.attribute).unwrap();
            assert_eq!(attr.parent, Some(link.owner));
            assert_eq!(attr.value, if i == 0 { "#q" } else { "#r" });
            assert!(!attr.source.frames.is_empty());
            assert!(attr.range.length > 0);
        }
        assert!(record.blockers().contains(&"svg-resource-transitive-dependencies-required"));
        cancel.abort();
        assert!(record.paint_template_links(0).is_none());
        assert!(record.validate(&mut host, &mut o).is_err());
    }
}
#[test]
fn acquired_paint_template_failures_are_atomic() {
    for (children, message) in [
        ("<linearGradient id='p' href='#p'/>", "cycle"),
        ("<linearGradient id='p' href='#q'/><radialGradient id='q' href='#p'/>", "cycle"),
        ("<linearGradient id='p' href='#missing'/>", "missing"),
        ("<linearGradient id='p' href='#q'/><linearGradient id='q'/><radialGradient id='q'/>", "ambiguous"),
        ("<linearGradient id='p' href='#q'/><pattern id='q'/>", "kind"),
        ("<pattern id='p' href='#q'/><linearGradient id='q'/>", "kind"),
        ("<linearGradient id='p' href='#q'/><path id='q'/>", "kind"),
        ("<linearGradient id='p' href='#q'/><linearGradient xmlns='urn:other' id='q'/>", "kind"),
        ("<linearGradient id='p' href='' xlink:href='#q'/><linearGradient id='q'/>", "fragment"),
        ("<linearGradient id='p' href='#%71'/><linearGradient id='q'/>", "fragment"),
        ("<linearGradient id='p' href='paint.svg#q'/><linearGradient id='q'/>", "fragment"),
        ("<linearGradient id='p' href='https://page.test/paint.svg#q'/><linearGradient id='q'/>", "fragment"),
        ("<g xml:base='https://other.test/'><linearGradient id='p' href='#q'/></g><linearGradient id='q'/>", "base"),
        ("<linearGradient id='p' xml:base='' href='#q'/><linearGradient id='q'/>", "base"),
    ] {
        let (mut record, mut host, mut o, _) = template_record(children);
        record.validate_fragments(&mut host, &mut o, 10000).unwrap();
        let error = record.validate_paint_templates(&mut host, &mut o, 10000).unwrap_err();
        assert!(error.contains(message), "{message}: {error}");
        assert!(record.bindings().is_empty());
        assert!(record.fragment_target(0).is_none());
        assert!(record.paint_template_links(0).is_none());
    }
}
#[test]
fn acquired_paint_templates_bound_depth_work_and_observation_lifetime() {
    for count in [64, 65] {
        let mut children = "<linearGradient id='p' href='#n0'/>".to_owned();
        for i in 0..count {
            children.push_str(&format!(
                "<linearGradient id='n{i}' {} />",
                if i + 1 < count {
                    format!("href='#n{}'", i + 1)
                } else {
                    String::new()
                }
            ));
        }
        let (mut record, mut host, mut o, _) = template_record(&children);
        let result = record.validate_paint_templates(&mut host, &mut o, 100000);
        if count == 64 {
            result.unwrap();
            assert_eq!(record.paint_template_links(0).unwrap().1.len(), 64);
        } else {
            assert!(result.unwrap_err().contains("depth"));
            assert!(record.paint_template_links(0).is_none());
        }
    }
    for mode in 0..3 {
        let (mut record, mut host, mut o, _) = template_record("<linearGradient id='p'/>");
        record
            .validate_paint_templates(&mut host, &mut o, 10000)
            .unwrap();
        assert_eq!(record.paint_template_links(0).unwrap().1.len(), 0);
        match mode {
            0 => {
                assert!(record
                    .validate_paint_templates(&mut host, &mut o, 0)
                    .is_err());
            }
            1 => {
                o.revision = "changed".into();
                assert!(record.validate(&mut host, &mut o).is_err());
            }
            _ => {
                record.validate_fragments(&mut host, &mut o, 10000).unwrap();
            }
        }
        assert!(record.paint_template_links(0).is_none());
    }
}

#[test]
fn acquired_paint_templates_preserve_slots_and_share_only_body_coordinates() {
    let mut host = SvgComputedHost::default();
    let mut o = Observer {
        dual: true,
        ..Default::default()
    };
    let e = Arc::new(host.capture(&sources(), &mut o).unwrap());
    let mut r = response();
    r.bytes = b"<svg xmlns='http://www.w3.org/2000/svg'><linearGradient id='p' href='#r'/><radialGradient id='q' href='#r'/><linearGradient id='r'/></svg>".to_vec();
    let cancel = AbortSignal::new();
    let mut record =
        SvgResourceRecord::capture(e, &mut host, &mut o, vec![r], cancel.clone()).unwrap();
    record
        .validate_paint_templates(&mut host, &mut o, 10000)
        .unwrap();
    let (a, a_links) = record.paint_template_links(0).unwrap();
    let (b, b_links) = record.paint_template_links(1).unwrap();
    assert_ne!(a.slot, b.slot);
    assert!(Arc::ptr_eq(&a.body, &b.body));
    assert_ne!(a_links[0].owner, b_links[0].owner);
    assert_eq!(a_links[0].target, b_links[0].target);
    cancel.abort();
    assert!(record
        .validate_paint_templates(&mut host, &mut o, 10000)
        .is_err());
    assert!(record.paint_template_links(0).is_none());
    assert!(record.paint_template_links(1).is_none());
}

fn profile() -> cem_ml::svg_paint_clone_profile::SvgPaintCloneProfile {
    cem_ml::svg_paint_clone_profile::SvgPaintCloneProfile::for_browser("chromium", "148.0.7778.96")
        .unwrap()
}
#[test]
fn acquired_content_attributes_and_children_keep_distinct_sources() {
    let (mut r, mut host, mut o, _) = template_record("<linearGradient id='p' x2='50%' href='#q'/><radialGradient id='q' gradientUnits='userSpaceOnUse' href='#r'/><linearGradient id='r' x1='10%' x2='90%'><stop stop-color='red'/></linearGradient>");
    let (_, _, cap) = fixture::setup("<p/>");
    r.prepare_paint_content(&mut host, &mut o, profile(), &cap, 100000)
        .unwrap();
    let (binding, plan) = r.paint_content(0).unwrap();
    let field = |name| {
        plan.attributes
            .iter()
            .find(|a| a.name == name)
            .unwrap()
            .source
            .unwrap()
    };
    assert_eq!(
        binding.body.tree().node(field("x2").1).unwrap().value,
        "50%"
    );
    assert_eq!(
        binding.body.tree().node(field("x1").1).unwrap().value,
        "10%"
    );
    assert_ne!(field("x2").0, field("x1").0);
    assert!(plan
        .attributes
        .iter()
        .find(|a| a.name == "y1")
        .unwrap()
        .source
        .is_none());
    assert_eq!(plan.content_owner, Some(field("x1").0));
    assert_eq!(plan.svg2_content_owner, plan.content_owner);
    assert_eq!(plan.nodes.len(), 1);
    assert_eq!(
        plan.nodes[0].source_ancestors[0],
        plan.content_owner.unwrap()
    );
    assert_ne!(plan.host, plan.content_owner.unwrap());
    assert!(Arc::ptr_eq(
        &plan.inventory.document.tree,
        binding.body.tree()
    ));
}
#[test]
fn acquired_content_selection_records_pinned_browser_differences() {
    let (_, _, cap) = fixture::setup("<p/>");
    for (kind, child, own) in [
        ("linearGradient", "<title>note</title>", false),
        ("linearGradient", "<stop stop-color='blue'/>", true),
        ("pattern", "<title>note</title>", true),
        ("pattern", "", false),
    ] {
        let terminal = if kind == "pattern" {
            "<rect width='16' height='16'/>"
        } else {
            "<stop/>"
        };
        let (mut r, mut h, mut o, _) = template_record(&format!(
            "<{kind} id='p' href='#q'>{child}</{kind}><{kind} id='q'>{terminal}</{kind}>"
        ));
        r.prepare_paint_content(&mut h, &mut o, profile(), &cap, 100000)
            .unwrap();
        let (_, plan) = r.paint_content(0).unwrap();
        assert_eq!(plan.content_owner == Some(plan.host), own);
        if kind == "pattern" && !child.is_empty() {
            assert_ne!(plan.svg2_content_owner, plan.content_owner);
        }
    }
}
#[test]
fn acquired_content_styles_and_nested_dependencies_keep_body_base_and_policy() {
    let (_, _, cap) = fixture::setup("<p/>");
    let (mut r, mut h, mut o, _) = template_record("<style>image { filter:url(./effects.svg#f) }</style><pattern id='p' href='#q' color='blue'/><pattern id='q' color='red' style='opacity:1'><image href='./image.svg' style='fill:url(./nested.svg#p)'/></pattern>");
    r.prepare_paint_content(&mut h, &mut o, profile(), &cap, 100000)
        .unwrap();
    let (binding, plan) = r.paint_content(0).unwrap();
    assert!(plan
        .style_sources
        .iter()
        .any(|s| s.source_node == plan.content_owner.unwrap()
            || binding.body.tree().node(s.source_node).unwrap().parent == plan.content_owner));
    let urls: Vec<_> = plan
        .dependencies
        .iter()
        .map(|d| d.resolution.as_ref().unwrap().resolved_url.as_str())
        .collect();
    for url in [
        "https://page.test/effects.svg#f",
        "https://page.test/image.svg",
        "https://page.test/nested.svg#p",
    ] {
        assert!(urls.contains(&url), "{urls:?}");
    }
    for d in &plan.dependencies {
        assert_eq!(d.resolution.as_ref().unwrap().resource_policy_stamp, "v1");
        assert_eq!(d.base_url, URL);
        assert!(binding.body.tree().node(d.source_node).is_some());
    }
    assert!(plan
        .blockers
        .iter()
        .any(|s| s == "svg-acquired-style-evaluation-required"));
}
#[test]
fn acquired_content_rejects_active_contexts_and_revokes_atomically() {
    let (_, _, cap) = fixture::setup("<p/>");
    for source in [
        "<linearGradient id='p'/><script/>",
        "<linearGradient id='p'><animate attributeName='x2'/></linearGradient>",
        "<linearGradient id='p' onload='x()'/>",
        "<g xml:base='other/'><linearGradient id='p'/></g>",
        "<linearGradient id='p'/><foreignObject/>",
    ] {
        let (mut r, mut h, mut o, _) = template_record(source);
        assert!(r
            .prepare_paint_content(&mut h, &mut o, profile(), &cap, 100000)
            .is_err());
        assert!(r.paint_content(0).is_none());
        assert!(r.bindings().is_empty());
    }
    let (mut r, mut h, mut o, cancel) =
        template_record("<linearGradient id='p'><stop/></linearGradient>");
    r.prepare_paint_content(&mut h, &mut o, profile(), &cap, 100000)
        .unwrap();
    cancel.abort();
    assert!(r.paint_content(0).is_none());
    assert!(r.validate(&mut h, &mut o).is_err());
}
#[test]
fn acquired_content_budget_and_narrower_revalidation_clear_plans() {
    let (_, _, cap) = fixture::setup("<p/>");
    for mode in 0..3 {
        let (mut r, mut h, mut o, _) = template_record("<linearGradient id='p'/>");
        r.prepare_paint_content(&mut h, &mut o, profile(), &cap, 100000)
            .unwrap();
        assert!(r.paint_content(0).unwrap().1.content_owner.is_none());
        match mode {
            0 => {
                assert!(r
                    .prepare_paint_content(&mut h, &mut o, profile(), &cap, 0)
                    .is_err());
            }
            1 => {
                r.validate_paint_templates(&mut h, &mut o, 100000).unwrap();
            }
            _ => {
                o.revision = "changed".into();
                assert!(r.validate(&mut h, &mut o).is_err());
            }
        }
        assert!(r.paint_content(0).is_none());
    }
}

fn nested_response(url: &str, content_type: &str, bytes: &[u8]) -> SvgResourceResponse {
    SvgResourceResponse {
        requested_url: url.into(),
        final_url: url.into(),
        revision: "nested:1".into(),
        content_type: content_type.into(),
        bytes: bytes.to_vec(),
    }
}
#[test]
fn acquired_nested_graph_preserves_transitive_sources_and_lifetime() {
    let (_, _, cap) = fixture::setup("<p/>");
    let (mut r, mut h, mut o, _) = template_record("<style>@import './theme.css';</style><pattern id='p'><image href='./nested.svg'/></pattern>");
    r.prepare_paint_content(&mut h, &mut o, profile(), &cap, 100000)
        .unwrap();
    let cancel = AbortSignal::new();
    r.bind_nested_snapshots(
        &mut h,
        &mut o,
        &cap,
        vec![
            nested_response(
                "https://page.test/theme.css",
                "text/css",
                b"@import './theme.css'; image { filter:url(./nested.svg#f) }",
            ),
            nested_response(
                "https://page.test/nested.svg",
                "image/svg+xml",
                b"<svg xmlns='http://www.w3.org/2000/svg'><image href='./pixel.png'/></svg>",
            ),
            nested_response(
                "https://page.test/pixel.png",
                "image/png",
                b"opaque fixture bytes",
            ),
        ],
        cancel.clone(),
    )
    .unwrap();
    let graph = r.nested_snapshots().unwrap();
    assert_eq!(graph.bodies().len(), 3);
    assert!(graph.edges().len() >= 5);
    for edge in graph.edges() {
        assert!(edge.source.node(edge.node).is_some());
        assert_eq!(edge.resolution.resource_policy_stamp, "v1");
    }
    let body = &graph.bodies()["https://page.test/pixel.png"];
    assert_eq!(body.bytes(), b"opaque fixture bytes");
    assert_eq!(
        body.digest(),
        cem_ml::content_cache::ContentHash::from_blake3(body.bytes()).header_value()
    );
    cancel.abort();
    assert!(r.nested_snapshots().is_none());
    assert!(r.paint_content(0).is_none());
    assert!(r.validate(&mut h, &mut o).is_err());
}
#[test]
fn acquired_nested_graph_rejects_incomplete_or_inconsistent_snapshots() {
    let (_, _, cap) = fixture::setup("<p/>");
    for mode in 0..5 {
        let (mut r, mut h, mut o, _) = template_record(
            "<style>@import './theme.css';</style><linearGradient id='p'><stop/></linearGradient>",
        );
        r.prepare_paint_content(&mut h, &mut o, profile(), &cap, 100000)
            .unwrap();
        let mut response = nested_response(
            "https://page.test/theme.css",
            "text/css",
            b"stop { stop-color:red }",
        );
        match mode {
            0 => response.requested_url = "https://page.test/other.css".into(),
            1 => response.content_type = "text/html".into(),
            2 => response.bytes = b"@import './missing.css';".to_vec(),
            3 => response.revision.clear(),
            _ => response.final_url = "https://page.test/redirect.css".into(),
        }
        assert!(r
            .bind_nested_snapshots(&mut h, &mut o, &cap, vec![response], AbortSignal::new())
            .is_err());
        assert!(r.nested_snapshots().is_none());
        assert!(r.bindings().is_empty());
    }
}

#[test]
fn acquired_nested_graph_requires_original_resolution_owner() {
    let (_, _, cap) = fixture::setup("<p/>");
    let (_, _, other) = fixture::setup("<p/>");
    let (mut r, mut h, mut o, _) = template_record("<linearGradient id='p'/>");
    r.prepare_paint_content(&mut h, &mut o, profile(), &cap, 100000)
        .unwrap();
    assert!(r
        .bind_nested_snapshots(&mut h, &mut o, &other, vec![], AbortSignal::new())
        .is_err());
    assert!(r.paint_content(0).is_none());
}

#[test]
fn acquired_color_winners_reuse_source_cascade_and_current_color_ancestry() {
    let (_, _, cap) = fixture::setup("<p/>");
    for (style, expected) in [
        ("", [255, 0, 0, 255]),
        (
            "<style>stop {stop-color:blue !important}</style>",
            [0, 0, 255, 255],
        ),
    ] {
        let (mut r, mut h, mut o, _) = template_record(&format!("{style}<linearGradient id='p' color='blue' href='#q'/><linearGradient id='q' color='red'><stop style='stop-color:currentColor' /></linearGradient>"));
        r.prepare_paint_content(&mut h, &mut o, profile(), &cap, 100000)
            .unwrap();
        let (binding, plan) = r.paint_content(0).unwrap();
        let colors = plan.colors.as_ref().unwrap();
        assert_eq!(colors.len(), 1);
        assert_eq!(colors[0].rgba, expected);
        assert_eq!(colors[0].host, plan.host);
        assert_eq!(colors[0].source, plan.nodes[0].source);
        assert!(binding
            .body
            .tree()
            .node(colors[0].paint_winner.document_source())
            .is_some());
        if style.is_empty() {
            assert!(colors[0]
                .inheritance_nodes
                .contains(&plan.content_owner.unwrap()));
            assert!(!colors[0].inheritance_nodes.contains(&plan.host));
        }
    }
}

#[test]
fn acquired_nested_graph_preserves_denied_policy_and_rejects_extra_bodies() {
    let (_, _, cap) = fixture::setup("<p/>");
    let (mut r, mut h, mut o, _) = template_record("<style>@import 'https://page.test/assets/denied.css';</style><linearGradient id='p'><stop/></linearGradient>");
    r.prepare_paint_content(&mut h, &mut o, profile(), &cap, 100000)
        .unwrap();
    assert!(r.paint_content(0).unwrap().1.dependencies[0]
        .resolution
        .is_err());
    assert!(r
        .bind_nested_snapshots(&mut h, &mut o, &cap, vec![], AbortSignal::new())
        .is_err());
    for duplicate in [false, true] {
        let (mut r, mut h, mut o, _) = template_record("<linearGradient id='p'/>");
        r.prepare_paint_content(&mut h, &mut o, profile(), &cap, 100000)
            .unwrap();
        let mut responses = vec![nested_response(
            "https://page.test/unused.png",
            "image/png",
            b"opaque",
        )];
        if duplicate {
            responses.push(nested_response(
                "https://page.test/unused.png",
                "image/png",
                b"opaque",
            ));
        }
        assert!(r
            .bind_nested_snapshots(&mut h, &mut o, &cap, responses, AbortSignal::new())
            .is_err());
        assert!(r.paint_content(0).is_none());
    }
}
