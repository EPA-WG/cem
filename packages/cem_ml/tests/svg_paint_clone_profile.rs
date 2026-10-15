use cem_ml::svg_paint_clone_profile::{
    CloneProbe, CloneProbeObservation, SvgPaintCloneProfile, CHROMIUM_CLONE_PROFILE_VERSION,
};
fn profile() -> SvgPaintCloneProfile {
    SvgPaintCloneProfile::for_browser("chromium", CHROMIUM_CLONE_PROFILE_VERSION).unwrap()
}
fn observations() -> Vec<CloneProbeObservation> {
    let p = profile();
    CloneProbe::ALL
        .iter()
        .map(|case| CloneProbeObservation {
            case: *case,
            source_color: [255, 0, 0, 255],
            host_color: [0, 0, 255, 255],
            pixel: p.expected_pixel(*case),
        })
        .collect()
}
#[test]
fn profile_requires_exact_browser_identity_without_fallback() {
    assert_eq!(profile().browser(), ("chromium", "148.0.7778.96"));
    for (engine, version) in [
        ("firefox", CHROMIUM_CLONE_PROFILE_VERSION),
        ("Chromium", CHROMIUM_CLONE_PROFILE_VERSION),
        ("chromium", "148"),
        ("chromium", "148.0.7778.97"),
        ("chromium", ""),
        ("chromium", "149.0.0.0"),
    ] {
        assert!(SvgPaintCloneProfile::for_browser(engine, version).is_err());
    }
}
#[test]
fn the_six_measured_expectations_are_explicit_not_svg2_substitutions() {
    let p = profile();
    assert_eq!(p.expected_pixel(CloneProbe::OwnGradient), [0, 0, 255, 255]);
    assert_eq!(p.expected_pixel(CloneProbe::OwnPattern), [0, 0, 255, 255]);
    assert_eq!(
        p.expected_pixel(CloneProbe::InheritedGradient),
        [255, 0, 0, 255]
    );
    assert_eq!(
        p.expected_pixel(CloneProbe::InheritedPattern),
        [255, 0, 0, 255]
    );
    assert_eq!(
        p.expected_pixel(CloneProbe::DescriptiveGradient),
        [255, 0, 0, 255]
    );
    assert_eq!(
        p.expected_pixel(CloneProbe::DescriptivePattern),
        [0, 0, 0, 0]
    );
    p.verify_probes(&observations()).unwrap();
}
#[test]
fn missing_duplicate_or_extra_observations_do_not_confirm_the_profile() {
    let p = profile();
    let good = observations();
    assert!(p.verify_probes(&[]).is_err());
    assert!(p.verify_probes(&good[..5]).is_err());
    let mut repeated = good.clone();
    repeated[5] = repeated[0].clone();
    assert!(p.verify_probes(&repeated).is_err());
    let mut extra = good;
    extra.push(extra[0].clone());
    assert!(p.verify_probes(&extra).is_err());
}
#[test]
fn each_pixel_and_both_computed_color_contexts_are_required() {
    let p = profile();
    for index in 0..6 {
        let mut changed = observations();
        changed[index].pixel[0] ^= 1;
        assert!(p.verify_probes(&changed).is_err());
        let mut changed = observations();
        changed[index].source_color = [0, 0, 255, 255];
        assert!(p.verify_probes(&changed).is_err());
        let mut changed = observations();
        changed[index].host_color = [255, 0, 0, 255];
        assert!(p.verify_probes(&changed).is_err());
    }
}
#[test]
fn probe_order_is_irrelevant_but_a_prior_match_does_not_latch_acceptance() {
    let p = profile();
    let mut values = observations();
    values.reverse();
    p.verify_probes(&values).unwrap();
    values[0].pixel = [10, 20, 30, 255];
    assert!(p.verify_probes(&values).is_err());
    p.verify_probes(&observations()).unwrap();
}
#[test]
fn matching_probes_do_not_grant_instance_style_or_execution_authority() {
    let p = profile();
    p.verify_probes(&observations()).unwrap();
    assert!(p
        .blockers()
        .contains(&"svg-paint-clone-style-evaluation-required"));
    assert!(p
        .blockers()
        .contains(&"svg-paint-clone-execution-evidence-required"));
    assert!(p
        .blockers()
        .contains(&"svg-computed-winning-source-required"));
    assert!(p
        .blockers()
        .contains(&"svg-computed-resource-acquisition-required"));
}
