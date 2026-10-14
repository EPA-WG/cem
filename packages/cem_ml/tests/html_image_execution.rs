use cem_ml::{html_image_execution::*, import::import_data_bytes, scheduler::AbortSignal};

#[test]
fn selected_images_keep_native_candidates_and_acquisitions_separate() {
    let tree = import_data_bytes(br#"<html xmlns="http://www.w3.org/1999/xhtml"><body><picture><source srcset="wide.png 1x, retina.png 2x" media="screen"/><img src="fallback.png" srcset="small.png 1x"/></picture></body></html>"#, "application/xml", "cem", "https://test/source").unwrap();
    let plan = ImagePlan::from_tree(tree, "https://test/assets/", 100, 16).unwrap();
    assert_eq!(plan.targets().len(), 1);
    assert_eq!(plan.targets()[0].urls.len(), 4);
    let observation = ImageObservation {
        url: "https://test/assets/retina.png".into(),
        width: 10,
        height: 10,
        complete: true,
    };
    let cancel = AbortSignal::new();
    let response = ImageResponse {
        requested_url: observation.url.clone(),
        final_url: observation.url.clone(),
        content_type: "image/png".into(),
        revision: "1".into(),
        bytes: vec![1, 2, 3],
    };
    let mut record = ImageRecord::capture(
        plan.clone(),
        vec![response],
        "env:1",
        |_| Ok(observation.clone()),
        cancel.clone(),
        1024,
    )
    .unwrap();
    assert_eq!(record.responses()[0].bytes, vec![1, 2, 3]);
    assert!(record
        .validate(&plan, "env:1", |_| Ok(observation.clone()))
        .is_ok());
    assert!(record
        .validate(&plan, "env:2", |_| Ok(observation.clone()))
        .is_err());
    assert!(record
        .validate(&plan, "env:1", |_| Ok(observation.clone()))
        .is_err());
}
#[test]
fn foreign_unloaded_missing_and_cancelled_images_fail_closed() {
    let tree = import_data_bytes(
        br#"<img xmlns="http://www.w3.org/1999/xhtml" srcset="a.png 1x, b.png 2x"/>"#,
        "application/xml",
        "cem",
        "https://test/source",
    )
    .unwrap();
    let plan = ImagePlan::from_tree(tree, "https://test/", 100, 4).unwrap();
    let observation = ImageObservation {
        url: "https://test/foreign.png".into(),
        width: 1,
        height: 1,
        complete: true,
    };
    assert!(plan.observe(|_| Ok(observation.clone())).is_err());
    let valid = ImageObservation {
        url: "https://test/a.png".into(),
        ..observation
    };
    assert!(ImageRecord::capture(
        plan.clone(),
        vec![],
        "1",
        |_| Ok(valid.clone()),
        AbortSignal::new(),
        10
    )
    .is_err());
    assert!(plan
        .observe(|_| Ok(ImageObservation {
            complete: false,
            ..valid.clone()
        }))
        .is_err());
    let cancel = AbortSignal::new();
    cancel.abort();
    assert!(ImageRecord::capture(plan, vec![], "1", |_| Ok(valid.clone()), cancel, 10).is_err());
}
#[test]
fn picture_candidates_do_not_cross_owners_or_inert_templates() {
    let tree = import_data_bytes(br#"<html xmlns="http://www.w3.org/1999/xhtml"><body><picture><source srcset="a.png"/><img src="b.png"/><source srcset="late.png"/></picture><img src="c.png"/><template><img src="inert.png"/></template></body></html>"#, "application/xml", "cem", "https://test/source").unwrap();
    let plan = ImagePlan::from_tree(tree, "https://test/", 100, 4).unwrap();
    assert_eq!(plan.targets().len(), 2);
    assert_eq!(
        plan.targets()[0].urls,
        vec!["https://test/a.png", "https://test/b.png"]
    );
    assert_eq!(plan.targets()[1].urls, vec!["https://test/c.png"]);
}
#[test]
fn acquisition_bounds_owner_and_callback_cancellation_are_enforced() {
    let make = || {
        let tree = import_data_bytes(
            br#"<img xmlns="http://www.w3.org/1999/xhtml" src="a.png"/>"#,
            "application/xml",
            "cem",
            "https://test/source",
        )
        .unwrap();
        ImagePlan::from_tree(tree, "https://test/", 10, 1).unwrap()
    };
    let value = ImageObservation {
        url: "https://test/a.png".into(),
        width: 1,
        height: 1,
        complete: true,
    };
    let response = || ImageResponse {
        requested_url: value.url.clone(),
        final_url: value.url.clone(),
        revision: "1".into(),
        content_type: "image/png".into(),
        bytes: vec![1, 2],
    };
    let plan = make();
    assert!(ImageRecord::capture(
        plan.clone(),
        vec![response()],
        "1",
        |_| Ok(value.clone()),
        AbortSignal::new(),
        1
    )
    .is_err());
    let mut record = ImageRecord::capture(
        plan.clone(),
        vec![response()],
        "1",
        |_| Ok(value.clone()),
        AbortSignal::new(),
        2,
    )
    .unwrap();
    assert!(record
        .validate(&make(), "1", |_| Ok(value.clone()))
        .is_err());
    assert!(record.validate(&plan, "1", |_| Ok(value.clone())).is_err());
    let cancel = AbortSignal::new();
    assert!(ImageRecord::capture(
        plan,
        vec![response()],
        "1",
        |_| {
            cancel.abort();
            Ok(value.clone())
        },
        cancel.clone(),
        2
    )
    .is_err());
}

#[test]
fn repeated_candidate_occurrences_cannot_be_inferred_from_current_url() {
    for markup in [
        r#"<img srcset="a.png 1x, ./a.png 2x"/>"#,
        r#"<img src="a.png" srcset="a.png 2x"/>"#,
        r#"<picture><source srcset="a.png"/><img src="a.png"/></picture>"#,
    ] {
        let xml = format!(r#"<html xmlns="http://www.w3.org/1999/xhtml">{markup}</html>"#);
        let tree = import_data_bytes(
            xml.as_bytes(),
            "application/xml",
            "cem",
            "https://test/source",
        )
        .unwrap();
        let plan = ImagePlan::from_tree(tree, "https://test/", 100, 4).unwrap();
        assert_eq!(plan.targets()[0].occurrences.len(), 2);
        let error = plan
            .observe(|_| {
                Ok(ImageObservation {
                    url: "https://test/a.png".into(),
                    width: 1,
                    height: 1,
                    complete: true,
                })
            })
            .unwrap_err();
        assert!(error.contains("ambiguous"), "{error}");
    }
}

#[test]
fn candidate_occurrences_retain_source_attribute_and_descriptor() {
    let tree = import_data_bytes(
        br#"<img xmlns="http://www.w3.org/1999/xhtml" srcset="a.png 1x, b.png 2x" src="c.png"/>"#,
        "application/xml",
        "cem",
        "https://test/source",
    )
    .unwrap();
    let plan = ImagePlan::from_tree(tree, "https://test/", 100, 4).unwrap();
    let occurrences = &plan.targets()[0].occurrences;
    assert_eq!(occurrences.len(), 3);
    assert_eq!(occurrences[1].ordinal, 1);
    assert_eq!(occurrences[1].candidate.as_ref().unwrap().url_range, 10..15);
    assert_eq!(
        occurrences[1].candidate.as_ref().unwrap().descriptor,
        cem_ml::import::srcset::SrcsetDescriptor::Density(2.0)
    );
    assert_eq!(occurrences[0].attribute, occurrences[1].attribute);
    assert_ne!(occurrences[0].attribute, occurrences[2].attribute);
    assert!(occurrences[2].candidate.is_none());
    assert!(plan.tree().node(occurrences[1].owner).is_some());
    assert!(plan
        .observe(|_| Ok(ImageObservation {
            url: "https://test/b.png".into(),
            width: 1,
            height: 1,
            complete: true
        }))
        .is_ok());
}

#[test]
fn delivery_ledger_seals_selected_bodies_and_revokes_with_execution() {
    let cancel = AbortSignal::new();
    let mut ledger = ImageDeliveryCapture::new(cancel.clone());
    let response = || ImageResponse {
        requested_url: "https://test/a.png".into(),
        final_url: "https://test/a.png".into(),
        content_type: "image/png".into(),
        revision: String::new(),
        bytes: vec![1, 2, 3],
    };
    ledger.accept(response()).unwrap();
    ledger.accept(response()).unwrap();
    let selected = ledger.seal(&["https://test/a.png".into()]).unwrap();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].bytes, [1, 2, 3]);
    assert_eq!(selected[0].revision.len(), 64);
    assert!(ledger.accept(response()).is_err());
    let mut ledger = ImageDeliveryCapture::new(cancel.clone());
    cancel.abort();
    assert!(ledger.accept(response()).is_err());
}

#[test]
fn delivery_missing_changed_oversized_and_redirected_bodies_fail_closed() {
    let response = || ImageResponse {
        requested_url: "https://test/a.png".into(),
        final_url: "https://test/a.png".into(),
        content_type: "image/png".into(),
        revision: String::new(),
        bytes: vec![1],
    };
    let mut ledger = ImageDeliveryCapture::new(AbortSignal::new());
    assert!(ledger.seal(&["https://test/a.png".into()]).is_err());
    assert!(ledger.accept(response()).is_err());
    for mode in ["changed", "oversized", "redirect", "mime"] {
        let mut ledger = ImageDeliveryCapture::new(AbortSignal::new());
        ledger.accept(response()).unwrap();
        let mut other = response();
        match mode {
            "changed" => other.bytes = vec![2],
            "oversized" => other.bytes = vec![1; 4 * 1024 * 1024 + 1],
            "redirect" => other.final_url = "https://test/b.png".into(),
            _ => other.content_type = "image/svg+xml".into(),
        }
        assert!(ledger.accept(other).is_err());
        assert!(ledger.seal(&["https://test/a.png".into()]).is_err());
    }
}
