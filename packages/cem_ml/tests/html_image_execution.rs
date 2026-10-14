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
